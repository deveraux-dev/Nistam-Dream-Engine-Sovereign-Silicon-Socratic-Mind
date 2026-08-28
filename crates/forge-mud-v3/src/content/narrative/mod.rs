//! Ported verbatim from F:\NewRepo\crates\forge-game-systems\src\narrative\ (2026-08-17 truth-hunt lineage port).
//! Narrative State Machine — the full event/identity/ending/oath/shadow/entropy system.
//!
//! Deterministic, no-alloc on hot path. All state is integer-only.
//! This module implements the "missing 25 systems" from the design packet.

pub mod state;
pub mod oath;
pub mod oath_scaling;
pub mod identity;
pub mod entropy;
pub mod erasure;
pub mod event;
pub mod shadow_memory;
pub mod endings;
pub mod root_cycle;
pub mod discovery;
pub mod event_sites;
pub mod persistence;
pub mod lunar_harmonics;

use state::{PlayerState, WorldState};
use entropy::EntropyLedger;
use endings::EndingPressure;
use root_cycle::RootCycle;
use shadow_memory::ShadowMemory;
use discovery::ZoneDiscoveryState;
use event::EventState;

/// Aggregate narrative state — lives on GameCartridge.
pub struct NarrativeState {
    /// Player-facing narrative state (choices, oath status, identity).
    pub player: PlayerState,
    /// World-facing narrative state (time, season, cycle).
    pub world: WorldState,
    /// Entropy ledger tracking all decisions and their weights.
    pub entropy: EntropyLedger,
    /// Pressure towards specific endings based on player choices.
    pub pressure: EndingPressure,
    /// Root cycle (day/season/year progression).
    pub root_cycle: RootCycle,
    /// Shadow memory of past iterations and "echoes".
    pub shadow: ShadowMemory,
    /// Discovery state per zone (tells/evidence).
    pub zone_discovery: [ZoneDiscoveryState; 13],
    /// Event resolution state per zone.
    pub events: [EventState; 13],
    /// Item provenance: tracks which event sites produced artifacts (item_id → zone_idx).
    pub item_provenance: [(u32, u8); 32],
    /// Number of items in provenance array.
    pub provenance_count: u8,
    /// Tincture/craft entropy accumulator (incremented by host, drained into EntropyLedger).
    pub pending_item_entropy: u8,
    /// Current act (1-4). Derived from events resolved.
    pub act: u8,
    /// Active event prompt: Some((zone_idx, cursor_pos)) when player is choosing resolution.
    pub event_prompt: Option<(usize, u8)>,
}

impl NarrativeState {
    /// Construct a new narrative state with seeded zone discovery and events.
    pub fn new(seed: u64) -> Self {
        let mut zone_discovery: [ZoneDiscoveryState; 13] =
            core::array::from_fn(|i| ZoneDiscoveryState::new(i as u16));
        // Seed tells into each zone (4-8 tells per zone based on index)
        for i in 0..13 {
            let count = 4 + (i as u8 % 5); // 4-8 tells per zone
            discovery::seed_zone_tells(&mut zone_discovery[i], seed, count);
        }
        Self {
            player: PlayerState::default(),
            world: WorldState::default(),
            entropy: EntropyLedger::default(),
            pressure: EndingPressure::default(),
            root_cycle: RootCycle::new(1, seed, 0),
            shadow: ShadowMemory::default(),
            zone_discovery,
            events: core::array::from_fn(|i| EventState::new(i as u16)),
            item_provenance: [(0, 0); 32],
            provenance_count: 0,
            pending_item_entropy: 0,
            act: 1,
            event_prompt: None,
        }
    }

    /// Recompute act from events resolved.
    pub fn update_act(&mut self) {
        let resolved = self.events.iter().filter(|e| e.is_resolved()).count();
        self.act = if resolved >= 10 { 4 }
            else if resolved >= 7 { 3 }
            else if resolved >= 3 { 2 }
            else { 1 };
    }
}
