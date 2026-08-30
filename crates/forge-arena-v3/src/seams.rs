//! Merge seams documentation — cross-domain integrations and design choices.
//!
//! ## SEAM: BuffType Merger from pp-sieve (2026-08-19)
//!
//! **Donor:** E:\.airgap\milestones\13forge-consolidation-2026-06-15\pp-server\crates\pp-sieve\src\buffs.rs
//!
//! **Merged into:** TinctureBuff, TinctureBuffType (tinctures.rs)
//!
//! ### Capabilities Merged:
//! - Added `amount: i32` field to TinctureBuff for permyriad/per-tick modifiers (ported from ActiveBuff.amount).
//! - Added `total_modifier(buffs, buff_type) -> i32` — sums active buff amounts of a given type.
//! - Added `tick_buffs(buffs) -> Vec<TinctureBuffType>` — decrements ticks, clears expired, returns expired types.
//!
//! ### Deliberately Left Un-merged:
//! - BuffType vocabulary: donor has 19 variants (AttackUp/Down, Defense, Speed, Stealth, Vision, Debuffs, Regen).
//!   TinctureBuffType stays at 5 variants (None, QuicksilverSpeed, GiantMass, ShrinkMass, CrucibleTrial) —
//!   a deliberately smaller, focused vocabulary for potion effects only. Earthcalling aspects (Bear armor, Fox stealth, etc.)
//!   remain donor-domain; no cross-pollination intended.
//! - MAX_ACTIVE_BUFFS: stays at 4 (forge-arena's design) vs. donor's 8. Not a bug; deliberate scope.
//! - Donor's source_id (String) and element (Option<String>) fields: not merged (arena tinctures need no attribution/element tags).
//! - `apply_buff` overflow logic (replace-shortest): arena's `find_empty_buff_slot` + existing apply_tincture_effect pattern
//!   sufficient; no merge needed.
//! - `calculate_buffed_stat` permyriad formula: donor supplies reference; arena doesn't yet use stacking buffs.
//!   Available if future stat-modifers are added (not this wave).
//!
//! **Proof:** all 3 pre-existing tests (basilicon_heals_capped, void_extract_spikes_opponent, crucible_reduces_hp)
//! + 5 new tests for total_modifier/tick_buffs pass; cargo build/test clean.
