//! vocal_synth — sin/cos vocal primitives for the Recording Studio take path.
//! Integer core: pp-math `sin_mdeg`/`cos_mdeg` phase + permyriad envelope;
//! f32 exists ONLY at the sample write (ADR-0015 DDSP leaf, like deadpan.rs).
// @forge:allow_float

use crate::dsp::AudioBuffer;
use forge_harmonics::scale_voice::note_to_mhz;
use pp_math::fixed_point::trig::{cos_mdeg, sin_mdeg};

/// One vocal event: the MIDI-note half of a `SyllabicMidiEvent` without the
/// channel (forge-audio must not dep forge-calligraphy — callers map it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocalNote {
    /// MIDI note number; pitch resolves via `forge_harmonics::note_to_mhz`.
    pub note: u8,
    /// Velocity (0..127) → permyriad amplitude.
    pub velocity: u8,
    /// Note length in milliseconds.
    pub duration_ms: u16,
    /// Coda/final: short percussive tap — fast attack, long-tail release.
    pub is_final: bool,
}

/// Full turn in milli-degrees (`sin_mdeg` domain).
const TURN_MDEG: i64 = 360_000;
/// Attack ramp (ms). Finals snap faster.
const ATTACK_MS: u32 = 10;
/// Release ramp (ms): syllable vs final coda tail.
const RELEASE_MS: u32 = 40;
const RELEASE_FINAL_MS: u32 = 120;
/// Output headroom at the f32 leaf (matches `generate_synth`'s 0.5).
const LEAF_GAIN: f32 = 0.5;

/// Render a note sequence into a mono take. Voice = fundamental sine, a cos octave
/// partial at 1/4, and odd sine partials (3rd at 1/9, 5th at 1/25) — a harmonic
/// stack, still 2 zero-crossings per cycle.
/// Phase, envelope, and mix stay integer permyriad until the final write.
pub fn render_vocal_take(notes: &[VocalNote], sample_rate: u32) -> AudioBuffer {
    let sr = sample_rate.max(1) as i64;
    let mut samples: Vec<f32> = Vec::new();
    for n in notes {
        let n_samples = (sr * n.duration_ms as i64 / 1000).max(1);
        let freq_mhz = note_to_mhz(n.note) as i64;
        let attack = ms_to_samples(if n.is_final { 2 } else { ATTACK_MS }, sr);
        let release = ms_to_samples(if n.is_final { RELEASE_FINAL_MS } else { RELEASE_MS }, sr)
            .min(n_samples);
        let amp_pmy = n.velocity.min(127) as i64 * 10_000 / 127;
        // Phase accumulator numerator: phase_mdeg = acc / sr (mdeg/sample = mhz*360/sr).
        let mut acc: i64 = 0;
        for i in 0..n_samples {
            let phase = ((acc / sr) % TURN_MDEG) as i32;
            let phase2 = (((acc * 2) / sr) % TURN_MDEG) as i32;
            let phase3 = (((acc * 3) / sr) % TURN_MDEG) as i32;
            let phase5 = (((acc * 5) / sr) % TURN_MDEG) as i32;
            // Fundamental + octave shimmer + odd partials at 1/n² (reed/vocal-cord
            // colour). The odd partials are SINE: sin(3θ) and sin(5θ) vanish wherever
            // sin(θ) does, so the harmonic stack cannot add zero-crossings and the
            // pitch readback stays exact. All permyriad (-14100..14100 worst case).
            let wave_pmy = sin_mdeg(phase) as i64
                + cos_mdeg(phase2) as i64 / 4
                + sin_mdeg(phase3) as i64 / 9
                + sin_mdeg(phase5) as i64 / 25;
            let env_pmy = envelope_pmy(i, n_samples, attack, release);
            let out_pmy = wave_pmy * env_pmy / 10_000 * amp_pmy / 10_000;
            samples.push(out_pmy as f32 / 10_000.0 * LEAF_GAIN);
            acc += freq_mhz * 360;
        }
    }
    AudioBuffer { samples: vec![samples], sample_rate }
}

fn ms_to_samples(ms: u32, sr: i64) -> i64 {
    (sr * ms as i64 / 1000).max(1)
}

/// Linear attack/sustain/release, permyriad (0..=10000).
fn envelope_pmy(i: i64, len: i64, attack: i64, release: i64) -> i64 {
    let rise = if i < attack { i * 10_000 / attack } else { 10_000 };
    let tail_start = len - release;
    let fall = if i >= tail_start { (len - i) * 10_000 / release } else { 10_000 };
    rise.min(fall).clamp(0, 10_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio_session::SESSION_SAMPLE_RATE;

    fn a4(duration_ms: u16, is_final: bool) -> VocalNote {
        VocalNote { note: 69, velocity: 100, duration_ms, is_final }
    }

    #[test]
    fn take_is_audible_and_pitched_at_a4() {
        let buf = render_vocal_take(&[a4(500, false)], SESSION_SAMPLE_RATE);
        let s = &buf.samples[0];
        assert_eq!(s.len(), SESSION_SAMPLE_RATE as usize / 2);
        let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.2, "PCM readback: take is silent (peak {peak})");
        // 2 zero-crossings per cycle → crossings/2 / secs ≈ 440 Hz.
        let crossings = s.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
        let hz = crossings as f32 / 2.0 / 0.5;
        assert!((hz - 440.0).abs() < 44.0, "PCM readback: pitch {hz} Hz, want ~440");
    }

    /// The voice carries overtones, not just a fundamental. Goertzel-style
    /// correlation at each partial; the 3rd and 5th must be present and ordered
    /// 1 > 2 > 3 > 5, and the pitch must not move (2026-08-01, Sean: "needs harmonics").
    #[test]
    fn take_carries_a_harmonic_stack() {
        use std::f32::consts::TAU;
        let buf = render_vocal_take(&[a4(500, false)], SESSION_SAMPLE_RATE);
        let s = &buf.samples[0];
        let sr = SESSION_SAMPLE_RATE as f32;
        let mag = |mult: f32| -> f32 {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, &v) in s.iter().enumerate() {
                let t = TAU * 440.0 * mult * i as f32 / sr;
                re += v * t.cos();
                im += v * t.sin();
            }
            (re * re + im * im).sqrt() / s.len() as f32
        };
        let (h1, h2, h3, h5) = (mag(1.0), mag(2.0), mag(3.0), mag(5.0));
        assert!(h3 > 0.005, "3rd partial missing (h3 {h3})");
        assert!(h5 > 0.001, "5th partial missing (h5 {h5})");
        assert!(h1 > h2 && h2 > h3 && h3 > h5, "stack out of order: {h1} {h2} {h3} {h5}");
    }

    #[test]
    fn envelope_opens_and_closes() {
        let buf = render_vocal_take(&[a4(300, false)], SESSION_SAMPLE_RATE);
        let s = &buf.samples[0];
        assert!(s[0].abs() < 0.05, "attack starts near zero, got {}", s[0]);
        let last = s[s.len() - 1].abs();
        assert!(last < 0.05, "release lands near zero, got {last}");
        let mid = s[s.len() / 2].abs().max(s[s.len() / 2 + 9].abs());
        assert!(mid > 0.05, "sustain is live, got {mid}");
    }

    #[test]
    fn final_coda_starts_decaying_before_a_syllable_does() {
        let sr = SESSION_SAMPLE_RATE;
        let take = |f| render_vocal_take(&[VocalNote { note: 37, velocity: 80, duration_ms: 200, is_final: f }], sr);
        // 60ms before the end: the 120ms final release is already mid-fade
        // (~50% envelope) while the 40ms syllable release hasn't begun (100%).
        let rms_at_60ms_out = |b: &AudioBuffer| {
            let s = &b.samples[0];
            let n = (sr as usize * 10 / 1000).max(1);
            let end = s.len() - sr as usize * 55 / 1000;
            let t = &s[end - n..end];
            (t.iter().map(|v| v * v).sum::<f32>() / n as f32).sqrt()
        };
        let (fin, syl) = (rms_at_60ms_out(&take(true)), rms_at_60ms_out(&take(false)));
        assert!(syl > fin * 1.5, "syllable {syl} should still sustain where the final {fin} is fading");
    }

    #[test]
    fn render_is_deterministic() {
        let notes = [a4(120, false), VocalNote { note: 57, velocity: 90, duration_ms: 300, is_final: false }];
        let a = render_vocal_take(&notes, SESSION_SAMPLE_RATE);
        let b = render_vocal_take(&notes, SESSION_SAMPLE_RATE);
        assert_eq!(a.samples, b.samples, "integer core must render bit-identical");
    }

    #[test]
    fn empty_sequence_is_an_empty_take() {
        let buf = render_vocal_take(&[], SESSION_SAMPLE_RATE);
        assert!(buf.samples[0].is_empty());
    }
}
