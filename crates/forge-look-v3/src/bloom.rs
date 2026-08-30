//! Bloom — separable 5-tap gaussian blur (energy-responsive kernel width).
//!
//! Two passes: horizontal then vertical. Kernel width modulated by `artifact_glow`
//! from VibeUniforms. Threshold pass extracts pixels above luminance 1.0.
//! MEGA-05 spec compliant.

#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

/// Compute 5-tap gaussian weights for a given radius.
/// Returns [center, tap1, tap2] weights that sum to ~1.0.
/// `radius`: 0.0 = passthrough (center=1.0), 1.0 = maximum blur.
#[inline]
pub fn gaussian_weights(radius: f32) -> [f32; 3] {
    if radius < 0.001 {
        return [1.0, 0.0, 0.0]; // passthrough
    }
    let sigma = radius.max(0.1);
    let s2 = sigma * sigma;
    // Gaussian: exp(-x²/(2σ²))
    let w0 = 1.0; // center
    let w1 = (-1.0 / (2.0 * s2)).exp();
    let w2 = (-4.0 / (2.0 * s2)).exp();
    let sum = w0 + 2.0 * w1 + 2.0 * w2;
    [w0 / sum, w1 / sum, w2 / sum]
}

/// Compute texel offsets for the 5-tap kernel.
/// Returns offsets [0, offset1, offset2] in texels.
/// `texel_size`: 1.0 / texture_dimension.
#[inline]
pub fn gaussian_offsets(texel_size: f32) -> [f32; 3] {
    [0.0, texel_size * 1.0, texel_size * 2.0]
}

/// Luminance threshold — returns how much of a pixel contributes to bloom.
/// Pixels below threshold return 0. Linear ramp above threshold to `max`.
#[inline]
pub fn bloom_threshold(r: f32, g: f32, b: f32, threshold: f32) -> f32 {
    let lum = r * 0.2126 + g * 0.7152 + b * 0.0722;
    if lum <= threshold {
        0.0
    } else {
        ((lum - threshold) / (1.0 - threshold + 0.001)).min(1.0)
    }
}

/// Apply 5-tap 1D gaussian blur to a row of samples.
/// `samples`: [left2, left1, center, right1, right2].
/// `weights`: from `gaussian_weights()`.
#[inline]
pub fn apply_1d(samples: [f32; 5], weights: &[f32; 3]) -> f32 {
    samples[2] * weights[0]
        + (samples[1] + samples[3]) * weights[1]
        + (samples[0] + samples[4]) * weights[2]
}

/// Energy-responsive blur radius. Maps artifact_glow (permyriad) to blur radius.
/// 0 = no blur, 10000 = max blur (radius=1.0).
#[inline]
pub fn energy_to_radius(artifact_glow: u32) -> f32 {
    (artifact_glow as f32 / 10000.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_one() {
        let w = gaussian_weights(0.5);
        let sum = w[0] + 2.0 * w[1] + 2.0 * w[2];
        assert!((sum - 1.0).abs() < 0.001, "sum={}", sum);
    }

    #[test]
    fn zero_radius_passthrough() {
        let w = gaussian_weights(0.0);
        assert!((w[0] - 1.0).abs() < 0.001);
        assert!(w[1] < 0.001);
        assert!(w[2] < 0.001);
    }

    #[test]
    fn identity_input() {
        let w = gaussian_weights(0.5);
        let result = apply_1d([0.7, 0.7, 0.7, 0.7, 0.7], &w);
        assert!((result - 0.7).abs() < 0.001);
    }

    #[test]
    fn threshold_below_is_zero() {
        assert!(bloom_threshold(0.3, 0.3, 0.3, 1.0) < 0.001);
    }

    #[test]
    fn threshold_above_is_nonzero() {
        assert!(bloom_threshold(1.5, 1.5, 1.5, 1.0) > 0.5);
    }

    #[test]
    fn energy_maps_linearly() {
        assert!((energy_to_radius(0) - 0.0).abs() < 0.001);
        assert!((energy_to_radius(5000) - 0.5).abs() < 0.001);
        assert!((energy_to_radius(10000) - 1.0).abs() < 0.001);
    }
}
