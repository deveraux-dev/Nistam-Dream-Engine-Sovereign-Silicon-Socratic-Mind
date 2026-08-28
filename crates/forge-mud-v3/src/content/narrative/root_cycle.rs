//! Root-Cycle — roguelike run structure with deterministic erasure schedules.
//!
//! A run is a Root-Cycle. Death opens routes. Erasure changes world-state.
//! Shadows learn habits. Factions move while the player acts elsewhere.

use crate::combat_brain::DeathCause;

// ── Cycle Events ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Major events that occur within a root cycle.
pub enum CycleEventKind {
    /// The cycle began.
    CycleStarted,
    /// The player died.
    PlayerDied,
    /// A spirit route was opened (death via sacrifice/fall).
    SpiritRouteOpened,
    /// An erasure event was scheduled.
    ErasureScheduled,
    /// An erasure event was executed.
    ErasureExecuted,
    /// An erasure event was prevented.
    ErasurePrevented,
    /// The Shadow evolved to a new form.
    ShadowEvolved,
    /// A faction moved/acted.
    FactionMoved,
    /// A route was unlocked for the player.
    RouteUnlocked,
    /// A route closed.
    RouteClosed,
    /// A name was restored.
    NameRestored,
    /// A name was lost/erased.
    NameLost,
}

#[derive(Debug, Clone, Copy)]
/// A single recorded event within a cycle.
pub struct CycleEvent {
    /// Type of event.
    pub kind: CycleEventKind,
    /// Game tick when event occurred.
    pub tick: u64,
    /// Which cycle this event belongs to.
    pub cycle_id: u32,
    /// Context-dependent payload (e.g., entity ID, count, etc.).
    pub data: u64,
}

// ── Root Cycle ───────────────────────────────────────────────────────────────

/// Maximum number of events that can be recorded in a cycle.
pub const MAX_CYCLE_EVENTS: usize = 128;

/// A roguelike run structure with deterministic erasure schedules.
pub struct RootCycle {
    /// Unique ID for this cycle/run.
    pub cycle_id: u32,
    /// Seed used for generating erasure schedule.
    pub seed: u64,
    /// Game tick at which the cycle started.
    pub start_tick: u64,
    /// Number of times the player died in this cycle.
    pub death_count: u16,
    /// Recorded events in this cycle.
    pub events: [CycleEvent; MAX_CYCLE_EVENTS],
    /// Current number of recorded events.
    pub event_count: u16,
    /// Scheduled game ticks for erasure events (up to 8).
    pub erasure_schedule: [u64; 8],
    /// Number of scheduled erasures in this cycle.
    pub erasure_count: u8,
    /// Whether the cycle is currently active.
    pub active: bool,
}

impl RootCycle {
    /// Construct a new cycle and generate erasure schedule.
    pub fn new(cycle_id: u32, seed: u64, start_tick: u64) -> Self {
        let mut cycle = Self {
            cycle_id,
            seed,
            start_tick,
            death_count: 0,
            events: [CycleEvent { kind: CycleEventKind::CycleStarted, tick: 0, cycle_id: 0, data: 0 }; MAX_CYCLE_EVENTS],
            event_count: 0,
            erasure_schedule: [0; 8],
            erasure_count: 0,
            active: true,
        };
        cycle.generate_erasure_schedule();
        cycle.record(CycleEventKind::CycleStarted, start_tick, 0);
        cycle
    }

    /// Generate deterministic erasure schedule from seed.
    fn generate_erasure_schedule(&mut self) {
        let mut h = self.seed;
        for i in 0..4u8 {
            h ^= (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
            h = h.wrapping_mul(0xBF58476D1CE4E5B9);
            h ^= h >> 27;
            // Schedule erasures between 30s and 5min (1800-18000 ticks at 60Hz)
            let offset = 1800 + (h % 16200);
            self.erasure_schedule[i as usize] = self.start_tick + offset;
            self.erasure_count += 1;
        }
    }

    /// Record a cycle event if capacity allows.
    pub fn record(&mut self, kind: CycleEventKind, tick: u64, data: u64) {
        if (self.event_count as usize) < MAX_CYCLE_EVENTS {
            self.events[self.event_count as usize] = CycleEvent {
                kind,
                tick,
                cycle_id: self.cycle_id,
                data,
            };
            self.event_count += 1;
        }
    }

    /// Record a player death and potentially unlock a spirit route.
    pub fn record_death(&mut self, tick: u64, cause: DeathCause) {
        self.death_count += 1;
        self.record(CycleEventKind::PlayerDied, tick, cause as u64);
        if matches!(cause, DeathCause::Sacrifice | DeathCause::Fall) {
            self.record(CycleEventKind::SpiritRouteOpened, tick, 0);
        }
    }

    /// Check if any scheduled erasure should fire at the current tick.
    pub fn pending_erasure(&self, current_tick: u64) -> Option<u8> {
        for i in 0..self.erasure_count as usize {
            if self.erasure_schedule[i] != 0 && current_tick >= self.erasure_schedule[i] {
                return Some(i as u8);
            }
        }
        None
    }

    /// Mark an erasure as fired and record whether it was prevented.
    pub fn consume_erasure(&mut self, index: u8, tick: u64, prevented: bool) {
        if (index as usize) < self.erasure_count as usize {
            self.erasure_schedule[index as usize] = 0;
            let kind = if prevented { CycleEventKind::ErasurePrevented } else { CycleEventKind::ErasureExecuted };
            self.record(kind, tick, index as u64);
        }
    }

    /// Mark the cycle as ended and record final state.
    pub fn end_cycle(&mut self, tick: u64) {
        self.active = false;
        self.record(CycleEventKind::RouteClosed, tick, self.death_count as u64);
    }

    /// Calculate elapsed game ticks since cycle start.
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.start_tick)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_generates_deterministic_schedule() {
        let a = RootCycle::new(1, 42, 1000);
        let b = RootCycle::new(1, 42, 1000);
        assert_eq!(a.erasure_schedule, b.erasure_schedule);
        assert_eq!(a.erasure_count, 4);
    }

    #[test]
    fn different_seed_different_schedule() {
        let a = RootCycle::new(1, 42, 1000);
        let b = RootCycle::new(1, 99, 1000);
        assert_ne!(a.erasure_schedule, b.erasure_schedule);
    }

    #[test]
    fn death_opens_spirit_route_on_sacrifice() {
        let mut cycle = RootCycle::new(1, 42, 0);
        let initial_count = cycle.event_count;
        cycle.record_death(100, DeathCause::Sacrifice);
        assert_eq!(cycle.death_count, 1);
        // Should have PlayerDied + SpiritRouteOpened
        assert_eq!(cycle.event_count, initial_count + 2);
    }

    #[test]
    fn combat_death_does_not_open_spirit_route() {
        let mut cycle = RootCycle::new(1, 42, 0);
        let initial_count = cycle.event_count;
        cycle.record_death(100, DeathCause::Combat);
        assert_eq!(cycle.event_count, initial_count + 1); // only PlayerDied
    }

    #[test]
    fn pending_erasure_fires_at_scheduled_tick() {
        let cycle = RootCycle::new(1, 42, 0);
        // No erasure at tick 0
        assert_eq!(cycle.pending_erasure(0), None);
        // Should fire at first scheduled tick
        let first_tick = cycle.erasure_schedule[0];
        assert!(cycle.pending_erasure(first_tick).is_some());
    }

    #[test]
    fn consumed_erasure_does_not_fire_again() {
        let mut cycle = RootCycle::new(1, 42, 0);
        let first_tick = cycle.erasure_schedule[0];
        let idx = cycle.pending_erasure(first_tick).unwrap();
        cycle.consume_erasure(idx, first_tick, false);
        // The consumed slot is now 0, so it won't match again
        assert_eq!(cycle.erasure_schedule[idx as usize], 0);
    }

    #[test]
    fn cycle_end_records_event() {
        let mut cycle = RootCycle::new(1, 42, 0);
        cycle.record_death(50, DeathCause::Combat);
        cycle.record_death(80, DeathCause::Combat);
        cycle.end_cycle(100);
        assert!(!cycle.active);
    }

    #[test]
    fn erasure_schedule_within_bounds() {
        let cycle = RootCycle::new(1, 12345, 0);
        for i in 0..cycle.erasure_count as usize {
            let tick = cycle.erasure_schedule[i];
            assert!(tick >= 1800, "Erasure too early: {}", tick);
            assert!(tick <= 18000, "Erasure too late: {}", tick);
        }
    }
}
