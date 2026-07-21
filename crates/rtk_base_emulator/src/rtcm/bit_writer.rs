/// Побитовая запись MSB-first в байтовый буфер (как Dart `_BitWriter`).
pub struct BitWriter {
    bytes: Vec<u8>,
    bit_pos: u8,
    current: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_pos: 0,
            current: 0,
        }
    }

    pub fn write(&mut self, value: u64, n_bits: u32) {
        debug_assert!(n_bits <= 64);
        let mask = if n_bits == 64 {
            u64::MAX
        } else {
            (1u64 << n_bits) - 1
        };
        self.write_bits((value & mask) as u128, n_bits);
    }

    pub fn write_bits(&mut self, value: u128, n_bits: u32) {
        let mask = if n_bits == 128 {
            u128::MAX
        } else {
            (1u128 << n_bits) - 1
        };
        let v = value & mask;
        for i in (0..n_bits).rev() {
            let bit = ((v >> i) & 1) as u8;
            self.current = (self.current << 1) | bit;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.bit_pos = 0;
            }
        }
    }

    pub fn to_bytes(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.bytes.push((self.current << (8 - self.bit_pos)) & 0xff);
        }
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_byte_aligned_value() {
        let mut w = BitWriter::new();
        w.write(0xAB, 8);
        assert_eq!(w.to_bytes(), vec![0xAB]);
    }

    #[test]
    fn packs_msb_first_across_bytes() {
        let mut w = BitWriter::new();
        w.write(0b1011, 4);
        w.write(0b0001, 4);
        assert_eq!(w.to_bytes(), vec![0b1011_0001]);
    }

    #[test]
    fn pads_remaining_bits() {
        let mut w = BitWriter::new();
        w.write(0b101, 3);
        assert_eq!(w.to_bytes(), vec![0b1010_0000]);
    }
}
