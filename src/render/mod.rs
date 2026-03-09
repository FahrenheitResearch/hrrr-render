/// Map renderer for HRRR weather data.
///
/// Renders gridded weather data to PNG images with:
/// - Lambert Conformal Conic projection
/// - Field-specific color tables
/// - Natural Earth geodata base map (dark theme)
/// - Color legend bar

pub mod color;
pub mod contour;
pub mod mapbase;
pub mod projection;

use color::{Color, text_color};
use projection::LambertProjection;
use std::io::{self, BufWriter};

use crate::fields::FieldDef;

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

    let scale_x = nx as f64 / width as f64;
    let scale_y = ny as f64 / height as f64;

    let contour_mask = if field.name == "h500" {
        Some(contour::height_contour_mask(values, nx, ny))
    } else {
        None
    };

    let legend_width = 60u32;
    let img_width = width + legend_width;

    // 1. Draw base map (ocean, land, lakes) using Natural Earth geodata
    let mut pixel_buf = vec![[20u8, 20, 30, 255]; (img_width * height) as usize];
    mapbase::draw_base_map(&mut pixel_buf, img_width, height, proj, width);

    // 2. Overlay weather data on top of base map
    let is_transparent_field = matches!(field.name, "ref" | "precip");

    for row in 0..height {
        for col in 0..width {
            let gi = col as f64 * scale_x;
            let gj = (height - 1 - row) as f64 * scale_y;

            let i = gi.round() as isize;
            let j = gj.round() as isize;

            if i < 0 || i >= nx as isize || j < 0 || j >= ny as isize { continue; }

            let idx = j as usize * nx + i as usize;
            if idx >= values.len() { continue; }
            let val = values[idx];
            if val.is_nan() { continue; }

            let c = if let Some(ref cm) = contour_mask {
                if cm[idx] {
                    let base = color_fn(color::normalize(val, vmin, vmax));
                    [(base[0] as f64 * 0.5) as u8, (base[1] as f64 * 0.5) as u8, (base[2] as f64 * 0.5) as u8, 255]
                } else {
                    color_fn(color::normalize(val, vmin, vmax))
                }
            } else {
                color_fn(color::normalize(val, vmin, vmax))
            };

            // Skip transparent values (no echo for ref/precip)
            if is_transparent_field && c[3] == 0 { continue; }

            let pidx = (row * img_width + col) as usize;
            if pidx < pixel_buf.len() {
                pixel_buf[pidx] = c;
            }
        }

        // Legend bar for this row
        let t = 1.0 - (row as f64 / height as f64);
        let legend_color = color_fn(t);
        for lx in 0..legend_width {
            let pidx = (row * img_width + width + lx) as usize;
            if pidx < pixel_buf.len() {
                pixel_buf[pidx] = legend_color;
            }
        }
    }

    // 3. Draw overlay features (borders, coastlines, rivers, cities) on top
    mapbase::draw_overlay_features(&mut pixel_buf, img_width, height, proj, width);

    // Draw legend labels
    draw_legend_labels(&mut pixel_buf, img_width, height, width, legend_width, field);

    // Draw title
    draw_title(&mut pixel_buf, img_width, field);

    // Encode to PNG
    encode_png(&pixel_buf, img_width, height)
}

/// Draw legend text labels.
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

        let value = vmin + t * range;
        draw_number(pixels, img_width, height, data_width + 6, y as u32, value, tc);
    }
}

/// Draw a simple number at a position.
fn draw_number(
    pixels: &mut [Color],
    img_width: u32,
    height: u32,
    x: u32,
    y: u32,
    _value: f64,
    color: Color,
) {
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
    let band_height = 20u32;
    let band_color: Color = [10, 10, 20, 255];
    for y in 0..band_height.min(pixels.len() as u32 / img_width) {
        for x in 0..img_width {
            let idx = (y * img_width + x) as usize;
            if idx < pixels.len() {
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

/// Render weather data to a raw RGBA pixel buffer (no PNG encoding).
/// Returns (pixels, total_width, height) where total_width includes the legend bar.
pub fn render_to_pixels(
    values: &[f64],
    field: &FieldDef,
    proj: &LambertProjection,
    width: u32,
    height: u32,
) -> (Vec<[u8; 4]>, u32, u32) {
    let nx = proj.nx as usize;
    let ny = proj.ny as usize;

    let color_fn = color::color_for_field(field.name);
    let (vmin, vmax) = field.value_range;

    let scale_x = nx as f64 / width as f64;
    let scale_y = ny as f64 / height as f64;

    let contour_mask = if field.name == "h500" {
        Some(contour::height_contour_mask(values, nx, ny))
    } else {
        None
    };

    let legend_width = 60u32;
    let img_width = width + legend_width;

    // 1. Draw base map
    let mut pixel_buf = vec![[20u8, 20, 30, 255]; (img_width * height) as usize];
    mapbase::draw_base_map(&mut pixel_buf, img_width, height, proj, width);

    // 2. Overlay weather data
    let is_transparent_field = matches!(field.name, "ref" | "precip");

    for row in 0..height {
        for col in 0..width {
            let gi = col as f64 * scale_x;
            let gj = (height - 1 - row) as f64 * scale_y;

            let i = gi.round() as isize;
            let j = gj.round() as isize;

            if i < 0 || i >= nx as isize || j < 0 || j >= ny as isize { continue; }

            let idx = j as usize * nx + i as usize;
            if idx >= values.len() { continue; }
            let val = values[idx];
            if val.is_nan() { continue; }

            let c = if let Some(ref cm) = contour_mask {
                if cm[idx] {
                    let base = color_fn(color::normalize(val, vmin, vmax));
                    [(base[0] as f64 * 0.5) as u8, (base[1] as f64 * 0.5) as u8, (base[2] as f64 * 0.5) as u8, 255]
                } else {
                    color_fn(color::normalize(val, vmin, vmax))
                }
            } else {
                color_fn(color::normalize(val, vmin, vmax))
            };

            if is_transparent_field && c[3] == 0 { continue; }

            let pidx = (row * img_width + col) as usize;
            if pidx < pixel_buf.len() {
                pixel_buf[pidx] = c;
            }
        }

        // Legend bar
        let t = 1.0 - (row as f64 / height as f64);
        let legend_color = color_fn(t);
        for lx in 0..legend_width {
            let pidx = (row * img_width + width + lx) as usize;
            if pidx < pixel_buf.len() {
                pixel_buf[pidx] = legend_color;
            }
        }
    }

    // 3. Overlay features
    mapbase::draw_overlay_features(&mut pixel_buf, img_width, height, proj, width);
    draw_legend_labels(&mut pixel_buf, img_width, height, width, legend_width, field);
    draw_title(&mut pixel_buf, img_width, field);

    (pixel_buf, img_width, height)
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

        let flat: Vec<u8> = pixels.iter().flat_map(|c| c.iter().copied()).collect();

        writer.write_image_data(&flat).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("PNG write error: {}", e))
        })?;
    }

    Ok(png_data)
}
