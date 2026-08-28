//! Recipe 6: Harmonic Organ / Pad — Parallel sines at integer multiples.
//!
//! Configurable partial count (max 16), per-partial amplitude rolloff.

use super::primitives::{osc_sine, envelope_ar, SeedRng};
use super::RecipeParams;

/// Additive synthesis: organ tones, ambient drones, pads.
pub fn recipe_additive(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let base_freq = params.ring_frequency_hz.clamp(20.0, 4000.0);
    let partial_count = (params.partial_count as usize).clamp(1, 16);
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);

    let mut rng = SeedRng::new(params.seed);
    let detune_amount = params.aberration_detune * 0.005;

    let mut detunes = [0.0f32; 16];
    for i in 0..partial_count {
        detunes[i] = 1.0 + rng.next_f32() * detune_amount;
    }

    let fog_damp = 1.0 - params.fog_cutoff_mod * 0.4;

    let norm: f32 = (1..=partial_count).map(|k| 1.0 / k as f32).sum();
    let inv_norm = if norm > 0.0 { 1.0 / norm } else { 1.0 };

    let mut phases = [0.0f32; 16];

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let env = envelope_ar(t, 0.005, decay);

        let mut sum = 0.0f32;
        for k in 0..partial_count {
            let partial_num = (k + 1) as f32;
            let amplitude = 1.0 / partial_num;
            sum += osc_sine(phases[k]) * amplitude;

            let partial_freq = base_freq * partial_num * detunes[k];
            phases[k] += partial_freq * inv_sr;
            phases[k] -= phases[k].floor();
        }

        let mut out = sum * inv_norm * env * volume * fog_damp;
        if params.distortion_amount > 0.01 {
            let drive = 1.0 + params.distortion_amount * 3.0;
            out = (out * drive).tanh();
        }

        *sample = out;
    }
}
