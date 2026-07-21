# rtcm3_parser

Инкрементальный парсер кадров RTCM3.

## Возможности

- `Parser` — накопление байт потока и извлечение кадров с валидным CRC-24Q
- `Crc24q` — публичный расчёт контрольной суммы
- `Frame::from_payload` — сборка кадра (`D3` + 10-bit len + payload + CRC)

## Пример

```rust
use rtcm3_parser::{Frame, Parser};

let frame = Frame::from_payload(&[0x3E, 0xD0]).unwrap();
let parsed = Parser::new().add_data(frame.as_bytes());
assert_eq!(parsed[0].message_number(), Some(1005));
```
