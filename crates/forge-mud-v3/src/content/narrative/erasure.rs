//! Erasure System — Name-Shear events and their consequences.
//!
//! Erasure is not deletion. It is relational collapse.
//! A name removed from the ledger makes the dead legally unhappened.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Scale of an erasure event from minor to catastrophic.
pub enum ErasureSeverity {
    /// Surface mark removed (graffiti, minor record).
    Surface,
    /// Name removed from one ledger.
    Partial,
    /// Name removed from all public records.
    Complete,
    /// Name + grave + witnesses silenced.
    Absolute,
    /// Metaphysical: entity's relations collapse.
    Ontological,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Who or what is performing an erasure.
pub enum ErasureAgent {
    /// The Ledger Church (institutional erasure).
    LedgerChurch,
    /// The Root Pruners (environmental erasure).
    RootPruners,
    /// The player character (volitional erasure).
    Player,
    /// The Shadow (malevolent erasure).
    Shadow,
    /// Pure entropy (systemic erasure).
    Entropy,
}

#[derive(Debug, Clone, Copy)]
/// A record of an erasure event (name removed from existence).
pub struct ErasureEvent {
    /// The entity being erased.
    pub target_entity: u64,
    /// Hash of the name being erased.
    pub target_name_hash: u64,
    /// How complete the erasure is.
    pub severity: ErasureSeverity,
    /// Who/what initiated the erasure.
    pub agent: ErasureAgent,
    /// Which cycle this erasure occurred in.
    pub cycle_id: u32,
    /// Game tick when the erasure occurred.
    pub tick: u64,
    /// Zone where the erasure took effect.
    pub zone_id: u16,
    /// How many witnesses were silenced.
    pub witnesses_silenced: u8,
    /// Whether the erasure was prevented/interrupted.
    pub prevented: bool,
}

/// Effects of an erasure on world state.
#[derive(Debug, Clone, Copy, Default)]
pub struct ErasureEffects {
    /// Change to world memory integrity (negative = damage).
    pub memory_integrity_delta: i8,
    /// Change to public fear level.
    pub public_fear_delta: i8,
    /// Change to shadow pressure (how much it advances).
    pub shadow_pressure_delta: i8,
    /// Entropy cost of performing the erasure.
    pub entropy_cost: u16,
    /// Change to root bloom (cycle acceleration).
    pub root_bloom_delta: i8,
    /// Change to spirit leak (otherworldly presence).
    pub spirit_leak_delta: i8,
}

/// Calculate the world effects of an erasure at a given severity level.
pub fn erasure_effects(severity: ErasureSeverity, prevented: bool) -> ErasureEffects {
    if prevented {
        return ErasureEffects {
            memory_integrity_delta: 2,
            public_fear_delta: -1,
            shadow_pressure_delta: 1,
            entropy_cost: 3,
            ..Default::default()
        };
    }
    match severity {
        ErasureSeverity::Surface => ErasureEffects {
            memory_integrity_delta: -1,
            public_fear_delta: 1,
            entropy_cost: 2,
            ..Default::default()
        },
        ErasureSeverity::Partial => ErasureEffects {
            memory_integrity_delta: -3,
            public_fear_delta: 3,
            shadow_pressure_delta: 2,
            entropy_cost: 5,
            ..Default::default()
        },
        ErasureSeverity::Complete => ErasureEffects {
            memory_integrity_delta: -8,
            public_fear_delta: 6,
            shadow_pressure_delta: 5,
            entropy_cost: 10,
            root_bloom_delta: 2,
            spirit_leak_delta: 1,
        },
        ErasureSeverity::Absolute => ErasureEffects {
            memory_integrity_delta: -15,
            public_fear_delta: 10,
            shadow_pressure_delta: 8,
            entropy_cost: 15,
            root_bloom_delta: 5,
            spirit_leak_delta: 3,
        },
        ErasureSeverity::Ontological => ErasureEffects {
            memory_integrity_delta: -25,
            public_fear_delta: 15,
            shadow_pressure_delta: 12,
            entropy_cost: 20,
            root_bloom_delta: 10,
            spirit_leak_delta: 8,
        },
    }
}

// ── Major Erasure Schedule ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Phases of a major erasure event (warning and opportunity to interrupt).
pub enum MajorErasurePhase {
    /// Hidden priming — world shows subtle signs.
    Priming,
    /// Warning cues — diegetic signals intensify.
    Warning,
    /// Active — player can interrupt before finality.
    Active,
    /// Aftermath — consequences locked in, cannot be undone.
    Aftermath,
}

#[derive(Debug, Clone, Copy)]
/// A scheduled major erasure event with defined phases.
pub struct MajorErasure {
    /// The entity that is scheduled for erasure.
    pub target_entity: u64,
    /// Current phase of the erasure event.
    pub phase: MajorErasurePhase,
    /// Game tick when priming starts.
    pub priming_tick: u64,
    /// Game tick when warning signs appear.
    pub warning_tick: u64,
    /// Game tick when player can interrupt.
    pub active_tick: u64,
    /// Game tick when event becomes final.
    pub deadline_tick: u64,
    /// Whether the erasure has been prevented.
    pub prevented: bool,
    /// How severe the erasure will be if completed.
    pub severity: ErasureSeverity,
}

impl MajorErasure {
    /// Construct a new major erasure scheduled starting at a given tick.
    pub fn new(target: u64, start_tick: u64, severity: ErasureSeverity) -> Self {
        Self {
            target_entity: target,
            phase: MajorErasurePhase::Priming,
            priming_tick: start_tick,
            warning_tick: start_tick + 3000,  // ~50 seconds at 60Hz
            active_tick: start_tick + 6000,
            deadline_tick: start_tick + 9000,
            prevented: false,
            severity,
        }
    }

    /// Advance the erasure's phase based on current game tick.
    pub fn advance(&mut self, current_tick: u64) {
        if self.prevented { return; }
        if current_tick >= self.deadline_tick {
            self.phase = MajorErasurePhase::Aftermath;
        } else if current_tick >= self.active_tick {
            self.phase = MajorErasurePhase::Active;
        } else if current_tick >= self.warning_tick {
            self.phase = MajorErasurePhase::Warning;
        }
    }

    /// Mark this erasure as prevented/interrupted.
    pub fn prevent(&mut self) {
        self.prevented = true;
    }

    /// Check if the player can currently interrupt this erasure.
    pub fn is_interruptible(&self) -> bool {
        self.phase == MajorErasurePhase::Active && !self.prevented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasure_severity_scaling() {
        let surface = erasure_effects(ErasureSeverity::Surface, false);
        let absolute = erasure_effects(ErasureSeverity::Absolute, false);
        assert!(absolute.entropy_cost > surface.entropy_cost);
        assert!(absolute.memory_integrity_delta < surface.memory_integrity_delta);
    }

    #[test]
    fn prevention_reduces_damage() {
        let prevented = erasure_effects(ErasureSeverity::Complete, true);
        let happened = erasure_effects(ErasureSeverity::Complete, false);
        assert!(prevented.memory_integrity_delta > happened.memory_integrity_delta);
    }

    #[test]
    fn major_erasure_phase_progression() {
        let mut me = MajorErasure::new(42, 1000, ErasureSeverity::Complete);
        assert_eq!(me.phase, MajorErasurePhase::Priming);
        me.advance(4000);
        assert_eq!(me.phase, MajorErasurePhase::Warning);
        me.advance(7000);
        assert_eq!(me.phase, MajorErasurePhase::Active);
        assert!(me.is_interruptible());
        me.advance(10000);
        assert_eq!(me.phase, MajorErasurePhase::Aftermath);
    }

    #[test]
    fn prevention_stops_progression() {
        let mut me = MajorErasure::new(42, 1000, ErasureSeverity::Absolute);
        me.advance(7000);
        assert_eq!(me.phase, MajorErasurePhase::Active);
        me.prevent();
        me.advance(10000);
        assert_eq!(me.phase, MajorErasurePhase::Active); // frozen
        assert!(!me.is_interruptible()); // prevented
    }

    #[test]
    fn ontological_erasure_is_catastrophic() {
        let e = erasure_effects(ErasureSeverity::Ontological, false);
        assert_eq!(e.memory_integrity_delta, -25);
        assert_eq!(e.spirit_leak_delta, 8);
    }
}
