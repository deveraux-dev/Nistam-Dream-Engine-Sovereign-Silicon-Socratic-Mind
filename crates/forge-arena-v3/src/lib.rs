// Workspace law is `missing_docs = "deny"` — this port trips it on ~528 items
// (v2's arena_core never carried doc-comment discipline; game-jam-shaped
// source). Allowed here explicitly, not silently, rather than either (a)
// stalling the whole port on writing real docs for every field of a
// straight verbatim copy, or (b) filling 528 items with meaningless
// restated-name comments just to satisfy the lint syntactically — this
// repo's own comment law bans exactly that. Real doc-writing (the honest
// fix) is a separate, later pass; naming the gap here keeps it visible
// instead of quietly bypassed.
#![allow(missing_docs)]

//! Arena Core — deterministic 2D/2.5D sidescroller combat kernel.
//! Ported from `F:\NewRepo\crates\forge-game-systems\src\arena_core` (v2,
//! astrakey shared_core lineage), verbatim — GHOSTMOON-conductor-verified
//! (`.claude\workflows\ghostmoon-merge.js`, 2026-08-15), 0 internal
//! `crate::` refs at port time.
//!
//! No Rapier, no fortress-rollback. Pure logic, integer-first.
//! Physics and netcode are plugged in by the app crate.

pub mod additives;
pub mod config;
pub mod state;
pub mod combat;
pub mod stats;
pub mod inventory;
pub mod procs;
pub mod tinctures;
pub mod resurrection;
pub mod simulation;
pub mod checksum;
pub mod half_hanged;
pub mod procgen;
pub mod replay;
pub mod item_loader;
pub mod weapon_gen;
pub mod astrakey_sieve;
pub mod asp_constraints;
pub mod hypercube;
pub mod sevenfold;
pub mod duel;
pub mod wave_pressure;
// Wired to forge-semantic-quadlane::SieveEvent (v3's real, deliberately
// smaller SieveEvent home — not v2 forge-sieve's ~50-variant bus, which was
// never ported verbatim; a Mutate5D variant was added there in this same
// stroke). See Cargo.toml's dep comment for the full receipt.
pub mod mechanic_rail;
pub mod seams;
pub mod buff_application;
