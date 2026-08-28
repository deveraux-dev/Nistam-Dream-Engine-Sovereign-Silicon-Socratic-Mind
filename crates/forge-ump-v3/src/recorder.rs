//! recorder.rs — the live-append side of the tape + secondary indices. A [`Recorder`]
//! buffers UMP events as they flow and seals them into a [`TimelineTape`] entry on
//! `commit(tick, moon, essence)`; a [`TapeIndex`] gives O(1) scrub-by-moon / scrub-by-
//! codeword and finds epoch boundaries. This is the seam a live producer calls — the
//! tape stops being an orphan the moment something `observe`s + `commit`s here.

use crate::packet::{Stamped, Ump};
use crate::provenance_tag::Tier;
use crate::timeline::{SealedTuple, TimelineError, TimelineTape};

/// Accumulates UMP events for the current tick, then seals them into one tape entry.
///
/// A live producer does: `rec.observe(ump)…` as events arrive, then
/// `rec.commit(clock.tick_id, moon, essence_id)` once per committed moment. Pending
/// events are folded into that moment's `content_seal` and cleared.
pub struct Recorder {
    tape: TimelineTape,
    pending: Vec<Stamped<Ump>>,
    tier: Tier,
    /// Count of commits that carried no events (heartbeats) — a liveness gauge.
    heartbeats: u64,
}

impl Recorder {
    /// A recorder sealing at `jr_quantize_us` jitter granularity, tagging every commit `tier`.
    pub fn new(jr_quantize_us: i64, tier: Tier) -> Self {
        Self {
            tape: TimelineTape::new(jr_quantize_us),
            pending: Vec::new(),
            tier,
            heartbeats: 0,
        }
    }

    /// Resume recording onto an already-sealed tape (e.g. one loaded from
    /// `.forge/timeline.chain` at boot). The tape's own `jr_quantize_us` and chain head
    /// carry forward, so appends continue the SAME verified chain — durable across restart.
    pub fn from_tape(tape: TimelineTape, tier: Tier) -> Self {
        Self {
            tape,
            pending: Vec::new(),
            tier,
            heartbeats: 0,
        }
    }

    /// Retag subsequent commits (e.g. a HITL verdict promotes Cloud → HumanVerified).
    pub fn set_tier(&mut self, tier: Tier) {
        self.tier = tier;
    }

    /// The current provenance tier.
    #[inline]
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// Buffer one event for the current (uncommitted) moment.
    pub fn observe(&mut self, event: Stamped<Ump>) {
        self.pending.push(event);
    }

    /// Buffer many events at once.
    pub fn observe_slice(&mut self, events: &[Stamped<Ump>]) {
        self.pending.extend_from_slice(events);
    }

    /// How many events are buffered for the next commit.
    #[inline]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Drop buffered events without committing them (abort the current moment).
    pub fn discard_pending(&mut self) -> usize {
        let n = self.pending.len();
        self.pending.clear();
        n
    }

    /// Seal the buffered events into one entry at `(tick_id, moon, essence_id)` and
    /// clear the buffer. A commit with no events is a valid heartbeat (silence recorded).
    pub fn commit(
        &mut self,
        tick_id: u64,
        moon: u8,
        essence_id: u8,
    ) -> Result<SealedTuple, TimelineError> {
        if self.pending.is_empty() {
            self.heartbeats += 1;
        }
        let entry = self.tape.record(tick_id, moon, essence_id, self.tier, &self.pending)?;
        self.pending.clear();
        Ok(entry)
    }

    /// Commit an explicit heartbeat (no events) — records that a tick passed in silence.
    pub fn heartbeat(
        &mut self,
        tick_id: u64,
        moon: u8,
        essence_id: u8,
    ) -> Result<SealedTuple, TimelineError> {
        self.discard_pending();
        self.commit(tick_id, moon, essence_id)
    }

    /// Number of event-less commits so far.
    #[inline]
    pub fn heartbeats(&self) -> u64 {
        self.heartbeats
    }

    /// Borrow the growing tape.
    #[inline]
    pub fn tape(&self) -> &TimelineTape {
        &self.tape
    }

    /// How many moments have been committed.
    #[inline]
    pub fn len(&self) -> usize {
        self.tape.len()
    }

    /// True until the first commit.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tape.is_empty()
    }

    /// Consume the recorder, yielding the finished tape (drops any pending events).
    pub fn into_tape(self) -> TimelineTape {
        self.tape
    }
}

/// A moon transition on the tape — an epoch boundary the scrubber marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoonTransition {
    /// Index of the first entry in the NEW moon.
    pub index: usize,
    /// Moon of the entry just before the transition.
    pub from_moon: u8,
    /// Moon of the entry the transition lands on.
    pub to_moon: u8,
    /// Tick of the first entry in the new moon.
    pub tick_id: u64,
}

/// Secondary indices over a tape: by-moon lanes, by-codeword lanes, and histograms.
/// Built once from a snapshot; rebuild after more records land.
pub struct TapeIndex {
    /// `by_moon[m]` = entry indices with moon `m` (m in 0..=13; 0 = unbound).
    by_moon: [Vec<usize>; 14],
    /// `by_essence[id]` = entry indices with codeword `id` (0..=63).
    by_essence: Vec<Vec<usize>>,
    transitions: Vec<MoonTransition>,
    len: usize,
}

impl TapeIndex {
    /// Build the indices from a tape snapshot.
    pub fn build(tape: &TimelineTape) -> Self {
        let mut by_moon: [Vec<usize>; 14] = Default::default();
        let mut by_essence: Vec<Vec<usize>> = (0..64).map(|_| Vec::new()).collect();
        let mut transitions = Vec::new();
        let mut prev_moon: Option<u8> = None;
        for (i, e) in tape.entries().iter().enumerate() {
            let m = if e.moon <= 13 { e.moon } else { 0 };
            by_moon[m as usize].push(i);
            if (e.essence_id as usize) < by_essence.len() {
                by_essence[e.essence_id as usize].push(i);
            }
            if let Some(pm) = prev_moon {
                if pm != e.moon {
                    transitions.push(MoonTransition {
                        index: i,
                        from_moon: pm,
                        to_moon: e.moon,
                        tick_id: e.tick_id,
                    });
                }
            }
            prev_moon = Some(e.moon);
        }
        Self { by_moon, by_essence, transitions, len: tape.len() }
    }

    /// Entry indices recorded under `moon` (0 = unbound; >13 folds to unbound).
    pub fn entries_for_moon(&self, moon: u8) -> &[usize] {
        let m = if moon <= 13 { moon } else { 0 };
        &self.by_moon[m as usize]
    }

    /// Entry indices carrying codeword `essence_id` (out of range = empty).
    pub fn entries_for_essence(&self, essence_id: u8) -> &[usize] {
        match self.by_essence.get(essence_id as usize) {
            Some(v) => v,
            None => &[],
        }
    }

    /// Per-moon commit counts, index = moon (0..=13).
    pub fn moon_histogram(&self) -> [usize; 14] {
        let mut h = [0usize; 14];
        for (m, v) in self.by_moon.iter().enumerate() {
            h[m] = v.len();
        }
        h
    }

    /// Per-codeword commit counts, index = essence_id (0..=63).
    pub fn essence_histogram(&self) -> [usize; 64] {
        let mut h = [0usize; 64];
        for (id, v) in self.by_essence.iter().enumerate() {
            if id < 64 {
                h[id] = v.len();
            }
        }
        h
    }

    /// The epoch boundaries — every moon change, in tape order.
    #[inline]
    pub fn transitions(&self) -> &[MoonTransition] {
        &self.transitions
    }

    /// The codeword that sounded most often (id, count); `None` for an empty tape.
    pub fn densest_essence(&self) -> Option<(u8, usize)> {
        // First-wins on ties (lowest codeword id) — deterministic, unlike max_by_key.
        self.by_essence
            .iter()
            .enumerate()
            .map(|(id, v)| (id as u8, v.len()))
            .filter(|&(_, c)| c > 0)
            .fold(None, |best, cur| match best {
                Some(b) if b.1 >= cur.1 => Some(b),
                _ => Some(cur),
            })
    }

    /// Distinct moons that appear on the tape, ascending.
    pub fn moons_present(&self) -> Vec<u8> {
        (0u8..14)
            .filter(|&m| !self.by_moon[m as usize].is_empty())
            .collect()
    }

    /// Total entries the index covers.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    /// True when the indexed tape had no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Stamped, Ump};
    use crate::timeline::TimelineTape;

    fn ev(tag: u32) -> Stamped<Ump> {
        Stamped { universal_tick_us: tag as i64, payload: Ump::new([tag, 0, 0, 0]) }
    }

    // ── recorder ──

    #[test]
    fn recorder_buffers_then_commits() {
        let mut r = Recorder::new(10, Tier::Local);
        assert!(r.is_empty());
        r.observe(ev(1));
        r.observe(ev(2));
        assert_eq!(r.pending_len(), 2);
        let e = r.commit(100, 3, 40).unwrap();
        assert_eq!(e.tick_id, 100);
        assert_eq!(e.moon, 3);
        assert_eq!(e.essence_id, 40);
        assert_eq!(r.pending_len(), 0, "commit clears the buffer");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn buffered_events_change_the_seal() {
        let mut a = Recorder::new(10, Tier::Local);
        let mut b = Recorder::new(10, Tier::Local);
        a.observe(ev(1));
        let ea = a.commit(0, 1, 0).unwrap();
        let eb = b.commit(0, 1, 0).unwrap(); // no events
        assert_ne!(ea.content_seal, eb.content_seal, "events must fold into the seal");
    }

    #[test]
    fn observe_slice_and_discard() {
        let mut r = Recorder::new(10, Tier::Local);
        r.observe_slice(&[ev(1), ev(2), ev(3)]);
        assert_eq!(r.pending_len(), 3);
        assert_eq!(r.discard_pending(), 3);
        assert_eq!(r.pending_len(), 0);
    }

    #[test]
    fn empty_commit_counts_as_heartbeat() {
        let mut r = Recorder::new(10, Tier::Local);
        r.commit(0, 1, 0).unwrap(); // silence
        r.observe(ev(1));
        r.commit(1, 1, 0).unwrap(); // not silent
        r.heartbeat(2, 1, 0).unwrap(); // explicit heartbeat, drops nothing pending
        assert_eq!(r.heartbeats(), 2);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn heartbeat_drops_pending() {
        let mut r = Recorder::new(10, Tier::Local);
        r.observe(ev(9));
        let e = r.heartbeat(5, 2, 7).unwrap();
        assert_eq!(r.pending_len(), 0);
        // equals a bare no-event commit at the same coord
        let mut r2 = Recorder::new(10, Tier::Local);
        let e2 = r2.commit(5, 2, 7).unwrap();
        assert_eq!(e.content_seal, e2.content_seal);
    }

    #[test]
    fn from_tape_resumes_the_same_chain_across_restart() {
        // Session 1: record, seal, "persist" (serialize).
        let mut r = Recorder::new(10, Tier::Cloud);
        for i in 0..8u64 {
            r.observe(ev(i as u32 + 1));
            r.commit(i * 10, ((i % 13) + 1) as u8, (i % 64) as u8).unwrap();
        }
        let bytes = r.into_tape().to_bytes();

        // Session 2 (fresh process): load the tape and RESUME on it.
        let loaded = TimelineTape::from_bytes(&bytes).unwrap();
        let head_before = loaded.chain_root();
        let mut r2 = Recorder::from_tape(loaded, Tier::Cloud);
        assert_eq!(r2.len(), 8, "resumed recorder sees the full history");
        // Appends continue the SAME chain, not a genesis restart.
        r2.observe(ev(99));
        let e = r2.commit(80, 1, 5).unwrap();
        assert_ne!(e.chain_seal, head_before, "the new link folds the loaded head");
        assert!(r2.tape().verify_chain().is_ok(), "resumed chain stays intact");
        assert_eq!(r2.len(), 9);
    }

    #[test]
    fn recorded_tape_verifies_and_survives_serde() {
        let mut r = Recorder::new(10, Tier::HumanVerified);
        for i in 0..50u64 {
            r.observe(ev(i as u32 + 1));
            r.commit(i * 10, ((i % 13) + 1) as u8, (i % 64) as u8).unwrap();
        }
        let tape = r.into_tape();
        assert!(tape.verify_chain().is_ok());
        let back = TimelineTape::from_bytes(&tape.to_bytes()).unwrap();
        assert_eq!(back, tape);
    }

    #[test]
    fn recorder_rejects_backward_tick() {
        let mut r = Recorder::new(10, Tier::Local);
        r.commit(100, 1, 0).unwrap();
        assert!(matches!(r.commit(50, 1, 0), Err(TimelineError::NonMonotonic { .. })));
    }

    // ── index ──

    fn tape_moons(seq: &[u8]) -> TimelineTape {
        let mut t = TimelineTape::new(10);
        for (i, &m) in seq.iter().enumerate() {
            t.record(i as u64 * 10, m, (i % 64) as u8, Tier::Local, &[]).unwrap();
        }
        t
    }

    #[test]
    fn index_groups_by_moon() {
        let t = tape_moons(&[1, 1, 2, 1, 3, 3]);
        let idx = TapeIndex::build(&t);
        assert_eq!(idx.entries_for_moon(1), &[0, 1, 3]);
        assert_eq!(idx.entries_for_moon(2), &[2]);
        assert_eq!(idx.entries_for_moon(3), &[4, 5]);
        assert!(idx.entries_for_moon(7).is_empty());
    }

    #[test]
    fn index_groups_by_essence() {
        let mut t = TimelineTape::new(10);
        t.record(0, 1, 5, Tier::Local, &[]).unwrap();
        t.record(1, 1, 9, Tier::Local, &[]).unwrap();
        t.record(2, 1, 5, Tier::Local, &[]).unwrap();
        let idx = TapeIndex::build(&t);
        assert_eq!(idx.entries_for_essence(5), &[0, 2]);
        assert_eq!(idx.entries_for_essence(9), &[1]);
        assert!(idx.entries_for_essence(63).is_empty());
        assert!(idx.entries_for_essence(200).is_empty(), "out of range = empty");
    }

    #[test]
    fn index_finds_moon_transitions() {
        let t = tape_moons(&[1, 1, 2, 2, 2, 3]);
        let idx = TapeIndex::build(&t);
        let tr = idx.transitions();
        assert_eq!(tr.len(), 2);
        assert_eq!(tr[0], MoonTransition { index: 2, from_moon: 1, to_moon: 2, tick_id: 20 });
        assert_eq!(tr[1], MoonTransition { index: 5, from_moon: 2, to_moon: 3, tick_id: 50 });
    }

    #[test]
    fn index_histograms_and_density() {
        let t = tape_moons(&[1, 1, 1, 2]);
        let idx = TapeIndex::build(&t);
        let mh = idx.moon_histogram();
        assert_eq!(mh[1], 3);
        assert_eq!(mh[2], 1);
        assert_eq!(mh[5], 0);
        // essence ids were i%64 = 0,1,2,3 → all distinct, density 1 each, lowest id wins tie
        assert_eq!(idx.densest_essence(), Some((0, 1)));
        assert_eq!(idx.moons_present(), vec![1, 2]);
        assert_eq!(idx.len(), 4);
    }

    #[test]
    fn densest_essence_picks_the_max() {
        let mut t = TimelineTape::new(10);
        for (i, &id) in [7u8, 7, 7, 3, 3, 9].iter().enumerate() {
            t.record(i as u64, 1, id, Tier::Local, &[]).unwrap();
        }
        let idx = TapeIndex::build(&t);
        assert_eq!(idx.densest_essence(), Some((7, 3)));
    }

    #[test]
    fn empty_tape_index_is_inert() {
        let t = TimelineTape::new(10);
        let idx = TapeIndex::build(&t);
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        assert!(idx.transitions().is_empty());
        assert_eq!(idx.densest_essence(), None);
        assert!(idx.moons_present().is_empty());
        assert_eq!(idx.moon_histogram(), [0usize; 14]);
    }

    #[test]
    fn unbound_moon_folds_to_slot_zero() {
        let t = tape_moons(&[0, 200, 5]); // 0 and 200 → unbound slot 0
        let idx = TapeIndex::build(&t);
        assert_eq!(idx.entries_for_moon(0), &[0, 1]);
        assert_eq!(idx.entries_for_moon(5), &[2]);
    }
}
