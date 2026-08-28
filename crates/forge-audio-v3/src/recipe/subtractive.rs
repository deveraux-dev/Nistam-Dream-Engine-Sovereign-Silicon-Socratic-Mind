//! Recipe 5: Squelch / Brass Impact — Saw → resonant lowpass.
//!
//! Envelope → VCA + cutoff. harmonic_content → resonance.

use super::primitives::{osc_saw, envelope_ar, SeedRng, BiquadState, filter_resonant_lp};
use super::RecipeParams;

/// Subtractive synthesis: stabs, brass, shadow impacts.
pub fn recipe_subtractive(params: &RecipeParams, buf: &mut [f32]) {
    let sr = params.sample_rate as f32;
    let inv_sr = 1.0 / sr;
    let freq = params.ring_frequency_hz.clamp(40.0, 8000.0);
    let q = (params.harmonic_content * 15.0 + 0.5).clamp(0.5, 20.0);
    let attack = (1.0 - params.attack_sharpness) * 0.01 + 0.001;
    let decay = params.decay_secs.max(0.01);
    let volume = (10.0f32).powf(params.intensity_db / 20.0).clamp(0.0, 1.0);

    let mut rng = SeedRng::new(params.seed);
    let detune = params.aberration_detune * 0.01;
    let pitch_jitter = 1.0 + rng.next_f32() * detune;

    let mut phase = 0.0f32;
    let mut filter_state = BiquadState::new();

    let fog_cutoff_scale = 1.0 - params.fog_cutoff_mod * 0.5;
    let base_cutoff = (freq * 4.0 * fog_cutoff_scale).clamp(20.0, 16000.0);

    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 * inv_sr;
        let env = envelope_ar(t, attack, decay);

        let cutoff = (base_cutoff * env + 200.0).clamp(20.0, 20000.0);
        let saw = osc_saw(phase);
        let filtered = filter_resonant_lp(&mut filter_state, saw, cutoff, q, sr);

        let mut out = filtered * env * volume;
        if params.distortion_amount > 0.01 {
            let drive = 1.0 + params.distortion_amount * 4.0;
            out = (out * drive).tanh();
        }

        *sample = out;

        phase += freq * pitch_jitter * inv_sr;
        phase -= phase.floor();
    }
}
