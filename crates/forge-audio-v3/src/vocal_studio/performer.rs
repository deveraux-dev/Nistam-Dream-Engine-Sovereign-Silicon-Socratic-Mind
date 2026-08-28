//! Performer pipeline — singers, storytellers, narrators.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//!
//! Singers: key detection → soft auto-tune → harmony generation
//! Storytellers: pacing normalization → emphasis boost → beat-grid quantize
//! Narrators: filler removal → loudness normalization → consistent pacing

use crate::alchemy::pitch::yin_track;
use crate::alchemy::vocoder::phase_vocoder;
use crate::bpm::{detect_bpm, snap_bpm};
use crate::dsp::AudioBuffer;
use crate::key_detect::detect_key;
use crate::speech_clip::{edit_speech, ClipConfig};

/// Performance processing mode.
#[derive(Debug, Clone)]
pub enum PerformanceMode {
    /// Vocal performance with pitch correction and optional harmonies.
    Singer {
        /// Auto-tune strength: 0.0 = off, 1.0 = hard snap to scale.
        auto_tune_strength: f32,
        /// Harmony voices to generate (semitone intervals, e.g. [4, 7] for major triad).
        harmony_intervals: Vec<i8>,
    },
    /// Spoken word with pacing normalization.
    Storyteller {
        /// Target pacing BPM (speech rhythm, not music tempo). 0 = auto.
        pace_bpm: f32,
        /// Emphasis peak boost in dB (applied to detected emphasis points).
        emphasis_boost_db: f32,
    },
    /// Clean narration: remove fillers, normalize loudness.
    Narrator {
        /// Target integrated loudness in LUFS.
        target_lufs: f32,
    },
}

/// Process a vocal performance according to the selected mode.
pub fn process_performance(buf: &AudioBuffer, mode: &PerformanceMode) -> AudioBuffer {
    let sr = buf.sample_rate;
    match mode {
        PerformanceMode::Singer { auto_tune_strength, harmony_intervals } => {
            let mono = buf.to_mono();
            let key_result = detect_key(&mono, sr);

            // Apply soft auto-tune to lead vocal
            let key_index = key_result.map(|k| camelot_to_key_index(&k.camelot)).unwrap_or(0);
            let tuned = auto_tune_soft(&mono, sr, key_index, *auto_tune_strength);

            // Generate and mix harmony voices
            if harmony_intervals.is_empty() {
                return AudioBuffer { samples: vec![tuned], sample_rate: sr };
            }

            let mut mix = tuned.clone();
            let harmony_gain = 0.35 / harmony_intervals.len().max(1) as f32;
            for &interval in harmony_intervals {
                let harmony = generate_harmony(&mono, sr, interval);
                for (i, h) in harmony.iter().enumerate() {
                    if i < mix.len() {
                        mix[i] += h * harmony_gain;
                    }
                }
            }
            // Soft-clip to prevent overs
            mix.iter_mut().for_each(|s| *s = s.clamp(-0.98, 0.98));
            AudioBuffer { samples: vec![mix], sample_rate: sr }
        }

        PerformanceMode::Storyteller { pace_bpm, emphasis_boost_db } => {
            let mono = buf.to_mono();
            let detected_bpm = snap_bpm(detect_bpm(buf));
            let target = if *pace_bpm > 0.0 { *pace_bpm } else { detected_bpm };

            // Time-stretch to target pacing
            let ratio = detected_bpm / target;
            let stretched = if (ratio - 1.0).abs() > 0.05 {
                phase_vocoder(&mono, sr, ratio)
            } else {
                mono.clone()
            };

            // Boost emphasis peaks
            let boosted = boost_emphasis(&stretched, sr, *emphasis_boost_db);
            AudioBuffer { samples: vec![boosted], sample_rate: sr }
        }

        PerformanceMode::Narrator { target_lufs } => {
            // Remove fillers and pauses via speech_clip
            let config = ClipConfig {
                min_pause_secs: 0.25,
                silence_threshold: 0.02,
                max_filler_duration_secs: 0.5,
                ..ClipConfig::default()
            };
            let bpm = snap_bpm(detect_bpm(buf));
            let result = edit_speech(buf, bpm, &config);

            // Normalize to target LUFS
            let mut output = result.output.to_mono();
            normalize_lufs(&mut output, *target_lufs);
            AudioBuffer { samples: vec![output], sample_rate: result.output.sample_rate }
        }
    }
}

/// Generate a harmony voice at the given semitone interval.
pub fn generate_harmony(mono: &[f32], sr: u32, interval_semitones: i8) -> Vec<f32> {
    if mono.is_empty() {
        return vec![];
    }
    let ratio = 2.0f32.powf(-(interval_semitones as f32) / 12.0);
    let stretched = phase_vocoder(mono, sr, ratio);
    // Resample back to original length
    resample_linear(&stretched, mono.len())
}

/// Soft auto-tune: gently correct pitch toward nearest scale tone.
/// `strength` 0.0 = no correction, 1.0 = snap to nearest.
pub fn auto_tune_soft(mono: &[f32], sr: u32, key_index: u8, strength: f32) -> Vec<f32> {
    if strength < 0.01 || mono.is_empty() {
        return mono.to_vec();
    }

    let hop = 512;
    let window = 2048;
    let pitches = yin_track(mono, sr, window, hop, 0.15);
    let scale = scale_for_key(key_index);

    let mut output = mono.to_vec();

    for (frame_idx, &f0) in pitches.iter().enumerate() {
        if f0 < 50.0 || f0 > 2000.0 {
            continue; // unvoiced, skip
        }

        let midi = 69.0 + 12.0 * (f0 / 440.0).log2();
        let chroma = ((midi as i32) % 12 + 12) % 12;
        let target_chroma = nearest_scale_tone(chroma as u8, &scale);
        let correction_semitones = target_chroma as f32 - chroma as f32;

        // Only correct if off by less than a whole tone
        if correction_semitones.abs() > 2.0 || correction_semitones.abs() < 0.01 {
            continue;
        }

        // Apply micro pitch correction to this frame's samples
        let frame_start = frame_idx * hop;
        let frame_end = (frame_start + window).min(output.len());
        let correction = correction_semitones * strength;
        let ratio = 2.0f32.powf(correction / 12.0);

        // Simple per-frame resample for micro-correction
        let frame_len = frame_end - frame_start;
        let src = &mono[frame_start..frame_end];
        for j in 0..frame_len {
            let src_pos = j as f32 * ratio;
            let idx = src_pos as usize;
            if idx + 1 < src.len() {
                let frac = src_pos - idx as f32;
                output[frame_start + j] = src[idx] * (1.0 - frac) + src[idx + 1] * frac;
            }
        }
    }

    output
}

// ── Internals ────────────────────────────────────────────────────────────────

/// Get the scale tones (chromatic set) for a key index.
/// Even indices = major (B in Camelot), odd = minor (A in Camelot).
fn scale_for_key(key_index: u8) -> Vec<u8> {
    let root = (key_index / 2) % 12;
    let intervals = if key_index % 2 == 0 {
        &[0, 2, 4, 5, 7, 9, 11][..] // major
    } else {
        &[0, 2, 3, 5, 7, 8, 10][..] // natural minor
    };
    intervals.iter().map(|&i| (root + i) % 12).collect()
}

/// Find the nearest scale tone to a given chroma value.
fn nearest_scale_tone(chroma: u8, scale: &[u8]) -> u8 {
    let mut best = scale[0];
    let mut best_dist = circular_dist(chroma, best, 12);
    for &tone in &scale[1..] {
        let d = circular_dist(chroma, tone, 12);
        if d < best_dist {
            best_dist = d;
            best = tone;
        }
    }
    best
}

/// Circular distance on a ring of size `modulus`.
fn circular_dist(a: u8, b: u8, modulus: u8) -> u8 {
    let d = if a > b { a - b } else { b - a };
    d.min(modulus - d)
}

/// Linear resample to target length.
fn resample_linear(signal: &[f32], target_len: usize) -> Vec<f32> {
    if signal.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    let ratio = signal.len() as f64 / target_len as f64;
    (0..target_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = signal[idx.min(signal.len() - 1)];
            let b = signal[(idx + 1).min(signal.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

/// Boost emphasis peaks: find RMS peaks and apply gain.
fn boost_emphasis(signal: &[f32], _sr: u32, boost_db: f32) -> Vec<f32> {
    if boost_db.abs() < 0.1 {
        return signal.to_vec();
    }
    let gain = 10.0f32.powf(boost_db / 20.0);
    let window = 4096;
    let hop = window / 2;
    let mut output = signal.to_vec();

    // Find global RMS for threshold
    let global_rms = (signal.iter().map(|s| s * s).sum::<f32>() / signal.len().max(1) as f32).sqrt();
    let threshold = global_rms * 1.5;

    let n_frames = signal.len().saturating_sub(window) / hop;
    for frame in 0..n_frames {
        let start = frame * hop;
        let end = start + window;
        let rms = (signal[start..end].iter().map(|s| s * s).sum::<f32>() / window as f32).sqrt();
        if rms > threshold {
            // Apply smooth gain to this frame
            for i in start..end.min(output.len()) {
                let env = 0.5 * (1.0 - ((i - start) as f32 / window as f32 * std::f32::consts::PI * 2.0).cos());
                output[i] *= 1.0 + (gain - 1.0) * env * 0.5;
            }
        }
    }
    output.iter_mut().for_each(|s| *s = s.clamp(-0.98, 0.98));
    output
}

/// Simple LUFS-approximate normalization.
fn normalize_lufs(signal: &mut [f32], target_lufs: f32) {
    if signal.is_empty() {
        return;
    }
    let rms = (signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32).sqrt();
    if rms < 1e-10 {
        return;
    }
    // Approximate: LUFS ≈ 20*log10(rms) - 0.691 (simplified K-weighting)
    let current_lufs = 20.0 * rms.log10() - 0.691;
    let gain_db = target_lufs - current_lufs;
    let gain = 10.0f32.powf(gain_db / 20.0);
    signal.iter_mut().for_each(|s| *s = (*s * gain).clamp(-0.98, 0.98));
}

/// Convert a Camelot string (e.g. "8B") to a key_index (0-23).
/// Even = major (B), odd = minor (A). Root derived from Camelot number.
fn camelot_to_key_index(camelot: &str) -> u8 {
    use crate::camelot::parse_camelot;
    match parse_camelot(camelot) {
        Some((num, is_major)) => {
            let root = ((num as u16 + 6) % 12) as u8; // Camelot → pitch class
            if is_major { root * 2 } else { root * 2 + 1 }
        }
        None => 0,
    }
}
