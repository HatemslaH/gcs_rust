use rtcm3_parser::{Frame, Parser};

/// Собирает валидный RTCM3-кадр с корректным CRC-24Q.
fn make_frame(payload: &[u8]) -> Vec<u8> {
    Frame::from_payload(payload)
        .expect("payload within RTCM3 limits")
        .into_bytes()
}

#[test]
fn parses_single_complete_frame() {
    let frame = make_frame(&[0x3E, 0xD0]); // message type 1005
    let mut parser = Parser::new();

    let frames = parser.add_data(&frame);

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].as_bytes(), frame.as_slice());
    assert_eq!(frames[0].payload(), &[0x3E, 0xD0]);
    assert_eq!(frames[0].message_number(), Some(1005));
    assert_eq!(parser.buffered_len(), 0);
}

#[test]
fn parses_frame_split_across_chunks() {
    let frame = make_frame(&[0x01, 0x23, 0x45, 0x67]);
    let mut parser = Parser::new();

    assert!(parser.add_data(&frame[..2]).is_empty());
    assert!(parser.buffered_len() > 0);

    assert!(parser.add_data(&frame[2..5]).is_empty());

    let frames = parser.add_data(&frame[5..]);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].as_bytes(), frame.as_slice());
}

#[test]
fn parses_multiple_frames_in_one_chunk() {
    let a = make_frame(&[0xAA]);
    let b = make_frame(&[0xBB, 0xCC]);
    let mut input = a.clone();
    input.extend_from_slice(&b);

    let frames = Parser::new().add_data(&input);

    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].as_bytes(), a.as_slice());
    assert_eq!(frames[1].as_bytes(), b.as_slice());
}

#[test]
fn skips_noise_before_preamble() {
    let frame = make_frame(&[0x10, 0x20]);
    let mut input = vec![0x00, 0xFF, 0x01];
    input.extend_from_slice(&frame);

    let frames = Parser::new().add_data(&input);

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].as_bytes(), frame.as_slice());
}

#[test]
fn rejects_invalid_crc() {
    let mut frame = make_frame(&[0x11, 0x22]);
    let last = frame.len() - 1;
    frame[last] ^= 0xFF;

    let mut parser = Parser::new();
    let frames = parser.add_data(&frame);

    assert!(frames.is_empty());
    // После отбраковки preamble оставшиеся байты битого кадра могут остаться в буфере.
    assert!(parser.buffered_len() < frame.len());
}

#[test]
fn recovers_after_invalid_crc_and_finds_next_frame() {
    let good = make_frame(&[0x42]);
    let mut bad = make_frame(&[0x11]);
    let last = bad.len() - 1;
    bad[last] ^= 0x01;

    let mut input = bad;
    input.extend_from_slice(&good);

    let frames = Parser::new().add_data(&input);

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].as_bytes(), good.as_slice());
}

#[test]
fn rejects_nonzero_reserved_bits() {
    let mut frame = make_frame(&[0x01]);
    // Выставляем reserved-биты в байте длины.
    frame[1] |= 0x04;
    // CRC оставляем старым — кадр всё равно должен быть отвергнут по reserved.

    let frames = Parser::new().add_data(&frame);
    assert!(frames.is_empty());
}

#[test]
fn clear_discards_partial_buffer() {
    let frame = make_frame(&[0x01, 0x02, 0x03]);
    let mut parser = Parser::new();

    parser.add_data(&frame[..4]);
    assert!(parser.buffered_len() > 0);

    parser.clear();
    assert_eq!(parser.buffered_len(), 0);

    let frames = parser.add_data(&frame);
    assert_eq!(frames.len(), 1);
}

#[test]
fn empty_payload_frame() {
    let frame = make_frame(&[]);
    let frames = Parser::new().add_data(&frame);

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].payload_len(), 0);
    assert!(frames[0].payload().is_empty());
    assert_eq!(frames[0].message_number(), None);
}

#[test]
fn frame_as_ref() {
    let raw = make_frame(&[0x3E, 0xD0]);
    let frame = Parser::new().add_data(&raw).pop().unwrap();
    let bytes: &[u8] = frame.as_ref();
    assert_eq!(bytes, raw.as_slice());
}

#[test]
fn message_number_from_short_payload() {
    let frame = make_frame(&[0xFF]);
    let parsed: Frame = Parser::new().add_data(&frame).pop().unwrap();
    assert_eq!(parsed.message_number(), None);
}

#[test]
fn from_payload_crc_roundtrip() {
    let payload = [0x01, 0x23, 0x45];
    let encoded = Frame::from_payload(&payload).unwrap();
    let parsed = Parser::new().add_data(encoded.as_bytes());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].payload(), &payload);
    assert_eq!(parsed[0].as_bytes(), encoded.as_bytes());
}
