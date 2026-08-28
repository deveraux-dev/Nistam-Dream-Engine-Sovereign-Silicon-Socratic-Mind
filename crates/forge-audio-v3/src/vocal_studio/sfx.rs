//! Professional sound FX synthesis — layered, not toys.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//!
//! Each SFX is built from 3-4 layered synthesis components (noise, subtractive,
//! FM, additive) combined and shaped with ADSR envelopes. This is what separates
//! professional sound design from toy beeps.

use crate::dsp::AudioBuffer;

/// Sound FX type with parameters that control the synthesis.
#[derive(Debug, Clone)]
pub enum SfxKind {
    /// Physical impact: punch, slam, hit. `force` 0.0–1.0 scales sub-bass and transient.
    Impact { force: f32 },
    /// Movement whoosh: swipe, pass-by. `speed` controls sweep rate, `doppler` adds pitch shift.
    Whoosh { speed: f32, doppler: bool },
    /// Environmental ambience: wind, room, forest. `density` controls layer count.
    Ambient { density: f32 },
    /// Musical stinger/hit: cinematic accent. `key` = MIDI root, `urgency` 0.0–1.0.
    Stinger { key: u8, urgency: f32 },
    /// Tension riser or drop. `drop` = true means frequency drops instead of rises.
    RiserDrop { duration_secs: f32, drop: bool },
}

/// Synthesize a professional sound effect.
pub fn synthesize_sfx(kind: &SfxKind, duration_secs: f32, sr: u32) -> AudioBuffer {
    let n = (duration_secs * sr as f32) as usize;
    let samples = match kind {
        SfxKind::Impact { force } => synth_impact(n, sr, *force),
        SfxKind::Whoosh { speed, doppler } => synth_whoosh(n, sr, *speed, *doppler),
        SfxKind::Ambient { density } => synth_ambient(n, sr, *density),
        SfxKind::Stinger { key, urgency } => synth_stinger(n, sr, *key, *urgency),
        SfxKind::RiserDrop { duration_secs: _, drop } => synth_riser(n, sr, *drop),
    };
    AudioBuffer { samples: vec![samples], sample_rate: sr }
}

// ── Synthesis engines ────────────────────────────────────────────────────────

/// Impact: noise burst + sub-bass thump + bitcrushed transient.
fn synth_impact(n: usize, sr: u32, force: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    let sr_f = sr as f32;

    // Layer 1: Sub-bass sine thump (40-80 Hz, fast decay)
    let sub_freq = 40.0 + force * 40.0;
    for i in 0..n {
        let t = i as f32 / sr_f;
        let env = (-t * (8.0 + force * 12.0)).exp(); // fast exponential decay
        out[i] += (TAU * sub_freq * t).sin() * env * force * 0.6;
    }

    // Layer 2: Noise burst (shaped transient)
    let mut rng = 0xCAFE_BABEu32;
    let burst_len = (sr_f * 0.03) as usize; // 30ms burst
    for i in 0..burst_len.min(n) {
        rng = lcg(rng);
        let noise = rng_to_f32(rng);
        let env = 1.0 - (i as f32 / burst_len as f32);
        out[i] += noise * env * env * force * 0.4;
    }

    // Layer 3: Bitcrushed transient click
    let click_len = (sr_f * 0.005) as usize; // 5ms
    for i in 0..click_len.min(n) {
        rng = lcg(rng);
        let noise = rng_to_f32(rng);
        // Bitcrush to 4 bits
        let crushed = (noise * 8.0).round() / 8.0;
        out[i] += crushed * force * 0.3;
    }

    out
}

/// Whoosh: bandpass-swept noise + optional Doppler pitch shift.
fn synth_whoosh(n: usize, sr: u32, speed: f32, doppler: bool) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    let sr_f = sr as f32;
    let mut rng = 0xBEEF_F00Du32;

    // Bandpass center sweeps from 200 Hz → 4000 Hz → 200 Hz
    for i in 0..n {
        let t = i as f32 / n as f32; // normalized 0..1
        let sweep = (t * std::f32::consts::PI).sin(); // bell shape
        let center_freq = 200.0 + sweep * 3800.0 * speed;

        rng = lcg(rng);
        let noise = rng_to_f32(rng);

        // Simple 1-pole bandpass approximation
        let w = TAU * center_freq / sr_f;
        let filtered = noise * w.sin(); // crude but effective for SFX

        // Envelope: bell shape
        let env = sweep * (1.0 - (2.0 * t - 1.0).powi(4));

        // Doppler pitch modulation
        let doppler_mod = if doppler {
            1.0 + 0.1 * (2.0 * t - 1.0) * speed
        } else {
            1.0
        };

        out[i] = filtered * env * 0.5 * doppler_mod;
    }

    out
}

/// Ambient: layered detuned oscillators + filtered noise bed.
fn synth_ambient(n: usize, sr: u32, density: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    let sr_f = sr as f32;
    let num_layers = 2 + (density * 3.0) as usize; // 2-5 layers
    let mut rng = 0xD00D_FACEu32;

    // Layer: detuned sine drones
    let base_freq = 80.0;
    for layer in 0..num_layers {
        let detune = 1.0 + (layer as f32 * 0.007); // slight detuning per layer
        let freq = base_freq * detune;
        let phase_offset = layer as f32 * 1.7;
        for i in 0..n {
            let t = i as f32 / sr_f;
            let env = fade_in_out(i, n, (sr_f * 0.5) as usize);
            out[i] += (TAU * freq * t + phase_offset).sin() * env * 0.15 / num_layers as f32;
        }
    }

    // Noise bed (very quiet, filtered)
    let noise_gain = density * 0.05;
    let mut filtered = 0.0f32;
    let alpha = 0.001; // very low-pass
    for i in 0..n {
        rng = lcg(rng);
        let noise = rng_to_f32(rng);
        filtered += alpha * (noise - filtered); // 1-pole LPF
        let env = fade_in_out(i, n, (sr_f * 0.5) as usize);
        out[i] += filtered * noise_gain * env;
    }

    out
}

/// Stinger: chord stab with ADSR (root + minor 3rd + 5th).
fn synth_stinger(n: usize, sr: u32, key: u8, urgency: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    let sr_f = sr as f32;

    let root_hz = 440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0);
    let notes = [root_hz, root_hz * 1.1892, root_hz * 1.4983]; // root, m3, P5

    let attack = (sr_f * 0.005) as usize;
    let decay = (sr_f * (0.1 + (1.0 - urgency) * 0.3)) as usize;
    let sustain_level = 0.3 + urgency * 0.4;

    for &freq in &notes {
        for i in 0..n {
            let env = if i < attack {
                i as f32 / attack as f32
            } else if i < attack + decay {
                let d = (i - attack) as f32 / decay as f32;
                1.0 - d * (1.0 - sustain_level)
            } else {
                let release_pos = (i - attack - decay) as f32 / (n - attack - decay).max(1) as f32;
                sustain_level * (1.0 - release_pos).max(0.0)
            };

            let t = i as f32 / sr_f;
            // Saw wave (richer than sine)
            let phase = (freq * t) % 1.0;
            let saw = 2.0 * phase - 1.0;
            out[i] += saw * env * 0.2;
        }
    }

    out.iter_mut().for_each(|s| *s = s.clamp(-0.98, 0.98));
    out
}

/// Riser (or drop): frequency sweep + noise buildup.
fn synth_riser(n: usize, sr: u32, drop: bool) -> Vec<f32> {
    let mut out = vec![0.0f32; n];
    let sr_f = sr as f32;
    let mut rng = 0xFADE_1234u32;

    for i in 0..n {
        let t = i as f32 / n as f32; // 0..1
        let progress = if drop { 1.0 - t } else { t };

        // Quadratic frequency sweep: 100 Hz → 4000 Hz
        let freq = 100.0 + progress * progress * 3900.0;
        let phase = i as f32 / sr_f * TAU * freq;
        let osc = phase.sin();

        // Noise buildup
        rng = lcg(rng);
        let noise = rng_to_f32(rng) * progress * 0.3;

        // Volume envelope: builds toward climax
        let env = progress.powf(0.5);

        out[i] = (osc * 0.4 + noise) * env;
    }

    out.iter_mut().for_each(|s| *s = s.clamp(-0.98, 0.98));
    out
}

// ── Utilities ────────────────────────────────────────────────────────────────

const TAU: f32 = std::f32::consts::TAU;

fn lcg(state: u32) -> u32 {
    state.wrapping_mul(1664525).wrapping_add(1013904223)
}

fn rng_to_f32(state: u32) -> f32 {
    (state as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn fade_in_out(i: usize, total: usize, fade_len: usize) -> f32 {
    let fi = if i < fade_len { i as f32 / fade_len as f32 } else { 1.0 };
    let fo = if i > total - fade_len { (total - i) as f32 / fade_len as f32 } else { 1.0 };
    fi.min(fo)
}
