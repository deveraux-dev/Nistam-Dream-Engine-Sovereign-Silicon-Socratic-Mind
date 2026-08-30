//! The triadic session cadence (Floor/Circuit/Surface) as a compiled law,
//! instead of prose a human has to remember to follow.
//!
//! Ported verbatim (values and transition law) from the REAL live source —
//! `F:\NewRepo\crates\forge-book\src\session_cadence.rs` (2026-08-15) — not
//! from aspire.rs:317's summary of it, which turned out to be stale: the
//! aspire row's mechanism text still names the pre-08-02 budget `[4000,
//! 3000,3000]`; Sean changed it to `[3000,3000,4000]` on 08-02 specifically
//! because funding the floor over the shipping rung starves the only rung
//! that can close a wave. This crate carries the corrected, live values.
//!
//! [`DONE_INVARIANT`]/[`ShipRungs`]/[`ship_check`] are the load-bearing
//! half of the source module — they operationalize this repo's own
//! `CLAUDE.md` witness law (`WAVE_CLOSE=PHOTON`) as a gate a caller cannot
//! satisfy by naming a green test alone.
//!
//! Not ported: the source's `cadence_chapter()`/`Chapter`/`AtlasSection`
//! bridge into forge-book's atlas — forge-book has no v3 home yet, same
//! scoping forge-midi-v3/forge-mp3-v3 already applied to forge-book's other
//! modules.
//!
//! No wall-clock lives here (C14 firewall): every duration this crate
//! touches is caller-supplied milliseconds, never `Instant::now()`.

#![deny(missing_docs)]

pub mod histogram;
pub mod topo;

/// One phase of the triadic loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Phase {
    /// Backend primitive — logic, invariants, memory layouts.
    Floor = 0,
    /// Engine wiring — the primitive becomes callable without breakage.
    Circuit = 1,
    /// The visible receipt — paint, frame, sound, painted terminal line.
    Surface = 2,
}

impl Phase {
    /// Every phase in cadence order.
    pub const ALL: [Phase; 3] = [Phase::Floor, Phase::Circuit, Phase::Surface];

    /// Discriminant value for wire encoding or tape-tag comparison.
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode a stored phase. `None` outside `0..=2` — an unknown phase is
    /// corruption, never a default.
    #[inline(always)]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Floor),
            1 => Some(Self::Circuit),
            2 => Some(Self::Surface),
            _ => None,
        }
    }

    /// The only lawful successor: `Floor -> Circuit -> Surface -> seal (None)`.
    /// `Surface` has no successor — a session does not walk backward into a
    /// new `Floor`, it seals and a new session starts one.
    pub const fn next(self) -> Option<Phase> {
        match self {
            Phase::Floor => Some(Phase::Circuit),
            Phase::Circuit => Some(Phase::Surface),
            Phase::Surface => None,
        }
    }

    /// This phase's permyriad share of the session ([`BUDGET_PMY`]).
    pub fn budget_pmy(self) -> u16 {
        BUDGET_PMY.iter().find(|(p, _)| *p == self).map_or(0, |(_, b)| *b)
    }
}

/// Energy budget per phase, permyriad of one session (sums to `10_000`).
///
/// `30/30/40` — the return (Surface) carries the heaviest weight, never the
/// foundation. It was `40/30/30` until 08-02: the floor drew the largest
/// share while the only rung that can close a wave drew the smallest, which
/// is a budget that funds groundwork and starves shipping.
pub const BUDGET_PMY: [(Phase, u16); 3] =
    [(Phase::Floor, 3_000), (Phase::Circuit, 3_000), (Phase::Surface, 4_000)];

const _: () = assert!(BUDGET_PMY[0].1 as u32 + BUDGET_PMY[1].1 as u32 + BUDGET_PMY[2].1 as u32 == 10_000);

/// Why a requested phase transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CadenceError {
    /// `Floor -> Surface` was requested directly — `Circuit` was never run.
    SkippedCircuit,
    /// The transition runs backward, or past `Surface`'s seal. A session
    /// that wants a new cadence starts a new one at `Floor`; it never
    /// rewinds or continues an existing one.
    RanBackward,
    /// `from == to`: advancing to the same phase is not a transition.
    NoTransition,
}

/// Validate and perform one cadence transition.
///
/// A transition is lawful only to [`Phase::next`]. Skipping the surface is
/// the deceptive cadence — refused, not discouraged. `Surface` has no
/// lawful successor at all: it seals.
pub fn advance(from: Phase, to: Phase) -> Result<Phase, CadenceError> {
    if from == to {
        return Err(CadenceError::NoTransition);
    }
    match from.next() {
        Some(n) if n == to => Ok(to),
        _ if from == Phase::Floor && to == Phase::Surface => Err(CadenceError::SkippedCircuit),
        _ => Err(CadenceError::RanBackward),
    }
}

/// The Done-invariant. A wave closes only on a SURFACE receipt: a passing
/// test is a prefilter of done, never the close — the session leaves the
/// desk with something visible.
pub const DONE_INVARIANT: &str = "DONE = the surface SHIPPED: visible on both loops (GPU \
and CPU), rendered not described, PROVEN by pixel capture (an uncaptured green render is \
VOID), then HITL-confirmed by Sean. A green test gates the close and never IS it; the check \
mark is what Sean's eyes award, not what the suite awards itself";

/// The four rungs [`DONE_INVARIANT`] names, as flags a caller must actually
/// set. A `&str` cannot refuse anything — this is the same sentence with a
/// gate under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipRungs {
    /// Visible on BOTH loops — GPU and CPU, not one of them.
    pub both_loops: bool,
    /// Actually rendered, not described in a receipt.
    pub rendered: bool,
    /// Pixels captured. Uncaptured green is VOID.
    pub captured: bool,
    /// Sean's eyes signed it. The check mark is his to award.
    pub hitl: bool,
}

/// The DONE gauge. `Ok` only when all four rungs stand; the `Err` names the
/// missing one so a caller never has to guess which rung it skipped.
pub fn ship_check(r: &ShipRungs) -> Result<(), String> {
    let missing: Vec<&str> = [
        (!r.both_loops).then_some("both loops (GPU and CPU)"),
        (!r.rendered).then_some("rendered, not described"),
        (!r.captured).then_some("pixel capture (uncaptured green is VOID)"),
        (!r.hitl).then_some("HITL — Sean's eyes award the check"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!("NOT SHIPPED — {} rung(s) owed: {}", missing.len(), missing.join(" \u{b7} ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budgets_sum_to_one_session_and_the_return_carries_the_weight() {
        let sum: u32 = BUDGET_PMY.iter().map(|(_, b)| *b as u32).sum();
        assert_eq!(sum, 10_000, "the three phases spend exactly one session");
        assert_eq!(Phase::Surface.budget_pmy(), 4_000, "the closing rung is the funded one");
        assert_eq!(Phase::Floor.budget_pmy(), 3_000);
        assert_eq!(Phase::Circuit.budget_pmy(), 3_000);
        assert!(
            Phase::Surface.budget_pmy() > Phase::Floor.budget_pmy(),
            "a budget that funds groundwork over shipping is how a wave ends with no surface"
        );
    }

    #[test]
    fn the_only_walk_is_floor_circuit_surface() {
        assert_eq!(advance(Phase::Floor, Phase::Circuit), Ok(Phase::Circuit));
        assert_eq!(advance(Phase::Circuit, Phase::Surface), Ok(Phase::Surface));
        assert_eq!(Phase::Surface.next(), None, "after the surface, the seal");
        assert_eq!(advance(Phase::Floor, Phase::Surface), Err(CadenceError::SkippedCircuit));
        assert_eq!(advance(Phase::Surface, Phase::Floor), Err(CadenceError::RanBackward), "no walking backwards");
    }

    /// L07: bijection over the full 3x3 pair space, not just the happy edges.
    #[test]
    fn every_pair_is_covered_exactly_once() {
        for from in Phase::ALL {
            for to in Phase::ALL {
                let r = advance(from, to);
                match (from, to) {
                    (Phase::Floor, Phase::Circuit) | (Phase::Circuit, Phase::Surface) => {
                        assert_eq!(r, Ok(to))
                    }
                    (a, b) if a == b => assert_eq!(r, Err(CadenceError::NoTransition)),
                    (Phase::Floor, Phase::Surface) => {
                        assert_eq!(r, Err(CadenceError::SkippedCircuit))
                    }
                    _ => assert_eq!(r, Err(CadenceError::RanBackward)),
                }
            }
        }
    }

    #[test]
    fn as_u8_and_from_u8_round_trip() {
        for p in Phase::ALL {
            assert_eq!(Phase::from_u8(p.as_u8()), Some(p));
        }
        assert_eq!(Phase::from_u8(3), None);
    }

    #[test]
    fn done_means_the_surface_shipped_not_that_it_answered() {
        assert!(DONE_INVARIANT.contains("surface"), "the invariant names its organ");
    }

    #[test]
    fn the_four_rungs_are_gauged_and_a_missing_one_refuses() {
        assert!(ship_check(&ShipRungs { both_loops: true, rendered: true, captured: true, hitl: true }).is_ok());
        for (r, want) in [
            (ShipRungs { both_loops: false, rendered: true, captured: true, hitl: true }, "both loops"),
            (ShipRungs { both_loops: true, rendered: false, captured: true, hitl: true }, "rendered"),
            (ShipRungs { both_loops: true, rendered: true, captured: false, hitl: true }, "capture"),
            (ShipRungs { both_loops: true, rendered: true, captured: true, hitl: false }, "HITL"),
        ] {
            let err = ship_check(&r).expect_err("a missing rung must refuse");
            assert!(err.contains(want), "refusal names the missing rung: {err}");
        }
        let suite_only = ShipRungs { both_loops: false, rendered: false, captured: false, hitl: true };
        assert!(ship_check(&suite_only).is_err(), "green tests do not award the check");
    }
}
