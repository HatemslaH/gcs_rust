use chrono::Local;
use serde_json::json;

use crate::ubx::Ecef;

/// Режим приёмника эмулятора RTK-базы.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RtkEmulatorMode {
    Disabled,
    SurveyIn,
    Fixed,
}

impl RtkEmulatorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::SurveyIn => "surveyIn",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RtkBaseEmulatorState {
    /// Качество сведения 0..1 (1 — быстро сходится).
    pub survey_quality: f64,

    /// Широта/долгота/высота MSL базы (градусы / метры) — для NAV-PVT.
    pub latitude: f64,
    pub longitude: f64,
    pub height_msl: f64,

    /// Явно заданные ECEF (см + HP 0.1 mm). `None` — считать из LLH.
    pub ecef_x_cm: Option<i32>,
    pub ecef_y_cm: Option<i32>,
    pub ecef_z_cm: Option<i32>,
    pub ecef_x_hp: i32,
    pub ecef_y_hp: i32,
    pub ecef_z_hp: i32,
    pub survey_dur_seconds: i32,
    pub survey_obs: i32,

    pub lat_hp: i32,
    pub lon_hp: i32,
    pub height_hp: i32,

    pub mode: RtkEmulatorMode,

    pub min_dur_seconds: i32,
    pub acc_limit_meters: f64,

    /// Средняя точность сведения, метры.
    pub mean_acc_meters: f64,
    pub survey_valid: bool,
    pub survey_active: bool,
    pub mean_acc_override: Option<f64>,
    pub force_valid: bool,
    pub force_fail: bool,
    pub client_connected: bool,

    pub event_log: Vec<String>,
}

impl RtkBaseEmulatorState {
    const MAX_LOG_ENTRIES: usize = 40;

    pub fn new() -> Self {
        Self {
            survey_quality: 0.7,
            latitude: 55.7558,
            longitude: 37.6173,
            height_msl: 150.0,
            ecef_x_cm: None,
            ecef_y_cm: None,
            ecef_z_cm: None,
            ecef_x_hp: 0,
            ecef_y_hp: 0,
            ecef_z_hp: 0,
            survey_dur_seconds: 0,
            survey_obs: 0,
            lat_hp: 0,
            lon_hp: 0,
            height_hp: 0,
            mode: RtkEmulatorMode::Disabled,
            min_dur_seconds: 60,
            acc_limit_meters: 5.0,
            mean_acc_meters: 5.0,
            survey_valid: false,
            survey_active: false,
            mean_acc_override: None,
            force_valid: false,
            force_fail: false,
            client_connected: false,
            event_log: Vec::new(),
        }
    }

    pub fn add_log(&mut self, message: &str) {
        let ts = Local::now().format("%H:%M:%S");
        let entry = format!("[{ts}] {message}");
        self.event_log.insert(0, entry);
        if self.event_log.len() > Self::MAX_LOG_ENTRIES {
            self.event_log.pop();
        }
    }

    pub fn should_emit_rtcm(&self) -> bool {
        (self.survey_valid || self.mode == RtkEmulatorMode::Fixed) && !self.force_fail
    }

    pub fn status_label(&self) -> &str {
        if !self.client_connected {
            return "waiting_client";
        }

        if self.force_fail {
            return "forced_fail";
        }

        if self.mode == RtkEmulatorMode::Fixed {
            return "fixed";
        }

        if self.survey_valid {
            return "valid";
        }

        if self.survey_active {
            return "surveying";
        }

        "idle"
    }

    pub fn to_panel_json_string(&self) -> String {
        json!({
            "type": "state",
            "status": self.status_label(),
            "clientConnected": self.client_connected,
            "mode": self.mode.as_str(),
            "latitude": self.latitude,
            "longitude": self.longitude,
            "heightMsl": self.height_msl,
            "surveyQuality": self.survey_quality,
            "meanAcc": self.mean_acc_meters,
            "meanAccOverride": self.mean_acc_override,
            "minDur": self.min_dur_seconds,
            "accLimit": self.acc_limit_meters,
            "dur": self.survey_dur_seconds,
            "obs": self.survey_obs,
            "valid": self.survey_valid,
            "active": self.survey_active,
            "forceValid": self.force_valid,
            "forceFail": self.force_fail,
            "log": self.event_log,
        })
        .to_string()
    }

    pub fn resolved_ecef_cm_hp(&self) -> (i32, i32, i32, i32, i32, i32) {
        if self.ecef_x_cm.is_some() && self.ecef_y_cm.is_some() && self.ecef_z_cm.is_some() {
            return (
                self.ecef_x_cm.unwrap(),
                self.ecef_y_cm.unwrap(),
                self.ecef_z_cm.unwrap(),
                self.ecef_x_hp,
                self.ecef_y_hp,
                self.ecef_z_hp,
            );
        }

        let ecef = Ecef::from_llh(self.latitude, self.longitude, self.height_msl);
        let x = Ecef::split_meters_to_cm_hp(ecef.x);
        let y = Ecef::split_meters_to_cm_hp(ecef.y);
        let z = Ecef::split_meters_to_cm_hp(ecef.z);

        (
            x.0 as i32, y.0 as i32, z.0 as i32, x.1 as i32, y.1 as i32, z.1 as i32,
        )
    }

    pub fn sync_llh_from_ecef(&mut self) {
        if self.ecef_x_cm.is_none() || self.ecef_y_cm.is_none() || self.ecef_z_cm.is_none() {
            return;
        }

        let x = self.ecef_x_cm.unwrap() as f64 * 0.01 + self.ecef_x_hp as f64 * 0.0001;
        let y = self.ecef_y_cm.unwrap() as f64 * 0.01 + self.ecef_y_hp as f64 * 0.0001;
        let z = self.ecef_z_cm.unwrap() as f64 * 0.01 + self.ecef_z_hp as f64 * 0.0001;

        let llh = Ecef::new(x, y, z).to_llh();

        self.latitude = llh.0;
        self.longitude = llh.1;
        self.height_msl = llh.2;
    }

    pub fn tick_survey(&mut self) {
        if self.mode != RtkEmulatorMode::SurveyIn || self.force_fail {
            return;
        }

        self.survey_active = !self.survey_valid;
        if self.survey_valid && !self.force_valid {
            return;
        }

        self.survey_dur_seconds += 1;
        let quality = self.survey_quality.clamp(0.0, 1.0);
        let obs_delta = 1.max((8.0 + quality * 24.0).round() as i32);
        self.survey_obs += obs_delta;

        if let Some(override_acc) = self.mean_acc_override {
            self.mean_acc_meters = override_acc;
        } else {
            let floor = 0.01f64.max(self.acc_limit_meters * 0.4);
            let decay = 0.04 + quality * 0.22;
            self.mean_acc_meters =
                floor.max(self.mean_acc_meters * (1.0 - decay) + floor * decay * 0.15);
        }

        if self.force_valid
            || (self.survey_dur_seconds >= self.min_dur_seconds
                && self.mean_acc_meters <= self.acc_limit_meters)
        {
            self.survey_valid = true;
            self.survey_active = false;
            self.add_log(&format!(
                "Сведение успешно (valid=true, dur={}s, meanAcc={:.3}m)",
                self.survey_dur_seconds, self.mean_acc_meters,
            ));
        }
    }

    pub fn start_survey_in(&mut self) {
        self.mode = RtkEmulatorMode::SurveyIn;
        self.survey_valid = false;
        self.survey_active = true;
        self.survey_dur_seconds = 0;
        self.survey_obs = 0;
        self.force_valid = false;
        self.force_fail = false;
        self.ecef_x_cm = None;
        self.ecef_y_cm = None;
        self.ecef_z_cm = None;
        self.ecef_x_hp = 0;
        self.ecef_y_hp = 0;
        self.ecef_z_hp = 0;

        if self.mean_acc_override.is_none() {
            self.mean_acc_meters = (self.acc_limit_meters * 2.5).max(2.0);
        }

        self.add_log(&format!(
            "Команда сведения (SURVEY_IN), minDur={}s, accLimit={:.2}m",
            self.min_dur_seconds, self.acc_limit_meters,
        ));
    }

    pub fn disable_mode(&mut self) {
        self.mode = RtkEmulatorMode::Disabled;
        self.survey_valid = false;
        self.survey_active = false;
        self.survey_dur_seconds = 0;
        self.survey_obs = 0;
        self.force_valid = false;
        self.force_fail = false;
        self.add_log("Режим DISABLED");
    }

    pub fn apply_fixed_mode(&mut self) {
        self.mode = RtkEmulatorMode::Fixed;
        self.survey_active = false;
        self.survey_valid = false;
        self.force_valid = false;
        self.force_fail = false;
        self.sync_llh_from_ecef();
        self.add_log("Режим FIXED применён");
    }

    pub fn reset_survey(&mut self) {
        self.survey_valid = false;
        self.survey_active = self.mode == RtkEmulatorMode::SurveyIn;
        self.survey_dur_seconds = 0;
        self.survey_obs = 0;
        self.force_valid = false;
        self.force_fail = false;

        if self.mean_acc_override.is_none() {
            self.mean_acc_meters = (self.acc_limit_meters * 2.5).max(2.0);
        }

        self.add_log("Сброс сведения");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_emit_rtcm_requires_valid_or_fixed() {
        let mut s = RtkBaseEmulatorState::new();
        assert!(!s.should_emit_rtcm());
        s.survey_valid = true;
        assert!(s.should_emit_rtcm());
        s.force_fail = true;
        assert!(!s.should_emit_rtcm());
        s.force_fail = false;
        s.survey_valid = false;
        s.mode = RtkEmulatorMode::Fixed;
        assert!(s.should_emit_rtcm());
    }

    #[test]
    fn tick_survey_progresses_and_validates() {
        let mut s = RtkBaseEmulatorState::new();
        s.start_survey_in();
        s.min_dur_seconds = 2;
        s.acc_limit_meters = 100.0;
        s.mean_acc_meters = 1.0;
        s.tick_survey();
        assert_eq!(s.survey_dur_seconds, 1);
        assert!(s.survey_obs > 0);
        s.tick_survey();
        assert!(s.survey_valid);
        assert!(!s.survey_active);
    }

    #[test]
    fn panel_json_contains_type_state() {
        let s = RtkBaseEmulatorState::new();
        let json = s.to_panel_json_string();
        assert!(json.contains("\"type\":\"state\""));
        assert!(json.contains("\"mode\":\"disabled\""));
    }
}
