//! futuresight.rs — the always-on Ghost driver. Runs the [`GhostPlayhead`] continuously
//! ahead of a live clock: each real tick, re-project the tape, sweep the radar, and DIFF
//! contacts across ticks so a NEW fault RAISES, a fixed one CLEARS, and one the real clock
//! reaches IMPACTS. Rolls a [`Verdict`] over the whole projected future and checks a
//! [`LawBook`] of registered locked-laws. Decoupled from `world_clock`, fed its ticks.
//! Reads only — the future is projected, never written.

use crate::ghost::{CollisionKind, CollisionRadar, Contact, GhostPlayhead, HologramMap, Projection};
use crate::timeline::{SealedTuple, TimelineTape};

/// Severity rollup of a projection — worst contact wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// No radar contacts — the plan is collision-free to the horizon.
    Clear,
    /// Only soft contacts (e.g. same-tick disagreements) — proceed with care.
    Warning,
    /// A structural fault or locked-law breach sits in the future — do not commit blind.
    Blocked,
}

impl Verdict {
    /// The worse of two verdicts (`Blocked > Warning > Clear`).
    pub fn worst(self, other: Verdict) -> Verdict {
        use Verdict::*;
        match (self, other) {
            (Blocked, _) | (_, Blocked) => Blocked,
            (Warning, _) | (_, Warning) => Warning,
            _ => Clear,
        }
    }

    /// The verdict a single collision kind carries.
    pub fn of_kind(kind: CollisionKind) -> Verdict {
        match kind {
            // structural faults corrupt state if committed — hard block.
            CollisionKind::NonMonotonic
            | CollisionKind::InvalidEssence
            | CollisionKind::InvalidMoon
            | CollisionKind::LawViolation => Verdict::Blocked,
            // two things at one instant: recoverable, but flagged.
            CollisionKind::SameTickConflict => Verdict::Warning,
        }
    }

    /// Roll a verdict over every contact in a sweep.
    pub fn of_contacts(contacts: &[Contact]) -> Verdict {
        contacts
            .iter()
            .fold(Verdict::Clear, |acc, c| acc.worst(Verdict::of_kind(c.kind)))
    }

    #[inline]
    /// True when the verdict carries no warning or block.
    pub fn is_clear(self) -> bool {
        self == Verdict::Clear
    }

    #[inline]
    /// True when the verdict is a hard block.
    pub fn is_blocked(self) -> bool {
        self == Verdict::Blocked
    }
}

/// A contact's lifecycle as the real clock advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    /// A fault appeared in the projection this tick (was not there last tick).
    Raised(Contact),
    /// A fault the projection carried last tick is gone (a fix landed, or it impacted).
    Cleared(Contact),
    /// The real clock reached a fault's tick — futuresight ran out; it is NOW.
    Impact(Contact),
}

impl Alert {
    /// The contact this alert concerns.
    pub fn contact(&self) -> Contact {
        match self {
            Alert::Raised(c) | Alert::Cleared(c) | Alert::Impact(c) => *c,
        }
    }
}

/// One registered locked-law: a named predicate every pending moment must satisfy.
#[derive(Clone, Copy)]
pub struct LockedLaw {
    /// Stable law id (e.g. `"no-broke-essence-in-vital-moon"`).
    pub id: &'static str,
    /// The verdict a breach carries.
    pub severity: Verdict,
    /// Returns `true` when the moment OBEYS the law.
    pub predicate: fn(&SealedTuple) -> bool,
}

/// A breach of a registered locked-law, sited in the future with its lead time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LawContact {
    /// Which registered law was breached.
    pub law_id: &'static str,
    /// The pending tick where the breach sits.
    pub tick: u64,
    /// Ticks until the real clock arrives.
    pub lead_ticks: u64,
    /// The verdict this breach carries.
    pub severity: Verdict,
    /// Index of the offending moment within the projected future slice.
    pub index: usize,
}

/// The book of locked-laws the radar enforces against a plan.
#[derive(Clone, Default)]
pub struct LawBook {
    laws: Vec<LockedLaw>,
}

impl LawBook {
    /// An empty book.
    pub fn new() -> Self {
        Self { laws: Vec::new() }
    }

    /// Register a law (builder style).
    pub fn with(mut self, law: LockedLaw) -> Self {
        self.laws.push(law);
        self
    }

    /// Register a law in place.
    pub fn register(&mut self, law: LockedLaw) {
        self.laws.push(law);
    }

    /// How many laws are registered.
    #[inline]
    pub fn len(&self) -> usize {
        self.laws.len()
    }

    #[inline]
    /// True when no laws are registered.
    pub fn is_empty(&self) -> bool {
        self.laws.is_empty()
    }

    /// Check every pending moment against every law; return breaches earliest-lead first.
    pub fn check(&self, proj: &Projection) -> Vec<LawContact> {
        let now = proj.now();
        let mut out = Vec::new();
        for (i, e) in proj.frames().iter().enumerate() {
            for law in &self.laws {
                if !(law.predicate)(e) {
                    out.push(LawContact {
                        law_id: law.id,
                        tick: e.tick_id,
                        lead_ticks: e.tick_id.saturating_sub(now),
                        severity: law.severity,
                        index: i,
                    });
                }
            }
        }
        out.sort_by_key(|c| (c.lead_ticks, c.index));
        out
    }

    /// The rolled verdict of all law breaches in the projection.
    pub fn verdict(&self, proj: &Projection) -> Verdict {
        self.check(proj)
            .iter()
            .fold(Verdict::Clear, |acc, c| acc.worst(c.severity))
    }
}

/// The always-on futuresight engine. Hold one per live clock; `tick` it as the real clock
/// advances and read `verdict` / `alerts` / `map` between ticks.
pub struct Futuresight {
    ghost: GhostPlayhead,
    last_contacts: Vec<Contact>,
    verdict: Verdict,
    horizon: Option<u64>,
    laws: LawBook,
}

impl Futuresight {
    /// A fresh engine anchored at the real clock's current tick.
    pub fn new(now_tick: u64) -> Self {
        Self {
            ghost: GhostPlayhead::at(now_tick),
            last_contacts: Vec::new(),
            verdict: Verdict::Clear,
            horizon: None,
            laws: LawBook::new(),
        }
    }

    /// A fresh engine with a locked-law book.
    pub fn with_laws(now_tick: u64, laws: LawBook) -> Self {
        Self {
            ghost: GhostPlayhead::at(now_tick),
            last_contacts: Vec::new(),
            verdict: Verdict::Clear,
            horizon: None,
            laws,
        }
    }

    /// The real-clock tick the engine is anchored to.
    #[inline]
    pub fn now(&self) -> u64 {
        self.ghost.now()
    }

    /// The current rolled verdict over the projected future.
    #[inline]
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The furthest pending tick seen at the last sweep.
    #[inline]
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    /// The contacts standing after the last sweep.
    #[inline]
    pub fn contacts(&self) -> &[Contact] {
        &self.last_contacts
    }

    /// The current always-on holographic map (re-projected on demand).
    pub fn map(&self, tape: &TimelineTape) -> HologramMap {
        HologramMap::project(tape, self.ghost.now())
    }

    /// Advance to the real clock's new tick and re-sweep. Returns the alert stream:
    /// faults that RAISED, CLEARED, or IMPACTED (the real clock reached them) since the
    /// last tick. Also refreshes `verdict`, `horizon`, and `contacts`.
    pub fn tick(&mut self, tape: &TimelineTape, real_tick: u64) -> Vec<Alert> {
        let prev_now = self.ghost.now();
        self.ghost.advance_now(real_tick);
        let proj = self.ghost.project(tape);

        // built-in radar + registered laws → one contact set (laws mapped to LawViolation).
        let mut now_contacts = CollisionRadar::scan(&proj);
        for lc in self.laws.check(&proj) {
            now_contacts.push(Contact {
                tick: lc.tick,
                lead_ticks: lc.lead_ticks,
                kind: CollisionKind::LawViolation,
                index: lc.index,
            });
        }
        now_contacts.sort_by_key(|c| (c.lead_ticks, c.tick, c.index));

        let mut alerts = Vec::new();

        // IMPACT: any prior contact whose tick fell into (prev_now, real_tick].
        for c in &self.last_contacts {
            if c.tick > prev_now && c.tick <= real_tick {
                alerts.push(Alert::Impact(*c));
            }
        }
        // RAISED: in now, not in last (by tick+kind).
        for c in &now_contacts {
            if !self.last_contacts.iter().any(|p| p.tick == c.tick && p.kind == c.kind) {
                alerts.push(Alert::Raised(*c));
            }
        }
        // CLEARED: in last, not in now, and not an impact (still ahead of the clock).
        for c in &self.last_contacts {
            let gone = !now_contacts.iter().any(|n| n.tick == c.tick && n.kind == c.kind);
            let impacted = c.tick > prev_now && c.tick <= real_tick;
            if gone && !impacted {
                alerts.push(Alert::Cleared(*c));
            }
        }

        self.verdict = Verdict::of_contacts(&now_contacts);
        self.horizon = proj.horizon();
        self.last_contacts = now_contacts;
        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Stamped, Ump};
    use crate::provenance_tag::Tier;

    fn ev(t: u32) -> Vec<Stamped<Ump>> {
        vec![Stamped { universal_tick_us: t as i64, payload: Ump::new([t, 0, 0, 0]) }]
    }

    fn tape_of(n: u64) -> TimelineTape {
        let mut t = TimelineTape::new(10);
        for i in 0..n {
            t.record(i * 100, ((i % 13) + 1) as u8, (i % 64) as u8, Tier::Local, &ev(i as u32 + 1))
                .unwrap();
        }
        t
    }

    // ── verdict ──

    #[test]
    fn verdict_worst_orders_severities() {
        use Verdict::*;
        assert_eq!(Clear.worst(Warning), Warning);
        assert_eq!(Warning.worst(Blocked), Blocked);
        assert_eq!(Clear.worst(Clear), Clear);
        assert_eq!(Blocked.worst(Clear), Blocked);
    }

    #[test]
    fn verdict_of_kind_maps_severity() {
        assert_eq!(Verdict::of_kind(CollisionKind::InvalidEssence), Verdict::Blocked);
        assert_eq!(Verdict::of_kind(CollisionKind::SameTickConflict), Verdict::Warning);
        assert_eq!(Verdict::of_kind(CollisionKind::LawViolation), Verdict::Blocked);
    }

    #[test]
    fn clean_plan_is_clear() {
        let tape = tape_of(20);
        let mut fs = Futuresight::new(0);
        let alerts = fs.tick(&tape, 0);
        assert!(alerts.is_empty());
        assert_eq!(fs.verdict(), Verdict::Clear);
        assert_eq!(fs.horizon(), Some(1900));
    }

    // ── law book ──

    fn no_essence_above_3(e: &SealedTuple) -> bool {
        e.essence_id <= 3
    }

    #[test]
    fn lawbook_checks_and_verdicts() {
        let book = LawBook::new().with(LockedLaw {
            id: "essence-cap-3",
            severity: Verdict::Blocked,
            predicate: no_essence_above_3,
        });
        let tape = tape_of(10); // essences 0..9
        let proj = GhostPlayhead::at(0).project(&tape);
        let breaches = book.check(&proj);
        assert!(!breaches.is_empty());
        assert_eq!(breaches[0].law_id, "essence-cap-3");
        // earliest breach is essence 4 at tick 400.
        assert_eq!(breaches.iter().min_by_key(|c| c.lead_ticks).unwrap().tick, 400);
        assert_eq!(book.verdict(&proj), Verdict::Blocked);
    }

    #[test]
    fn empty_lawbook_is_clear() {
        let book = LawBook::new();
        let tape = tape_of(5);
        let proj = GhostPlayhead::at(0).project(&tape);
        assert!(book.check(&proj).is_empty());
        assert_eq!(book.verdict(&proj), Verdict::Clear);
    }

    // ── futuresight driver ──

    #[test]
    fn future_fault_raises_then_impacts() {
        // A bad moon sits at tick 500; the real clock walks up to it.
        let mut t = TimelineTape::new(10);
        for i in 0..5u64 {
            t.record(i * 100, 1, 1, Tier::Local, &ev(i as u32 + 1)).unwrap();
        }
        t.record(500, 99, 1, Tier::Local, &ev(99)).unwrap(); // invalid moon in the future

        let mut fs = Futuresight::new(0);
        let a0 = fs.tick(&t, 0);
        assert!(a0.iter().any(|a| matches!(a, Alert::Raised(c) if c.kind == CollisionKind::InvalidMoon)));
        assert_eq!(fs.verdict(), Verdict::Blocked);

        // clock advances to 300 — still ahead, no impact yet, no new alert.
        let a1 = fs.tick(&t, 300);
        assert!(a1.is_empty(), "fault already known, still ahead: {a1:?}");
        assert_eq!(fs.verdict(), Verdict::Blocked);

        // clock reaches 500 — the fault IMPACTS.
        let a2 = fs.tick(&t, 500);
        assert!(a2.iter().any(|a| matches!(a, Alert::Impact(c) if c.tick == 500)));
    }

    #[test]
    fn fixing_the_plan_clears_the_alert() {
        // A fault at 400; then a corrected tape (fault removed) → CLEARED.
        let mut bad = TimelineTape::new(10);
        bad.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        bad.record(400, 200, 1, Tier::Local, &ev(2)).unwrap(); // invalid moon

        let mut good = TimelineTape::new(10);
        good.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        good.record(400, 7, 1, Tier::Local, &ev(2)).unwrap(); // fixed

        let mut fs = Futuresight::new(0);
        let a0 = fs.tick(&bad, 0);
        assert!(a0.iter().any(|a| matches!(a, Alert::Raised(_))));
        assert_eq!(fs.verdict(), Verdict::Blocked);

        // same clock, corrected plan → the fault clears.
        let a1 = fs.tick(&good, 0);
        assert!(a1.iter().any(|a| matches!(a, Alert::Cleared(c) if c.tick == 400)));
        assert_eq!(fs.verdict(), Verdict::Clear);
    }

    #[test]
    fn futuresight_with_laws_blocks_on_breach() {
        let book = LawBook::new().with(LockedLaw {
            id: "essence-cap-3",
            severity: Verdict::Blocked,
            predicate: no_essence_above_3,
        });
        let tape = tape_of(10); // essence climbs past 3
        let mut fs = Futuresight::with_laws(0, book);
        fs.tick(&tape, 0);
        assert_eq!(fs.verdict(), Verdict::Blocked, "a locked-law breach blocks the plan");
        assert!(fs.contacts().iter().any(|c| c.kind == CollisionKind::LawViolation));
    }

    #[test]
    fn map_reflects_the_current_now() {
        let tape = tape_of(10);
        let mut fs = Futuresight::new(0);
        fs.tick(&tape, 550);
        let map = fs.map(&tape);
        assert_eq!(map.now(), 550);
        assert_eq!(map.essence_track().len(), 4); // 600,700,800,900
    }

    #[test]
    fn horizon_recedes_as_the_clock_advances() {
        let tape = tape_of(10); // horizon 900
        let mut fs = Futuresight::new(0);
        fs.tick(&tape, 0);
        assert_eq!(fs.horizon(), Some(900));
        fs.tick(&tape, 850);
        assert_eq!(fs.horizon(), Some(900));
        fs.tick(&tape, 950); // past the end
        assert_eq!(fs.horizon(), None, "nothing left ahead");
    }

    #[test]
    fn alert_contact_accessor() {
        let c = Contact { tick: 1, lead_ticks: 1, kind: CollisionKind::InvalidMoon, index: 0 };
        assert_eq!(Alert::Raised(c).contact(), c);
        assert_eq!(Alert::Impact(c).contact(), c);
    }
}
