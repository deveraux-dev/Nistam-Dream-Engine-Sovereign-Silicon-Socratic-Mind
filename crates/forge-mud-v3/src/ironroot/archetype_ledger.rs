//! CYOA archetype recording: entity-generic ledger bridge for choice pressures.

use crate::ironroot::cyoa::{ChoiceArchetype, archetype_pressure};
use crate::overlay::{Domain, Ledger, Mod, OverlayEntry, Scope};

/// Archetype pole key (one, shared across all archetypes).
const POLE_KEY: u16 = 0;
/// Archetype art keys (one per art, base + art index 0..6).
const ART_KEY_BASE: u16 = 1;

/// Record a choice archetype into the ledger under the given scope.
/// Appends pole tally and nonzero art deltas to Domain::Archetype.
pub fn record_choice(ledger: &mut Ledger, scope: Scope, _seed: u64, a: ChoiceArchetype) {
    let p = archetype_pressure(a);
    ledger.append(OverlayEntry {
        domain: Domain::Archetype,
        key: POLE_KEY,
        modification: Mod::Accumulate(p.pole_q as i64),
        priority: 0,
        scope,
    });
    for (art, delta) in p.art_delta.0.iter().enumerate() {
        if *delta != 0 {
            ledger.append(OverlayEntry {
                domain: Domain::Archetype,
                key: ART_KEY_BASE + art as u16,
                modification: Mod::Accumulate(*delta as i64),
                priority: 0,
                scope,
            });
        }
    }
}

/// Read the dominant pole tally from the ledger (signed force/water).
pub fn dominant_pole(ledger: &Ledger, seed: u64) -> i64 {
    ledger.resolve_i64(Domain::Archetype, POLE_KEY, seed, 0)
}

/// Read a single art's total delta from the ledger.
pub fn art_delta(ledger: &Ledger, art: usize, seed: u64) -> i64 {
    if art >= 7 { return 0; }
    ledger.resolve_i64(Domain::Archetype, ART_KEY_BASE + art as u16, seed, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ironroot::cyoa::ChoiceArchetype;

    #[test]
    fn record_and_read_pole() {
        let mut ledger = Ledger::default();
        record_choice(&mut ledger, Scope::Operator, 42, ChoiceArchetype::Cut);
        let pole = dominant_pole(&ledger, 42);
        assert_eq!(pole, 1500);
    }

    #[test]
    fn record_and_read_art_delta() {
        let mut ledger = Ledger::default();
        record_choice(&mut ledger, Scope::Operator, 42, ChoiceArchetype::Trick);
        assert_eq!(art_delta(&ledger, 0, 42), 0);
        assert_eq!(art_delta(&ledger, 1, 42), 2);
        assert_eq!(art_delta(&ledger, 2, 42), 1);
    }

    #[test]
    fn accumulate_multiple_choices() {
        let mut ledger = Ledger::default();
        record_choice(&mut ledger, Scope::Operator, 42, ChoiceArchetype::Cut);
        record_choice(&mut ledger, Scope::Operator, 42, ChoiceArchetype::Trick);
        let pole = dominant_pole(&ledger, 42);
        assert_eq!(pole, 1500 + 500);
        assert_eq!(art_delta(&ledger, 1, 42), 1 + 2);
    }
}
