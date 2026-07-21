use crate::Parser;
use crate::crc24q::Crc24q;

/// Ошибка сборки кадра RTCM3 из payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Длина payload превышает 10-битный лимит (1023 байт).
    PayloadTooLong { len: usize, max: usize },
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooLong { len, max } => {
                write!(f, "RTCM3 payload too long: {len} > {max}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Полный кадр RTCM3: preamble + заголовок длины + payload + CRC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    bytes: Vec<u8>,
}

impl Frame {
    /// Создаёт кадр из уже проверенных сырых байт.
    ///
    /// Вызывающий код обязан гарантировать, что `bytes` — валидный кадр RTCM3.
    pub(crate) fn from_validated(bytes: Vec<u8>) -> Self {
        debug_assert!(bytes.len() >= 6);
        debug_assert_eq!(bytes[0], Parser::PREAMBLE);
        Self { bytes }
    }

    /// Собирает кадр RTCM3 из payload: `D3` + 10-bit len + payload + CRC-24Q.
    pub fn from_payload(payload: &[u8]) -> Result<Self, FrameError> {
        let len = payload.len();
        if len > Parser::MAX_PAYLOAD_LEN {
            return Err(FrameError::PayloadTooLong {
                len,
                max: Parser::MAX_PAYLOAD_LEN,
            });
        }

        let mut bytes = Vec::with_capacity(3 + len + 3);
        bytes.push(Parser::PREAMBLE);
        bytes.push(((len >> 8) & 0x03) as u8);
        bytes.push((len & 0xFF) as u8);
        bytes.extend_from_slice(payload);

        let crc = Crc24q::calculate(&bytes, 3 + len, 0);
        bytes.push(((crc >> 16) & 0xFF) as u8);
        bytes.push(((crc >> 8) & 0xFF) as u8);
        bytes.push((crc & 0xFF) as u8);

        Ok(Self::from_validated(bytes))
    }

    /// Сырые байты кадра целиком (включая preamble и CRC).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Владение сырыми байтами кадра.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Длина payload в байтах (10-битное поле из заголовка).
    pub fn payload_len(&self) -> usize {
        (((self.bytes[1] as usize) & 0x03) << 8) | (self.bytes[2] as usize)
    }

    /// Payload без заголовка и CRC.
    pub fn payload(&self) -> &[u8] {
        let len = self.payload_len();
        &self.bytes[3..3 + len]
    }

    /// Номер сообщения RTCM (первые 12 бит payload), если payload ≥ 2 байт.
    pub fn message_number(&self) -> Option<u16> {
        let payload = self.payload();
        if payload.len() < 2 {
            return None;
        }
        Some(((payload[0] as u16) << 4) | ((payload[1] as u16) >> 4))
    }
}

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::Frame;
    use crate::Parser;

    #[test]
    fn from_payload_roundtrip_through_parser() {
        let payload = [0x3E, 0xD0]; // message type 1005
        let frame = Frame::from_payload(&payload).unwrap();

        let parsed = Parser::new().add_data(frame.as_bytes());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].payload(), &payload);
        assert_eq!(parsed[0].as_bytes(), frame.as_bytes());
        assert_eq!(parsed[0].message_number(), Some(1005));
    }

    #[test]
    fn from_payload_rejects_too_long() {
        let payload = vec![0u8; Parser::MAX_PAYLOAD_LEN + 1];
        assert!(Frame::from_payload(&payload).is_err());
    }
}
