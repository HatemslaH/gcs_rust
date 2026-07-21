use crate::ffi::ubx_pvt_data_t;
use std::ffi::CStr;

/// NAV-PVT с единицами, как в Dart `UbxPvtData`.
#[derive(Debug, Clone, PartialEq)]
pub struct UbxPvtData {
    pub i_tow: u32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub valid: u8,
    pub t_acc: u32,
    pub nano: i32,
    pub fix_type: u8,
    pub fix_type_string: String,
    pub flags: u8,
    pub flags2: u8,
    pub num_sv: u8,
    /// градусы
    pub latitude: f64,
    /// градусы
    pub longitude: f64,
    /// метры
    pub height: f64,
    /// метры
    pub h_msl: f64,
    /// метры
    pub h_acc: f64,
    /// метры
    pub v_acc: f64,
    /// м/с
    pub vel_n: f64,
    /// м/с
    pub vel_e: f64,
    /// м/с
    pub vel_d: f64,
    /// м/с
    pub g_speed: f64,
    /// градусы
    pub head_mot: f64,
    /// м/с
    pub s_acc: f64,
    /// градусы
    pub head_acc: f64,
    pub p_dop: f64,
    pub flags3: u16,
    /// градусы
    pub head_veh: f64,
    /// градусы
    pub mag_dec: f64,
    /// градусы
    pub mag_acc: f64,
}

impl UbxPvtData {
    pub(crate) unsafe fn from_raw(raw: &ubx_pvt_data_t) -> Self {
        let fix_type_string = unsafe { CStr::from_ptr(raw.fixTypeString.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        Self {
            i_tow: raw.iTOW,
            year: raw.year,
            month: raw.month,
            day: raw.day,
            hour: raw.hour,
            minute: raw.min,
            second: raw.sec,
            valid: raw.valid,
            t_acc: raw.tAcc,
            nano: raw.nano,
            fix_type: raw.fixType,
            fix_type_string,
            flags: raw.flags,
            flags2: raw.flags2,
            num_sv: raw.numSV,
            latitude: f64::from(raw.lat) / 10_000_000.0,
            longitude: f64::from(raw.lon) / 10_000_000.0,
            height: f64::from(raw.height) / 1000.0,
            h_msl: f64::from(raw.hMSL) / 1000.0,
            h_acc: f64::from(raw.hAcc) / 1000.0,
            v_acc: f64::from(raw.vAcc) / 1000.0,
            vel_n: f64::from(raw.velN) / 1000.0,
            vel_e: f64::from(raw.velE) / 1000.0,
            vel_d: f64::from(raw.velD) / 1000.0,
            g_speed: f64::from(raw.gSpeed) / 1000.0,
            head_mot: f64::from(raw.headMot) / 100_000.0,
            s_acc: f64::from(raw.sAcc) / 1000.0,
            head_acc: f64::from(raw.headAcc) / 100_000.0,
            p_dop: f64::from(raw.pDOP) / 100.0,
            flags3: raw.flags3,
            head_veh: f64::from(raw.headVeh) / 100_000.0,
            mag_dec: f64::from(raw.magDec) / 100_000.0,
            mag_acc: f64::from(raw.magAcc) / 100_000.0,
        }
    }
}

/// NAV-SVIN с единицами, как в Dart `UbxSvinData`.
#[derive(Debug, Clone, PartialEq)]
pub struct UbxSvinData {
    pub i_tow: u32,
    pub dur: u32,
    /// ECEF mean, метры (см + HP 0.1 mm)
    pub mean_x: f64,
    pub mean_y: f64,
    pub mean_z: f64,
    /// HP в миллиметрах (для отображения)
    pub mean_x_hp_mm: f64,
    pub mean_y_hp_mm: f64,
    pub mean_z_hp_mm: f64,
    /// сырые ECEF mean, см (I4)
    pub mean_x_cm: i32,
    pub mean_y_cm: i32,
    pub mean_z_cm: i32,
    /// сырые ECEF HP, 0.1 mm (−99…99)
    pub mean_x_hp: i8,
    pub mean_y_hp: i8,
    pub mean_z_hp: i8,
    /// метры
    pub mean_acc: f64,
    pub obs: u32,
    pub valid: bool,
    pub active: bool,
}

impl UbxSvinData {
    pub(crate) fn from_raw(raw: &crate::ffi::ubx_svin_data_t) -> Self {
        Self {
            i_tow: raw.iTOW,
            dur: raw.dur,
            mean_x_cm: raw.meanX,
            mean_y_cm: raw.meanY,
            mean_z_cm: raw.meanZ,
            mean_x_hp: raw.meanXHP,
            mean_y_hp: raw.meanYHP,
            mean_z_hp: raw.meanZHP,
            mean_x: f64::from(raw.meanX) * 0.01 + f64::from(raw.meanXHP) * 0.0001,
            mean_y: f64::from(raw.meanY) * 0.01 + f64::from(raw.meanYHP) * 0.0001,
            mean_z: f64::from(raw.meanZ) * 0.01 + f64::from(raw.meanZHP) * 0.0001,
            mean_x_hp_mm: f64::from(raw.meanXHP) * 0.1,
            mean_y_hp_mm: f64::from(raw.meanYHP) * 0.1,
            mean_z_hp_mm: f64::from(raw.meanZHP) * 0.1,
            mean_acc: f64::from(raw.meanAcc) / 10_000.0,
            obs: raw.obs,
            valid: raw.valid == 1,
            active: raw.active == 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UbxAckData {
    pub class_id: u8,
    pub msg_id: u8,
}

impl std::fmt::Display for UbxAckData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ACK: Class 0x{:02x}, Msg 0x{:02x}",
            self.class_id, self.msg_id
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UbxNakData {
    pub class_id: u8,
    pub msg_id: u8,
}

impl std::fmt::Display for UbxNakData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NAK: Class 0x{:02x}, Msg 0x{:02x}",
            self.class_id, self.msg_id
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UbxMessage {
    Pvt(UbxPvtData),
    Svin(UbxSvinData),
    Ack(UbxAckData),
    Nak(UbxNakData),
}

#[derive(Debug, Default, Clone)]
pub struct RtkBaseState {
    pub svin_data: Option<UbxSvinData>,
    pub pvt_data: Option<UbxPvtData>,
}

impl RtkBaseState {
    pub fn clear(&mut self) {
        self.svin_data = None;
        self.pvt_data = None;
    }
}
