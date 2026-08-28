//! Musical key detection via chromagram analysis and Krumhansl-Kessler correlation.
//!
//! Analyzes first 30 seconds of audio, computes a 12-bin chromagram (one per pitch class),
//! correlates against 24 key profiles (12 major + 12 minor), returns Camelot notation.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchClass {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B,
}

impl PitchClass {
    const ALL: [PitchClass; 12] = [
        PitchClass::C, PitchClass::Cs, PitchClass::D, PitchClass::Ds,
        PitchClass::E, PitchClass::F, PitchClass::Fs, PitchClass::G,
        PitchClass::Gs, PitchClass::A, PitchClass::As, PitchClass::B,
    ];

    // Only called from this file's own test (`PitchClass::A.index()` at
    // `#[cfg(test)]` line ~324) — a lib-only build never sees that call site,
    // so the allow stays (2026-08-20, restored after a revasc pass wrongly
    // removed it as "stale": it wasn't stale, just test-only).
    #[allow(dead_code)]
    fn index(self) -> usize {
        match self {
            PitchClass::C => 0,  PitchClass::Cs => 1, PitchClass::D => 2,
            PitchClass::Ds => 3, PitchClass::E => 4,  PitchClass::F => 5,
            PitchClass::Fs => 6, PitchClass::G => 7,  PitchClass::Gs => 8,
            PitchClass::A => 9,  PitchClass::As => 10, PitchClass::B => 11,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PitchClass::C => "C",   PitchClass::Cs => "C#", PitchClass::D => "D",
            PitchClass::Ds => "D#", PitchClass::E => "E",   PitchClass::F => "F",
            PitchClass::Fs => "F#", PitchClass::G => "G",   PitchClass::Gs => "G#",
            PitchClass::A => "A",   PitchClass::As => "A#", PitchClass::B => "B",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Major(PitchClass),
    Minor(PitchClass),
}

impl Key {
    pub fn name(&self) -> String {
        match self {
            Key::Major(p) => format!("{} major", p.name()),
            Key::Minor(p) => format!("{} minor", p.name()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyResult {
    pub key: Key,
    pub camelot: String,
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Krumhansl-Kessler key profiles
// ---------------------------------------------------------------------------

const KK_MAJOR: [f64; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const KK_MINOR: [f64; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

// Camelot wheel: [pitch_class_index] -> camelot code
const CAMELOT_MAJOR: [&str; 12] = ["8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B"];
const CAMELOT_MINOR: [&str; 12] = ["5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A"];

// ---------------------------------------------------------------------------
// Harmonic coherence — the conductor seam's runtime read of `pp_math::spectral`
// ---------------------------------------------------------------------------

/// DRAINED 08-02 -> [`crate::camelot::affinity_pmy`]. A wheel parser and adjacency
/// rule were written HERE while `camelot.rs` already owned both (`parse_camelot`,
/// `key_distance`, `is_compatible`) — the locating grep was capped and returned
/// only this file. Forwarder kept so no caller breaks (root#revascularize).
pub fn camelot_affinity_pmy(a: &str, b: &str) -> i64 {
    crate::camelot::affinity_pmy(a, b)
}

/// Read the harmonic spectrum of a track set: which tracks form a coherent mixable
/// cluster, and which are loud strangers.
///
/// The conductor/DAW read. Diagonal = each track's own energy; off-diagonal =
/// Camelot adjacency. `Spectrum::Uncoupled` therefore means loud and in a clashing
/// key — visible to the operator, never scored as structure. Integer throughout, so
/// the verdict replays bit-identically off the same set.
pub fn harmonic_spectrum(tracks: &[(String, i64)], floor: i32) -> pp_math::spectral::Spectrum {
    crate::camelot::harmonic_spectrum(tracks, floor)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect the musical key from audio samples (f32 mono, first 30 seconds used).
pub fn detect_key(samples: &[f32], sample_rate: u32) -> Option<KeyResult> {
    if samples.is_empty() || sample_rate == 0 {
        return None;
    }

    // Cap at 30 seconds
    let max_samples = (sample_rate as usize) * 30;
    let samples = if samples.len() > max_samples { &samples[..max_samples] } else { samples };

    // Compute chromagram
    let chroma = compute_chromagram(samples, sample_rate);

    // Check for silence
    let total: f64 = chroma.iter().sum();
    if total < 1e-10 {
        return None;
    }

    // Correlate against all 24 keys, find best match
    let mut best_corr = f64::NEG_INFINITY;
    let mut best_key = Key::Major(PitchClass::C);
    let mut best_idx = 0usize;
    let mut is_major = true;

    for (i, &pc) in PitchClass::ALL.iter().enumerate() {
        // Rotate profile to match this root
        let major_profile = rotate_profile(&KK_MAJOR, i);
        let minor_profile = rotate_profile(&KK_MINOR, i);

        let major_corr = pearson_correlation(&chroma, &major_profile);
        let minor_corr = pearson_correlation(&chroma, &minor_profile);

        if major_corr > best_corr {
            best_corr = major_corr;
            best_key = Key::Major(pc);
            best_idx = i;
            is_major = true;
        }
        if minor_corr > best_corr {
            best_corr = minor_corr;
            best_key = Key::Minor(pc);
            best_idx = i;
            is_major = false;
        }
    }

    // Confidence: map correlation from [-1, 1] to [0, 1]
    let confidence = ((best_corr + 1.0) / 2.0).clamp(0.0, 1.0);

    let camelot = if is_major {
        CAMELOT_MAJOR[best_idx]
    } else {
        CAMELOT_MINOR[best_idx]
    };

    Some(KeyResult {
        key: best_key,
        camelot: camelot.to_string(),
        confidence,
    })
}

// ---------------------------------------------------------------------------
// Chromagram via Goertzel algorithm
// ---------------------------------------------------------------------------

/// Compute a 12-bin chromagram by summing Goertzel energy for each pitch class
/// across multiple octaves, using overlapping frames.
fn compute_chromagram(samples: &[f32], sample_rate: u32) -> [f64; 12] {
    let mut chroma = [0.0f64; 12];
    let frame_size = 4096usize;
    let hop_size = 2048usize;
    let sr = sample_rate as f64;

    // Frequencies: C1 to B7 (7 octaves, 84 notes)
    // MIDI 24 (C1) to MIDI 107 (B7)
    let mut note_freqs: Vec<(usize, f64)> = Vec::with_capacity(84);
    for midi in 24..108 {
        let freq = 440.0 * 2.0f64.powf((midi as f64 - 69.0) / 12.0);
        if freq < sr / 2.0 {
            let pitch_class = (midi % 12) as usize;
            note_freqs.push((pitch_class, freq));
        }
    }

    let mut frame_count = 0u64;
    let mut pos = 0;

    while pos + frame_size <= samples.len() {
        // Apply Hann window
        let frame: Vec<f64> = (0..frame_size)
            .map(|i| {
                let w = 0.5 * (1.0 - (2.0 * PI * i as f64 / (frame_size - 1) as f64).cos());
                samples[pos + i] as f64 * w
            })
            .collect();

        // Goertzel for each note frequency
        for &(pc, freq) in &note_freqs {
            let power = goertzel(&frame, freq, sr);
            chroma[pc] += power;
        }

        frame_count += 1;
        pos += hop_size;
    }

    // Normalize by frame count
    if frame_count > 0 {
        for bin in &mut chroma {
            *bin /= frame_count as f64;
        }
    }

    chroma
}

/// Goertzel algorithm — compute power at a single frequency. O(N) per frequency.
fn goertzel(frame: &[f64], target_freq: f64, sample_rate: f64) -> f64 {
    let n = frame.len();
    let k = (target_freq * n as f64 / sample_rate).round();
    let w = 2.0 * PI * k / n as f64;
    let coeff = 2.0 * w.cos();

    let mut s0;
    let mut s1 = 0.0;
    let mut s2 = 0.0;

    for &sample in frame {
        s0 = sample + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }

    // Power = s1^2 + s2^2 - coeff * s1 * s2
    let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
    power.abs()
}

// ---------------------------------------------------------------------------
// Correlation
// ---------------------------------------------------------------------------

/// Rotate a 12-element profile by `shift` positions (for transposing key profiles).
fn rotate_profile(profile: &[f64; 12], shift: usize) -> [f64; 12] {
    let mut rotated = [0.0; 12];
    for i in 0..12 {
        rotated[i] = profile[(i + 12 - shift) % 12];
    }
    rotated
}

/// Pearson correlation coefficient between two 12-element arrays.
fn pearson_correlation(a: &[f64; 12], b: &[f64; 12]) -> f64 {
    let n = 12.0;
    let sum_a: f64 = a.iter().sum();
    let sum_b: f64 = b.iter().sum();
    let mean_a = sum_a / n;
    let mean_b = sum_b / n;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..12 {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    let denom = (var_a * var_b).sqrt();
    if denom < 1e-15 {
        return 0.0;
    }

    cov / denom
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI as PI32;

    /// Generate a sine wave at a given frequency.
    fn sine_wave(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        (0..num_samples)
            .map(|i| (2.0 * PI32 * freq * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    /// Generate a scale with the tonic weighted more heavily (longer duration).
    fn scale_samples(freqs: &[f32], sample_rate: u32, note_duration: f32) -> Vec<f32> {
        let mut samples = Vec::new();
        for (i, &freq) in freqs.iter().enumerate() {
            // Tonic (first note) gets 3x duration to bias the chromagram
            let dur = if i == 0 { note_duration * 3.0 } else { note_duration };
            samples.extend(sine_wave(freq, sample_rate, dur));
        }
        // End on the tonic again
        samples.extend(sine_wave(freqs[0], sample_rate, note_duration * 2.0));
        samples
    }

    #[test]
    fn test_chromagram_single_pitch() {
        // A4 = 440Hz should produce strong energy in the A bin (index 9)
        let samples = sine_wave(440.0, 44100, 2.0);
        let chroma = compute_chromagram(&samples, 44100);

        let max_idx = chroma.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap().0;

        assert_eq!(max_idx, PitchClass::A.index(), "A4 should produce strongest A bin");
    }

    #[test]
    fn test_key_c_major() {
        // C major scale: C4, D4, E4, F4, G4, A4, B4
        let freqs = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 493.88];
        let samples = scale_samples(&freqs, 44100, 0.5);
        let result = detect_key(&samples, 44100).unwrap();

        assert_eq!(result.key, Key::Major(PitchClass::C), "C major scale should detect as C major");
        assert_eq!(result.camelot, "8B");
    }

    #[test]
    fn a_clashing_loud_track_reads_as_uncoupled_not_as_a_mode() {
        use pp_math::spectral::{Spectrum, COUPLING_FLOOR_PMY};
        // Equal energy on all three, so COUPLING is the only thing that can decide: 8B/9B are
        // one wheel step apart in the same mode, 2A clashes with both. A louder clashing track
        // would legitimately carry the mode — energy is real signal — so the discriminator
        // this seam buys is over tracks of comparable loudness, which is the mixing case.
        let mut tracks: Vec<(String, i64)> = Vec::with_capacity(3); // @forge:allow_alloc test fixture, no callback
        for (c, e) in [("8B", 5_000i64), ("9B", 5_000), ("2A", 5_000)] {
            tracks.push((String::from(c), e)); // @forge:allow_alloc test fixture, no callback
        }
        let s = harmonic_spectrum(&tracks, COUPLING_FLOOR_PMY);
        let Spectrum::Coupled(p) = s else { panic!("a mixable pair is a mode, got {s:?}") };
        assert!(
            p.x[0].0 > p.x[2].0 && p.x[1].0 > p.x[2].0,
            "the mixable pair must carry the mode over the louder clashing track: {:?}",
            p.x
        );
        // A lone loud track couples to nothing and must stay visible as noise.
        let lone = harmonic_spectrum(&tracks[2..], COUPLING_FLOOR_PMY);
        assert!(matches!(lone, Spectrum::Uncoupled { .. }), "got {lone:?}");
    }

    #[test]
    fn the_drained_names_still_reach_the_owner() {
        // Substance lives in `camelot::spectral_tests`; this only proves the
        // forwarders forward, so the fold left no dangling surface.
        assert_eq!(camelot_affinity_pmy("8B", "9B"), crate::camelot::affinity_pmy("8B", "9B"));
    }

    #[test]
    fn test_camelot_mapping() {
        // Verify all 24 Camelot codes are unique and correctly formatted
        let mut all_codes: Vec<&str> = Vec::new();
        all_codes.extend_from_slice(&CAMELOT_MAJOR);
        all_codes.extend_from_slice(&CAMELOT_MINOR);

        // All should be unique
        let mut sorted = all_codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 24, "All 24 Camelot codes should be unique");

        // All should match pattern: number (1-12) + letter (A or B)
        for code in all_codes {
            let letter = code.chars().last().unwrap();
            assert!(letter == 'A' || letter == 'B', "Camelot code {} should end with A or B", code);
            let num: u32 = code[..code.len()-1].parse().unwrap();
            assert!((1..=12).contains(&num), "Camelot number should be 1-12, got {}", num);
        }
    }

    #[test]
    fn test_empty_input() {
        assert!(detect_key(&[], 44100).is_none());
        assert!(detect_key(&[0.0; 100], 0).is_none());
    }

    #[test]
    fn test_silence_returns_none() {
        let silence = vec![0.0f32; 44100 * 5];
        assert!(detect_key(&silence, 44100).is_none());
    }

    #[test]
    fn test_confidence_range() {
        let samples = sine_wave(440.0, 44100, 2.0);
        let result = detect_key(&samples, 44100).unwrap();
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0,
            "Confidence {} should be in [0, 1]", result.confidence);
    }

    #[test]
    fn test_a_minor() {
        // A minor scale: A4, B4, C5, D5, E5, F5, G5
        let freqs = [440.00, 493.88, 523.25, 587.33, 659.25, 698.46, 783.99];
        let samples = scale_samples(&freqs, 44100, 0.5);
        let result = detect_key(&samples, 44100).unwrap();

        assert_eq!(result.key, Key::Minor(PitchClass::A), "A minor scale should detect as A minor");
        assert_eq!(result.camelot, "8A");
    }

    #[test]
    fn test_pearson_identical() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let r = pearson_correlation(&a, &a);
        assert!((r - 1.0).abs() < 1e-10, "Self-correlation should be 1.0");
    }
}
