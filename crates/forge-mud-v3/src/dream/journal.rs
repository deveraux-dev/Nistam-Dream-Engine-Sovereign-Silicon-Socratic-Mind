//! Dream Journal — the day's reel, printed at sleep.
//!
//! Field shape ported from the donor DJ tool's post-set review journal
//! (`F:\NewRepo\work\dream_diamonds\crates\dream_journal.rs:17-27`) — same
//! quality/incident fields, `timestamp: String` swapped for `sleep_tick: u64`
//! (ticks, not wall-clock — this crate's whole convention, and the reason
//! `forge-envelope`'s `EphemeralEnvelope` shreds on tick deadlines too), and
//! the DJ-specific `TrackTransition`/`save()` JSON persistence dropped —
//! out of scope for the MUD's in-memory session journal.

/// Sentinel byte for sleep/wake events (`ORACLE-C-DREAM-DIAMONDS-EUX.md:110`).
/// Same byte, same meaning as the `#vixi:geom` cart dialect's own mapping
/// (`forge-vix-v3/src/geom.rs:177-178`, `"sleep"|"wake" => 246`) — this is a
/// cross-reference, not a second implementation; the two never collide
/// because the MUD cart path never routes through `forge_core_v3::sentinel`.
pub const SENTINEL_SLEEP_WAKE: u8 = 246;

/// Sentinel byte for the gift a night leaves (`§8:238`, `"gift" => 247` in
/// `forge-vix-v3/src/geom.rs:178`). Distinct from the sleep/wake mark: 246 is
/// the crossing, 247 is what survives it.
pub const SENTINEL_GIFT: u8 = 247;

/// The day's reel, printed once at sleep (`ORACLE-C-DREAM-DIAMONDS-EUX.md:230-231`).
#[derive(Clone, Debug, PartialEq)]
pub struct DreamJournal {
    /// The tick this journal was printed on (sleep event).
    pub sleep_tick: u64,
    /// Best moment of the session, 0.0..=1.0.
    pub peak_quality: f32,
    /// Worst moment of the session, 0.0..=1.0.
    pub lowest_quality: f32,
    /// Count of clipping incidents.
    pub clipping_events: u64,
    /// Count of dead-air incidents.
    pub dead_air_events: u64,
    /// Count of phase-drift incidents.
    pub phase_drift_incidents: u64,
    /// Short authored summary of the session.
    pub narrative: String,
}

impl DreamJournal {
    /// A fresh journal for a session starting at `sleep_tick` with no incidents yet.
    pub fn new(sleep_tick: u64) -> Self {
        Self {
            sleep_tick,
            peak_quality: 0.0,
            lowest_quality: 1.0,
            clipping_events: 0,
            dead_air_events: 0,
            phase_drift_incidents: 0,
            narrative: String::new(),
        }
    }

    /// Fold one quality sample into the running peak/lowest — the journal
    /// only ever widens its recorded range, never narrows it.
    pub fn observe_quality(&mut self, quality: f32) {
        let q = quality.clamp(0.0, 1.0);
        if q > self.peak_quality {
            self.peak_quality = q;
        }
        if q < self.lowest_quality {
            self.lowest_quality = q;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_matches_the_cart_dialects_own_mapping() {
        assert_eq!(SENTINEL_SLEEP_WAKE, 246);
        assert_eq!(SENTINEL_GIFT, 247);
        assert_ne!(SENTINEL_GIFT, SENTINEL_SLEEP_WAKE, "the crossing is not the survivor");
    }

    #[test]
    fn fresh_journal_has_no_incidents() {
        let j = DreamJournal::new(120);
        assert_eq!(j.sleep_tick, 120);
        assert_eq!(j.clipping_events, 0);
        assert_eq!(j.dead_air_events, 0);
        assert_eq!(j.phase_drift_incidents, 0);
    }

    #[test]
    fn observe_quality_widens_the_range_only() {
        let mut j = DreamJournal::new(0);
        j.observe_quality(0.5);
        assert_eq!(j.peak_quality, 0.5);
        assert_eq!(j.lowest_quality, 0.5);
        j.observe_quality(0.8);
        j.observe_quality(0.2);
        assert_eq!(j.peak_quality, 0.8);
        assert_eq!(j.lowest_quality, 0.2);
        j.observe_quality(0.6);
        assert_eq!(j.peak_quality, 0.8, "a middling sample must not narrow the peak");
        assert_eq!(j.lowest_quality, 0.2, "a middling sample must not narrow the lowest");
    }
}
