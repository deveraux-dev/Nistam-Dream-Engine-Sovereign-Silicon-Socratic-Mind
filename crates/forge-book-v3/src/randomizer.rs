//! Randomizer — a seeded deterministic weighted-table roller (mulberry). The
//! generator behind procedural pages; same seed, same roll.

use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// A weighted table of string outcomes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedTable {
    /// Weighted outcomes: (weight, item) pairs.
    pub entries: Vec<(u32, String)>,
}

impl WeightedTable {
    /// Create a new empty weighted table.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add an outcome with a weight (weight 0 is ignored on roll).
    pub fn add(&mut self, weight: u32, item: impl Into<String>) -> &mut Self {
        self.entries.push((weight, item.into()));
        self
    }
    /// Sum of all entry weights.
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|(w, _)| *w).sum()
    }
    /// True if the table has no weighted outcomes.
    pub fn is_empty(&self) -> bool {
        self.total_weight() == 0
    }
    /// Roll against `rng`; returns the chosen item, or None if empty.
    pub fn roll(&self, rng: &mut Mulberry32) -> Option<&str> {
        let total = self.total_weight();
        if total == 0 {
            return None;
        }
        let mut r = rng.below(total);
        for (w, item) in &self.entries {
            if r < *w {
                return Some(item);
            }
            r -= *w;
        }
        self.entries.last().map(|(_, s)| s.as_str())
    }
}

/// forge_core_v3::Mulberry32 deliberately carries no Default (Crate Zero, no
/// derives beyond what the algorithm itself needs) — this is the skip-field
/// fallback `serde(default = "...")` needs instead of the `Default` trait.
fn skipped_rng() -> Mulberry32 {
    Mulberry32::new(0)
}

/// A seeded roller — deterministic stream of rolls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Randomizer {
    #[serde(skip, default = "skipped_rng")]
    rng: Mulberry32,
}

impl Randomizer {
    /// Create a new randomizer with the given seed.
    pub fn new(seed: u32) -> Self {
        Self { rng: Mulberry32::new(u64::from(seed)) }
    }
    /// Roll a table once.
    pub fn roll(&mut self, table: &WeightedTable) -> Option<String> {
        table.roll(&mut self.rng).map(str::to_string)
    }
    /// Roll `n` times.
    pub fn rolls(&mut self, table: &WeightedTable, n: usize) -> Vec<String> {
        (0..n).filter_map(|_| self.roll(table)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> WeightedTable {
        let mut t = WeightedTable::new();
        t.add(1, "rare").add(9, "common");
        t
    }

    #[test]
    fn same_seed_same_rolls() {
        let t = table();
        let mut a = Randomizer::new(42);
        let mut b = Randomizer::new(42);
        assert_eq!(a.rolls(&t, 50), b.rolls(&t, 50));
    }

    #[test]
    fn weights_bias_the_outcome() {
        let t = table();
        let mut r = Randomizer::new(7);
        let out = r.rolls(&t, 1000);
        let common = out.iter().filter(|s| *s == "common").count();
        assert!(common > 700, "9:1 weight should favour common, got {common}");
    }

    #[test]
    fn empty_table_rolls_none() {
        let mut r = Randomizer::new(1);
        assert!(r.roll(&WeightedTable::new()).is_none());
    }
}
