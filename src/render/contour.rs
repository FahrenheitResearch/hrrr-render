/// Contour line generation for height fields (e.g., 500mb heights).
///
/// Uses a simple marching-squares approach to generate contour lines
/// at fixed intervals.

/// Generate contour "hits" for a grid of values.
/// Returns a boolean mask where true = pixel is on a contour line.
pub fn contour_mask(
    values: &[f64],
    nx: usize,
    ny: usize,
    interval: f64,
) -> Vec<bool> {
    let mut mask = vec![false; nx * ny];

    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let idx = j * nx + i;
            let v00 = values[idx];
            let v10 = values[idx + 1];
            let v01 = values[(j + 1) * nx + i];
            let v11 = values[(j + 1) * nx + i + 1];

            if v00.is_nan() || v10.is_nan() || v01.is_nan() || v11.is_nan() {
                continue;
            }

            // Check if a contour line crosses this cell
            let level_min = v00.min(v10).min(v01).min(v11);
            let level_max = v00.max(v10).max(v01).max(v11);

            // Find the lowest contour level above level_min
            let first_level = (level_min / interval).ceil() * interval;

            if first_level <= level_max {
                // There's at least one contour crossing this cell
                // Mark all four corners
                mask[idx] = true;
                mask[idx + 1] = true;
                mask[(j + 1) * nx + i] = true;
                mask[(j + 1) * nx + i + 1] = true;
            }
        }
    }

    mask
}

/// For 500mb heights, generate contour lines every 60 meters (6 decameters).
pub fn height_contour_mask(values: &[f64], nx: usize, ny: usize) -> Vec<bool> {
    // Values should already be in decameters, contour every 6 dam
    contour_mask(values, nx, ny, 6.0)
}
