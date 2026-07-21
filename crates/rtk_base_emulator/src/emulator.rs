use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::Serialize;
use thiserror::Error;

use crate::control::RTK_CONTROL_PORT;
use crate::rtk_base_emulator_state::RtkEmulatorMode;
use crate::server::{RTK_BASE_EMULATOR_PORT, RtkBaseEmulatorServer};

pub type Result<T> = std::result::Result<T, EmulatorError>;

/// Ошибки публичного API эмулятора.
#[derive(Debug, Error)]
pub enum EmulatorError {
    #[error("эмулятор уже запущен")]
    AlreadyRunning,
    #[error("эмулятор не запущен")]
    NotRunning,
    #[error("сетевая ошибка: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for EmulatorError {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Other(value.to_string())
    }
}

/// Конфигурация сетевых портов эмулятора.
#[derive(Debug, Clone)]
pub struct EmulatorConfig {
    pub bind: IpAddr,
    pub data_port: u16,
    pub control_port: u16,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            data_port: RTK_BASE_EMULATOR_PORT,
            control_port: RTK_CONTROL_PORT,
        }
    }
}

impl EmulatorConfig {
    pub fn new(bind: IpAddr, data_port: u16, control_port: u16) -> Self {
        Self {
            bind,
            data_port,
            control_port,
        }
    }

    pub fn localhost(data_port: u16, control_port: u16) -> Self {
        Self::new(IpAddr::V4(Ipv4Addr::LOCALHOST), data_port, control_port)
    }
}

/// Снимок состояния для UI / FRB (owned-поля, без lifetime).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorSnapshot {
    pub is_running: bool,
    pub status: String,
    pub mode: String,
    pub latitude: f64,
    pub longitude: f64,
    pub height_msl: f64,
    pub survey_quality: f64,
    pub mean_acc: f64,
    pub mean_acc_override: Option<f64>,
    pub min_dur: i32,
    pub acc_limit: f64,
    pub dur: i32,
    pub obs: i32,
    pub valid: bool,
    pub active: bool,
    pub force_valid: bool,
    pub force_fail: bool,
    pub client_connected: bool,
    pub log: Vec<String>,
    pub control_panel_url: String,
    pub data_endpoint: String,
}

/// Фасад эмулятора RTK-базы. Управляет жизненным циклом TCP/HTTP-серверов.
///
/// Управление сведением — через веб-панель [`Emulator::control_panel_url`]
/// или методы `set_*` / `cmd_*` (удобно для FRB).
pub struct Emulator {
    config: EmulatorConfig,
    server: RtkBaseEmulatorServer,
    is_running: bool,
}

impl Emulator {
    pub fn new(config: EmulatorConfig) -> Self {
        let data_bind = SocketAddr::new(config.bind, config.data_port);
        let control_bind = SocketAddr::new(config.bind, config.control_port);
        Self {
            server: RtkBaseEmulatorServer::new(data_bind, control_bind),
            config,
            is_running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(EmulatorError::AlreadyRunning);
        }
        self.server.start().await?;
        self.is_running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }
        self.server.stop().await;
        self.is_running = false;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn data_endpoint(&self) -> SocketAddr {
        SocketAddr::new(self.config.bind, self.config.data_port)
    }

    pub fn control_endpoint(&self) -> SocketAddr {
        SocketAddr::new(self.config.bind, self.config.control_port)
    }

    pub fn control_panel_url(&self) -> String {
        self.server.control_panel_url()
    }

    /// Актуальный снимок состояния (для FRB / native UI).
    pub async fn snapshot(&self) -> EmulatorSnapshot {
        let state = self.server.state_snapshot().await;
        EmulatorSnapshot {
            is_running: self.is_running,
            status: state.status_label().to_string(),
            mode: state.mode.as_str().to_string(),
            latitude: state.latitude,
            longitude: state.longitude,
            height_msl: state.height_msl,
            survey_quality: state.survey_quality,
            mean_acc: state.mean_acc_meters,
            mean_acc_override: state.mean_acc_override,
            min_dur: state.min_dur_seconds,
            acc_limit: state.acc_limit_meters,
            dur: state.survey_dur_seconds,
            obs: state.survey_obs,
            valid: state.survey_valid,
            active: state.survey_active,
            force_valid: state.force_valid,
            force_fail: state.force_fail,
            client_connected: state.client_connected,
            log: state.event_log.clone(),
            control_panel_url: self.control_panel_url(),
            data_endpoint: self.data_endpoint().to_string(),
        }
    }

    pub async fn set_position(&mut self, latitude: f64, longitude: f64, height_msl: f64) {
        self.server
            .with_state_mut(|s| {
                s.latitude = latitude;
                s.longitude = longitude;
                s.height_msl = height_msl;
                s.ecef_x_cm = None;
                s.ecef_y_cm = None;
                s.ecef_z_cm = None;
            })
            .await;
    }

    pub async fn set_survey_quality(&mut self, quality: f64) {
        self.server
            .with_state_mut(|s| {
                s.survey_quality = quality.clamp(0.0, 1.0);
            })
            .await;
    }

    pub async fn start_survey_in(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.start_survey_in();
            })
            .await;
    }

    pub async fn apply_fixed_mode(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.apply_fixed_mode();
            })
            .await;
    }

    pub async fn disable_mode(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.disable_mode();
            })
            .await;
    }

    pub async fn force_valid(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.force_fail = false;
                s.force_valid = true;
                s.survey_valid = true;
                s.survey_active = false;
                if s.mode == RtkEmulatorMode::Disabled {
                    s.mode = RtkEmulatorMode::SurveyIn;
                }
                s.add_log("Force valid с API");
            })
            .await;
    }

    pub async fn force_fail(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.force_valid = false;
                s.force_fail = true;
                s.survey_valid = false;
                s.add_log("Force fail с API");
            })
            .await;
    }

    pub async fn clear_force(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.force_valid = false;
                s.force_fail = false;
                s.add_log("Force-флаги сняты");
            })
            .await;
    }

    pub async fn reset_survey(&mut self) {
        self.server
            .with_state_mut(|s| {
                s.reset_survey();
            })
            .await;
    }
}
