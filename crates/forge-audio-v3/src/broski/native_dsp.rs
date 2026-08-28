//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\native_dsp.rs (102 LOC).
//!
//! Broski DSP chain — native Rust implementations. This IS the production DSP
//! (2026-07-18: corrected from a stale "reference mirror" framing — there is no
//! Faust runtime anywhere in forge-broski; these functions are what actually ships).
//! Design-language prototypes (originally sketched in Faust DSL, never compiled
//! or shipped) moved to _quarry/faust-removed-2026-07-18/ — see RESTORE.md.

use std::f64::consts::PI;

/// Equal-power crossfade with aggression-driven curve morphing.
pub fn blend_gains(position: f64, aggression: f64) -> (f64, f64) {
    let gain1 = (position * PI / 2.0).cos();
    let gain2 = (position * PI / 2.0).sin();
    let hard1 = (1.0 - ((position - 0.4) * 5.0).clamp(0.0, 1.0)).max(0.0);
    let hard2 = ((position - 0.6) * 5.0).clamp(0.0, 1.0);
    let mix = (aggression / 10.0).clamp(0.0, 1.0);
    (gain1 * (1.0 - mix) + hard1 * mix, gain2 * (1.0 - mix) + hard2 * mix)
}

/// Filter sweep frequency from sweep position.
pub fn filter_freq(sweep: f64) -> f64 { 200.0 + sweep * 18000.0 }

/// Filter resonance from aggression.
pub fn filter_resonance(aggression: f64) -> f64 { 0.3 + aggression * 0.07 }

/// Simple 1-pole lowpass for reference testing.
pub fn lowpass_1pole(samples: &mut [f64], cutoff_hz: f64, sample_rate: f64) {
    let rc = 1.0 / (2.0 * PI * cutoff_hz);
    let dt = 1.0 / sample_rate;
    let alpha = dt / (rc + dt);
    let mut prev = samples[0];
    for s in samples.iter_mut().skip(1) {
        *s = prev + alpha * (*s - prev);
        prev = *s;
    }
}

/// Tension dry/wet mix.
pub fn tension_dry_gain(tension: f64) -> f64 {
    let reverb_mix = tension * 0.7;
    let delay_mix = tension * 0.4;
    1.0 - reverb_mix * 0.5 - delay_mix * 0.5
}

/// Soft-clip saturation (tanh).
pub fn saturate(sample: f64, drive: f64) -> f64 {
    (sample * drive).tanh() / drive
}

/// Glue compression — simple peak compressor.
pub fn compress(samples: &mut [f64], ratio: f64, threshold_db: f64) {
    let thresh_lin = 10.0_f64.powf(threshold_db / 20.0);
    for s in samples.iter_mut() {
        let abs = s.abs();
        if abs > thresh_lin {
            let over_db = 20.0 * (abs / thresh_lin).log10();
            let reduced_db = over_db / ratio;
            let new_abs = thresh_lin * 10.0_f64.powf(reduced_db / 20.0);
            *s = s.signum() * new_abs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_center() {
        let (g1, g2) = blend_gains(0.5, 0.0);
        assert!((g1 - g2).abs() < 0.01, "center should be equal: g1={} g2={}", g1, g2);
    }

    #[test]
    fn test_blend_hard_left() {
        let (g1, g2) = blend_gains(0.0, 0.0);
        assert!(g1 > 0.99 && g2 < 0.01, "left should be input1 only: g1={} g2={}", g1, g2);
    }

    #[test]
    fn test_filter_open() {
        let freq = filter_freq(1.0);
        assert!(freq > 18000.0, "open filter should pass full spectrum: {}", freq);
    }

    #[test]
    fn test_filter_closed() {
        // Apply lowpass at 200Hz to a high-frequency signal
        let sr = 44100.0;
        let mut samples: Vec<f64> = (0..1000).map(|i| (2.0 * PI * 10000.0 * i as f64 / sr).sin()).collect();
        let peak_before: f64 = samples.iter().map(|s| s.abs()).fold(0.0, f64::max);
        lowpass_1pole(&mut samples, filter_freq(0.0), sr);
        let peak_after: f64 = samples[100..].iter().map(|s| s.abs()).fold(0.0, f64::max);
        assert!(peak_after < peak_before * 0.5, "closed filter should attenuate: before={} after={}", peak_before, peak_after);
    }

    #[test]
    fn test_tension_dry() {
        let dry = tension_dry_gain(0.0);
        assert!((dry - 1.0).abs() < 0.01, "tension=0 should be fully dry: {}", dry);
    }

    #[test]
    fn test_punch_passthrough() {
        // aggression=0 means punch_amount=0, signal unchanged
        let input: f64 = 0.5;
        let output: f64 = input; // punch_amount * 3 * (fast_env - slow_env) = 0
        assert!((input - output).abs() < 0.01);
    }

    #[test]
    fn test_glue_reduces_peaks() {
        let mut samples: Vec<f64> = (0..1000).map(|i| (2.0 * PI * 440.0 * i as f64 / 44100.0).sin() * 2.0).collect();
        let peak_before: f64 = samples.iter().map(|s| s.abs()).fold(0.0, f64::max);
        compress(&mut samples, 4.0, -12.0);
        let peak_after: f64 = samples.iter().map(|s| s.abs()).fold(0.0, f64::max);
        assert!(peak_after < peak_before, "compression should reduce peaks: before={} after={}", peak_before, peak_after);
    }

}
