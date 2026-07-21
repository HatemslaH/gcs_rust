use crate::crc24q::Crc24q;
use crate::frame::Frame;

/// Инкрементальный парсер потока RTCM3.
///
/// Накапливает входящие байты и извлекает целые кадры с валидным CRC-24Q.
#[derive(Debug, Default, Clone)]
pub struct Parser {
    buffer: Vec<u8>,
}

impl Parser {
    /// Preamble RTCM3.
    pub const PREAMBLE: u8 = 0xD3;

    /// Минимальный размер кадра: заголовок (3) + CRC (3).
    pub const MIN_FRAME_LEN: usize = 6;

    /// Максимальная длина payload по спецификации (10 бит).
    pub const MAX_PAYLOAD_LEN: usize = 1023;

    pub fn new() -> Self {
        Self::default()
    }

    /// Добавляет данные и возвращает список целых RTCM3-кадров
    /// (включая заголовок и CRC) с корректной контрольной суммой.
    ///
    /// Неполные кадры остаются во внутреннем буфере до следующих вызовов.
    /// Байты до preamble и кадры с неверным CRC отбрасываются.
    pub fn add_data(&mut self, data: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(data);
        let mut frames = Vec::new();

        while self.buffer.len() >= 3 {
            if self.buffer[0] != Self::PREAMBLE {
                self.buffer.remove(0);
                continue;
            }

            // Byte 1: RRRRRR LL — reserved (6 бит) + старшие биты длины.
            // Byte 2: LLLLLLLL — младшие 8 бит длины.
            // Reserved-биты должны быть нулевыми.
            if self.buffer[1] & 0xFC != 0 {
                self.buffer.remove(0);
                continue;
            }

            let payload_len = (((self.buffer[1] as usize) & 0x03) << 8) | (self.buffer[2] as usize);
            let total_len = 3 + payload_len + 3;

            if self.buffer.len() < total_len {
                break;
            }

            let frame_bytes = &self.buffer[..total_len];
            let received_crc = ((frame_bytes[total_len - 3] as u32) << 16)
                | ((frame_bytes[total_len - 2] as u32) << 8)
                | (frame_bytes[total_len - 1] as u32);

            let calculated_crc = Crc24q::calculate(frame_bytes, 3 + payload_len, 0);

            if received_crc == calculated_crc {
                let frame = self.buffer.drain(..total_len).collect::<Vec<u8>>();
                frames.push(Frame::from_validated(frame));
            } else {
                // CRC неверный — сдвигаемся на 1 байт и ищем следующий preamble.
                self.buffer.remove(0);
            }
        }

        frames
    }

    /// Количество байт, ожидающих завершения кадра.
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// Очищает внутренний буфер.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
