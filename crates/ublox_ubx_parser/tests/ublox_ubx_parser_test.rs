//! Порт Dart-тестов `ublox_ubx_parser_test.dart`.

use std::collections::HashSet;
use ublox_ubx_parser::{RtkBaseState, UbxAckData, UbxKeys, UbxNakData};

mod ubx_ack_data {
    use super::*;

    #[test]
    fn to_string_contains_ack_prefix() {
        let ack = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert!(ack.to_string().starts_with("ACK:"));
    }

    #[test]
    fn to_string_formats_class_id_with_leading_zero() {
        let ack = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert_eq!(ack.to_string(), "ACK: Class 0x05, Msg 0x01");
    }

    #[test]
    fn to_string_formats_single_digit_hex_with_leading_zero() {
        let ack = UbxAckData {
            class_id: 0x00,
            msg_id: 0x0f,
        };
        assert_eq!(ack.to_string(), "ACK: Class 0x00, Msg 0x0f");
    }

    #[test]
    fn to_string_formats_two_digit_hex_without_extra_zeros() {
        let ack = UbxAckData {
            class_id: 0xab,
            msg_id: 0xcd,
        };
        assert_eq!(ack.to_string(), "ACK: Class 0xab, Msg 0xcd");
    }

    #[test]
    fn stores_class_id_correctly() {
        let ack = UbxAckData {
            class_id: 0x06,
            msg_id: 0x8a,
        };
        assert_eq!(ack.class_id, 0x06);
    }

    #[test]
    fn stores_msg_id_correctly() {
        let ack = UbxAckData {
            class_id: 0x06,
            msg_id: 0x8a,
        };
        assert_eq!(ack.msg_id, 0x8a);
    }

    #[test]
    fn two_objects_with_same_fields_have_same_to_string() {
        let a = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        let b = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn two_objects_with_different_fields_have_different_to_string() {
        let a = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        let b = UbxAckData {
            class_id: 0x06,
            msg_id: 0x01,
        };
        assert_ne!(a.to_string(), b.to_string());
    }
}

mod ubx_nak_data {
    use super::*;

    #[test]
    fn to_string_contains_nak_prefix() {
        let nak = UbxNakData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert!(nak.to_string().starts_with("NAK:"));
    }

    #[test]
    fn to_string_formats_class_id_and_msg_id_correctly() {
        let nak = UbxNakData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert_eq!(nak.to_string(), "NAK: Class 0x05, Msg 0x01");
    }

    #[test]
    fn to_string_with_zero_values() {
        let nak = UbxNakData {
            class_id: 0x00,
            msg_id: 0x00,
        };
        assert_eq!(nak.to_string(), "NAK: Class 0x00, Msg 0x00");
    }

    #[test]
    fn to_string_formats_ff_correctly() {
        let nak = UbxNakData {
            class_id: 0xff,
            msg_id: 0xff,
        };
        assert_eq!(nak.to_string(), "NAK: Class 0xff, Msg 0xff");
    }

    #[test]
    fn stores_class_id_correctly() {
        let nak = UbxNakData {
            class_id: 0x06,
            msg_id: 0x8a,
        };
        assert_eq!(nak.class_id, 0x06);
    }

    #[test]
    fn stores_msg_id_correctly() {
        let nak = UbxNakData {
            class_id: 0x06,
            msg_id: 0x8a,
        };
        assert_eq!(nak.msg_id, 0x8a);
    }

    #[test]
    fn ack_and_nak_with_same_fields_produce_different_to_string() {
        let ack = UbxAckData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        let nak = UbxNakData {
            class_id: 0x05,
            msg_id: 0x01,
        };
        assert_ne!(ack.to_string(), nak.to_string());
    }
}

mod rtk_base_state {
    use super::*;

    #[test]
    fn pvt_data_is_none_initially() {
        let state = RtkBaseState::default();
        assert!(state.pvt_data.is_none());
    }

    #[test]
    fn svin_data_is_none_initially() {
        let state = RtkBaseState::default();
        assert!(state.svin_data.is_none());
    }

    #[test]
    fn clear_sets_pvt_data_to_none() {
        let mut state = RtkBaseState::default();
        state.clear();
        assert!(state.pvt_data.is_none());
    }

    #[test]
    fn clear_sets_svin_data_to_none() {
        let mut state = RtkBaseState::default();
        state.clear();
        assert!(state.svin_data.is_none());
    }

    #[test]
    fn calling_clear_twice_does_not_panic() {
        let mut state = RtkBaseState::default();
        state.clear();
        state.clear();
        assert!(state.pvt_data.is_none());
        assert!(state.svin_data.is_none());
    }

    #[test]
    fn different_instances_are_independent() {
        let mut a = RtkBaseState::default();
        let b = RtkBaseState::default();
        a.clear();
        assert!(b.pvt_data.is_none());
        assert!(b.svin_data.is_none());
    }
}

mod ubx_keys {
    use super::*;

    #[test]
    fn cfg_msg_out_rtcm3x_type1005_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_UART1, 0x2091_02be);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1005_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_USB, 0x2091_02c0);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1077_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_UART1, 0x2091_02cd);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1077_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_USB, 0x2091_02cf);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1087_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1087_UART1, 0x2091_02d2);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1087_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1087_USB, 0x2091_02d4);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1097_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1097_UART1, 0x2091_0319);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1097_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1097_USB, 0x2091_031b);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1127_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1127_UART1, 0x2091_02d7);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1127_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1127_USB, 0x2091_02d9);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1230_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1230_UART1, 0x2091_0304);
    }

    #[test]
    fn cfg_msg_out_rtcm3x_type1230_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1230_USB, 0x2091_0306);
    }

    #[test]
    fn cfg_msg_out_ubx_nav_pvt_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_UBX_NAV_PVT_UART1, 0x2091_0007);
    }

    #[test]
    fn cfg_msg_out_ubx_nav_pvt_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_UBX_NAV_PVT_USB, 0x2091_0009);
    }

    #[test]
    fn cfg_msg_out_ubx_nav_svin_uart1_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_UBX_NAV_SVIN_UART1, 0x2091_0089);
    }

    #[test]
    fn cfg_msg_out_ubx_nav_svin_usb_has_expected_value() {
        assert_eq!(UbxKeys::CFG_MSG_OUT_UBX_NAV_SVIN_USB, 0x2091_008b);
    }

    #[test]
    fn cfg_tmode_mode_has_expected_value() {
        assert_eq!(UbxKeys::CFG_TMODE_MODE, 0x2003_0001);
    }

    #[test]
    fn cfg_tmode_svin_min_dur_has_expected_value() {
        assert_eq!(UbxKeys::CFG_TMODE_SVIN_MIN_DUR, 0x4003_0010);
    }

    #[test]
    fn cfg_tmode_svin_acc_limit_has_expected_value() {
        assert_eq!(UbxKeys::CFG_TMODE_SVIN_ACC_LIMIT, 0x4003_0011);
    }

    #[test]
    fn uart1_and_usb_keys_differ_for_same_message_type() {
        assert_ne!(
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_USB
        );
        assert_ne!(
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_USB
        );
        assert_ne!(
            UbxKeys::CFG_MSG_OUT_UBX_NAV_PVT_UART1,
            UbxKeys::CFG_MSG_OUT_UBX_NAV_PVT_USB
        );
        assert_ne!(
            UbxKeys::CFG_MSG_OUT_UBX_NAV_SVIN_UART1,
            UbxKeys::CFG_MSG_OUT_UBX_NAV_SVIN_USB
        );
    }

    #[test]
    fn cfg_tmode_svin_min_dur_and_acc_limit_differ() {
        assert_ne!(
            UbxKeys::CFG_TMODE_SVIN_MIN_DUR,
            UbxKeys::CFG_TMODE_SVIN_ACC_LIMIT
        );
    }

    #[test]
    fn all_rtcm3_configuration_keys_are_unique() {
        let rtcm_keys = [
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1005_USB,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1077_USB,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1087_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1087_USB,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1097_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1097_USB,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1127_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1127_USB,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1230_UART1,
            UbxKeys::CFG_MSG_OUT_RTCM3X_TYPE1230_USB,
        ];
        let unique: HashSet<_> = rtcm_keys.iter().copied().collect();
        assert_eq!(unique.len(), rtcm_keys.len());
    }
}
