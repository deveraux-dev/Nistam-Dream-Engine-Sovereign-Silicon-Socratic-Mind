//! Harmonic context for the audio brush.
//!
//! Maps `BardPhraseKind` + a live harmonic context (root + `ScaleMask`) to
//! concrete MIDI notes. Every emitted note is gated through `ScaleMask::is_member`
//! — no off-scale pitch can be physically emitted.
//!
//! All ops are O(1), integer, zero-heap. Held by the 120 Hz CPU thread.

use forge_harmonics::{note_to_mhz, ScaleMask, VoicePreset};
use crate::phrase::BardPhraseKind;

/// A Dorian (C, D, E, F#, G, A, B): the default brush scale.
/// Contains A (pc 9) and F# (pc 6) — preserving the proven grave-bell sound
/// (A3 root + F#3 minor-third-below) while enforcing in-key harmonic constraint.
const A_DORIAN: ScaleMask = ScaleMask(0xAD5);

/// Harmonic context for one brush stroke session.
/// Held by the 120 Hz CPU thread; zero-heap to mutate.
#[derive(Debug, Clone, Copy)]
pub struct HarmonicBrushState {
    /// Tonic MIDI note (0..127). Default = A3 (57) — the grave-bell home.
    pub root: u8,
    /// Active scale; gates every emitted note in-key. Default = A Dorian.
    pub scale: ScaleMask,
    /// Brush timbre preset. Default = Hearth (warm body, rich low harmonics).
    pub voice: VoicePreset,
}

impl Default for HarmonicBrushState {
    fn default() -> Self {
        Self { root: 57, scale: A_DORIAN, voice: VoicePreset::Hearth }
    }
}

impl HarmonicBrushState {
    /// Snap `midi_note` to the nearest in-scale pitch at or below it.
    /// Scans down up to one octave; falls back to the tonic if nothing found.
    pub fn nearest_in_scale(self, mut midi_note: u8) -> u8 {
        for _ in 0..12 {
            if self.scale.is_member(midi_note) {
                return midi_note;
            }
            midi_note = midi_note.saturating_sub(1);
        }
        self.root
    }

    /// MIDI notes (up to 2) for a given phrase. Silent phrases return `[None, None]`.
    /// All returned notes are guaranteed in-scale via `nearest_in_scale`.
    pub fn phrase_notes(self, phrase: BardPhraseKind) -> [Option<u8>; 2] {
        match phrase {
            BardPhraseKind::MinorThirdDescent => {
                let hi = self.nearest_in_scale(self.root);
                // Minor-third interval: 3 semitones below root, then snapped in-scale.
                let lo = self.nearest_in_scale(hi.saturating_sub(3));
                [Some(hi), Some(lo)]
            }
            BardPhraseKind::SilentHold | BardPhraseKind::RefusalRest => [None, None],
        }
    }

    /// Root frequency in millihertz — for use at the DSP f32 boundary.
    #[inline]
    pub fn root_mhz(self) -> u32 {
        note_to_mhz(self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_root_is_a3_and_scale_contains_grave_bell_dyad() {
        let h = HarmonicBrushState::default();
        assert_eq!(h.root, 57, "A3 = MIDI 57");
        // A Dorian must contain both notes of the grave-bell dyad.
        assert!(h.scale.is_member(57), "A3 (root) must be in A Dorian");
        assert!(h.scale.is_member(54), "F#3 (minor third below A3) must be in A Dorian");
    }

    #[test]
    fn minor_third_descent_default_yields_a3_and_f_sharp3() {
        let h = HarmonicBrushState::default();
        let [hi, lo] = h.phrase_notes(BardPhraseKind::MinorThirdDescent);
        assert_eq!(hi, Some(57), "hi = A3 (MIDI 57)");
        assert_eq!(lo, Some(54), "lo = F#3 (MIDI 54, minor third below A3)");
    }

    #[test]
    fn all_emitted_notes_are_in_scale() {
        let h = HarmonicBrushState::default();
        for note in h.phrase_notes(BardPhraseKind::MinorThirdDescent).into_iter().flatten() {
            assert!(h.scale.is_member(note), "note MIDI {note} must be in the active scale");
        }
    }

    #[test]
    fn silent_phrases_yield_no_notes() {
        let h = HarmonicBrushState::default();
        assert_eq!(h.phrase_notes(BardPhraseKind::SilentHold), [None, None]);
        assert_eq!(h.phrase_notes(BardPhraseKind::RefusalRest), [None, None]);
    }

    #[test]
    fn root_mhz_matches_a4_440000() {
        let mut h = HarmonicBrushState::default();
        h.root = 69; // A4
        assert_eq!(h.root_mhz(), 440_000);
    }

    #[test]
    fn nearest_in_scale_snaps_g_sharp_to_g() {
        // G#3 = MIDI 56 (pc 8) is not in A Dorian → snaps down to G3 (MIDI 55, pc 7).
        let h = HarmonicBrushState::default();
        assert_eq!(h.nearest_in_scale(56), 55, "G#3 snaps down to G3");
    }

    #[test]
    fn changing_root_shifts_phrase_notes() {
        // Root = C4 (60, pc 0); A Dorian still applies; nearest_in_scale(60) = 60 (C is in A Dorian).
        // Minor third below: 60-3=57 (A3, pc 9) — also in A Dorian.
        let mut h = HarmonicBrushState::default();
        h.root = 60;
        let [hi, lo] = h.phrase_notes(BardPhraseKind::MinorThirdDescent);
        assert_eq!(hi, Some(60));
        assert_eq!(lo, Some(57));
    }
}
