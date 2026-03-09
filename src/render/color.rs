/// Color tables for HRRR weather fields.
///
/// Each color table maps a normalized value (0.0-1.0) to an RGBA color.
/// The value is normalized based on the field's value_range.

/// RGBA color as (r, g, b, a) bytes.
pub type Color = [u8; 4];

/// A color stop: (position 0.0-1.0, color).
pub type ColorStop = (f64, Color);

/// Interpolate between color stops for a normalized value t in [0.0, 1.0].
pub fn interpolate(stops: &[ColorStop], t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);

    if stops.is_empty() {
        return [128, 128, 128, 255];
    }
    if stops.len() == 1 {
        return stops[0].1;
    }

    // Find the two surrounding stops
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }

    for i in 0..stops.len() - 1 {
        let (t0, c0) = &stops[i];
        let (t1, c1) = &stops[i + 1];
        if t >= *t0 && t <= *t1 {
            let frac = (t - t0) / (t1 - t0);
            return [
                lerp_u8(c0[0], c1[0], frac),
                lerp_u8(c0[1], c1[1], frac),
                lerp_u8(c0[2], c1[2], frac),
                255,
            ];
        }
    }

    stops[stops.len() - 1].1
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}

/// Normalize a raw value to [0, 1] based on a range.
pub fn normalize(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return 0.5;
    }
    (value - min) / (max - min)
}

/// Temperature color table: blue-cyan-green-yellow-red (cold to hot)
pub fn temperature_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [  30,   0, 120, 255]),  // deep purple (very cold)
        (0.10, [   0,  50, 200, 255]),  // blue
        (0.20, [   0, 130, 255, 255]),  // light blue
        (0.30, [   0, 200, 200, 255]),  // cyan
        (0.40, [   0, 200, 100, 255]),  // cyan-green
        (0.50, [  50, 200,  50, 255]),  // green
        (0.60, [ 150, 220,  50, 255]),  // yellow-green
        (0.70, [ 255, 255,   0, 255]),  // yellow
        (0.80, [ 255, 180,   0, 255]),  // orange
        (0.90, [ 255,  50,   0, 255]),  // red
        (1.00, [ 180,   0,  50, 255]),  // dark red (very hot)
    ];
    interpolate(stops, t)
}

/// Dewpoint color table: brown-green (dry to moist)
pub fn dewpoint_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 140,  80,  20, 255]),  // brown (very dry)
        (0.15, [ 180, 120,  40, 255]),  // tan
        (0.30, [ 200, 180,  80, 255]),  // pale yellow
        (0.45, [ 180, 220, 100, 255]),  // yellow-green
        (0.55, [ 100, 200, 100, 255]),  // green
        (0.70, [  50, 180, 100, 255]),  // medium green
        (0.80, [   0, 150, 100, 255]),  // dark green
        (0.90, [   0, 120, 150, 255]),  // teal
        (1.00, [   0,  80, 120, 255]),  // dark teal (tropical)
    ];
    interpolate(stops, t)
}

/// NWS-style reflectivity color table
pub fn reflectivity_color(t: f64) -> Color {
    // Map t (0-1) back to approximate dBZ for NWS color mapping
    // Range is typically -10 to 75 dBZ
    let dbz = -10.0 + t * 85.0;

    if dbz < 5.0 {
        [0, 0, 0, 0] // transparent / no echo
    } else if dbz < 10.0 {
        [100, 100, 100, 255] // gray
    } else if dbz < 15.0 {
        [75, 150, 75, 255]
    } else if dbz < 20.0 {
        [0, 200, 0, 255] // green
    } else if dbz < 25.0 {
        [0, 255, 0, 255] // bright green
    } else if dbz < 30.0 {
        [0, 255, 100, 255]
    } else if dbz < 35.0 {
        [255, 255, 0, 255] // yellow
    } else if dbz < 40.0 {
        [255, 200, 0, 255] // dark yellow
    } else if dbz < 45.0 {
        [255, 130, 0, 255] // orange
    } else if dbz < 50.0 {
        [255, 0, 0, 255] // red
    } else if dbz < 55.0 {
        [200, 0, 0, 255] // dark red
    } else if dbz < 60.0 {
        [255, 0, 255, 255] // magenta
    } else if dbz < 65.0 {
        [150, 0, 200, 255] // purple
    } else if dbz < 70.0 {
        [100, 100, 255, 255] // blue-purple
    } else {
        [255, 255, 255, 255] // white (extreme)
    }
}

/// CAPE color table: white-yellow-orange-red-magenta
pub fn cape_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 255, 255, 255, 255]),  // white (no CAPE)
        (0.05, [ 200, 200, 200, 255]),  // light gray
        (0.10, [ 255, 255, 150, 255]),  // pale yellow
        (0.20, [ 255, 255,   0, 255]),  // yellow
        (0.30, [ 255, 200,   0, 255]),  // gold
        (0.40, [ 255, 150,   0, 255]),  // orange
        (0.55, [ 255,  50,   0, 255]),  // red-orange
        (0.70, [ 220,   0,   0, 255]),  // red
        (0.85, [ 200,   0, 150, 255]),  // magenta
        (1.00, [ 150,   0, 200, 255]),  // purple (extreme)
    ];
    interpolate(stops, t)
}

/// Wind speed color table: calm blue through purple
pub fn wind_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 200, 230, 255, 255]),  // pale blue (calm)
        (0.10, [ 100, 180, 255, 255]),  // light blue
        (0.20, [   0, 130, 200, 255]),  // blue
        (0.30, [   0, 200, 100, 255]),  // green
        (0.40, [ 100, 230,  50, 255]),  // lime
        (0.50, [ 255, 255,   0, 255]),  // yellow
        (0.60, [ 255, 200,   0, 255]),  // gold
        (0.70, [ 255, 130,   0, 255]),  // orange
        (0.80, [ 255,   0,   0, 255]),  // red
        (0.90, [ 200,   0, 100, 255]),  // magenta
        (1.00, [ 150,   0, 200, 255]),  // purple (hurricane)
    ];
    interpolate(stops, t)
}

/// Visibility color table: red (low vis) to green (good vis)
pub fn visibility_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 150,   0,   0, 255]),  // dark red (zero vis)
        (0.10, [ 255,   0,   0, 255]),  // red
        (0.20, [ 255, 100,   0, 255]),  // orange
        (0.40, [ 255, 200,   0, 255]),  // yellow
        (0.60, [ 200, 255, 100, 255]),  // yellow-green
        (0.80, [ 100, 230, 100, 255]),  // green
        (1.00, [ 200, 255, 200, 255]),  // pale green (unlimited)
    ];
    interpolate(stops, t)
}

/// Precipitation rate color table
pub fn precip_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent (no precip)
        (0.02, [ 100, 200, 100, 255]),  // light green
        (0.10, [   0, 255,   0, 255]),  // green
        (0.20, [   0, 200,   0, 255]),  // dark green
        (0.30, [ 255, 255,   0, 255]),  // yellow
        (0.40, [ 255, 200,   0, 255]),  // gold
        (0.50, [ 255, 130,   0, 255]),  // orange
        (0.60, [ 255,   0,   0, 255]),  // red
        (0.80, [ 200,   0, 100, 255]),  // magenta
        (1.00, [ 150,   0, 200, 255]),  // purple
    ];
    interpolate(stops, t)
}

/// 500mb height color table: for contour-style rendering
pub fn height_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0, 200, 255]),  // blue (trough)
        (0.20, [   0, 150, 255, 255]),  // light blue
        (0.40, [ 100, 255, 100, 255]),  // green
        (0.50, [ 255, 255,   0, 255]),  // yellow
        (0.60, [ 255, 200,   0, 255]),  // gold
        (0.80, [ 255, 100,   0, 255]),  // orange
        (1.00, [ 255,   0,   0, 255]),  // red (ridge)
    ];
    interpolate(stops, t)
}

/// CIN color table: green (weak inhibition) to blue-purple (strong cap)
pub fn cin_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 150,   0, 200, 255]),  // purple (strong cap, most negative)
        (0.15, [  80,   0, 180, 255]),
        (0.30, [   0,  50, 200, 255]),  // blue
        (0.50, [   0, 130, 200, 255]),  // light blue
        (0.70, [   0, 180, 150, 255]),  // teal
        (0.85, [ 100, 220, 100, 255]),  // green (weak cap)
        (1.00, [ 200, 255, 200, 255]),  // pale green (no cap)
    ];
    interpolate(stops, t)
}

/// SRH/Helicity color table: blue (negative) through warm (positive)
pub fn helicity_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0, 150, 255]),  // blue (anticyclonic)
        (0.15, [   0, 100, 200, 255]),
        (0.30, [ 200, 200, 200, 255]),  // gray (near zero)
        (0.40, [ 255, 255, 150, 255]),  // pale yellow
        (0.50, [ 255, 255,   0, 255]),  // yellow
        (0.60, [ 255, 200,   0, 255]),  // gold
        (0.70, [ 255, 130,   0, 255]),  // orange
        (0.80, [ 255,  50,   0, 255]),  // red
        (0.90, [ 200,   0, 100, 255]),  // magenta
        (1.00, [ 150,   0, 200, 255]),  // purple (extreme)
    ];
    interpolate(stops, t)
}

/// Updraft helicity color table: transparent for low, hot for high
pub fn updraft_helicity_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent
        (0.05, [ 100, 100, 100, 255]),  // gray
        (0.15, [   0, 200, 200, 255]),  // cyan
        (0.25, [   0, 255,   0, 255]),  // green
        (0.40, [ 255, 255,   0, 255]),  // yellow
        (0.55, [ 255, 130,   0, 255]),  // orange
        (0.70, [ 255,   0,   0, 255]),  // red
        (0.85, [ 200,   0, 200, 255]),  // magenta
        (1.00, [ 255, 255, 255, 255]),  // white (extreme)
    ];
    interpolate(stops, t)
}

/// Moisture / PWAT color table: brown-green-blue
pub fn moisture_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 140,  80,  20, 255]),  // brown (very dry)
        (0.20, [ 200, 180, 100, 255]),  // tan
        (0.35, [ 100, 200, 100, 255]),  // green
        (0.50, [   0, 180, 130, 255]),  // teal
        (0.65, [   0, 130, 200, 255]),  // blue
        (0.80, [   0,  80, 200, 255]),  // dark blue
        (0.90, [ 100,   0, 200, 255]),  // purple
        (1.00, [ 200, 100, 255, 255]),  // violet (tropical)
    ];
    interpolate(stops, t)
}

/// Hail size color table
pub fn hail_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent (no hail)
        (0.05, [ 100, 200, 100, 255]),  // green (small)
        (0.20, [ 255, 255,   0, 255]),  // yellow (1")
        (0.40, [ 255, 150,   0, 255]),  // orange (significant)
        (0.60, [ 255,   0,   0, 255]),  // red (severe)
        (0.80, [ 200,   0, 200, 255]),  // magenta (giant)
        (1.00, [ 255, 255, 255, 255]),  // white (extreme)
    ];
    interpolate(stops, t)
}

/// Lifted index color table: reversed (negative = unstable = warm colors)
pub fn lifted_index_color(t: f64) -> Color {
    // t=0 is most negative (very unstable), t=1 is most positive (stable)
    let stops: &[ColorStop] = &[
        (0.00, [ 150,   0, 200, 255]),  // purple (extremely unstable)
        (0.10, [ 255,   0,   0, 255]),  // red
        (0.20, [ 255, 130,   0, 255]),  // orange
        (0.35, [ 255, 255,   0, 255]),  // yellow
        (0.50, [ 200, 200, 200, 255]),  // gray (neutral)
        (0.65, [ 100, 200, 255, 255]),  // light blue
        (0.80, [   0, 100, 200, 255]),  // blue
        (1.00, [   0,   0, 150, 255]),  // dark blue (stable)
    ];
    interpolate(stops, t)
}

/// Vorticity color table: blue (negative/anticyclonic) - red (positive/cyclonic)
pub fn vorticity_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0, 200, 255]),  // blue (strong anticyclonic)
        (0.30, [   0, 150, 255, 255]),  // light blue
        (0.45, [ 200, 200, 200, 255]),  // gray
        (0.55, [ 200, 200, 200, 255]),  // gray
        (0.70, [ 255, 150,   0, 255]),  // orange
        (0.85, [ 255,   0,   0, 255]),  // red
        (1.00, [ 200,   0, 100, 255]),  // magenta (strong cyclonic)
    ];
    interpolate(stops, t)
}

/// Cloud cover / percentage color table
pub fn percentage_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [  20,  25,  35, 255]),  // dark (clear)
        (0.20, [  50,  60,  80, 255]),
        (0.40, [ 100, 110, 130, 255]),
        (0.60, [ 150, 160, 170, 255]),
        (0.80, [ 200, 210, 215, 255]),
        (1.00, [ 240, 245, 250, 255]),  // white (overcast)
    ];
    interpolate(stops, t)
}

/// MSLP / pressure color table
pub fn pressure_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [ 200,   0, 200, 255]),  // magenta (deep low)
        (0.15, [ 100,   0, 200, 255]),  // purple
        (0.30, [   0, 100, 255, 255]),  // blue
        (0.45, [   0, 200, 200, 255]),  // cyan
        (0.55, [ 100, 230, 100, 255]),  // green
        (0.70, [ 255, 255,   0, 255]),  // yellow
        (0.85, [ 255, 150,   0, 255]),  // orange
        (1.00, [ 255,   0,   0, 255]),  // red (strong high)
    ];
    interpolate(stops, t)
}

/// Angle color table (0-180 for critical angle)
pub fn angle_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0, 150, 255]),  // blue (small angle)
        (0.25, [   0, 150, 200, 255]),  // cyan
        (0.40, [ 100, 230, 100, 255]),  // green
        (0.50, [ 255, 255,   0, 255]),  // yellow (90 degrees - perpendicular)
        (0.60, [ 255, 200,   0, 255]),  // gold
        (0.75, [ 255, 100,   0, 255]),  // orange
        (1.00, [ 255,   0,   0, 255]),  // red (large angle)
    ];
    interpolate(stops, t)
}

/// STP color table: transparent at zero, ramps through yellow-orange-red-magenta
pub fn stp_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent (zero)
        (0.02, [ 100, 100, 100, 255]),  // gray (trace)
        (0.08, [ 180, 255, 180, 255]),  // pale green (0-1)
        (0.17, [ 255, 255,   0, 255]),  // yellow (~2)
        (0.33, [ 255, 180,   0, 255]),  // orange (~4)
        (0.50, [ 255,  80,   0, 255]),  // dark orange (~6)
        (0.67, [ 255,   0,   0, 255]),  // red (~8)
        (0.83, [ 200,   0, 150, 255]),  // magenta (~10)
        (1.00, [ 150,   0, 255, 255]),  // purple (~12)
    ];
    interpolate(stops, t)
}

/// SCP color table: similar ramp, broader range (0-20)
pub fn scp_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent
        (0.02, [ 100, 100, 100, 255]),  // gray
        (0.05, [ 150, 220, 150, 255]),  // pale green (~1)
        (0.15, [ 255, 255,   0, 255]),  // yellow (~3)
        (0.25, [ 255, 200,   0, 255]),  // gold (~5)
        (0.40, [ 255, 130,   0, 255]),  // orange (~8)
        (0.60, [ 255,   0,   0, 255]),  // red (~12)
        (0.80, [ 200,   0, 150, 255]),  // magenta (~16)
        (1.00, [ 150,   0, 255, 255]),  // purple (~20)
    ];
    interpolate(stops, t)
}

/// SHIP color table: 0-5 range
pub fn ship_color(t: f64) -> Color {
    let stops: &[ColorStop] = &[
        (0.00, [   0,   0,   0,   0]),  // transparent
        (0.04, [ 100, 100, 100, 255]),  // gray
        (0.10, [ 150, 220, 150, 255]),  // pale green
        (0.20, [ 255, 255,   0, 255]),  // yellow (~1)
        (0.40, [ 255, 180,   0, 255]),  // orange (~2)
        (0.60, [ 255,  50,   0, 255]),  // red-orange (~3)
        (0.80, [ 255,   0,   0, 255]),  // red (~4)
        (1.00, [ 200,   0, 200, 255]),  // magenta (~5)
    ];
    interpolate(stops, t)
}

/// Get the appropriate color function for a field name.
pub fn color_for_field(field_name: &str) -> fn(f64) -> Color {
    match field_name {
        "temp2m" | "sfct" => temperature_color,
        "dewp2m" | "td850" => dewpoint_color,
        "ref" | "ref1km" | "ref4km" | "maxref" => reflectivity_color,
        "sbcape" | "mlcape" | "mucape" | "cape03" => cape_color,
        "sbcin" | "mlcin" | "mucin" => cin_color,
        "wind10m" | "gust" | "wind250" | "wind80m" => wind_color,
        "vis" => visibility_color,
        "precip" | "apcp" => precip_color,
        "h500" | "h700" | "h850" => height_color,
        "t500" | "t700" | "t850" => temperature_color,
        "srh3km" | "srh1km" | "efhl" => helicity_color,
        "uh25" | "uh02" | "uh03" => updraft_helicity_color,
        "mnuh02" => cin_color,  // negative values, similar scale
        "maxuvv" | "maxdvv" => wind_color,
        "lftx" | "4lftx" => lifted_index_color,
        "hail" | "hailsfc" | "graupel" => hail_color,
        "retop" | "vil" => cape_color,
        "pwat" => moisture_color,
        "rh2m" | "tcc" | "cpofp" => percentage_color,
        "mslp" => pressure_color,
        "pblh" | "lcl" | "lfc" | "el" | "ceil" | "fzlev" | "esp" => cape_color,
        "ltng" => updraft_helicity_color,
        "relv2km" | "relv1km" => vorticity_color,
        "cangle" => angle_color,
        "ustm" | "vstm" => vorticity_color,
        "stp" => stp_color,
        "scp" => scp_color,
        "ship" => ship_color,
        "shr01" | "shr06" | "ebs" => wind_color,
        _ => temperature_color,
    }
}

/// Background color for pixels with no data (ocean, outside domain).
pub fn background_color() -> Color {
    [20, 20, 30, 255]
}

/// Border color for state/country lines.
pub fn border_color() -> Color {
    [80, 80, 80, 255]
}

/// Text/legend color.
pub fn text_color() -> Color {
    [220, 220, 220, 255]
}
