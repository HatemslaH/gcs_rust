# gcs_rust

Низкоуровневый GCS Rust-стек: парсинг RTCM3 и эмулятор RTK-базы.

## Структура

```
crates/
  rtcm3_parser/        # инкрементальный парсер + CRC-24Q + сборка кадров
  rtk_base_emulator/   # TCP UBX/RTCM эмулятор + веб-панель
```

## Подключение извне

```toml
# только парсер
rtcm3_parser = { git = "…", package = "rtcm3_parser" }

# эмулятор (rtcm3_parser подтянется транзитивно)
rtk_base_emulator = { git = "…", package = "rtk_base_emulator" }
```

## Локальная разработка

```bash
cargo test --workspace
cargo run -p rtk_base_emulator
cargo fmt --all
```

## Roadmap

Позже: общий `ubx`-crate и optional umbrella `gcs_rust` с features — без реализации сейчас.
