//! Interactive Camelot Harmonic Stream Router & Multi-Channel Mute Governor.
//!
//! Provides:
//! 1. `CamelotKey`: 1A..12B Camelot harmonic wheel key tracking, compatibility, and wheel modulation.
//! 2. `HarmonicPreset`: Four distinct selectable presets:
//!    - 12-TET Pentatonic
//!    - Astrolabe Monzo11 Just Intonation
//!    - S13 Trit MetaRouter
//!    - Minor Blues
//! 3. `StreamKind` & `StreamClassifier`: Thinking (<think> / reasoning tokens) vs. Chat / output stream classification.
//! 4. `MuteGovernor`: Lock-free global mute (`MUTE_AUDIO: AtomicBool`) and env/CLI controls.
//! 5. `InteractiveHarmonicRouter`: Statefully processes token streams into harmonic voice notes.

use core::sync::atomic::{AtomicBool, Ordering};
use crate::scale_voice::{note_to_mhz, raw_word_pitch, VoicePreset};
use crate::starmonics::nearest_star_monzo;

/// Global atomic mute governor.
pub static MUTE_AUDIO: AtomicBool = AtomicBool::new(false);

/// Query whether global audio singing is muted.
#[inline]
pub fn is_audio_muted() -> bool {
    MUTE_AUDIO.load(Ordering::Relaxed)
}

/// Set global audio singing mute state.
#[inline]
pub fn set_audio_muted(muted: bool) {
    MUTE_AUDIO.store(muted, Ordering::Relaxed);
}

/// Toggle global audio singing mute state, returning the new muted state.
#[inline]
pub fn toggle_audio_mute() -> bool {
    let prev = MUTE_AUDIO.fetch_xor(true, Ordering::Relaxed);
    !prev
}

/// Initialize mute state from environment variables (`FORGE_MUTE=1` or `FORGE_SING=0`).
pub fn init_mute_from_env() -> bool {
    if let Ok(v) = std::env::var("FORGE_MUTE") {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            set_audio_muted(true);
            return true;
        }
    }
    if let Ok(v) = std::env::var("FORGE_SING") {
        if v == "0" || v.eq_ignore_ascii_case("false") {
            set_audio_muted(true);
            return true;
        }
    }
    is_audio_muted()
}

/// Camelot Harmonic Wheel Key (1A..12B).
///
/// Number is 1..=12.
/// `is_minor` is `true` for 'A' (minor), `false` for 'B' (major).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CamelotKey {
    /// Number around the Camelot circle (1..=12).
    pub number: u8,
    /// Mode: `true` for minor (A), `false` for major (B).
    pub is_minor: bool,
}

impl CamelotKey {
    /// Canonical default Camelot key: 8A (A minor, relative to 8B C major).
    pub const DEFAULT_8A: Self = Self { number: 8, is_minor: true };
    /// 8B (C major).
    pub const DEFAULT_8B: Self = Self { number: 8, is_minor: false };

    /// Create a new Camelot key, clamping number to 1..=12.
    pub const fn new(number: u8, is_minor: bool) -> Self {
        let n = if number == 0 {
            12
        } else if number > 12 {
            ((number - 1) % 12) + 1
        } else {
            number
        };
        Self { number: n, is_minor }
    }

    /// Parse a Camelot key from a string (e.g., "8A", "8a", "12B", "12b").
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.len() < 2 || trimmed.len() > 3 {
            return None;
        }
        let (num_str, letter) = trimmed.split_at(trimmed.len() - 1);
        let num: u8 = num_str.parse().ok()?;
        if !(1..=12).contains(&num) {
            return None;
        }
        match letter {
            "A" | "a" => Some(Self { number: num, is_minor: true }),
            "B" | "b" => Some(Self { number: num, is_minor: false }),
            _ => None,
        }
    }

    /// Derive a Camelot key from a CATALOG_16 star index ($idx \in [0, 15]$).
    ///
    /// - Pitch Class: `pc = (9 - idx).rem_euclid(12)`
    /// - Mode: Minor (A) for $idx < 12$, Major (B) for $idx \ge 12$
    /// - Camelot Number: Derived via inverse pitch class map (8A for Sirius at $idx=0$).
    pub fn from_star_idx(idx: usize) -> Option<Self> {
        if idx >= 16 {
            return None;
        }
        let pc = (9 - (idx as i32)).rem_euclid(12);
        let is_minor = idx < 12;
        let n = if is_minor {
            ((pc - 9) * 7 + 8).rem_euclid(12)
        } else {
            (pc * 7 + 8).rem_euclid(12)
        };
        let number = if n == 0 { 12 } else { n as u8 };
        Some(Self { number, is_minor })
    }

    /// Format into a fixed 4-byte array (e.g. `b"8A\0\0"` or `b"12B\0"`), returning the string slice.
    pub fn format_fixed<'a>(&self, buf: &'a mut [u8; 4]) -> &'a str {
        let num = self.number;
        let letter = if self.is_minor { b'A' } else { b'B' };
        let len = if num >= 10 {
            buf[0] = b'1';
            buf[1] = b'0' + (num - 10);
            buf[2] = letter;
            3
        } else {
            buf[0] = b'0' + num;
            buf[1] = letter;
            2
        };
        core::str::from_utf8(&buf[..len]).unwrap_or("8A")
    }

    /// Pitch class of the tonic (0=C, 1=C#, 2=D, 3=Eb, 4=E, 5=F, 6=F#, 7=G, 8=Ab, 9=A, 10=Bb, 11=B).
    ///
    /// Wheel mapping (circle of fifths):
    /// 8B = C major (0), 8A = A minor (9)
    /// Moving +1 on the wheel adds +7 semitones (fifth up).
    pub const fn tonic_pitch_class(&self) -> u8 {
        let base_offset = (self.number as i32 - 8) * 7;
        let mode_offset = if self.is_minor { 9 } else { 0 };
        (base_offset + mode_offset).rem_euclid(12) as u8
    }

    /// Check if this key is harmonically compatible with another key:
    /// - Same key
    /// - Relative major/minor (same number, opposite mode)
    /// - Adjacent on the wheel (±1, same mode, wrapping 12 <-> 1)
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }
        if self.number == other.number {
            return true;
        }
        if self.is_minor == other.is_minor {
            let diff = (self.number as i8 - other.number as i8).unsigned_abs();
            if diff == 1 || diff == 11 {
                return true;
            }
        }
        false
    }

    /// The 4 compatible keys: `[Same, -1 (subdominant), +1 (dominant), Relative]`.
    pub fn compatible_keys(&self) -> [Self; 4] {
        let prev_num = if self.number == 1 { 12 } else { self.number - 1 };
        let next_num = if self.number == 12 { 1 } else { self.number + 1 };
        [
            *self,
            Self { number: prev_num, is_minor: self.is_minor },
            Self { number: next_num, is_minor: self.is_minor },
            Self { number: self.number, is_minor: !self.is_minor },
        ]
    }

    /// Step by `delta` positions on the wheel (wrapping around 1..=12).
    pub fn step_wheel(&self, delta: i8) -> Self {
        let n = ((self.number as i16 - 1 + delta as i16).rem_euclid(12) + 1) as u8;
        Self { number: n, is_minor: self.is_minor }
    }

    /// Toggle relative mode (A <-> B).
    pub fn toggle_mode(&self) -> Self {
        Self { number: self.number, is_minor: !self.is_minor }
    }

    /// Root MIDI note for a target octave offset (0 = mid octave ~C4=60).
    pub fn root_midi_note(&self, octave_shift: i32) -> u8 {
        let base = 60 + self.tonic_pitch_class() as i32 + (octave_shift * 12);
        base.clamp(0, 127) as u8
    }

    /// Five pentatonic scale notes anchored to this Camelot key in the specified octave.
    pub fn pentatonic_notes(&self, octave_shift: i32) -> [u8; 5] {
        let root = self.root_midi_note(octave_shift);
        if self.is_minor {
            // Minor pentatonic: 0, 3, 5, 7, 10 semitones
            [
                root,
                (root + 3).min(127),
                (root + 5).min(127),
                (root + 7).min(127),
                (root + 10).min(127),
            ]
        } else {
            // Major pentatonic: 0, 2, 4, 7, 9 semitones
            [
                root,
                (root + 2).min(127),
                (root + 4).min(127),
                (root + 7).min(127),
                (root + 9).min(127),
            ]
        }
    }

    /// The seven-degree pentatonic span: [`pentatonic_notes`] plus the octave-up
    /// repeat of its first two degrees. `DEFAULT_8B.pentatonic_span_7(0)` is
    /// `PENTATONIC_C` exactly — the word-note canon in this key's own register.
    pub fn pentatonic_span_7(&self, octave_shift: i32) -> [u8; 7] {
        let p = self.pentatonic_notes(octave_shift);
        [p[0], p[1], p[2], p[3], p[4], p[0].saturating_add(12).min(127), p[1].saturating_add(12).min(127)]
    }

    /// The CATALOG_16 star this key belongs to — [`from_star_idx`]'s inverse (L07).
    /// `None` for the eight major keys no star occupies.
    pub fn star_idx(&self) -> Option<usize> {
        let pc = self.tonic_pitch_class() as i32;
        if self.is_minor {
            Some((9 - pc).rem_euclid(12) as usize)
        } else if (6..=9).contains(&pc) {
            Some((12 + (9 - pc)) as usize)
        } else {
            None
        }
    }

    /// Six minor blues scale notes anchored to this Camelot key in the specified octave.
    pub fn blues_notes(&self, octave_shift: i32) -> [u8; 6] {
        let root = self.root_midi_note(octave_shift);
        // Minor blues: 0, 3, 5, 6 (blue note), 7, 10
        [
            root,
            (root + 3).min(127),
            (root + 5).min(127),
            (root + 6).min(127),
            (root + 7).min(127),
            (root + 10).min(127),
        ]
    }
}

/// The four distinct selectable harmonic presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HarmonicPreset {
    /// 1. 12-TET Pentatonic: standard equal-tempered pentatonic modulated by active Camelot key.
    Pentatonic12Tet = 0,
    /// 2. Astrolabe Monzo11 Just Intonation: 16-star Monzo11 pure 5-limit intervals and microtones.
    AstrolabeMonzo11 = 1,
    /// 3. S13 Trit MetaRouter: balanced ternary (-1, 0, +1) trit-modulated registers and triads.
    S13TritMetaRouter = 2,
    /// 4. Minor Blues: hexatonic blues scale with dim-5th blue note.
    MinorBlues = 3,
}

impl HarmonicPreset {
    /// All four harmonic presets in canonical order.
    pub const ALL: [Self; 4] = [
        Self::Pentatonic12Tet,
        Self::AstrolabeMonzo11,
        Self::S13TritMetaRouter,
        Self::MinorBlues,
    ];

    /// Construct a preset from an index 0..3 (wrapping).
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Pentatonic12Tet,
            1 => Self::AstrolabeMonzo11,
            2 => Self::S13TritMetaRouter,
            _ => Self::MinorBlues,
        }
    }

    /// Advance to the next preset in sequence.
    pub fn next(&self) -> Self {
        Self::from_index(*self as usize + 1)
    }

    /// Descriptive name of the preset.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pentatonic12Tet => "12-TET Pentatonic",
            Self::AstrolabeMonzo11 => "Astrolabe Monzo11 Just Intonation",
            Self::S13TritMetaRouter => "S13 Trit MetaRouter",
            Self::MinorBlues => "Minor Blues",
        }
    }

    /// Short 3-4 character badge for HUD and telemetry.
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Pentatonic12Tet => "12T",
            Self::AstrolabeMonzo11 => "M11",
            Self::S13TritMetaRouter => "S13",
            Self::MinorBlues => "BLU",
        }
    }
}

/// Stream classification kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    /// High-octave, lighter Monzo11/Glass timbre, faster micro-pulses (40-60ms).
    Thinking,
    /// Grounded (0 register), resonant Camelot progression, standard cadence (90ms).
    Chat,
}

/// Stream classifier detecting thinking blocks (<think>, [thinking], etc.) across streaming tokens.
#[derive(Clone, Debug)]
pub struct StreamClassifier {
    /// Active classification kind for incoming tokens.
    pub current_kind: StreamKind,
    tag_buf: [u8; 32],
    tag_len: usize,
}

impl StreamClassifier {
    /// Construct a new stream classifier default to `StreamKind::Chat`.
    pub fn new() -> Self {
        Self {
            current_kind: StreamKind::Chat,
            tag_buf: [0u8; 32],
            tag_len: 0,
        }
    }

    /// Feed a byte slice / token chunk, updating stream classification state.
    pub fn feed(&mut self, chunk: &[u8]) -> StreamKind {
        for &b in chunk {
            if b == b'<' || b == b'[' {
                self.tag_buf[0] = b;
                self.tag_len = 1;
            } else if self.tag_len > 0 {
                if self.tag_len < self.tag_buf.len() {
                    self.tag_buf[self.tag_len] = b;
                    self.tag_len += 1;
                }
                if b == b'>' || b == b']' {
                    let tag = &self.tag_buf[..self.tag_len];
                    if tag.eq_ignore_ascii_case(b"<think>")
                        || tag.eq_ignore_ascii_case(b"[thinking]")
                        || tag.eq_ignore_ascii_case(b"<thought>")
                        || tag.eq_ignore_ascii_case(b"[thought]")
                        || tag.eq_ignore_ascii_case(b"<reasoning>")
                    {
                        self.current_kind = StreamKind::Thinking;
                    } else if tag.eq_ignore_ascii_case(b"</think>")
                        || tag.eq_ignore_ascii_case(b"[/thinking]")
                        || tag.eq_ignore_ascii_case(b"</thought>")
                        || tag.eq_ignore_ascii_case(b"[/thought]")
                        || tag.eq_ignore_ascii_case(b"</reasoning>")
                    {
                        self.current_kind = StreamKind::Chat;
                    }
                    self.tag_len = 0;
                }
            }
        }
        self.current_kind
    }
}

impl Default for StreamClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// A computed harmonic note event emitted by the stream router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HarmonicVoiceNote {
    /// Frequency in millihertz (integer equal-tempered or just-intonation).
    pub freq_mhz: u32,
    /// Note duration in milliseconds.
    pub duration_ms: u16,
    /// Timbre preset envelope.
    pub voice: VoicePreset,
    /// MIDI note representation.
    pub midi_note: u8,
    /// Stream kind (Thinking vs Chat).
    pub stream_kind: StreamKind,
    /// Active Camelot harmonic key.
    pub camelot_key: CamelotKey,
}

/// Interactive Camelot Harmonic Stream Router.
///
/// Modulates active key smoothly along compatible transitions, routes
/// words to pitch/timbre according to preset and stream classification.
#[derive(Clone, Debug)]
pub struct InteractiveHarmonicRouter {
    /// Active Camelot harmonic key.
    pub key: CamelotKey,
    /// Active harmonic preset.
    pub preset: HarmonicPreset,
    /// Stream classifier tracker.
    pub classifier: StreamClassifier,
    /// Number of words routed in current key before wheel modulation.
    pub words_in_key: u32,
}

impl InteractiveHarmonicRouter {
    /// Construct a new interactive harmonic router with initial key and preset.
    pub fn new(key: CamelotKey, preset: HarmonicPreset) -> Self {
        Self {
            key,
            preset,
            classifier: StreamClassifier::new(),
            words_in_key: 0,
        }
    }

    /// Cycle to the next harmonic preset.
    pub fn cycle_preset(&mut self) -> HarmonicPreset {
        self.preset = self.preset.next();
        self.preset
    }

    /// Explicitly set the active Camelot key.
    pub fn set_key(&mut self, key: CamelotKey) {
        self.key = key;
        self.words_in_key = 0;
    }

    /// Modulate active Camelot key along the wheel.
    pub fn step_key(&mut self, delta: i8) {
        self.key = self.key.step_wheel(delta);
        self.words_in_key = 0;
    }

    /// Modulate to relative major/minor.
    pub fn toggle_mode(&mut self) {
        self.key = self.key.toggle_mode();
        self.words_in_key = 0;
    }

    /// Feed a token chunk to update classifier.
    pub fn feed_chunk(&mut self, chunk: &[u8]) -> StreamKind {
        self.classifier.feed(chunk)
    }

    /// Route a single word into a `HarmonicVoiceNote`.
    pub fn route_word(&mut self, word: &[u8]) -> HarmonicVoiceNote {
        let stream = self.classifier.current_kind;
        let raw = raw_word_pitch(word);

        // Word count triggers smooth harmonic modulation across long streams
        self.words_in_key = self.words_in_key.saturating_add(1);
        if self.words_in_key > 32 {
            // Modulate +1 on wheel (circle of fifths lift)
            self.step_key(1);
        }

        let (octave_shift, dur_ms, default_voice) = match stream {
            StreamKind::Thinking => (1, 50u16, VoicePreset::Glass),
            StreamKind::Chat => (0, 90u16, VoicePreset::Hearth),
        };

        match self.preset {
            HarmonicPreset::Pentatonic12Tet => {
                let scale = self.key.pentatonic_notes(octave_shift);
                let note = *scale.iter().min_by_key(|&&n| (n as i32 - raw as i32).abs()).unwrap_or(&scale[0]);
                HarmonicVoiceNote {
                    freq_mhz: note_to_mhz(note),
                    duration_ms: dur_ms,
                    voice: default_voice,
                    midi_note: note,
                    stream_kind: stream,
                    camelot_key: self.key,
                }
            }
            HarmonicPreset::AstrolabeMonzo11 => {
                let star = nearest_star_monzo(note_to_mhz(raw.clamp(36, 96)));
                let mhz = if stream == StreamKind::Thinking {
                    star.milli_hz_12tet.saturating_mul(2)
                } else {
                    star.milli_hz_12tet
                };
                HarmonicVoiceNote {
                    freq_mhz: mhz,
                    duration_ms: dur_ms,
                    voice: VoicePreset::Glass,
                    midi_note: (raw % 12) + 60,
                    stream_kind: stream,
                    camelot_key: self.key,
                }
            }
            HarmonicPreset::S13TritMetaRouter => {
                // Balanced ternary trit routing: {-1, 0, +1}
                let trit = ((raw as i32 % 3) - 1) + octave_shift;
                let scale = self.key.pentatonic_notes(trit);
                let note = scale[(raw as usize) % scale.len()];
                HarmonicVoiceNote {
                    freq_mhz: note_to_mhz(note),
                    duration_ms: dur_ms,
                    voice: VoicePreset::Reed,
                    midi_note: note,
                    stream_kind: stream,
                    camelot_key: self.key,
                }
            }
            HarmonicPreset::MinorBlues => {
                let blues = self.key.blues_notes(octave_shift);
                let note = blues[(raw as usize) % blues.len()];
                HarmonicVoiceNote {
                    freq_mhz: note_to_mhz(note),
                    duration_ms: dur_ms,
                    voice: default_voice,
                    midi_note: note,
                    stream_kind: stream,
                    camelot_key: self.key,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camelot_parsing_and_formatting() {
        let k8a = CamelotKey::parse("8A").expect("parse 8A");
        assert_eq!(k8a.number, 8);
        assert!(k8a.is_minor);

        let mut buf = [0u8; 4];
        assert_eq!(k8a.format_fixed(&mut buf), "8A");

        let k12b = CamelotKey::parse("12b").expect("parse 12b");
        assert_eq!(k12b.number, 12);
        assert!(!k12b.is_minor);
        assert_eq!(k12b.format_fixed(&mut buf), "12B");
    }

    #[test]
    fn test_camelot_compatibility() {
        let k8a = CamelotKey::new(8, true);
        let k8b = CamelotKey::new(8, false);
        let k9a = CamelotKey::new(9, true);
        let k7a = CamelotKey::new(7, true);
        let k3b = CamelotKey::new(3, false);

        assert!(k8a.is_compatible_with(&k8a));
        assert!(k8a.is_compatible_with(&k8b)); // relative
        assert!(k8a.is_compatible_with(&k9a)); // +1
        assert!(k8a.is_compatible_with(&k7a)); // -1
        assert!(!k8a.is_compatible_with(&k3b)); // distant
    }

    #[test]
    fn test_stream_classifier() {
        let mut sc = StreamClassifier::new();
        assert_eq!(sc.current_kind, StreamKind::Chat);

        sc.feed(b"<think> Let us consider the harmonic structure");
        assert_eq!(sc.current_kind, StreamKind::Thinking);

        sc.feed(b" deeply. </think> Here is the answer.");
        assert_eq!(sc.current_kind, StreamKind::Chat);
    }

    #[test]
    fn test_mute_governor() {
        set_audio_muted(false);
        assert!(!is_audio_muted());
        assert!(toggle_audio_mute());
        assert!(is_audio_muted());
        assert!(!toggle_audio_mute());
        assert!(!is_audio_muted());
    }

    #[test]
    fn test_all_presets_route_cleanly() {
        let mut router = InteractiveHarmonicRouter::new(CamelotKey::DEFAULT_8A, HarmonicPreset::Pentatonic12Tet);
        for preset in HarmonicPreset::ALL {
            router.preset = preset;
            let note = router.route_word(b"sovereignty");
            assert!(note.freq_mhz > 0);
            assert!(note.duration_ms > 0);
        }
    }
}
