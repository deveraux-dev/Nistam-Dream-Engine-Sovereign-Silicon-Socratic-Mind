//! Ported verbatim from F:\NewRepo\crates\forge-calligraphy\src\syllabic_to_event.rs
//! (2026-08-17 truth-hunt lineage port — this file was declared in lib.rs by an
//! earlier pass of this same port but never actually written; filling the gap now).
//!
//! syllabic_to_event — Cree syllabic → MIDI event adapter (CR-5 crossroad).
//!
//! ## Thesis
//! Cree syllabics encode phoneme structure as glyph geometry:
//! - **Orientation** → vowel class (E/I/O/A) → pitch
//! - **Base shape** → onset consonant → octave/timbre offset
//! - **Superscript** → final consonant → release/reverb tail
//!
//! This maps directly to an audio envelope: onset → attack, vowel → sustained
//! pitch, final → release character. The learner SEEs the glyph, HEARS its
//! phoneme structure, and the seehear colour confirms the mapping.
//!
//! ## Design (Firewall Law compliant)
//! This module produces pure `(channel, note, velocity)` tuples — it does NOT
//! depend on `forge-midi`. The consumer (forge-studio / teacher overlay) passes
//! these tuples to `MidiOut::note_on`. This keeps forge-calligraphy's zero
//! engine-dep firewall intact.
//!
//! ## Pitch mapping (IPA-grounded, pentatonic-consonant)
//! Vowel class determines the base MIDI note (within C-major pentatonic):
//! - E (ê) → C4 (60) — the "front unrounded" question
//! - I (î) → E4 (64) — the "high front" brightness
//! - O (ô) → G4 (67) — the "mid back rounded" body
//! - A (â) → A4 (69) — the "open" root
//!
//! Onset consonant class shifts the octave:
//! - Vowel-only (no consonant) → octave 4 (base)
//! - Stops (p, t, k)          → octave 3 (foundation, percussive)
//! - Affricate (c/ch)         → octave 3 + 2 semitones (bright edge)
//! - Fricative (s)            → octave 5 (high, airy)
//! - Nasals (m, n)            → octave 3 (warm, resonant)
//! - Approximants (w, y)      → octave 4 (neutral glide)
//!
//! Final consonant → velocity reduction (softer release) + suggested reverb.
//!
//! ## Research sources
//! - Western Cree syllabics (Wikipedia): orientation = vowel, shape = consonant
//! - r12a.github.io Plains Cree orthography: complete UCAS CV grid
//! - Unicode 17.0 UCAS block chart: codepoint→name canonical mapping

use crate::cree_syllabics::SyllabicEntry;

// ── Vowel classification ──────────────────────────────────────────────────────

/// Vowel classes in the Cree syllabary (determined by glyph orientation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VowelClass {
    /// ê — front unrounded vowel
    E,
    /// î — high front vowel
    I,
    /// ô — mid back rounded vowel
    O,
    /// â — open vowel
    A,
    /// Long variant (dot above) — same pitch, longer duration
    Long(Box_),
    /// No vowel (final consonant / non-syllabic)
    None,
}

/// Newtype to allow Long variant without heap. We encode the inner class as u8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box_(pub u8); // 0=E, 1=I, 2=O, 3=A

/// Consonant onset classes (base shape of the glyph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnsetClass {
    /// Pure vowel (ᐁ ᐃ ᐅ ᐊ series)
    Vowel,
    /// Stops: p (ᐸ), t (ᑕ), k (ᑲ)
    Stop,
    /// Affricate: c/ch (ᒐ)
    Affricate,
    /// Fricative: s (ᓴ)
    Fricative,
    /// Nasals: m (ᒪ), n (ᓇ)
    Nasal,
    /// Approximants: w (ᐘ), y (ᔭ)
    Approximant,
    /// Alphabetic consonant (ᐦ h, ᕒ r, ᓬ l) or final
    Alphabetic,
    /// Final/superscript consonant (no onset)
    Final,
}

// ── MIDI note constants (C-major pentatonic) ──────────────────────────────────

/// Base MIDI notes for each vowel class (octave 4).
const NOTE_E: u8 = 60; // C4
const NOTE_I: u8 = 64; // E4
const NOTE_O: u8 = 67; // G4
const NOTE_A: u8 = 69; // A4

/// Octave offsets for onset consonant classes.
const OCTAVE_OFFSET_STOP: i8 = -12;      // one octave down
const OCTAVE_OFFSET_AFFRICATE: i8 = -10; // octave down + 2 semitones
const OCTAVE_OFFSET_FRICATIVE: i8 = 12;  // one octave up
const OCTAVE_OFFSET_NASAL: i8 = -12;     // one octave down (warm)
const OCTAVE_OFFSET_APPROX: i8 = 0;      // same octave (glide)
const OCTAVE_OFFSET_VOWEL: i8 = 0;       // same octave

// ── Public API ────────────────────────────────────────────────────────────────

/// MIDI event data for a single syllabic character.
/// Produced here, consumed by `forge-midi::midi_out::MidiOut` (or any MIDI sink).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyllabicMidiEvent {
    /// MIDI channel (0 = pitched melodic, 9 = drum).
    pub channel: u8,
    /// MIDI note number (0..127).
    pub note: u8,
    /// Velocity (0..127). Finals get softer velocity.
    pub velocity: u8,
    /// Suggested duration in milliseconds (for the teacher's note-off timer).
    pub duration_ms: u16,
    /// True if this is a final/superscript (coda) character — suggests reverb.
    pub is_final: bool,
}

/// Convert a syllabic entry into a MIDI event for the teacher.
///
/// Returns `None` for punctuation (᙮) and the hyphen (᐀) which have no
/// phonemic content.
pub fn syllabic_to_event(entry: &SyllabicEntry) -> Option<SyllabicMidiEvent> {
    let (cp, _ch, name) = *entry;

    // Skip punctuation
    if name.contains("HYPHEN") || name.contains("FULL STOP") {
        return None;
    }

    let onset = classify_onset(name);
    let vowel = classify_vowel(name, cp);

    // Finals: short percussive tap on channel 9 (drum)
    if matches!(onset, OnsetClass::Final) {
        return Some(SyllabicMidiEvent {
            channel: 9,
            note: final_to_drum_note(name),
            velocity: 80,
            duration_ms: 50,
            is_final: true,
        });
    }

    // Alphabetic consonants (h, r, l): short pitched note, mid velocity
    if matches!(onset, OnsetClass::Alphabetic) {
        return Some(SyllabicMidiEvent {
            channel: 0,
            note: 55, // G3 — a neutral "spoken consonant"
            velocity: 60,
            duration_ms: 100,
            is_final: false,
        });
    }

    // CV syllables and pure vowels: pitched note
    let base = match vowel {
        VowelClass::E => NOTE_E,
        VowelClass::I => NOTE_I,
        VowelClass::O => NOTE_O,
        VowelClass::A => NOTE_A,
        VowelClass::Long(inner) => match inner.0 {
            0 => NOTE_E,
            1 => NOTE_I,
            2 => NOTE_O,
            _ => NOTE_A,
        },
        VowelClass::None => NOTE_A, // fallback for unclassifiable
    };

    let offset = match onset {
        OnsetClass::Stop => OCTAVE_OFFSET_STOP,
        OnsetClass::Affricate => OCTAVE_OFFSET_AFFRICATE,
        OnsetClass::Fricative => OCTAVE_OFFSET_FRICATIVE,
        OnsetClass::Nasal => OCTAVE_OFFSET_NASAL,
        OnsetClass::Approximant => OCTAVE_OFFSET_APPROX,
        OnsetClass::Vowel => OCTAVE_OFFSET_VOWEL,
        _ => 0,
    };

    let note = (base as i8 + offset).clamp(0, 127) as u8;

    let is_long = matches!(vowel, VowelClass::Long(_));
    let duration_ms = if is_long { 600 } else { 300 };
    let velocity = if is_long { 100 } else { 90 };

    Some(SyllabicMidiEvent {
        channel: 0,
        note,
        velocity,
        duration_ms,
        is_final: false,
    })
}

/// Extract the vowel class from a UCAS character name.
///
/// Delegates to the structured [`crate::phonology`] decoder (which parses the
/// canonical name into a [`Phoneme`](crate::phonology::Phoneme) once) and projects its `(vowel, long)` onto
/// the pitch-pipeline's [`VowelClass`]. Replaced the hand-rolled `ends_with` chain
/// 2026-07-11: the structured decoder is proven against the whole core-Cree grid,
/// so the pitch mapping now rides that same oracle. Diphthongs (ai/oy/ay) project
/// onto their nearest cardinal so every syllable still sounds.
pub fn classify_vowel(name: &str, _cp: u32) -> VowelClass {
    use crate::phonology::{parse_name, Vowel};
    let Some(p) = parse_name(name) else {
        return VowelClass::None;
    };
    let cardinal = match p.vowel {
        Vowel::E => 0u8,
        Vowel::I => 1,
        Vowel::O => 2,
        Vowel::A | Vowel::Ai | Vowel::Ay => 3, // open nucleus → A pitch
        Vowel::Oy => 2,                        // back nucleus → O pitch
    };
    if p.long {
        return VowelClass::Long(Box_(cardinal));
    }
    match cardinal {
        0 => VowelClass::E,
        1 => VowelClass::I,
        2 => VowelClass::O,
        _ => VowelClass::A,
    }
}

/// Classify the onset consonant from the character name.
///
/// Delegates to [`crate::phonology`]: parses the name into a [`Phoneme`](crate::phonology::Phoneme) and maps
/// its [`Consonant`](crate::phonology::Consonant) onto the pitch pipeline's
/// articulatory [`OnsetClass`]. Names that are not a plain CV/V syllable fall back
/// to structural [`Role`](crate::phonology::Role): finals → `Final`, bare-consonant
/// codas → `Alphabetic`, everything else → `Vowel` (the neutral pitch base).
pub fn classify_onset(name: &str) -> OnsetClass {
    use crate::phonology::{parse_name, standalone_coda, Consonant};

    if let Some(p) = parse_name(name) {
        return match p.consonant {
            Consonant::None => OnsetClass::Vowel,
            Consonant::P | Consonant::T | Consonant::K => OnsetClass::Stop,
            Consonant::C => OnsetClass::Affricate,
            Consonant::S | Consonant::Sh => OnsetClass::Fricative,
            Consonant::M | Consonant::N => OnsetClass::Nasal,
            Consonant::Y | Consonant::W => OnsetClass::Approximant,
            Consonant::L | Consonant::R | Consonant::H | Consonant::Th => OnsetClass::Alphabetic,
            Consonant::Other => OnsetClass::Vowel,
        };
    }

    // Non-CV glyphs: classify structurally.
    if name.contains("FINAL") {
        return OnsetClass::Final;
    }
    if standalone_coda(name).is_some() {
        return OnsetClass::Alphabetic; // a spoken bare consonant
    }
    OnsetClass::Vowel
}

/// Map final consonant name to a GM drum note for percussive articulation.
fn final_to_drum_note(name: &str) -> u8 {
    let upper = name.to_ascii_uppercase();
    if upper.contains("ACUTE") {
        // t-final → rim shot
        37
    } else if upper.contains("GRAVE") {
        // k-final → bass drum
        36
    } else if upper.contains("TOP HALF RING") {
        // s-final → hi-hat closed
        42
    } else if upper.contains("RIGHT HALF RING") {
        // n-final → snare ghost
        38
    } else if upper.contains("RING") && !upper.contains("HALF") {
        // w-final → tambourine
        54
    } else if upper.contains("HORIZONTAL") {
        // c-final → wood block
        76
    } else if upper.contains("BOTTOM HALF") {
        // p-final → bass drum soft
        35
    } else if upper.contains("MIDDLE DOT") {
        // m-final → muted triangle
        80
    } else {
        // default percussion
        39 // hand clap
    }
}

// ── Batch helpers ─────────────────────────────────────────────────────────────

/// Convert a string of Cree syllabics into a sequence of MIDI events.
/// Non-syllabic characters and spaces are skipped.
pub fn text_to_events(text: &str) -> Vec<SyllabicMidiEvent> {
    text.chars()
        .filter_map(|ch| {
            crate::cree_syllabics::by_char(ch)
                .and_then(syllabic_to_event)
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cree_syllabics;

    #[test]
    fn vowel_e_maps_to_c4() {
        // ᐁ = CANADIAN SYLLABICS E
        let entry = cree_syllabics::by_char('ᐁ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.note, 60); // C4
        assert_eq!(ev.channel, 0);
    }

    #[test]
    fn vowel_i_maps_to_e4() {
        // ᐃ = CANADIAN SYLLABICS I
        let entry = cree_syllabics::by_char('ᐃ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.note, 64); // E4
    }

    #[test]
    fn vowel_o_maps_to_g4() {
        // ᐅ = CANADIAN SYLLABICS O
        let entry = cree_syllabics::by_char('ᐅ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.note, 67); // G4
    }

    #[test]
    fn vowel_a_maps_to_a4() {
        // ᐊ = CANADIAN SYLLABICS A
        let entry = cree_syllabics::by_char('ᐊ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.note, 69); // A4
    }

    #[test]
    fn stop_onset_drops_octave() {
        // ᐸ = CANADIAN SYLLABICS PA → A4 - 12 = A3 (57)
        let entry = cree_syllabics::by_char('ᐸ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.note, 57); // A3 (A4=69 - 12)
    }

    #[test]
    fn final_goes_to_drum_channel() {
        // ᐟ = CANADIAN SYLLABICS FINAL ACUTE (t-final)
        let entry = cree_syllabics::by_char('ᐟ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.channel, 9); // drum channel
        assert!(ev.is_final);
    }

    #[test]
    fn long_vowel_has_longer_duration() {
        // ᐄ = CANADIAN SYLLABICS II (long I)
        let entry = cree_syllabics::by_char('ᐄ').unwrap();
        let ev = syllabic_to_event(entry).unwrap();
        assert_eq!(ev.duration_ms, 600);
        assert_eq!(ev.velocity, 100);
    }

    #[test]
    fn hyphen_returns_none() {
        // ᐀ = CANADIAN SYLLABICS HYPHEN
        let entry = cree_syllabics::by_char('᐀').unwrap();
        assert!(syllabic_to_event(entry).is_none());
    }

    #[test]
    fn text_to_events_parses_word() {
        // ᓀᐦᐃᔭᐍᐏᐣ = nêhiyawêwin (the Cree word for "Cree language")
        let events = text_to_events("ᓀᐦᐃᔭᐍᐏᐣ");
        assert!(!events.is_empty());
        // Should have events for each syllabic character
        assert!(events.len() >= 5);
    }
}
