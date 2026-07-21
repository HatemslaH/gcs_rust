//! Инкрементальный парсер кадров RTCM3.
//!
//! Принимает поток байт (в том числе фрагментированный) и возвращает
//! полные кадры с корректным CRC-24Q.

mod crc24q;
mod frame;
mod parser;

pub use crc24q::Crc24q;
pub use frame::{Frame, FrameError};
pub use parser::Parser;
