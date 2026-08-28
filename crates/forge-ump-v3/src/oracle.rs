//! oracle.rs — the futuresight ORACLE. Answers planning questions over the tape WITHOUT
//! committing anything: would this moment be admitted? what's the soonest hazard ahead?
//! which candidate tick is safe? how does plan A's future differ from plan B's? Every
//! answer is a read-only hypothetical over the [`GhostPlayhead`] projection — the ghost
//! hits the error in the map before the real clock (or a real commit) ever does.

use crate::futuresight::{LawBook, Verdict};
use crate::ghost::{CollisionRadar, Contact, GhostPlayhead, Projection};
use crate::timeline::TimelineTape;

/// A would-be commit — the thing a planner is about to append to the tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingMoment {
    /// The tick the moment would land at.
    pub tick: u64,
    /// The moon lane the moment would be recorded under.
    pub moon: u8,
    /// The essence codeword the moment would carry.
    pub essence_id: u8,
}

impl PendingMoment {
    /// Build a would-be commit.
    pub fn new(tick: u64, moon: u8, essence_id: u8) -> Self {
        Self { tick, moon, essence_id }
    }
}

/// Why a would-be commit is refused (or `Admitted`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The append is clean — safe to commit.
    Admitted,
    /// Would tick before the tape's end (append-only violation).
    WouldRewindHistory {
        /// The tape's current last tick.
        last_tick: u64,
    },
    /// Codeword outside the 64-slot essence codebook.
    InvalidEssence,
    /// Moon outside `0..=13`.
    InvalidMoon,
    /// Shares the end tick but disagrees on essence with the last commit.
    SameTickConflict,
    /// A registered locked-law refused it.
    LawViolation {
        /// The id of the law that refused the moment.
        law_id: &'static str,
    },
}

impl Admission {
    #[inline]
    /// True only for the clean-append case.
    pub fn is_admitted(self) -> bool {
        self == Admission::Admitted
    }

    /// The verdict this admission rolls up to.
    pub fn verdict(self) -> Verdict {
        match self {
            Admission::Admitted => Verdict::Clear,
            Admission::SameTickConflict => Verdict::Warning,
            _ => Verdict::Blocked,
        }
    }
}

/// The delta between two projected futures — what a plan change did to the horizon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDiff {
    /// Contacts present in `after` but not `before` (new faults introduced).
    pub raised: Vec<Contact>,
    /// Contacts present in `before` but not `after` (faults resolved).
    pub cleared: Vec<Contact>,
    /// The `before` projection's horizon.
    pub horizon_before: Option<u64>,
    /// The `after` projection's horizon.
    pub horizon_after: Option<u64>,
}

impl ProjectionDiff {
    /// Diff two projections (same `now`, different tape/plan).
    pub fn between(before: &Projection, after: &Projection) -> Self {
        let cb = CollisionRadar::scan(before);
        let ca = CollisionRadar::scan(after);
        let same = |a: &Contact, b: &Contact| a.tick == b.tick && a.kind == b.kind;
        let raised = ca.iter().filter(|c| !cb.iter().any(|p| same(c, p))).copied().collect();
        let cleared = cb.iter().filter(|c| !ca.iter().any(|n| same(c, n))).copied().collect();
        Self {
            raised,
            cleared,
            horizon_before: before.horizon(),
            horizon_after: after.horizon(),
        }
    }

    /// The plan change removed faults without adding any — a strict improvement.
    pub fn is_improvement(&self) -> bool {
        !self.cleared.is_empty() && self.raised.is_empty()
    }

    /// The plan change introduced new faults — a regression.
    pub fn regressed(&self) -> bool {
        !self.raised.is_empty()
    }

    /// Nothing changed in the fault set.
    pub fn is_neutral(&self) -> bool {
        self.raised.is_empty() && self.cleared.is_empty()
    }
}

/// A candidate plan's futuresight verdict — for ranking branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchVerdict {
    /// Index into the candidate branch slice.
    pub branch: usize,
    /// The rolled verdict over that branch's projected future.
    pub verdict: Verdict,
    /// The soonest hazard on that branch, if any.
    pub first_hazard: Option<Contact>,
}

/// Sort key for a breach's severity — lower is worse, so `min_by_key` picks the
/// one that actually blocks. Clear ranks last: a law whose breach carries no
/// severity never outranks one that does.
fn severity_rank(severity: Verdict) -> u8 {
    match severity {
        Verdict::Blocked => 0,
        Verdict::Warning => 1,
        Verdict::Clear => 2,
    }
}

/// The read-only planning oracle over a tape's future.
pub struct FutureOracle;

impl FutureOracle {
    /// Project the tape's future from `now`.
    fn project<'t>(tape: &'t TimelineTape, now: u64) -> Projection<'t> {
        GhostPlayhead::at(now).project(tape)
    }

    /// Would appending `pending` to the tape be admitted? Append-only semantics: it must
    /// tick at-or-after the last commit, carry a legal codeword/moon, and not conflict.
    pub fn admits(tape: &TimelineTape, pending: PendingMoment) -> Admission {
        if pending.essence_id > 63 {
            return Admission::InvalidEssence;
        }
        if pending.moon > 13 {
            return Admission::InvalidMoon;
        }
        if let Some(last) = tape.last() {
            if pending.tick < last.tick_id {
                return Admission::WouldRewindHistory { last_tick: last.tick_id };
            }
            if pending.tick == last.tick_id && pending.essence_id != last.essence_id {
                return Admission::SameTickConflict;
            }
        }
        Admission::Admitted
    }

    /// Admittance including a locked-law book.
    pub fn admits_with(tape: &TimelineTape, pending: PendingMoment, laws: &LawBook) -> Admission {
        let base = Self::admits(tape, pending);
        if !base.is_admitted() {
            return base;
        }
        // Build the would-be sealed moment and test the laws against it.
        let probe = crate::timeline::SealedTuple {
            tick_id: pending.tick,
            content_seal: 0,
            chain_seal: 0,
            moon: pending.moon,
            essence_id: pending.essence_id,
            source_kind: 0,
            flags: 0,
            reserved: 0,
        };
        // A one-frame projection anchored just before the pending tick.
        let now = pending.tick.saturating_sub(1);
        let frames = [probe];
        let proj = Self::synthetic_projection(now, &frames);
        // WORST severity wins, then earliest lead, then frame index. The old
        // `for … { return }` read only the first contact in LEAD order and
        // dropped `LockedLaw::severity` on the floor, so a hard-Blocked law was
        // masked whenever a soft Warning law breached ahead of it — the caller
        // fixed the warning and got ambushed by the block. Deterministic on ties.
        match laws
            .check(&proj)
            .into_iter()
            .min_by_key(|c| (severity_rank(c.severity), c.lead_ticks, c.index))
        {
            Some(lc) => Admission::LawViolation { law_id: lc.law_id },
            None => Admission::Admitted,
        }
    }

    /// The soonest fault in the tape's pending future (built-in radar), measured from `now`.
    pub fn next_hazard(tape: &TimelineTape, now: u64) -> Option<Contact> {
        CollisionRadar::first_contact(&Self::project(tape, now))
    }

    /// The tick up to which the plan is collision-free — the horizon if clear, else the
    /// tick of the first hazard.
    pub fn clear_until(tape: &TimelineTape, now: u64) -> Option<u64> {
        let proj = Self::project(tape, now);
        match CollisionRadar::first_contact(&proj) {
            Some(c) => Some(c.tick),
            None => proj.horizon(),
        }
    }

    /// The current rolled verdict over the pending future.
    pub fn verdict(tape: &TimelineTape, now: u64) -> Verdict {
        Verdict::of_contacts(&CollisionRadar::scan(&Self::project(tape, now)))
    }

    /// Of `candidates` (in order), the first tick at which appending `(moon, essence)` is
    /// cleanly admitted — the safest slot to schedule the moment.
    pub fn safest_tick(
        tape: &TimelineTape,
        moon: u8,
        essence_id: u8,
        candidates: &[u64],
    ) -> Option<u64> {
        candidates.iter().copied().find(|&tick| {
            Self::admits(tape, PendingMoment { tick, moon, essence_id }).is_admitted()
        })
    }

    /// Rank candidate plans (tapes) by futuresight verdict from `now`; Clear branches first,
    /// then by soonest hazard lead.
    pub fn rank_branches(tapes: &[&TimelineTape], now: u64) -> Vec<BranchVerdict> {
        let mut out: Vec<BranchVerdict> = tapes
            .iter()
            .enumerate()
            .map(|(branch, tape)| {
                let proj = Self::project(tape, now);
                BranchVerdict {
                    branch,
                    verdict: Verdict::of_contacts(&CollisionRadar::scan(&proj)),
                    first_hazard: CollisionRadar::first_contact(&proj),
                }
            })
            .collect();
        out.sort_by_key(|b| {
            let sev = match b.verdict {
                Verdict::Clear => 0,
                Verdict::Warning => 1,
                Verdict::Blocked => 2,
            };
            let lead = b.first_hazard.map(|c| c.lead_ticks).unwrap_or(u64::MAX);
            (sev, std::cmp::Reverse(lead), b.branch)
        });
        out
    }

    /// Build a projection over a raw frame slice (test/probe helper).
    fn synthetic_projection<'a>(now: u64, frames: &'a [crate::timeline::SealedTuple]) -> Projection<'a> {
        // Reconstruct a Projection by scanning the frames as the future — the radar and
        // law checks only read `now()` + `frames()`, both satisfied here.
        Projection::over_frames(frames, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futuresight::{LockedLaw, Verdict};
    use crate::ghost::CollisionKind;
    use crate::packet::{Stamped, Ump};
    use crate::provenance_tag::Tier;
    use crate::timeline::SealedTuple;

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

    // ── admits ──

    #[test]
    fn admits_a_clean_append() {
        let t = tape_of(5); // last tick 400
        assert_eq!(FutureOracle::admits(&t, PendingMoment::new(500, 7, 12)), Admission::Admitted);
    }

    #[test]
    fn refuses_history_rewind() {
        let t = tape_of(5); // last tick 400
        assert_eq!(
            FutureOracle::admits(&t, PendingMoment::new(300, 1, 1)),
            Admission::WouldRewindHistory { last_tick: 400 }
        );
    }

    #[test]
    fn refuses_invalid_essence_and_moon() {
        let t = tape_of(3);
        assert_eq!(FutureOracle::admits(&t, PendingMoment::new(500, 1, 200)), Admission::InvalidEssence);
        assert_eq!(FutureOracle::admits(&t, PendingMoment::new(500, 99, 5)), Admission::InvalidMoon);
    }

    #[test]
    fn same_tick_conflict_is_a_warning() {
        let mut t = TimelineTape::new(10);
        t.record(400, 1, 5, Tier::Local, &ev(1)).unwrap();
        let a = FutureOracle::admits(&t, PendingMoment::new(400, 1, 9)); // same tick, diff essence
        assert_eq!(a, Admission::SameTickConflict);
        assert_eq!(a.verdict(), Verdict::Warning);
    }

    #[test]
    fn same_tick_same_essence_is_admitted() {
        let mut t = TimelineTape::new(10);
        t.record(400, 1, 5, Tier::Local, &ev(1)).unwrap();
        assert!(FutureOracle::admits(&t, PendingMoment::new(400, 1, 5)).is_admitted());
    }

    #[test]
    fn admits_with_locked_law() {
        let book = LawBook::new().with(LockedLaw {
            id: "no-essence-63",
            severity: Verdict::Blocked,
            predicate: |e| e.essence_id != 63,
        });
        let t = tape_of(3);
        assert_eq!(
            FutureOracle::admits_with(&t, PendingMoment::new(500, 1, 63), &book),
            Admission::LawViolation { law_id: "no-essence-63" }
        );
        assert!(FutureOracle::admits_with(&t, PendingMoment::new(500, 1, 10), &book).is_admitted());
    }

    /// A hard law must never hide behind a soft one that breached first. The
    /// warning law is registered FIRST, so lead-order alone would report it.
    #[test]
    fn the_blocking_law_is_reported_not_the_first_one_registered() {
        let book = LawBook::new()
            .with(LockedLaw {
                id: "soft-prefer-even-essence",
                severity: Verdict::Warning,
                predicate: |e| e.essence_id % 2 == 0,
            })
            .with(LockedLaw {
                id: "hard-no-essence-63",
                severity: Verdict::Blocked,
                predicate: |e| e.essence_id != 63,
            });
        let t = tape_of(3);
        // 63 breaches BOTH laws — the blocking one is the verdict.
        assert_eq!(
            FutureOracle::admits_with(&t, PendingMoment::new(500, 1, 63), &book),
            Admission::LawViolation { law_id: "hard-no-essence-63" }
        );
        // 7 breaches only the soft law — still refused, and named honestly.
        assert_eq!(
            FutureOracle::admits_with(&t, PendingMoment::new(500, 1, 7), &book),
            Admission::LawViolation { law_id: "soft-prefer-even-essence" }
        );
        // A moment that obeys both is clean.
        assert!(FutureOracle::admits_with(&t, PendingMoment::new(500, 1, 10), &book).is_admitted());
    }

    // ── hazard scanning ──

    #[test]
    fn next_hazard_and_clear_until_on_clean_tape() {
        let t = tape_of(10);
        assert!(FutureOracle::next_hazard(&t, 0).is_none());
        assert_eq!(FutureOracle::clear_until(&t, 0), Some(900)); // clear all the way to horizon
    }

    #[test]
    fn clear_until_stops_at_the_first_hazard() {
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        t.record(300, 99, 1, Tier::Local, &ev(2)).unwrap(); // bad moon at 300
        t.record(500, 1, 1, Tier::Local, &ev(3)).unwrap();
        assert_eq!(FutureOracle::clear_until(&t, 0), Some(300));
        assert_eq!(FutureOracle::verdict(&t, 0), Verdict::Blocked);
    }

    // ── safest tick ──

    #[test]
    fn safest_tick_picks_first_admissible() {
        let t = tape_of(5); // last 400
        // 200/300 rewind history; 400 conflicts? essence 12 vs last essence 4 at 400 → conflict.
        let pick = FutureOracle::safest_tick(&t, 1, 12, &[200, 300, 400, 500, 600]);
        assert_eq!(pick, Some(500)); // first clean append slot
    }

    #[test]
    fn safest_tick_none_when_all_rewind() {
        let t = tape_of(5); // last 400
        assert_eq!(FutureOracle::safest_tick(&t, 1, 12, &[100, 200, 300]), None);
    }

    // ── projection diff ──

    #[test]
    fn diff_detects_a_fix() {
        let mut bad = TimelineTape::new(10);
        bad.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        bad.record(400, 200, 1, Tier::Local, &ev(2)).unwrap(); // bad moon
        let mut good = TimelineTape::new(10);
        good.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        good.record(400, 7, 1, Tier::Local, &ev(2)).unwrap();

        let pb = GhostPlayhead::at(0).project(&bad);
        let pg = GhostPlayhead::at(0).project(&good);
        let diff = ProjectionDiff::between(&pb, &pg);
        assert!(diff.is_improvement());
        assert!(!diff.regressed());
        assert_eq!(diff.cleared.len(), 1);
    }

    #[test]
    fn diff_detects_a_regression() {
        let mut good = TimelineTape::new(10);
        good.record(400, 7, 1, Tier::Local, &ev(1)).unwrap();
        let mut bad = TimelineTape::new(10);
        bad.record(400, 200, 1, Tier::Local, &ev(1)).unwrap();
        let diff = ProjectionDiff::between(
            &GhostPlayhead::at(0).project(&good),
            &GhostPlayhead::at(0).project(&bad),
        );
        assert!(diff.regressed());
        assert!(!diff.is_improvement());
    }

    #[test]
    fn diff_neutral_when_unchanged() {
        let t = tape_of(8);
        let a = GhostPlayhead::at(0).project(&t);
        let b = GhostPlayhead::at(0).project(&t);
        assert!(ProjectionDiff::between(&a, &b).is_neutral());
    }

    // ── branch ranking ──

    #[test]
    fn rank_branches_puts_clear_first() {
        let clean = tape_of(10);
        let mut faulty = TimelineTape::new(10);
        faulty.record(100, 1, 1, Tier::Local, &ev(1)).unwrap();
        faulty.record(200, 200, 1, Tier::Local, &ev(2)).unwrap(); // bad
        let ranked = FutureOracle::rank_branches(&[&faulty, &clean], 0);
        assert_eq!(ranked[0].branch, 1, "the clean branch ranks first");
        assert_eq!(ranked[0].verdict, Verdict::Clear);
        assert_eq!(ranked[1].verdict, Verdict::Blocked);
    }

    #[test]
    fn synthetic_projection_probe_scans() {
        // The probe path used by admits_with must scan a one-frame future.
        let frames = [SealedTuple {
            tick_id: 10,
            content_seal: 0,
            chain_seal: 0,
            moon: 1,
            essence_id: 200, // invalid
            source_kind: 0,
            flags: 0,
            reserved: 0,
        }];
        let contacts = CollisionRadar::scan_frames(0, &frames);
        assert_eq!(contacts[0].kind, CollisionKind::InvalidEssence);
    }
}
