//! Loot — a weighted loot roll producing an item name + rarity, harvested from
//! deveraux_mud generateLoot. Deterministic via mulberry.

use crate::items::Rarity;
use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// A weighted loot entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootEntry {
    /// Drop weight; higher values make this entry more likely to roll.
    pub weight: u32,
    /// The loot item name.
    pub name: String,
    /// The rarity tier of this loot drop.
    pub rarity: Rarity,
}

/// A loot table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootTable {
    /// The weighted loot entries in this table.
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// Constructs an empty loot table.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a weighted entry and returns a mutable reference for chaining.
    pub fn add(&mut self, weight: u32, name: impl Into<String>, rarity: Rarity) -> &mut Self {
        self.entries.push(LootEntry { weight, name: name.into(), rarity });
        self
    }
    /// Sums all entry weights.
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|e| e.weight).sum()
    }
    /// Roll one drop against `rng`.
    pub fn roll(&self, rng: &mut Mulberry32) -> Option<(&str, Rarity)> {
        let total = self.total_weight();
        if total == 0 {
            return None;
        }
        let mut r = rng.below(total);
        for e in &self.entries {
            if r < e.weight {
                return Some((&e.name, e.rarity));
            }
            r -= e.weight;
        }
        self.entries.last().map(|e| (e.name.as_str(), e.rarity))
    }
}

/// A seeded drop table.
pub fn mob_drops() -> LootTable {
    let mut t = LootTable::new();
    t.add(60, "copper coin", Rarity::Common)
        .add(25, "root fibre", Rarity::Uncommon)
        .add(10, "warden shard", Rarity::Rare)
        .add(4, "void ember", Rarity::Epic)
        .add(1, "grandmaster relic", Rarity::Mythic);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_are_deterministic() {
        let t = mob_drops();
        let mut a = Mulberry32::new(42);
        let mut b = Mulberry32::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        let mut ra = Mulberry32::new(5);
        assert!(t.roll(&mut ra).is_some());
    }

    #[test]
    fn weights_favour_common() {
        let t = mob_drops();
        let mut rng = Mulberry32::new(11);
        let commons = (0..1000).filter(|_| t.roll(&mut rng) == Some(("copper coin", Rarity::Common))).count();
        assert!(commons > 500);
    }

    #[test]
    fn empty_table_none() {
        assert!(LootTable::new().roll(&mut Mulberry32::new(1)).is_none());
    }
}
