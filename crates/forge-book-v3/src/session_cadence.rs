//! session_cadence — the triadic session loop as a compiled law (Sean 08-02):
//! Floor → Circuit → Surface, permyriad budgets, seal at close — and the
//! Done-invariant: DONE is the surface answering, never a green log alone.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// One phase of the triadic loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Backend primitive — logic, invariants, memory layouts.
    Floor,
    /// Engine wiring — the primitive becomes callable without breakage.
    Circuit,
    /// The visible receipt — paint, frame, sound, painted terminal line.
    Surface,
}

/// Energy budget per phase, permyriad of one session (sums to 10_000).
///
/// 30/30/40 — the SAME split `oracle1_governor::BOARD_ROW_WEIGHTS` puts on a board row
/// (loc 30 / difficulty 30 / roi 40), and for the same reason: the RETURN carries the
/// heaviest weight, never the foundation. Surface IS the return here — [`DONE_INVARIANT`]
/// closes on it and nothing else.
///
/// It was 40/30/30 until 08-02 (Sean: "it does the 30 30 40 split"): the floor drew the
/// largest share while the only rung that can close a wave drew the smallest, which is a
/// budget that funds groundwork and starves shipping.
pub const BUDGET_PMY: [(Phase, u16); 3] =
    [(Phase::Floor, 3_000), (Phase::Circuit, 3_000), (Phase::Surface, 4_000)];

/// The Done-invariant. A wave closes only on a SURFACE receipt: a passing test is a
/// prefilter of done, never the close — the session leaves the desk with something visible.
///
/// RAISED 08-02 (Sean, "when its visible rendered proven HITL with full gpu cpu rendering
/// its shipped gets a check"). The first cut of this constant, written the same morning,
/// closed on "one visible receipt (painted line, frame, sound)" — which a single drawn line
/// on one loop satisfies. That is a surface ANSWERING, not a surface SHIPPING. The bar now
/// names all four rungs, because three of them were being read as optional:
/// both loops actually render, a capture proves the pixels (root#lock-gate: a GREEN render
/// claimed without forgewright/forgevision capture is VOID), and SEAN's eyes sign it. The
/// check mark is the fourth rung, never the first.
pub const DONE_INVARIANT: &str = "DONE = the surface SHIPPED: visible on both loops (GPU \
and CPU), rendered not described, PROVEN by pixel capture (forgewright/forgevision — an \
uncaptured green render is VOID, root#lock-gate), then HITL-confirmed by Sean. A green \
test gates the close and never IS it; the check mark is what Sean's eyes award, not what \
the suite awards itself";

/// The four rungs [`DONE_INVARIANT`] names, as flags a caller must actually set.
///
/// A `&str` cannot refuse anything — the old invariant was true, unenforced, and read as
/// satisfied by whichever rung the session happened to reach. This is the same sentence
/// with a gate under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipRungs {
    /// Visible on BOTH loops — GPU and CPU, not one of them.
    pub both_loops: bool,
    /// Actually rendered, not described in a receipt.
    pub rendered: bool,
    /// Pixels captured (forgewright/forgevision). Uncaptured green = VOID, root#lock-gate.
    pub captured: bool,
    /// Sean's eyes signed it. The check mark is his to award.
    pub hitl: bool,
}

/// The DONE gauge. `Ok` only when all four rungs stand; the `Err` NAMES the missing one so
/// a caller never has to guess which rung it skipped.
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
    Err(format!("NOT SHIPPED — {} rung(s) owed: {}", missing.len(), missing.join(" · ")))
}

impl Phase {
    /// The only lawful successor: Floor → Circuit → Surface → seal (None).
    pub const fn next(self) -> Option<Phase> {
        match self {
            Phase::Floor => Some(Phase::Circuit),
            Phase::Circuit => Some(Phase::Surface),
            Phase::Surface => None,
        }
    }

    /// A transition is lawful only to `next()`. Skipping the surface is the
    /// deceptive cadence — refused, not discouraged.
    pub fn advance(self, to: Phase) -> Result<Phase, String> {
        match self.next() {
            Some(n) if n == to => Ok(to),
            lawful => Err(format!("phase skip refused: {self:?} -> {to:?} (lawful: {lawful:?})")),
        }
    }

    /// This phase's permyriad share of the session.
    pub fn budget_pmy(self) -> u16 {
        BUDGET_PMY.iter().find(|(p, _)| *p == self).map_or(0, |(_, b)| *b)
    }
}

/// Bind the cadence into the atlas — the law's live reader.
pub fn cadence_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Cadence — the Triadic Loop", AtlasSection::Capabilities);
    ch.add_lore(DONE_INVARIANT);
    for (p, b) in BUDGET_PMY {
        ch.add_lore(format!("{p:?} {b} pmy — next: {:?}", p.next()));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: SESSION-CADENCE]
    #[test]
    fn budgets_sum_to_one_session_and_the_return_carries_the_weight() {
        let sum: u32 = BUDGET_PMY.iter().map(|(_, b)| *b as u32).sum();
        assert_eq!(sum, 10_000, "the three phases spend exactly one session");
        // 30/30/40, the same shape BOARD_ROW_WEIGHTS puts on a row: the RETURN is heaviest.
        assert_eq!(Phase::Surface.budget_pmy(), 4_000, "the closing rung is the funded one");
        assert_eq!(Phase::Floor.budget_pmy(), 3_000);
        assert_eq!(Phase::Circuit.budget_pmy(), 3_000);
        assert!(
            Phase::Surface.budget_pmy() > Phase::Floor.budget_pmy(),
            "a budget that funds groundwork over shipping is how a wave ends with no surface"
        );
    }

    // [BOARD: SESSION-CADENCE]
    #[test]
    fn the_only_walk_is_floor_circuit_surface() {
        let c = Phase::Floor.advance(Phase::Circuit).expect("floor feeds circuit");
        let s = c.advance(Phase::Surface).expect("circuit feeds surface");
        assert_eq!(s.next(), None, "after the surface, the seal");
        assert!(Phase::Floor.advance(Phase::Surface).is_err(), "the skip is refused");
        assert!(Phase::Surface.advance(Phase::Floor).is_err(), "no walking backwards");
    }

    // [BOARD: SESSION-CADENCE]
    #[test]
    fn done_means_the_surface_shipped_not_that_it_answered() {
        assert!(DONE_INVARIANT.contains("surface"), "the invariant names its organ");
        let ch = cadence_chapter();
        assert_eq!(ch.lore_count(), 4, "invariant + three phase rows");
    }

    // [BOARD: SESSION-CADENCE] The gauge is the law (Sean 08-02 "DONE_INVARIANT needs a
    // verb"): the bar was raised in prose the same morning the old one was written in prose,
    // and prose is what let "a painted line" pass as shipped for a whole session.
    #[test]
    fn the_four_rungs_are_gauged_and_a_missing_one_refuses() {
        // All four land -> the check is earned.
        assert!(ship_check(&ShipRungs { both_loops: true, rendered: true, captured: true, hitl: true }).is_ok());
        // Any single miss refuses, and the refusal NAMES the rung so nobody guesses.
        for (r, want) in [
            (ShipRungs { both_loops: false, rendered: true, captured: true, hitl: true }, "both loops"),
            (ShipRungs { both_loops: true, rendered: false, captured: true, hitl: true }, "rendered"),
            (ShipRungs { both_loops: true, rendered: true, captured: false, hitl: true }, "capture"),
            (ShipRungs { both_loops: true, rendered: true, captured: true, hitl: false }, "HITL"),
        ] {
            let err = ship_check(&r).expect_err("a missing rung must refuse");
            assert!(err.contains(want), "refusal names the missing rung: {err}");
        }
        // A green suite alone is the prefilter, never the close — three rungs still owed.
        let suite_only = ShipRungs { both_loops: false, rendered: false, captured: false, hitl: true };
        assert!(ship_check(&suite_only).is_err(), "green tests do not award the check");
    }
}
