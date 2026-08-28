//! Dispatch outcome types — per-unit and per-intent status, and the veto and
//! result records the transport layer returns to a caller.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-daemon-types\src\outcome.rs`
//! (2026-08-15).

use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

/// High-level dispatch outcome for a single unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchOutcome {
    /// The unit completed successfully.
    Ok,
    /// A watchman vetoed the unit before or during dispatch.
    Vetoed,
    /// The unit failed.
    Err,
}

/// Per-unit status tracked in the plan-forward DB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitStatus {
    /// Queued, not yet dispatched.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Done,
    /// Completed with a failure.
    Failed,
    /// Blocked by a watchman veto.
    Vetoed,
}

impl std::fmt::Display for UnitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnitStatus::Pending => "Pending",
            UnitStatus::Running => "Running",
            UnitStatus::Done => "Done",
            UnitStatus::Failed => "Failed",
            UnitStatus::Vetoed => "Vetoed",
        };
        write!(f, "{s}")
    }
}

/// Intent-level status — aggregate of all unit statuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    /// No unit has started yet.
    Pending,
    /// At least one unit is running.
    Running,
    /// Every unit completed successfully.
    Done,
    /// At least one unit failed.
    Failed,
    /// Every unit was vetoed.
    Vetoed,
    /// Some units were vetoed, others completed.
    PartiallyVetoed,
}

impl std::fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IntentStatus::Pending => "Pending",
            IntentStatus::Running => "Running",
            IntentStatus::Done => "Done",
            IntentStatus::Failed => "Failed",
            IntentStatus::Vetoed => "Vetoed",
            IntentStatus::PartiallyVetoed => "PartiallyVetoed",
        };
        write!(f, "{s}")
    }
}

/// Outcome of a single dispatched unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitOutcome {
    /// The unit this outcome describes.
    pub unit_id: UnitId,
    /// This unit's terminal status.
    pub status: UnitStatus,
    /// Expert that handled this unit (e.g. "design_worker", "execute_worker").
    pub expert_name: String,
    /// Structured output from the worker.
    pub outputs: serde_json::Value,
    /// Populated when status is Vetoed.
    pub veto: Option<VetoDetail>,
    /// Populated when status is Failed.
    pub error: Option<String>,
    /// Caller-supplied unix-milliseconds completion timestamp (no wall-clock
    /// read inside this crate — C14 firewall).
    pub completed_at_ms: u128,
}

/// Details of a watchman veto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VetoDetail {
    /// Name of the watchman that issued the veto.
    pub watchman_name: String,
    /// Human-readable reason.
    pub reason: String,
    /// Lane that was blocked.
    pub lane: u8,
}

/// Full result returned to the transport after all units complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// The intent this result answers.
    pub intent_id: i64,
    /// The intent's original text.
    pub intent_text: String,
    /// Aggregate status across all units.
    pub status: IntentStatus,
    /// Per-unit outcomes, in dispatch order.
    pub units: Vec<UnitOutcome>,
    /// Snapshot dir path if writes occurred (absolute path string).
    pub snapshot_dir: Option<String>,
    /// Caller-supplied unix-milliseconds completion timestamp.
    pub completed_at_ms: u128,
}
