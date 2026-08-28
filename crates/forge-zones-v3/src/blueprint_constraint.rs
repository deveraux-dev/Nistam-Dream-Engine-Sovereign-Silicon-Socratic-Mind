//! Blueprint constraint system — declarative rules about blueprint validity.
//! Ported from `TODO\quarry-sort\ARCHITECTS-2026-08-17\tile-crawler-architecture-newrepo\
//! blueprint_constraint.rs` (v2 donor, byte-identical to the `tile-crawler-architecture`
//! sibling copy, hash-confirmed). `pp_math`→`pp_math_v3`,
//! `crate::architecture::blueprint::NodeId`→`crate::blueprint::NodeId`; serde derived
//! unconditionally, matching this crate's existing precedent (no `blueprint` feature flag here).
//!
//! Three tiers:
//! - Hard: must pass before commit (errors block generation)
//! - Soft: guide quality (warnings, scored)
//! - Preference: encode user taste (influence generation, never block)

use pp_math_v3::fixed_point::MilliUnit;
use serde::{Deserialize, Serialize};

use crate::blueprint::NodeId;

// ─── Constraint Set ──────────────────────────────────────────────────

/// The full set of constraints a blueprint's generation/validation is judged against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintSet {
    /// This set's own id.
    pub id: String,
    /// Constraints that must pass before commit.
    pub hard: Vec<HardConstraint>,
    /// Constraints that guide quality without blocking.
    pub soft: Vec<SoftConstraint>,
    /// Constraints that encode taste, never block.
    pub preferences: Vec<Preference>,
}

impl ConstraintSet {
    /// An empty constraint set with the given id.
    pub fn empty(id: impl Into<String>) -> Self {
        Self { id: id.into(), hard: Vec::new(), soft: Vec::new(), preferences: Vec::new() }
    }
}

// ─── Hard Constraints (must pass) ────────────────────────────────────

/// A constraint that blocks commit when violated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardConstraint {
    /// This constraint's own id.
    pub id: String,
    /// What this constraint actually checks.
    pub kind: HardConstraintKind,
    /// Optional: only applies to this node.
    #[serde(default, with = "crate::blueprint_serde_shim::opt_node_id")]
    pub target: Option<NodeId>,
}

/// What a hard constraint actually checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardConstraintKind {
    /// Player spawn must exist.
    PlayerSpawnExists,
    /// At least one exit must exist.
    ExitExists,
    /// Critical path from spawn to exit must be traversable.
    CriticalPathReachable,
    /// Player spawn must not overlap a hazard.
    SpawnNotInHazard,
    /// Enemy spawns must be inside playable space.
    EnemySpawnsInBounds,
    /// No jump exceeds max traversable distance.
    MaxJumpDistance {
        /// The maximum allowed jump distance.
        max: MilliUnit,
    },
    /// No orphaned rooms (all rooms reachable from spawn).
    NoOrphanedRooms,
    /// No overlapping blocking volumes.
    NoOverlappingBlockers,
    /// Minimum corridor/passage width.
    MinCorridorWidth {
        /// The minimum allowed width.
        min: MilliUnit,
    },
    /// Boss arena must have lockable exits.
    BossArenaLockable,
    /// Required narrative objects must be present.
    NarrativeObjectsPresent {
        /// Tags the required narrative objects must carry.
        required_tags: Vec<String>,
    },
    /// Required lighting anchors exist.
    LightingAnchorsExist,
    /// Exits target valid rooms/zones.
    ExitTargetsValid,
}

// ─── Soft Constraints (quality guidance) ─────────────────────────────

/// A constraint that scores/warns without blocking commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftConstraint {
    /// This constraint's own id.
    pub id: String,
    /// What this constraint actually checks.
    pub kind: SoftConstraintKind,
    /// Optional: only applies to this node.
    #[serde(default, with = "crate::blueprint_serde_shim::opt_node_id")]
    pub target: Option<NodeId>,
}

/// What a soft constraint actually checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoftConstraintKind {
    /// Encounter density within target range (permyriad per unit area).
    EncounterDensity {
        /// Minimum acceptable density.
        min: i32,
        /// Maximum acceptable density.
        max: i32,
    },
    /// Critical path length within range (MilliUnits).
    PathLength {
        /// Minimum acceptable length.
        min: MilliUnit,
        /// Maximum acceptable length.
        max: MilliUnit,
    },
    /// At least N optional branches.
    MinOptionalBranches {
        /// The minimum branch count.
        count: u32,
    },
    /// Verticality score target (permyriad, 0=flat, 10000=vertical).
    VerticalityTarget {
        /// Minimum acceptable verticality.
        min: i32,
        /// Maximum acceptable verticality.
        max: i32,
    },
    /// Recovery point after high-risk section.
    RecoveryAfterRisk,
    /// Visual landmark near exits.
    LandmarkNearExits {
        /// Maximum distance a landmark may be from an exit.
        max_distance: MilliUnit,
    },
    /// Balance ranged vs melee enemy placement (permyriad ratio).
    EnemyTypeBalance {
        /// Minimum acceptable ranged-enemy ratio.
        ranged_ratio_min: i32,
        /// Maximum acceptable ranged-enemy ratio.
        ranged_ratio_max: i32,
    },
    /// Avoid repeated identical platform gaps.
    MaxRepeatedGaps {
        /// Maximum consecutive identical gaps allowed.
        max_consecutive: u32,
    },
}

// ─── Preferences (user taste, never blocks) ──────────────────────────

/// A taste preference that influences generation without blocking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preference {
    /// This preference's own id.
    pub id: String,
    /// What taste this preference encodes.
    pub kind: PreferenceKind,
}

/// What taste a preference encodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreferenceKind {
    /// "More vertical" — bias generation toward height.
    MoreVertical,
    /// "Less maze-like" — prefer linear paths.
    LessMaze,
    /// "More readable" — wider corridors, clearer sightlines.
    MoreReadable,
    /// "More oppressive" — tighter spaces, less light.
    MoreOppressive,
    /// "Fewer enemies, more hazards."
    HazardsOverEnemies,
    /// "Boss arena should feel exposed."
    BossExposed,
    /// "Keep path linear but visually rich."
    LinearButRich,
    /// Custom preference with freeform description.
    Custom {
        /// The freeform description.
        description: String,
    },
}
