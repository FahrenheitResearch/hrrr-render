//! Natural Earth map base layer rendering through Lambert Conformal Conic projection.
//! Uses rustmaps geodata for proper coastlines, borders, rivers, lakes, and cities.

use std::sync::OnceLock;
use super::color::Color;
use super::projection::LambertProjection;

static GEO_DATA: OnceLock<rustmaps::geo::GeoData> = OnceLock::new();

fn get_geodata() -> &'static rustmaps::geo::GeoData {
    GEO_DATA.get_or_init(|| {
        let data_dir = std::path::PathBuf::from(r"C:\Users\drew\rustmaps\data");
        rustmaps::geo::GeoData::load(&data_dir).expect("Failed to load Natural Earth geodata")
    })
}

// Dark theme colors matching rustmaps
const OCEAN: Color = [13, 17, 23, 255];
const LAND: Color = [22, 27, 34, 255];
const LAKE: Color = [13, 17, 23, 255];
const COASTLINE: Color = [74, 143, 212, 255];
const COUNTRY: Color = [125, 133, 144, 255];
const STATE: Color = [61, 68, 77, 255];
const RIVER: Color = [26, 58, 92, 255];
const CITY_DOT: Color = [201, 209, 217, 255];
const CITY_LABEL: Color = [139, 148, 158, 255];
const MAJOR_LABEL: Color = [201, 209, 217, 255];
const HALO: Color = [8, 10, 14, 180];

/// Convert (lat, lon) degrees to pixel coordinates using Lambert projection.
#[inline]
fn latlon_to_px(lat: f64, lon: f64, proj: &LambertProjection, data_w: u32, h: u32) -> (f64, f64) {
    let (gi, gj) = proj.latlon_to_grid(lat, lon);
    let sx = data_w as f64 / proj.nx as f64;
    let sy = h as f64 / proj.ny as f64;
    (gi * sx, h as f64 - 1.0 - gj * sy)
}

/// Draw the base map (ocean fill, land polygons, lakes) under weather data.
pub fn draw_base_map(pixels: &mut [Color], img_w: u32, h: u32, proj: &LambertProjection, data_w: u32) {
    let geo = get_geodata();

    // Fill entire image with ocean
    for y in 0..h {
        for x in 0..data_w {
            let idx = (y * img_w + x) as usize;
            if idx < pixels.len() {
                pixels[idx] = OCEAN;
            }
        }
    }

    // Fill land polygons
    for poly in geo.land_for_zoom(5) {
        fill_polygon_lambert(pixels, img_w, h, poly, proj, data_w, LAND);
    }

    // Fill lakes
    for poly in geo.lakes_for_zoom(5) {
        fill_polygon_lambert(pixels, img_w, h, poly, proj, data_w, LAKE);
    }
}

/// Draw overlay features (rivers, borders, coastlines, cities) on top of weather data.
pub fn draw_overlay_features(pixels: &mut [Color], img_w: u32, h: u32, proj: &LambertProjection, data_w: u32) {
    let geo = get_geodata();
    let (min_lat, min_lon, max_lat, max_lon) = proj.bounding_box();
    let margin = 2.0;

    // Rivers
    for line in &geo.rivers {
        if !polyline_in_bounds(line, min_lat - margin, min_lon - margin, max_lat + margin, max_lon + margin) { continue; }
        draw_polyline_lambert(pixels, img_w, h, line, proj, data_w, RIVER, 0.6);
    }

    // State borders
    for line in &geo.state_borders {
        if !polyline_in_bounds(line, min_lat - margin, min_lon - margin, max_lat + margin, max_lon + margin) { continue; }
        draw_polyline_lambert(pixels, img_w, h, line, proj, data_w, STATE, 0.8);
    }

    // Country borders
    for line in &geo.country_borders {
        if !polyline_in_bounds(line, min_lat - margin, min_lon - margin, max_lat + margin, max_lon + margin) { continue; }
        draw_polyline_lambert(pixels, img_w, h, line, proj, data_w, COUNTRY, 1.0);
    }

    // Coastlines
    for line in geo.coastlines_for_zoom(5) {
        if !polyline_in_bounds(line, min_lat - margin, min_lon - margin, max_lat + margin, max_lon + margin) { continue; }
        draw_polyline_lambert(pixels, img_w, h, line, proj, data_w, COASTLINE, 1.2);
    }

    // Cities (tier 0-2 at this scale, roughly equivalent to zoom 5-6)
    for city in &geo.cities {
        if city.tier > 2 { continue; }
        if city.lat < min_lat - 1.0 || city.lat > max_lat + 1.0 ||
           city.lon < min_lon - 1.0 || city.lon > max_lon + 1.0 { continue; }

        let (px, py) = latlon_to_px(city.lat, city.lon, proj, data_w, h);
        let pxi = px.round() as i32;
        let pyi = py.round() as i32;
        if pxi < 0 || pxi >= data_w as i32 || pyi < 0 || pyi >= h as i32 { continue; }

        let radius = match city.tier {
            0 => 3.0_f32,
            1 => 2.5,
            _ => 2.0,
        };
        draw_filled_circle(pixels, img_w, h, pxi, pyi, radius, CITY_DOT);

        let lc = if city.tier <= 1 { MAJOR_LABEL } else { CITY_LABEL };
        draw_text_halo(pixels, img_w, h, pxi + radius as i32 + 3, pyi - 3, &city.name, lc);
    }
}

// --- Polygon fill via scanline with Lambert projection ---

fn fill_polygon_lambert(
    pixels: &mut [Color], img_w: u32, h: u32,
    poly: &[(f64, f64)], proj: &LambertProjection, data_w: u32, color: Color,
) {
    if poly.len() < 3 { return; }

    // Project all polygon points to pixel coords
    let projected: Vec<(f64, f64)> = poly.iter()
        .map(|&(lon, lat)| latlon_to_px(lat, lon, proj, data_w, h))
        .collect();

    // Find bounding box
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(_, y) in &projected {
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }

    let y_start = (min_y.floor() as i32).max(0);
    let y_end = (max_y.ceil() as i32).min(h as i32 - 1);

    // Scanline fill
    for y in y_start..=y_end {
        let yf = y as f64 + 0.5;
        let mut intersections = Vec::new();

        for i in 0..projected.len() {
            let j = (i + 1) % projected.len();
            let (x0, y0) = projected[i];
            let (x1, y1) = projected[j];

            if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                let t = (yf - y0) / (y1 - y0);
                intersections.push(x0 + t * (x1 - x0));
            }
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for pair in intersections.chunks(2) {
            if pair.len() < 2 { break; }
            let x_start = (pair[0].ceil() as i32).max(0);
            let x_end = (pair[1].floor() as i32).min(data_w as i32 - 1);
            for x in x_start..=x_end {
                let idx = (y as u32 * img_w + x as u32) as usize;
                if idx < pixels.len() {
                    pixels[idx] = color;
                }
            }
        }
    }
}

// --- Anti-aliased line drawing with Lambert projection ---

fn draw_polyline_lambert(
    pixels: &mut [Color], img_w: u32, h: u32,
    line: &[(f64, f64)], proj: &LambertProjection, data_w: u32, color: Color, width: f32,
) {
    if line.len() < 2 { return; }

    for seg in line.windows(2) {
        let (lon0, lat0) = seg[0];
        let (lon1, lat1) = seg[1];
        let (x0, y0) = latlon_to_px(lat0, lon0, proj, data_w, h);
        let (x1, y1) = latlon_to_px(lat1, lon1, proj, data_w, h);

        // Skip segments entirely outside viewport with margin
        let margin = 50.0;
        if (x0 < -margin && x1 < -margin) || (x0 > data_w as f64 + margin && x1 > data_w as f64 + margin) { continue; }
        if (y0 < -margin && y1 < -margin) || (y0 > h as f64 + margin && y1 > h as f64 + margin) { continue; }

        draw_line_aa(pixels, img_w, h, data_w, x0 as f32, y0 as f32, x1 as f32, y1 as f32, color, width);
    }
}

fn draw_line_aa(
    pixels: &mut [Color], img_w: u32, h: u32, data_w: u32,
    x0: f32, y0: f32, x1: f32, y1: f32, color: Color, width: f32,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 { return; }

    let steps = (len * 1.5).ceil() as i32;
    let half_w = width / 2.0;

    // Perpendicular direction
    let nx = -dy / len;
    let ny = dx / len;

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = x0 + dx * t;
        let cy = y0 + dy * t;

        // Draw perpendicular samples for width
        let w_steps = (width * 2.0).ceil() as i32;
        for ws in -w_steps..=w_steps {
            let d = ws as f32 * 0.5;
            let px = cx + nx * d;
            let py = cy + ny * d;

            let dist = d.abs();
            if dist > half_w + 0.5 { continue; }

            let alpha = if dist < half_w - 0.5 {
                1.0
            } else {
                1.0 - (dist - (half_w - 0.5))
            };

            if alpha <= 0.0 { continue; }

            let xi = px.round() as i32;
            let yi = py.round() as i32;
            if xi >= 0 && xi < data_w as i32 && yi >= 0 && yi < h as i32 {
                blend_pixel(pixels, img_w, xi, yi, color, alpha);
            }
        }
    }
}

// --- Circle drawing ---

fn draw_filled_circle(pixels: &mut [Color], img_w: u32, h: u32, cx: i32, cy: i32, radius: f32, color: Color) {
    let r = radius.ceil() as i32 + 1;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            if dist > radius + 0.5 { continue; }
            let alpha = if dist < radius - 0.5 { 1.0 } else { 1.0 - (dist - (radius - 0.5)) };
            if alpha <= 0.0 { continue; }
            let px = cx + dx;
            let py = cy + dy;
            if px >= 0 && px < img_w as i32 && py >= 0 && py < h as i32 {
                blend_pixel(pixels, img_w, px, py, color, alpha);
            }
        }
    }
}

// --- Text rendering with halo (5x7 bitmap font) ---

fn draw_text_halo(pixels: &mut [Color], img_w: u32, h: u32, x: i32, y: i32, text: &str, color: Color) {
    // Draw dark halo first
    for &(dx, dy) in &[(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
        draw_text_inner(pixels, img_w, h, x + dx, y + dy, text, HALO, 0.5);
    }
    // Foreground
    draw_text_inner(pixels, img_w, h, x, y, text, color, 0.95);
}

fn draw_text_inner(pixels: &mut [Color], img_w: u32, h: u32, x: i32, y: i32, text: &str, color: Color, alpha: f32) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some(glyph) = char_glyph(ch) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        let px = cx + col;
                        let py = y + row as i32;
                        if px >= 0 && px < img_w as i32 && py >= 0 && py < h as i32 {
                            blend_pixel(pixels, img_w, px, py, color, alpha);
                        }
                    }
                }
            }
        }
        cx += 6;
    }
}

// --- Pixel blending ---

#[inline]
fn blend_pixel(pixels: &mut [Color], img_w: u32, x: i32, y: i32, color: Color, alpha: f32) {
    let idx = (y as u32 * img_w + x as u32) as usize;
    if idx >= pixels.len() { return; }
    let a = (alpha * (color[3] as f32 / 255.0)).min(1.0);
    if a >= 0.99 {
        pixels[idx] = [color[0], color[1], color[2], 255];
    } else if a > 0.0 {
        let inv = 1.0 - a;
        let dst = pixels[idx];
        pixels[idx] = [
            (color[0] as f32 * a + dst[0] as f32 * inv) as u8,
            (color[1] as f32 * a + dst[1] as f32 * inv) as u8,
            (color[2] as f32 * a + dst[2] as f32 * inv) as u8,
            255,
        ];
    }
}

// --- Bounds check for polylines ---

fn polyline_in_bounds(line: &[(f64, f64)], min_lat: f64, min_lon: f64, max_lat: f64, max_lon: f64) -> bool {
    // (lon, lat) pairs in geodata
    line.iter().any(|&(lon, lat)| {
        lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon
    })
}

// --- 5x7 bitmap font glyphs ---

fn char_glyph(c: char) -> Option<[u8; 7]> {
    Some(match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => return None,
    })
}
