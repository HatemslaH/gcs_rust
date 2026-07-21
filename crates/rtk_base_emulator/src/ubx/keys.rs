/// Ключи CFG-TMODE / MSGOUT, используемые приложением.
pub struct RtkEmulatorUbxKeys;

impl RtkEmulatorUbxKeys {
    pub const CFG_TMODE_MODE: u32 = 0x20030001;
    pub const CFG_TMODE_POS_TYPE: u32 = 0x20030002;
    pub const CFG_TMODE_ECEF_X: u32 = 0x40030003;
    pub const CFG_TMODE_ECEF_Y: u32 = 0x40030004;
    pub const CFG_TMODE_ECEF_Z: u32 = 0x40030005;
    pub const CFG_TMODE_ECEF_X_HP: u32 = 0x20030006;
    pub const CFG_TMODE_ECEF_Y_HP: u32 = 0x20030007;
    pub const CFG_TMODE_ECEF_Z_HP: u32 = 0x20030008;
    pub const CFG_TMODE_LAT: u32 = 0x40030009;
    pub const CFG_TMODE_LON: u32 = 0x4003000a;
    pub const CFG_TMODE_HEIGHT: u32 = 0x4003000b;
    pub const CFG_TMODE_LAT_HP: u32 = 0x2003000c;
    pub const CFG_TMODE_LON_HP: u32 = 0x2003000d;
    pub const CFG_TMODE_HEIGHT_HP: u32 = 0x2003000e;
    pub const CFG_TMODE_FIXED_POS_ACC: u32 = 0x4003000f;
    pub const CFG_TMODE_SVIN_MIN_DUR: u32 = 0x40030010;
    pub const CFG_TMODE_SVIN_ACC_LIMIT: u32 = 0x40030011;
}
