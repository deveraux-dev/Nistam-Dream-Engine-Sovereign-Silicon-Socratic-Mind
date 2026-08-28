//! forge-sieve-v3 — Prime Resonance Sieve beachhead.
//!
//! Ported from `F:\NewRepo\crates\forge-sieve` (`resonance.rs` + `prime_seed.rs`
//! only; see this crate's `Cargo.toml` for the scoping rationale). All values
//! integer-only; no floats, no HashMap, no allocation inside the hot fold loop.

pub mod resonance;
pub mod prime_seed;
pub mod combat;
/// World sieves: Land, Terrain, Weather, Moon, Ecology, Infection state machines.
pub mod world;
pub mod social;
pub mod npc_bq;
