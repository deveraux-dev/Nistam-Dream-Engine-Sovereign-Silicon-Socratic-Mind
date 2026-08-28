//! VocalFrame — the unified 120 Hz integer voice primitive.
//!
//! Emitted once per DET-CLOCK tick from mic_capture's streaming API.
//! Carries pitch, loudness, onset, emotion, and chroma in the integer
//! Permyriad domain — the SINGLE type that feeds gameplay (resonance_combat,
//! rhythm_judge, bard_aura), authoring (dialogue_cues, cutscene SetMood),
//! and production (youtube scene-map, spectral duck trigger).
//!
//! This is the capillary blood cell of the vocal pipeline. DSP leaves
//! produce floats; VocalFrame converts ONCE at the boundary and stays
//! integer from there on. Two-Clock invariant: DSP leaf → VocalFrame →
//! all 120 Hz consumers.
//!
//! Layout: 28 bytes, Copy, no heap, no float.

/// Permyriad: 10000 = 1.0 (full scale). Convention across 13forge.
pub type Permyriad = i32;

/// Full-scale Permyriad constant.
pub const PMY_FULL: Permyriad = 10_000;

/// A single frame of vocal analysis at 120 Hz.
///
/// All fields are integer-domain. The DSP boundary (mic_capture) converts
/// f32 signals into this form exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct VocalFrame {
    /// Fundamental frequency in millihertz (Hz × 1000). 0 = unvoiced.
    /// Range: 0 (silence) to ~2_000_000 (2 kHz soprano).
    /// Feeds: resonance_combat::phase_align_q, auto_tune, bard_aura.
    pub f0_mhz: i32,

    /// RMS loudness in Permyriad (0 = silence, 10000 = 0 dBFS).
    /// Feeds: combat pressure, aura magnitude, duck trigger.
    pub rms_q: Permyriad,

    /// True on frames where a syllable onset (attack) is detected.
    /// Feeds: rhythm_judge hit, scene_map beat boundary, euclid modulation.
    pub onset: bool,

    /// Emotion 4-vector in Permyriad: [valence, arousal, tension, release].
    /// Feeds: AuraKind selection, VFX reactivity, cutscene SetMood.
    pub emotion: [Permyriad; 4],

    /// Pitch class (0–11, C=0). 255 = unvoiced/unknown.
    /// Feeds: camelot key matching, scale targeting, harmonic mixing.
    pub chroma: u8,
}

impl VocalFrame {
    /// An empty/silent frame (all zeros, unvoiced).
    pub const SILENT: Self = Self {
        f0_mhz: 0,
        rms_q: 0,
        onset: false,
        emotion: [5000, 5000, 5000, 5000], // neutral midpoint
        chroma: 255,
    };

    /// Is this frame voiced (has a detectable pitch)?
    #[inline]
    pub fn is_voiced(&self) -> bool {
        self.f0_mhz > 0 && self.chroma != 255
    }

    /// Is this frame loud enough to be considered active speech?
    /// Threshold: 250 permyriad ≈ -32 dBFS.
    #[inline]
    pub fn is_active(&self, threshold_q: Permyriad) -> bool {
        self.rms_q > threshold_q
    }

    /// Dominant emotion dimension (index 0–3: valence/arousal/tension/release).
    /// Returns the index with the highest deviation from neutral (5000).
    #[inline]
    pub fn dominant_emotion(&self) -> usize {
        let mut best_idx = 0;
        let mut best_dev = 0i32;
        for (i, &e) in self.emotion.iter().enumerate() {
            let dev = (e - 5000).abs();
            if dev > best_dev {
                best_dev = dev;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Convert from DSP-leaf floats. The ONE float→integer boundary.
    /// Called by mic_capture::drain_frame() — nowhere else.
    #[inline]
    pub fn from_dsp(
        f0_hz: f32,
        rms: f32,
        onset: bool,
        valence: f32,
        arousal: f32,
        tension: f32,
        release: f32,
    ) -> Self {
        let f0_mhz = (f0_hz * 1000.0) as i32;
        let rms_q = (rms.clamp(0.0, 1.0) * 10_000.0) as Permyriad;

        let chroma = if f0_hz > 50.0 && f0_hz < 2000.0 {
            let midi = 69.0 + 12.0 * (f0_hz / 440.0).log2();
            ((midi as i32).rem_euclid(12)) as u8
        } else {
            255
        };

        Self {
            f0_mhz,
            rms_q,
            onset,
            emotion: [
                (valence.clamp(0.0, 1.0) * 10_000.0) as Permyriad,
                (arousal.clamp(0.0, 1.0) * 10_000.0) as Permyriad,
                (tension.clamp(0.0, 1.0) * 10_000.0) as Permyriad,
                (release.clamp(0.0, 1.0) * 10_000.0) as Permyriad,
            ],
            chroma,
        }
    }
}

/// Emotion dimension indices for readability.
pub const VALENCE: usize = 0;
pub const AROUSAL: usize = 1;
pub const TENSION: usize = 2;
pub const RELEASE: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_dsp() {
        let frame = VocalFrame::from_dsp(440.0, 0.5, true, 0.8, 0.6, 0.3, 0.4);
        assert_eq!(frame.f0_mhz, 440_000);
        assert_eq!(frame.rms_q, 5000);
        assert!(frame.onset);
        assert_eq!(frame.chroma, 9); // A = pitch class 9
        assert_eq!(frame.emotion[VALENCE], 8000);
        assert_eq!(frame.emotion[AROUSAL], 6000);
        assert!(frame.is_voiced());
    }

    #[test]
    fn test_silent_frame() {
        let frame = VocalFrame::SILENT;
        assert!(!frame.is_voiced());
        assert!(!frame.is_active(250));
        assert_eq!(frame.dominant_emotion(), 0); // all neutral, first wins
    }

    #[test]
    fn test_dominant_emotion() {
        let frame = VocalFrame::from_dsp(300.0, 0.3, false, 0.5, 0.9, 0.5, 0.5);
        assert_eq!(frame.dominant_emotion(), AROUSAL); // 9000 vs 5000
    }
}
