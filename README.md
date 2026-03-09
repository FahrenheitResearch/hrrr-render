# hrrr-render

The world's fastest HRRR (High Resolution Rapid Refresh) map renderer, written in pure Rust.

Downloads HRRR GRIB2 data from NOAA's AWS S3 bucket using byte-range requests,
parses GRIB2 format, and renders beautiful weather maps as PNG using rayon parallelism.

## Supported fields

- `temp2m` - 2m Temperature
- `dewp2m` - 2m Dewpoint
- `wind10m` - 10m Wind Speed
- `ref` - Composite Reflectivity
- `cape` - Convective Available Potential Energy
- `vis` - Visibility
- `precip` - Precipitation Rate
- `h500` - 500mb Geopotential Heights

## Usage

```
hrrr-render --field temp2m --run latest --forecast 00 --output temp.png
hrrr-render --field ref --run 2024031512 --forecast 06 --output ref.png
hrrr-render --field cape --run latest --forecast 03 --output cape.png --width 1920 --height 1080
```

## Build

```
cargo build --release
```

## How it works

1. Fetches the `.idx` sidecar file to locate the requested field's byte range
2. Downloads only the relevant GRIB2 message via HTTP Range request
3. Parses GRIB2 sections (grid definition, product definition, data representation)
4. Unpacks data values (simple packing or JPEG2000)
5. Renders to PNG using Lambert Conformal Conic projection with color mapping
6. Draws state/country borders and a color legend
