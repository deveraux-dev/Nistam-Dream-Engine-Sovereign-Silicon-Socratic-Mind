//! pp-math-v3 — the INTEGER math home: fixed-point types, formation geometry,
//! and integer-native spectral/eigen work.
//!
//! ONE HOME (L05, Sean 2026-08-24): the seven f64 hazard-equation modules that
//! used to live here (fluid, atmospheric, thermal, electrical, structural,
//! catastrophic, psychrometric) were TWINS of `forge-pp-lore-v3`'s, which is the
//! published float lane per `.forge/domains.tsv` LANE FLOAT and
//! `forge-mud-v3/Cargo.toml`'s named exception 5. Every live consumer of this
//! crate imports only `fixed_point`, `formation` or `spectral`; the f64 twins
//! had zero callers. See `_attic/2026-08-24/pp-math-v3-f64-twins/RESTORE.md`.

// Workspace law is `missing_docs = "deny"` — this verbatim port trips it
// (v2 pp-math never carried doc-comment discipline). Allowed explicitly,
// same reasoning as forge-arena-v3's lib.rs: real doc-writing is a separate,
// later pass, not 100+ filler comments to satisfy the lint syntactically.
#![allow(missing_docs)]

pub mod fixed_point;
pub mod formation;
pub mod spectral; // integer-native power iteration — eigenpairs the sim can replay
pub mod power_iteration; // integer-native dominant eigenpair + resolvent pole (Sean 08-02): no f32, no sqrt, bit-identical on x86/ARM/WASM

pub use fixed_point::{MilliUnit, Permyriad, Vec2Milli, Vec3Milli};
