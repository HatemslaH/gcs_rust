use chrono::{DateTime, Datelike, Timelike, Utc};
use rtcm3_parser::Frame;

use crate::ubx::Ecef;

use super::bit_writer::BitWriter;

/// Сборка структурно валидных RTCM3-кадров с CRC24Q.
pub struct RtcmFrameBuilder {
    pub station_id: u16,
}

impl RtcmFrameBuilder {
    pub fn new(station_id: u16) -> Self {
        Self { station_id }
    }

    /// Полный набор коррекций для одного тика.
    pub fn build_cadence(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        height_m: f64,
        utc: DateTime<Utc>,
    ) -> Vec<Vec<u8>> {
        let ecef = Ecef::from_llh(lat_deg, lon_deg, height_m);
        let epoch_ms = Self::gps_itow_ms(utc);
        vec![
            self.build_type_1005(&ecef),
            self.build_msm7(1077, epoch_ms, &[1, 3, 8, 11, 14, 17, 19, 22]),
            self.build_msm7(1087, epoch_ms, &[1, 2, 5, 6, 7, 9]),
            self.build_msm7(1097, epoch_ms, &[1, 4, 7, 11, 13, 15, 19]),
            self.build_msm7(1127, epoch_ms, &[6, 9, 11, 14, 16, 21, 23, 28]),
            self.build_type_1230(),
        ]
    }

    pub fn build_type_1005(&self, ecef: &Ecef) -> Vec<u8> {
        let mut bits = BitWriter::new();
        bits.write(1005, 12);
        bits.write((self.station_id as u64) & 0xfff, 12);
        bits.write(1, 6); // ITRF year
        bits.write(0, 1); // GPS indicator
        bits.write(0, 1); // GLO
        bits.write(0, 1); // GAL
        bits.write(1, 1); // ref station indicator
        bits.write(Self::to_signed_38((ecef.x * 10_000.0).round() as i64), 38); // 0.0001 m
        bits.write(0, 1); // oscillator
        bits.write(0, 1); // reserved
        bits.write(Self::to_signed_38((ecef.y * 10_000.0).round() as i64), 38);
        bits.write(0, 2); // quarter cycle
        bits.write(Self::to_signed_38((ecef.z * 10_000.0).round() as i64), 38);
        Self::wrap_frame(&bits.to_bytes())
    }

    /// MSM7: корректный заголовок + несколько спутников с заполненными полями.
    pub fn build_msm7(&self, message_number: u16, epoch_ms: u32, prns: &[u8]) -> Vec<u8> {
        let sats: Vec<u8> = prns
            .iter()
            .copied()
            .filter(|&p| (1..=64).contains(&p))
            .collect();
        let n_sat = sats.len();
        let mut bits = BitWriter::new();

        bits.write(message_number as u64, 12);
        bits.write((self.station_id as u64) & 0xfff, 12);
        bits.write((epoch_ms as u64) & 0x3fff_ffff, 30);
        bits.write(0, 1); // multiple message
        bits.write(0, 3); // IODS
        bits.write(0, 7); // reserved
        bits.write(0, 2); // clock steering
        bits.write(0, 2); // ext clock
        bits.write(0, 1); // GNSS divergence
        bits.write(0, 3); // smoothing interval

        // Satellite mask 64 bits: PRN 1 = MSB
        let mut sat_mask: u128 = 0;
        for &prn in &sats {
            sat_mask |= 1u128 << (64 - prn);
        }
        bits.write_bits(sat_mask, 64);

        // Signal mask 32 bits — signal ID 1 (MSB)
        bits.write(1u64 << 31, 32);

        // Cell mask: nSat × 1 signal
        for _ in 0..n_sat {
            bits.write(1, 1);
        }

        // Satellite data MSM7
        for i in 0..n_sat {
            bits.write(120 + i as u64, 8);
        }
        for _ in 0..n_sat {
            bits.write(0, 4);
        }
        for _ in 0..n_sat {
            bits.write(0, 10);
        }

        // Signal data MSM7 per cell
        for i in 0..n_sat {
            bits.write(5000 + i as u64 * 17, 20);
            bits.write(8000 + i as u64 * 13, 24);
            bits.write(200 + i as u64, 10);
            bits.write(0, 1);
            bits.write(400 + i as u64 * 3, 10);
            bits.write(0, 15);
        }

        Self::wrap_frame(&bits.to_bytes())
    }

    pub fn build_type_1230(&self) -> Vec<u8> {
        let mut bits = BitWriter::new();
        bits.write(1230, 12);
        bits.write((self.station_id as u64) & 0xfff, 12);
        bits.write(0, 1); // bias indicator
        bits.write(0, 3); // reserved
        bits.write(0xf, 4); // signals mask
        for _ in 0..4 {
            bits.write(0, 16);
        }
        Self::wrap_frame(&bits.to_bytes())
    }

    fn wrap_frame(payload: &[u8]) -> Vec<u8> {
        Frame::from_payload(payload)
            .expect("RTCM payload within 10-bit length limit")
            .into_bytes()
    }

    fn to_signed_38(value: i64) -> u64 {
        let v = value.clamp(-(1i64 << 37), (1i64 << 37) - 1);
        if v < 0 {
            (v + (1i64 << 38)) as u64
        } else {
            v as u64
        }
    }

    fn gps_itow_ms(utc: DateTime<Utc>) -> u32 {
        // Dart: Sunday → 0, иначе weekday Mon=1..Sat=6
        let day_of_week = utc.weekday().number_from_monday() % 7;
        let sod = utc.hour() * 3600 + utc.minute() * 60 + utc.second();
        (day_of_week * 86_400 + sod) * 1000 + utc.timestamp_subsec_millis()
    }
}

impl Default for RtcmFrameBuilder {
    fn default() -> Self {
        Self::new(2001)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rtcm3_parser::{Crc24q, Parser};

    fn assert_valid_rtcm(frame: &[u8]) {
        assert_eq!(frame[0], Parser::PREAMBLE);
        let len = (((frame[1] as usize) & 0x03) << 8) | frame[2] as usize;
        assert_eq!(frame.len(), 3 + len + 3);
        let crc = Crc24q::calculate(frame, 3 + len, 0);
        let got = ((frame[3 + len] as u32) << 16)
            | ((frame[4 + len] as u32) << 8)
            | frame[5 + len] as u32;
        assert_eq!(got, crc);

        let parsed = Parser::new().add_data(frame);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].as_bytes(), frame);
    }

    #[test]
    fn cadence_frames_have_valid_crc() {
        let builder = RtcmFrameBuilder::default();
        let utc = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let frames = builder.build_cadence(55.75, 37.61, 150.0, utc);
        assert_eq!(frames.len(), 6);
        for frame in frames {
            assert_valid_rtcm(&frame);
        }
    }
}
