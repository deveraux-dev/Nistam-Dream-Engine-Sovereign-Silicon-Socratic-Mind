//! Ported verbatim from F:\NewRepo\crates\forge-consequence\src\quest.rs (2026-08-17 truth-hunt lineage port, completing the 2026-08-13 wce-tags-port; moved out of forge-core-v3 into this sibling crate 2026-08-17 — see forge-consequence-v3/Cargo.toml).
//!
//! Procedural Quest Generation based on Quest Seeds.
//!
//! Provides the data structures and deterministic generator logic to map a `QuestSeed`
//! to a procedural quest descriptor (`ProceduralQuest`). All generation steps are
//! fully integer-deterministic and self-contained.

use forge_core_v3::consequence::query::ConsequenceKind;
use forge_core_v3::consequence::tags::*;
use serde::{Deserialize, Serialize};

/// Lightweight, integer-deterministic pseudo-random number generator (LCG).
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Create a new generator from a seed value.
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Retrieve the next pseudo-random u32 value.
    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// Retrieve a pseudo-random u32 within a specific range (exclusive of max).
    fn next_range(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let diff = max - min;
        min + (self.next_u32() % diff)
    }
}

/// Node representing a Quest Seed in the simulation hierarchy.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct QuestSeed {
    /// Deterministic seed hash used for procedural generation.
    pub seed_hash: u64,
    /// Current completion status of the quest (0 = Inactive, 1 = Active, 2 = Completed).
    pub completion_state: u8,
}

impl QuestSeed {
    /// Construct a new default quest seed.
    pub fn new(seed_hash: u64) -> Self {
        Self {
            seed_hash,
            completion_state: 0,
        }
    }

    /// Set active status on this seed.
    pub fn activate(mut self) -> Self {
        self.completion_state = 1;
        self
    }

    /// Set completed status on this seed.
    pub fn complete(mut self) -> Self {
        self.completion_state = 2;
        self
    }

    /// Procedurally generate a detailed `ProceduralQuest` descriptor from the seed hash.
    /// The output is 100% deterministic given the same seed hash.
    pub fn generate_quest(&self) -> ProceduralQuest {
        let mut rng = SimpleRng::new(self.seed_hash);

        // Deterministically select a quest template index
        let template_idx = rng.next_range(0, 4);

        let (title, description, required_consequence, required_src_family, required_tgt_family, target_count) = match template_idx {
            0 => (
                "Erosion's Legacy".to_string(),
                "Direct strong water flow onto the ancient stone formations to erode the voxels and reveal path hints.".to_string(),
                ConsequenceKind::VoxelBreak,
                SRC_FAMILY_FLUID,
                TGT_FAMILY_TERRAIN,
                rng.next_range(5, 15),
            ),
            1 => (
                "Grave Bell Resonance".to_string(),
                "Strike the grave bell to generate rhythmic sound waves that shatter structural stone boundaries.".to_string(),
                ConsequenceKind::Shatter,
                SRC_FAMILY_SOUND,
                TGT_FAMILY_TERRAIN,
                rng.next_range(1, 5),
            ),
            2 => (
                "Flames of Ignition".to_string(),
                "Apply heat or direct combustion onto wood material to ignite a controlled burn and clear the sector.".to_string(),
                ConsequenceKind::Ignite,
                SRC_FAMILY_FIRE,
                TGT_FAMILY_TERRAIN,
                rng.next_range(3, 10),
            ),
            _ => (
                "Lightning Strike Witness".to_string(),
                "Harness lightning and direct it onto structural elements or entities to observe and record high-energy damage consequences.".to_string(),
                ConsequenceKind::Damage,
                SRC_FAMILY_GRAVITY,
                TGT_FAMILY_ENTITY,
                rng.next_range(2, 6),
            ),
        };

        ProceduralQuest {
            quest_id: (self.seed_hash & 0xFFFFFFFF) as u32,
            title,
            description,
            required_consequence,
            required_src_family,
            required_tgt_family,
            target_count,
            current_progress: 0,
            completion_state: self.completion_state,
        }
    }
}

/// Fully generated procedural quest descriptor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProceduralQuest {
    /// Unique quest identifier.
    pub quest_id: u32,
    /// Title of the quest.
    pub title: String,
    /// In-depth description and objective information.
    pub description: String,
    /// The specific consequence kind that triggers progress.
    pub required_consequence: ConsequenceKind,
    /// Filter progress checking to this source family.
    pub required_src_family: u8,
    /// Filter progress checking to this target family.
    pub required_tgt_family: u8,
    /// Total number of matching consequence occurrences required for completion.
    pub target_count: u32,
    /// Current number of matched consequences.
    pub current_progress: u32,
    /// Current status: 0 = Inactive, 1 = Active, 2 = Completed.
    pub completion_state: u8,
}

impl ProceduralQuest {
    /// Register a witnessed consequence and evaluate progress increment.
    /// Returns `true` if this update completed the quest.
    pub fn register_consequence(&mut self, kind: ConsequenceKind, src_family: u8, tgt_family: u8) -> bool {
        if self.completion_state != 1 {
            return false; // Only active quests accumulate progress
        }

        if kind == self.required_consequence
            && src_family == self.required_src_family
            && tgt_family == self.required_tgt_family
        {
            self.current_progress = self.current_progress.saturating_add(1);
            if self.current_progress >= self.target_count {
                self.completion_state = 2; // Mark as Completed
                return true;
            }
        }

        false
    }

    /// Explicitly check whether the quest requirements have been fully satisfied.
    pub fn check_satisfaction(&self) -> bool {
        self.current_progress >= self.target_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rng_determinism() {
        let mut rng1 = SimpleRng::new(12345);
        let mut rng2 = SimpleRng::new(12345);

        assert_eq!(rng1.next_u32(), rng2.next_u32());
        assert_eq!(rng1.next_range(10, 100), rng2.next_range(10, 100));
    }

    #[test]
    fn test_procedural_quest_generation() {
        let seed = QuestSeed::new(987654321).activate();
        let quest = seed.generate_quest();

        assert_eq!(quest.completion_state, 1);
        assert!(!quest.title.is_empty());
        assert!(!quest.description.is_empty());
        assert!(quest.target_count > 0);
    }

    #[test]
    fn test_quest_progress_and_satisfaction() {
        let seed = QuestSeed::new(112233).activate();
        let mut quest = seed.generate_quest();

        // Check template invariants: progress must be zero initially
        assert_eq!(quest.current_progress, 0);
        assert!(!quest.check_satisfaction());

        // Feed some unrelated consequences
        let completed = quest.register_consequence(
            ConsequenceKind::None,
            SRC_FAMILY_FIRE,
            TGT_FAMILY_FLUID,
        );
        assert!(!completed);
        assert_eq!(quest.current_progress, 0);

        // Feed correct consequences up to completion
        let req_kind = quest.required_consequence;
        let req_src = quest.required_src_family;
        let req_tgt = quest.required_tgt_family;
        let target = quest.target_count;

        for i in 1..=target {
            let is_done = quest.register_consequence(req_kind, req_src, req_tgt);
            if i == target {
                assert!(is_done);
                assert_eq!(quest.completion_state, 2);
            } else {
                assert!(!is_done);
                assert_eq!(quest.completion_state, 1);
            }
        }

        assert!(quest.check_satisfaction());
    }
}
