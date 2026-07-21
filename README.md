# gcs_rust

Низкоуровневый GCS Rust-стек: RTCM3, UBX (u-blox), GeoTIFF → XYZ PNG и эмулятор RTK-базы.

Лицензия: MIT.

## Структура

```
crates/
  rtcm3_parser/        # инкрементальный парсер + CRC-24Q + сборка кадров
  ublox_ubx_parser/    # UBX: C FFI (u-blox-bg) + bindgen, PVT/SVIN/ACK, CFG pack
  geotiff_slicer/      # парсинг GeoTIFF + нарезка XYZ PNG (Web Mercator)
  rtk_base_emulator/   # TCP UBX/RTCM эмулятор + веб-панель
```

Подробности — в README каждого crate.

## Подключение извне

```toml
rtcm3_parser = { git = "https://github.com/HatemslaH/gcs_rust.git", package = "rtcm3_parser" }

ublox_ubx_parser = { git = "https://github.com/HatemslaH/gcs_rust.git", package = "ublox_ubx_parser" }

geotiff_slicer = { git = "https://github.com/HatemslaH/gcs_rust.git", package = "geotiff_slicer" }

# эмулятор (rtcm3_parser подтянется транзитивно)
rtk_base_emulator = { git = "https://github.com/HatemslaH/gcs_rust.git", package = "rtk_base_emulator" }
```

Для `ublox_ubx_parser` Cargo подтянет git-submodule `u-blox-bg` только если клон репозитория сделан с `--recurse-submodules` (или submodule инициализирован вручную). Нужны C-компилятор и libclang (bindgen).

## Локальная разработка

```bash
git clone --recurse-submodules https://github.com/HatemslaH/gcs_rust.git
# или в уже клонированном репо:
git submodule update --init --recursive

cargo fmt --all
cargo test --workspace
cargo run -p rtk_base_emulator
```

Windows: для bindgen обычно достаточно LLVM (`C:\Program Files\LLVM\bin`, либо `LIBCLANG_PATH`).
Linux: `libclang-dev`.
