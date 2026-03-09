/// Map renderer for HRRR weather data.
///
/// Renders gridded weather data to PNG images with:
/// - Lambert Conformal Conic projection
/// - Field-specific color tables
/// - State/country borders
/// - Color legend bar
/// - Parallel row rendering via rayon

pub mod color;
pub mod contour;
pub mod projection;

use color::{Color, background_color, border_color, text_color};
use projection::LambertProjection;
use rayon::prelude::*;
use std::io::{self, BufWriter};

use crate::fields::FieldDef;

/// Embedded simplified CONUS state borders as polylines.
/// Each polyline is a series of (lat, lon) pairs, with None separating segments.
/// This is a simplified version with key state boundary points.
fn state_borders() -> Vec<Vec<(f64, f64)>> {
    // Major US borders + coastline approximation
    // US-Canada border (lower 48 relevant portion)
    let us_canada = vec![
        (49.0, -123.0), (49.0, -120.0), (49.0, -117.0), (49.0, -114.0),
        (49.0, -111.0), (49.0, -108.0), (49.0, -105.0), (49.0, -102.0),
        (49.0, -99.0), (49.0, -97.0), (49.0, -95.3),
        (48.0, -89.5), (47.5, -85.0), (46.5, -84.5),
        (45.0, -83.5), (43.5, -82.5), (42.5, -82.0), (42.0, -81.0),
        (42.5, -79.5), (43.2, -79.0), (43.5, -76.5),
        (44.5, -75.5), (45.0, -74.5), (45.0, -71.5), (47.0, -67.8),
    ];

    // US-Mexico border
    let us_mexico = vec![
        (32.5, -117.1), (32.7, -114.7), (31.3, -111.0), (31.3, -108.2),
        (31.8, -106.5), (29.8, -104.4), (29.5, -103.0), (29.0, -102.0),
        (28.0, -100.5), (27.5, -99.5), (26.5, -99.0), (26.0, -97.5),
        (25.9, -97.2),
    ];

    // West coast
    let west_coast = vec![
        (48.4, -124.7), (47.5, -124.5), (46.2, -124.0), (44.6, -124.1),
        (43.0, -124.4), (42.0, -124.3), (41.0, -124.1), (40.0, -124.3),
        (38.5, -123.2), (37.5, -122.5), (36.5, -122.0), (35.5, -121.0),
        (34.5, -120.5), (34.0, -119.5), (33.5, -118.0), (32.7, -117.2),
    ];

    // East coast (simplified)
    let east_coast = vec![
        (25.0, -80.5), (26.0, -80.0), (27.5, -80.3), (28.5, -80.6),
        (30.0, -81.5), (31.0, -81.2), (32.0, -80.8), (33.0, -79.5),
        (34.5, -77.5), (35.5, -75.5), (36.5, -76.0), (37.0, -76.0),
        (37.5, -76.2), (38.5, -75.1), (39.5, -74.2), (40.5, -74.0),
        (41.0, -72.0), (41.5, -71.5), (42.0, -70.5), (43.0, -70.5),
        (43.5, -70.0), (44.5, -67.5), (45.0, -67.0), (47.0, -67.8),
    ];

    // Gulf coast
    let gulf_coast = vec![
        (25.0, -80.5), (25.5, -81.5), (26.5, -82.0), (28.0, -82.8),
        (29.0, -83.5), (29.5, -84.5), (30.0, -85.5), (30.3, -87.5),
        (30.2, -88.5), (30.0, -89.5), (29.5, -90.0), (29.2, -91.0),
        (29.5, -93.0), (29.5, -94.5), (28.5, -96.0), (27.5, -97.0),
        (26.0, -97.2), (25.9, -97.2),
    ];

    // Major state lines (simplified)
    // Mississippi River approximate path
    let mississippi = vec![
        (47.5, -95.2), (46.5, -94.0), (45.0, -93.3), (44.0, -91.5),
        (43.0, -91.2), (42.0, -90.5), (41.0, -91.0), (40.0, -91.5),
        (39.0, -90.8), (38.5, -90.2), (37.5, -89.5), (36.5, -89.5),
        (35.5, -90.0), (34.5, -90.5), (33.5, -91.0), (32.5, -91.0),
        (31.5, -91.3), (30.5, -91.0), (29.5, -90.0),
    ];

    // Some key straight-line borders
    // CA-NV border (approximate)
    let ca_nv = vec![(42.0, -120.0), (39.0, -120.0), (35.8, -115.6)];
    // OR-CA border
    let or_ca = vec![(42.0, -124.3), (42.0, -120.0)];
    // WA-OR border (Columbia River approximate)
    let wa_or = vec![(46.2, -124.0), (46.2, -120.0), (46.0, -118.0), (46.0, -117.0)];
    // MT-ND border
    let mt_nd = vec![(49.0, -104.0), (46.0, -104.0)];
    // WY-CO border
    let wy_co = vec![(41.0, -109.0), (41.0, -102.0)];
    // CO-NM border
    let co_nm = vec![(37.0, -109.0), (37.0, -103.0)];
    // KS-OK border
    let ks_ok = vec![(37.0, -102.0), (37.0, -94.6)];
    // OK-TX border (approximate)
    let ok_tx = vec![(36.5, -103.0), (36.5, -100.0), (34.5, -100.0), (34.0, -99.5)];
    // PA-NY border approximate
    let pa_ny = vec![(42.0, -79.8), (42.0, -75.0), (41.5, -75.0)];

    vec![
        us_canada, us_mexico, west_coast, east_coast, gulf_coast,
        mississippi, ca_nv, or_ca, wa_or, mt_nd, wy_co, co_nm, ks_ok, ok_tx, pa_ny,
    ]
}

/// Render weather data to a PNG image buffer.
pub fn render_to_png(
    values: &[f64],
    field: &FieldDef,
    proj: &LambertProjection,
    width: u32,
    height: u32,
) -> io::Result<Vec<u8>> {
    let nx = proj.nx as usize;
    let ny = proj.ny as usize;

    let color_fn = color::color_for_field(field.name);
    let (vmin, vmax) = field.value_range;
    let bg = background_color();

    // Compute scale factors to map output pixels to grid coordinates
    let scale_x = nx as f64 / width as f64;
    let scale_y = ny as f64 / height as f64;

    // Generate contour mask for height fields
    let contour_mask = if field.name == "h500" {
        Some(contour::height_contour_mask(values, nx, ny))
    } else {
        None
    };

    // Render rows in parallel
    let legend_width = 60u32;
    let img_width = width + legend_width;

    let rows: Vec<Vec<Color>> = (0..height)
        .into_par_iter()
        .map(|row| {
            let mut row_pixels = Vec::with_capacity(img_width as usize);

            for col in 0..width {
                // Map output pixel to grid coordinate
                let gi = col as f64 * scale_x;
                let gj = row as f64 * scale_y;

                let i = gi.round() as isize;
                let j = gj.round() as isize;

                if i < 0 || i >= nx as isize || j < 0 || j >= ny as isize {
                    row_pixels.push(bg);
                    continue;
                }

                let idx = j as usize * nx + i as usize;
                if idx >= values.len() {
                    row_pixels.push(bg);
                    continue;
                }
                let val = values[idx];

                if val.is_nan() {
                    row_pixels.push(bg);
                } else {
                    // Check contour
                    if let Some(ref cm) = contour_mask {
                        if cm[idx] {
                            // Darken for contour
                            let base = color_fn(color::normalize(val, vmin, vmax));
                            row_pixels.push([
                                (base[0] as f64 * 0.5) as u8,
                                (base[1] as f64 * 0.5) as u8,
                                (base[2] as f64 * 0.5) as u8,
                                255,
                            ]);
                            continue;
                        }
                    }

                    let t = color::normalize(val, vmin, vmax);
                    let c = color_fn(t);

                    // For reflectivity, skip transparent values (no echo)
                    if field.name == "ref" && c[3] == 0 {
                        row_pixels.push(bg);
                    } else {
                        row_pixels.push(c);
                    }
                }
            }

            // Legend bar pixels for this row
            let t = 1.0 - (row as f64 / height as f64); // top=max, bottom=min
            let legend_color = color_fn(t);
            for _ in 0..legend_width {
                row_pixels.push(legend_color);
            }

            row_pixels
        })
        .collect();

    // Draw borders on top
    let mut pixel_buf: Vec<Color> = rows.into_iter().flatten().collect();
    draw_borders(&mut pixel_buf, img_width, height, proj, width);

    // Draw legend labels
    draw_legend_labels(&mut pixel_buf, img_width, height, width, legend_width, field);

    // Draw title
    draw_title(&mut pixel_buf, img_width, field);

    // Encode to PNG
    encode_png(&pixel_buf, img_width, height)
}

/// Draw state/country borders onto the pixel buffer.
fn draw_borders(
    pixels: &mut [Color],
    img_width: u32,
    height: u32,
    proj: &LambertProjection,
    data_width: u32,
) {
    let bc = border_color();
    let borders = state_borders();
    let _scale_x = data_width as f64 / proj.nx as f64;
    let _scale_y = height as f64 / proj.ny as f64;

    for polyline in &borders {
        for window in polyline.windows(2) {
            let (lat0, lon0) = window[0];
            let (lat1, lon1) = window[1];

            let (gi0, gj0) = proj.latlon_to_grid(lat0, lon0);
            let (gi1, gj1) = proj.latlon_to_grid(lat1, lon1);

            let x0 = (gi0 / (proj.nx as f64 / data_width as f64)) as i32;
            let y0 = (gj0 / (proj.ny as f64 / height as f64)) as i32;
            let x1 = (gi1 / (proj.nx as f64 / data_width as f64)) as i32;
            let y1 = (gj1 / (proj.ny as f64 / height as f64)) as i32;

            draw_line(pixels, img_width as i32, height as i32, x0, y0, x1, y1, bc);
        }
    }
}

/// Bresenham line drawing.
fn draw_line(
    pixels: &mut [Color],
    width: i32,
    height: i32,
    x0: i32, y0: i32, x1: i32, y1: i32,
    color: Color,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            let idx = (y * width + x) as usize;
            if idx < pixels.len() {
                pixels[idx] = color;
            }
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draw legend text labels (simplified - draws tick marks).
fn draw_legend_labels(
    pixels: &mut [Color],
    img_width: u32,
    height: u32,
    data_width: u32,
    _legend_width: u32,
    field: &FieldDef,
) {
    let tc = text_color();
    let (vmin, vmax) = field.value_range;
    let range = vmax - vmin;

    // Draw tick marks at regular intervals
    let num_ticks = 10;
    for tick in 0..=num_ticks {
        let t = tick as f64 / num_ticks as f64;
        let y = ((1.0 - t) * (height as f64 - 1.0)) as i32;

        // Draw tick mark
        for x in data_width as i32..(data_width as i32 + 5) {
            if y >= 0 && y < height as i32 {
                let idx = (y * img_width as i32 + x) as usize;
                if idx < pixels.len() {
                    pixels[idx] = tc;
                }
            }
        }

        // Draw a small number indicator using simple pixel patterns
        let value = vmin + t * range;
        draw_number(pixels, img_width, height, data_width + 6, y as u32, value, tc);
    }
}

/// Draw a simple number at a position (very basic pixel font).
fn draw_number(
    pixels: &mut [Color],
    img_width: u32,
    height: u32,
    x: u32,
    y: u32,
    _value: f64,
    color: Color,
) {
    // Simple marker dot instead of full font rendering
    // A proper implementation would use a bitmap font
    let cx = x as i32 + 2;
    let cy = y as i32;
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let px = cx + dx;
            let py = cy + dy;
            if px >= 0 && px < img_width as i32 && py >= 0 && py < height as i32 {
                let idx = (py * img_width as i32 + px) as usize;
                if idx < pixels.len() {
                    pixels[idx] = color;
                }
            }
        }
    }
}

/// Draw a title bar at the top of the image.
fn draw_title(
    pixels: &mut [Color],
    img_width: u32,
    _field: &FieldDef,
) {
    // Draw a dark band at the top
    let band_height = 20u32;
    let band_color: Color = [10, 10, 20, 255];
    for y in 0..band_height.min(pixels.len() as u32 / img_width) {
        for x in 0..img_width {
            let idx = (y * img_width + x) as usize;
            if idx < pixels.len() {
                // Blend with existing
                let existing = pixels[idx];
                pixels[idx] = [
                    ((existing[0] as u16 + band_color[0] as u16) / 2) as u8,
                    ((existing[1] as u16 + band_color[1] as u16) / 2) as u8,
                    ((existing[2] as u16 + band_color[2] as u16) / 2) as u8,
                    255,
                ];
            }
        }
    }
}

/// Encode RGBA pixel data to PNG bytes.
fn encode_png(pixels: &[Color], width: u32, height: u32) -> io::Result<Vec<u8>> {
    let mut png_data = Vec::new();

    {
        let mut encoder = png::Encoder::new(BufWriter::new(&mut png_data), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);

        let mut writer = encoder.write_header().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("PNG header error: {}", e))
        })?;

        // Flatten RGBA pixels
        let flat: Vec<u8> = pixels.iter().flat_map(|c| c.iter().copied()).collect();

        writer.write_image_data(&flat).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("PNG write error: {}", e))
        })?;
    }

    Ok(png_data)
}
