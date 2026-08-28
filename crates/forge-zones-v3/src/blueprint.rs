//! Moved to `forge-core-v3::zones::blueprint` 2026-08-20 (L05 one-home —
//! the schema needed to be reachable without a new dependency edge, per
//! `forge-core-v3` being the workspace `dag_root`). Re-exported below so
//! every caller in this crate keeps working unchanged.

pub use forge_core_v3::zones::blueprint::*;
