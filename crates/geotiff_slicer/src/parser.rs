use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;

use crate::decoder::decode_tile_to_rgba;
use crate::lzw::GeoTiffLzwDecoder;
use crate::mercator::{MercatorTileIndex, TILE_SIZE, Wgs84Bounds};

const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;
const TAG_BITS_PER_SAMPLE: u16 = 258;
const TAG_COMPRESSION: u16 = 259;
const TAG_SAMPLES_PER_PIXEL: u16 = 277;
const TAG_TILE_WIDTH: u16 = 322;
const TAG_TILE_LENGTH: u16 = 323;
const TAG_STRIP_OFFSETS: u16 = 273;
const TAG_TILE_OFFSETS: u16 = 324;
const TAG_TILE_BYTE_COUNTS: u16 = 325;
const TAG_PREDICTOR: u16 = 317;
const TAG_JPEG_TABLES: u16 = 347;
const TAG_MODEL_PIXEL_SCALE: u16 = 33550;
const TAG_MODEL_TIEPOINT: u16 = 33922;
const TAG_GEO_KEY_DIRECTORY: u16 = 34735;
const TAG_ROWS_PER_STRIP: u16 = 278;
const TAG_STRIP_BYTE_COUNTS: u16 = 279;

const COMPRESSION_NONE: u16 = 1;
const COMPRESSION_LZW: u16 = 5;
const COMPRESSION_OLD_JPEG: u16 = 6;
const COMPRESSION_JPEG: u16 = 7;

const GEO_KEY_GEOGRAPHIC_TYPE: u16 = 2048;
const EPSG_WGS84: u16 = 4326;

const MAX_CACHED_TILES: usize = 128;
const MAX_CACHED_STRIPS: usize = 64;

#[derive(Debug, Clone)]
pub struct TileParsingError {
    pub message: String,
    pub description: String,
}

impl TileParsingError {
    pub fn new(message: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            description: description.into(),
        }
    }
}

impl std::fmt::Display for TileParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.description)
    }
}

impl std::error::Error for TileParsingError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Endian {
    Little,
    Big,
}

struct TagInfo {
    tag_type: u16,
    count: u32,
    value_or_offset: u32,
}

struct LruCache {
    map: HashMap<usize, Arc<[u8]>>,
    order: VecDeque<usize>,
    max_size: usize,
}

impl LruCache {
    fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_size,
        }
    }

    /// Возвращает Arc-клон без перестановки LRU (горячий путь).
    fn get(&self, key: usize) -> Option<Arc<[u8]>> {
        self.map.get(&key).cloned()
    }

    fn insert(&mut self, key: usize, value: Arc<[u8]>) {
        if self.map.contains_key(&key) {
            self.map.insert(key, value);
            return;
        }
        self.map.insert(key, value);
        self.order.push_back(key);
        while self.order.len() > self.max_size {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

pub struct GeoTiffRaster {
    bytes: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    samples_per_pixel: usize,
    _bits_per_sample: u32,
    compression: u32,
    pub bounds: Wgs84Bounds,
    is_tiled: bool,
    tile_width: usize,
    tile_length: usize,
    strip_offsets: Arc<[usize]>,
    tile_offsets: Arc<[usize]>,
    tile_byte_counts: Arc<[usize]>,
    strip_byte_counts: Arc<[usize]>,
    rows_per_strip: usize,
    predictor: u32,
    jpeg_tables: Option<Arc<[u8]>>,
    inv_scale_x: f64,
    inv_scale_y: f64,
    tie_i: f64,
    tie_j: f64,
    x_origin: f64,
    y_origin: f64,
    tile_cache: LruCache,
    strip_cache: LruCache,
    last_tile_index: Option<usize>,
    last_tile_rgba: Option<Arc<[u8]>>,
    last_strip_index: Option<usize>,
    last_strip_rgba: Option<Arc<[u8]>>,
    lzw_decoder: GeoTiffLzwDecoder,
    /// Общий кэш TIFF-тайлов между rayon-воркерами (decode-once).
    shared_tiles: Option<Arc<DashMap<usize, Arc<[u8]>>>>,
    shared_strips: Option<Arc<DashMap<usize, Arc<[u8]>>>>,
}

impl GeoTiffRaster {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TileParsingError> {
        let file_bytes = std::fs::read(path)
            .map_err(|e| TileParsingError::new("Не удалось открыть файл", e.to_string()))?;
        Self::from_bytes(file_bytes)
    }

    fn from_bytes(file_bytes: Vec<u8>) -> Result<Self, TileParsingError> {
        let endian = read_endian(&file_bytes)?;
        let first_ifd_offset = get_u32(&file_bytes, 4, endian) as usize;
        let tags = read_tags(&file_bytes, endian, first_ifd_offset)?;

        let width = read_tag_scalar(&tags, &file_bytes, endian, TAG_IMAGE_WIDTH, 0)?;
        let height = read_tag_scalar(&tags, &file_bytes, endian, TAG_IMAGE_LENGTH, 0)?;
        let samples_per_pixel =
            read_tag_scalar(&tags, &file_bytes, endian, TAG_SAMPLES_PER_PIXEL, 1)? as usize;
        let compression = read_tag_scalar(&tags, &file_bytes, endian, TAG_COMPRESSION, 1)?;
        let bits_per_sample = read_tag_scalar(&tags, &file_bytes, endian, TAG_BITS_PER_SAMPLE, 8)?;

        assert_supported_compression(compression)?;

        if !is_compressed_compression(compression) && bits_per_sample != 8 {
            return Err(TileParsingError::new(
                "Неподдерживаемая глубина цвета",
                "Поддерживаются только 8-битные каналы.",
            ));
        }

        assert_geographic_wgs84(&tags, &file_bytes, endian)?;

        let pixel_scale = read_tag_doubles(&tags, &file_bytes, endian, TAG_MODEL_PIXEL_SCALE, 3)?;
        let tiepoint = read_tag_doubles(&tags, &file_bytes, endian, TAG_MODEL_TIEPOINT, 6)?;

        let tile_width = read_tag_scalar(&tags, &file_bytes, endian, TAG_TILE_WIDTH, 0)? as usize;
        let tile_length = read_tag_scalar(&tags, &file_bytes, endian, TAG_TILE_LENGTH, 0)? as usize;
        let is_tiled = tile_width > 0 && tile_length > 0;

        let strip_offsets = if is_tiled {
            Vec::new()
        } else {
            read_tag_int_list(&tags, &file_bytes, endian, TAG_STRIP_OFFSETS)?
        };
        let tile_offsets = if is_tiled {
            read_tag_int_list(&tags, &file_bytes, endian, TAG_TILE_OFFSETS)?
        } else {
            Vec::new()
        };
        let tile_byte_counts = if is_tiled {
            read_tag_int_list(&tags, &file_bytes, endian, TAG_TILE_BYTE_COUNTS)?
        } else {
            Vec::new()
        };
        let strip_byte_counts = if is_tiled {
            Vec::new()
        } else {
            read_tag_int_list(&tags, &file_bytes, endian, TAG_STRIP_BYTE_COUNTS)?
        };

        let rows_per_strip =
            read_tag_scalar(&tags, &file_bytes, endian, TAG_ROWS_PER_STRIP, height)? as usize;
        let predictor = read_tag_scalar(&tags, &file_bytes, endian, TAG_PREDICTOR, 1)?;

        let jpeg_tables = if is_compressed_compression(compression)
            && (compression == COMPRESSION_JPEG as u32
                || compression == COMPRESSION_OLD_JPEG as u32)
        {
            tags.get(&TAG_JPEG_TABLES)
                .map(|tag| Arc::<[u8]>::from(read_tag_bytes(tag, &file_bytes)))
        } else {
            None
        };

        let origin_lon = tiepoint[3];
        let origin_lat = tiepoint[4];
        let scale_x = pixel_scale[0];
        let scale_y = pixel_scale[1];

        Ok(Self {
            bytes: Arc::from(file_bytes),
            width,
            height,
            samples_per_pixel,
            _bits_per_sample: bits_per_sample,
            compression,
            bounds: Wgs84Bounds {
                west: origin_lon,
                south: origin_lat - height as f64 * scale_y,
                east: origin_lon + width as f64 * scale_x,
                north: origin_lat,
            },
            is_tiled,
            tile_width: if is_tiled { tile_width } else { width as usize },
            tile_length: if is_tiled {
                tile_length
            } else {
                rows_per_strip
            },
            strip_offsets: Arc::from(strip_offsets),
            tile_offsets: Arc::from(tile_offsets),
            tile_byte_counts: Arc::from(tile_byte_counts),
            strip_byte_counts: Arc::from(strip_byte_counts),
            rows_per_strip,
            predictor,
            jpeg_tables,
            inv_scale_x: 1.0 / pixel_scale[0],
            inv_scale_y: 1.0 / pixel_scale[1],
            tie_i: tiepoint[0],
            tie_j: tiepoint[1],
            x_origin: tiepoint[3],
            y_origin: tiepoint[4],
            tile_cache: LruCache::new(MAX_CACHED_TILES),
            strip_cache: LruCache::new(MAX_CACHED_STRIPS),
            last_tile_index: None,
            last_tile_rgba: None,
            last_strip_index: None,
            last_strip_rgba: None,
            lzw_decoder: GeoTiffLzwDecoder::new(),
            shared_tiles: None,
            shared_strips: None,
        })
    }

    /// Фабрика Sync-воркеров для rayon (без сырых указателей LZW).
    pub fn worker_factory(&self) -> GeoTiffWorkerFactory {
        GeoTiffWorkerFactory {
            bytes: Arc::clone(&self.bytes),
            width: self.width,
            height: self.height,
            samples_per_pixel: self.samples_per_pixel,
            bits_per_sample: self._bits_per_sample,
            compression: self.compression,
            bounds: self.bounds,
            is_tiled: self.is_tiled,
            tile_width: self.tile_width,
            tile_length: self.tile_length,
            strip_offsets: Arc::clone(&self.strip_offsets),
            tile_offsets: Arc::clone(&self.tile_offsets),
            tile_byte_counts: Arc::clone(&self.tile_byte_counts),
            strip_byte_counts: Arc::clone(&self.strip_byte_counts),
            rows_per_strip: self.rows_per_strip,
            predictor: self.predictor,
            jpeg_tables: self.jpeg_tables.clone(),
            inv_scale_x: self.inv_scale_x,
            inv_scale_y: self.inv_scale_y,
            tie_i: self.tie_i,
            tie_j: self.tie_j,
            x_origin: self.x_origin,
            y_origin: self.y_origin,
        }
    }

    pub fn close(&mut self) {
        self.tile_cache.clear();
        self.strip_cache.clear();
        self.last_tile_index = None;
        self.last_tile_rgba = None;
        self.last_strip_index = None;
        self.last_strip_rgba = None;
    }
}

/// Общие immutable-данные растра; Send+Sync для rayon.
#[derive(Clone)]
pub struct GeoTiffWorkerFactory {
    bytes: Arc<[u8]>,
    width: u32,
    height: u32,
    samples_per_pixel: usize,
    bits_per_sample: u32,
    compression: u32,
    bounds: Wgs84Bounds,
    is_tiled: bool,
    tile_width: usize,
    tile_length: usize,
    strip_offsets: Arc<[usize]>,
    tile_offsets: Arc<[usize]>,
    tile_byte_counts: Arc<[usize]>,
    strip_byte_counts: Arc<[usize]>,
    rows_per_strip: usize,
    predictor: u32,
    jpeg_tables: Option<Arc<[u8]>>,
    inv_scale_x: f64,
    inv_scale_y: f64,
    tie_i: f64,
    tie_j: f64,
    x_origin: f64,
    y_origin: f64,
}

impl GeoTiffWorkerFactory {
    pub fn spawn(&self) -> GeoTiffRaster {
        GeoTiffRaster {
            bytes: Arc::clone(&self.bytes),
            width: self.width,
            height: self.height,
            samples_per_pixel: self.samples_per_pixel,
            _bits_per_sample: self.bits_per_sample,
            compression: self.compression,
            bounds: self.bounds,
            is_tiled: self.is_tiled,
            tile_width: self.tile_width,
            tile_length: self.tile_length,
            strip_offsets: Arc::clone(&self.strip_offsets),
            tile_offsets: Arc::clone(&self.tile_offsets),
            tile_byte_counts: Arc::clone(&self.tile_byte_counts),
            strip_byte_counts: Arc::clone(&self.strip_byte_counts),
            rows_per_strip: self.rows_per_strip,
            predictor: self.predictor,
            jpeg_tables: self.jpeg_tables.clone(),
            inv_scale_x: self.inv_scale_x,
            inv_scale_y: self.inv_scale_y,
            tie_i: self.tie_i,
            tie_j: self.tie_j,
            x_origin: self.x_origin,
            y_origin: self.y_origin,
            tile_cache: LruCache::new(MAX_CACHED_TILES),
            strip_cache: LruCache::new(MAX_CACHED_STRIPS),
            last_tile_index: None,
            last_tile_rgba: None,
            last_strip_index: None,
            last_strip_rgba: None,
            lzw_decoder: GeoTiffLzwDecoder::new(),
            shared_tiles: None,
            shared_strips: None,
        }
    }

    pub fn spawn_with_shared(
        &self,
        shared_tiles: Option<Arc<DashMap<usize, Arc<[u8]>>>>,
        shared_strips: Option<Arc<DashMap<usize, Arc<[u8]>>>>,
    ) -> GeoTiffRaster {
        let mut raster = self.spawn();
        raster.shared_tiles = shared_tiles;
        raster.shared_strips = shared_strips;
        raster
    }

    pub fn is_tiled_compressed(&self) -> bool {
        self.is_tiled && is_compressed_compression(self.compression)
    }

    pub fn is_strip_compressed(&self) -> bool {
        !self.is_tiled && is_compressed_compression(self.compression)
    }
}

impl GeoTiffRaster {
    /// Рендерит XYZ-тайл в переиспользуемый буфер `rgba` (длина ≥ 256×256×4).
    /// Возвращает `true`, если есть хотя бы один непрозрачный пиксель.
    pub fn render_mercator_tile_rgba(&mut self, tile: MercatorTileIndex, rgba: &mut [u8]) -> bool {
        debug_assert!(rgba.len() >= TILE_SIZE * TILE_SIZE * 4);
        rgba[..TILE_SIZE * TILE_SIZE * 4].fill(0);

        let mut has_visible_pixel = false;

        let n = 1u32 << tile.z;
        let world_size = n as f64 * TILE_SIZE as f64;
        let lon_scale = 360.0 / world_size;
        let base_lon = tile.x as f64 * TILE_SIZE as f64 * lon_scale - 180.0;
        let inv_pi = 1.0 / PI;
        let height = self.height as i32;
        let width = self.width as i32;

        // src_x = floor(src_x_base + px * src_x_scale) — линейно по px.
        let src_x_scale = lon_scale * self.inv_scale_x;
        let src_x_base = (base_lon - self.x_origin) * self.inv_scale_x + self.tie_i;

        for py in 0..TILE_SIZE {
            let global_y = tile.y as f64 * TILE_SIZE as f64 + py as f64;
            let lat_rad = (PI * (1.0 - 2.0 * global_y / world_size)).sinh().atan();
            let lat = lat_rad * 180.0 * inv_pi;
            let src_y = ((self.y_origin - lat) * self.inv_scale_y + self.tie_j).floor() as i32;
            if src_y < 0 || src_y >= height {
                continue;
            }

            let dst_base = py * TILE_SIZE * 4;
            self.sample_mercator_row(
                src_y as usize,
                src_x_base,
                src_x_scale,
                width,
                &mut rgba[dst_base..dst_base + TILE_SIZE * 4],
                &mut has_visible_pixel,
            );
        }

        has_visible_pixel
    }

    fn sample_mercator_row(
        &mut self,
        src_y: usize,
        src_x_base: f64,
        src_x_scale: f64,
        width: i32,
        dst_row: &mut [u8],
        has_visible: &mut bool,
    ) -> bool {
        if is_compressed_compression(self.compression) {
            if self.is_tiled {
                return self.sample_row_tiled(
                    src_y,
                    src_x_base,
                    src_x_scale,
                    width,
                    dst_row,
                    has_visible,
                );
            }
            return self.sample_row_strip(
                src_y,
                src_x_base,
                src_x_scale,
                width,
                dst_row,
                has_visible,
            );
        }
        self.sample_row_uncompressed(src_y, src_x_base, src_x_scale, width, dst_row, has_visible)
    }

    fn sample_row_tiled(
        &mut self,
        src_y: usize,
        src_x_base: f64,
        src_x_scale: f64,
        width: i32,
        dst_row: &mut [u8],
        has_visible: &mut bool,
    ) -> bool {
        let tile_width = self.tile_width;
        let tile_length = self.tile_length;
        let tiles_across = (self.width as usize + tile_width - 1) / tile_width;
        let tile_row = src_y / tile_length;
        let local_y = src_y - tile_row * tile_length;

        let mut sticky_index: Option<usize> = None;
        let mut sticky_rgba: Option<Arc<[u8]>> = None;

        let mut px = 0usize;
        while px < TILE_SIZE {
            let src_x = (src_x_base + px as f64 * src_x_scale).floor() as i32;
            if src_x < 0 || src_x >= width {
                px += 1;
                continue;
            }
            let src_x = src_x as usize;
            let tile_col = src_x / tile_width;
            let tile_index = tile_row * tiles_across + tile_col;

            if sticky_index != Some(tile_index) {
                match self.get_cached_tile(tile_index) {
                    Ok(v) => {
                        sticky_index = Some(tile_index);
                        sticky_rgba = Some(v);
                    }
                    Err(_) => {
                        px += 1;
                        continue;
                    }
                }
            }
            let tile_rgba = sticky_rgba.as_ref().unwrap();
            let local_x0 = src_x - tile_col * tile_width;
            let row_off = local_y * tile_width * 4;

            // Спан: consecutive px с src_x, src_x+1, ... в пределах того же TIFF-тайла.
            let mut run = 1usize;
            let mut expected = src_x + 1;
            while px + run < TILE_SIZE {
                let next_x = (src_x_base + (px + run) as f64 * src_x_scale).floor() as i32;
                if next_x < 0 || next_x >= width {
                    break;
                }
                let next_x = next_x as usize;
                if next_x != expected || next_x / tile_width != tile_col {
                    break;
                }
                expected += 1;
                run += 1;
            }

            let max_run = tile_width - local_x0;
            let run = run.min(max_run);
            let src_off = row_off + local_x0 * 4;
            let dst_off = px * 4;
            let bytes = run * 4;
            dst_row[dst_off..dst_off + bytes].copy_from_slice(&tile_rgba[src_off..src_off + bytes]);

            if !*has_visible {
                for i in 0..run {
                    if dst_row[dst_off + i * 4 + 3] != 0 {
                        *has_visible = true;
                        break;
                    }
                }
            }

            // Overzoom: несколько px → один src_x.
            if run == 1 {
                let mut same = 1usize;
                while px + same < TILE_SIZE {
                    let next_x = (src_x_base + (px + same) as f64 * src_x_scale).floor() as i32;
                    if next_x != src_x as i32 {
                        break;
                    }
                    same += 1;
                }
                if same > 1 {
                    let pixel = [
                        dst_row[dst_off],
                        dst_row[dst_off + 1],
                        dst_row[dst_off + 2],
                        dst_row[dst_off + 3],
                    ];
                    for i in 1..same {
                        let o = (px + i) * 4;
                        dst_row[o..o + 4].copy_from_slice(&pixel);
                    }
                    px += same;
                    continue;
                }
            }

            px += run;
        }
        true
    }

    fn sample_row_strip(
        &mut self,
        src_y: usize,
        src_x_base: f64,
        src_x_scale: f64,
        width: i32,
        dst_row: &mut [u8],
        has_visible: &mut bool,
    ) -> bool {
        let strip_index = src_y / self.rows_per_strip;
        let Ok(strip_rgba) = self.get_cached_strip(strip_index) else {
            return false;
        };
        let row_in_strip = src_y % self.rows_per_strip;
        let row_off = row_in_strip * self.width as usize * 4;
        let img_w = self.width as usize;

        let mut px = 0usize;
        while px < TILE_SIZE {
            let src_x = (src_x_base + px as f64 * src_x_scale).floor() as i32;
            if src_x < 0 || src_x >= width {
                px += 1;
                continue;
            }
            let src_x = src_x as usize;

            let mut run = 1usize;
            let mut expected = src_x + 1;
            while px + run < TILE_SIZE {
                let next_x = (src_x_base + (px + run) as f64 * src_x_scale).floor() as i32;
                if next_x < 0 || next_x >= width {
                    break;
                }
                let next_x = next_x as usize;
                if next_x != expected {
                    break;
                }
                expected += 1;
                run += 1;
            }

            let max_run = img_w - src_x;
            let run = run.min(max_run);
            let src_off = row_off + src_x * 4;
            let dst_off = px * 4;
            let bytes = run * 4;
            dst_row[dst_off..dst_off + bytes]
                .copy_from_slice(&strip_rgba[src_off..src_off + bytes]);

            if !*has_visible {
                for i in 0..run {
                    if dst_row[dst_off + i * 4 + 3] != 0 {
                        *has_visible = true;
                        break;
                    }
                }
            }

            if run == 1 {
                let mut same = 1usize;
                while px + same < TILE_SIZE {
                    let next_x = (src_x_base + (px + same) as f64 * src_x_scale).floor() as i32;
                    if next_x != src_x as i32 {
                        break;
                    }
                    same += 1;
                }
                if same > 1 {
                    let pixel = [
                        dst_row[dst_off],
                        dst_row[dst_off + 1],
                        dst_row[dst_off + 2],
                        dst_row[dst_off + 3],
                    ];
                    for i in 1..same {
                        let o = (px + i) * 4;
                        dst_row[o..o + 4].copy_from_slice(&pixel);
                    }
                    px += same;
                    continue;
                }
            }

            px += run;
        }
        true
    }

    fn sample_row_uncompressed(
        &mut self,
        src_y: usize,
        src_x_base: f64,
        src_x_scale: f64,
        width: i32,
        dst_row: &mut [u8],
        has_visible: &mut bool,
    ) -> bool {
        let spp = self.samples_per_pixel;
        for px in 0..TILE_SIZE {
            let src_x = (src_x_base + px as f64 * src_x_scale).floor() as i32;
            if src_x < 0 || src_x >= width {
                continue;
            }
            let Some(offset) = self.pixel_offset(src_x as usize, src_y) else {
                continue;
            };
            let dst = px * 4;
            let r = self.bytes[offset];
            if spp == 1 {
                dst_row[dst] = r;
                dst_row[dst + 1] = r;
                dst_row[dst + 2] = r;
                dst_row[dst + 3] = 255;
            } else {
                dst_row[dst] = r;
                dst_row[dst + 1] = self.bytes[offset + 1];
                dst_row[dst + 2] = self.bytes[offset + 2];
                dst_row[dst + 3] = if spp >= 4 {
                    self.bytes[offset + 3]
                } else {
                    255
                };
            }
            if dst_row[dst + 3] != 0 {
                *has_visible = true;
            }
        }
        true
    }

    fn get_cached_tile(&mut self, tile_index: usize) -> Result<Arc<[u8]>, TileParsingError> {
        if self.last_tile_index == Some(tile_index) {
            if let Some(ref rgba) = self.last_tile_rgba {
                return Ok(Arc::clone(rgba));
            }
        }

        if let Some(ref shared) = self.shared_tiles {
            if let Some(cached) = shared.get(&tile_index) {
                let cached = Arc::clone(cached.value());
                self.last_tile_index = Some(tile_index);
                self.last_tile_rgba = Some(Arc::clone(&cached));
                return Ok(cached);
            }
        }

        if let Some(cached) = self.tile_cache.get(tile_index) {
            self.last_tile_index = Some(tile_index);
            self.last_tile_rgba = Some(Arc::clone(&cached));
            return Ok(cached);
        }

        if tile_index >= self.tile_offsets.len() {
            return Err(TileParsingError::new(
                "Некорректный GeoTIFF",
                format!("Индекс тайла {tile_index} вне диапазона."),
            ));
        }

        let offset = self.tile_offsets[tile_index];
        let length = self.tile_byte_counts[tile_index];
        if offset + length > self.bytes.len() {
            return Err(TileParsingError::new(
                "Некорректный TIFF",
                "Попытка чтения за пределами файла.",
            ));
        }
        let compressed = &self.bytes[offset..offset + length];
        let compression = self.compression as u16;
        let tile_width = self.tile_width;
        let tile_length = self.tile_length;
        let samples_per_pixel = self.samples_per_pixel;
        let predictor = self.predictor as u16;
        let jpeg_tables = self.jpeg_tables.as_deref();
        let rgba = decode_tile_to_rgba(
            &mut self.lzw_decoder,
            compressed,
            compression,
            tile_width,
            tile_length,
            samples_per_pixel,
            predictor,
            jpeg_tables,
        )?;
        let rgba: Arc<[u8]> = Arc::from(rgba);
        if let Some(ref shared) = self.shared_tiles {
            shared.insert(tile_index, Arc::clone(&rgba));
        } else {
            self.tile_cache.insert(tile_index, Arc::clone(&rgba));
        }
        self.last_tile_index = Some(tile_index);
        self.last_tile_rgba = Some(Arc::clone(&rgba));
        Ok(rgba)
    }

    fn get_cached_strip(&mut self, strip_index: usize) -> Result<Arc<[u8]>, TileParsingError> {
        if self.last_strip_index == Some(strip_index) {
            if let Some(ref rgba) = self.last_strip_rgba {
                return Ok(Arc::clone(rgba));
            }
        }

        if let Some(ref shared) = self.shared_strips {
            if let Some(cached) = shared.get(&strip_index) {
                let cached = Arc::clone(cached.value());
                self.last_strip_index = Some(strip_index);
                self.last_strip_rgba = Some(Arc::clone(&cached));
                return Ok(cached);
            }
        }

        if let Some(cached) = self.strip_cache.get(strip_index) {
            self.last_strip_index = Some(strip_index);
            self.last_strip_rgba = Some(Arc::clone(&cached));
            return Ok(cached);
        }

        if strip_index >= self.strip_offsets.len() {
            return Err(TileParsingError::new(
                "Некорректный GeoTIFF",
                format!("Индекс strip {strip_index} вне диапазона."),
            ));
        }

        let rows_in_strip = if strip_index == self.strip_offsets.len() - 1 {
            self.height as usize - strip_index * self.rows_per_strip
        } else {
            self.rows_per_strip
        };
        let offset = self.strip_offsets[strip_index];
        let length = self.strip_byte_counts[strip_index];
        if offset + length > self.bytes.len() {
            return Err(TileParsingError::new(
                "Некорректный TIFF",
                "Попытка чтения за пределами файла.",
            ));
        }
        let compressed = &self.bytes[offset..offset + length];
        let compression = self.compression as u16;
        let width = self.width as usize;
        let samples_per_pixel = self.samples_per_pixel;
        let predictor = self.predictor as u16;
        let jpeg_tables = self.jpeg_tables.as_deref();
        let rgba = decode_tile_to_rgba(
            &mut self.lzw_decoder,
            compressed,
            compression,
            width,
            rows_in_strip,
            samples_per_pixel,
            predictor,
            jpeg_tables,
        )?;
        let rgba: Arc<[u8]> = Arc::from(rgba);
        if let Some(ref shared) = self.shared_strips {
            shared.insert(strip_index, Arc::clone(&rgba));
        } else {
            self.strip_cache.insert(strip_index, Arc::clone(&rgba));
        }
        self.last_strip_index = Some(strip_index);
        self.last_strip_rgba = Some(Arc::clone(&rgba));
        Ok(rgba)
    }

    fn pixel_offset(&self, x: usize, y: usize) -> Option<usize> {
        if self.compression != COMPRESSION_NONE as u32 {
            return None;
        }

        if self.is_tiled {
            let tiles_across = (self.width as usize + self.tile_width - 1) / self.tile_width;
            let tile_col = x / self.tile_width;
            let tile_row = y / self.tile_length;
            let tile_index = tile_row * tiles_across + tile_col;

            if tile_index >= self.tile_offsets.len() {
                return None;
            }

            let local_x = x % self.tile_width;
            let local_y = y % self.tile_length;
            let tile_offset = self.tile_offsets[tile_index];
            let pixel_index = (local_y * self.tile_width + local_x) * self.samples_per_pixel;
            return Some(tile_offset + pixel_index);
        }

        let strip_index = y / self.rows_per_strip;
        if strip_index >= self.strip_offsets.len() {
            return None;
        }

        let row_in_strip = y % self.rows_per_strip;
        let strip_offset = self.strip_offsets[strip_index];
        let pixel_index = (row_in_strip * self.width as usize + x) * self.samples_per_pixel;
        Some(strip_offset + pixel_index)
    }
}

fn data_type_size(tag_type: u16) -> usize {
    match tag_type {
        1 => 1,
        2 => 1,
        3 => 2,
        4 => 4,
        5 => 8,
        7 => 1,
        12 => 8,
        _ => 0,
    }
}

fn is_compressed_compression(compression: u32) -> bool {
    compression == COMPRESSION_LZW as u32
        || compression == COMPRESSION_OLD_JPEG as u32
        || compression == COMPRESSION_JPEG as u32
}

fn assert_supported_compression(compression: u32) -> Result<(), TileParsingError> {
    if compression == COMPRESSION_NONE as u32 || is_compressed_compression(compression) {
        return Ok(());
    }
    Err(TileParsingError::new(
        "Неподдерживаемый формат GeoTIFF",
        "Поддерживаются несжатые TIFF (compression=1), LZW (5) и JPEG (6/7).",
    ))
}

fn read_endian(bytes: &[u8]) -> Result<Endian, TileParsingError> {
    let byte_order_value = get_u16(bytes, 0, Endian::Little);
    if byte_order_value == 0x4949 {
        Ok(Endian::Little)
    } else if byte_order_value == 0x4D4D {
        Ok(Endian::Big)
    } else {
        Err(TileParsingError::new(
            "Некорректный TIFF",
            "Не удалось определить порядок байт.",
        ))
    }
}

fn get_u16(bytes: &[u8], offset: usize, endian: Endian) -> u16 {
    let b = &bytes[offset..offset + 2];
    match endian {
        Endian::Little => u16::from_le_bytes([b[0], b[1]]),
        Endian::Big => u16::from_be_bytes([b[0], b[1]]),
    }
}

fn get_u32(bytes: &[u8], offset: usize, endian: Endian) -> u32 {
    let b = &bytes[offset..offset + 4];
    match endian {
        Endian::Little => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        Endian::Big => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
    }
}

fn get_f64(bytes: &[u8], offset: usize, endian: Endian) -> f64 {
    let b = &bytes[offset..offset + 8];
    match endian {
        Endian::Little => f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        Endian::Big => f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
    }
}

fn read_tags(
    bytes: &[u8],
    endian: Endian,
    ifd_offset: usize,
) -> Result<HashMap<u16, TagInfo>, TileParsingError> {
    let mut tags = HashMap::new();
    let num_tags = get_u16(bytes, ifd_offset, endian) as usize;
    let tag_block_offset = ifd_offset + 2;

    for i in 0..num_tags {
        let tag_offset = tag_block_offset + i * 12;
        let tag_id = get_u16(bytes, tag_offset, endian);
        tags.insert(
            tag_id,
            TagInfo {
                tag_type: get_u16(bytes, tag_offset + 2, endian),
                count: get_u32(bytes, tag_offset + 4, endian),
                value_or_offset: get_u32(bytes, tag_offset + 8, endian),
            },
        );
    }
    Ok(tags)
}

fn assert_geographic_wgs84(
    tags: &HashMap<u16, TagInfo>,
    bytes: &[u8],
    endian: Endian,
) -> Result<(), TileParsingError> {
    if !tags.contains_key(&TAG_MODEL_PIXEL_SCALE) || !tags.contains_key(&TAG_MODEL_TIEPOINT) {
        return Err(TileParsingError::new(
            "Файл без геопривязки",
            "GeoTIFF должен содержать теги ModelPixelScale и ModelTiepoint.",
        ));
    }

    let geo_keys_tag = tags.get(&TAG_GEO_KEY_DIRECTORY).ok_or_else(|| {
        TileParsingError::new(
            "Файл без геопривязки",
            "GeoTIFF должен содержать GeoKeyDirectory.",
        )
    })?;

    let keys = read_tag_short_list(geo_keys_tag, bytes, endian)?;
    if keys.len() < 4 {
        return Err(TileParsingError::new(
            "Некорректный GeoTIFF",
            "Повреждён GeoKeyDirectory.",
        ));
    }

    let num_keys = keys[3] as usize;
    for i in 0..num_keys {
        let base = 4 + i * 4;
        if base + 3 >= keys.len() {
            break;
        }
        let key_id = keys[base];
        let value = keys[base + 3];
        if key_id == GEO_KEY_GEOGRAPHIC_TYPE && value != EPSG_WGS84 {
            return Err(TileParsingError::new(
                "Неподдерживаемая проекция",
                "Поддерживается только EPSG:4326 (WGS84).",
            ));
        }
    }
    Ok(())
}

fn read_tag_scalar(
    tags: &HashMap<u16, TagInfo>,
    bytes: &[u8],
    endian: Endian,
    tag_id: u16,
    fallback: u32,
) -> Result<u32, TileParsingError> {
    let Some(tag) = tags.get(&tag_id) else {
        return Ok(fallback);
    };
    if data_type_size(tag.tag_type) * tag.count as usize <= 4 {
        return Ok(tag.value_or_offset);
    }
    let list = read_tag_short_list(tag, bytes, endian)?;
    Ok(list.first().copied().unwrap_or(fallback as u16) as u32)
}

fn read_tag_int_list(
    tags: &HashMap<u16, TagInfo>,
    bytes: &[u8],
    endian: Endian,
    tag_id: u16,
) -> Result<Vec<usize>, TileParsingError> {
    let tag = tags.get(&tag_id).ok_or_else(|| {
        TileParsingError::new(
            "Некорректный TIFF",
            format!("Отсутствует обязательный тег {tag_id}."),
        )
    })?;

    if data_type_size(tag.tag_type) * tag.count as usize <= 4 {
        return Ok(vec![tag.value_or_offset as usize]);
    }

    let mut list = Vec::with_capacity(tag.count as usize);
    let data_ptr = tag.value_or_offset as usize;
    for i in 0..tag.count as usize {
        let value = if tag.tag_type == 3 {
            get_u16(bytes, data_ptr + i * 2, endian) as usize
        } else if tag.tag_type == 4 {
            get_u32(bytes, data_ptr + i * 4, endian) as usize
        } else {
            return Err(TileParsingError::new(
                "Некорректный TIFF",
                format!("Неподдерживаемый тип тега {tag_id}."),
            ));
        };
        list.push(value);
    }
    Ok(list)
}

fn read_tag_bytes(tag: &TagInfo, bytes: &[u8]) -> Vec<u8> {
    let inline_size = data_type_size(tag.tag_type) * tag.count as usize;
    if inline_size <= 4 {
        let mut buf = tag.value_or_offset.to_le_bytes().to_vec();
        buf.truncate(tag.count as usize);
        return buf;
    }
    bytes[tag.value_or_offset as usize..tag.value_or_offset as usize + tag.count as usize].to_vec()
}

fn read_tag_short_list(
    tag: &TagInfo,
    bytes: &[u8],
    endian: Endian,
) -> Result<Vec<u16>, TileParsingError> {
    if data_type_size(tag.tag_type) * tag.count as usize <= 4 {
        return Ok(vec![tag.value_or_offset as u16]);
    }

    let mut list = Vec::with_capacity(tag.count as usize);
    let data_ptr = tag.value_or_offset as usize;
    for i in 0..tag.count as usize {
        list.push(get_u16(bytes, data_ptr + i * 2, endian));
    }
    Ok(list)
}

fn read_tag_doubles(
    tags: &HashMap<u16, TagInfo>,
    bytes: &[u8],
    endian: Endian,
    tag_id: u16,
    expected: usize,
) -> Result<Vec<f64>, TileParsingError> {
    let tag = tags.get(&tag_id).ok_or_else(|| {
        TileParsingError::new("Файл без геопривязки", format!("Отсутствует тег {tag_id}."))
    })?;

    let data_ptr = tag.value_or_offset as usize;
    let mut values = Vec::with_capacity(expected);
    for i in 0..expected {
        values.push(get_f64(bytes, data_ptr + i * 8, endian));
    }
    Ok(values)
}
