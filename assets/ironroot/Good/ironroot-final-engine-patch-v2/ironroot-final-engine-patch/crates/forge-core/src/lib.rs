//! Minimal forge-core surface needed by the UMP alpha patch.
//!
//! In the full repo, merge these definitions into the canonical forge-core crate.

pub mod brutal_hash;
pub mod spine;

pub use brutal_hash::{brutal_hash64, BrutalHash, BrutalHashInput};
pub use spine::{CarrierKind, Lane};
