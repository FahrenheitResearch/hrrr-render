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
}

/// All supported HRRR fields.
pub static FIELDS: &[FieldDef] = &[
    FieldDef {
        name: "temp2m",
        label: "2m Temperature",
        unit: "F",
        discipline: 0,
        category: 0,
        number: 0,
        level: "2 m above ground",
        idx_name: "TMP",
        value_range: (-40.0, 120.0), // Fahrenheit
        kelvin_to_fahrenheit: true,
    },
    FieldDef {
        name: "dewp2m",
        label: "2m Dewpoint",
        unit: "F",
        discipline: 0,
        category: 0,
        number: 6,
        level: "2 m above ground",
        idx_name: "DPT",
        value_range: (-40.0, 90.0),
        kelvin_to_fahrenheit: true,
    },
    FieldDef {
        name: "wind10m",
        label: "10m Wind Speed",
        unit: "kt",
        discipline: 0,
        category: 2,
        number: 1,
        level: "10 m above ground",
        idx_name: "WIND",
        value_range: (0.0, 80.0), // knots
        kelvin_to_fahrenheit: false,
    },
    FieldDef {
        name: "ref",
        label: "Composite Reflectivity",
        unit: "dBZ",
        discipline: 0,
        category: 16,
        number: 196,
        level: "entire atmosphere",
        idx_name: "REFC",
        value_range: (-10.0, 75.0),
        kelvin_to_fahrenheit: false,
    },
    FieldDef {
        name: "cape",
        label: "CAPE",
        unit: "J/kg",
        discipline: 0,
        category: 7,
        number: 6,
        level: "surface",
        idx_name: "CAPE",
        value_range: (0.0, 5000.0),
        kelvin_to_fahrenheit: false,
    },
    FieldDef {
        name: "vis",
        label: "Visibility",
        unit: "mi",
        discipline: 0,
        category: 19,
        number: 0,
        level: "surface",
        idx_name: "VIS",
        value_range: (0.0, 10.0), // miles
        kelvin_to_fahrenheit: false,
    },
    FieldDef {
        name: "precip",
        label: "Precipitation Rate",
        unit: "in/hr",
        discipline: 0,
        category: 1,
        number: 7,
        level: "surface",
        idx_name: "PRATE",
        value_range: (0.0, 2.0), // in/hr
        kelvin_to_fahrenheit: false,
    },
    FieldDef {
        name: "h500",
        label: "500mb Heights",
        unit: "dam",
        discipline: 0,
        category: 3,
        number: 5,
        level: "500 mb",
        idx_name: "HGT",
        value_range: (480.0, 600.0), // decameters
        kelvin_to_fahrenheit: false,
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

/// Convert Kelvin to Fahrenheit.
pub fn k_to_f(k: f64) -> f64 {
    (k - 273.15) * 9.0 / 5.0 + 32.0
}

/// Convert m/s to knots.
pub fn ms_to_kt(ms: f64) -> f64 {
    ms * 1.94384
}

/// Convert meters to miles.
pub fn m_to_mi(m: f64) -> f64 {
    m / 1609.344
}

/// Convert kg/m2/s to in/hr.
pub fn kgm2s_to_inhr(v: f64) -> f64 {
    // 1 kg/m2/s = 1 mm/s = 3600 mm/hr = 141.73 in/hr
    v * 3600.0 / 25.4
}

/// Convert geopotential meters to decameters.
pub fn gpm_to_dam(gpm: f64) -> f64 {
    gpm / 10.0
}

/// Apply unit conversion for a given field.
pub fn convert_values(field: &FieldDef, values: &mut [f64]) {
    match field.name {
        "temp2m" | "dewp2m" => {
            for v in values.iter_mut() {
                if !v.is_nan() {
                    *v = k_to_f(*v);
                }
            }
        }
        "wind10m" => {
            for v in values.iter_mut() {
                if !v.is_nan() {
                    *v = ms_to_kt(*v);
                }
            }
        }
        "vis" => {
            for v in values.iter_mut() {
                if !v.is_nan() {
                    *v = m_to_mi(*v);
                }
            }
        }
        "precip" => {
            for v in values.iter_mut() {
                if !v.is_nan() {
                    *v = kgm2s_to_inhr(*v);
                }
            }
        }
        "h500" => {
            for v in values.iter_mut() {
                if !v.is_nan() {
                    *v = gpm_to_dam(*v);
                }
            }
        }
        _ => {}
    }
}
