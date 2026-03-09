/// Lambert Conformal Conic projection math.
///
/// Converts between (lat, lon) geographic coordinates and (x, y) grid coordinates
/// for the HRRR's native Lambert Conformal Conic projection.

use std::f64::consts::PI;

const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;

/// Earth radius in meters (GRS80/WGS84 sphere used by GRIB2)
const EARTH_RADIUS: f64 = 6_371_229.0;

/// Lambert Conformal Conic projection parameters.
#[derive(Debug, Clone)]
pub struct LambertProjection {
    /// First standard parallel (radians)
    pub latin1: f64,
    /// Second standard parallel (radians)
    pub latin2: f64,
    /// Longitude of orientation (radians)
    pub lov: f64,
    /// Latitude of first grid point (radians)
    pub la1: f64,
    /// Longitude of first grid point (radians)
    pub lo1: f64,
    /// Grid spacing X (meters)
    pub dx: f64,
    /// Grid spacing Y (meters)
    pub dy: f64,
    /// Number of grid points in X
    pub nx: u32,
    /// Number of grid points in Y
    pub ny: u32,

    // Derived constants
    n: f64,
    f_val: f64,
    _rho0: f64,
    rho1: f64,
    theta1: f64,
}

impl LambertProjection {
    /// Create a new Lambert Conformal Conic projection from HRRR grid parameters.
    /// All angle inputs are in degrees.
    pub fn new(
        latin1_deg: f64,
        latin2_deg: f64,
        lov_deg: f64,
        la1_deg: f64,
        lo1_deg: f64,
        dx: f64,
        dy: f64,
        nx: u32,
        ny: u32,
    ) -> Self {
        let latin1 = latin1_deg * DEG_TO_RAD;
        let latin2 = latin2_deg * DEG_TO_RAD;
        let lov = lov_deg * DEG_TO_RAD;
        let la1 = la1_deg * DEG_TO_RAD;
        let lo1 = lo1_deg * DEG_TO_RAD;

        // Compute cone constant n
        let n = if (latin1 - latin2).abs() < 1e-10 {
            latin1.sin()
        } else {
            let ln_ratio = ((PI / 4.0 + latin2 / 2.0).tan().ln())
                - ((PI / 4.0 + latin1 / 2.0).tan().ln());
            (latin1.cos().ln() - latin2.cos().ln()) / ln_ratio
        };

        let f_val = (latin1.cos() * (PI / 4.0 + latin1 / 2.0).tan().powf(n)) / n;
        let _rho0 = EARTH_RADIUS * f_val; // rho at lat=90 would be 0 for n>0

        // rho and theta for the first grid point
        let rho1 = EARTH_RADIUS * f_val / (PI / 4.0 + la1 / 2.0).tan().powf(n);
        let theta1 = n * (lo1 - lov);

        LambertProjection {
            latin1, latin2, lov, la1, lo1, dx, dy, nx, ny,
            n, f_val, _rho0, rho1, theta1,
        }
    }

    /// Create from HRRR's standard grid parameters.
    pub fn hrrr_default() -> Self {
        Self::new(
            38.5,    // latin1
            38.5,    // latin2
            -97.5,   // lov (262.5 - 360)
            21.138,  // la1
            -122.72, // lo1 (237.28 - 360)
            3000.0,  // dx meters
            3000.0,  // dy meters
            1799,    // nx
            1059,    // ny
        )
    }

    /// Convert (lat, lon) in degrees to fractional grid (i, j) coordinates.
    /// Returns (i, j) where (0,0) is the first grid point.
    pub fn latlon_to_grid(&self, lat_deg: f64, lon_deg: f64) -> (f64, f64) {
        let lat = lat_deg * DEG_TO_RAD;
        let lon = lon_deg * DEG_TO_RAD;

        let rho = EARTH_RADIUS * self.f_val / (PI / 4.0 + lat / 2.0).tan().powf(self.n);
        let theta = self.n * (lon - self.lov);

        // Grid coordinates relative to the first point
        let x = rho * theta.sin() - self.rho1 * self.theta1.sin();
        let y = self.rho1 * self.theta1.cos() - rho * theta.cos();

        let i = x / self.dx;
        let j = y / self.dy;

        (i, j)
    }

    /// Convert grid (i, j) to (lat, lon) in degrees.
    pub fn grid_to_latlon(&self, i: f64, j: f64) -> (f64, f64) {
        let x = self.rho1 * self.theta1.sin() + i * self.dx;
        let y = self.rho1 * self.theta1.cos() - j * self.dy;

        let rho = (x * x + y * y).sqrt() * self.n.signum();
        let theta = x.atan2(y); // atan2(x, y) for Lambert Conformal convention

        let lat = (2.0 * ((EARTH_RADIUS * self.f_val / rho).powf(1.0 / self.n)).atan()
            - PI / 2.0)
            * RAD_TO_DEG;
        let mut lon = (self.lov + theta / self.n) * RAD_TO_DEG;

        // Normalize longitude to [-180, 180]
        while lon > 180.0 { lon -= 360.0; }
        while lon < -180.0 { lon += 360.0; }

        (lat, lon)
    }

    /// Get the bounding box of the grid in (lat, lon) degrees.
    /// Returns (min_lat, min_lon, max_lat, max_lon).
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let (lat0, lon0) = self.grid_to_latlon(0.0, 0.0);
        let (lat1, lon1) = self.grid_to_latlon(self.nx as f64 - 1.0, 0.0);
        let (lat2, lon2) = self.grid_to_latlon(0.0, self.ny as f64 - 1.0);
        let (lat3, lon3) = self.grid_to_latlon(self.nx as f64 - 1.0, self.ny as f64 - 1.0);

        let min_lat = lat0.min(lat1).min(lat2).min(lat3);
        let max_lat = lat0.max(lat1).max(lat2).max(lat3);
        let min_lon = lon0.min(lon1).min(lon2).min(lon3);
        let max_lon = lon0.max(lon1).max(lon2).max(lon3);

        (min_lat, min_lon, max_lat, max_lon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let proj = LambertProjection::hrrr_default();

        // Grid point (0,0) should map back to (la1, lo1) approximately
        let (lat, lon) = proj.grid_to_latlon(0.0, 0.0);
        assert!((lat - 21.138).abs() < 0.1, "lat={}", lat);
        assert!((lon - (-122.72)).abs() < 0.1, "lon={}", lon);

        // Round-trip test for a point in the middle of the grid
        let (lat_mid, lon_mid) = proj.grid_to_latlon(900.0, 530.0);
        let (i, j) = proj.latlon_to_grid(lat_mid, lon_mid);
        assert!((i - 900.0).abs() < 0.01, "i={}", i);
        assert!((j - 530.0).abs() < 0.01, "j={}", j);
    }
}
