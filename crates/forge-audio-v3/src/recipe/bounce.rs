//! Recipe 3: Accelerating Bounce — Ramp-up LFO modulates ramp-down LFO rate.
//!
//! Routes to VCA or filter cutoff. attack_sharpness → LFO speed.

use super::primitives::{osc_sine, envelope_ar, SeedRng, BiquadState, filter_lowpass};
use super::RecipeParams;

/// Bounce synthesis: accelerating physics sounds.
pub fn recipe_bounce(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let base_freq = params.ring_frequency_hz.clamp(40.0, 2000.0);
    let lfo_speed = params.lfo_rate_hz.clamp(1.0, 30.0);
    let attack_speed = params.attack_sharpness * 20.0 + 2.0;
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);

    let mut rng = SeedRng::new(params.seed);
    let detune = params.aberration_detune * 0.015;
    let pitch_jitter = 1.0 + rng.next_f32() * detune;

    let mut osc_phase = 0.0f32;
    let mut lfo_phase = 0.0f32;
    let mut filter_state = BiquadState::new();

    let fog_damp = 1.0 - params.fog_cutoff_mod * 0.4;

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let env = envelope_ar(t, 0.002, decay);

        let ramp_up = (t * attack_speed).min(1.0);
        let current_lfo_rate = lfo_speed * (1.0 + ramp_up * 3.0);
        let lfo_val = (osc_sine(lfo_phase) + 1.0) * 0.5;

        let osc = osc_sine(osc_phase) * env * lfo_val * volume * fog_damp;

        let cutoff = (base_freq * (1.0 + lfo_val * 2.0)).clamp(20.0, 16000.0);
        let filtered = filter_lowpass(&mut filter_state, osc, cutoff, params.filter_q.clamp(0.5, 10.0), sr);

        let mut out = filtered;
        if params.distortion_amount > 0.01 {
            let drive = 1.0 + params.distortion_amount * 3.0;
            out = (out * drive).tanh();
        }

        *sample = out;

        osc_phase += base_freq * pitch_jitter * inv_sr;
        osc_phase -= osc_phase.floor();
        lfo_phase += current_lfo_rate * inv_sr;
        lfo_phase -= lfo_phase.floor();
    }
}
