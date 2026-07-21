# geotiff_slicer

Парсинг GeoTIFF и нарезка XYZ PNG-тайлов (Web Mercator). Порт Rust-логики из Flutter-пакета `geotiff_slicer` — без flutter_rust_bridge.

## API

```rust
use geotiff_slicer::{resolve_zoom_range, slice_geotiff, GeoTiffTileSlicer};

let zoom = resolve_zoom_range("map.tif".into())?;
let result = slice_geotiff("map.tif".into(), "tiles".into(), Some(zoom.min_zoom), Some(zoom.max_zoom))?;
```

Или напрямую:

```rust
let written = GeoTiffTileSlicer::slice("map.tif", "tiles", min_zoom, max_zoom)?;
```

## Зависимости

`zune-jpeg`, `rayon`, `dashmap`, `crc32fast` — как в исходном crate.
