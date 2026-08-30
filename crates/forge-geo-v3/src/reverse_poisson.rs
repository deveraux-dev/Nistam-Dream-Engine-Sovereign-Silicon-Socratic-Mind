//! Ported verbatim from F:\NewRepo\crates\CUI\forge-render\src\reverse_poisson.rs (2026-08-17 truth-hunt lineage port).

// =============================================================================
// reverse_poisson.rs — Reverse Poisson Differential Inspection
// =============================================================================
// Decomposes a known-good reference surface into its expected gradient field,
// compares against observed gradients from a real capture, and isolates defect
// geometry as the residual.
//
// Forward Poisson: normals → height (reconstruction)
// Reverse Poisson: height → gradients (decomposition)
// Differential: expected_gradients - observed_gradients = defect_signal
//
// The residual undergoes magnitude thresholding to produce a defect map
// with absolute depth measurement capability (calibrated against synthetic).
//
// All math is integer-based (Permyriad). No floating-point on the hot path.
// =============================================================================

use crate::synthetic_weld::SyntheticWeld;

/// Result of the reverse Poisson differential inspection.
#[derive(Debug, Clone)]
pub struct DifferentialResult {
    /// Residual magnitude at each pixel (0 = matches reference, >0 = deviation)
    pub residual_magnitude: Vec<u32>,
    /// Residual direction: positive = deeper than expected, negative = raised
    pub residual_signed: Vec<i32>,
    /// Detected defect regions from the residual
    pub defect_regions: Vec<DifferentialDefect>,
    /// Calibrated depth map (mm × 1000, using synthetic transfer function)
    pub depth_map_microns: Vec<u32>,
    /// Overall match score (Permyriad: 10000 = perfect match, 0 = total mismatch)
    pub match_score: u32,
    /// Grid dimensions
    pub width: u32,
    pub height: u32,
}

/// A defect detected via differential gradient analysis.
#[derive(Debug, Clone)]
pub struct DifferentialDefect {
    /// Bounding box
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Maximum residual magnitude in this region (Permyriad)
    pub max_residual: u32,
    /// Estimated depth in microns (calibrated from synthetic)
    pub depth_microns: u32,
    /// Gradient direction classification
    pub direction: GradientDirection,
}

/// Gradient direction at a defect — indicates defect type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDirection {
    /// Inward (negative divergence) — void/porosity
    Inward,
    /// Lateral (shear component dominant) — crack
    Lateral,
    /// Flat (zero gradient where slope expected) — lack of fusion
    Flat,
    /// Outward (positive divergence) — raised inclusion or spatter
    Outward,
}

/// Configuration for differential inspection.
#[derive(Debug, Clone, Copy)]
pub struct DifferentialConfig {
    /// Minimum residual magnitude (Permyriad) to flag as defect
    pub residual_threshold: u32,
    /// Minimum connected region size (pixels) to count as defect
    pub min_region_size: u32,
    /// Calibration factor: Permyriad residual per micron of depth
    /// Derived from synthetic: if 1mm depth = 3000 Permyriad residual,
    /// then factor = 3000/1000 = 3 Permyriad per micron
    pub permyriad_per_micron: u32,
}

impl Default for DifferentialConfig {
    fn default() -> Self {
        Self {
            residual_threshold: 200,  // 2% deviation triggers
            min_region_size: 4,
            permyriad_per_micron: 3,  // Calibrated from synthetic
        }
    }
}

/// Extract gradient field from an image (observed gradients).
/// Uses central difference on luminance values.
///
/// Input: RGBA pixels, width, height
/// Output: (gradient_x, gradient_y) as i32 arrays in Permyriad scale
pub fn extract_observed_gradients(
    rgba_pixels: &[u8],
    width: u32,
    height: u32,
) -> (Vec<i32>, Vec<i32>) {
    let w = width as usize;
    let h = height as usize;
    let total = w * h;

    // Convert to luminance (Permyriad: 0-10000)
    let mut luma: Vec<i32> = Vec::with_capacity(total);
    for pixel in rgba_pixels.chunks_exact(4).take(total) {
        let l = (pixel[0] as u32 * 2126 + pixel[1] as u32 * 7152 + pixel[2] as u32 * 722) / 1000;
        luma.push(l as i32); // 0-2550 range (luminance × 10)
    }

    // Compute gradients via central difference
    let mut gx: Vec<i32> = vec![0; total];
    let mut gy: Vec<i32> = vec![0; total];

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            gx[idx] = (luma[idx + 1] - luma[idx - 1]) / 2;
            gy[idx] = (luma[(y + 1) * w + x] - luma[(y - 1) * w + x]) / 2;
        }
    }

    (gx, gy)
}

/// Compute the differential residual between expected and observed gradient fields.
///
/// residual = observed - expected
/// Where residual ≈ 0: surface matches reference (healthy)
/// Where residual ≠ 0: surface deviates (defect)
pub fn compute_residual(
    expected_gx: &[i32],
    expected_gy: &[i32],
    observed_gx: &[i32],
    observed_gy: &[i32],
    width: u32,
    height: u32,
    config: &DifferentialConfig,
) -> DifferentialResult {
    let w = width as usize;
    let h = height as usize;
    let total = w * h;

    assert_eq!(expected_gx.len(), total);
    assert_eq!(observed_gx.len(), total);

    // Compute residual magnitude and signed value
    let mut residual_magnitude: Vec<u32> = vec![0; total];
    let mut residual_signed: Vec<i32> = vec![0; total];
    let mut depth_map_microns: Vec<u32> = vec![0; total];

    let mut total_residual: u64 = 0;
    let mut max_possible: u64 = 0;

    for i in 0..total {
        let rx = observed_gx[i] - expected_gx[i];
        let ry = observed_gy[i] - expected_gy[i];

        // Magnitude: |rx| + |ry| (L1 norm, faster than L2, integer-safe)
        let mag = (rx.unsigned_abs() + ry.unsigned_abs()) as u32;
        residual_magnitude[i] = mag;

        // Signed: divergence approximation (positive = deeper than expected)
        // Use the component aligned with the expected gradient direction
        let expected_mag = (expected_gx[i].unsigned_abs() + expected_gy[i].unsigned_abs()) as i32;
        if expected_mag > 0 {
            // Project residual onto expected gradient direction
            let dot = rx as i64 * expected_gx[i] as i64 + ry as i64 * expected_gy[i] as i64;
            residual_signed[i] = (dot / expected_mag as i64) as i32;
        } else {
            residual_signed[i] = -(mag as i32); // No expected gradient → any residual is a void
        }

        // Calibrated depth
        if mag > config.residual_threshold {
            depth_map_microns[i] = mag / config.permyriad_per_micron.max(1);
        }

        total_residual += mag as u64;
        max_possible += 500; // Normalization constant (typical max gradient)
    }

    // Match score: how well the observed matches expected (10000 = perfect)
    let match_score = if max_possible > 0 {
        10000u32.saturating_sub(((total_residual * 10000) / max_possible).min(10000) as u32)
    } else {
        10000
    };

    // Extract defect regions via flood fill on thresholded residual
    let defect_regions = extract_defect_regions(
        &residual_magnitude,
        &residual_signed,
        &depth_map_microns,
        width, height, config,
    );

    DifferentialResult {
        residual_magnitude,
        residual_signed,
        defect_regions,
        depth_map_microns,
        match_score,
        width,
        height,
    }
}

/// Run the full differential inspection pipeline.
///
/// 1. Generate or load synthetic reference
/// 2. Extract observed gradients from the real image
/// 3. Compute residual
/// 4. Threshold and classify defects
pub fn inspect_differential(
    rgba_pixels: &[u8],
    width: u32,
    height: u32,
    reference: &SyntheticWeld,
    config: &DifferentialConfig,
) -> DifferentialResult {
    // Extract observed gradients from the real image
    let (obs_gx, obs_gy) = extract_observed_gradients(rgba_pixels, width, height);

    // The reference may be a different size — we need to resample
    // For now, require same dimensions (synthetic generated at image size)
    assert_eq!(reference.width, width, "Reference and image must be same width");
    assert_eq!(reference.height, height, "Reference and image must be same height");

    // Compute residual
    compute_residual(
        &reference.gradient_x,
        &reference.gradient_y,
        &obs_gx,
        &obs_gy,
        width, height, config,
    )
}

/// Extract defect regions from the thresholded residual via flood fill.
fn extract_defect_regions(
    magnitude: &[u32],
    signed: &[i32],
    depth_microns: &[u32],
    width: u32,
    height: u32,
    config: &DifferentialConfig,
) -> Vec<DifferentialDefect> {
    let w = width as usize;
    let h = height as usize;
    let total = w * h;
    let mut visited: Vec<bool> = vec![false; total];
    let mut regions: Vec<DifferentialDefect> = Vec::new();

    for start_y in 0..h {
        for start_x in 0..w {
            let start_idx = start_y * w + start_x;
            if visited[start_idx] || magnitude[start_idx] < config.residual_threshold {
                continue;
            }

            // Flood fill
            let mut stack: Vec<(usize, usize)> = vec![(start_x, start_y)];
            let mut min_x = start_x;
            let mut max_x = start_x;
            let mut min_y = start_y;
            let mut max_y = start_y;
            let mut max_mag: u32 = 0;
            let mut max_depth: u32 = 0;
            let mut sum_signed: i64 = 0;
            let mut _sum_gx: i64 = 0;
            let mut _sum_gy: i64 = 0;
            let mut count: u32 = 0;

            while let Some((cx, cy)) = stack.pop() {
                let idx = cy * w + cx;
                if visited[idx] || magnitude[idx] < config.residual_threshold {
                    continue;
                }
                visited[idx] = true;
                count += 1;

                max_mag = max_mag.max(magnitude[idx]);
                max_depth = max_depth.max(depth_microns[idx]);
                sum_signed += signed[idx] as i64;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                // 4-connected
                if cx > 0 { stack.push((cx - 1, cy)); }
                if cx < w - 1 { stack.push((cx + 1, cy)); }
                if cy > 0 { stack.push((cx, cy - 1)); }
                if cy < h - 1 { stack.push((cx, cy + 1)); }
            }

            if count < config.min_region_size {
                continue;
            }

            // Classify gradient direction
            let avg_signed = sum_signed / count as i64;
            let direction = if avg_signed < -(config.residual_threshold as i64 / 2) {
                GradientDirection::Inward // Void
            } else if avg_signed > (config.residual_threshold as i64 / 2) {
                GradientDirection::Outward // Raised
            } else {
                // Check aspect ratio for lateral vs flat
                let rw = (max_x - min_x + 1) as u32;
                let rh = (max_y - min_y + 1) as u32;
                let aspect = if rh > 0 { rw * 1000 / rh } else { 1000 };
                if aspect > 3000 || aspect < 333 {
                    GradientDirection::Lateral // Crack-like
                } else {
                    GradientDirection::Flat // Lack of fusion
                }
            };

            regions.push(DifferentialDefect {
                x: min_x as u32,
                y: min_y as u32,
                width: (max_x - min_x + 1) as u32,
                height: (max_y - min_y + 1) as u32,
                max_residual: max_mag,
                depth_microns: max_depth,
                direction,
            });
        }
    }

    regions.sort_by(|a, b| b.max_residual.cmp(&a.max_residual));
    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic_weld::{generate_synthetic, SyntheticConfig};

    #[test]
    fn test_perfect_match_high_score() {
        // Compare synthetic against itself — should get near-perfect match
        let config = SyntheticConfig { width: 32, height: 64, defects: vec![], ..Default::default() };
        let weld = generate_synthetic(&config);

        // "Observe" the same gradients (perfect match)
        let result = compute_residual(
            &weld.gradient_x, &weld.gradient_y,
            &weld.gradient_x, &weld.gradient_y,
            config.width, config.height,
            &DifferentialConfig::default(),
        );

        // Residual should be zero everywhere
        assert!(result.residual_magnitude.iter().all(|&m| m == 0));
        assert_eq!(result.match_score, 10000);
        assert!(result.defect_regions.is_empty());
    }

    #[test]
    fn test_defect_produces_residual() {
        // Synthetic with defects vs synthetic without defects
        let clean = SyntheticConfig { width: 64, height: 128, defects: vec![], ..Default::default() };
        let dirty = SyntheticConfig::default();
        // Resize dirty to match clean
        let dirty = SyntheticConfig { width: 64, height: 128, ..dirty };

        let clean_weld = generate_synthetic(&clean);
        let dirty_weld = generate_synthetic(&dirty);

        let result = compute_residual(
            &clean_weld.gradient_x, &clean_weld.gradient_y,
            &dirty_weld.gradient_x, &dirty_weld.gradient_y,
            64, 128,
            &DifferentialConfig { residual_threshold: 50, min_region_size: 2, ..Default::default() },
        );

        // Should detect some residual where defects are
        assert!(result.residual_magnitude.iter().any(|&m| m > 0));
        assert!(result.match_score < 10000);
    }

    #[test]
    fn test_gradient_extraction() {
        // Simple gradient: linear ramp
        let w = 10u32;
        let h = 10u32;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        // Horizontal ramp: left=0, right=255
        for y in 0..h {
            for x in 0..w {
                let val = (x * 255 / (w - 1)) as u8;
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx] = val;
                pixels[idx + 1] = val;
                pixels[idx + 2] = val;
                pixels[idx + 3] = 255;
            }
        }

        let (gx, gy) = extract_observed_gradients(&pixels, w, h);
        // Horizontal gradient should be positive (increasing left to right)
        let mid = (5 * w + 5) as usize;
        assert!(gx[mid] > 0);
        // Vertical gradient should be ~0 (no vertical change)
        assert_eq!(gy[mid], 0);
    }
}
