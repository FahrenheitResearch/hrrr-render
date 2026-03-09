/// Field definitions mapping CLI field names to GRIB2 parameter IDs
/// and .idx search strings.

/// A weather field that can be requested for rendering.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// CLI name (e.g., "temp2m")
    pub name: &'static str,
    /// Human-readable label
    pub label: &'static str,
    /// Unit string for display
    pub unit: &'static str,
    /// GRIB2 discipline
    pub discipline: u8,
    /// GRIB2 parameter category
    pub category: u8,
    /// GRIB2 parameter number
    pub number: u8,
    /// Level/surface string to match in .idx file (e.g., "2 m above ground")
    pub level: &'static str,
    /// The .idx field name (e.g., "TMP", "DPT", "REFC")
    pub idx_name: &'static str,
    /// Typical value range (min, max) for color mapping
    pub value_range: (f64, f64),
    /// Whether to convert from Kelvin to Fahrenheit for display
    pub kelvin_to_fahrenheit: bool,
    /// Category group for GUI organization
    pub group: &'static str,
}

/// All supported HRRR fields, organized by category.
pub static FIELDS: &[FieldDef] = &[
    // ── Surface / Thermodynamic ──────────────────────────────────────
    FieldDef {
        name: "temp2m", label: "2m Temperature", unit: "°F",
        discipline: 0, category: 0, number: 0,
        level: "2 m above ground", idx_name: "TMP",
        value_range: (-40.0, 120.0), kelvin_to_fahrenheit: true,
        group: "Surface",
    },
    FieldDef {
        name: "dewp2m", label: "2m Dewpoint", unit: "°F",
        discipline: 0, category: 0, number: 6,
        level: "2 m above ground", idx_name: "DPT",
        value_range: (-40.0, 90.0), kelvin_to_fahrenheit: true,
        group: "Surface",
    },
    FieldDef {
        name: "rh2m", label: "2m Relative Humidity", unit: "%",
        discipline: 0, category: 1, number: 1,
        level: "2 m above ground", idx_name: "RH",
        value_range: (0.0, 100.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
    FieldDef {
        name: "wind10m", label: "10m Wind Speed", unit: "kt",
        discipline: 0, category: 2, number: 1,
        level: "10 m above ground", idx_name: "WIND",
        value_range: (0.0, 80.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
    FieldDef {
        name: "gust", label: "Surface Gust", unit: "kt",
        discipline: 0, category: 2, number: 22,
        level: "surface", idx_name: "GUST",
        value_range: (0.0, 100.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
    FieldDef {
        name: "vis", label: "Visibility", unit: "mi",
        discipline: 0, category: 19, number: 0,
        level: "surface", idx_name: "VIS",
        value_range: (0.0, 10.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
    FieldDef {
        name: "mslp", label: "MSLP", unit: "mb",
        discipline: 0, category: 3, number: 192,
        level: "mean sea level", idx_name: "MSLMA",
        value_range: (980.0, 1050.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
    FieldDef {
        name: "sfct", label: "Surface Temperature", unit: "°F",
        discipline: 0, category: 0, number: 0,
        level: "surface", idx_name: "TMP",
        value_range: (-40.0, 140.0), kelvin_to_fahrenheit: true,
        group: "Surface",
    },

    // ── Reflectivity / Radar ─────────────────────────────────────────
    FieldDef {
        name: "ref", label: "Composite Reflectivity", unit: "dBZ",
        discipline: 0, category: 16, number: 196,
        level: "entire atmosphere", idx_name: "REFC",
        value_range: (-10.0, 75.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },
    FieldDef {
        name: "ref1km", label: "1km Reflectivity", unit: "dBZ",
        discipline: 0, category: 16, number: 195,
        level: "1000 m above ground", idx_name: "REFD",
        value_range: (-10.0, 75.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },
    FieldDef {
        name: "ref4km", label: "4km Reflectivity", unit: "dBZ",
        discipline: 0, category: 16, number: 195,
        level: "4000 m above ground", idx_name: "REFD",
        value_range: (-10.0, 75.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },
    FieldDef {
        name: "retop", label: "Echo Top", unit: "kft",
        discipline: 0, category: 16, number: 197,
        level: "cloud top", idx_name: "RETOP",
        value_range: (0.0, 60.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },
    FieldDef {
        name: "vil", label: "VIL", unit: "kg/m²",
        discipline: 0, category: 16, number: 199,
        level: "entire atmosphere", idx_name: "VIL",
        value_range: (0.0, 80.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },
    FieldDef {
        name: "maxref", label: "Hourly Max Reflectivity", unit: "dBZ",
        discipline: 0, category: 16, number: 196,
        level: "1000 m above ground", idx_name: "MAXREF",
        value_range: (-10.0, 75.0), kelvin_to_fahrenheit: false,
        group: "Radar",
    },

    // ── Convective / Severe ──────────────────────────────────────────
    FieldDef {
        name: "sbcape", label: "Surface-Based CAPE", unit: "J/kg",
        discipline: 0, category: 7, number: 6,
        level: "surface", idx_name: "CAPE",
        value_range: (0.0, 5000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "sbcin", label: "Surface-Based CIN", unit: "J/kg",
        discipline: 0, category: 7, number: 7,
        level: "surface", idx_name: "CIN",
        value_range: (-300.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "mlcape", label: "Mixed-Layer CAPE", unit: "J/kg",
        discipline: 0, category: 7, number: 6,
        level: "90-0 mb above ground", idx_name: "CAPE",
        value_range: (0.0, 5000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "mlcin", label: "Mixed-Layer CIN", unit: "J/kg",
        discipline: 0, category: 7, number: 7,
        level: "90-0 mb above ground", idx_name: "CIN",
        value_range: (-300.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "mucape", label: "Most-Unstable CAPE", unit: "J/kg",
        discipline: 0, category: 7, number: 6,
        level: "180-0 mb above ground", idx_name: "CAPE",
        value_range: (0.0, 6000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "mucin", label: "Most-Unstable CIN", unit: "J/kg",
        discipline: 0, category: 7, number: 7,
        level: "180-0 mb above ground", idx_name: "CIN",
        value_range: (-300.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "cape03", label: "0-3km CAPE", unit: "J/kg",
        discipline: 0, category: 7, number: 6,
        level: "0-3000 m above ground", idx_name: "CAPE",
        value_range: (0.0, 500.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "lftx", label: "Lifted Index", unit: "°C",
        discipline: 0, category: 7, number: 192,
        level: "500-1000 mb", idx_name: "LFTX",
        value_range: (-12.0, 12.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "4lftx", label: "Best Lifted Index", unit: "°C",
        discipline: 0, category: 7, number: 193,
        level: "180-0 mb above ground", idx_name: "4LFTX",
        value_range: (-12.0, 12.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "lfc", label: "Level of Free Convection", unit: "m",
        discipline: 0, category: 3, number: 5,
        level: "level of free convection", idx_name: "HGT",
        value_range: (0.0, 5000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "el", label: "Equilibrium Level", unit: "m",
        discipline: 0, category: 3, number: 5,
        level: "equilibrium level", idx_name: "HGT",
        value_range: (0.0, 18000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "lcl", label: "LCL Height", unit: "m",
        discipline: 0, category: 3, number: 5,
        level: "level of adiabatic condensation from sfc", idx_name: "HGT",
        value_range: (0.0, 4000.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },

    // ── Shear / Helicity / Storm Motion ──────────────────────────────
    FieldDef {
        name: "srh3km", label: "0-3km SRH", unit: "m²/s²",
        discipline: 0, category: 7, number: 8,
        level: "3000-0 m above ground", idx_name: "HLCY",
        value_range: (-100.0, 600.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },
    FieldDef {
        name: "srh1km", label: "0-1km SRH", unit: "m²/s²",
        discipline: 0, category: 7, number: 8,
        level: "1000-0 m above ground", idx_name: "HLCY",
        value_range: (-100.0, 500.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },
    FieldDef {
        name: "efhl", label: "Effective Helicity", unit: "m²/s²",
        discipline: 0, category: 7, number: 192,
        level: "surface", idx_name: "EFHL",
        value_range: (-100.0, 600.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },
    FieldDef {
        name: "cangle", label: "Critical Angle", unit: "°",
        discipline: 0, category: 2, number: 0,
        level: "0-500 m above ground", idx_name: "CANGLE",
        value_range: (0.0, 180.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },
    FieldDef {
        name: "ustm", label: "Bunkers Storm Motion U", unit: "kt",
        discipline: 0, category: 2, number: 27,
        level: "0-6000 m above ground", idx_name: "USTM",
        value_range: (-40.0, 40.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },
    FieldDef {
        name: "vstm", label: "Bunkers Storm Motion V", unit: "kt",
        discipline: 0, category: 2, number: 28,
        level: "0-6000 m above ground", idx_name: "VSTM",
        value_range: (-40.0, 40.0), kelvin_to_fahrenheit: false,
        group: "Shear",
    },

    // ── Updraft Helicity ─────────────────────────────────────────────
    FieldDef {
        name: "uh25", label: "Max UH 2-5km", unit: "m²/s²",
        discipline: 0, category: 7, number: 199,
        level: "5000-2000 m above ground", idx_name: "MXUPHL",
        value_range: (0.0, 250.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "uh02", label: "Max UH 0-2km", unit: "m²/s²",
        discipline: 0, category: 7, number: 199,
        level: "2000-0 m above ground", idx_name: "MXUPHL",
        value_range: (0.0, 200.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "uh03", label: "Max UH 0-3km", unit: "m²/s²",
        discipline: 0, category: 7, number: 199,
        level: "3000-0 m above ground", idx_name: "MXUPHL",
        value_range: (0.0, 200.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "mnuh02", label: "Min UH 0-2km", unit: "m²/s²",
        discipline: 0, category: 7, number: 200,
        level: "2000-0 m above ground", idx_name: "MNUPHL",
        value_range: (-200.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "maxuvv", label: "Max Updraft Velocity", unit: "m/s",
        discipline: 0, category: 2, number: 220,
        level: "100-1000 mb above ground", idx_name: "MAXUVV",
        value_range: (0.0, 50.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "maxdvv", label: "Max Downdraft Velocity", unit: "m/s",
        discipline: 0, category: 2, number: 221,
        level: "100-1000 mb above ground", idx_name: "MAXDVV",
        value_range: (-30.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "relv2km", label: "0-2km Relative Vorticity", unit: "1/s",
        discipline: 0, category: 2, number: 12,
        level: "2000-0 m above ground", idx_name: "RELV",
        value_range: (-0.01, 0.01), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },
    FieldDef {
        name: "relv1km", label: "0-1km Relative Vorticity", unit: "1/s",
        discipline: 0, category: 2, number: 12,
        level: "1000-0 m above ground", idx_name: "RELV",
        value_range: (-0.01, 0.01), kelvin_to_fahrenheit: false,
        group: "Mesoscale",
    },

    // ── Hail / Lightning ─────────────────────────────────────────────
    FieldDef {
        name: "hail", label: "Max Hail Size", unit: "in",
        discipline: 0, category: 1, number: 34,
        level: "entire atmosphere", idx_name: "HAIL",
        value_range: (0.0, 4.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "hailsfc", label: "Surface Hail", unit: "in",
        discipline: 0, category: 1, number: 34,
        level: "surface", idx_name: "HAIL",
        value_range: (0.0, 4.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "ltng", label: "Lightning Threat", unit: "index",
        discipline: 0, category: 17, number: 192,
        level: "entire atmosphere", idx_name: "LTNG",
        value_range: (0.0, 5.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },
    FieldDef {
        name: "graupel", label: "Total Column Graupel", unit: "kg/m²",
        discipline: 0, category: 1, number: 74,
        level: "entire atmosphere", idx_name: "TCOLG",
        value_range: (0.0, 10.0), kelvin_to_fahrenheit: false,
        group: "Severe",
    },

    // ── Moisture / Precipitation ─────────────────────────────────────
    FieldDef {
        name: "pwat", label: "Precipitable Water", unit: "in",
        discipline: 0, category: 1, number: 3,
        level: "entire atmosphere", idx_name: "PWAT",
        value_range: (0.0, 3.0), kelvin_to_fahrenheit: false,
        group: "Moisture",
    },
    FieldDef {
        name: "precip", label: "Precipitation Rate", unit: "in/hr",
        discipline: 0, category: 1, number: 7,
        level: "surface", idx_name: "PRATE",
        value_range: (0.0, 2.0), kelvin_to_fahrenheit: false,
        group: "Moisture",
    },
    FieldDef {
        name: "apcp", label: "Total Precipitation", unit: "in",
        discipline: 0, category: 1, number: 8,
        level: "surface", idx_name: "APCP",
        value_range: (0.0, 3.0), kelvin_to_fahrenheit: false,
        group: "Moisture",
    },
    FieldDef {
        name: "cpofp", label: "Prob Frozen Precip", unit: "%",
        discipline: 0, category: 1, number: 39,
        level: "surface", idx_name: "CPOFP",
        value_range: (0.0, 100.0), kelvin_to_fahrenheit: false,
        group: "Moisture",
    },

    // ── Upper Air ────────────────────────────────────────────────────
    FieldDef {
        name: "h500", label: "500mb Heights", unit: "dam",
        discipline: 0, category: 3, number: 5,
        level: "500 mb", idx_name: "HGT",
        value_range: (480.0, 600.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "t500", label: "500mb Temperature", unit: "°C",
        discipline: 0, category: 0, number: 0,
        level: "500 mb", idx_name: "TMP",
        value_range: (-40.0, 0.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "h700", label: "700mb Heights", unit: "dam",
        discipline: 0, category: 3, number: 5,
        level: "700 mb", idx_name: "HGT",
        value_range: (280.0, 320.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "t700", label: "700mb Temperature", unit: "°C",
        discipline: 0, category: 0, number: 0,
        level: "700 mb", idx_name: "TMP",
        value_range: (-20.0, 20.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "h850", label: "850mb Heights", unit: "dam",
        discipline: 0, category: 3, number: 5,
        level: "850 mb", idx_name: "HGT",
        value_range: (120.0, 160.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "t850", label: "850mb Temperature", unit: "°C",
        discipline: 0, category: 0, number: 0,
        level: "850 mb", idx_name: "TMP",
        value_range: (-20.0, 40.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "td850", label: "850mb Dewpoint", unit: "°C",
        discipline: 0, category: 0, number: 6,
        level: "850 mb", idx_name: "DPT",
        value_range: (-30.0, 25.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },
    FieldDef {
        name: "wind250", label: "250mb Wind", unit: "kt",
        discipline: 0, category: 2, number: 2,
        level: "250 mb", idx_name: "UGRD",
        value_range: (0.0, 150.0), kelvin_to_fahrenheit: false,
        group: "Upper Air",
    },

    // ── Boundary Layer / Clouds ──────────────────────────────────────
    FieldDef {
        name: "pblh", label: "PBL Height", unit: "m",
        discipline: 0, category: 3, number: 196,
        level: "surface", idx_name: "HPBL",
        value_range: (0.0, 5000.0), kelvin_to_fahrenheit: false,
        group: "Boundary Layer",
    },
    FieldDef {
        name: "tcc", label: "Total Cloud Cover", unit: "%",
        discipline: 0, category: 6, number: 1,
        level: "entire atmosphere", idx_name: "TCDC",
        value_range: (0.0, 100.0), kelvin_to_fahrenheit: false,
        group: "Boundary Layer",
    },
    FieldDef {
        name: "ceil", label: "Cloud Ceiling", unit: "ft",
        discipline: 0, category: 3, number: 5,
        level: "cloud ceiling", idx_name: "HGT",
        value_range: (0.0, 25000.0), kelvin_to_fahrenheit: false,
        group: "Boundary Layer",
    },

    // ── Composite Indices ────────────────────────────────────────────
    FieldDef {
        name: "esp", label: "Enhanced Stretching Potential", unit: "",
        discipline: 0, category: 7, number: 0,
        level: "0-3000 m above ground", idx_name: "ESP",
        value_range: (0.0, 5.0), kelvin_to_fahrenheit: false,
        group: "Composites",
    },
    FieldDef {
        name: "fzlev", label: "Freezing Level", unit: "ft",
        discipline: 0, category: 3, number: 5,
        level: "0C isotherm", idx_name: "HGT",
        value_range: (0.0, 18000.0), kelvin_to_fahrenheit: false,
        group: "Composites",
    },
    FieldDef {
        name: "wind80m", label: "80m Wind Speed", unit: "kt",
        discipline: 0, category: 2, number: 2,
        level: "80 m above ground", idx_name: "UGRD",
        value_range: (0.0, 100.0), kelvin_to_fahrenheit: false,
        group: "Surface",
    },
];

/// Look up a field by its CLI name.
pub fn lookup_field(name: &str) -> Option<&'static FieldDef> {
    FIELDS.iter().find(|f| f.name == name)
}

/// Get the list of all field names (for CLI help).
pub fn field_names() -> Vec<&'static str> {
    FIELDS.iter().map(|f| f.name).collect()
}

/// Get all unique group names in order.
pub fn field_groups() -> Vec<&'static str> {
    let mut groups = Vec::new();
    for f in FIELDS {
        if !groups.contains(&f.group) {
            groups.push(f.group);
        }
    }
    groups
}

/// Get fields in a specific group.
pub fn fields_in_group(group: &str) -> Vec<&'static FieldDef> {
    FIELDS.iter().filter(|f| f.group == group).collect()
}

/// Convert Kelvin to Fahrenheit.
pub fn k_to_f(k: f64) -> f64 {
    (k - 273.15) * 9.0 / 5.0 + 32.0
}

/// Convert Kelvin to Celsius.
pub fn k_to_c(k: f64) -> f64 {
    k - 273.15
}

/// Convert m/s to knots.
pub fn ms_to_kt(ms: f64) -> f64 {
    ms * 1.94384
}

/// Convert meters to miles.
pub fn m_to_mi(m: f64) -> f64 {
    m / 1609.344
}

/// Convert meters to feet.
pub fn m_to_ft(m: f64) -> f64 {
    m * 3.28084
}

/// Convert meters to kilofeet.
pub fn m_to_kft(m: f64) -> f64 {
    m * 3.28084 / 1000.0
}

/// Convert kg/m2/s to in/hr.
pub fn kgm2s_to_inhr(v: f64) -> f64 {
    v * 3600.0 / 25.4
}

/// Convert kg/m2 to inches.
pub fn kgm2_to_in(v: f64) -> f64 {
    v / 25.4
}

/// Convert geopotential meters to decameters.
pub fn gpm_to_dam(gpm: f64) -> f64 {
    gpm / 10.0
}

/// Convert hail size from meters to inches.
pub fn m_to_in(m: f64) -> f64 {
    m * 39.3701
}

/// Convert Pascals to millibars.
pub fn pa_to_mb(pa: f64) -> f64 {
    pa / 100.0
}

/// Apply unit conversion for a given field.
pub fn convert_values(field: &FieldDef, values: &mut [f64]) {
    match field.name {
        "temp2m" | "dewp2m" | "sfct" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = k_to_f(*v); }
            }
        }
        "t500" | "t700" | "t850" | "td850" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = k_to_c(*v); }
            }
        }
        "wind10m" | "gust" | "ustm" | "vstm" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = ms_to_kt(*v); }
            }
        }
        "wind250" | "wind80m" => {
            // These are U-component, convert m/s to kt for display
            for v in values.iter_mut() {
                if !v.is_nan() { *v = v.abs() * 1.94384; }
            }
        }
        "vis" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = m_to_mi(*v); }
            }
        }
        "precip" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = kgm2s_to_inhr(*v); }
            }
        }
        "apcp" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = kgm2_to_in(*v); }
            }
        }
        "h500" | "h700" | "h850" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = gpm_to_dam(*v); }
            }
        }
        "retop" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = m_to_kft(*v); }
            }
        }
        "ceil" | "fzlev" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = m_to_ft(*v); }
            }
        }
        "hail" | "hailsfc" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = m_to_in(*v); }
            }
        }
        "mslp" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = pa_to_mb(*v); }
            }
        }
        "pwat" => {
            for v in values.iter_mut() {
                if !v.is_nan() { *v = kgm2_to_in(*v); }
            }
        }
        _ => {}
    }
}
