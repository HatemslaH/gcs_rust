use crate::ffi::{self, ubx_packed_message};
use crate::keys::UbxKeys;
use crate::types::{RtkBaseState, UbxAckData, UbxMessage, UbxNakData, UbxPvtData, UbxSvinData};
use std::sync::{Mutex, OnceLock};

/// Глобальный лок: C-парсер хранит состояние в static.
fn native_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Безопасная обёртка над C `ublox_ubx_parser` (аналог Dart `UbxParser`).
///
/// Нативное состояние — process-global; доступ сериализуется мьютексом.
pub struct UbxParser {
    received: Vec<UbxMessage>,
    rtk_base_state: RtkBaseState,
}

impl UbxParser {
    pub fn new() -> Self {
        let _guard = native_lock().lock().expect("ublox_ubx_parser lock");
        unsafe {
            ffi::ublox_ubx_parser_init();
        }
        Self {
            received: Vec::new(),
            rtk_base_state: RtkBaseState::default(),
        }
    }

    pub fn rtk_base_state(&self) -> &RtkBaseState {
        &self.rtk_base_state
    }

    pub fn clear_rtk_base_classes(&mut self) {
        self.rtk_base_state.clear();
    }

    pub fn clear(&mut self) {
        self.received.clear();
    }

    pub fn pvt_messages(&self) -> impl Iterator<Item = &UbxPvtData> {
        self.received.iter().filter_map(|m| match m {
            UbxMessage::Pvt(p) => Some(p),
            _ => None,
        })
    }

    pub fn svin_messages(&self) -> impl Iterator<Item = &UbxSvinData> {
        self.received.iter().filter_map(|m| match m {
            UbxMessage::Svin(s) => Some(s),
            _ => None,
        })
    }

    /// Подать байты потока; вернуть сообщения, собранные из этого чанка.
    pub fn add_data(&mut self, data: &[u8]) -> Vec<UbxMessage> {
        let _guard = native_lock().lock().expect("ublox_ubx_parser lock");
        let mut messages = Vec::new();

        for &byte in data {
            let result = unsafe { ffi::ubx_parse(byte) };
            if result != 0 || unsafe { ffi::ubx_is_message_ready() } != 1 {
                continue;
            }

            let msg_type = unsafe { ffi::ubx_get_last_message_type() };
            match msg_type {
                1 => {
                    let ptr = unsafe { ffi::ubx_get_pvt_data() };
                    if !ptr.is_null() {
                        let pvt = unsafe { UbxPvtData::from_raw(&*ptr) };
                        self.rtk_base_state.pvt_data = Some(pvt.clone());
                        messages.push(UbxMessage::Pvt(pvt));
                    }
                }
                2 => {
                    let ptr = unsafe { ffi::ubx_get_svin_data() };
                    if !ptr.is_null() {
                        let svin = unsafe { UbxSvinData::from_raw(&*ptr) };
                        self.rtk_base_state.svin_data = Some(svin.clone());
                        messages.push(UbxMessage::Svin(svin));
                    }
                }
                3 => {
                    if let Some((class_id, msg_id)) = read_ack_ids() {
                        messages.push(UbxMessage::Ack(UbxAckData { class_id, msg_id }));
                    }
                }
                4 => {
                    if let Some((class_id, msg_id)) = read_ack_ids() {
                        messages.push(UbxMessage::Nak(UbxNakData { class_id, msg_id }));
                    }
                }
                _ => {}
            }
        }

        self.received.extend(messages.iter().cloned());
        messages
    }

    pub fn pack_ubx_tmode_svin_min_dur(&self, seconds: u32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_svin_min_dur(seconds) })
    }

    pub fn pack_ubx_acc_min(&self, accuracy_meters: f64) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_acc_min(accuracy_meters) })
    }

    pub fn pack_ubx_mode(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_mode() })
    }

    pub fn pack_ubx_mode_disabled(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_mode_disabled() })
    }

    pub fn pack_ubx_mode_fixed(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_mode_fixed() })
    }

    pub fn pack_ubx_pos_type_llh(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_pos_type_llh() })
    }

    pub fn pack_ubx_pos_type_ecef(&self) -> Vec<u8> {
        self.pack_ubx_valset(
            UbxKeys::CFG_TMODE_POS_TYPE,
            1,
            UbxKeys::CFG_TMODE_POS_TYPE_ECEF,
        )
    }

    /// `lat` в единицах протокола: 1e-7 deg (I4).
    pub fn pack_ubx_tmode_lat(&self, lat: i32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_lat(lat) })
    }

    /// `lon` в единицах протокола: 1e-7 deg (I4).
    pub fn pack_ubx_tmode_lon(&self, lon: i32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_lon(lon) })
    }

    /// `height_cm` в единицах протокола: cm (I4).
    pub fn pack_ubx_tmode_height(&self, height_cm: i32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_height(height_cm) })
    }

    pub fn pack_ubx_tmode_lat_hp(&self, lat_hp: i8) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_lat_hp(lat_hp) })
    }

    pub fn pack_ubx_tmode_lon_hp(&self, lon_hp: i8) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_lon_hp(lon_hp) })
    }

    pub fn pack_ubx_tmode_height_hp(&self, height_hp: i8) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_tmode_height_hp(height_hp) })
    }

    pub fn pack_ubx_tmode_ecef_x(&self, x_cm: i32) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_X, 4, x_cm as u32)
    }

    pub fn pack_ubx_tmode_ecef_y(&self, y_cm: i32) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_Y, 4, y_cm as u32)
    }

    pub fn pack_ubx_tmode_ecef_z(&self, z_cm: i32) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_Z, 4, z_cm as u32)
    }

    pub fn pack_ubx_tmode_ecef_x_hp(&self, x_hp: i8) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_X_HP, 1, x_hp as i32 as u32)
    }

    pub fn pack_ubx_tmode_ecef_y_hp(&self, y_hp: i8) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_Y_HP, 1, y_hp as i32 as u32)
    }

    pub fn pack_ubx_tmode_ecef_z_hp(&self, z_hp: i8) -> Vec<u8> {
        self.pack_ubx_valset(UbxKeys::CFG_TMODE_ECEF_Z_HP, 1, z_hp as i32 as u32)
    }

    /// FIXED_POS_ACC в единицах 0.1 mm (U4).
    pub fn pack_ubx_fixed_pos_acc(&self, accuracy_0_1_mm: u32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_fixed_pos_acc(accuracy_0_1_mm) })
    }

    pub fn pack_ubx_valdel_all(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_valdel_all() })
    }

    pub fn pack_ubx_restart(&self) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_restart() })
    }

    pub fn pack_ubx_valset(&self, key: u32, value_size: u8, value: u32) -> Vec<u8> {
        copy_packed(|| unsafe { ffi::ubx_pack_valset(key, value_size, value) })
    }
}

impl Default for UbxParser {
    fn default() -> Self {
        Self::new()
    }
}

fn read_ack_ids() -> Option<(u8, u8)> {
    let raw = unsafe { ffi::ubx_get_raw_message() };
    if raw.is_null() {
        return None;
    }
    let msg = unsafe { &*raw };
    if msg.data.is_null() || msg.len < 2 {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(msg.data, msg.len as usize) };
    Some((slice[0], slice[1]))
}

fn copy_packed(f: impl FnOnce() -> ubx_packed_message) -> Vec<u8> {
    let _guard = native_lock().lock().expect("ublox_ubx_parser lock");
    let msg = f();
    if msg.data.is_null() || msg.len == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(msg.data, msg.len as usize) }.to_vec()
}
