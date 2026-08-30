//! Authoring — the scribe: typed text + nib pressure -> per-char emphasis, with a
//! live buffer. Harvested from the old lore-book panel keystroke model.

use crate::block::{Emphasis, TextBlock};
use crate::ink::{InkId, Quill};
use serde::{Deserialize, Serialize};

/// Average pen pressure (permyriad) -> an emphasis band.
fn band(avg_pmy: u16) -> Emphasis {
    match avg_pmy {
        0..=2500 => Emphasis::Whisper,
        2501..=6500 => Emphasis::Plain,
        6501..=8500 => Emphasis::Shout,
        _ => Emphasis::Chant,
    }
}

/// A live authoring buffer: one pressure sample per character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scribe {
    /// The accumulated text as characters are typed.
    pub buffer: String,
    /// Pressure reading in permyriad units for each character in the buffer.
    pub per_char_pmy: Vec<u16>,
    /// The current pen/nib, tracking ink color and cumulative pressure samples.
    pub quill: Quill,
}

impl Scribe {
    /// Construct a new empty scribe with the given ink color.
    pub fn new(ink: InkId) -> Self {
        Self { buffer: String::new(), per_char_pmy: Vec::new(), quill: Quill::new(ink) }
    }

    /// Type one character at nib `pressure_pmy`. Keeps buffer/pressure in sync.
    pub fn keystroke(&mut self, c: char, pressure_pmy: u16) {
        self.buffer.push(c);
        self.per_char_pmy.push(pressure_pmy.min(10_000));
        self.quill.press(pressure_pmy as u32);
    }

    /// Delete the trailing character (and its pressure sample).
    pub fn backspace(&mut self) -> bool {
        if self.buffer.pop().is_some() {
            self.per_char_pmy.pop();
            true
        } else {
            false
        }
    }

    /// Invariant: one pressure sample per character.
    pub fn in_sync(&self) -> bool {
        self.buffer.chars().count() == self.per_char_pmy.len()
    }

    /// Mean pressure across the buffer (permyriad).
    pub fn avg_pressure(&self) -> u16 {
        if self.per_char_pmy.is_empty() {
            return 0;
        }
        let sum: u32 = self.per_char_pmy.iter().map(|&p| p as u32).sum();
        (sum / self.per_char_pmy.len() as u32) as u16
    }

    /// Bake the buffer into an inked, emphasis-banded text block.
    pub fn to_block(&self) -> TextBlock {
        TextBlock::new(self.buffer.clone()).emphasize(band(self.avg_pressure())).inked(self.quill.ink)
    }

    /// Clear both the buffer and pressure history.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.per_char_pmy.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystrokes_stay_in_sync() {
        let mut s = Scribe::new(InkId::Blood);
        for (c, p) in [('h', 9000), ('i', 9500)] {
            s.keystroke(c, p);
        }
        assert_eq!(s.buffer, "hi");
        assert!(s.in_sync());
        assert!(s.backspace());
        assert_eq!(s.buffer, "h");
        assert!(s.in_sync());
    }

    #[test]
    fn pressure_bands_the_emphasis() {
        let mut soft = Scribe::new(InkId::Sepia);
        soft.keystroke('a', 1000);
        assert_eq!(soft.to_block().emphasis, Emphasis::Whisper);

        let mut hard = Scribe::new(InkId::Blood);
        hard.keystroke('A', 9500);
        assert_eq!(hard.to_block().emphasis, Emphasis::Chant);
        assert_eq!(hard.to_block().ink, InkId::Blood);
    }

    #[test]
    fn empty_scribe_is_synced() {
        let s = Scribe::new(InkId::Gold);
        assert!(s.in_sync());
        assert_eq!(s.avg_pressure(), 0);
    }
}
