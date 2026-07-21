pub struct Ecef {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Ecef {
    const A: f64 = 6378137.0;
    const F: f64 = 1.0 / 298.257223563;
    const E2: f64 = Self::F * (2.0 - Self::F);

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn from_llh(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Self {
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();

        let sin_lat = lat_rad.sin();
        let cos_lat = lat_rad.cos();
        let sin_lon = lon_rad.sin();
        let cos_lon = lon_rad.cos();

        let n = Self::A / (1.0 - Self::E2 * sin_lat * sin_lat).sqrt();

        Self {
            x: (n + alt_m) * cos_lat * cos_lon,
            y: (n + alt_m) * cos_lat * sin_lon,
            z: (n * (1.0 - Self::E2) + alt_m) * sin_lat,
        }
    }

    pub fn to_llh(&self) -> (f64, f64, f64) {
        let lon = self.y.atan2(self.x);
        let p = (self.x * self.x + self.y * self.y).sqrt();

        let mut lat = self.z.atan2(p * (1.0 - Self::E2));
        let mut height = 0.0;

        for _ in 0..10 {
            let sin_lat = lat.sin();
            let n = Self::A / (1.0 - Self::E2 * sin_lat * sin_lat).sqrt();
            height = p / lat.cos() - n;
            lat = self.z.atan2(p * (1.0 - Self::E2 * n / (n + height)));
        }

        (lat.to_degrees(), lon.to_degrees(), height)
    }

    pub fn split_meters_to_cm_hp(meters: f64) -> (u16, u16) {
        let total_01_mm = (meters * 1000.0).round();

        let mut cm = (total_01_mm / 100.0).round();
        let mut hp = total_01_mm - cm * 100.0;

        if hp > 99.0 {
            cm += 1.0;
            hp -= 100.0;
        } else if hp < -99.0 {
            cm -= 1.0;
            hp += 100.0;
        }

        (cm as u16, hp as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llh_ecef_roundtrip_moscow() {
        let lat = 55.7558;
        let lon = 37.6173;
        let alt = 150.0;
        let ecef = Ecef::from_llh(lat, lon, alt);
        let (lat2, lon2, alt2) = ecef.to_llh();
        assert!((lat2 - lat).abs() < 1e-5, "lat {lat2} vs {lat}");
        assert!((lon2 - lon).abs() < 1e-5, "lon {lon2} vs {lon}");
        assert!((alt2 - alt).abs() < 1e-2, "alt {alt2} vs {alt}");
    }

    #[test]
    fn split_meters_to_cm_hp_basic() {
        let (cm, hp) = Ecef::split_meters_to_cm_hp(12.3456);
        // 12.3456 m → 123456 * 0.1mm → cm=1235, hp roughly residual
        assert!(cm > 0);
        assert!(hp <= 99);
    }
}
