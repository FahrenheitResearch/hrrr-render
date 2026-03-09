/// Grid Definition Templates for GRIB2 Section 3.
///
/// HRRR uses Template 30: Lambert Conformal Conic projection.

use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Cursor, Read};

/// Lambert Conformal Conic grid definition (Template 3.30)
#[derive(Debug, Clone)]
pub struct LambertConformal {
    pub nx: u32,
    pub ny: u32,
    pub la1: f64,      // latitude of first grid point (degrees)
    pub lo1: f64,      // longitude of first grid point (degrees)
    pub lad: f64,      // LaD - latitude where Dx/Dy are specified (degrees)
    pub lov: f64,      // LoV - orientation of the grid (degrees)
    pub dx: f64,       // X-direction grid length (m)
    pub dy: f64,       // Y-direction grid length (m)
    pub latin1: f64,   // first standard parallel (degrees)
    pub latin2: f64,   // second standard parallel (degrees)
    pub scan_mode: u8, // scanning mode flags
}

impl LambertConformal {
    /// Parse Template 3.30 from the grid definition section data.
    /// `data` starts after the common Section 3 header (source, num_points, etc.)
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);

        // Shape of the Earth (1 byte) - usually 6 = spherical with radius 6371229m
        let _shape = cur.read_u8()?;
        let _scale_factor_radius = cur.read_u8()?;
        let _scaled_radius = cur.read_u32::<BigEndian>()?;
        let _scale_factor_major = cur.read_u8()?;
        let _scaled_major = cur.read_u32::<BigEndian>()?;
        let _scale_factor_minor = cur.read_u8()?;
        let _scaled_minor = cur.read_u32::<BigEndian>()?;

        let nx = cur.read_u32::<BigEndian>()?;
        let ny = cur.read_u32::<BigEndian>()?;

        let la1 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let lo1 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;

        let _resolution = cur.read_u8()?;

        let lad = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let lov = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;

        let dx = cur.read_u32::<BigEndian>()? as f64 / 1000.0; // mm to m
        let dy = cur.read_u32::<BigEndian>()? as f64 / 1000.0;

        let _projection_center = cur.read_u8()?;
        let scan_mode = cur.read_u8()?;

        let latin1 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let latin2 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;

        let _lat_south_pole = cur.read_i32::<BigEndian>()?;
        let _lon_south_pole = cur.read_i32::<BigEndian>()?;

        Ok(LambertConformal {
            nx, ny, la1, lo1, lad, lov, dx, dy, latin1, latin2, scan_mode,
        })
    }
}

/// Latitude/Longitude grid definition (Template 3.0) - for completeness
#[derive(Debug, Clone)]
pub struct LatLon {
    pub nx: u32,
    pub ny: u32,
    pub la1: f64,
    pub lo1: f64,
    pub la2: f64,
    pub lo2: f64,
    pub dx: f64,
    pub dy: f64,
    pub scan_mode: u8,
}

impl LatLon {
    pub fn parse(data: &[u8]) -> io::Result<Self> {
        let mut cur = Cursor::new(data);

        // Earth shape fields (14 bytes)
        let mut skip = [0u8; 14];
        cur.read_exact(&mut skip)?;

        let nx = cur.read_u32::<BigEndian>()?;
        let ny = cur.read_u32::<BigEndian>()?;
        let _basic_angle = cur.read_u32::<BigEndian>()?;
        let _subdiv = cur.read_u32::<BigEndian>()?;

        let la1 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let lo1 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let _resolution = cur.read_u8()?;
        let la2 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let lo2 = cur.read_i32::<BigEndian>()? as f64 / 1_000_000.0;
        let dx = cur.read_u32::<BigEndian>()? as f64 / 1_000_000.0;
        let dy = cur.read_u32::<BigEndian>()? as f64 / 1_000_000.0;
        let scan_mode = cur.read_u8()?;

        Ok(LatLon { nx, ny, la1, lo1, la2, lo2, dx, dy, scan_mode })
    }
}
