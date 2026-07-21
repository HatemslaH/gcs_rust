use crate::parser::{GeoTiffRaster, TileParsingError};
use crate::slicer::GeoTiffTileSlicer;

/// Диапазон зумов для нарезки GeoTIFF.
#[derive(Debug, Clone)]
pub struct GeoTiffZoomRangeDto {
    pub min_zoom: u32,
    pub max_zoom: u32,
}

/// Результат нарезки GeoTIFF в XYZ PNG-тайлы.
#[derive(Debug, Clone)]
pub struct GeoTiffSliceResult {
    pub min_zoom: u32,
    pub max_zoom: u32,
    pub tiles_written: u32,
}

/// Ошибка парсинга / нарезки GeoTIFF (пробрасывается во Flutter).
#[derive(Debug, Clone)]
pub struct GeoTiffError {
    pub message: String,
    pub description: String,
}

impl std::fmt::Display for GeoTiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message, self.description)
    }
}

impl std::error::Error for GeoTiffError {}

impl From<TileParsingError> for GeoTiffError {
    fn from(value: TileParsingError) -> Self {
        Self {
            message: value.message,
            description: value.description,
        }
    }
}

/// Определяет min/max zoom для файла GeoTIFF.
pub fn resolve_zoom_range(input_path: String) -> Result<GeoTiffZoomRangeDto, GeoTiffError> {
    let range = GeoTiffTileSlicer::resolve_zoom_range(input_path)?;
    Ok(GeoTiffZoomRangeDto {
        min_zoom: range.min_zoom,
        max_zoom: range.max_zoom,
    })
}

/// Нарезает GeoTIFF в XYZ PNG. Если zoom не задан — берётся авто-диапазон.
pub fn slice_geotiff(
    input_path: String,
    output_dir: String,
    min_zoom: Option<u32>,
    max_zoom: Option<u32>,
) -> Result<GeoTiffSliceResult, GeoTiffError> {
    let mut raster = GeoTiffRaster::open(&input_path)?;
    let auto = GeoTiffTileSlicer::zoom_range_for(&raster);
    let resolved_min = min_zoom.unwrap_or(auto.min_zoom);
    let resolved_max = max_zoom.unwrap_or(auto.max_zoom);
    let tiles_written =
        GeoTiffTileSlicer::slice_raster(&mut raster, &output_dir, resolved_min, resolved_max)?;
    Ok(GeoTiffSliceResult {
        min_zoom: resolved_min,
        max_zoom: resolved_max,
        tiles_written: tiles_written as u32,
    })
}
