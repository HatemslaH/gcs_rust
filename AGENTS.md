# AGENTS.md — инструкции для ИИ/агентов

## Workspace layout

```
gcs_rust/
  crates/rtcm3_parser/       # протокол RTCM3: parse, CRC-24Q, Frame::from_payload
  crates/rtk_base_emulator/  # бизнес-логика эмулятора: UBX, MSM, TCP, панель
```

Новые crates класть в `crates/<name>` и добавлять в корневой `[workspace].members`.

## Границы ответственности

- **Протокол RTCM** (framing, preamble, длина, CRC) — только в `rtcm3_parser`.
- **MSM / сообщения / эмуляция базы** — в `rtk_base_emulator`.
- **Не дублировать CRC-24Q** и сборку заголовка кадра в emulator или других crates — использовать `rtcm3_parser::Crc24q` / `Frame::from_payload`.

Зависимости между crates workspace — через `[workspace.dependencies]` в корневом `Cargo.toml`.

## Проверка

```bash
cargo fmt --all
cargo test --workspace
cargo run -p rtk_base_emulator
```

## Язык

Комментарии и доменные сообщения/логи — на русском (как в существующем коде).
Идентификаторы кода (типы, функции, модули) — на английском.
