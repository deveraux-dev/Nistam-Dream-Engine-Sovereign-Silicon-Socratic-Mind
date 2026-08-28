//! Remix & remaster pipeline — tempo/key match + loudness normalization.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//!
//! Takes a source track and remixes it to a target BPM, key (Camelot), and
//! loudness level. Uses phase vocoder for time-stretch, HPSS for stem split,
//! and spectral restoration for cleanup.

use crate::alchemy::hpss::hpss_separate;
use crate::alchemy::restoration::restore_spectral;
use crate::alchemy::vocoder::phase_vocoder;
use crate::bpm::{detect_bpm, snap_bpm};
use crate::camelot::key_distance;
use crate::dsp::AudioBuffer;
use crate::key_detect::detect_key;

/// Remix target parameters.
#[derive(Debug, Clone)]
pub struct RemixTarget {
    /// Target BPM (0 = keep original).
    pub target_bpm: f32,
    /// Target key in Camelot notation (e.g. "8B"). Empty = keep original.
    pub target_key: String,
    /// Target integrated loudness in LUFS (e.g. -14.0 for streaming).
    pub loudness_lufs: f32,
}

/// Remix a track to match target BPM, key, and loudness.
pub fn remix_track(buf: &AudioBuffer, target: &RemixTarget) -> AudioBuffer {
    let sr = buf.sample_rate;
    let mut mono = buf.to_mono();

    // 1. Time-stretch to target BPM
    if target.target_bpm > 0.0 {
        let source_bpm = snap_bpm(detect_bpm(buf));
        let ratio = source_bpm / target.target_bpm;
        if (ratio - 1.0).abs() > 0.02 {
            mono = phase_vocoder(&mono, sr, ratio);
        }
    }

    // 2. Pitch-shift to target key (via vocoder stretch + resample)
    if !target.target_key.is_empty() {
        let source_camelot = detect_key(&mono, sr)
            .map(|k| k.camelot)
            .unwrap_or_default();
        if !source_camelot.is_empty() {
            if let Some(dist) = key_distance(&source_camelot, &target.target_key) {
                if dist > 0 {
                    // Each Camelot step ≈ 1 semitone (perfect fifth = 7 semitones / 7 steps)
                    let semitones = dist as f32;
                    let ratio = 2.0f32.powf(-semitones / 12.0);
                    let stretched = phase_vocoder(&mono, sr, ratio);
                    mono = resample_linear(&stretched, mono.len());
                }
            }
        }
    }

    // 3. Spectral restoration (clean up artifacts)
    mono = restore_spectral(&mono, sr);

    // 4. Normalize to target LUFS
    normalize_lufs(&mut mono, target.loudness_lufs);

    AudioBuffer { samples: vec![mono], sample_rate: sr }
}

/// Split a track into harmonic and percussive stems via HPSS.
pub fn stem_split(buf: &AudioBuffer) -> (AudioBuffer, AudioBuffer) {
    let sr = buf.sample_rate;
    let mono = buf.to_mono();
    let (harmonic, percussive) = hpss_separate(&mono, sr);
    (
        AudioBuffer { samples: vec![harmonic], sample_rate: sr },
        AudioBuffer { samples: vec![percussive], sample_rate: sr },
    )
}

/// Match the loudness and spectral balance of a reference track.
pub fn match_master(buf: &AudioBuffer, reference: &AudioBuffer) -> AudioBuffer {
    let sr = buf.sample_rate;
    let mut mono = buf.to_mono();
    let ref_mono = reference.to_mono();

    // Match integrated loudness
    let src_rms = rms_of(&mono);
    let ref_rms = rms_of(&ref_mono);
    if src_rms > 1e-10 && ref_rms > 1e-10 {
        let gain = ref_rms / src_rms;
        mono.iter_mut().for_each(|s| *s *= gain);
    }

    // Light spectral restore to match tonal balance
    mono = restore_spectral(&mono, sr);
    mono.iter_mut().for_each(|s| *s = s.clamp(-0.98, 0.98));

    AudioBuffer { samples: vec![mono], sample_rate: sr }
}

// ── Internals ────────────────────────────────────────────────────────────────

fn rms_of(signal: &[f32]) -> f32 {
    if signal.is_empty() { return 0.0; }
    (signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32).sqrt()
}

fn normalize_lufs(signal: &mut [f32], target_lufs: f32) {
    let rms = rms_of(signal);
    if rms < 1e-10 { return; }
    let current_lufs = 20.0 * rms.log10() - 0.691;
    let gain = 10.0f32.powf((target_lufs - current_lufs) / 20.0);
    signal.iter_mut().for_each(|s| *s = (*s * gain).clamp(-0.98, 0.98));
}

fn resample_linear(signal: &[f32], target_len: usize) -> Vec<f32> {
    if signal.is_empty() || target_len == 0 { return vec![0.0; target_len]; }
    let ratio = signal.len() as f64 / target_len as f64;
    (0..target_len).map(|i| {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let a = signal[idx.min(signal.len() - 1)];
        let b = signal[(idx + 1).min(signal.len() - 1)];
        a + (b - a) * frac
    }).collect()
}
