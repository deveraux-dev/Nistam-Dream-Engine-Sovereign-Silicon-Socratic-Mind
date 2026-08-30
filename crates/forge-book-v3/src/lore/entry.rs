//! LineEntry — one spoken line or one static-lore slot.
//!
//! Holds the typed text (source of truth), per-character emphasis derived
//! from Wacom pen pressure at authoring time, the line's pace (derived from
//! stroke speed), and optional ink-segment annotations that ride alongside
//! the text without ever being parsed into it (no OCR — see spec §4).

use serde::{Deserialize, Serialize};

/// One pen-down → pen-up annotation stroke. Stored alongside (not over) the
/// typed text. Presentational + audit-bearing; not parsed back into text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InkSegment {
    /// Captured points across the stroke.
    pub points: Vec<InkPoint>,
    /// Permyriad. Average stroke speed across this segment (used to drive
    /// the line's `line_pace` once aggregated).
    pub avg_speed_permyriad: u16,
    /// Permyriad. Peak pressure observed within this segment.
    pub max_pressure_permyriad: u16,
}

/// One sample inside an [`InkSegment`]. Integer-only — MilliUnit positions
/// (`1000 = 1 unit`), Permyriad pressure, ms-since-segment-start timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InkPoint {
    /// MilliUnit X. Relative to the line's text-box origin.
    pub x_milli: i64,
    /// MilliUnit Y.
    pub y_milli: i64,
    /// Permyriad. Sampled pen pressure at this point.
    pub pressure_permyriad: u16,
    /// Milliseconds since the segment's pen-down moment.
    pub t_ms_since_pen_down: u32,
}

/// One authored line — a spoken line in a [`DialogueTree`] or a slot in a
/// [`LoreCodex`]. The typed `text` is the source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineEntry {
    /// `blake3_8` stable identity. Compute via `forge_lore::id_of(...)` of a
    /// stable composite key like `format!("{}:{}", tree_id, node_id)`.
    pub line_id: u64,
    /// Reference to a [`Voice`](crate::lore::Voice). Zero means "unset" — fails the
    /// missing-voice gate.
    pub voice_id: u64,
    /// Typed text, UTF-8, source of truth. Never derived from `ink_segments`.
    pub text: String,
    /// Permyriad per glyph cluster. **Invariant:** `len() == text.chars().count()`.
    /// The editor re-derives this on every text edit; lint checks the invariant.
    pub per_char_emphasis: Vec<u16>,
    /// Permyriad. Line-level pace; derived from average stroke speed at
    /// authoring time, or [`Voice::default_pace`](crate::lore::Voice::default_pace)
    /// if no pen samples were recorded.
    pub line_pace: u16,
    /// Annotation track. Empty = author didn't use the pen on this line.
    pub ink_segments: Vec<InkSegment>,
    /// `u64` hashes of dialogue tags this line asserts. Consumed by
    /// `HarmonicDialogueCue::required_dialogue_tags` at runtime via the
    /// cue-derivation path.
    pub dialogue_tags: Vec<u64>,
    /// BCP-47 locale, e.g. `"en-CA"`. Default locale of `text`.
    pub locale: String,
}

impl LineEntry {
    /// Construct a line with nominal-emphasis defaults sized to `text`.
    /// Useful for tests and as a starting point in the editor before the
    /// author has annotated anything.
    pub fn new_with_defaults(line_id: u64, voice_id: u64, text: impl Into<String>) -> Self {
        let text = text.into();
        let char_count = text.chars().count();
        Self {
            line_id,
            voice_id,
            per_char_emphasis: vec![5000; char_count],
            text,
            line_pace: 5000,
            ink_segments: Vec::new(),
            dialogue_tags: Vec::new(),
            locale: "en-CA".to_string(),
        }
    }

    /// Does the per-char emphasis vector match the text's glyph-cluster
    /// length? Editor enforces this; lint blocks save if violated.
    pub fn emphasis_in_sync(&self) -> bool {
        self.per_char_emphasis.len() == self.text.chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_defaults_emphasis_matches_text_length() {
        let e = LineEntry::new_with_defaults(1, 2, "hello");
        assert_eq!(e.per_char_emphasis.len(), 5);
        assert!(e.emphasis_in_sync());
    }

    #[test]
    fn empty_text_yields_empty_emphasis() {
        let e = LineEntry::new_with_defaults(1, 2, "");
        assert!(e.per_char_emphasis.is_empty());
        assert!(e.emphasis_in_sync());
    }

    #[test]
    fn emphasis_in_sync_detects_drift() {
        let mut e = LineEntry::new_with_defaults(1, 2, "hello");
        e.text = "hello world".to_string();
        assert!(!e.emphasis_in_sync());
    }

    #[test]
    fn multi_byte_glyphs_count_correctly() {
        // 5 chars by char count, but more bytes.
        let e = LineEntry::new_with_defaults(1, 2, "héllo");
        assert_eq!(e.per_char_emphasis.len(), 5);
        assert!(e.emphasis_in_sync());
    }

    #[test]
    fn ink_point_is_integer_only() {
        let p = InkPoint {
            x_milli: 1500,
            y_milli: -2000,
            pressure_permyriad: 7500,
            t_ms_since_pen_down: 120,
        };
        // Compile-time check: no f32/f64 fields.
        let _ = p.x_milli + p.y_milli;
        assert_eq!(p.pressure_permyriad, 7500);
    }
}
