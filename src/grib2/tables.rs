/// GRIB2 code tables for discipline, category, and parameter lookups.

/// Discipline 0 = Meteorological products
/// Discipline 10 = Oceanographic products

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterInfo {
    pub discipline: u8,
    pub category: u8,
    pub number: u8,
    pub name: &'static str,
    pub unit: &'static str,
    pub abbrev: &'static str,
}

/// Look up a parameter by discipline/category/number.
pub fn lookup_parameter(discipline: u8, category: u8, number: u8) -> Option<ParameterInfo> {
    // Meteorological products (discipline 0)
    if discipline == 0 {
        match (category, number) {
            // Temperature (category 0)
            (0, 0) => Some(ParameterInfo {
                discipline, category, number,
                name: "Temperature", unit: "K", abbrev: "TMP",
            }),
            (0, 6) => Some(ParameterInfo {
                discipline, category, number,
                name: "Dewpoint Temperature", unit: "K", abbrev: "DPT",
            }),
            // Moisture (category 1)
            (1, 7) => Some(ParameterInfo {
                discipline, category, number,
                name: "Precipitation Rate", unit: "kg m-2 s-1", abbrev: "PRATE",
            }),
            // Momentum (category 2)
            (2, 1) => Some(ParameterInfo {
                discipline, category, number,
                name: "Wind Speed", unit: "m s-1", abbrev: "WIND",
            }),
            (2, 2) => Some(ParameterInfo {
                discipline, category, number,
                name: "U-Component of Wind", unit: "m s-1", abbrev: "UGRD",
            }),
            (2, 3) => Some(ParameterInfo {
                discipline, category, number,
                name: "V-Component of Wind", unit: "m s-1", abbrev: "VGRD",
            }),
            // Mass (category 3)
            (3, 5) => Some(ParameterInfo {
                discipline, category, number,
                name: "Geopotential Height", unit: "gpm", abbrev: "HGT",
            }),
            // Stability (category 7)
            (7, 6) => Some(ParameterInfo {
                discipline, category, number,
                name: "Convective Available Potential Energy", unit: "J kg-1", abbrev: "CAPE",
            }),
            // Misc (category 19)
            (19, 0) => Some(ParameterInfo {
                discipline, category, number,
                name: "Visibility", unit: "m", abbrev: "VIS",
            }),
            // Composite reflectivity
            (16, 196) => Some(ParameterInfo {
                discipline, category, number,
                name: "Composite Reflectivity", unit: "dBZ", abbrev: "REFC",
            }),
            _ => None,
        }
    } else {
        None
    }
}

/// Surface type lookup from GRIB2 Table 4.5
pub fn surface_type_name(surface_type: u8) -> &'static str {
    match surface_type {
        1 => "Ground or Water Surface",
        2 => "Cloud Base Level",
        3 => "Level of Cloud Tops",
        4 => "Level of 0C Isotherm",
        100 => "Isobaric Surface (Pa)",
        103 => "Specified Height Above Ground (m)",
        104 => "Sigma Level",
        200 => "Entire Atmosphere",
        _ => "Unknown",
    }
}
