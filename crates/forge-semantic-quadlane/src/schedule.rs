//! u64 tick-keyed scheduling with due-event draining.
//!
//! Fixed-capacity, heap-free (no_hotpath_alloc, COMPILER-GROUPS.md §4 gap 1):
//! the schedule is an inline array of [`SCHEDULE_CAP`] slots. `arm` past
//! capacity fails loud with [`ScheduleError::CapacityExceeded`] instead of
//! reallocating; draining writes into a caller-owned fixed array.

/// Maximum events the schedule holds at once. Arm #65 is refused, not grown.
pub const SCHEDULE_CAP: usize = 64;

/// A scheduled event tied to an absolute master tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScheduledEvent {
    /// Absolute tick at which this event fires.
    pub fire_tick: u64,
    /// Phrase kind tag (0..=255 typically).
    pub tag: u32,
    /// Reserved for future use; do not rely on interpretation.
    pub _reserved: [u8; 4],
}

impl ScheduledEvent {
    /// The all-zero slot filler. Never observable through the public API:
    /// only `events[..len]` is live.
    pub const EMPTY: Self = Self { fire_tick: 0, tag: 0, _reserved: [0; 4] };
}

/// Errors from schedule operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleError {
    /// No error; operation succeeded (never returned).
    Ok,
    /// Schedule capacity exceeded.
    CapacityExceeded,
}

/// u64 tick-based event schedule. Stores armed events inline and drains due ones.
#[derive(Debug, Clone)]
pub struct TickSchedule {
    events: [ScheduledEvent; SCHEDULE_CAP],
    len: usize,
}

impl Default for TickSchedule {
    fn default() -> Self {
        Self::new()
    }
}

impl TickSchedule {
    /// Create a new empty schedule.
    pub fn new() -> Self {
        Self { events: [ScheduledEvent::EMPTY; SCHEDULE_CAP], len: 0 }
    }

    /// Count of currently armed events.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when no events are armed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Arm an event to fire at its `fire_tick`. Refuses (loud, not silent) once
    /// [`SCHEDULE_CAP`] events are armed.
    pub fn arm(&mut self, event: ScheduledEvent) -> Result<(), ScheduleError> {
        if self.len == SCHEDULE_CAP {
            return Err(ScheduleError::CapacityExceeded);
        }
        self.events[self.len] = event;
        self.len += 1;
        Ok(())
    }

    /// Cancel every armed instance of tag `tag`. Returns count removed.
    pub fn cancel(&mut self, tag: u32) -> usize {
        let before = self.len;
        let mut keep = 0;
        for i in 0..self.len {
            if self.events[i].tag != tag {
                self.events[keep] = self.events[i];
                keep += 1;
            }
        }
        self.len = keep;
        before - self.len
    }

    /// Drain every event due at or before tick `now` into `out`. Returns the
    /// count drained; `out[..count]` is live, the tail is untouched. `out` is
    /// [`SCHEDULE_CAP`]-sized, so the drain can never overflow it.
    pub fn drain_due(&mut self, now: u64, out: &mut [ScheduledEvent; SCHEDULE_CAP]) -> usize {
        let mut drained = 0;
        let mut i = 0;
        while i < self.len {
            if self.events[i].fire_tick <= now {
                out[drained] = self.events[i];
                drained += 1;
                self.len -= 1;
                self.events[i] = self.events[self.len];
            } else {
                i += 1;
            }
        }
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_event_round_trip() {
        let ev = ScheduledEvent {
            fire_tick: 42,
            tag: 99,
            _reserved: [0; 4],
        };
        assert_eq!(ev.fire_tick, 42);
        assert_eq!(ev.tag, 99);
    }

    #[test]
    fn tick_schedule_arm_and_drain() {
        let mut sched = TickSchedule::new();
        sched.arm(ScheduledEvent { fire_tick: 10, tag: 1, _reserved: [0; 4] }).unwrap();
        sched.arm(ScheduledEvent { fire_tick: 20, tag: 2, _reserved: [0; 4] }).unwrap();

        let mut out = [ScheduledEvent::EMPTY; SCHEDULE_CAP];
        let n = sched.drain_due(15, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].tag, 1);

        let n = sched.drain_due(25, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].tag, 2);
        assert!(sched.is_empty());
    }

    #[test]
    fn tick_schedule_cancel() {
        let mut sched = TickSchedule::new();
        sched.arm(ScheduledEvent { fire_tick: 10, tag: 5, _reserved: [0; 4] }).unwrap();
        sched.arm(ScheduledEvent { fire_tick: 20, tag: 5, _reserved: [0; 4] }).unwrap();
        sched.arm(ScheduledEvent { fire_tick: 30, tag: 9, _reserved: [0; 4] }).unwrap();

        let removed = sched.cancel(5);
        assert_eq!(removed, 2);

        let mut out = [ScheduledEvent::EMPTY; SCHEDULE_CAP];
        let n = sched.drain_due(100, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].tag, 9);
    }

    #[test]
    fn arm_65th_is_refused_loud_not_grown() {
        let mut sched = TickSchedule::new();
        for i in 0..SCHEDULE_CAP as u64 {
            sched
                .arm(ScheduledEvent { fire_tick: i, tag: i as u32, _reserved: [0; 4] })
                .unwrap();
        }
        assert_eq!(sched.len(), SCHEDULE_CAP);
        let refused = sched.arm(ScheduledEvent { fire_tick: 999, tag: 999, _reserved: [0; 4] });
        assert_eq!(refused, Err(ScheduleError::CapacityExceeded));
        assert_eq!(sched.len(), SCHEDULE_CAP, "refused arm must not mutate the schedule");
    }

    #[test]
    fn drain_of_full_schedule_fits_out_array_exactly() {
        let mut sched = TickSchedule::new();
        for i in 0..SCHEDULE_CAP as u64 {
            sched
                .arm(ScheduledEvent { fire_tick: i, tag: i as u32, _reserved: [0; 4] })
                .unwrap();
        }
        let mut out = [ScheduledEvent::EMPTY; SCHEDULE_CAP];
        let n = sched.drain_due(u64::MAX, &mut out);
        assert_eq!(n, SCHEDULE_CAP);
        assert!(sched.is_empty());
        let mut tags: Vec<u32> = out.iter().map(|e| e.tag).collect();
        tags.sort_unstable();
        assert_eq!(tags, (0..SCHEDULE_CAP as u32).collect::<Vec<u32>>(), "no event lost or duplicated");
    }
}
