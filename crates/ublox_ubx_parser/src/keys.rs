//! CFG-ключи: значения из нативного `ubx-enum.h` / `ubx-cfg-valset.h` (bindgen).

use crate::ffi;

/// Удобные имена (как Dart `UbxKeys`); числа — из C, без дублирования литералов.
pub struct UbxKeys;

impl UbxKeys {
    pub const CFG_MSG_OUT_RTCM3X_TYPE1005_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1005_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1005_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1005_USB as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1077_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1077_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1077_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1077_USB as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1087_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1087_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1087_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1087_USB as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1097_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1097_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1097_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1097_USB as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1127_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1127_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1127_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1127_USB as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1230_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1230_UART1 as u32;
    pub const CFG_MSG_OUT_RTCM3X_TYPE1230_USB: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_RTCM_3X_TYPE1230_USB as u32;

    pub const CFG_MSG_OUT_UBX_NAV_PVT_UART1: u32 = ffi::UBX_KEY_CFG_MSGOUT_UBX_NAV_PVT_UART1 as u32;
    pub const CFG_MSG_OUT_UBX_NAV_PVT_USB: u32 = ffi::UBX_KEY_CFG_MSGOUT_UBX_NAV_PVT_USB as u32;
    pub const CFG_MSG_OUT_UBX_NAV_SVIN_UART1: u32 =
        ffi::UBX_KEY_CFG_MSGOUT_UBX_NAV_SVIN_UART1 as u32;
    pub const CFG_MSG_OUT_UBX_NAV_SVIN_USB: u32 = ffi::UBX_KEY_CFG_MSGOUT_UBX_NAV_SVIN_USB as u32;

    pub const CFG_TMODE_MODE: u32 = ffi::UBX_KEY_CFG_TMODE_MODE as u32;
    pub const CFG_TMODE_POS_TYPE: u32 = ffi::UBX_KEY_CFG_TMODE_POS_TYPE as u32;

    /// POS_TYPE: 0 = ECEF, 1 = LLH (`ubx-cfg-valset.h`).
    pub const CFG_TMODE_POS_TYPE_ECEF: u32 = ffi::UBX_CFG_VALSET_TMODE_POS_TYPE_ECEF as u32;
    pub const CFG_TMODE_POS_TYPE_LLH: u32 = ffi::UBX_CFG_VALSET_TMODE_POS_TYPE_LLH as u32;

    pub const CFG_TMODE_ECEF_X: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_X as u32;
    pub const CFG_TMODE_ECEF_Y: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_Y as u32;
    pub const CFG_TMODE_ECEF_Z: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_Z as u32;
    pub const CFG_TMODE_ECEF_X_HP: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_X_HP as u32;
    pub const CFG_TMODE_ECEF_Y_HP: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_Y_HP as u32;
    pub const CFG_TMODE_ECEF_Z_HP: u32 = ffi::UBX_KEY_CFG_TMODE_ECEF_Z_HP as u32;

    pub const CFG_TMODE_LAT: u32 = ffi::UBX_KEY_CFG_TMODE_LAT as u32;
    pub const CFG_TMODE_LON: u32 = ffi::UBX_KEY_CFG_TMODE_LON as u32;
    pub const CFG_TMODE_HEIGHT: u32 = ffi::UBX_KEY_CFG_TMODE_HEIGHT as u32;
    pub const CFG_TMODE_LAT_HP: u32 = ffi::UBX_KEY_CFG_TMODE_LAT_HP as u32;
    pub const CFG_TMODE_LON_HP: u32 = ffi::UBX_KEY_CFG_TMODE_LON_HP as u32;
    pub const CFG_TMODE_HEIGHT_HP: u32 = ffi::UBX_KEY_CFG_TMODE_HEIGHT_HP as u32;
    pub const CFG_TMODE_FIXED_POS_ACC: u32 = ffi::UBX_KEY_CFG_TMODE_FIXED_POS_ACC as u32;
    pub const CFG_TMODE_SVIN_MIN_DUR: u32 = ffi::UBX_KEY_CFG_TMODE_SVIN_MIN_DUR as u32;
    pub const CFG_TMODE_SVIN_ACC_LIMIT: u32 = ffi::UBX_KEY_CFG_TMODE_SVIN_ACC_LIMIT as u32;
}
