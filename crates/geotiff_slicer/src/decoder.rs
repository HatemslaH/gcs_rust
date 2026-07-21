use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::lzw::GeoTiffLzwDecoder;
use crate::parser::TileParsingError;

const COMPRESSION_LZW: u16 = 5;
const COMPRESSION_OLD_JPEG: u16 = 6;
const COMPRESSION_JPEG: u16 = 7;

pub fn decode_tile_to_rgba(
    lzw: &mut GeoTiffLzwDecoder,
    compressed_data: &[u8],
    compression: u16,
    tile_width: usize,
    tile_height: usize,
    samples_per_pixel: usize,
    predictor: u16,
    jpeg_tables: Option<&[u8]>,
) -> Result<Vec<u8>, TileParsingError> {
    match compression {
        COMPRESSION_LZW => decode_lzw_tile(
            lzw,
            compressed_data,
            tile_width,
            tile_height,
            samples_per_pixel,
            predictor,
        ),
        COMPRESSION_OLD_JPEG | COMPRESSION_JPEG => decode_jpeg_tile(
            compressed_data,
            tile_width,
            tile_height,
            samples_per_pixel,
            jpeg_tables,
        ),
        _ => Err(TileParsingError::new(
            "Неподдерживаемый формат GeoTIFF",
            format!("Сжатый тайл с compression={compression} не поддерживается."),
        )),
    }
}

fn decode_lzw_tile(
    lzw: &mut GeoTiffLzwDecoder,
    compressed_data: &[u8],
    tile_width: usize,
    tile_height: usize,
    samples_per_pixel: usize,
    predictor: u16,
) -> Result<Vec<u8>, TileParsingError> {
    let pixel_count = tile_width * tile_height;
    let mut raw = vec![0u8; pixel_count * samples_per_pixel];
    lzw.decode(compressed_data, 0, compressed_data.len(), &mut raw);

    if predictor == 2 {
        for y in 0..tile_height {
            let mut index = (y * tile_width + 1) * samples_per_pixel;
            let row_end = (y * tile_width + tile_width) * samples_per_pixel;
            while index < row_end {
                raw[index] = raw[index].wrapping_add(raw[index - samples_per_pixel]);
                index += 1;
            }
        }
    }

    if samples_per_pixel == 4 {
        return Ok(raw);
    }

    Ok(interleaved_to_rgba(&raw, pixel_count, samples_per_pixel)?)
}

fn decode_jpeg_tile(
    compressed_data: &[u8],
    tile_width: usize,
    tile_height: usize,
    samples_per_pixel: usize,
    jpeg_tables: Option<&[u8]>,
) -> Result<Vec<u8>, TileParsingError> {
    let merged = merge_jpeg_tables(compressed_data, jpeg_tables);

    // Для GeoTIFF RGBA JPEG (4 компоненты) просим сырой CMYK и инвертируем как jpeg-decoder/Dart.
    let out_space = if samples_per_pixel >= 4 {
        ColorSpace::CMYK
    } else if samples_per_pixel == 1 {
        ColorSpace::Luma
    } else {
        ColorSpace::RGB
    };
    let options = DecoderOptions::default().jpeg_set_out_colorspace(out_space);

    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(merged.as_slice()), options);
    let pixels = decoder.decode().map_err(|e| {
        TileParsingError::new(
            "Не удалось декодировать JPEG GeoTIFF",
            format!("Ошибка декодирования JPEG-тайла: {e}"),
        )
    })?;
    let info = decoder.info().ok_or_else(|| {
        TileParsingError::new(
            "Не удалось декодировать JPEG GeoTIFF",
            "Нет метаданных JPEG после decode.",
        )
    })?;

    let src_w = info.width as usize;
    let src_h = info.height as usize;
    let out_cs = decoder.output_colorspace().unwrap_or(out_space);

    let rgba = match out_cs {
        ColorSpace::CMYK if samples_per_pixel >= 4 => {
            // zune-jpeg / libjpeg-turbo отдают сырые 4 плоскости как Dart JpegData
            // (без Adobe-инверсии). jpeg-decoder требовал 255-x — здесь не нужно.
            pixels
        }
        ColorSpace::RGB => {
            let pixel_count = src_w * src_h;
            let mut rgba = vec![0u8; pixel_count * 4];
            let mut src = 0usize;
            let mut dst = 0usize;
            for _ in 0..pixel_count {
                rgba[dst] = pixels[src];
                rgba[dst + 1] = pixels[src + 1];
                rgba[dst + 2] = pixels[src + 2];
                rgba[dst + 3] = 255;
                src += 3;
                dst += 4;
            }
            rgba
        }
        ColorSpace::Luma => {
            let pixel_count = src_w * src_h;
            let mut rgba = vec![0u8; pixel_count * 4];
            for (i, &v) in pixels.iter().take(pixel_count).enumerate() {
                let dst = i * 4;
                rgba[dst] = v;
                rgba[dst + 1] = v;
                rgba[dst + 2] = v;
                rgba[dst + 3] = 255;
            }
            rgba
        }
        ColorSpace::RGBA => pixels,
        other => {
            return Err(TileParsingError::new(
                "Не удалось декодировать JPEG GeoTIFF",
                format!("Неподдерживаемый JPEG colorspace: {other:?}."),
            ));
        }
    };

    Ok(fit_rgba_to_tile(
        &rgba,
        src_w,
        src_h,
        tile_width,
        tile_height,
    ))
}

fn merge_jpeg_tables(compressed_data: &[u8], jpeg_tables: Option<&[u8]>) -> Vec<u8> {
    let Some(jpeg_tables) = jpeg_tables else {
        return compressed_data.to_vec();
    };
    if jpeg_tables.len() < 4 || compressed_data.len() < 2 {
        return compressed_data.to_vec();
    }

    let tables_inner_len = jpeg_tables.len() - 4;
    let mut merged = Vec::with_capacity(2 + tables_inner_len + compressed_data.len() - 2);
    merged.push(compressed_data[0]);
    merged.push(compressed_data[1]);
    merged.extend_from_slice(&jpeg_tables[2..jpeg_tables.len() - 2]);
    merged.extend_from_slice(&compressed_data[2..]);
    merged
}

fn fit_rgba_to_tile(
    src_rgba: &[u8],
    src_width: usize,
    src_height: usize,
    tile_width: usize,
    tile_height: usize,
) -> Vec<u8> {
    if src_width == tile_width
        && src_height == tile_height
        && src_rgba.len() >= tile_width * tile_height * 4
    {
        return src_rgba[..tile_width * tile_height * 4].to_vec();
    }

    let mut rgba = vec![0u8; tile_width * tile_height * 4];
    let copy_w = src_width.min(tile_width);
    let copy_h = src_height.min(tile_height);
    for y in 0..copy_h {
        let src_row = y * src_width * 4;
        let dst_row = y * tile_width * 4;
        rgba[dst_row..dst_row + copy_w * 4]
            .copy_from_slice(&src_rgba[src_row..src_row + copy_w * 4]);
    }
    rgba
}

fn interleaved_to_rgba(
    raw: &[u8],
    pixel_count: usize,
    samples_per_pixel: usize,
) -> Result<Vec<u8>, TileParsingError> {
    let mut rgba = vec![0u8; pixel_count * 4];

    match samples_per_pixel {
        1 => {
            let mut dst = 0;
            for i in 0..pixel_count {
                let v = raw[i];
                rgba[dst] = v;
                rgba[dst + 1] = v;
                rgba[dst + 2] = v;
                rgba[dst + 3] = 255;
                dst += 4;
            }
        }
        3 => {
            let mut src = 0;
            let mut dst = 0;
            for _ in 0..pixel_count {
                rgba[dst] = raw[src];
                rgba[dst + 1] = raw[src + 1];
                rgba[dst + 2] = raw[src + 2];
                rgba[dst + 3] = 255;
                src += 3;
                dst += 4;
            }
        }
        n if n >= 4 => {
            let mut src = 0;
            let mut dst = 0;
            for _ in 0..pixel_count {
                rgba[dst] = raw[src];
                rgba[dst + 1] = raw[src + 1];
                rgba[dst + 2] = raw[src + 2];
                rgba[dst + 3] = raw[src + 3];
                src += samples_per_pixel;
                dst += 4;
            }
        }
        _ => {
            return Err(TileParsingError::new(
                "Неподдерживаемый формат GeoTIFF",
                format!("samplesPerPixel={samples_per_pixel} не поддерживается при LZW."),
            ));
        }
    }

    Ok(rgba)
}
