//! Recipe 2: FM Bass / Sub-Growl — Low-frequency FM with self-feedback.
//!
//! Concentrates energy below 200Hz. reverb_amount → feedback coefficient.

use super::primitives::{osc_sine, envelope_ar, SeedRng};
use super::RecipeParams;

/// FM Bass synthesis: sub-bass growl for Void materials.
pub fn recipe_fm_bass(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let freq = params.ring_frequency_hz.clamp(20.0, 200.0);
    let ratio = if params.fm_ratio > 0.0 { params.fm_ratio.clamp(0.5, 1.5) } else { 1.0 };
    let mod_freq = freq * ratio;
    let feedback = params.fm_feedback.clamp(0.0, 0.95);
    let mod_index = params.harmonic_content * 4.0;
    let attack = 0.005;
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);

    let mut rng = SeedRng::new(params.seed);
    let detune = params.aberration_detune * 0.01;
    let pitch_jitter = 1.0 + rng.next_f32() * detune;

    let carrier_freq = freq * pitch_jitter;
    let mut carrier_phase = 0.0f32;
    let mut mod_phase = 0.0f32;
    let mut prev_mod_out = 0.0f32;

    let fog_damp = 1.0 - params.fog_cutoff_mod * 0.3;

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let env = envelope_ar(t, attack, decay);

        let fb_input = prev_mod_out * feedback;
        let mod_signal = osc_sine(mod_phase + fb_input / (2.0 * std::f32::consts::PI));
        prev_mod_out = mod_signal;

        let carrier = osc_sine(carrier_phase + mod_signal * mod_index / (2.0 * std::f32::consts::PI));

        let mut out = carrier * env * volume * fog_damp;
        if params.distortion_amount > 0.01 {
            let drive = 1.0 + params.distortion_amount * 6.0;
            out = (out * drive).tanh();
        }

        *sample = out;

        carrier_phase += carrier_freq * inv_sr;
        carrier_phase -= carrier_phase.floor();
        mod_phase += mod_freq * inv_sr;
        mod_phase -= mod_phase.floor();
    }
}
