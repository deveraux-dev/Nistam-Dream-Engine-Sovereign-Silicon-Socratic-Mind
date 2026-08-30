//! Story — a narrative arc: acts, each a list of beats carrying tension. The
//! spine of a dialogue/quest chapter; the climax is the peak-tension beat.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// One story beat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beat {
    /// Brief description of the beat.
    pub summary: String,
    /// Tension level as a permille (0-10000 = 0-100%).
    pub tension_pmy: u32,
}

/// One act — a titled run of beats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    /// Name of the act.
    pub title: String,
    /// List of beats in the act.
    pub beats: Vec<Beat>,
}

/// A whole narrative arc.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arc {
    /// List of acts in the arc.
    pub acts: Vec<Act>,
}

impl Arc {
    /// Create a new, empty arc.
    pub fn new() -> Self {
        Self::default()
    }
    /// Open an act; returns its id.
    pub fn act(&mut self, title: impl Into<String>) -> usize {
        let id = self.acts.len();
        self.acts.push(Act { title: title.into(), beats: Vec::new() });
        id
    }
    /// Add a beat to an act.
    pub fn beat(&mut self, act: usize, summary: impl Into<String>, tension_pmy: u32) {
        if let Some(a) = self.acts.get_mut(act) {
            a.beats.push(Beat { summary: summary.into(), tension_pmy: tension_pmy.min(10_000) });
        }
    }
    /// Total count of beats across all acts.
    pub fn beat_count(&self) -> usize {
        self.acts.iter().map(|a| a.beats.len()).sum()
    }
    /// The peak-tension beat across the whole arc.
    pub fn climax(&self) -> Option<&Beat> {
        self.acts.iter().flat_map(|a| &a.beats).max_by_key(|b| b.tension_pmy)
    }
    /// Convert the arc to a chapter with formatted lore.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Story".into()));
        for a in &self.acts {
            ch.add_lore(format!("== {} ==", a.title));
            for b in &a.beats {
                ch.add_lore(format!("  {} [{}]", b.summary, b.tension_pmy));
            }
        }
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arc() -> Arc {
        let mut a = Arc::new();
        let one = a.act("Setup");
        a.beat(one, "the road opens", 2000);
        let two = a.act("Confrontation");
        a.beat(two, "the warden wakes", 9000);
        a.beat(two, "the gate falls", 6000);
        a
    }

    #[test]
    fn climax_is_peak_tension() {
        assert_eq!(arc().climax().unwrap().summary, "the warden wakes");
    }

    #[test]
    fn counts_beats_and_binds() {
        let a = arc();
        assert_eq!(a.beat_count(), 3);
        // 2 act headers + 3 beats = 5 lore lines
        assert_eq!(a.to_chapter("Arc").lore_count(), 5);
    }

    #[test]
    fn empty_arc_no_climax() {
        assert!(Arc::new().climax().is_none());
    }
}
