/// hrrr-render CLI: The world's fastest HRRR weather map renderer.
///
/// Usage:
///   hrrr-render --field temp2m --run latest --forecast 00 --output temp.png
///   hrrr-render --field ref --run 2024031512 --forecast 06 --output ref.png

use clap::Parser;
use std::fs;
use std::process;
use std::time::Instant;

/// The world's fastest HRRR weather map renderer.
#[derive(Parser, Debug)]
#[command(name = "hrrr-render")]
#[command(about = "Render HRRR weather model data as beautiful PNG maps")]
#[command(version)]
struct Args {
    /// Weather field to render
    #[arg(
        short, long,
        default_value = "ref",
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

    /// Render all 8 fields as a showcase benchmark
    #[arg(long)]
    showcase: bool,
}

fn render_one(field_name: &str, run: &str, forecast_hour: u8, output: &str, width: u32, height: u32) -> Result<(), String> {
    let field = hrrr_render::fields::lookup_field(field_name)
        .ok_or_else(|| format!("Unknown field '{}'", field_name))?;

    let (date, run_hour) = hrrr_render::fetch::parse_run(run)
        .map_err(|e| e.to_string())?;

    eprintln!("\n{}", "=".repeat(60));
    eprintln!("  {} ({}) | {}{:02}z f{:02}", field.label, field.unit, date, run_hour, forecast_hour);
    eprintln!("{}", "=".repeat(60));

    // 1. Fetch idx
    let t0 = Instant::now();
    let entries = hrrr_render::fetch::fetch_idx(&date, run_hour, forecast_hour)
        .map_err(|e| e.to_string())?;
    let idx_time = t0.elapsed();
    eprintln!("  idx fetch:     {:>8.1}ms  ({} entries)", idx_time.as_secs_f64() * 1000.0, entries.len());

    // 2. Find field range
    let t1 = Instant::now();
    let (start, end) = hrrr_render::fetch::find_field_range(&entries, field.idx_name, field.level)
        .map_err(|e| e.to_string())?;
    let range_bytes = end.map(|e| e - start + 1).unwrap_or(0);
    let range_time = t1.elapsed();
    eprintln!("  field lookup:  {:>8.1}ms  (bytes {}-{})", range_time.as_secs_f64() * 1000.0, start, end.unwrap_or(0));

    // 3. Download GRIB2 range
    let t2 = Instant::now();
    let grib_data = hrrr_render::fetch::fetch_grib2_range(&date, run_hour, forecast_hour, start, end)
        .map_err(|e| e.to_string())?;
    let dl_time = t2.elapsed();
    let dl_kb = grib_data.len() as f64 / 1024.0;
    let dl_speed = dl_kb / dl_time.as_secs_f64() / 1024.0; // MB/s
    eprintln!("  download:      {:>8.1}ms  ({:.0} KB, {:.1} MB/s)", dl_time.as_secs_f64() * 1000.0, dl_kb, dl_speed);

    // 4+5+6. Parse GRIB2 + unpack data (using grib crate)
    let t3 = Instant::now();
    let (mut values, nx, ny) = hrrr_render::parse_grib2_field(&grib_data)
        .map_err(|e| e.to_string())?;
    let parse_time = Instant::now() - t3;
    let num_vals = values.len();
    let non_nan = values.iter().filter(|v| !v.is_nan()).count();
    eprintln!("  parse+unpack:  {:>8.1}ms  ({}x{}, {} points, {} valid)", parse_time.as_secs_f64() * 1000.0, nx, ny, num_vals, non_nan);
    let unpack_time = parse_time;

    // 7. Unit conversion
    let t5 = Instant::now();
    hrrr_render::fields::convert_values(field, &mut values);
    let conv_time = t5.elapsed();

    // Quick stats
    let valid_vals: Vec<f64> = values.iter().filter(|v| !v.is_nan()).copied().collect();
    let (min_v, max_v, mean_v) = if !valid_vals.is_empty() {
        let min = valid_vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = valid_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = valid_vals.iter().sum();
        (min, max, sum / valid_vals.len() as f64)
    } else {
        (0.0, 0.0, 0.0)
    };
    eprintln!("  unit convert:  {:>8.1}ms  (min={:.1}, max={:.1}, mean={:.1} {})", conv_time.as_secs_f64() * 1000.0, min_v, max_v, mean_v, field.unit);

    // 8. Projection setup
    let t6 = Instant::now();
    let proj = hrrr_render::render::projection::LambertProjection::new(
        38.5, 38.5, -97.5,
        21.138, -122.72,
        3000.0, 3000.0,
        nx as u32, ny as u32,
    );
    let proj_time = t6.elapsed();
    eprintln!("  projection:    {:>8.1}ms", proj_time.as_secs_f64() * 1000.0);

    // 9. Render
    let t7 = Instant::now();
    let png_data = hrrr_render::render::render_to_png(&values, field, &proj, width, height)
        .map_err(|e| e.to_string())?;
    let render_time = t7.elapsed();
    let pixels = width as u64 * height as u64;
    let mpix_per_sec = pixels as f64 / render_time.as_secs_f64() / 1_000_000.0;
    eprintln!("  render+PNG:    {:>8.1}ms  ({}x{}, {:.0} Mpix/s)", render_time.as_secs_f64() * 1000.0, width, height, mpix_per_sec);

    // 10. Write to disk
    let t8 = Instant::now();
    fs::write(output, &png_data).map_err(|e| e.to_string())?;
    let write_time = t8.elapsed();
    eprintln!("  file write:    {:>8.1}ms  ({:.0} KB)", write_time.as_secs_f64() * 1000.0, png_data.len() as f64 / 1024.0);

    let total = t0.elapsed();
    eprintln!("  ──────────────────────────────────");
    eprintln!("  TOTAL:         {:>8.1}ms", total.as_secs_f64() * 1000.0);
    eprintln!("  Network:       {:>8.1}ms ({:.0}%)", (idx_time + dl_time).as_secs_f64() * 1000.0, (idx_time + dl_time).as_secs_f64() / total.as_secs_f64() * 100.0);
    eprintln!("  Compute:       {:>8.1}ms ({:.0}%)", (parse_time + unpack_time + conv_time + proj_time + render_time).as_secs_f64() * 1000.0, (parse_time + unpack_time + conv_time + proj_time + render_time).as_secs_f64() / total.as_secs_f64() * 100.0);

    Ok(())
}

fn main() {
    let args = Args::parse();

    if args.showcase {
        eprintln!("╔══════════════════════════════════════════════════════════╗");
        eprintln!("║         hrrr-render SHOWCASE BENCHMARK                  ║");
        eprintln!("║         The World's Fastest HRRR Map Renderer           ║");
        eprintln!("╚══════════════════════════════════════════════════════════╝");

        let fields = ["ref", "temp2m", "dewp2m", "wind10m", "cape", "vis", "precip", "h500"];
        let forecast_hour: u8 = args.forecast.parse().unwrap_or(0);
        let total_start = Instant::now();
        let mut success = 0;
        let mut fail = 0;

        for field_name in &fields {
            let output = format!("output/{}.png", field_name);
            fs::create_dir_all("output").ok();
            match render_one(field_name, &args.run, forecast_hour, &output, args.width, args.height) {
                Ok(()) => {
                    eprintln!("  -> {}", output);
                    success += 1;
                }
                Err(e) => {
                    eprintln!("  FAILED: {}", e);
                    fail += 1;
                }
            }
        }

        let total = total_start.elapsed();
        eprintln!("\n╔══════════════════════════════════════════════════════════╗");
        eprintln!("║  SHOWCASE COMPLETE                                      ║");
        eprintln!("║  {} of {} fields rendered in {:.1}s                    ║", success, fields.len(), total.as_secs_f64());
        eprintln!("║  Avg: {:.1}ms per field                                ║", total.as_secs_f64() * 1000.0 / fields.len() as f64);
        if fail > 0 {
            eprintln!("║  {} fields failed                                       ║", fail);
        }
        eprintln!("╚══════════════════════════════════════════════════════════╝");
        return;
    }

    // Single field mode
    let valid_fields = hrrr_render::fields::field_names();
    if !valid_fields.contains(&args.field.as_str()) {
        eprintln!(
            "Error: Unknown field '{}'. Valid fields: {}",
            args.field,
            valid_fields.join(", ")
        );
        process::exit(1);
    }

    let forecast_hour: u8 = match args.forecast.parse() {
        Ok(h) if h <= 48 => h,
        _ => {
            eprintln!("Error: Forecast hour must be 00-48, got '{}'", args.forecast);
            process::exit(1);
        }
    };

    eprintln!("hrrr-render v{}", env!("CARGO_PKG_VERSION"));

    match render_one(&args.field, &args.run, forecast_hour, &args.output, args.width, args.height) {
        Ok(()) => eprintln!("\nDone!"),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
