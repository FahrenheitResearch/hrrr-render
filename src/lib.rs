/// hrrr-render: The world's fastest HRRR weather map renderer.

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

    let (date, run_hour) = fetch::parse_run(run)?;

    eprintln!(
        "Rendering {} ({}), run={}{:02}z, f{:02}",
        field.label, field.unit, date, run_hour, forecast_hour
    );

    let grib_data = fetch::fetch_field(&date, run_hour, forecast_hour, field.idx_name, field.level)?;

    // Parse using the grib crate
    let (mut values, nx, ny) = parse_grib2_field(&grib_data)?;

    eprintln!("Unpacked {} values ({}x{})", values.len(), nx, ny);

    // Apply unit conversions
    fields::convert_values(field, &mut values);

    // Set up projection with HRRR standard parameters
    let proj = render::projection::LambertProjection::new(
        38.5, 38.5, -97.5,
        21.138, -122.72,
        3000.0, 3000.0,
        nx as u32, ny as u32,
    );

    let png_data = render::render_to_png(&values, field, &proj, width, height)?;
    Ok(png_data)
}

/// Parse a GRIB2 byte buffer using the `grib` crate and return (values, nx, ny).
pub fn parse_grib2_field(data: &[u8]) -> io::Result<(Vec<f64>, usize, usize)> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let grib2 = grib::Grib2::<grib::SeekableGrib2Reader<Cursor<&[u8]>>>::read_with_seekable(cursor)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("GRIB2 parse error: {:?}", e)))?;

    // Get the first submessage
    let mut submessages = grib2.submessages();
    let (_, first) = submessages.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "No submessages in GRIB2 data")
    })?;

    // Get grid shape
    let (nx, ny) = first.grid_shape()
        .unwrap_or((1799, 1059));

    // Decode data values
    let decoder = grib::Grib2SubmessageDecoder::from(first)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Decoder init error: {:?}", e)))?;

    let decoded = decoder.dispatch()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Decode error: {:?}", e)))?;

    let values: Vec<f64> = decoded.map(|v| v as f64).collect();

    Ok((values, nx, ny))
}
