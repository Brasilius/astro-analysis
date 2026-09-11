use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Star {
    pub id: u32,
    pub proper: String,
    pub ra: f64,
    pub dec: f64,
    pub mag: f64,
    pub con: String,
    pub dist: f64,
    pub spect: String,
}
impl Star {
    pub fn name(&self) -> String {
        if self.proper.is_empty() {
            format!("HYG {}", self.id)
        } else {
            self.proper.clone()
        }
    }
}
pub fn catalog(bytes: &[u8]) -> Result<Vec<Star>, String> {
    let stars = csv::Reader::from_reader(bytes)
        .deserialize::<Star>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let stars: Vec<_> = stars
        .into_iter()
        .filter(|s| {
            s.id != 0
                && s.mag.is_finite()
                && s.mag <= 6.0
                && s.ra.is_finite()
                && (0.0..24.0).contains(&s.ra)
                && s.dec.is_finite()
                && (-90.0..=90.0).contains(&s.dec)
        })
        .collect();
    if stars.is_empty() {
        Err("Catalog contained no usable stars".into())
    } else {
        Ok(stars)
    }
}
pub fn sidereal(unix: i64, longitude: f64) -> f64 {
    let d = unix as f64 / 86400.0 + 2440587.5 - 2451545.0;
    (280.46061837 + 360.98564736629 * d + longitude).rem_euclid(360.0)
}
/// Geometric altitude and azimuth (north=0°, east=90°), precessed from J2000.
pub fn horizontal(ra: f64, dec: f64, lat: f64, lon: f64, unix: i64) -> (f64, f64) {
    let t = (unix as f64 / 86400.0 + 2440587.5 - 2451545.0) / 36525.0;
    let zeta = ((2306.2181 * t + 0.30188 * t * t + 0.017998 * t * t * t) / 3600.0).to_radians();
    let z = ((2306.2181 * t + 1.09468 * t * t + 0.018203 * t * t * t) / 3600.0).to_radians();
    let theta = ((2004.3109 * t - 0.42665 * t * t - 0.041833 * t * t * t) / 3600.0).to_radians();
    let a = (ra * 15.0).to_radians() + zeta;
    let d = dec.to_radians();
    let b = theta.cos() * d.cos() * a.cos() - theta.sin() * d.sin();
    let ra_date = (d.cos() * a.sin()).atan2(b) + z;
    let dec_date = (theta.sin() * d.cos() * a.cos() + theta.cos() * d.sin())
        .clamp(-1.0, 1.0)
        .asin();
    let h = sidereal(unix, lon).to_radians() - ra_date;
    let p = lat.to_radians();
    let alt = (p.sin() * dec_date.sin() + p.cos() * dec_date.cos() * h.cos())
        .clamp(-1.0, 1.0)
        .asin();
    let az = (-dec_date.cos() * h.sin())
        .atan2(dec_date.sin() * p.cos() - dec_date.cos() * h.cos() * p.sin());
    (alt.to_degrees(), az.to_degrees().rem_euclid(360.0))
}
pub fn project(alt: f64, az: f64) -> (f64, f64) {
    let r = (90.0 - alt) / 90.0;
    (-r * az.to_radians().sin(), r * az.to_radians().cos())
}
pub fn direction(az: f64) -> &'static str {
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][((az / 45.0 + 0.5) as usize) % 8]
}
#[cfg(test)]
mod tests {
    use super::*;
    const J2000: i64 = 946728000;
    #[test]
    fn zenith_and_nadir() {
        let ra = sidereal(J2000, 0.0) / 15.0;
        assert!((horizontal(ra, 30.0, 30.0, 0.0, J2000).0 - 90.0).abs() < 1e-5);
        assert!((horizontal((ra + 12.0) % 24.0, -30.0, 30.0, 0.0, J2000).0 + 90.0).abs() < 1e-5);
    }
    #[test]
    fn east_horizon() {
        let ra = ((sidereal(J2000, 0.0) + 90.0) % 360.0) / 15.0;
        let (alt, az) = horizontal(ra, 0.0, 0.0, 0.0, J2000);
        assert!(alt.abs() < 1e-6);
        assert!((az - 90.0).abs() < 1e-6);
    }
    #[test]
    fn bundled_catalog_is_real_and_bright() {
        let stars = catalog(include_bytes!("../data/bright_stars.csv")).unwrap();
        assert!(stars.len() > 4000);
        assert!(stars.iter().any(|s| s.proper == "Sirius" && s.mag < -1.0));
    }
}
