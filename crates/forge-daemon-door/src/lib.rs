//! forge-daemon-door: the 5D TCP control surface on port 13013.
//!
//! Exports the ForgeWire frame codec, protocol messages (integer-only),
//! and the whitelist-enforced accept/dispatch loop. No inference, no mutating verbs.

#![deny(missing_docs)]
#![allow(unsafe_code)]

pub mod wire;
pub mod protocol;
pub mod door;
pub mod staleness;
/// Two-lane door-verb doctrine (2026-08-22) — `.ron` spec parser + shared
/// runtime encode/decode helpers. See `codegen.rs`'s own module doc for the
/// full scope statement (proof-of-concept pass, not yet spliced into the
/// live `TOOL_TABLE`/`DaemonMsg`/dispatch).
pub mod codegen;
/// Lane RON specs (`*.ron`) + Lane WELD handlers (`*.rs`), one sibling pair
/// per verb, plus the generated extension namespace. `mod.rs` only declares
/// submodules — no logic of its own (L05: that's `codegen.rs`).
pub mod verbs;
pub mod egress;
pub mod gemma_client;
pub mod oracle_escalate;
pub mod platform;
pub mod singleton;
pub mod timeline_futuresight;
pub mod timeline_recorder;
pub mod winproc;
pub mod ghost_reaper;

// The textually mounted `beat_batch.rs` below says `use forge_ump::…` — its
// one home compiles inside forge-audio-v3, which names the dep that way. An
// extern-crate rename is a crate-wide path root the mounted submodule can
// see (a crate-root alias `mod` cannot be, and cargo refuses one package
// under two dependency names — both measured 2026-08-16).
extern crate forge_ump_v3 as forge_ump;

/// `BeatBatch`'s ONE home (L05) is
/// `crates/forge-audio-v3/src/sovereign_comms/beat_batch.rs` (behind that
/// crate's `sovereign-broadcast` feature). Mounted textually: no cargo edge —
/// depending on forge-audio-v3 would pull the whole audio engine into the
/// daemon for one 32-byte-frame codec — and no second definition on disk.
#[path = "../../forge-audio-v3/src/sovereign_comms/beat_batch.rs"]
pub mod beat_batch;
pub mod nostr_lane;
pub mod beacon_valve;
pub mod mma_nostr;

// Drained from `F:\NewRepo\crates\forge-daemon-types` (2026-08-15) — the
// closed-loop control-plane types the daemon, engine, and door speak.
// `swarm.rs` was NOT ported: its `SemanticPrimitiveResponse` duplicates
// `protocol::DaemonMsg::QuerySemanticPrimitive`'s already-live (stubbed)
// wire op, and porting its serde types would contradict `protocol.rs`'s own
// stated law ("integer-only, hand-rolled codec (no serde)"). Filling that
// stub for real needs a v3-native invariants file first — a separate task.
pub mod atom;
pub mod audit;
pub mod codebook;
pub mod intent;
pub mod outcome;
pub mod semantic;
pub mod snapshot;
pub mod transport;
pub mod unit;

pub use wire::{FrameHeader, FRAME_MAGIC, HEADER_LEN, MAX_FRAME_LEN, KIND_CALL, KIND_RESULT, KIND_FAULT};
pub use protocol::{DAEMON_ADDR, DaemonMsg, daemon_addr};
pub use door::{Whitelist, WhitelistError};
pub use intent::{Intent, IntentHash};
pub use transport::{Transport, TransportError};
pub use unit::{AtomicUnit, UnitId, UnitLane};
pub use outcome::{DispatchOutcome, IntentResult, IntentStatus, UnitOutcome, UnitStatus, VetoDetail};
pub use snapshot::SnapshotHandle;
pub use audit::{AuthorityDecisionSummary, DaemonDispatchRecord, DAEMON_DISPATCH_RECORD_VERSION};
pub use semantic::{
    AdjudicatedMeaningSet, ConfidenceTrace, ContextKey, DaemonLane, DebateAgreement, ExpertAxis,
    ExpertDebate, ExpertId, ExpertVote, MeaningId, PlanningMode, PriorKey, QuantizedScore,
    RejectedMeaning, ResolvedMeaning, RouteRequest, SymbolName, clamp_confidence,
    compute_final_confidence, planning_mode_for,
};
