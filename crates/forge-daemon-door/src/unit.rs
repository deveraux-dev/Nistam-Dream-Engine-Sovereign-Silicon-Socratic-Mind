//! A single decomposed unit of work produced by the planner.
//!
//! Ported from `F:\NewRepo\crates\forge-daemon-types\src\unit.rs` (2026-08-15).
//! `Lane` renamed [`UnitLane`]: `forge-core-v3::spine::Lane` already owns the
//! name `Lane` for a different concept (commit-authority class); this crate
//! does not depend on `forge-core-v3`, so there is no compile-time collision,
//! but two same-named-but-different `Lane` types in the workspace violates
//! L05 (one home per live name) in spirit. `UnitLane` keeps both honest.

/// Identifier for an [`AtomicUnit`], assigned by the storage layer on insert.
pub type UnitId = i64;

/// Routing lane for a unit — determines which expert handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum UnitLane {
    /// Design-mode work: specs, analysis, plans. Routes to design_worker.
    Design = 0,
    /// Execute-mode work: builds, fixes, deploys. Routes to execute_worker.
    Execute = 1,
}

impl UnitLane {
    /// Discriminant value for wire encoding.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode a stored lane. `None` outside `0..=1`.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(UnitLane::Design),
            1 => Some(UnitLane::Execute),
            _ => None,
        }
    }
}

impl std::fmt::Display for UnitLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitLane::Design => write!(f, "Design"),
            UnitLane::Execute => write!(f, "Execute"),
        }
    }
}

/// A single decomposed unit of work produced by the planner.
///
/// Corresponds to one TaskNode from the existing PlannerAdapter.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AtomicUnit {
    /// Pre-insert placeholder only — always 0 from the planner. The
    /// globally-unique DB primary key is assigned by the storage layer on
    /// insert and returned separately. Do not use this field as a DB key.
    pub id: UnitId,
    /// Sequential position within the parent intent (0-indexed).
    pub seq: usize,
    /// Human-readable description from the planner TaskNode.
    pub description: String,
    /// Structured payload (full TaskNode serialized as JSON for audit trail).
    pub payload: serde_json::Value,
    /// Routing lane assigned by the planner adapter.
    pub lane: UnitLane,
    /// File paths this unit is expected to write (populated by planner adapter).
    pub writes_paths: Vec<std::path::PathBuf>,
    /// IDs of units that must complete before this one runs.
    pub depends_on: Vec<UnitId>,
}

impl AtomicUnit {
    /// This unit's id.
    pub fn id(&self) -> UnitId {
        self.id
    }

    /// This unit's routing lane.
    pub fn lane(&self) -> UnitLane {
        self.lane
    }

    /// Paths this unit is expected to write.
    pub fn writes_paths(&self) -> &[std::path::PathBuf] {
        &self.writes_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_u8_and_from_u8_round_trip() {
        for lane in [UnitLane::Design, UnitLane::Execute] {
            assert_eq!(UnitLane::from_u8(lane.as_u8()), Some(lane));
        }
        assert_eq!(UnitLane::from_u8(2), None);
    }

    #[test]
    fn display_matches_variant_name() {
        assert_eq!(UnitLane::Design.to_string(), "Design");
        assert_eq!(UnitLane::Execute.to_string(), "Execute");
    }
}
