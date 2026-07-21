# ublox_ubx_parser

Rust-обёртка над C-парсером UBX (`u-blox-bg` + `ubx_parser.c`), порт Dart-пакета `ublox_ubx_parser`.

## Возможности

- `UbxParser::add_data` — инкрементальный разбор потока (NAV-PVT, NAV-SVIN, ACK/NAK)
- `RtkBaseState` — последний PVT/SVIN снимок
- `pack_ubx_*` — сборка CFG-VALSET / VALDEL / RST
- `UbxKeys` — CFG-ключи TMODE / MSGOUT

Нативный код: `native/ubx_parser.c` + submodule [`u-blox-bg`](https://github.com/boris-gu/u-blox-bg.git).
Сборка C через `cc`, FFI и CFG-ключи — через `bindgen` из заголовков (источник правды — C).

```bash
git submodule update --init --recursive
cargo test -p ublox_ubx_parser
```
