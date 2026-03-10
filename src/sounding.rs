/// HRRR model sounding extraction.
///
/// Fetches vertical profiles from HRRR pressure level data (wrfprsf##.grib2)
/// at any lat/lon point within the CONUS domain.

use std::io;
use crate::fetch;
use crate::render::projection::LambertProjection;

/// A single level in a model sounding.
#[derive(Debug, Clone)]
pub struct ModelSoundingLevel {
    pub pressure_mb: f64,
    pub height_m: f64,
    pub temp_c: f64,
    pub dewpoint_c: f64,
    pub wind_dir: f64,      // degrees
    pub wind_speed_kts: f64, // knots
}

/// A complete model sounding profile.
#[derive(Debug, Clone)]
pub struct ModelSounding {
    pub levels: Vec<ModelSoundingLevel>,
    pub lat: f64,
    pub lon: f64,
    pub run_date: String,
    pub run_hour: u8,
    pub forecast_hour: u8,
}

/// Pressure levels to extract (mb). Using every-other level from full HRRR output
/// to keep download reasonable while maintaining good vertical resolution.
const SOUNDING_LEVELS: &[u16] = &[
    1000, 975, 950, 925, 900, 875, 850, 825, 800, 775,
    750, 725, 700, 675, 650, 625, 600, 575, 550, 525,
    500, 450, 400, 350, 300, 250, 200, 150, 100,
];
// 29 levels × 5 vars = 145 fields

/// Fetch a model sounding at the given lat/lon.
///
/// Uses the HRRR pressure file (wrfprsf) to extract TMP, DPT, HGT, UGRD, VGRD
/// at ~29 pressure levels. Downloads are parallelized.
pub fn fetch_model_sounding(
    run: &str,
    forecast_hour: u8,
    lat: f64,
    lon: f64,
    status_fn: &dyn Fn(&str),
) -> io::Result<ModelSounding> {
    let (date, run_hour) = fetch::parse_run(run)?;

    // Set up projection to find nearest grid point
    let proj = LambertProjection::hrrr_default();
    let (gi, gj) = proj.latlon_to_grid(lat, lon);
    let ix = gi.round() as usize;
    let iy = gj.round() as usize;

    if ix >= 1799 || iy >= 1059 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("Location ({}, {}) is outside HRRR domain", lat, lon)));
    }

    // Build field list: 5 vars × N levels
    let mut field_specs: Vec<(String, String)> = Vec::new();
    for &pres in SOUNDING_LEVELS {
        let level = format!("{} mb", pres);
        for var in &["TMP", "DPT", "HGT", "UGRD", "VGRD"] {
            field_specs.push((var.to_string(), level.clone()));
        }
    }

    let fields_ref: Vec<(&str, &str)> = field_specs.iter()
        .map(|(n, l)| (n.as_str(), l.as_str()))
        .collect();

    status_fn(&format!("Fetching {} fields from pressure file...", fields_ref.len()));

    // Use the pressure file product
    let results = fetch::fetch_fields_parallel_product(
        &date, run_hour, forecast_hour, &fields_ref, "wrfprsf"
    )?;

    status_fn("Parsing sounding data...");

    // Parse each field and extract value at the target grid point
    let nx = 1799usize;
    let mut levels: Vec<ModelSoundingLevel> = Vec::new();

    let vars_per_level = 5; // TMP, DPT, HGT, UGRD, VGRD

    for (li, &pres) in SOUNDING_LEVELS.iter().enumerate() {
        let base_idx = li * vars_per_level;

        // Parse each variable and extract the grid point value
        let tmp_val = extract_grid_value(&results[base_idx], ix, iy, nx)?;
        let dpt_val = extract_grid_value(&results[base_idx + 1], ix, iy, nx)?;
        let hgt_val = extract_grid_value(&results[base_idx + 2], ix, iy, nx)?;
        let u_val = extract_grid_value(&results[base_idx + 3], ix, iy, nx)?;
        let v_val = extract_grid_value(&results[base_idx + 4], ix, iy, nx)?;

        // Skip if any value is NaN
        if tmp_val.is_nan() || dpt_val.is_nan() || hgt_val.is_nan() {
            continue;
        }

        // Convert: GRIB2 temperatures are in Kelvin
        let temp_c = tmp_val - 273.15;
        let dewpoint_c = dpt_val - 273.15;

        // Convert winds from m/s to knots and compute dir/speed
        let u_kts = u_val * 1.94384;
        let v_kts = v_val * 1.94384;
        let wind_speed = (u_kts * u_kts + v_kts * v_kts).sqrt();
        let wind_dir = if wind_speed < 0.5 {
            0.0
        } else {
            // Meteorological convention: direction wind is FROM
            let dir = (270.0 - v_val.atan2(u_val).to_degrees()) % 360.0;
            if dir < 0.0 { dir + 360.0 } else { dir }
        };

        levels.push(ModelSoundingLevel {
            pressure_mb: pres as f64,
            height_m: hgt_val,
            temp_c,
            dewpoint_c,
            wind_dir,
            wind_speed_kts: wind_speed,
        });
    }

    // Sort by decreasing pressure (surface first)
    levels.sort_by(|a, b| b.pressure_mb.partial_cmp(&a.pressure_mb)
        .unwrap_or(std::cmp::Ordering::Equal));

    // Also fetch surface data from the surface file for the surface level
    // Surface has PRES, TMP, DPT at surface, and UGRD/VGRD at 10m
    if let Ok(sfc_level) = fetch_surface_level(&date, run_hour, forecast_hour, ix, iy, nx) {
        // Insert surface level at the beginning if its pressure is higher than the first level
        if levels.is_empty() || sfc_level.pressure_mb > levels[0].pressure_mb {
            levels.insert(0, sfc_level);
        }
    }

    status_fn(&format!("Sounding complete: {} levels", levels.len()));

    Ok(ModelSounding {
        levels,
        lat,
        lon,
        run_date: date,
        run_hour,
        forecast_hour,
    })
}

/// Extract a single grid point value from a GRIB2 field buffer.
fn extract_grid_value(grib_data: &[u8], ix: usize, iy: usize, _nx: usize) -> io::Result<f64> {
    let (values, actual_nx, _ny) = crate::parse_grib2_field(grib_data)?;
    let idx = iy * actual_nx + ix;
    if idx < values.len() {
        Ok(values[idx])
    } else {
        Ok(f64::NAN)
    }
}

/// Fetch the surface level from the surface file (wrfsfcf).
fn fetch_surface_level(
    date: &str, run_hour: u8, forecast_hour: u8,
    ix: usize, iy: usize, nx: usize,
) -> io::Result<ModelSoundingLevel> {
    let fields: &[(&str, &str)] = &[
        ("PRES", "surface"),
        ("TMP", "2 m above ground"),
        ("DPT", "2 m above ground"),
        ("UGRD", "10 m above ground"),
        ("VGRD", "10 m above ground"),
        ("HGT", "surface"),
    ];

    let results = fetch::fetch_fields_parallel(date, run_hour, forecast_hour, fields)?;

    let pres = extract_grid_value(&results[0], ix, iy, nx)?;
    let tmp = extract_grid_value(&results[1], ix, iy, nx)?;
    let dpt = extract_grid_value(&results[2], ix, iy, nx)?;
    let u = extract_grid_value(&results[3], ix, iy, nx)?;
    let v = extract_grid_value(&results[4], ix, iy, nx)?;
    let hgt = extract_grid_value(&results[5], ix, iy, nx)?;

    let temp_c = tmp - 273.15;
    let dewpoint_c = dpt - 273.15;
    let u_kts = u * 1.94384;
    let v_kts = v * 1.94384;
    let wind_speed = (u_kts * u_kts + v_kts * v_kts).sqrt();
    let wind_dir = if wind_speed < 0.5 {
        0.0
    } else {
        let dir = (270.0 - v.atan2(u).to_degrees()) % 360.0;
        if dir < 0.0 { dir + 360.0 } else { dir }
    };

    Ok(ModelSoundingLevel {
        pressure_mb: pres / 100.0, // Pa to mb
        height_m: hgt,
        temp_c,
        dewpoint_c,
        wind_dir,
        wind_speed_kts: wind_speed,
    })
}
