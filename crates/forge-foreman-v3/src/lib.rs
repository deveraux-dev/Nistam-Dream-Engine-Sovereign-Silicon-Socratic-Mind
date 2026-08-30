//! forge-foreman-v3 — the deterministic orchestrator of HANDOFF §11, MIGRATION §M2.
//!
//! The loop, exactly as specced: take the next census item → brief the sidecar
//! (grind work: read v2 source, draft the syntax-level port) → land the draft →
//! run `pipeline.gate` → on green, commit stamped rows and advance; on red,
//! bounded retries, then queue a brief for `claude -p` — fail-closed, a queue
//! survives the night.
//!
//! ## Boundaries this crate keeps
//!
//! - **The foreman owns all filesystem I/O.** The sidecar is inference only:
//!   it is spoken to over loopback TCP frames and never touches a file.
//! - **Drafts are Speculative until the gate speaks.** A draft lands in a
//!   staging workspace under `target/foreman/stage/` — outside the main
//!   workspace — and is promoted into `crates/` only after the gate runs green
//!   there. A red draft never enters the workspace, so the tree's own gates
//!   stay green while the sidecar is wrong (which is often).
//! - **Provenance is stamped, not defaulted.** Gated sidecar source commits as
//!   `PriorAuthority`/`LLMCandidate`/`Compile`; the census flip commits with
//!   `ReceiptKind::Promote` (MIGRATION §LANE DELEGATION). The `tape` driver's
//!   hand trio stays what it always was — hand-committed source.
//! - **The fail path writes, it does not wait.** After
//!   `pipeline.on_red`'s retry budget, the brief and the failing draft move to
//!   `.forge/brief-queue/<crate>/` and the census row flips to `queued`.
//!   Nothing pings an attended human; nothing vanishes.
//!
//! Mechanism values (endpoint, gate command, retry budget) come from
//! `.forge/v3-directives.ron` and a missing key is an error, never a default.

pub mod amortize_trigger;
pub mod arbiter;
pub mod beat_status;
pub mod census;
pub mod claim;
pub mod client;
pub mod directives;
pub mod dauer;
pub mod drift;
pub mod flywheel;
pub mod flywheel_beat;
pub mod gate;
pub mod hook;
pub mod land;
pub mod oracle;
pub mod queue;
pub mod rolls;
pub mod run;
pub mod sidecar_launch;
pub mod sip;
pub mod staleness;
pub mod velocity;
pub mod tripwire;
pub mod receipt;
pub mod weld;
pub mod weld_lane;
