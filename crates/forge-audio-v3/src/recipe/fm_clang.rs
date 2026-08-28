//! Recipe 1: FM Metal/Bell Clang — Sine carrier FM'd by Sine modulator.
//!
//! Inharmonic ratios for metallic clangs, integer ratios for bell tones.

use super::primitives::{osc_sine, envelope_ar, SeedRng};
use super::RecipeParams;

/// FM Clang synthesis: metallic impact sounds.
pub fn recipe_fm_clang(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let freq = params.ring_frequency_hz.clamp(20.0, 20000.0);
    let ratio = params.fm_ratio;
    let mod_freq = freq / ratio;
    let mod_index = params.harmonic_content * 8.0;
    let attack = (1.0 - params.attack_sharpness) * 0.002 + 0.001;
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);
    let detune = params.aberration_detune * 0.02;

    let mut rng = SeedRng::new(params.seed);
    let pitch_jitter = 1.0 + rng.next_f32() * detune;

    let carrier_freq = freq * pitch_jitter;
    let mut carrier_phase = 0.0f32;
    let mut mod_phase = 0.0f32;

    let fog_damp = 1.0 - params.fog_cutoff_mod * 0.5;

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let mod_env = envelope_ar(t, attack, decay);
        let mod_signal = osc_sine(mod_phase) * mod_index * mod_env;
        let carrier_signal = osc_sine(carrier_phase + mod_signal / (2.0 * std::f32::consts::PI));
        let carrier_env = envelope_ar(t, attack * 0.5, decay * 1.2);

        let mut out = carrier_signal * carrier_env * volume * fog_damp;
        if params.distortion_amount > 0.01 {
            out = soft_clip(out, params.distortion_amount);
        }

        *sample = out;

        carrier_phase += carrier_freq * inv_sr;
        carrier_phase -= carrier_phase.floor();
        mod_phase += mod_freq * inv_sr;
        mod_phase -= mod_phase.floor();
    }
}

#[inline]
fn soft_clip(x: f32, amount: f32) -> f32 {
    let drive = 1.0 + amount * 4.0;
    (x * drive).tanh()
}
