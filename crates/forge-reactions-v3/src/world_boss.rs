//! World boss types — spawn formula, attempt states, solution contracts.

use crate::faction::{FactionId, WorldBossId, RelicId};
use crate::solution_path::SolutionPathKind;

// ── Spawn Stages ─────────────────────────────────────────────────────────────

/// Spawn stage — lifecycle stage of a world boss from hidden to conclusion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpawnStage {
    /// World boss is hidden and not yet publicly known.
    #[default]
    Hidden = 0,
    /// World boss is rumored to exist.
    Rumor = 1,
    /// World boss presence is manifesting as an omen.
    Omen = 2,
    /// World boss is callable (can be summoned).
    Callable = 3,
    /// A spawn attempt has been committed to.
    Committed = 4,
    /// World boss spawn succeeded.
    Succeeded = 5,
    /// World boss spawn failed.
    Failed = 6,
    /// World boss spawn was prevented.
    Prevented = 7,
    /// World boss spawn was missed.
    Missed = 8,
    /// World boss was touched by the crown.
    CrownTouched = 9,
}

impl SpawnStage {
    /// Returns true if this stage is a terminal (concluding) stage.
    pub const fn is_terminal(self) -> bool {
        matches!(self,
            Self::Succeeded | Self::Failed | Self::Prevented | Self::Missed
        )
    }
}

// ── Attempt State ────────────────────────────────────────────────────────────

/// Attempt state — the state of a world boss spawn attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AttemptState {
    /// World boss has not been seen.
    Unseen = 0,
    /// World boss is rumored.
    Rumored = 1,
    /// World boss omen is active.
    OmenActive = 2,
    /// World boss can be called.
    Callable = 3,
    /// Attempt has been committed.
    Committed = 4,
    /// Attempt succeeded.
    Succeeded = 5,
    /// Attempt failed.
    Failed = 6,
    /// Attempt was prevented.
    Prevented = 7,
    /// Attempt was missed.
    Missed = 8,
    /// Attempt resulted in crown touch.
    CrownTouched = 9,
}

impl AttemptState {
    /// Returns true if this state represents a consumed (used up) attempt.
    pub const fn attempt_consumed(self) -> bool {
        matches!(self,
            Self::Committed | Self::Succeeded | Self::Failed | Self::CrownTouched
        )
    }
}

// ── Relic Variants ───────────────────────────────────────────────────────────

/// Relic variant — the type or condition of a relic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RelicVariant {
    /// The true, authentic relic.
    True = 0,
    /// An echo or copy of the relic.
    Echo = 1,
    /// A broken or damaged relic.
    Broken = 2,
    /// A surrendered relic.
    Surrendered = 3,
    /// A relic touched by the crown.
    CrownTouched = 4,
}

/// Relic ownership mode — the ownership status or mode of a relic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RelicOwnershipMode {
    /// Relic is claimed by someone.
    Claimed = 0,
    /// Relic has been surrendered.
    Surrendered = 1,
    /// Relic is buried.
    Buried = 2,
    /// Relic is exposed.
    Exposed = 3,
    /// Relic has been forged.
    Forged = 4,
    /// Relic has been traded.
    Traded = 5,
    /// Relic has been destroyed.
    Destroyed = 6,
    /// Relic is unclaimed.
    Unclaimed = 7,
}

// ── Solution Imprint ─────────────────────────────────────────────────────────

/// Solution imprint — metadata for a solution path to a world boss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolutionImprint {
    /// The kind of solution path.
    pub path: SolutionPathKind,
    /// Suffix or label for this solution.
    pub suffix: &'static str,
    /// Bonus domain or category for this solution.
    pub bonus_domain: &'static str,
    /// Hidden cost or consequences of this solution.
    pub hidden_cost: &'static str,
}

// ── World Boss Definition ────────────────────────────────────────────────────

/// Faction world boss definition — static metadata for a faction's world boss.
#[derive(Clone, Debug)]
pub struct FactionWorldBossDef {
    /// World boss identifier.
    pub id: WorldBossId,
    /// The faction this world boss is associated with.
    pub faction: FactionId,
    /// Display name for this world boss.
    pub display_name: &'static str,
    /// Zones where this world boss has bias or affinity.
    pub zone_bias: &'static [&'static str],
    /// Solution paths available to defeat this world boss.
    pub solution_paths: &'static [SolutionPathKind],
    /// The true relic dropped by this world boss.
    pub true_relic: RelicId,
    /// Optional echo relic variant.
    pub echo_relic: Option<RelicId>,
    /// Optional broken relic variant.
    pub broken_relic: Option<RelicId>,
}

// ── Spawn Thresholds ─────────────────────────────────────────────────────────

/// Root state thresholds — spawn stage thresholds based on root quality.
#[derive(Clone, Copy, Debug)]
pub struct RootStateThresholds {
    /// Threshold value for rumor stage.
    pub rumor_q: u16,
    /// Threshold value for omen stage.
    pub omen_q: u16,
    /// Threshold value for callable stage.
    pub callable_q: u16,
}

/// Thresholds for a healthy root state.
pub const HEALTHY_ROOT: RootStateThresholds = RootStateThresholds {
    rumor_q: 4000, omen_q: 5500, callable_q: 7000,
};
/// Thresholds for a corrupted root state.
pub const CORRUPTED_ROOT: RootStateThresholds = RootStateThresholds {
    rumor_q: 3000, omen_q: 4250, callable_q: 5500,
};
/// Thresholds for a void leak root state.
pub const VOID_LEAK: RootStateThresholds = RootStateThresholds {
    rumor_q: 2000, omen_q: 3000, callable_q: 4000,
};
