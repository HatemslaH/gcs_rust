use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct MercatorTileIndex {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Wgs84Bounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

pub const TILE_SIZE: usize = 256;

pub fn tiles_for_bounds(bounds: Wgs84Bounds, zoom: u32) -> Vec<MercatorTileIndex> {
    let n = 1u32 << zoom;
    let x_min = lon_to_tile_x(bounds.west, zoom).clamp(0, n - 1);
    let x_max = lon_to_tile_x(bounds.east, zoom).clamp(0, n - 1);
    let y_min = lat_to_tile_y(bounds.north, zoom).clamp(0, n - 1);
    let y_max = lat_to_tile_y(bounds.south, zoom).clamp(0, n - 1);

    let mut tiles = Vec::new();
    for x in x_min..=x_max {
        for y in y_min..=y_max {
            tiles.push(MercatorTileIndex { z: zoom, x, y });
        }
    }
    tiles
}

pub fn count_tiles_for_zoom_range(bounds: Wgs84Bounds, min_zoom: u32, max_zoom: u32) -> usize {
    let mut total = 0usize;
    for z in min_zoom..=max_zoom {
        total += tiles_for_bounds(bounds, z).len();
    }
    total
}

pub fn compute_max_zoom(bounds: Wgs84Bounds, image_width: u32, image_height: u32) -> u32 {
    if image_width == 0 || image_height == 0 {
        return 0;
    }

    let deg_per_pixel_lon = (bounds.east - bounds.west) / image_width as f64;
    let deg_per_pixel_lat = (bounds.north - bounds.south) / image_height as f64;
    let deg_per_pixel = deg_per_pixel_lon.min(deg_per_pixel_lat);

    let zoom = ((360.0 / (TILE_SIZE as f64 * deg_per_pixel)).ln() / 2.0_f64.ln()).floor() as i32;
    zoom.clamp(0, 22) as u32
}

fn lon_to_tile_x(lon: f64, zoom: u32) -> u32 {
    ((lon + 180.0) / 360.0 * (1u32 << zoom) as f64).floor() as u32
}

fn lat_to_tile_y(lat: f64, zoom: u32) -> u32 {
    let lat_rad = lat * PI / 180.0;
    ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0 * (1u32 << zoom) as f64).floor()
        as u32
}
