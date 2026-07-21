//! Эмулятор RTK-базы (UBX + RTCM + веб-панель).
//!
//! Подключается как обычная Rust-библиотека или через Flutter Rust Bridge.
//!
//! # Пример
//!
//! ```no_run
//! use rtk_base_emulator::{Emulator, EmulatorConfig};
//!
//! #[tokio::main]
//! async fn main() -> rtk_base_emulator::Result<()> {
//!     let mut emu = Emulator::new(EmulatorConfig::default());
//!     emu.start().await?;
//!     println!("{}", emu.control_panel_url());
//!     emu.stop().await;
//!     Ok(())
//! }
//! ```

mod control;
mod emulator;
mod rtcm;
mod rtk_base_emulator_state;
mod server;
pub mod ubx;

pub use control::RTK_CONTROL_PORT;
pub use emulator::{Emulator, EmulatorConfig, EmulatorError, EmulatorSnapshot, Result};
pub use rtk_base_emulator_state::RtkEmulatorMode;
pub use server::RTK_BASE_EMULATOR_PORT;

/// Низкоуровневая сборка RTCM3-кадров (для тестов и расширенных сценариев).
pub mod rtcm_api {
    pub use crate::rtcm::RtcmFrameBuilder;
}
