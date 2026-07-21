use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::{
    rtk_base_emulator_state::RtkBaseEmulatorState,
    ubx::{Codec, Frame},
};

pub struct NavEncoder;

impl NavEncoder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn pack_ack_ack(&self, class_id: u8, message_id: u8) -> Vec<u8> {
        Codec::pack(&Frame::new(
            Codec::CLASS_ACK,
            Codec::MSG_ACK_ACK,
            vec![class_id & 0xff, message_id & 0xff],
        ))
    }

    pub fn pack_nav_pvt(&self, state: &RtkBaseEmulatorState, utc: DateTime<Utc>) -> Vec<u8> {
        let mut payload = vec![0u8; 92];

        let i_tow = Self::gps_itow_ms(utc);
        Codec::write_u4(&mut payload, 0, i_tow);
        Codec::write_u2(&mut payload, 4, utc.year() as u16);
        payload[6] = utc.month() as u8;
        payload[7] = utc.day() as u8;
        payload[8] = utc.hour() as u8;
        payload[9] = utc.minute() as u8;
        payload[10] = utc.second() as u8;
        payload[11] = 0x07; // validDate|validTime|fullyResolved
        Codec::write_u4(&mut payload, 12, 20); // tAcc ns
        Codec::write_i4(&mut payload, 16, 0); // nano
        payload[20] = 3; // 3D fix
        payload[21] = 0x01; // gnssFixOK
        payload[22] = 0;
        payload[23] = 18; // numSV

        let lat = (state.latitude * 1e7).round() as i32;
        let lon = (state.longitude * 1e7).round() as i32;
        let height_mm = (state.height_msl * 1000.0).round() as i32;
        Codec::write_i4(&mut payload, 24, lon);
        Codec::write_i4(&mut payload, 28, lat);
        Codec::write_i4(&mut payload, 32, height_mm);
        Codec::write_i4(&mut payload, 36, height_mm);

        let acc = ((state.mean_acc_meters * 1000.0).round() as i32).clamp(10, 50_000) as u32;
        Codec::write_u4(&mut payload, 40, acc);
        Codec::write_u4(&mut payload, 44, acc);
        // vel / speed / heading zeros
        Codec::write_u2(&mut payload, 76, 120); // pDOP * 100

        Codec::pack(&Frame::new(Codec::CLASS_NAV, Codec::MSG_NAV_PVT, payload))
    }

    pub fn pack_nav_svin(&self, state: &RtkBaseEmulatorState, utc: DateTime<Utc>) -> Vec<u8> {
        let mut payload = vec![0u8; 40];

        payload[0] = 0; // version
        Codec::write_u4(&mut payload, 4, Self::gps_itow_ms(utc));
        Codec::write_u4(&mut payload, 8, state.survey_dur_seconds as u32);

        let (x_cm, y_cm, z_cm, x_hp, y_hp, z_hp) = state.resolved_ecef_cm_hp();
        Codec::write_i4(&mut payload, 12, x_cm);
        Codec::write_i4(&mut payload, 16, y_cm);
        Codec::write_i4(&mut payload, 20, z_cm);
        payload[24] = (x_hp & 0xff) as u8;
        payload[25] = (y_hp & 0xff) as u8;
        payload[26] = (z_hp & 0xff) as u8;

        // meanAcc в 0.1 mm
        let mean_acc =
            ((state.mean_acc_meters * 10_000.0).round() as u64).clamp(1, u32::MAX as u64) as u32;
        Codec::write_u4(&mut payload, 28, mean_acc);
        Codec::write_u4(&mut payload, 32, state.survey_obs as u32);
        payload[36] = if state.survey_valid { 1 } else { 0 };
        payload[37] = if state.survey_active { 1 } else { 0 };

        Codec::pack(&Frame::new(Codec::CLASS_NAV, Codec::MSG_NAV_SVIN, payload))
    }

    /// Упрощённый GPS TOW в мс от начала недели (достаточно для телеметрии UI).
    fn gps_itow_ms(utc: DateTime<Utc>) -> u32 {
        // chrono/Dart weekday: Mon=1..Sun=7 → GPS: Sun=0..Sat=6
        let day_of_week = utc.weekday().number_from_monday() % 7;
        let sod = utc.hour() * 3600 + utc.minute() * 60 + utc.second();
        (day_of_week * 86_400 + sod) * 1000 + utc.timestamp_subsec_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtk_base_emulator_state::RtkBaseEmulatorState;
    use chrono::TimeZone;

    #[test]
    fn pack_ack_ack_uses_ack_class() {
        let enc = NavEncoder::new();
        let bytes = enc.pack_ack_ack(Codec::CLASS_CFG, Codec::MSG_CFG_VALSET);
        assert_eq!(bytes[2], Codec::CLASS_ACK);
        assert_eq!(bytes[3], Codec::MSG_ACK_ACK);
        assert_eq!(&bytes[6..8], &[Codec::CLASS_CFG, Codec::MSG_CFG_VALSET]);
    }

    #[test]
    fn pack_nav_pvt_has_expected_length() {
        let enc = NavEncoder::new();
        let state = RtkBaseEmulatorState::new();
        let utc = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let bytes = enc.pack_nav_pvt(&state, utc);
        // 8 header/ck + 92 payload
        assert_eq!(bytes.len(), 100);
        assert_eq!(bytes[2], Codec::CLASS_NAV);
        assert_eq!(bytes[3], Codec::MSG_NAV_PVT);
    }

    #[test]
    fn pack_nav_svin_has_expected_length() {
        let enc = NavEncoder::new();
        let state = RtkBaseEmulatorState::new();
        let utc = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let bytes = enc.pack_nav_svin(&state, utc);
        assert_eq!(bytes.len(), 48);
        assert_eq!(bytes[3], Codec::MSG_NAV_SVIN);
    }
}
