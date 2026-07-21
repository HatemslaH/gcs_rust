//! Парсинг GeoTIFF и нарезка XYZ PNG (порт из Flutter-пакета `geotiff_slicer`).

pub mod api;
pub mod decoder;
pub mod lzw;
pub mod mercator;
pub mod parser;
pub mod png;
pub mod slicer;

pub use api::geotiff::{
    GeoTiffError, GeoTiffSliceResult, GeoTiffZoomRangeDto, resolve_zoom_range, slice_geotiff,
};
pub use mercator::Wgs84Bounds;
pub use parser::{GeoTiffRaster, TileParsingError};
pub use slicer::{GeoTiffTileSlicer, GeoTiffZoomRange};
