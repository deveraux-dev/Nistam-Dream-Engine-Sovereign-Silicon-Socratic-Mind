//! Spectral ducking — frequency-domain auto-mix with phase cancellation mitigation.

use num_complex::Complex32;
use crate::alchemy::stft;

/// Apply spectral ducking to Track B based on Track A's spectral dominance.
/// `alpha` controls maximum ducking depth (0.0 = no ducking, 1.0 = full silence possible).
/// Typical value: 0.6.
pub fn spectral_duck(
    track_a: &[f32],
    track_b: &[f32],
    sample_rate: u32,
    alpha: f32,
) -> Vec<f32> {
    let frame_size = 2048;
    let hop = 512;
    let n_bins = frame_size / 2 + 1;
    let epsilon = 1e-10f32;
    let _ = sample_rate; // reserved for future frequency-dependent ducking

    let len = track_a.len().max(track_b.len());
    let mut a = track_a.to_vec();
    let mut b = track_b.to_vec();
    a.resize(len, 0.0);
    b.resize(len, 0.0);

    let frames_a = stft::stft_forward(&a, frame_size, hop);
    let frames_b = stft::stft_forward(&b, frame_size, hop);
    let n_frames = frames_a.len().min(frames_b.len());

    let mut ducked_frames = Vec::with_capacity(n_frames);

    for t in 0..n_frames {
        let mut out = frames_b[t].clone();
        for k in 0..n_bins {
            let xa = frames_a[t][k];
            let xb = frames_b[t][k];

            let power_a = xa.norm_sqr();
            let power_b = xb.norm_sqr();

            // Cross-power spectral density
            let sxy = xa * xb.conj();
            let sxy_mag = sxy.norm();

            // Normalized cross-correlation weight
            let denom_corr = (power_a * power_b).sqrt() + epsilon;
            let w_xy = (sxy_mag / denom_corr).min(1.0);

            // Ducking gain
            let denom = power_a + power_b + epsilon;
            let gain = 1.0 - alpha * (power_a / denom) * w_xy;
            let gain = gain.clamp(0.05, 1.0);

            // Phase cancellation mitigation
            if w_xy > 0.5 && sxy_mag > epsilon {
                let phase_diff = sxy.arg();
                if phase_diff.abs() > 2.0 {
                    let corrected = Complex32::from_polar(xb.norm() * gain, xb.arg() - phase_diff);
                    out[k] = corrected;
                    continue;
                }
            }

            out[k] = xb * gain;
        }

        // realfft requires DC and Nyquist bins to be purely real
        out[0] = Complex32::new(out[0].re, 0.0);
        if n_bins > 1 {
            let last = n_bins - 1;
            out[last] = Complex32::new(out[last].re, 0.0);
        }

        ducked_frames.push(out);
    }

    stft::stft_inverse(&ducked_frames, frame_size, hop, track_b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ducking_reduces_collision() {
        let sr = 44100;
        let n = 22050;
        let track_a: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let track_b: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let ducked_b = spectral_duck(&track_a, &track_b, sr as u32, 0.6);
        let orig_e: f32 = track_b.iter().map(|s| s * s).sum();
        let ducked_e: f32 = ducked_b.iter().map(|s| s * s).sum();
        assert!(ducked_e < orig_e * 0.9,
            "Ducked energy {} should be < 90% of original {}", ducked_e, orig_e);
    }

    #[test]
    fn test_ducking_preserves_non_overlapping() {
        let sr = 44100;
        let n = 22050;
        let track_a: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let track_b: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin())
            .collect();
        let ducked_b = spectral_duck(&track_a, &track_b, sr as u32, 0.6);
        let orig_e: f32 = track_b.iter().map(|s| s * s).sum();
        let ducked_e: f32 = ducked_b.iter().map(|s| s * s).sum();
        let ratio = ducked_e / orig_e;
        assert!(ratio > 0.7,
            "Non-overlapping ducked energy ratio {} should be > 0.7", ratio);
    }

    #[test]
    fn test_ducking_alpha_limits_depth() {
        let sr = 44100;
        let n = 22050;
        let track_a: Vec<f32> = (0..n)
            .map(|i| 0.8 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let track_b = track_a.clone();
        let ducked_0 = spectral_duck(&track_a, &track_b, sr as u32, 0.0);
        let e_0: f32 = ducked_0.iter().map(|s| s * s).sum();
        let e_orig: f32 = track_b.iter().map(|s| s * s).sum();
        let ratio = e_0 / e_orig;
        assert!(ratio > 0.9, "Alpha=0.0 ratio {} should be ~1.0", ratio);
    }
}
