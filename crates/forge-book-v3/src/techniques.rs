//! Techniques — use-based mastery, harvested from ironroot skill_book. Mastery is
//! permyriad (0..10000); it grows with use, novelty rewards, repetition soft-caps.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::mulberry::fnv1a64_str;
use serde::{Deserialize, Serialize};

/// A learnable technique — a craft the author masters by doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Technique {
    /// Name of the technique.
    pub name: String,
    /// Mastery level in permyriad (0..10000).
    pub mastery_pmy: u16,
    /// Total number of practice repetitions.
    pub reps: u32,
    /// Hash of the last context used in practice.
    pub last_use_hash: u64,
}

impl Technique {
    /// Create a new technique with zero mastery.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), mastery_pmy: 0, reps: 0, last_use_hash: 0 }
    }

    /// Practice once at `difficulty_pmy` in `context`; returns mastery gained.
    /// Novel context grants full gain; repetition damps it; gain shrinks toward
    /// the cap (headroom-scaled). Deterministic — same inputs, same growth.
    pub fn practice(&mut self, difficulty_pmy: u16, context: &str) -> u16 {
        let ctx = fnv1a64_str(context);
        let novel = ctx != self.last_use_hash;
        self.last_use_hash = ctx;
        self.reps = self.reps.saturating_add(1);

        let headroom = 10_000u32.saturating_sub(self.mastery_pmy as u32);
        let rep_damp = 1 + (self.reps / 32);
        let base = ((difficulty_pmy as u32 * headroom) / 10_000) / rep_damp;
        let gain = if novel { base } else { base / 2 };
        let gain = gain.min(headroom) as u16;
        self.mastery_pmy = self.mastery_pmy.saturating_add(gain);
        gain
    }

    /// True if mastery has reached 10000 (grandmaster threshold).
    pub fn is_grandmaster(&self) -> bool {
        self.mastery_pmy >= 10_000
    }

    /// Mastery band label.
    pub fn grade(&self) -> &'static str {
        match self.mastery_pmy {
            0..=1666 => "Novice",
            1667..=3332 => "Apprentice",
            3333..=4999 => "Journeyman",
            5000..=6666 => "Adept",
            6667..=9999 => "Master",
            _ => "Grandmaster",
        }
    }
}

/// The author's technique book.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechniqueBook {
    /// List of learned techniques.
    pub techniques: Vec<Technique>,
}

impl TechniqueBook {
    /// Create a new empty technique book.
    pub fn new() -> Self {
        Self::default()
    }
    /// Learn a new technique and return its index.
    pub fn learn(&mut self, name: impl Into<String>) -> usize {
        let i = self.techniques.len();
        self.techniques.push(Technique::new(name));
        i
    }
    /// Get mutable reference to a technique by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Technique> {
        self.techniques.iter_mut().find(|t| t.name == name)
    }
    /// Number of techniques in the book.
    pub fn len(&self) -> usize {
        self.techniques.len()
    }
    /// True if the book contains no techniques.
    pub fn is_empty(&self) -> bool {
        self.techniques.is_empty()
    }
    /// Average mastery across the book, in permyriad.
    pub fn mastery_total(&self) -> u16 {
        if self.techniques.is_empty() {
            return 0;
        }
        let sum: u32 = self.techniques.iter().map(|t| t.mastery_pmy as u32).sum();
        (sum / self.techniques.len() as u32) as u16
    }
    /// Bind the technique book into a Learning chapter (grade per technique).
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Learning);
        for t in &self.techniques {
            ch.add_lore(format!("{} — {} ({}pmy)", t.name, t.grade(), t.mastery_pmy));
        }
        ch
    }
}

/// The studio's real crafts — the techniques of building this engine.
pub fn studio_techniques() -> TechniqueBook {
    let mut b = TechniqueBook::new();
    for name in [
        "Vixicoat (clean rust before the coat)",
        "Orphan-wire (new primitive gets a live caller)",
        "Proof-ladder (unproven -> proven -> verified)",
        "Rivercanon (return proven work to the bed)",
        "Corpse-walk (harvest concepts, never port)",
        "Terse-prose (cut till the next cut drops signal)",
        "Quantizer-parity (the bucket edge IS the live threshold)",
    ] {
        b.learn(name);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practice_grows_mastery() {
        let mut t = Technique::new("Vixicoat");
        assert_eq!(t.grade(), "Novice");
        let mut ctx = 0;
        for _ in 0..200 {
            ctx += 1;
            t.practice(9000, &format!("job-{ctx}"));
        }
        assert!(t.mastery_pmy > 0);
        assert!(t.mastery_pmy <= 10_000);
        assert!(t.reps >= 200);
    }

    #[test]
    fn novelty_beats_repetition() {
        let mut novel = Technique::new("a");
        let mut same = Technique::new("b");
        for i in 0..20 {
            novel.practice(8000, &format!("ctx-{i}"));
            same.practice(8000, "ctx-fixed");
        }
        assert!(novel.mastery_pmy >= same.mastery_pmy);
    }

    #[test]
    fn mastery_is_bounded() {
        let mut t = Technique::new("x");
        for i in 0..100_000 {
            t.practice(10_000, &format!("c{i}"));
        }
        assert!(t.mastery_pmy <= 10_000);
    }

    #[test]
    fn studio_book_binds_to_learning() {
        let b = studio_techniques();
        assert_eq!(b.len(), 7);
        let ch = b.to_chapter("Crafts");
        assert_eq!(ch.section, AtlasSection::Learning);
        assert_eq!(ch.lore_count(), 7);
    }
}
