use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use rayon::prelude::*;

use crate::mercator;
use crate::mercator::TILE_SIZE;
use crate::parser::{GeoTiffRaster, TileParsingError};
use crate::png::FastPngEncoder;

const ZOOM_SPAN: u32 = 6;
const MAX_ZOOM_CAP: u32 = 20;

#[derive(Debug, Clone, Copy)]
pub struct GeoTiffZoomRange {
    pub min_zoom: u32,
    pub max_zoom: u32,
}

pub struct GeoTiffTileSlicer;

impl GeoTiffTileSlicer {
    pub fn zoom_range_for(raster: &GeoTiffRaster) -> GeoTiffZoomRange {
        let computed_max_zoom =
            mercator::compute_max_zoom(raster.bounds, raster.width, raster.height);
        let max_zoom = computed_max_zoom.clamp(0, MAX_ZOOM_CAP);
        let min_zoom = max_zoom.saturating_sub(ZOOM_SPAN).min(max_zoom);
        GeoTiffZoomRange { min_zoom, max_zoom }
    }

    pub fn resolve_zoom_range(
        input_path: impl AsRef<Path>,
    ) -> Result<GeoTiffZoomRange, TileParsingError> {
        let raster = GeoTiffRaster::open(input_path)?;
        Ok(Self::zoom_range_for(&raster))
    }

    /// Нарезает тайлы. Возвращает число записанных PNG.
    pub fn slice(
        input_path: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
        min_zoom: u32,
        max_zoom: u32,
    ) -> Result<usize, TileParsingError> {
        let mut raster = GeoTiffRaster::open(input_path)?;
        Self::slice_raster(&mut raster, output_dir, min_zoom, max_zoom)
    }

    /// Параллельная нарезка с общим DashMap-кэшем TIFF decode.
    pub fn slice_raster(
        raster: &mut GeoTiffRaster,
        output_dir: impl AsRef<Path>,
        min_zoom: u32,
        max_zoom: u32,
    ) -> Result<usize, TileParsingError> {
        let output_dir = output_dir.as_ref();

        let total_tiles = mercator::count_tiles_for_zoom_range(raster.bounds, min_zoom, max_zoom);
        if total_tiles == 0 {
            return Err(TileParsingError::new(
                "Не удалось нарезать GeoTIFF",
                "Для указанного файла не найдено тайлов в диапазоне зумов.",
            ));
        }

        let mut jobs: Vec<(u32, mercator::MercatorTileIndex)> = Vec::with_capacity(total_tiles);
        let mut dirs: HashSet<(u32, u32)> = HashSet::new();
        for z in min_zoom..=max_zoom {
            let tiles = mercator::tiles_for_bounds(raster.bounds, z);
            for tile in tiles {
                dirs.insert((z, tile.x));
                jobs.push((z, tile));
            }
        }

        for &(z, x) in &dirs {
            let tile_dir = output_dir.join(z.to_string()).join(x.to_string());
            std::fs::create_dir_all(&tile_dir)
                .map_err(|e| TileParsingError::new("Не удалось создать каталог", e.to_string()))?;
        }

        let factory = raster.worker_factory();
        let shared_tiles = if factory.is_tiled_compressed() {
            Some(Arc::new(DashMap::new()))
        } else {
            None
        };
        let shared_strips = if factory.is_strip_compressed() {
            Some(Arc::new(DashMap::new()))
        } else {
            None
        };

        let output_dir = output_dir.to_path_buf();
        let results: Vec<Result<bool, TileParsingError>> = jobs
            .par_iter()
            .map_init(
                || {
                    (
                        factory.spawn_with_shared(shared_tiles.clone(), shared_strips.clone()),
                        vec![0u8; TILE_SIZE * TILE_SIZE * 4],
                        FastPngEncoder::new(),
                    )
                },
                |(worker, tile_rgba, png), (zoom, tile)| {
                    render_and_write_tile(worker, &output_dir, *zoom, *tile, tile_rgba, png)
                },
            )
            .collect();

        let mut tiles_written = 0usize;
        for result in results {
            if result? {
                tiles_written += 1;
            }
        }

        raster.close();
        Ok(tiles_written)
    }
}

fn render_and_write_tile(
    raster: &mut GeoTiffRaster,
    output_dir: &Path,
    zoom: u32,
    tile: mercator::MercatorTileIndex,
    tile_rgba: &mut [u8],
    png: &mut FastPngEncoder,
) -> Result<bool, TileParsingError> {
    if !raster.render_mercator_tile_rgba(tile, tile_rgba) {
        return Ok(false);
    }

    let png_bytes = png.encode_rgba(tile_rgba, TILE_SIZE, TILE_SIZE);
    let mut tile_path = PathBuf::with_capacity(output_dir.as_os_str().len() + 32);
    tile_path.push(output_dir);
    tile_path.push(zoom.to_string());
    tile_path.push(tile.x.to_string());
    tile_path.push(format!("{}.png", tile.y));

    std::fs::write(&tile_path, png_bytes)
        .map_err(|e| TileParsingError::new("Не удалось записать тайл", e.to_string()))?;
    Ok(true)
}

pub fn count_png_files(output_dir: &Path) -> Result<usize, TileParsingError> {
    let mut count = 0usize;
    collect_png_files(output_dir, &mut count)?;
    Ok(count)
}

fn collect_png_files(dir: &Path, count: &mut usize) -> Result<(), TileParsingError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| TileParsingError::new("Не удалось прочитать каталог", e.to_string()))?
    {
        let entry = entry
            .map_err(|e| TileParsingError::new("Не удалось прочитать каталог", e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_png_files(&path, count)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("png"))
            .unwrap_or(false)
        {
            *count += 1;
        }
    }
    Ok(())
}
