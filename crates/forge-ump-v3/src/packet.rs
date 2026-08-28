//! POD-safe packet types — re-export of the floor definition.
//!
//! `Ump` / `Group` / `Channel` / `Stamped<T>` were relocated down to
//! `forge_core_v3::spine::packet` on 2026-08-14 (Sean "we brought bytemuck already"),
//! completing the same relocation v2 did on 2026-07-12. This module re-exports them
//! so every `forge_ump_v3::packet::…` and `forge_ump_v3::{Ump, Channel, Group, Stamped}`
//! path is unchanged for consumers of this crate.

pub use forge_core_v3::spine::packet::*;
