/// Composite (derived) field computation from multiple GRIB2 fields.
///
/// These fields are not directly in the HRRR output — they're computed
/// from component fields, matching SPC methodology.

use std::io;
use crate::fetch;

/// Check if a field name is a composite that requires multi-fetch.
pub fn is_composite(name: &str) -> bool {
    matches!(name, "stp" | "scp" | "ship" | "shr01" | "shr06" | "ebs")
}

/// Composite field definitions for the GUI field list.
pub struct CompositeFieldDef {
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub value_range: (f64, f64),
    pub group: &'static str,
}

pub static COMPOSITE_FIELDS: &[CompositeFieldDef] = &[
    CompositeFieldDef {
        name: "shr01", label: "0-1km Bulk Shear", unit: "kt",
        value_range: (0.0, 80.0), group: "Tornado",
    },
    CompositeFieldDef {
        name: "shr06", label: "0-6km Bulk Shear", unit: "kt",
        value_range: (0.0, 100.0), group: "Tornado",
    },
    CompositeFieldDef {
        name: "ebs", label: "Effective Bulk Shear", unit: "kt",
        value_range: (0.0, 100.0), group: "Tornado",
    },
    CompositeFieldDef {
        name: "stp", label: "Sig Tornado Parameter", unit: "",
        value_range: (0.0, 12.0), group: "Tornado",
    },
    CompositeFieldDef {
        name: "scp", label: "Supercell Composite", unit: "",
        value_range: (0.0, 20.0), group: "Tornado",
    },
    CompositeFieldDef {
        name: "ship", label: "Sig Hail Parameter", unit: "",
        value_range: (0.0, 5.0), group: "Tornado",
    },
];

/// Compute a composite field. Returns (values, nx, ny).
pub fn compute_composite(
    name: &str, date: &str, run_hour: u8, fhour: u8,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    match name {
        "shr01" => compute_bulk_shear(date, run_hour, fhour, "0-1000 m above ground", status_fn),
        "shr06" => compute_bulk_shear(date, run_hour, fhour, "0-6000 m above ground", status_fn),
        "ebs" => compute_effective_bulk_shear(date, run_hour, fhour, status_fn),
        "stp" => compute_stp(date, run_hour, fhour, status_fn),
        "scp" => compute_scp(date, run_hour, fhour, status_fn),
        "ship" => compute_ship(date, run_hour, fhour, status_fn),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("Unknown composite: {}", name))),
    }
}

/// Compute bulk wind shear magnitude = sqrt(u² + v²), converted to knots.
fn compute_bulk_shear(
    date: &str, run_hour: u8, fhour: u8, level: &str,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    let fields: &[(&str, &str)] = &[
        ("VUCSH", level),
        ("VVCSH", level),
    ];

    status_fn("Fetching 2 components...");
    let results = fetch::fetch_fields_parallel(date, run_hour, fhour, fields)?;

    status_fn("Computing...");
    let (u, nx, ny) = crate::parse_grib2_field(&results[0])?;
    let (v, _, _) = crate::parse_grib2_field(&results[1])?;

    let n = u.len();
    let mut values = vec![f64::NAN; n];
    for i in 0..n {
        if !u[i].is_nan() && !v[i].is_nan() {
            // shear in 1/s, convert magnitude to knots
            values[i] = (u[i] * u[i] + v[i] * v[i]).sqrt() * 1.94384;
        }
    }

    Ok((values, nx, ny))
}

/// Effective bulk shear: uses storm motion (USTM/VSTM) layer.
/// Approximated as 0-6km shear when effective layer isn't directly available.
fn compute_effective_bulk_shear(
    date: &str, run_hour: u8, fhour: u8,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    // Use 0-6km as proxy for effective bulk shear
    compute_bulk_shear(date, run_hour, fhour, "0-6000 m above ground", status_fn)
}

/// Significant Tornado Parameter (fixed-layer, Thompson et al. 2003/2012)
///
/// STP = (MLCAPE/1500) * (ESRH/150) * (EBWD/12) * ((2000-MLLCL)/1000) * ((200+MLCIN)/150)
///
/// Using fixed layers: MLCAPE (90-0mb), 0-1km SRH, 0-6km shear, LCL height
fn compute_stp(
    date: &str, run_hour: u8, fhour: u8,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    let fields: &[(&str, &str)] = &[
        ("CAPE", "90-0 mb above ground"),
        ("CIN", "90-0 mb above ground"),
        ("HLCY", "1000-0 m above ground"),
        ("VUCSH", "0-6000 m above ground"),
        ("VVCSH", "0-6000 m above ground"),
        ("HGT", "level of adiabatic condensation from sfc"),
    ];

    status_fn("Fetching 6 components...");
    let results = fetch::fetch_fields_parallel(date, run_hour, fhour, fields)?;

    status_fn("Computing...");
    let (mlcape, nx, ny) = crate::parse_grib2_field(&results[0])?;
    let (mlcin, _, _) = crate::parse_grib2_field(&results[1])?;
    let (srh1, _, _) = crate::parse_grib2_field(&results[2])?;
    let (shr_u, _, _) = crate::parse_grib2_field(&results[3])?;
    let (shr_v, _, _) = crate::parse_grib2_field(&results[4])?;
    let (lcl, _, _) = crate::parse_grib2_field(&results[5])?;

    let n = mlcape.len();
    let mut values = vec![f64::NAN; n];

    for i in 0..n {
        let cape = mlcape.get(i).copied().unwrap_or(f64::NAN);
        let cin = mlcin.get(i).copied().unwrap_or(f64::NAN);
        let srh = srh1.get(i).copied().unwrap_or(f64::NAN);
        let su = shr_u.get(i).copied().unwrap_or(f64::NAN);
        let sv = shr_v.get(i).copied().unwrap_or(f64::NAN);
        let lcl_m = lcl.get(i).copied().unwrap_or(f64::NAN);

        if cape.is_nan() || cin.is_nan() || srh.is_nan()
            || su.is_nan() || sv.is_nan() || lcl_m.is_nan() { continue; }

        let cape_term = (cape / 1500.0).min(1.0).max(0.0);
        if cape < 1.0 { values[i] = 0.0; continue; }

        let srh_term = srh / 150.0;

        let shear_kt = (su * su + sv * sv).sqrt() * 1.94384;
        let shear_term = if shear_kt < 12.0 { 0.0 }
            else if shear_kt > 30.0 { 1.5 }
            else { shear_kt / 20.0 };

        // LCL: lower LCL = better for tornadoes. Surface height varies,
        // but LCL here is AGL (from sfc condensation level)
        let lcl_term = if lcl_m < 1000.0 { 1.0 }
            else if lcl_m > 2000.0 { 0.0 }
            else { (2000.0 - lcl_m) / 1000.0 };

        // CIN: less inhibition = better. CIN is negative.
        let cin_term = if cin > -50.0 { 1.0 }
            else if cin < -200.0 { 0.0 }
            else { (200.0 + cin) / 150.0 };

        values[i] = (cape_term * srh_term * shear_term * lcl_term * cin_term).max(0.0);
    }

    Ok((values, nx, ny))
}

/// Supercell Composite Parameter (Thompson et al. 2003)
///
/// SCP = (MUCAPE/1000) * (ESRH/50) * (EBWD/40)
fn compute_scp(
    date: &str, run_hour: u8, fhour: u8,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    let fields: &[(&str, &str)] = &[
        ("CAPE", "180-0 mb above ground"),
        ("HLCY", "3000-0 m above ground"),
        ("VUCSH", "0-6000 m above ground"),
        ("VVCSH", "0-6000 m above ground"),
    ];

    status_fn("Fetching 4 components...");
    let results = fetch::fetch_fields_parallel(date, run_hour, fhour, fields)?;

    status_fn("Computing...");
    let (mucape, nx, ny) = crate::parse_grib2_field(&results[0])?;
    let (srh3, _, _) = crate::parse_grib2_field(&results[1])?;
    let (shr_u, _, _) = crate::parse_grib2_field(&results[2])?;
    let (shr_v, _, _) = crate::parse_grib2_field(&results[3])?;

    let n = mucape.len();
    let mut values = vec![f64::NAN; n];

    for i in 0..n {
        let cape = mucape.get(i).copied().unwrap_or(f64::NAN);
        let srh = srh3.get(i).copied().unwrap_or(f64::NAN);
        let su = shr_u.get(i).copied().unwrap_or(f64::NAN);
        let sv = shr_v.get(i).copied().unwrap_or(f64::NAN);

        if cape.is_nan() || srh.is_nan() || su.is_nan() || sv.is_nan() { continue; }

        if cape < 1.0 { values[i] = 0.0; continue; }

        let cape_term = cape / 1000.0;
        let srh_term = srh / 50.0;
        let shear_kt = (su * su + sv * sv).sqrt() * 1.94384;
        let shear_term = if shear_kt < 10.0 { 0.0 }
            else { shear_kt / 40.0 };

        values[i] = (cape_term * srh_term * shear_term).max(0.0);
    }

    Ok((values, nx, ny))
}

/// Significant Hail Parameter (modified SPC formulation)
///
/// SHIP = (MUCAPE * MR * LR75 * T500 * SHR06) / 42_000_000
fn compute_ship(
    date: &str, run_hour: u8, fhour: u8,
    status_fn: &dyn Fn(&str),
) -> io::Result<(Vec<f64>, usize, usize)> {
    let fields: &[(&str, &str)] = &[
        ("CAPE", "180-0 mb above ground"),
        ("TMP", "500 mb"),
        ("TMP", "700 mb"),
        ("DPT", "700 mb"),
        ("VUCSH", "0-6000 m above ground"),
        ("VVCSH", "0-6000 m above ground"),
    ];

    status_fn("Fetching 6 components...");
    let results = fetch::fetch_fields_parallel(date, run_hour, fhour, fields)?;

    status_fn("Computing...");
    let (mucape, nx, ny) = crate::parse_grib2_field(&results[0])?;
    let (t500, _, _) = crate::parse_grib2_field(&results[1])?;
    let (t700, _, _) = crate::parse_grib2_field(&results[2])?;
    let (td700, _, _) = crate::parse_grib2_field(&results[3])?;
    let (shr_u, _, _) = crate::parse_grib2_field(&results[4])?;
    let (shr_v, _, _) = crate::parse_grib2_field(&results[5])?;

    let n = mucape.len();
    let mut values = vec![f64::NAN; n];

    for i in 0..n {
        let cape = mucape.get(i).copied().unwrap_or(f64::NAN);
        let t5 = t500.get(i).copied().unwrap_or(f64::NAN);
        let t7 = t700.get(i).copied().unwrap_or(f64::NAN);
        let td7 = td700.get(i).copied().unwrap_or(f64::NAN);
        let su = shr_u.get(i).copied().unwrap_or(f64::NAN);
        let sv = shr_v.get(i).copied().unwrap_or(f64::NAN);

        if cape.is_nan() || t5.is_nan() || t7.is_nan()
            || td7.is_nan() || su.is_nan() || sv.is_nan() { continue; }

        if cape < 1.0 { values[i] = 0.0; continue; }

        // Mixing ratio proxy from 700mb dewpoint (g/kg)
        // Simple approximation: MR ≈ 621.97 * es(Td) / (P - es(Td))
        let td7c = td7 - 273.15;
        let es = 6.112 * (17.67 * td7c / (td7c + 243.5)).exp();
        let mr = 621.97 * es / (700.0 - es);

        // 700-500mb lapse rate (C/km) — approximate 700-500mb thickness as ~2.5km
        let t7c = t7 - 273.15;
        let t5c = t5 - 273.15;
        let lr75 = -(t5c - t7c) / 2.5; // positive = steeper

        // Freezing level contribution: use T500 (more negative = colder aloft = bigger hail)
        let t500_term = (-t5c).max(0.0);

        // Shear
        let shear_kt = (su * su + sv * sv).sqrt() * 1.94384;

        // SHIP approximation
        let ship = (cape * mr * lr75 * t500_term * shear_kt) / 42_000_000.0;
        values[i] = ship.max(0.0);
    }

    Ok((values, nx, ny))
}
