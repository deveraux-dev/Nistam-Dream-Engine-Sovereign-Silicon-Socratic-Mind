//! QWERTY piano keyboard layout constants for synth input via `TuiDriver`.
//!
//! Two-row piano layout (root = C4, MIDI note 60):
//!
//!   w  e     t  y  u     o  p        ← black keys (upper row)
//!  a  s  d  f  g  h  j  k  l  ;     ← white keys (lower row)
//!
//! Load `SYNTH_KEYBOARD_DEFAULT_TOML` via `MappingEngine::from_toml` and pair
//! it with a `TuiDriver` to get a playable keyboard.

use crate::tui_driver::push_key;
pub use crate::tui_driver::TuiKeyQueue;

/// QWERTY char → MIDI note number. C4 = 60, one octave + a major third.
///
/// White keys: a s d f g h j k l ;
/// Black keys: w e t y u o p
pub const QWERTY_PIANO_MAP: &[(char, u8)] = &[
    // ── white keys (lower row) ───────────────────────────────────────────────
    ('a', 60), // C4
    ('s', 62), // D4
    ('d', 64), // E4
    ('f', 65), // F4
    ('g', 67), // G4
    ('h', 69), // A4
    ('j', 71), // B4
    ('k', 72), // C5
    ('l', 74), // D5
    (';', 76), // E5
    // ── black keys (upper row) ───────────────────────────────────────────────
    ('w', 61), // C#4
    ('e', 63), // D#4
    ('t', 66), // F#4
    ('y', 68), // G#4
    ('u', 70), // A#4
    ('o', 73), // C#5
    ('p', 75), // D#5
];

/// Source ID format used by both the map and the MappingEngine TOML.
pub fn key_source_id(ch: char) -> String {
    format!("key:{}", ch)
}

/// MIDI note for a QWERTY key, if it is in the piano map.
pub fn key_to_note(ch: char) -> Option<u8> {
    QWERTY_PIANO_MAP.iter().find(|(c, _)| *c == ch).map(|(_, n)| *n)
}

/// Default `MappingEngine` TOML — all 17 piano keys bound to `"note:{midi}"` targets.
///
/// Load with `MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML)`.
/// Pair with a `TuiDriver` to get a playable keyboard.
pub const SYNTH_KEYBOARD_DEFAULT_TOML: &str = r#"device = "QWERTY Piano"

# ── white keys (lower row) ──────────────────────────────────────────────────
[[bind]]
source = "key:a"
target = "note:60"

[[bind]]
source = "key:s"
target = "note:62"

[[bind]]
source = "key:d"
target = "note:64"

[[bind]]
source = "key:f"
target = "note:65"

[[bind]]
source = "key:g"
target = "note:67"

[[bind]]
source = "key:h"
target = "note:69"

[[bind]]
source = "key:j"
target = "note:71"

[[bind]]
source = "key:k"
target = "note:72"

[[bind]]
source = "key:l"
target = "note:74"

[[bind]]
source = "key:;"
target = "note:76"

# ── black keys (upper row) ──────────────────────────────────────────────────
[[bind]]
source = "key:w"
target = "note:61"

[[bind]]
source = "key:e"
target = "note:63"

[[bind]]
source = "key:t"
target = "note:66"

[[bind]]
source = "key:y"
target = "note:68"

[[bind]]
source = "key:u"
target = "note:70"

[[bind]]
source = "key:o"
target = "note:73"

[[bind]]
source = "key:p"
target = "note:75"
"#;

/// Push all 17 piano-key presses into a `TuiKeyQueue` (useful for testing).
pub fn push_all_piano_keys(queue: &TuiKeyQueue) {
    for (ch, _) in QWERTY_PIANO_MAP {
        push_key(queue, *ch, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::MappingEngine;
    use crate::tui_driver::{TuiDriver, new_tui_queue, push_key};
    use crate::controller::ControllerDriver;

    #[test]
    fn piano_map_has_17_keys() {
        assert_eq!(QWERTY_PIANO_MAP.len(), 17);
    }

    #[test]
    fn piano_map_no_duplicate_chars() {
        let mut chars: Vec<char> = QWERTY_PIANO_MAP.iter().map(|(c, _)| *c).collect();
        let before = chars.len();
        chars.sort();
        chars.dedup();
        assert_eq!(chars.len(), before, "duplicate chars in QWERTY_PIANO_MAP");
    }

    #[test]
    fn piano_map_no_duplicate_notes() {
        let mut notes: Vec<u8> = QWERTY_PIANO_MAP.iter().map(|(_, n)| *n).collect();
        let before = notes.len();
        notes.sort();
        notes.dedup();
        assert_eq!(notes.len(), before, "duplicate MIDI notes in QWERTY_PIANO_MAP");
    }

    #[test]
    fn key_to_note_c4() {
        assert_eq!(key_to_note('a'), Some(60));
    }

    #[test]
    fn key_to_note_unmapped_returns_none() {
        assert_eq!(key_to_note('z'), None);
    }

    #[test]
    fn key_source_id_format() {
        assert_eq!(key_source_id('a'), "key:a");
        assert_eq!(key_source_id(';'), "key:;");
    }

    #[test]
    fn default_toml_parses_17_binds() {
        let engine = MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML)
            .expect("SYNTH_KEYBOARD_DEFAULT_TOML must parse");
        assert_eq!(engine.binds.len(), 17, "expect 17 binds");
    }

    #[test]
    fn toml_and_map_agree() {
        let engine = MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML).unwrap();
        for (ch, note) in QWERTY_PIANO_MAP {
            let src = key_source_id(*ch);
            let bind = engine.binds.iter().find(|b| b.source == src)
                .unwrap_or_else(|| panic!("no bind for {}", src));
            assert_eq!(bind.target, format!("note:{}", note),
                "target mismatch for {}", src);
        }
    }

    #[test]
    fn white_key_c4_routes_to_note_60() {
        let engine = MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML).unwrap();
        let bind = engine.binds.iter().find(|b| b.source == "key:a").unwrap();
        assert_eq!(bind.target, "note:60");
    }

    #[test]
    fn black_key_csharp4_routes_to_note_61() {
        let engine = MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML).unwrap();
        let bind = engine.binds.iter().find(|b| b.source == "key:w").unwrap();
        assert_eq!(bind.target, "note:61");
    }

    #[test]
    fn tui_driver_to_mapping_end_to_end() {
        let q = new_tui_queue();
        push_key(&q, 'a', true);

        let mut driver = TuiDriver::new(q);
        let mut engine = MappingEngine::from_toml(SYNTH_KEYBOARD_DEFAULT_TOML).unwrap();

        let events = driver.poll();
        let (_, actions) = engine.apply(&events[0]);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].target, "note:60");
    }

    #[test]
    fn push_all_piano_keys_fills_queue() {
        let q = new_tui_queue();
        push_all_piano_keys(&q);
        let mut driver = TuiDriver::new(q);
        assert_eq!(driver.poll().len(), 17);
    }

    #[test]
    fn notes_span_c4_to_e5() {
        let notes: Vec<u8> = QWERTY_PIANO_MAP.iter().map(|(_, n)| *n).collect();
        assert!(notes.contains(&60), "C4 (60) missing");
        assert!(notes.contains(&76), "E5 (76) missing");
        assert!(notes.iter().all(|&n| n >= 60 && n <= 76), "note out of C4-E5 range");
    }
}
