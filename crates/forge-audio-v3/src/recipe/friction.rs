//! Recipe 4: Dynamic Friction & Fluids — Noise → state-variable filter.
//!
//! Velocity → cutoff, friction → Q. Optional fast LFO for granular rattling.

use super::primitives::{noise_white, noise_pink, PinkNoiseState, SeedRng, BiquadState, filter_resonant_lp, envelope_ar};
use super::RecipeParams;

/// Friction synthesis: scrapes, engines, turbulence.
pub fn recipe_friction(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let base_cutoff = params.ring_frequency_hz.clamp(100.0, 12000.0);
    let q = params.filter_q.clamp(0.5, 20.0);
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);
    let harmonic = params.harmonic_content;

    let mut rng = SeedRng::new(params.seed);
    let mut pink_state = PinkNoiseState::new();
    let mut filter_state = BiquadState::new();

    let fog_cutoff_scale = 1.0 - params.fog_cutoff_mod * 0.6;
    let effective_cutoff = (base_cutoff * fog_cutoff_scale).clamp(20.0, 20000.0);

    let lfo_rate = if harmonic > 0.5 { 15.0 + harmonic * 15.0 } else { 0.0 };
    let mut lfo_phase = 0.0f32;

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let env = envelope_ar(t, 0.003, decay);

        let noise = if harmonic > 0.3 {
            noise_white(&mut rng) * harmonic + noise_pink(&mut pink_state, &mut rng) * (1.0 - harmonic)
        } else {
            noise_pink(&mut pink_state, &mut rng)
        };

        let cutoff_sweep = effective_cutoff * (1.0 - t / (decay * 2.0)).max(0.3);

        let lfo_mod = if lfo_rate > 0.0 {
            let lfo = (lfo_phase * 2.0 * std::f32::consts::PI).sin();
            lfo_phase += lfo_rate * inv_sr;
            lfo_phase -= lfo_phase.floor();
            0.5 + 0.5 * lfo
        } else {
            1.0
        };

        let filtered = filter_resonant_lp(&mut filter_state, noise, cutoff_sweep, q, sr);
        let mut out = filtered * env * volume * lfo_mod;

        if params.distortion_amount > 0.01 {
            let drive = 1.0 + params.distortion_amount * 5.0;
            out = (out * drive).tanh();
        }

        *sample = out;
    }
}
