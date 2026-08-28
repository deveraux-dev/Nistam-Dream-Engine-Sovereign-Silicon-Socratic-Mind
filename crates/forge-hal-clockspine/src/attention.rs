//! The attention queue — what the world will notice, and when, in 0.13 ms.
//!
//! Drained from v2 `forge-core::tick_schedule` bones (32-byte events, fixed
//! array, sorted by fire tick, O(d) drain) and re-founded on the a000 law:
//! recognition is **0.13 ms** (SESSION-HANDOFF:498 "prose demoted to title
//! hover (a000: recognition is 0.13ms)"). The queue's whole per-tick drain
//! must fit inside that budget, and the budget is a TEST here, not a wish
//! (L01: if an assert can hold it, hold it with an assert).
//!
//! No allocation after construction. No floats. The caller owns tick time —
//! this structure never reads a clock (W15b: wall time converts to ticks
//! exactly once, far from here).

use crate::fixed::SimTick;

/// Queue capacity — 256 events x 32 bytes = 8 KiB, L2-resident by design.
pub const ATTENTION_CAP: usize = 256;

/// The a000 recognition budget in microseconds: a full drain must fit here.
pub const ATTENTION_BUDGET_US: u64 = 130;

/// One scheduled noticing: 32 bytes exactly, `Copy`, integer-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttentionEvent {
    /// The tick this event fires on.
    pub fire_tick: SimTick,
    /// Re-arm interval in ticks; 0 = one-shot.
    pub repeat_ticks: u32,
    /// The entity being noticed (game-defined id).
    pub entity: u32,
    /// Domain tag (FNV of the sensation family, game-defined).
    pub tag: u64,
    /// One payload word (sensation index, permyriad, whatever the tag says).
    pub payload: u64,
}

/// Fixed-capacity attention queue: sorted ascending by `fire_tick`, drained
/// front-first each tick. Full is a refused push, never a silent drop.
#[derive(Debug)]
pub struct AttentionQueue {
    /// Backing store; `len` live events sorted ascending by fire tick.
    events: [AttentionEvent; ATTENTION_CAP],
    len: usize,
}

impl Default for AttentionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl AttentionQueue {
    /// An empty queue.
    pub fn new() -> Self {
        const ZERO: AttentionEvent = AttentionEvent {
            fire_tick: SimTick(0),
            repeat_ticks: 0,
            entity: 0,
            tag: 0,
            payload: 0,
        };
        Self { events: [ZERO; ATTENTION_CAP], len: 0 }
    }

    /// Live event count.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when nothing is scheduled.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Schedule an event, keeping the array sorted by fire tick. Returns
    /// `false` when full — the caller must handle the refusal LOUDLY.
    pub fn schedule(&mut self, ev: AttentionEvent) -> bool {
        if self.len == ATTENTION_CAP {
            return false;
        }
        // Insertion point: first slot with a later fire tick (stable for
        // equal ticks: earlier schedules fire first).
        let mut at = self.len;
        for (i, e) in self.events[..self.len].iter().enumerate() {
            if e.fire_tick.0 > ev.fire_tick.0 {
                at = i;
                break;
            }
        }
        self.events.copy_within(at..self.len, at + 1);
        self.events[at] = ev;
        self.len += 1;
        true
    }

    /// Drain everything due at or before `now` into `out`, re-arming
    /// repeating events. Returns how many fired. O(d) in fired events; the
    /// budget test below holds the whole worst case under
    /// [`ATTENTION_BUDGET_US`].
    pub fn drain_due(&mut self, now: SimTick, out: &mut [Option<AttentionEvent>]) -> usize {
        let mut fired = 0usize;
        while fired < out.len() && self.len > 0 && self.events[0].fire_tick.0 <= now.0 {
            let ev = self.events[0];
            self.events.copy_within(1..self.len, 0);
            self.len -= 1;
            out[fired] = Some(ev);
            fired += 1;
            if ev.repeat_ticks > 0 {
                let mut re = ev;
                re.fire_tick = SimTick(ev.fire_tick.0 + ev.repeat_ticks as u64);
                // Re-arm cannot fail: we just freed a slot.
                let ok = self.schedule(re);
                debug_assert!(ok, "re-arm into a freed slot cannot fail");
            }
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(fire: u64, entity: u32) -> AttentionEvent {
        AttentionEvent {
            fire_tick: SimTick(fire),
            repeat_ticks: 0,
            entity,
            tag: 0x13,
            payload: 0,
        }
    }

    /// L02: the event's size is emitted by the compiler, not stated in prose.
    #[test]
    fn an_attention_event_is_exactly_32_bytes() {
        assert_eq!(std::mem::size_of::<AttentionEvent>(), 32);
        assert_eq!(std::mem::size_of::<AttentionQueue>(), 32 * ATTENTION_CAP + 8);
    }

    /// Sorted insert holds: whatever order events arrive, they fire in tick
    /// order, and equal ticks fire in schedule order.
    #[test]
    fn events_fire_in_tick_order() {
        let mut q = AttentionQueue::new();
        assert!(q.schedule(ev(30, 3)));
        assert!(q.schedule(ev(10, 1)));
        assert!(q.schedule(ev(20, 2)));
        assert!(q.schedule(ev(10, 4)), "equal tick, later schedule");
        let mut out = [None; ATTENTION_CAP];
        let n = q.drain_due(SimTick(30), &mut out);
        assert_eq!(n, 4);
        let order: Vec<u32> = out[..n].iter().map(|e| e.unwrap().entity).collect();
        assert_eq!(order, [1, 4, 2, 3]);
        assert!(q.is_empty());
    }

    /// Nothing early fires: draining at tick 9 leaves a tick-10 event alone.
    #[test]
    fn the_future_stays_scheduled() {
        let mut q = AttentionQueue::new();
        q.schedule(ev(10, 1));
        let mut out = [None; 4];
        assert_eq!(q.drain_due(SimTick(9), &mut out), 0);
        assert_eq!(q.len(), 1);
    }

    /// A repeating event re-arms itself at fire+interval, forever.
    #[test]
    fn a_heartbeat_rearms() {
        let mut q = AttentionQueue::new();
        let mut beat = ev(5, 7);
        beat.repeat_ticks = 5;
        q.schedule(beat);
        let mut out = [None; 4];
        for turn in 1..=4u64 {
            let n = q.drain_due(SimTick(turn * 5), &mut out);
            assert_eq!(n, 1, "beat {turn} missed");
            assert_eq!(q.len(), 1, "the beat must re-arm");
            out[0] = None;
        }
    }

    /// Full is a refusal, not a drop.
    #[test]
    fn a_full_queue_refuses_loudly() {
        let mut q = AttentionQueue::new();
        for i in 0..ATTENTION_CAP as u64 {
            assert!(q.schedule(ev(i, i as u32)));
        }
        assert!(!q.schedule(ev(0, 999)), "the 257th event must be refused");
        assert_eq!(q.len(), ATTENTION_CAP);
    }

    /// THE BUDGET (a000): the worst case — a completely full queue, every
    /// event due, all re-arming — drains inside 0.13 ms. Measured over
    /// several passes and judged on the BEST pass so a loaded test machine
    /// cannot fake a regression; a real regression slows every pass.
    #[test]
    fn a_full_drain_fits_in_the_recognition_budget() {
        let mut best_us = u64::MAX;
        for _ in 0..16 {
            let mut q = AttentionQueue::new();
            for i in 0..ATTENTION_CAP as u64 {
                let mut e = ev(i % 7, i as u32);
                e.repeat_ticks = 1000; // worst case: every fire re-arms (sorted re-insert)
                q.schedule(e);
            }
            let mut out = [None; ATTENTION_CAP];
            let start = std::time::Instant::now();
            let n = q.drain_due(SimTick(10), &mut out);
            let took = start.elapsed().as_micros() as u64;
            assert_eq!(n, ATTENTION_CAP, "every event was due");
            best_us = best_us.min(took);
        }
        assert!(
            best_us <= ATTENTION_BUDGET_US,
            "a full attention drain took {best_us}us — the a000 budget is {ATTENTION_BUDGET_US}us"
        );
    }
}

