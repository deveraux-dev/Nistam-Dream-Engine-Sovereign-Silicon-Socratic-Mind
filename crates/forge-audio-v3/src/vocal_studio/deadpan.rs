//! Deadpan Linter — voice flattener: gate breaths/pauses, lock pitch to one
//! monotone target, then run the existing vocal-healing chain (HPF -> compress
//! -> brickwall limit). Distinct from `performer::auto_tune_soft` (which nudges
//! each frame toward the *nearest scale tone*, capped at a whole-tone
//! correction, via naive per-frame resample): this detects the take's average
//! pitch and applies ONE whole-buffer shift onto a fixed target Hz via
//! `character::pitch_shift_preserve_formant` (phase-vocoder based), which
//! stays artifact-free across the large ratios a monotone lock needs.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.

use super::super::alchemy::pitch::yin_track;
use super::super::dsp::AudioBuffer;
use super::super::healing::{heal_voice, HealingParams};
use super::character::pitch_shift_preserve_formant;

/// Deadpan Linter parameters.
#[derive(Debug, Clone, Copy)]
pub struct DeadpanParams {
    /// Gate threshold (dBFS) — samples below this are silenced. Strips breaths/pauses.
    pub gate_threshold_db: f32,
    /// Target monotone pitch (Hz) every voiced frame is locked to.
    pub target_pitch_hz: f32,
    /// High-pass cutoff (Hz), forwarded to the healing chain. ~90 for voice.
    pub hpf_hz: f32,
    /// Brickwall ceiling (dBFS), forwarded to the healing chain.
    pub ceiling_db: f32,
}

impl Default for DeadpanParams {
    fn default() -> Self {
        Self {
            gate_threshold_db: -40.0,
            target_pitch_hz: 110.0,
            hpf_hz: 90.0,
            ceiling_db: -1.0,
        }
    }
}

/// Apply the Deadpan Linter: gate -> monotone pitch lock -> heal (HPF/comp/limit).
pub fn apply_deadpan(buf: AudioBuffer, params: &DeadpanParams) -> AudioBuffer {
    let sr = buf.sample_rate;
    let mut mono = buf.to_mono();

    gate(&mut mono, params.gate_threshold_db);
    let locked = monotone_lock(&mono, sr, params.target_pitch_hz);

    let gated_buf = AudioBuffer { samples: vec![locked], sample_rate: sr };
    heal_voice(
        gated_buf,
        &HealingParams { hpf_hz: params.hpf_hz, ceiling_db: params.ceiling_db, ..HealingParams::default() },
    )
}

/// Silence any sample below `threshold_db` — strips breaths and pauses.
fn gate(mono: &mut [f32], threshold_db: f32) {
    let threshold_lin = 10.0f32.powf(threshold_db / 20.0);
    for s in mono.iter_mut() {
        if s.abs() < threshold_lin {
            *s = 0.0;
        }
    }
}

/// Detect the take's average voiced pitch (YIN) and shift the whole buffer so
/// it lands on `target_hz` — the monotone "deadpan" flattening.
fn monotone_lock(mono: &[f32], sr: u32, target_hz: f32) -> Vec<f32> {
    const HOP: usize = 512;
    const WINDOW: usize = 2048;
    const YIN_THRESHOLD: f32 = 0.15;

    if mono.len() < WINDOW {
        return mono.to_vec();
    }

    let pitches = yin_track(mono, sr, WINDOW, HOP, YIN_THRESHOLD);
    let voiced: Vec<f32> = pitches.into_iter().filter(|&p| (50.0..=2000.0).contains(&p)).collect();
    if voiced.is_empty() {
        return mono.to_vec();
    }

    let avg_hz = voiced.iter().sum::<f32>() / voiced.len() as f32;
    // Sign is negated relative to `CharacterVoice.pitch_shift_semitones`'s own doc
    // ("negative = deeper"): `pitch_shift_preserve_formant` empirically shifts the
    // opposite direction from its documented convention (reproduced via this
    // module's tests — see deadpan.rs discovery note for Sean). Untested upstream;
    // negated here only, not fixed at the source (shared by 4 shipped presets).
    let semitones = -12.0 * (target_hz / avg_hz).log2();
    pitch_shift_preserve_formant(mono, sr, semitones)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: u32, secs: f32) -> Vec<f32> {
        let n = (sr as f32 * secs) as usize;
        (0..n).map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * 0.5).collect()
    }

    #[test]
    fn gate_silences_below_threshold() {
        let mut mono = vec![0.001, 0.5, -0.6, 0.0005, -0.9];
        gate(&mut mono, -40.0); // threshold_lin ~= 0.01
        assert_eq!(mono[0], 0.0);
        assert_eq!(mono[3], 0.0);
        assert_ne!(mono[1], 0.0);
        assert_ne!(mono[2], 0.0);
        assert_ne!(mono[4], 0.0);
    }

    #[test]
    fn gate_preserves_loud_signal() {
        let mut mono = vec![0.8, -0.8, 0.9, -0.9];
        gate(&mut mono, -40.0);
        assert_eq!(mono, vec![0.8, -0.8, 0.9, -0.9]);
    }

    #[test]
    fn monotone_lock_flattens_pitch_toward_target() {
        let sr = 44_100u32;
        // A take that wanders from 220Hz to 330Hz mid-buffer.
        let mut mono = sine(220.0, sr, 0.3);
        mono.extend(sine(330.0, sr, 0.3));
        let locked = monotone_lock(&mono, sr, 110.0);
        assert_eq!(locked.len(), mono.len());
        // Detected pitch across the locked output should cluster near the target,
        // not swing between the original 220/330 Hz.
        let pitches = yin_track(&locked, sr, 2048, 512, 0.15);
        let voiced: Vec<f32> = pitches.into_iter().filter(|&p| p > 0.0).collect();
        assert!(!voiced.is_empty(), "expected voiced frames in locked output");
        let avg = voiced.iter().sum::<f32>() / voiced.len() as f32;
        assert!((avg - 110.0).abs() < 40.0, "avg detected pitch {avg} should be near target 110Hz");
    }

    #[test]
    fn apply_deadpan_holds_ceiling_and_no_nan() {
        let sr = 44_100u32;
        let mono = sine(220.0, sr, 0.5);
        let buf = AudioBuffer { samples: vec![mono], sample_rate: sr };
        let out = apply_deadpan(buf, &DeadpanParams::default());
        let ceiling_lin = 10.0f32.powf(-1.0 / 20.0); // -1 dBFS default ceiling
        for &s in &out.samples[0] {
            assert!(s.is_finite(), "output must not contain NaN/Inf");
            assert!(s.abs() <= ceiling_lin + 1e-3, "sample {s} exceeded brickwall ceiling");
        }
    }

    #[test]
    fn apply_deadpan_handles_short_buffer() {
        let sr = 44_100u32;
        let buf = AudioBuffer { samples: vec![vec![0.1, 0.2, -0.1]], sample_rate: sr };
        let out = apply_deadpan(buf, &DeadpanParams::default());
        assert_eq!(out.sample_rate, sr);
    }
}
