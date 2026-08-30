//! Session-close topological order check for the Floor->Circuit->Surface DAG.
//!
//! Ported from aspire.rs:323 (CADENCE lane, `toposort-phase-dag`, NEXT) —
//! minus the `petgraph` dependency the aspire row's mechanism names. Checked
//! against `Cargo.lock` (2026-08-15): `petgraph` is not a dependency of any
//! crate in this workspace, so the row's "WIRE Cargo.lock:8225 petgraph" is
//! stale. A 3-node DAG does not clear L19's bar for pulling a graph crate —
//! hand-rolled order comparison is strictly cheaper and needs no `unsafe`,
//! no transitive deps, and no external crate to audit.
//!
//! No wrap exception: the real cadence law (`crate::Phase::next`, corrected
//! against the live source `F:\NewRepo\crates\forge-book\src\session_cadence.rs`)
//! gives `Surface` no successor at all — a session seals, it does not walk
//! back into a new `Floor`. One call to [`visited_order_is_topological`]
//! checks one unbroken session; a caller checking several sessions calls it
//! once per session, never concatenates them.

use crate::Phase;

/// Does `order` visit every phase it contains in a topological order of the
/// fixed `Floor -> Circuit -> Surface` DAG?
///
/// A topological order requires each phase to appear no earlier than any
/// phase that must precede it — rank must never decrease across the run.
/// Empty input is vacuously topological.
pub fn visited_order_is_topological(order: &[Phase]) -> bool {
    let mut last_rank: Option<u8> = None;
    for &phase in order {
        let rank = phase.as_u8();
        if let Some(prev) = last_rank {
            if rank < prev {
                return false;
            }
        }
        last_rank = Some(rank);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Phase::*;

    #[test]
    fn empty_and_single_element_orders_are_topological() {
        assert!(visited_order_is_topological(&[]));
        for p in Phase::ALL {
            assert!(visited_order_is_topological(&[p]));
        }
    }

    #[test]
    fn the_canonical_forward_order_passes() {
        assert!(visited_order_is_topological(&[Floor, Circuit, Surface]));
    }

    #[test]
    fn a_skip_ahead_still_counts_as_forward_order() {
        // Skip-legality is advance()'s job, not topo's — topo only asks
        // "does rank move forward", and Floor(0) -> Surface(2) does.
        assert!(visited_order_is_topological(&[Floor, Surface]));
    }

    #[test]
    fn a_second_session_appended_in_the_same_call_fails() {
        // No wrap: Surface has no successor, so concatenating two sessions
        // into one order is a caller error this function must catch.
        assert!(!visited_order_is_topological(&[
            Floor, Circuit, Surface, Floor, Circuit, Surface
        ]));
    }

    #[test]
    fn running_backward_within_a_cadence_fails() {
        assert!(!visited_order_is_topological(&[Surface, Circuit]));
        assert!(!visited_order_is_topological(&[Circuit, Floor]));
        assert!(!visited_order_is_topological(&[
            Floor, Circuit, Surface, Circuit
        ]));
    }
}
