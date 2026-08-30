//! Music — the Camelot harmonic key wheel for the Atlas (harvested from
//! forge-audio vocal_studio/camelot). Integer key distance for harmonic mixing.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// A Camelot key: number 1..=12, letter 'A' (minor) or 'B' (major).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamelotKey {
    /// Camelot wheel position, 1..=12.
    pub number: u8,
    /// 'A' for minor key, 'B' for major key.
    pub letter: char,
}

impl CamelotKey {
    /// Construct a key with auto-clamped number (1..=12) and validated letter.
    pub fn new(number: u8, letter: char) -> Self {
        Self { number: number.clamp(1, 12), letter: if letter == 'B' { 'B' } else { 'A' } }
    }

    /// Harmonic distance: ring distance of the numbers (0..6) plus a letter
    /// penalty (0 same, 1 different). 0 = same key, <=1 = compatible mix.
    pub fn distance(&self, other: &CamelotKey) -> u8 {
        let a = self.number as i32;
        let b = other.number as i32;
        let raw = ((a - b).rem_euclid(12)).min((b - a).rem_euclid(12)) as u8;
        let letter_pen = if self.letter == other.letter { 0 } else { 1 };
        raw + letter_pen
    }

    /// Adjacent-or-relative keys mix cleanly.
    pub fn compatible(&self, other: &CamelotKey) -> bool {
        self.distance(other) <= 1
    }

    /// Camelot key name (number + letter, e.g., "8A").
    pub fn name(&self) -> String {
        format!("{}{}", self.number, self.letter)
    }
}

/// Bind a set of keys into a Music chapter (each key + its clean neighbours).
pub fn to_chapter(keys: &[CamelotKey], title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Music".into()));
    for k in keys {
        let mates: Vec<String> = keys
            .iter()
            .filter(|o| **o != *k && k.compatible(o))
            .map(|o| o.name())
            .collect();
        ch.add_lore(format!("{} mixes with {}", k.name(), mates.join(", ")));
    }
    ch
}

/// The twelve minor keys (the 'A' ring) as a seed.
pub fn minor_ring() -> Vec<CamelotKey> {
    (1..=12).map(|n| CamelotKey::new(n, 'A')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_zero_distance() {
        let k = CamelotKey::new(8, 'A');
        assert_eq!(k.distance(&k), 0);
        assert!(k.compatible(&k));
    }

    #[test]
    fn adjacent_and_relative_are_compatible() {
        let k = CamelotKey::new(8, 'A');
        assert!(k.compatible(&CamelotKey::new(9, 'A'))); // +1 same letter
        assert!(k.compatible(&CamelotKey::new(7, 'A'))); // -1 same letter
        assert!(k.compatible(&CamelotKey::new(8, 'B'))); // relative
        assert!(!k.compatible(&CamelotKey::new(2, 'A'))); // far
    }

    #[test]
    fn ring_wraps() {
        let k = CamelotKey::new(12, 'A');
        assert!(k.compatible(&CamelotKey::new(1, 'A'))); // 12 -> 1 wraps
    }

    #[test]
    fn chapter_lists_mixes() {
        let ch = to_chapter(&minor_ring(), "Keys");
        assert_eq!(ch.lore_count(), 12);
    }
}
