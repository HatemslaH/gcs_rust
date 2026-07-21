use crate::{
    rtk_base_emulator_state::RtkBaseEmulatorState,
    ubx::{Codec, Frame, NavEncoder, RtkEmulatorUbxKeys},
};

/// Обработка входящих CFG-VALSET и формирование ACK.
pub struct CfgHandler<S, C = fn()>
where
    S: FnMut(&[u8]),
    C: FnMut(),
{
    send_bytes: S,
    on_state_changed: Option<C>,
    nav_encoder: NavEncoder,
}

impl<S, C> CfgHandler<S, C>
where
    S: FnMut(&[u8]),
    C: FnMut(),
{
    pub fn new(send_bytes: S, on_state_changed: Option<C>) -> Self {
        Self {
            send_bytes,
            on_state_changed,
            nav_encoder: NavEncoder::new(),
        }
    }

    pub fn handle_frame(&mut self, state: &mut RtkBaseEmulatorState, frame: &Frame) {
        if frame.class_id != Codec::CLASS_CFG || frame.message_id != Codec::MSG_CFG_VALSET {
            return;
        }

        Self::apply_valset(state, &frame.payload);
        let ack = self
            .nav_encoder
            .pack_ack_ack(frame.class_id, frame.message_id);
        (self.send_bytes)(&ack);
        if let Some(cb) = self.on_state_changed.as_mut() {
            cb();
        }
    }

    fn read_value(bd: &[u8], offset: usize, size: usize, signed: bool) -> i64 {
        match size {
            1 => {
                if signed {
                    bd.get(offset).copied().unwrap_or(0) as i8 as i64
                } else {
                    bd.get(offset).copied().unwrap_or(0) as i64
                }
            }
            2 => {
                let Some(bytes) = bd.get(offset..offset + 2) else {
                    return 0;
                };
                let arr: [u8; 2] = bytes.try_into().unwrap_or([0, 0]);
                if signed {
                    i16::from_le_bytes(arr) as i64
                } else {
                    u16::from_le_bytes(arr) as i64
                }
            }
            4 | 8 => {
                // Как в Dart: для size 8 читаем только 4 байта LE.
                let Some(bytes) = bd.get(offset..offset + 4) else {
                    return 0;
                };
                let arr: [u8; 4] = bytes.try_into().unwrap_or([0, 0, 0, 0]);
                if signed {
                    i32::from_le_bytes(arr) as i64
                } else {
                    u32::from_le_bytes(arr) as i64
                }
            }
            _ => 0,
        }
    }

    fn apply_valset(state: &mut RtkBaseEmulatorState, payload: &[u8]) {
        if payload.len() < 8 {
            return;
        }

        let mut offset = 4;
        while offset + 4 <= payload.len() {
            let Some(key) = Codec::read_u4(payload, offset) else {
                break;
            };
            offset += 4;
            let value_size = Codec::value_size_from_key(key);
            if offset + value_size > payload.len() {
                break;
            }

            let signed = matches!(
                key,
                RtkEmulatorUbxKeys::CFG_TMODE_LAT
                    | RtkEmulatorUbxKeys::CFG_TMODE_LON
                    | RtkEmulatorUbxKeys::CFG_TMODE_HEIGHT
                    | RtkEmulatorUbxKeys::CFG_TMODE_LAT_HP
                    | RtkEmulatorUbxKeys::CFG_TMODE_LON_HP
                    | RtkEmulatorUbxKeys::CFG_TMODE_HEIGHT_HP
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_X
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Y
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Z
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_X_HP
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Y_HP
                    | RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Z_HP
            );
            let value = Self::read_value(payload, offset, value_size, signed);
            offset += value_size;
            Self::apply_key(state, key, value);
        }
    }

    fn apply_key(state: &mut RtkBaseEmulatorState, key: u32, value: i64) {
        match key {
            RtkEmulatorUbxKeys::CFG_TMODE_SVIN_MIN_DUR => {
                state.min_dur_seconds = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_SVIN_ACC_LIMIT => {
                // value в 0.1 mm
                state.acc_limit_meters = value as f64 / 10_000.0;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_MODE => match value {
                0 => state.disable_mode(),
                1 => state.start_survey_in(),
                2 => state.apply_fixed_mode(),
                _ => {}
            },
            RtkEmulatorUbxKeys::CFG_TMODE_POS_TYPE => {
                // 0 = ECEF, 1 = LLH — хранение не требуется отдельно
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_X => {
                state.ecef_x_cm = Some(value as i32);
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Y => {
                state.ecef_y_cm = Some(value as i32);
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Z => {
                state.ecef_z_cm = Some(value as i32);
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_X_HP => {
                state.ecef_x_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Y_HP => {
                state.ecef_y_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_ECEF_Z_HP => {
                state.ecef_z_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_LAT => {
                state.latitude = value as f64 / 1e7;
                state.ecef_x_cm = None;
                state.ecef_y_cm = None;
                state.ecef_z_cm = None;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_LON => {
                state.longitude = value as f64 / 1e7;
                state.ecef_x_cm = None;
                state.ecef_y_cm = None;
                state.ecef_z_cm = None;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_HEIGHT => {
                state.height_msl = value as f64 / 100.0;
                state.ecef_x_cm = None;
                state.ecef_y_cm = None;
                state.ecef_z_cm = None;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_LAT_HP => {
                state.lat_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_LON_HP => {
                state.lon_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_HEIGHT_HP => {
                state.height_hp = value as i32;
            }
            RtkEmulatorUbxKeys::CFG_TMODE_FIXED_POS_ACC => {
                // 0.1 mm → метры (для отображения)
                state.mean_acc_meters = value as f64 / 10_000.0;
            }
            _ => {
                // MSGOUT и прочие ключи — только ACK
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtk_base_emulator_state::RtkEmulatorMode;
    use crate::ubx::RtkEmulatorUbxKeys;

    #[test]
    fn valset_mode_starts_survey_and_acks() {
        let mut state = RtkBaseEmulatorState::new();
        let mut sent: Vec<u8> = Vec::new();
        let mut handler = CfgHandler::new(|bytes| sent = bytes.to_vec(), None::<fn()>);

        let mut payload = vec![0u8; 4];
        payload.extend_from_slice(&RtkEmulatorUbxKeys::CFG_TMODE_MODE.to_le_bytes());
        payload.push(1); // survey-in
        let frame = Frame::new(Codec::CLASS_CFG, Codec::MSG_CFG_VALSET, payload);

        handler.handle_frame(&mut state, &frame);
        assert_eq!(state.mode, RtkEmulatorMode::SurveyIn);
        assert!(!sent.is_empty());
        assert_eq!(sent[2], Codec::CLASS_ACK);
    }

    #[test]
    fn ignores_non_valset_frames() {
        let mut state = RtkBaseEmulatorState::new();
        let mut called = false;
        let mut handler = CfgHandler::new(|_| called = true, None::<fn()>);
        let frame = Frame::new(Codec::CLASS_NAV, Codec::MSG_NAV_PVT, vec![]);
        handler.handle_frame(&mut state, &frame);
        assert!(!called);
    }
}
