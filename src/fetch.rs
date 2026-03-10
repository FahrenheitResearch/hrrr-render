/// S3/HTTP fetcher for HRRR GRIB2 data with byte-range support.
///
/// HRRR data is on AWS S3 at `noaa-hrrr-bdp-pds` (no auth required).
/// Each GRIB2 file has a `.idx` sidecar listing byte offsets per field.

use chrono::{Duration, Timelike, Utc};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io;
use std::sync::{Mutex, OnceLock};
use std::time;

const HRRR_BASE_URL: &str = "https://noaa-hrrr-bdp-pds.s3.amazonaws.com";

/// Global shared HTTP client with connection pooling.
fn shared_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .tcp_nodelay(true)
            .pool_idle_timeout(time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .timeout(time::Duration::from_secs(60))
            .connect_timeout(time::Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Global idx cache keyed by (date, run_hour, forecast_hour).
fn idx_cache() -> &'static Mutex<HashMap<(String, u8, u8), Vec<IdxEntry>>> {
    static CACHE: OnceLock<Mutex<HashMap<(String, u8, u8), Vec<IdxEntry>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// An entry parsed from a HRRR .idx file.
#[derive(Debug, Clone)]
pub struct IdxEntry {
    pub line_num: u32,
    pub byte_offset: u64,
    pub date: String,
    pub field_name: String,
    pub level: String,
    pub forecast: String,
}

/// Build the URL for a HRRR GRIB2 file.
pub fn grib2_url(date: &str, run_hour: u8, forecast_hour: u8) -> String {
    format!(
        "{}/hrrr.{}/conus/hrrr.t{:02}z.wrfsfcf{:02}.grib2",
        HRRR_BASE_URL, date, run_hour, forecast_hour
    )
}

/// Build the URL for the .idx sidecar file.
pub fn idx_url(date: &str, run_hour: u8, forecast_hour: u8) -> String {
    format!("{}.idx", grib2_url(date, run_hour, forecast_hour))
}

/// Resolve "latest" to a (date_string, run_hour) pair.
/// HRRR runs every hour with ~1.5hr delay.
pub fn resolve_latest() -> (String, u8) {
    let now = Utc::now();
    // Subtract 2 hours to account for processing delay
    let run_time = now - Duration::hours(2);
    let date = run_time.format("%Y%m%d").to_string();
    let hour = run_time.hour() as u8;
    (date, hour)
}

/// Parse a run specification like "2024031512" or "latest" into (date, hour).
pub fn parse_run(run: &str) -> io::Result<(String, u8)> {
    if run == "latest" {
        return Ok(resolve_latest());
    }

    if run.len() != 10 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Run must be YYYYMMDDHH or 'latest', got '{}'", run),
        ));
    }

    let date = &run[..8];
    let hour: u8 = run[8..10].parse().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "Invalid hour in run string")
    })?;

    if hour > 23 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Hour must be 0-23, got {}", hour),
        ));
    }

    // Validate date
    let _year: i32 = date[0..4].parse().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "Invalid year")
    })?;
    let _month: u32 = date[4..6].parse().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "Invalid month")
    })?;
    let _day: u32 = date[6..8].parse().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "Invalid day")
    })?;

    Ok((date.to_string(), hour))
}

/// Download and parse the .idx file for a given run/forecast.
/// Results are cached so repeated calls for the same (date, run_hour, forecast_hour) are free.
pub fn fetch_idx(date: &str, run_hour: u8, forecast_hour: u8) -> io::Result<Vec<IdxEntry>> {
    let key = (date.to_string(), run_hour, forecast_hour);

    // Check cache first
    {
        let cache = idx_cache().lock().unwrap();
        if let Some(entries) = cache.get(&key) {
            return Ok(entries.clone());
        }
    }

    // Cache miss - fetch from network
    let url = idx_url(date, run_hour, forecast_hour);
    eprintln!("Fetching idx: {}", url);

    let resp = shared_client().get(&url).send().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("HTTP error fetching idx: {}", e),
        )
    })?;

    if !resp.status().is_success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to fetch idx (HTTP {}): {}", resp.status(), url),
        ));
    }

    let text = resp.text().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Error reading idx body: {}", e),
        )
    })?;

    let entries = parse_idx(&text)?;

    // Store in cache
    {
        let mut cache = idx_cache().lock().unwrap();
        cache.insert(key, entries.clone());
    }

    Ok(entries)
}

/// Parse .idx file contents.
/// Format: `linenum:byte_offset:d=YYYYMMDDHH:FIELD:level:fcst`
pub fn parse_idx(text: &str) -> io::Result<Vec<IdxEntry>> {
    let mut entries = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(7, ':').collect();
        if parts.len() < 6 {
            continue;
        }

        let line_num: u32 = parts[0].parse().unwrap_or(0);
        let byte_offset: u64 = parts[1].parse().unwrap_or(0);
        let date = parts[2].trim_start_matches("d=").to_string();
        let field_name = parts[3].to_string();
        let level = parts[4].to_string();
        let forecast = if parts.len() > 5 {
            parts[5].to_string()
        } else {
            String::new()
        };

        entries.push(IdxEntry {
            line_num,
            byte_offset,
            date,
            field_name,
            level,
            forecast,
        });
    }

    Ok(entries)
}

/// Find the byte range for a specific field in the idx entries.
/// Returns (start_byte, end_byte) where end_byte is exclusive (or None for last field).
pub fn find_field_range(
    entries: &[IdxEntry],
    idx_name: &str,
    level: &str,
) -> io::Result<(u64, Option<u64>)> {
    for (i, entry) in entries.iter().enumerate() {
        if entry.field_name == idx_name && entry.level.contains(level) {
            let start = entry.byte_offset;
            let end = if i + 1 < entries.len() {
                Some(entries[i + 1].byte_offset - 1)
            } else {
                None
            };
            return Ok((start, end));
        }
    }

    // Try a more lenient match
    for (i, entry) in entries.iter().enumerate() {
        if entry.field_name == idx_name {
            let start = entry.byte_offset;
            let end = if i + 1 < entries.len() {
                Some(entries[i + 1].byte_offset - 1)
            } else {
                None
            };
            eprintln!(
                "Warning: exact level '{}' not found for {}, using '{}' instead",
                level, idx_name, entry.level
            );
            return Ok((start, end));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Field '{}' at level '{}' not found in idx. Available fields: {}",
            idx_name,
            level,
            entries
                .iter()
                .map(|e| format!("{}:{}", e.field_name, e.level))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ))
}

/// Download a byte range of the GRIB2 file.
pub fn fetch_grib2_range(
    date: &str,
    run_hour: u8,
    forecast_hour: u8,
    start: u64,
    end: Option<u64>,
) -> io::Result<Vec<u8>> {
    let url = grib2_url(date, run_hour, forecast_hour);

    let range = match end {
        Some(e) => format!("bytes={}-{}", start, e),
        None => format!("bytes={}-", start),
    };

    eprintln!("Fetching GRIB2 data: {} ({})", url, range);

    let resp = shared_client()
        .get(&url)
        .header("Range", &range)
        .send()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("HTTP error: {}", e)))?;

    if !resp.status().is_success() && resp.status().as_u16() != 206 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("HTTP {} fetching GRIB2 data", resp.status()),
        ));
    }

    let bytes = resp
        .bytes()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Error reading body: {}", e)))?;

    eprintln!("Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

/// High-level function: fetch a specific field from HRRR.
pub fn fetch_field(
    date: &str,
    run_hour: u8,
    forecast_hour: u8,
    idx_name: &str,
    level: &str,
) -> io::Result<Vec<u8>> {
    let entries = fetch_idx(date, run_hour, forecast_hour)?;
    let (start, end) = find_field_range(&entries, idx_name, level)?;
    fetch_grib2_range(date, run_hour, forecast_hour, start, end)
}

/// Fetch multiple fields in parallel using rayon.
///
/// Fetches the idx once (cached), then downloads all requested GRIB2 byte ranges concurrently.
/// `fields` is a slice of (idx_name, level) pairs.
/// Returns a Vec of GRIB2 byte buffers in the same order as the input fields.
pub fn fetch_fields_parallel(
    date: &str,
    run_hour: u8,
    fhour: u8,
    fields: &[(&str, &str)],
) -> io::Result<Vec<Vec<u8>>> {
    // Fetch and cache idx once
    let entries = fetch_idx(date, run_hour, fhour)?;

    // Resolve all byte ranges up front (fast, no I/O)
    let ranges: Vec<(u64, Option<u64>)> = fields
        .iter()
        .map(|(name, level)| find_field_range(&entries, name, level))
        .collect::<io::Result<Vec<_>>>()?;

    // Fetch all ranges in parallel via rayon
    let results: Vec<io::Result<Vec<u8>>> = ranges
        .into_par_iter()
        .map(|(start, end)| fetch_grib2_range(date, run_hour, fhour, start, end))
        .collect();

    // Collect results, propagating the first error
    results.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_idx() {
        let idx_text = r#"1:0:d=2024031512:REFC:entire atmosphere:anl:
2:5765388:d=2024031512:RETOP:cloud top:anl:
3:8914498:d=2024031512:VIS:surface:anl:
4:16968498:d=2024031512:TMP:2 m above ground:anl:
5:22003498:d=2024031512:DPT:2 m above ground:anl:
"#;
        let entries = parse_idx(idx_text).unwrap();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].field_name, "REFC");
        assert_eq!(entries[0].byte_offset, 0);
        assert_eq!(entries[3].field_name, "TMP");
        assert_eq!(entries[3].level, "2 m above ground");
    }

    #[test]
    fn test_find_field_range() {
        let entries = vec![
            IdxEntry { line_num: 1, byte_offset: 0, date: "2024031512".into(), field_name: "REFC".into(), level: "entire atmosphere".into(), forecast: "anl".into() },
            IdxEntry { line_num: 2, byte_offset: 5000, date: "2024031512".into(), field_name: "TMP".into(), level: "2 m above ground".into(), forecast: "anl".into() },
            IdxEntry { line_num: 3, byte_offset: 10000, date: "2024031512".into(), field_name: "DPT".into(), level: "2 m above ground".into(), forecast: "anl".into() },
        ];
        let (start, end) = find_field_range(&entries, "TMP", "2 m above ground").unwrap();
        assert_eq!(start, 5000);
        assert_eq!(end, Some(9999));
    }

    #[test]
    fn test_parse_run() {
        let (date, hour) = parse_run("2024031512").unwrap();
        assert_eq!(date, "20240315");
        assert_eq!(hour, 12);
    }
}
