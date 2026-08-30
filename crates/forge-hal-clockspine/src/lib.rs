//! forge-hal-clockspine — the v3 deterministic time/event spine.
//!
//! Hot-path determinism seam under ADR-013: MetronomeClock (u64 120 Hz, integer
//! ticks, never floats), spine replay-hash (same inputs → same hash → bit-exact
//! replay), TripleBuffer clock-isolation, and tick warden. All integer-only, no
//! floats, no heap churn in the hot path.
//!
//! # Modules
//!
//! * [`attention`] — AttentionQueue: what the world will notice and when,
//!   drained inside the a000 recognition budget (0.13 ms, test-held).
//! * [`epoch_arena`] — EpochArena: a fixed-capacity bump allocator that resets
//!   every T1 tick (ported 2026-08-14, verbatim from v2's `forge-hal`).
//! * [`expert_pool`] — MoeRouter/MoeRouterSoA: generic, width-generic,
//!   integer-only nearest-neighbour MoE routing (ported 2026-08-13, the
//!   `bq_ep16` weld).
//! * [`fixed`] — SimTick and Permyriad newtypes; the primitives of determinism.
//! * [`metronome`] — MetronomeClock: the 120 Hz master heartbeat.
//! * [`spine`] — Event spine: Primitives, Stacks, Sequencers; tick-stamped, laned,
//!   integer events stacking per unit. Replay-hashed for bit-exact replay.
//! * [`triple_buffer`] — Lock-free clock bridge: 1 producer, 2 consumers, zero
//!   steady-state heap churn.
//! * [`staleness`] — StalenessTier: generation-age model for feedback-channel staleness
//!   (ported from sf-wasm::ripple.rs, adapted for scrub-bar feedback chain urgency).
//! * [`tick_warden`] — MetronomeWarden: fire/collect GPU work on tick boundaries,
//!   non-blocking.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod attention;
pub mod collision_bridge;
pub mod epoch_arena;
pub mod expert_pool;
pub mod fixed;
pub mod metronome;
pub mod mom_router;
pub mod nipr;
pub mod spine;
pub mod staleness;
pub mod tick_warden;
pub mod triple_buffer;

pub use attention::{AttentionEvent, AttentionQueue, ATTENTION_BUDGET_US, ATTENTION_CAP};
pub use collision_bridge::{CollisionBridge, ResonanceImpulse};
pub use epoch_arena::EpochArena;
pub use expert_pool::{hamming, MoeCell, MoeRouter, MoeRouterSoA};
pub use fixed::{Permyriad, SimTick};
pub use metronome::{MetronomeClock, TickAccumulator};
pub use spine::{
    audio_brush_stroke, derive_thesias, fires_on, Modality, Primitive, Sequencer, Stack,
    MAX_STACK, SEQ_CAP,
};
pub use staleness::{score_from_age, StalenessTier};
pub use tick_warden::{BudgetManifest, DispatchFence, DispatchTicket, FenceOutcome, FenceState, MetronomeWarden, Warden};
pub use triple_buffer::{ClockPlane, TripleBuffer};
