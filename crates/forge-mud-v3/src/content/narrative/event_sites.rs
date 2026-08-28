//! Event Sites — 13 zone events from the forward map.
//!
//! Each zone has one event site. Events are resolved via EventResolver.
//! This module provides the zone→event mapping and resolution dispatch.

use super::event::{
    EventState, EventResolverInput, EventResolverOutput,
    ResolutionMode, resolve_event, apply_resolution,
};
use super::state::{PlayerState, WorldState};
use crate::faction_mind::Faction;

/// Zone-to-event mapping for all 13 zones (event ID and controlling faction).
pub const ZONE_EVENTS: [(u16, Option<Faction>); 13] = [
    (0, Some(Faction::LedgerChurch)),
    (1, Some(Faction::TollSaints)),
    (2, Some(Faction::RootPruners)),
    (3, Some(Faction::IndexMonks)),
    (4, Some(Faction::AnvilCovenant)),
    (5, Some(Faction::WidowCourts)),
    (6, Some(Faction::HollowAstronomers)),
    (7, Some(Faction::FreeGraves)),
    (8, Some(Faction::LedgerChurch)),
    (9, Some(Faction::RootPruners)),
    (10, None), // Spirit zone — no faction
    (11, None), // Shadow zone — no faction
    (12, None), // Root zone — no faction
];

/// Initialize event states for all 13 zones with faction ownership.
pub fn init_zone_events() -> [EventState; 13] {
    core::array::from_fn(|i| {
        let mut ev = EventState::new(i as u16);
        ev.faction_owner = ZONE_EVENTS[i].1;
        ev
    })
}

/// Resolve a zone event with a given resolution mode; returns resolver output or None if unresolvable.
pub fn resolve_zone_event(
    zone_idx: usize,
    events: &[EventState; 13],
    player: &PlayerState,
    world: &WorldState,
    resolution: ResolutionMode,
) -> Option<EventResolverOutput> {
    if zone_idx >= 13 { return None; }
    let event = &events[zone_idx];
    if event.is_resolved() { return None; }

    let input = EventResolverInput { event, player, world, resolution };
    Some(resolve_event(&input))
}

/// Apply event resolution output and update zone event state.
pub fn commit_zone_event(
    zone_idx: usize,
    events: &mut [EventState; 13],
    player: &mut PlayerState,
    world: &mut WorldState,
    resolution: ResolutionMode,
    output: &EventResolverOutput,
) {
    if zone_idx >= 13 { return; }
    apply_resolution(world, player, output);
    events[zone_idx].resolved_mode = Some(resolution);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_assigns_factions() {
        let events = init_zone_events();
        assert_eq!(events[0].faction_owner, Some(Faction::LedgerChurch));
        assert_eq!(events[10].faction_owner, None);
    }

    #[test]
    fn resolve_unresolved_event() {
        let events = init_zone_events();
        let player = PlayerState::default();
        let world = WorldState::default();
        let out = resolve_zone_event(0, &events, &player, &world, ResolutionMode::Spare);
        assert!(out.is_some());
    }

    #[test]
    fn cannot_resolve_twice() {
        let mut events = init_zone_events();
        let mut player = PlayerState::default();
        let mut world = WorldState::default();
        let out = resolve_zone_event(0, &events, &player, &world, ResolutionMode::Kill).unwrap();
        commit_zone_event(0, &mut events, &mut player, &mut world, ResolutionMode::Kill, &out);
        assert!(resolve_zone_event(0, &events, &player, &world, ResolutionMode::Spare).is_none());
    }
}
