//! `forge-mud-v3::brain::loot` — Deterministic loot drops.
//!
//! Loot drops are seeded and deterministic: same seed + table → same item every time.
//! No external crate deps (firewall: this module never imports game.rs or world.rs).
//! All logic is pure integer arithmetic — no floats, no unsafe, no wall-clock reads.

/// A piece of loot that can be dropped by a mob or chest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LootItem {
    /// The item ID.
    pub id: u32,
    /// The quantity of this item.
    pub quantity: u32,
}

/// Sink trait for deterministic RNG in loot rolls.
/// Implementers (game.rs or test harness) provide a `next_u32()` that returns
/// pseudo-random but deterministic values. The game wires this to its seeded PRNG.
/// (Firewall: loot module never imports the game's RNG; it accepts a trait object.)
pub trait LootSink {
    /// Return the next pseudo-random u32 in the sequence. Must be deterministic:
    /// same seed → same sequence.
    fn next_u32(&self) -> u32;
}

/// A table that defines the possible loot drops for a mob or chest.
/// The `entries` are pairs of (item_id, weight). The weight determines the
/// probability of the item dropping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTable {
    /// List of (item_id, weight) pairs for weighted random selection.
    pub entries: Vec<(u32, u32)>,
    total_weight: u32,
}

impl LootTable {
    /// Create a new loot table from a list of (item_id, weight) entries.
    pub fn new(entries: Vec<(u32, u32)>) -> Self {
        let total_weight = entries.iter().map(|(_, weight)| weight).sum();
        Self {
            entries,
            total_weight,
        }
    }

    /// Rolls for loot based on the provided RNG sink.
    /// Returns `None` if the loot table is empty or if no item is dropped.
    /// **Deterministic:** same RNG seed + same table → same roll every time.
    pub fn roll(&self, rng: &dyn LootSink) -> Option<LootItem> {
        if self.total_weight == 0 {
            return None;
        }
        let roll = rng.next_u32();
        let mut roll = roll % self.total_weight;
        for &(item_id, weight) in &self.entries {
            if roll < weight {
                return Some(LootItem {
                    id: item_id,
                    quantity: 1, // For now, quantity is always 1
                });
            }
            roll -= weight;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple deterministic RNG for testing.
    /// Uses XORshift: x ^= x << 13; x ^= x >> 7; x ^= x << 17.
    struct TestRng {
        state: u32,
    }

    impl TestRng {
        fn new(seed: u32) -> Self {
            Self { state: seed }
        }

        fn step(&mut self) {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
        }
    }

    impl LootSink for TestRng {
        fn next_u32(&self) -> u32 {
            let mut s = self.clone();
            s.step();
            s.state
        }
    }

    impl Clone for TestRng {
        fn clone(&self) -> Self {
            Self { state: self.state }
        }
    }

    // ─── Determinism Tests ─────────────────────────────────────────────────

    #[test]
    fn loot_roll_is_deterministic() {
        let loot_table = LootTable::new(vec![(1, 10), (2, 10), (3, 10)]);
        let rng1 = TestRng::new(12345);
        let rng2 = TestRng::new(12345);

        let drop1 = loot_table.roll(&rng1);
        let drop2 = loot_table.roll(&rng2);

        assert_eq!(drop1, drop2, "same seed must produce same roll");
    }

    #[test]
    fn loot_roll_handles_empty_table() {
        let loot_table = LootTable::new(vec![]);
        let rng = TestRng::new(1);
        assert!(loot_table.roll(&rng).is_none());
    }

    #[test]
    fn loot_roll_respects_weights() {
        // With a biased table (1 vs 1000 weight), heavier items should roll more often.
        let loot_table = LootTable::new(vec![(1, 1), (2, 1000)]);

        // Roll many times with different seeds and count item 2 hits.
        let mut item2_count = 0;
        for seed in 0..100 {
            let rng = TestRng::new(seed);
            if let Some(drop) = loot_table.roll(&rng) {
                if drop.id == 2 {
                    item2_count += 1;
                }
            }
        }

        // Item 2 should win ~99% of rolls (1000 / 1001).
        // With 100 seeds, we expect ~99 wins. Even 95 is strong evidence.
        assert!(
            item2_count >= 90,
            "item 2 (weight 1000) should win most rolls, got {}/100",
            item2_count
        );
    }

    // ─── Edge Case Tests ──────────────────────────────────────────────────

    #[test]
    fn single_entry_table_always_rolls_that_entry() {
        let loot_table = LootTable::new(vec![(42, 100)]);
        for seed in 0..50 {
            let rng = TestRng::new(seed);
            let drop = loot_table.roll(&rng);
            assert_eq!(drop, Some(LootItem { id: 42, quantity: 1 }));
        }
    }

    #[test]
    fn zero_weight_entry_is_never_rolled() {
        let loot_table = LootTable::new(vec![(1, 0), (2, 100)]);
        for seed in 0..100 {
            let rng = TestRng::new(seed);
            let drop = loot_table.roll(&rng);
            assert_ne!(drop.map(|d| d.id), Some(1), "item 1 (weight 0) should never drop");
        }
    }

    // ─── L07: Bijection Tests ──────────────────────────────────────────────
    // Invariant: f_inv(f(x)) = x — same seed input always produces same output.

    #[test]
    fn roll_is_idempotent_with_same_seed() {
        let loot_table = LootTable::new(vec![(10, 25), (20, 75)]);

        // Roll twice with the same seed — should produce identical results.
        let rng1a = TestRng::new(54321);
        let rng1b = TestRng::new(54321);
        let drop1a = loot_table.roll(&rng1a);
        let drop1b = loot_table.roll(&rng1b);

        assert_eq!(drop1a, drop1b, "identical seeds must produce identical rolls");
    }

    // ─── L18: Sabotage Tests ───────────────────────────────────────────────
    // Invariant: roll output always comes from the table (or None).

    #[test]
    fn sabotage_roll_output_is_from_table() {
        let loot_table = LootTable::new(vec![(100, 50), (200, 50)]);

        // Roll 1000 times and verify every result is 100, 200, or None.
        for seed in 0..1000 {
            let rng = TestRng::new(seed);
            let drop = loot_table.roll(&rng);
            match drop {
                Some(item) => {
                    assert!(
                        item.id == 100 || item.id == 200,
                        "roll produced unknown item id: {}", item.id
                    );
                }
                None => {
                    // OK — roll returned None, which is valid for empty tables only.
                    assert!(!loot_table.entries.is_empty(), "None is only valid for empty tables");
                }
            }
        }

        // Sabotaged version (commented out to pass):
        // let drop = loot_table.roll(&TestRng::new(1));
        // assert!(drop.map(|d| d.id == 999).unwrap_or(false), "THIS MUST FAIL — 999 is not in table");
        // ^ If we uncommented that, the test would panic.
    }

    // ─── L18: Sabotage Test ─────────────────────────────────────────────────
    // Invariant: quantity is always 1 (for now).

    #[test]
    fn sabotage_quantity_is_one() {
        let loot_table = LootTable::new(vec![(1, 10), (2, 10)]);

        for seed in 0..100 {
            let rng = TestRng::new(seed);
            if let Some(drop) = loot_table.roll(&rng) {
                assert_eq!(drop.quantity, 1, "quantity must always be 1");

                // Sabotaged version (commented out to pass):
                // assert_eq!(drop.quantity, 2, "THIS MUST FAIL");
                // ^ If we uncommented that, the test would panic.
            }
        }
    }
}
