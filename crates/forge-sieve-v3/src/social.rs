//! Social sieves — Quest, Reputation, Economy, Diplomacy, Trade, Witness, Farming, Fishing.
//! Ported from F:\NewRepo\crates\forge-sieve\src\social.rs (state structs only).
//! Donor's `impl Sieve for X { observe/evaluate/promote/snapshot }` match-statement
//! logic is NOT ported — `npc_bq.rs` (ARCH000, Sean 2026-08-22) reads these structs'
//! fields directly and classifies signal/noise via a trained BQ centroid per kind
//! instead.

use serde::{Deserialize, Serialize};
use crate::combat::PatternMap;

/// Quest progress state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum QuestStatus {
    /// Quest not yet available.
    Locked = 0,
    /// Quest available but not started.
    Unlocked = 1,
    /// Quest actively in progress.
    InProgress = 2,
    /// Quest successfully completed.
    Completed = 3,
    /// Quest failed.
    Failed = 4,
    /// Quest abandoned by player.
    Abandoned = 5,
}

/// Objective type classification for quest tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ObjectiveType {
    /// Binary flag objective.
    Flag = 0,
    /// Count-based progress objective.
    Count = 1,
    /// Location-based objective.
    Location = 2,
    /// Item collection objective.
    Item = 3,
    /// Survival duration objective.
    Survive = 4,
    /// Listen/observe objective.
    Listen = 5,
}

/// A single objective within a quest.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ObjectiveRecord {
    /// Type of objective (enum-mapped to ObjectiveType).
    pub obj_type: u8,
    /// Current progress toward target.
    pub current_count: u16,
    /// Total required to complete.
    pub target_count: u16,
    /// Completion status.
    pub is_complete: bool,
    /// Optional objective (doesn't block quest completion).
    pub is_optional: bool,
    /// Hidden from player UI.
    pub is_hidden: bool,
}

/// Per-quest state: objectives, status, failure conditions.
/// Quest state machine tracking objectives and progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestSieve {
    /// Unique quest identifier.
    pub quest_id: u32,
    /// Current quest status (Locked, InProgress, Completed, etc).
    pub status: QuestStatus,
    /// Array of up to 8 objectives for this quest.
    pub objectives: [ObjectiveRecord; 8],
    /// Number of active objectives.
    pub objective_count: u8,
    /// Min and max moon phase when this quest is active.
    pub moon_range: (u8, u8),
    /// Bitfield of failure conditions encountered.
    pub failure_flags: u16,
    /// Quest archetype (0=THE_LAND, 1=WISAKEDJAK, 2=THE_WALKER).
    pub archetype: u8,
    /// Narrative weight/importance of this quest.
    pub canon_weight: i32,
}

/// Player standing with an NPC, from first contact to kin.
/// Trust relationship tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum TrustTier {
    /// Unknown or untrusted entity.
    #[default]
    Stranger = 0,
    /// Familiar entity.
    Known = 1,
    /// Trusted entity.
    Trusted = 2,
    /// Family or intimate relationship.
    Kin = 3,
}

/// Per-NPC reputation accumulator (trades, generosity, violence observed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationSieve {
    /// NPC being tracked.
    pub npc_id: u32,
    /// Count of completed trades observed.
    pub observed_trades: u16,
    /// Accumulated generosity score.
    pub observed_generosity: i32,
    /// Accumulated violence score.
    pub observed_violence: i32,
    /// Current trust relationship tier.
    pub trust_tier: TrustTier,
}

/// Per-zone supply/demand/greed economy state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomySieve {
    /// Zone being tracked.
    pub zone_id: u32,
    /// Available supply for each resource type.
    pub resource_supply: [i32; 8],
    /// Observed demand for each resource type.
    pub resource_demand: [i32; 8],
    /// Hoarding behavior accumulator.
    pub hoarding_score: i32,
    /// Reciprocal trade tendency.
    pub reciprocity_score: i32,
    /// Greed indicator.
    pub greed: i32,
}

/// Per-faction diplomatic state (relations, treaties, promise history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomacySieve {
    /// Faction identifier.
    pub faction_id: u8,
    /// Relation score with each faction.
    pub relations: [i32; 8],
    /// Active diplomatic treaties (bitfield).
    pub treaties_active: u64,
    /// Promises kept to this faction.
    pub promises_kept: u16,
    /// Promises broken to this faction.
    pub promises_broken: u16,
    /// Momentum of trust/betrayal trend.
    pub trust_momentum: i32,
    /// Betrayal count per faction.
    pub betrayal_memory: [u16; 8],
    /// Observed negotiation pattern map.
    pub negotiation_patterns: PatternMap,
}

/// Per-merchant trade state (prices, demand, haggling patterns).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSieve {
    /// Merchant being tracked.
    pub merchant_id: u32,
    /// Last known price for each item type.
    pub price_memory: [i32; 32],
    /// Observed demand per item type.
    pub demand_observed: [u16; 32],
    /// Supply surplus per item type.
    pub supply_glut: [u16; 32],
    /// Haggling negotiation patterns.
    pub haggle_pattern: PatternMap,
    /// Active caravan routes.
    pub caravan_routes_active: u8,
    /// Smuggling incidents detected.
    pub smuggle_detected: u16,
}

/// A witnessed event's spread through gossip (who saw it, how far it travels).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessSieve {
    /// Event identifier.
    pub event_id: u64,
    /// Type of event witnessed.
    pub event_type: u8,
    /// NPC IDs of witnesses.
    pub witnesses: [u32; 8],
    /// Count of actual witnesses.
    pub witness_count: u8,
    /// Gossip spread radius.
    pub spread_radius: u8,
    /// Ticks since event occurred.
    pub ticks_since: u32,
    /// Public sentiment about the event.
    pub sentiment: i32,
}

/// Per-plot farming state (soil, crop, growth, rotation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmingSieve {
    /// Plot identifier.
    pub plot_id: u32,
    /// Soil quality (0-10000).
    pub soil_fertility: i32,
    /// Moisture level (0-10000).
    pub moisture: i32,
    /// Crop type planted.
    pub planted_crop: u8,
    /// Growth stage (0-4).
    pub growth_stage: u8,
    /// Days since planting.
    pub days_planted: u16,
    /// Expected yield.
    pub yield_estimate: i32,
    /// History of crops rotated.
    pub crop_rotation_history: [u8; 8],
    /// Current rotation index.
    pub rotation_index: u8,
    /// Ticks since last tended.
    pub neglect_ticks: u32,
    /// Pest infestation level.
    pub pest_pressure: i32,
    /// Companion planting bonus.
    pub companion_bonus: i32,
}

/// Per-water-body fishing state (fish populations, catch history, lures).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishingSieve {
    /// Water body identifier.
    pub water_body_id: u32,
    /// Fish population per species (0-7).
    pub fish_populations: [u16; 8],
    /// Catch history per species.
    pub catch_history: [u16; 8],
    /// Reproduction rate per species.
    pub reproduction_rate: [i32; 8],
    /// Water quality health score.
    pub water_health: i32,
    /// Water temperature (affects bite).
    pub water_temperature: i32,
    /// Lure effectiveness per type.
    pub lure_effectiveness: [i32; 8],
    /// Times this location has been fished.
    pub times_fished_here: u32,
    /// Most recent lure used.
    pub dominant_lure_used: u8,
    /// Moon phase bite modifier.
    pub moon_bite_modifier: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_tier_ordering() {
        assert!(TrustTier::Stranger < TrustTier::Kin);
    }

    #[test]
    fn quest_objective_completion() {
        let mut obj = ObjectiveRecord { obj_type: 1, current_count: 4, target_count: 5, is_complete: false, is_optional: false, is_hidden: false };
        obj.current_count += 1;
        obj.is_complete = obj.current_count >= obj.target_count;
        assert!(obj.is_complete);
    }

    #[test]
    fn quest_sieve_construction() {
        let q = QuestSieve {
            quest_id: 1, status: QuestStatus::InProgress,
            objectives: [ObjectiveRecord::default(); 8], objective_count: 1,
            moon_range: (1, 13), failure_flags: 0, archetype: 0, canon_weight: 100,
        };
        assert_eq!(q.quest_id, 1);
        assert_eq!(q.status, QuestStatus::InProgress);
        assert_eq!(q.objective_count, 1);
    }

    #[test]
    fn economy_sieve_resource_tracking() {
        let e = EconomySieve {
            zone_id: 1,
            resource_supply: [500; 8],
            resource_demand: [0; 8],
            hoarding_score: 0,
            reciprocity_score: 0,
            greed: 0,
        };
        assert_eq!(e.resource_supply[0], 500);
        assert_eq!(e.zone_id, 1);
    }
}
