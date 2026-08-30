//! Vignette — energy-responsive screen darkening (MEGA-05 spec).
//!
//! Low energy: wide open (radius 0.7). Peak energy: tunnel vision (radius 0.3).
//! Sub-bass buildup tightens gradually; drop snaps open.

#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

/// Smoothstep helper (cubic Hermite, no_std compatible).
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Compute vignette multiplier for a fragment at UV (0,0)→(1,1).
/// Returns 0.0 (black) to 1.0 (no darkening).
///
/// `combo_heat`: from VibeUniforms (permyriad 0–10000).
/// `energy_response`: how much heat tightens the vignette (permyriad, default 4000).
/// `base_radius`: where darkening starts (permyriad, default 7000 = 0.7).
/// `softness`: falloff width (permyriad, default 3000 = 0.3).
#[inline]
pub fn vignette(
    uv_x: f32,
    uv_y: f32,
    combo_heat: u32,
    energy_response: u32,
    base_radius: u32,
    softness: u32,
) -> f32 {
    // All integer math until the final computation
    // effective_radius = base_radius - (combo_heat * energy_response / 10000)
    let tighten = (combo_heat / 10) * (energy_response / 1000);
    let effective_radius_pmy = if tighten >= base_radius { 0 } else { base_radius - tighten };

    // Convert to float at the boundary
    let radius = effective_radius_pmy as f32 / 10000.0;
    let soft = softness as f32 / 10000.0;

    // Distance from center (0,0 = center in UV-centered coords)
    let cx = uv_x - 0.5;
    let cy = uv_y - 0.5;
    let dist = (cx * cx + cy * cy).sqrt() * 2.0; // normalized: corner = ~1.414

    // Smoothstep darkening from radius to radius+softness
    1.0 - smoothstep(radius, radius + soft, dist)
}

/// Convenience: default vignette with standard parameters.
#[inline]
pub fn vignette_default(uv_x: f32, uv_y: f32, combo_heat: u32) -> f32 {
    vignette(uv_x, uv_y, combo_heat, 4000, 7000, 3000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_is_full() {
        let v = vignette_default(0.5, 0.5, 0);
        assert!((v - 1.0).abs() < 0.001, "center={}", v);
    }

    #[test]
    fn corner_is_dark() {
        let v = vignette_default(0.0, 0.0, 0);
        assert!(v < 0.3, "corner={}", v);
    }

    #[test]
    fn high_energy_tightens() {
        // Use a point near the edge where vignette radius difference is visible
        let open = vignette_default(0.1, 0.5, 0);
        let tight = vignette_default(0.1, 0.5, 10000);
        assert!(tight < open, "tight={} should < open={}", tight, open);
    }

    #[test]
    fn zero_energy_unchanged() {
        let v = vignette_default(0.3, 0.3, 0);
        let v2 = vignette(0.3, 0.3, 0, 4000, 7000, 3000);
        assert!((v - v2).abs() < 0.001);
    }

    #[test]
    fn saturating_sub_no_panic() {
        // Max energy + max response should not underflow
        let v = vignette(0.5, 0.5, 10000, 10000, 7000, 3000);
        assert!(v >= 0.0 && v <= 1.0);
    }
}
