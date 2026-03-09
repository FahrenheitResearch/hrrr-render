/// hrrr-render CLI: The world's fastest HRRR weather map renderer.
///
/// Usage:
///   hrrr-render --field temp2m --run latest --forecast 00 --output temp.png
///   hrrr-render --field ref --run 2024031512 --forecast 06 --output ref.png

use clap::Parser;
use std::fs;
use std::process;

/// The world's fastest HRRR weather map renderer.
#[derive(Parser, Debug)]
#[command(name = "hrrr-render")]
#[command(about = "Render HRRR weather model data as beautiful PNG maps")]
#[command(version)]
struct Args {
    /// Weather field to render
    #[arg(
        short, long,
        help = "Field: temp2m, dewp2m, wind10m, ref, cape, vis, precip, h500"
    )]
    field: String,

    /// Model run time (YYYYMMDDHH or "latest")
    #[arg(short, long, default_value = "latest")]
    run: String,

    /// Forecast hour (00-48)
    #[arg(long, default_value = "00")]
    forecast: String,

    /// Output PNG file path
    #[arg(short, long, default_value = "output.png")]
    output: String,

    /// Output image width in pixels
    #[arg(long, default_value = "1799")]
    width: u32,

    /// Output image height in pixels
    #[arg(long, default_value = "1059")]
    height: u32,
}

fn main() {
    let args = Args::parse();

    // Validate field name
    let valid_fields = hrrr_render::fields::field_names();
    if !valid_fields.contains(&args.field.as_str()) {
        eprintln!(
            "Error: Unknown field '{}'. Valid fields: {}",
            args.field,
            valid_fields.join(", ")
        );
        process::exit(1);
    }

    // Parse forecast hour
    let forecast_hour: u8 = match args.forecast.parse() {
        Ok(h) if h <= 48 => h,
        _ => {
            eprintln!("Error: Forecast hour must be 00-48, got '{}'", args.forecast);
            process::exit(1);
        }
    };

    eprintln!("hrrr-render v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("Field: {} | Run: {} | Forecast: f{:02}", args.field, args.run, forecast_hour);
    eprintln!("Output: {} ({}x{})", args.output, args.width, args.height);

    let start = std::time::Instant::now();

    match hrrr_render::render_field(&args.run, forecast_hour, &args.field, args.width, args.height) {
        Ok(png_data) => {
            // Write PNG to disk
            if let Err(e) = fs::write(&args.output, &png_data) {
                eprintln!("Error writing output file '{}': {}", args.output, e);
                process::exit(1);
            }

            let elapsed = start.elapsed();
            eprintln!(
                "Done! Wrote {} ({} bytes) in {:.2}s",
                args.output,
                png_data.len(),
                elapsed.as_secs_f64()
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
