# rtk_base_emulator

Эмулятор RTK-базы: TCP (UBX + RTCM), веб-панель управления.

## Публичный API

- `Emulator` / `EmulatorConfig` — старт/стоп серверов
- `RtkEmulatorMode` — режим работы
- `rtcm_api::RtcmFrameBuilder` — низкоуровневая сборка RTCM (MSM и т.п.)

Framing и CRC RTCM берутся из `rtcm3_parser` (не дублируются в этом crate).

## Запуск

Из корня workspace:

```bash
cargo run -p rtk_base_emulator
```

Панель управления и порты — см. вывод при старте / `Emulator::control_panel_url()`.
