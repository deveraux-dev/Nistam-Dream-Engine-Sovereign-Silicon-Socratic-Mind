//! VibeMatrix post-process — sound-reactive visual modulation.
//!
//! Reads integer VibeUniforms, converts at the GPU boundary, applies:
//! - Chromatic aberration (RGB channel UV split)
//! - Artifact glow (additive bloom weight)
//! - Distortion (UV displacement)
//!
//! All math is CPU-testable. Entry points in `entry_points.rs`.

#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use crate::gpu_types::VibeUniforms;

/// Apply chromatic aberration: offset UV per-channel based on distance from center.
/// Returns (r_offset, g_offset, b_offset) UV deltas.
#[inline]
pub fn chromatic_offset(uv_x: f32, uv_y: f32, strength: f32) -> [(f32, f32); 3] {
    let cx = uv_x - 0.5;
    let cy = uv_y - 0.5;
    let dist = (cx * cx + cy * cy).sqrt();
    let offset = dist * strength;
    let dx = cx * offset;
    let dy = cy * offset;
    [
        (-dx, -dy),         // R channel: inward
        (0.0, 0.0),        // G channel: no shift
        (dx, dy),           // B channel: outward
    ]
}

/// Compute additive bloom weight from artifact_glow.
/// Returns a multiplier [0.0, 1.0] for the bloom composite pass.
#[inline]
pub fn bloom_weight(artifact_glow: u32) -> f32 {
    (artifact_glow as f32 / 10000.0).min(1.0)
}

/// Compute UV distortion magnitude from distortion_level.
/// Returns displacement scale for Perlin/noise UV offset.
#[inline]
pub fn distortion_magnitude(distortion_level: u32) -> f32 {
    distortion_level as f32 / 10000.0 * 0.05 // max 5% UV displacement
}

/// Full post-process pass: given a base colour and VibeUniforms, compute
/// the modulated output colour (before bloom composite).
/// `uv`: fragment UV (0,0)→(1,1).
/// Returns (r, g, b, bloom_add).
#[inline]
pub fn vibe_post_process(
    base_r: f32, base_g: f32, base_b: f32,
    uv_x: f32, uv_y: f32,
    vibe: &VibeUniforms,
) -> (f32, f32, f32, f32) {
    let chroma_str = vibe.chromatic_aberration as f32 / 10000.0;
    let glow = bloom_weight(vibe.artifact_glow);

    // Chromatic: in a real shader this samples the texture at offset UVs.
    // On CPU parity test we simulate as colour shift proportional to offset.
    let offsets = chromatic_offset(uv_x, uv_y, chroma_str);
    let r = base_r * (1.0 + offsets[0].0.abs() * 0.5);
    let g = base_g;
    let b = base_b * (1.0 + offsets[2].0.abs() * 0.5);

    // Additive glow boost (drives into bloom threshold)
    let r_out = (r + r * glow * 0.3).min(2.0);
    let g_out = (g + g * glow * 0.3).min(2.0);
    let b_out = (b + b * glow * 0.3).min(2.0);

    (r_out, g_out, b_out, glow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_vibe() -> VibeUniforms {
        VibeUniforms {
            combo_heat: 0, resonance_hz: 0, rain_intensity: 0,
            chromatic_aberration: 0, artifact_glow: 0,
            particle_density: 0, distortion_level: 0, _pad: 0,
        }
    }

    #[test]
    fn zero_energy_is_passthrough() {
        let (r, g, b, glow) = vibe_post_process(0.5, 0.5, 0.5, 0.5, 0.5, &zero_vibe());
        assert!((r - 0.5).abs() < 0.001);
        assert!((g - 0.5).abs() < 0.001);
        assert!((b - 0.5).abs() < 0.001);
        assert!(glow < 0.001);
    }

    #[test]
    fn max_glow_boosts() {
        let mut v = zero_vibe();
        v.artifact_glow = 10000;
        let (r, _, _, glow) = vibe_post_process(0.5, 0.5, 0.5, 0.5, 0.5, &v);
        assert!((glow - 1.0).abs() < 0.001);
        assert!(r > 0.5); // boosted
    }

    #[test]
    fn chromatic_at_center_is_zero() {
        let offsets = chromatic_offset(0.5, 0.5, 1.0);
        assert!(offsets[0].0.abs() < 0.001);
        assert!(offsets[2].0.abs() < 0.001);
    }

    #[test]
    fn chromatic_at_edge_is_nonzero() {
        let offsets = chromatic_offset(1.0, 1.0, 1.0);
        assert!(offsets[2].0.abs() > 0.01);
    }
}
