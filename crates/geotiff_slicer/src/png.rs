use crc32fast::Hasher;

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const IHDR: [u8; 4] = [73, 72, 68, 82];
const IDAT: [u8; 4] = [73, 68, 65, 84];
const IEND: [u8; 4] = [73, 69, 78, 68];

/// Быстрый PNG-энкодер (filter=None, zlib store) с переиспользуемыми буферами.
pub struct FastPngEncoder {
    raw: Vec<u8>,
    zlib: Vec<u8>,
    out: Vec<u8>,
}

impl FastPngEncoder {
    pub fn new() -> Self {
        Self {
            raw: Vec::new(),
            zlib: Vec::new(),
            out: Vec::new(),
        }
    }

    /// Кодирует RGBA; возвращает срез внутреннего буфера (валиден до следующего encode).
    pub fn encode_rgba(&mut self, rgba: &[u8], width: usize, height: usize) -> &[u8] {
        assert!(rgba.len() >= width * height * 4);

        let row_bytes = width * 4;
        let raw_len = height * (1 + row_bytes);
        self.raw.clear();
        self.raw.reserve(raw_len);

        let mut src = 0usize;
        for _ in 0..height {
            self.raw.push(0);
            self.raw.extend_from_slice(&rgba[src..src + row_bytes]);
            src += row_bytes;
        }

        let zlib_cap = raw_len + raw_len / 10 + 64;
        self.zlib.clear();
        self.zlib.resize(zlib_cap, 0);
        let zlib_len = write_zlib_store(&self.raw, &mut self.zlib);
        self.zlib.truncate(zlib_len);

        let mut ihdr = [0u8; 13];
        ihdr[0..4].copy_from_slice(&(width as u32).to_be_bytes());
        ihdr[4..8].copy_from_slice(&(height as u32).to_be_bytes());
        ihdr[8] = 8;
        ihdr[9] = 6;

        self.out.clear();
        self.out.reserve(8 + 12 + 13 + 12 + zlib_len + 12);
        self.out.extend_from_slice(&PNG_SIGNATURE);
        write_chunk(&mut self.out, &IHDR, &ihdr);
        write_chunk(&mut self.out, &IDAT, &self.zlib);
        write_chunk(&mut self.out, &IEND, &[]);
        &self.out
    }
}

impl Default for FastPngEncoder {
    fn default() -> Self {
        Self::new()
    }
}

fn write_zlib_store(src: &[u8], dst: &mut [u8]) -> usize {
    let mut pos = 0usize;
    dst[pos] = 0x78;
    dst[pos + 1] = 0x01;
    pos += 2;

    let mut offset = 0usize;
    while offset < src.len() {
        let mut block_len = src.len() - offset;
        if block_len > 65535 {
            block_len = 65535;
        }
        let is_final = offset + block_len >= src.len();
        dst[pos] = if is_final { 0x01 } else { 0x00 };
        dst[pos + 1] = (block_len & 0xff) as u8;
        dst[pos + 2] = ((block_len >> 8) & 0xff) as u8;
        let nlen = !(block_len as u16);
        dst[pos + 3] = (nlen & 0xff) as u8;
        dst[pos + 4] = ((nlen >> 8) & 0xff) as u8;
        pos += 5;
        dst[pos..pos + block_len].copy_from_slice(&src[offset..offset + block_len]);
        pos += block_len;
        offset += block_len;
    }

    let adler = adler32(src);
    dst[pos..pos + 4].copy_from_slice(&adler.to_be_bytes());
    pos + 4
}

fn adler32(data: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

fn write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut hasher = Hasher::new();
    hasher.update(chunk_type);
    hasher.update(data);
    out.extend_from_slice(&hasher.finalize().to_be_bytes());
}
