//! Blueprint validation report — output of running constraints against a blueprint.
//! Ported from `TODO\quarry-sort\ARCHITECTS-2026-08-17\tile-crawler-architecture-newrepo\
//! blueprint_validate.rs` (v2 donor: `pp_math`→`pp_math_v3`,
//! `crate::architecture::blueprint::NodeId`→`crate::blueprint::NodeId`). Unlike the donor's
//! `#[cfg_attr(feature = "blueprint", ...)]` gating, this crate derives Serialize/Deserialize
//! unconditionally — `forge-zones-v3` has no `blueprint` feature flag, `blueprint.rs` itself
//! already derives serde unconditionally, matching that precedent.
//!
//! Scope cut (L15, named plainly): `blueprint_export.rs`'s `GenerationCommit`/`GenerationTarget`
//! (from a `blueprint_commit` sibling module that does not exist in this quarry batch) and the
//! GJK-based room-overlap/spawn-safety/door-contact validators (need `forge_physics::gjk3d`,
//! not ported to v3 — `forge-physics-v3` only carries the Hermite spline kernel + physics-effect
//! types, no GJK) are NOT ported this pass. Only the three validators with zero physics-crate
//! dependency (connectivity, traversal, encounter) are ported.

use pp_math_v3::fixed_point::MilliUnit;
use serde::{Deserialize, Serialize};

use crate::blueprint::NodeId;

// ─── Validation Report ───────────────────────────────────────────────

/// Full result of running every validator against one blueprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Overall pass/warning/fail verdict.
    pub status: ValidationStatus,
    /// Overall score in permyriad (0 = terrible, 10000 = perfect).
    pub score: i32,
    /// Blocking issues.
    pub errors: Vec<ValidationIssue>,
    /// Non-blocking issues.
    pub warnings: Vec<ValidationIssue>,
    /// Scored/measured blueprint properties.
    pub metrics: BlueprintMetrics,
}

/// Overall pass/warning/fail verdict for a blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// No errors or warnings.
    Pass,
    /// Warnings only — can still commit.
    Warning,
    /// At least one error — cannot commit.
    Fail,
}

impl ValidationReport {
    /// A passing report with perfect score and no issues.
    pub fn pass(metrics: BlueprintMetrics) -> Self {
        Self { status: ValidationStatus::Pass, score: 10000, errors: Vec::new(), warnings: Vec::new(), metrics }
    }

    /// Quick check: can this blueprint be committed?
    pub fn can_commit(&self) -> bool {
        self.status != ValidationStatus::Fail
    }
}

// ─── Validation Issue ────────────────────────────────────────────────

/// One issue a validator found against a blueprint graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// This issue's own id (e.g. `"CONN-001"`).
    pub id: String,
    /// How serious this issue is.
    pub severity: IssueSeverity,
    /// Which validator module produced this issue.
    pub module: ValidatorModule,
    /// Human-readable description.
    pub message: String,
    /// Nodes involved in this issue.
    #[serde(default, with = "crate::blueprint_serde_shim::vec_node_id")]
    pub target_nodes: Vec<NodeId>,
    /// Constraint that triggered this issue.
    #[serde(default)]
    pub constraint_id: Option<String>,
    /// Suggested fix (human-readable).
    #[serde(default)]
    pub suggested_fix: Option<String>,
}

/// How serious a validation issue is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Blocks commit.
    Error,
    /// Does not block commit, but should be reviewed.
    Warning,
    /// Informational only.
    Info,
}

/// Which validator module produced an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidatorModule {
    /// Exits, rooms, critical path.
    Connectivity,
    /// Jumps, climbs, falls.
    Traversal,
    /// Spawns, patrols, boss arenas.
    Encounter,
    /// Lore objects, story triggers.
    Narrative,
    /// Lighting anchors, coverage, readability.
    Lighting,
    /// Geometry, sockets, budget.
    Construction,
}

// ─── Blueprint Metrics ───────────────────────────────────────────────

/// Scored/measured properties of a validated blueprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintMetrics {
    /// Percentage of nodes reachable from spawn (permyriad).
    pub reachable_percent: i32,
    /// Critical path length in MilliUnits.
    pub critical_path_length: MilliUnit,
    /// Number of optional paths/branches.
    pub optional_path_count: u32,
    /// Number of branch points.
    pub branch_count: u32,
    /// Average platform gap in MilliUnits (2D only).
    #[serde(default)]
    pub average_platform_gap: Option<MilliUnit>,
    /// Maximum required jump distance in MilliUnits (2D only).
    #[serde(default)]
    pub max_required_jump: Option<MilliUnit>,
    /// Verticality score (permyriad, 0=flat, 10000=fully vertical).
    pub verticality_score: i32,
    /// Encounter density (permyriad per 1000x1000 MilliUnit area).
    pub encounter_density: i32,
    /// Hazard density (permyriad per 1000x1000 MilliUnit area).
    pub hazard_density: i32,
    /// Lore object density (permyriad per 1000x1000 MilliUnit area).
    pub lore_density: i32,
    /// Light coverage (permyriad of playable area lit).
    pub light_coverage_percent: i32,
    /// Estimated generation cost (arbitrary units, for candidate comparison).
    pub generation_cost_estimate: u32,
}
