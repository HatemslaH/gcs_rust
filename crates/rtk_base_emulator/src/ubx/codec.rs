use crate::ubx::Frame;

pub struct Codec;

/// Сборка и потоковый разбор UBX (sync B5 62 + Fletcher CK_A/CK_B).
impl Codec {
    pub const SYNC1: u8 = 0xb5;
    pub const SYNC2: u8 = 0x62;

    pub const CLASS_CFG: u8 = 0x06;
    pub const MSG_CFG_VALSET: u8 = 0x8a;
    pub const CLASS_ACK: u8 = 0x05;
    pub const MSG_ACK_ACK: u8 = 0x01;
    pub const CLASS_NAV: u8 = 0x01;
    pub const MSG_NAV_PVT: u8 = 0x07;
    pub const MSG_NAV_SVIN: u8 = 0x3b;

    pub fn new() -> Self {
        Self {}
    }

    pub fn value_size_from_key(key: u32) -> usize {
        match (key >> 28) & 0x7 {
            1 => 1,
            2 => 1,
            3 => 2,
            4 => 4,
            5 => 8,
            _ => 4,
        }
    }

    pub fn pack(frame: &Frame) -> Vec<u8> {
        let len = frame.payload.len();
        let mut out = vec![0u8; len + 8];

        out[0] = Self::SYNC1;
        out[1] = Self::SYNC2;
        out[2] = frame.class_id;
        out[3] = frame.message_id;
        out[4] = (len & 0xff) as u8;
        out[5] = ((len >> 8) & 0xff) as u8;
        out[6..len + 6].copy_from_slice(&frame.payload);

        let mut ck_a: u8 = 0;
        let mut ck_b: u8 = 0;
        for byte in &out[2..len + 6] {
            ck_a = ck_a.wrapping_add(*byte);
            ck_b = ck_b.wrapping_add(ck_a);
        }

        out[len + 6] = ck_a;
        out[len + 7] = ck_b;

        out
    }

    pub fn write_u4(bd: &mut [u8], offset: usize, value: u32) {
        bd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn write_i4(bd: &mut [u8], offset: usize, value: i32) {
        bd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn write_u2(bd: &mut [u8], offset: usize, value: u16) {
        bd[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn write_i2(bd: &mut [u8], offset: usize, value: i16) {
        bd[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn read_u4(bd: &[u8], offset: usize) -> Option<u32> {
        bd.get(offset..offset + 4)?
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
    }

    pub fn read_i4(bd: &[u8], offset: usize) -> Option<i32> {
        bd.get(offset..offset + 4)?
            .try_into()
            .ok()
            .map(i32::from_le_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_size_from_key_encodes_type() {
        assert_eq!(Codec::value_size_from_key(0x2003_0001), 1);
        assert_eq!(Codec::value_size_from_key(0x4003_0003), 4);
        assert_eq!(Codec::value_size_from_key(0x5003_0000), 8);
    }

    #[test]
    fn pack_roundtrip_checksum_and_payload() {
        let packed = Codec::pack(&Frame::new(0x06, 0x8a, vec![1, 2, 3, 4]));
        assert_eq!(packed[0], Codec::SYNC1);
        assert_eq!(packed[1], Codec::SYNC2);
        assert_eq!(packed[2], 0x06);
        assert_eq!(packed[3], 0x8a);
        assert_eq!(packed[4], 4);
        assert_eq!(packed[5], 0);
        assert_eq!(&packed[6..10], &[1, 2, 3, 4]);
        assert_eq!(packed.len(), 12);

        let mut ck_a: u8 = 0;
        let mut ck_b: u8 = 0;
        for b in &packed[2..10] {
            ck_a = ck_a.wrapping_add(*b);
            ck_b = ck_b.wrapping_add(ck_a);
        }
        assert_eq!(packed[10], ck_a);
        assert_eq!(packed[11], ck_b);
    }

    #[test]
    fn write_read_integers_le() {
        let mut buf = [0u8; 8];
        Codec::write_u4(&mut buf, 0, 0x0102_0304);
        Codec::write_i4(&mut buf, 4, -2);
        assert_eq!(Codec::read_u4(&buf, 0), Some(0x0102_0304));
        assert_eq!(Codec::read_i4(&buf, 4), Some(-2));
        assert_eq!(Codec::read_u4(&buf, 6), None);
    }
}
