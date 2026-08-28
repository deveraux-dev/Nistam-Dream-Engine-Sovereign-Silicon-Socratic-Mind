//! Full semantic audit record — one per dispatched unit, appended to a
//! caller-owned append-only log.
//!
//! Ported from `F:\NewRepo\crates\forge-daemon-types\src\audit.rs`
//! (2026-08-15). All five `chrono::DateTime<Utc>` fields changed to
//! caller-supplied `u128` unix-ms timestamps (C14 firewall — no wall-clock
//! read inside this crate; this also drops `chrono` from the dependency
//! list entirely rather than merely bridging it, matching L19: a plain
//! `u128` does the job `forge-vcs-v3::tape::TapeRow` already proves).
//!
//! Separate from `forge-ump-v3`'s router telemetry: that is per-route NDE
//! router training signal, this is the full intent/unit/outcome audit for
//! replay and debugging.

use crate::outcome::{DispatchOutcome, VetoDetail};
use crate::snapshot::SnapshotHandle;
use serde::{Deserialize, Serialize};

/// Schema version stamped on every [`DaemonDispatchRecord`].
pub const DAEMON_DISPATCH_RECORD_VERSION: u16 = 1;

/// Full semantic audit record for one dispatched unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDispatchRecord {
    /// Schema version this record was written under.
    pub schema_version: u16,
    /// The intent this unit belongs to.
    pub intent_id: i64,
    /// The intent's original text.
    pub intent_text: String,
    /// sha256 of intent canonical bytes (hex, lowercase).
    pub intent_hash_hex: String,
    /// The dispatched unit's id.
    pub unit_id: i64,
    /// Sequential position within the parent intent.
    pub unit_seq: usize,
    /// Structured payload (full TaskNode serialized as JSON for audit trail).
    pub unit_payload: serde_json::Value,
    /// Routing lane, as [`crate::unit::UnitLane::as_u8`].
    pub lane: u8,
    /// sha256 of intent context blob (hex, lowercase). Empty if no context.
    pub context_blob_hash: String,
    /// Snapshot handle, if writes occurred.
    pub snapshot_handle: Option<SnapshotHandle>,
    /// This unit's dispatch outcome.
    pub outcome: DispatchOutcome,
    /// Veto details, if outcome was `Vetoed`.
    pub veto_details: Option<VetoDetail>,
    /// Expert that handled this unit.
    pub expert_name: String,
    /// Unix-ms when the transport received the intent.
    pub received_at_ms: u128,
    /// Unix-ms when the planner produced this unit.
    pub planned_at_ms: u128,
    /// Unix-ms when a pre-write snapshot was taken, if any.
    pub snapshotted_at_ms: Option<u128>,
    /// Unix-ms when this unit was dispatched to its expert.
    pub dispatched_at_ms: u128,
    /// Unix-ms when this unit reached a terminal state.
    pub completed_at_ms: u128,
    /// Summary of the authority lattice routing decision, if one ran.
    pub authority_decision: Option<AuthorityDecisionSummary>,
}

/// Summary of the authority lattice routing decision for one unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityDecisionSummary {
    /// Name of the expert that won the authority lattice vote.
    pub winning_expert: String,
    /// Lex-rank of the winning expert (lower = higher authority).
    pub lex_rank: u32,
    /// Strain score at dispatch time.
    pub strain_score: u16,
    /// Number of experts that participated in the vote.
    pub participant_count: usize,
}
