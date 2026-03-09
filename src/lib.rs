/// hrrr-render: The world's fastest HRRR weather map renderer.
///
/// This library provides:
/// - GRIB2 parsing (sections 0-8, simple packing + JPEG2000)
/// - Byte-range fetching from NOAA's AWS S3 HRRR archive
/// - Parallel map rendering with Lambert Conformal Conic projection
/// - Color tables for common weather fields

pub mod fetch;
pub mod fields;
pub mod grib2;
pub mod render;

use std::io;

/// High-level function: fetch, parse, and render a HRRR field to PNG bytes.
pub fn render_field(
    run: &str,
    forecast_hour: u8,
    field_name: &str,
    width: u32,
    height: u32,
) -> io::Result<Vec<u8>> {
    // Look up field definition
    let field = fields::lookup_field(field_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unknown field '{}'. Available: {}",
                field_name,
                fields::field_names().join(", ")
            ),
        )
    })?;

    // Parse run time
    let (date, run_hour) = fetch::parse_run(run)?;

    eprintln!(
        "Rendering {} ({}), run={}{:02}z, f{:02}",
        field.label, field.unit, date, run_hour, forecast_hour
    );

    // Fetch the GRIB2 data for this field
    let grib_data = fetch::fetch_field(&date, run_hour, forecast_hour, field.idx_name, field.level)?;

    // Parse the GRIB2 message
    let messages = grib2::parse_messages(&grib_data)?;
    if messages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No GRIB2 messages found in downloaded data",
        ));
    }

    let msg = &messages[0];
    eprintln!(
        "GRIB2: discipline={}, category={}, parameter={}",
        msg.indicator.discipline,
        msg.product_definition.parameter_category,
        msg.product_definition.parameter_number
    );

    // Get grid info
    let grid = msg.lambert_grid().unwrap_or_else(|_| {
        eprintln!("Warning: Could not parse Lambert grid from GRIB2, using HRRR defaults");
        grib2::templates::LambertConformal {
            nx: 1799,
            ny: 1059,
            la1: 21.138,
            lo1: -122.72,
            lad: 38.5,
            lov: -97.5,
            dx: 3000.0,
            dy: 3000.0,
            latin1: 38.5,
            latin2: 38.5,
            scan_mode: 0x40,
        }
    });

    eprintln!("Grid: {}x{}, la1={}, lo1={}", grid.nx, grid.ny, grid.la1, grid.lo1);

    // Unpack data values
    let mut values = msg.unpack_values()?;
    eprintln!("Unpacked {} values", values.len());

    // Apply unit conversions
    fields::convert_values(field, &mut values);

    // Set up projection
    let proj = render::projection::LambertProjection::new(
        grid.latin1, grid.latin2, grid.lov,
        grid.la1, grid.lo1,
        grid.dx, grid.dy,
        grid.nx, grid.ny,
    );

    // Render to PNG
    let png_data = render::render_to_png(&values, field, &proj, width, height)?;

    eprintln!("Rendered {}x{} PNG ({} bytes)", width + 60, height, png_data.len());

    Ok(png_data)
}
