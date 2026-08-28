//! forge-items-v3 — deterministic procedural item generation, ported from
//! `astrakey_item_forge_rs` (E:\.airgap milestones 2026-05-26 + 2026-06-15,
//! merged verbatim per this session's 3-recon-fan-out — the two snapshots are
//! complementary, not competing: 05-26 carries catalog.rs/drop_table.rs/
//! validation.rs, 06-15 carries forge.rs/icon.rs/item.rs/rng.rs/
//! stats_display.rs + the data/ tables. No signature drift found between them.
//!
//! Zero external dependencies (the donor's own Cargo.toml: "No external
//! crates are required") — `rng.rs`'s `Mulberry32` is proven algorithm-
//! identical to `forge_core::Mulberry32` (golden-value test in rng.rs), so
//! this crate's seeds are drop-in compatible with the rest of the workspace.
//!
//! Card-game bridge (Sean 2026-08-18, ASPIRE row `soulword-card-address-
//! bridge`): `ItemForge::generate_sword(seed: u64)` takes a bare `u64` and
//! doesn't care how it was derived — feeding it a `TritCell5D`-address-
//! derived seed (243 interior values) instead of an arbitrary one is the
//! whole bridge; nothing in this crate needs to change for that to work.
//!
//! Blanket allow instead of per-item churn on donor-verbatim, tested code
//! (same pattern `camera_lens.rs` documents) — the donor never carried doc
//! comments on its struct fields/enum variants, and this workspace's
//! `missing_docs` lint would otherwise demand 222 mechanical additions to
//! code ported byte-for-byte from a tested, working crate.
#![allow(missing_docs)]

pub mod catalog;
pub mod drop_table;
pub mod forge;
pub mod icon;
pub mod item;
pub mod rng;
pub mod stats_display;
pub mod validation;

pub use forge::{ForgeConfig, ItemForge};
pub use icon::{
    IconRef, icon_for_element, icon_for_slot, icon_for_tier, icon_for_item_id, resolve_item_icon,
    ICON_STAT_VIGOR, ICON_STAT_MOMENTUM, ICON_STAT_LOGIC_DEPTH, ICON_STAT_SHADOW_WEIGHT,
    ICON_STAT_TARNISH, ICON_STAT_RESONANCE, ICON_STAT_GUILT, ICON_STAT_CLARITY,
};
pub use item::{CatalogPayload, Damage, Defense, Element, Item, ItemSlot, ItemStats, Part, PartKind, SocketKind};
pub use stats_display::{
    StatId, StatDisplay, StatFormat, StatColourRole, STAT_DISPLAY,
    display_for, stat_value, format_stat, format_all_stats, format_nonzero_stats,
};
