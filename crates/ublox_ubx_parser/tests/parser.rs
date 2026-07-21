use ublox_ubx_parser::{UbxKeys, UbxMessage, UbxParser};

fn ubx_checksum(frame: &[u8]) -> (u8, u8) {
    let mut ck_a = 0u8;
    let mut ck_b = 0u8;
    for &b in &frame[2..frame.len() - 2] {
        ck_a = ck_a.wrapping_add(b);
        ck_b = ck_b.wrapping_add(ck_a);
    }
    (ck_a, ck_b)
}

fn make_ack_frame(class_id: u8, msg_id: u8) -> Vec<u8> {
    let mut frame = vec![0xB5, 0x62, 0x05, 0x01, 0x02, 0x00, class_id, msg_id, 0, 0];
    let (ck_a, ck_b) = ubx_checksum(&frame);
    let len = frame.len();
    frame[len - 2] = ck_a;
    frame[len - 1] = ck_b;
    frame
}

#[test]
fn pack_mode_is_valid_ubx_frame() {
    let parser = UbxParser::new();
    let frame = parser.pack_ubx_mode();
    assert!(frame.len() >= 8);
    assert_eq!(&frame[0..2], &[0xB5, 0x62]);
    assert_eq!(&frame[2..4], &[0x06, 0x8A]); // CFG-VALSET
    let (ck_a, ck_b) = ubx_checksum(&frame);
    assert_eq!(frame[frame.len() - 2], ck_a);
    assert_eq!(frame[frame.len() - 1], ck_b);
}

#[test]
fn pack_valset_uses_requested_key() {
    let parser = UbxParser::new();
    let frame = parser.pack_ubx_valset(
        UbxKeys::CFG_TMODE_POS_TYPE,
        1,
        UbxKeys::CFG_TMODE_POS_TYPE_ECEF,
    );
    // key at offset 10 (after sync+class+id+len+ver+layers+reserved)
    let key = u32::from_le_bytes(frame[10..14].try_into().unwrap());
    assert_eq!(key, UbxKeys::CFG_TMODE_POS_TYPE);
    assert_eq!(frame[14], 0);
}

#[test]
fn parses_ack_from_stream() {
    let mut parser = UbxParser::new();
    let frame = make_ack_frame(0x06, 0x8A);
    let messages = parser.add_data(&frame);
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        UbxMessage::Ack(ack) => {
            assert_eq!(ack.class_id, 0x06);
            assert_eq!(ack.msg_id, 0x8A);
        }
        other => panic!("ожидался ACK, получено {other:?}"),
    }
}

#[test]
fn pack_helpers_produce_nonempty_frames() {
    let parser = UbxParser::new();
    assert!(!parser.pack_ubx_mode_disabled().is_empty());
    assert!(!parser.pack_ubx_mode_fixed().is_empty());
    assert!(!parser.pack_ubx_pos_type_llh().is_empty());
    assert!(!parser.pack_ubx_pos_type_ecef().is_empty());
    assert!(!parser.pack_ubx_tmode_svin_min_dur(60).is_empty());
    assert!(!parser.pack_ubx_acc_min(1.5).is_empty());
    assert!(!parser.pack_ubx_restart().is_empty());
    assert!(!parser.pack_ubx_valdel_all().is_empty());
}
