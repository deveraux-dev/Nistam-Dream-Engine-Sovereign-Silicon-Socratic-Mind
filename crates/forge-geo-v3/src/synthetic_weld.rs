// =============================================================================
// synthetic_weld.rs — Procedural Reference Weld Generator
// =============================================================================
// Generates a mathematically perfect synthetic weld with known defects at
// known depths. Used as the absolute reference for calibration.
//
// The synthetic height field contains:
//   - Base metal (flat, known height)
//   - Weld bead (Gaussian cross-section, sinusoidal ripple along axis)
//   - Known defects at exact positions and depths
//
// All values in Permyriad (0-10000) height space.
// =============================================================================

/// A synthetic defect placed in the reference weld.
#[derive(Debug, Clone)]
pub struct SyntheticDefect {
    /// Position along the weld axis (0.0 to 1.0 normalized)
    pub axis_pos: f32,
    /// Position perpendicular to axis (0.0 = center, ±1.0 = edge)
    pub perp_pos: f32,
    /// Depth in Permyriad (how deep the void goes below the bead surface)
    pub depth: u32,
    /// Radius in normalized units (0.0 to 0.1 typical)
    pub radius: f32,
    /// Shape of the defect
    pub shape: DefectShape,
}

/// Shape of a synthetic defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefectShape {
    /// Circular void (porosity)
    Round,
    /// Elongated along axis (crack)
    LinearAxial,
    /// Wide perpendicular (lack of fusion)
    LinearPerp,
}

/// Configuration for the synthetic weld generator.
#[derive(Debug, Clone)]
pub struct SyntheticConfig {
    /// Width of the output grid (pixels)
    pub width: u32,
    /// Height of the output grid (pixels)
    pub height: u32,
    /// Bead width as fraction of grid perpendicular dimension (0.0-1.0)
    pub bead_width_frac: f32,
    /// Bead peak height in Permyriad (above base metal)
    pub bead_height: u32,
    /// Ripple frequency (number of ripples along the axis)
    pub ripple_count: u32,
    /// Ripple amplitude in Permyriad
    pub ripple_amplitude: u32,
    /// Base metal height in Permyriad
    pub base_height: u32,
    /// Defects to embed
    pub defects: Vec<SyntheticDefect>,
    /// Weld orientation (true = vertical axis along Y, false = horizontal along X)
    pub vertical: bool,
}

impl Default for SyntheticConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 512,
            bead_width_frac: 0.3,
            bead_height: 3000,
            ripple_count: 40,
            ripple_amplitude: 200,
            base_height: 5000,
            defects: default_defect_set(),
            vertical: true,
        }
    }
}

/// Standard set of known defects for calibration.
/// Covers the full range of types and depths.
fn default_defect_set() -> Vec<SyntheticDefect> {
    vec![
        // Small porosity at 20% along axis, center, shallow
        SyntheticDefect {
            axis_pos: 0.15, perp_pos: 0.0, depth: 1000,
            radius: 0.015, shape: DefectShape::Round,
        },
        // Medium porosity at 30%, slightly off-center
        SyntheticDefect {
            axis_pos: 0.30, perp_pos: 0.2, depth: 2000,
            radius: 0.025, shape: DefectShape::Round,
        },
        // Deep porosity at 45%, center
        SyntheticDefect {
            axis_pos: 0.45, perp_pos: 0.0, depth: 4000,
            radius: 0.02, shape: DefectShape::Round,
        },
        // Linear crack at 60%
        SyntheticDefect {
            axis_pos: 0.60, perp_pos: 0.0, depth: 3000,
            radius: 0.04, shape: DefectShape::LinearAxial,
        },
        // Lack of fusion at 75%
        SyntheticDefect {
            axis_pos: 0.75, perp_pos: 0.0, depth: 2500,
            radius: 0.05, shape: DefectShape::LinearPerp,
        },
        // Edge undercut at 90%
        SyntheticDefect {
            axis_pos: 0.90, perp_pos: 0.85, depth: 1500,
            radius: 0.02, shape: DefectShape::Round,
        },
    ]
}

/// Generated synthetic weld output.
pub struct SyntheticWeld {
    /// Height field in Permyriad (row-major, width × height)
    pub height_field: Vec<i32>,
    /// Width of the grid
    pub width: u32,
    /// Height of the grid
    pub height: u32,
    /// Expected gradient field (dx, dy) per pixel — Permyriad scaled
    pub gradient_x: Vec<i32>,
    pub gradient_y: Vec<i32>,
    /// Defect mask: true where a known defect exists
    pub defect_mask: Vec<bool>,
    /// Depth at each defect pixel (0 where no defect)
    pub defect_depth: Vec<u32>,
}

/// Generate a synthetic weld with known geometry and defects.
pub fn generate_synthetic(config: &SyntheticConfig) -> SyntheticWeld {
    let w = config.width as usize;
    let h = config.height as usize;
    let total = w * h;

    let mut height_field: Vec<i32> = vec![config.base_height as i32; total];
    let mut defect_mask: Vec<bool> = vec![false; total];
    let mut defect_depth: Vec<u32> = vec![0; total];

    // Determine axis and perpendicular dimensions
    let (axis_len, perp_len) = if config.vertical { (h, w) } else { (w, h) };
    let bead_center = perp_len as f32 / 2.0;
    let bead_sigma = (perp_len as f32 * config.bead_width_frac) / 4.0; // 2-sigma = bead width

    // Generate bead profile
    for axis_idx in 0..axis_len {
        let axis_frac = axis_idx as f32 / axis_len as f32;
        // Ripple along axis
        let ripple = (axis_frac * config.ripple_count as f32 * std::f32::consts::TAU).sin();
        let ripple_val = (ripple * config.ripple_amplitude as f32) as i32;

        for perp_idx in 0..perp_len {
            let perp_dist = (perp_idx as f32 - bead_center) / bead_sigma;
            // Gaussian bead cross-section
            let bead_val = (config.bead_height as f32 * (-0.5 * perp_dist * perp_dist).exp()) as i32;

            let (x, y) = if config.vertical {
                (perp_idx, axis_idx)
            } else {
                (axis_idx, perp_idx)
            };

            let idx = y * w + x;
            height_field[idx] = config.base_height as i32 + bead_val + ripple_val;
        }
    }

    // Embed defects
    for defect in &config.defects {
        let axis_pixel = (defect.axis_pos * axis_len as f32) as usize;
        let perp_pixel = (bead_center + defect.perp_pos * bead_sigma * 2.0) as usize;

        let radius_axis = match defect.shape {
            DefectShape::Round => (defect.radius * axis_len as f32) as usize,
            DefectShape::LinearAxial => (defect.radius * axis_len as f32 * 3.0) as usize,
            DefectShape::LinearPerp => (defect.radius * axis_len as f32) as usize,
        };
        let radius_perp = match defect.shape {
            DefectShape::Round => (defect.radius * perp_len as f32) as usize,
            DefectShape::LinearAxial => (defect.radius * perp_len as f32 * 0.3) as usize,
            DefectShape::LinearPerp => (defect.radius * perp_len as f32 * 3.0) as usize,
        };

        let r_axis = radius_axis.max(1);
        let r_perp = radius_perp.max(1);

        for da in 0..=(r_axis * 2) {
            for dp in 0..=(r_perp * 2) {
                let a = (axis_pixel + da).saturating_sub(r_axis);
                let p = (perp_pixel + dp).saturating_sub(r_perp);

                if a >= axis_len || p >= perp_len { continue; }

                // Elliptical distance
                let na = (a as f32 - axis_pixel as f32) / r_axis.max(1) as f32;
                let np = (p as f32 - perp_pixel as f32) / r_perp.max(1) as f32;
                let dist_sq = na * na + np * np;

                if dist_sq <= 1.0 {
                    // Smooth falloff within the defect
                    let falloff = 1.0 - dist_sq;
                    let depth_here = (defect.depth as f32 * falloff) as i32;

                    let (x, y) = if config.vertical { (p, a) } else { (a, p) };
                    let idx = y * w + x;
                    if idx < total {
                        height_field[idx] -= depth_here;
                        defect_mask[idx] = true;
                        defect_depth[idx] = defect_depth[idx].max(depth_here as u32);
                    }
                }
            }
        }
    }

    // Compute gradient field (reverse Poisson: height → gradients)
    let mut gradient_x: Vec<i32> = vec![0; total];
    let mut gradient_y: Vec<i32> = vec![0; total];

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            // Central difference
            gradient_x[idx] = (height_field[idx + 1] - height_field[idx - 1]) / 2;
            gradient_y[idx] = (height_field[(y + 1) * w + x] - height_field[(y - 1) * w + x]) / 2;
        }
    }

    SyntheticWeld {
        height_field,
        width: config.width,
        height: config.height,
        gradient_x,
        gradient_y,
        defect_mask,
        defect_depth,
    }
}

/// Render the synthetic weld as a grayscale image (for display/comparison).
/// Maps height field to [0, 255] pixel values.
pub fn render_to_pixels(weld: &SyntheticWeld) -> Vec<u8> {
    let min_h = weld.height_field.iter().copied().min().unwrap_or(0);
    let max_h = weld.height_field.iter().copied().max().unwrap_or(10000);
    let range = (max_h - min_h).max(1) as f32;

    weld.height_field.iter().flat_map(|&h| {
        let normalized = ((h - min_h) as f32 / range * 255.0) as u8;
        vec![normalized, normalized, normalized, 255u8]
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_generation() {
        let config = SyntheticConfig::default();
        let weld = generate_synthetic(&config);
        assert_eq!(weld.height_field.len(), (config.width * config.height) as usize);
        assert_eq!(weld.gradient_x.len(), weld.height_field.len());
        assert_eq!(weld.gradient_y.len(), weld.height_field.len());
        // Should have some defect pixels
        assert!(weld.defect_mask.iter().any(|&m| m));
    }

    #[test]
    fn test_bead_is_raised() {
        let config = SyntheticConfig {
            defects: vec![], // No defects, just bead
            ..Default::default()
        };
        let weld = generate_synthetic(&config);
        let w = config.width as usize;
        let h = config.height as usize;
        // Center of bead should be higher than edges
        let center_idx = (h / 2) * w + (w / 2);
        let edge_idx = (h / 2) * w + 0;
        assert!(weld.height_field[center_idx] > weld.height_field[edge_idx]);
    }

    #[test]
    fn test_defects_lower_surface() {
        let config = SyntheticConfig::default();
        let weld = generate_synthetic(&config);
        // Defect pixels should have non-zero depth
        let defect_count = weld.defect_depth.iter().filter(|&&d| d > 0).count();
        assert!(defect_count > 0);
    }

    #[test]
    fn test_gradients_nonzero_at_bead_edge() {
        let config = SyntheticConfig {
            defects: vec![],
            ..Default::default()
        };
        let weld = generate_synthetic(&config);
        // Gradient should be non-zero at bead edges (slope)
        let w = config.width as usize;
        let mid_y = config.height as usize / 2;
        let quarter_x = config.width as usize / 4; // On the bead slope
        let idx = mid_y * w + quarter_x;
        // At least one gradient component should be non-zero near bead edge
        assert!(weld.gradient_x[idx] != 0 || weld.gradient_y[idx] != 0);
    }

    #[test]
    fn test_render_produces_rgba() {
        let config = SyntheticConfig { width: 10, height: 10, defects: vec![], ..Default::default() };
        let weld = generate_synthetic(&config);
        let pixels = render_to_pixels(&weld);
        assert_eq!(pixels.len(), 10 * 10 * 4);
    }
}
