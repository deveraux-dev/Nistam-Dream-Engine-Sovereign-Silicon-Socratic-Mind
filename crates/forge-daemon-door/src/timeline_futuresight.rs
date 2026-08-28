//! timeline_futuresight.rs — the daemon's live Ghost. Runs `forge_ump_v3`'s futuresight
//! over the recorder's tape (`timeline_recorder::snapshot`), so the ONE BIN — forge-ump
//! tape → daemon futuresight → studio embed — sees collisions in projection before the
//! real clock reaches them. Read-only; gated `FORGE_TIMELINE=1` (the recorder's gate). The
//! daemon (and the studio that embeds it in-process) query this for the plan's verdict +
//! hazards. Ported from `F:\NewRepo\crates\forge-daemon\src\timeline_futuresight.rs`, with
//! `forge_ump::{Admission, Contact, FutureOracle, PendingMoment, Verdict}` →
//! `forge_ump_v3::{Admission, Contact, FutureOracle, PendingMoment, Verdict}` (Layer 2.5).

use std::sync::atomic::{AtomicU64, Ordering};

use forge_ump_v3::{Admission, Contact, FutureOracle, PendingMoment, Verdict};

/// The ghost's anchor — the real-clock "now" the daemon advances as it ticks.
static NOW: AtomicU64 = AtomicU64::new(0);

/// A readable futuresight status — what a `futuresight` status pane / MCP tool returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesightStatus {
    /// The current rolled verdict over the projected future.
    pub verdict: Verdict,
    /// The ghost's real-clock anchor at the time of this status.
    pub now: u64,
    /// The furthest pending tick seen, if any.
    pub horizon: Option<u64>,
    /// Ticks between `now` and `horizon`.
    pub lead: u64,
    /// Number of pending radar contacts.
    pub hazard_count: usize,
    /// The soonest predicted collision, if any.
    pub first_hazard: Option<Contact>,
    /// Tick up to which the plan is collision-free.
    pub clear_until: Option<u64>,
}

/// Advance the ghost's anchor as the daemon clock ticks forward.
pub fn advance_now(tick: u64) {
    NOW.store(tick, Ordering::Relaxed);
}

/// The current ghost anchor.
pub fn now() -> u64 {
    NOW.load(Ordering::Relaxed)
}

/// Current futuresight verdict over the recorder's tape (Clear when disabled).
pub fn verdict() -> Verdict {
    if !crate::timeline_recorder::enabled() {
        return Verdict::Clear;
    }
    FutureOracle::verdict(&crate::timeline_recorder::snapshot(), now())
}

/// Would appending `(tick, moon, essence)` to the tape be admitted? The pre-commit radar —
/// the daemon can consult this BEFORE it seals, catching a bad moment in projection.
pub fn admits(tick: u64, moon: u8, essence_id: u8) -> Admission {
    FutureOracle::admits(&crate::timeline_recorder::snapshot(), PendingMoment::new(tick, moon, essence_id))
}

/// The soonest hazard in the tape's pending future.
pub fn next_hazard() -> Option<Contact> {
    if !crate::timeline_recorder::enabled() {
        return None;
    }
    FutureOracle::next_hazard(&crate::timeline_recorder::snapshot(), now())
}

/// The tick up to which the plan is collision-free (horizon if fully clear).
pub fn clear_until() -> Option<u64> {
    FutureOracle::clear_until(&crate::timeline_recorder::snapshot(), now())
}

/// The full status snapshot for a status pane / tool.
pub fn status() -> FuturesightStatus {
    let tape = crate::timeline_recorder::snapshot();
    let n = now();
    let verdict = if crate::timeline_recorder::enabled() {
        FutureOracle::verdict(&tape, n)
    } else {
        Verdict::Clear
    };
    let first = FutureOracle::next_hazard(&tape, n);
    // horizon + lead via the last committed tick past now.
    let (horizon, lead) = match tape.last() {
        Some(e) if e.tick_id > n => (Some(e.tick_id), e.tick_id - n),
        _ => (None, 0),
    };
    let hazard_count = if first.is_some() {
        // count all contacts, not just the first.
        forge_ump_v3::CollisionRadar::scan(&forge_ump_v3::GhostPlayhead::at(n).project(&tape)).len()
    } else {
        0
    };
    FuturesightStatus {
        verdict,
        now: n,
        horizon,
        lead,
        hazard_count,
        first_hazard: first,
        clear_until: FutureOracle::clear_until(&tape, n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_verdict_is_clear() {
        // FORGE_TIMELINE unset in tests → the gate is closed → Clear, no panic.
        assert!(!crate::timeline_recorder::enabled());
        assert_eq!(verdict(), Verdict::Clear);
    }

    #[test]
    fn admits_validates_against_the_live_tape() {
        // The global tape is empty in tests (recorder gate off), so validity is all that gates.
        assert!(admits(100, 7, 12).is_admitted());
        assert_eq!(admits(100, 1, 200), Admission::InvalidEssence);
        assert_eq!(admits(100, 99, 5), Admission::InvalidMoon);
    }

    #[test]
    fn advance_now_moves_the_anchor() {
        advance_now(1234);
        assert_eq!(now(), 1234);
        advance_now(0); // reset for other tests
    }

    #[test]
    fn status_is_readable_when_idle() {
        advance_now(0);
        let s = status();
        assert_eq!(s.verdict, Verdict::Clear);
        assert_eq!(s.hazard_count, 0);
        assert!(s.first_hazard.is_none());
    }
}
