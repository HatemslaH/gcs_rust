use crate::ubx::{Codec, Frame};

pub struct StreamParser {
    buf: Vec<u8>,
}

impl StreamParser {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn add_data(&mut self, data: &[u8]) -> Vec<Frame> {
        self.buf.extend_from_slice(data);
        let mut frames = Vec::new();

        while self.buf.len() >= 8 {
            let Some(sync1_idx) = self.buf.iter().position(|&x| x == Codec::SYNC1) else {
                self.buf.clear();
                break;
            };

            if sync1_idx > 0 {
                self.buf.drain(0..sync1_idx);
            }

            if self.buf.len() < 2 {
                break;
            }

            if self.buf[1] != Codec::SYNC2 {
                self.buf.remove(0);
                continue;
            }

            if self.buf.len() < 6 {
                break;
            }

            let payload_len = self.buf[4] as usize + ((self.buf[5] as usize) << 8);
            let total = 8 + payload_len;

            if self.buf.len() < total {
                break;
            }

            let mut ck_a: u8 = 0;
            let mut ck_b: u8 = 0;
            for byte in &self.buf[2..payload_len + 6] {
                ck_a = ck_a.wrapping_add(*byte);
                ck_b = ck_b.wrapping_add(ck_a);
            }

            if ck_a != self.buf[payload_len + 6] || ck_b != self.buf[payload_len + 7] {
                self.buf.remove(0);
                continue;
            }

            frames.push(Frame::new(
                self.buf[2],
                self.buf[3],
                self.buf[6..payload_len + 6].to_vec(),
            ));
            self.buf.drain(0..total);
        }

        frames
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_frame_and_ignores_noise() {
        let frame = Codec::pack(&Frame::new(0x05, 0x01, vec![0x06, 0x8a]));
        let mut noisy = vec![0x00, 0xff];
        noisy.extend_from_slice(&frame);
        noisy.push(0x11);

        let mut parser = StreamParser::new();
        let frames = parser.add_data(&noisy);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].class_id, 0x05);
        assert_eq!(frames[0].message_id, 0x01);
        assert_eq!(frames[0].payload, vec![0x06, 0x8a]);
    }

    #[test]
    fn waits_for_complete_frame() {
        let packed = Codec::pack(&Frame::new(0x01, 0x07, vec![9, 8, 7]));
        let mut parser = StreamParser::new();
        assert!(parser.add_data(&packed[..4]).is_empty());
        let frames = parser.add_data(&packed[4..]);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, vec![9, 8, 7]);
    }
}
