//! Type definitions for the AstraKey Sieve engine.
//! ALL values are integers. No floats.

use serde::{Deserialize, Serialize};

/// Every game subsystem that consumes derived seeds.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemID {
    Items, ItemTiers, Levels, Endgame, Alchemy, Gems,
    Pets, Achievements, Pvp, Bosses, Secrets, Loot,
}

impl SystemID {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Items => "sys.items", Self::ItemTiers => "sys.item_tiers",
            Self::Levels => "sys.levels", Self::Endgame => "sys.endgame",
            Self::Alchemy => "sys.alchemy", Self::Gems => "sys.gems",
            Self::Pets => "sys.pets", Self::Achievements => "sys.achievements",
            Self::Pvp => "sys.pvp", Self::Bosses => "sys.bosses",
            Self::Secrets => "sys.secrets", Self::Loot => "sys.loot",
        }
    }

    pub const ALL: &'static [SystemID] = &[
        Self::Items, Self::ItemTiers, Self::Levels, Self::Endgame,
        Self::Alchemy, Self::Gems, Self::Pets, Self::Achievements,
        Self::Pvp, Self::Bosses, Self::Secrets, Self::Loot,
    ];
}

/// Output of the prime sieve.
#[derive(Clone, Debug)]
pub struct SieveResult {
    pub primes: Vec<u64>,
    pub upper_bound: u64,
}

impl SieveResult {
    pub fn count(&self) -> usize { self.primes.len() }
}

/// A per-system seed derived from a master sieve output via HMAC-SHA256.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DerivedSeed {
    pub system: SystemID,
    pub context: String,
    pub master_prime: u64,
    pub master_index: usize,
    pub seed_value: u64,
    pub derivation_hash: String,
}

/// A batch of derived seeds for one system.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeedPack {
    pub system: SystemID,
    pub master_upper_bound: u64,
    pub seeds: Vec<DerivedSeed>,
    pub version: u32,
}

/// Rarity tier (integer weight, not probability).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum RarityTier { Common = 0, Uncommon = 1, Rare = 2, Epic = 3, Mythic = 4 }

impl RarityTier {
    pub fn from_index(i: u8) -> Self {
        match i {
            0 => Self::Common, 1 => Self::Uncommon, 2 => Self::Rare,
            3 => Self::Epic, _ => Self::Mythic,
        }
    }
}
