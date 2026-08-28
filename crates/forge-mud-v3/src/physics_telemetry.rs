//! Physics query telemetry — a debug-only, lock-free observability ring for
//! `World5D::ground_y`/`body_blocked`, mirroring `forge-audio-v3::telemetry`'s
//! proven pattern (const-init atomics, small ring buffer, no locks, no
//! allocation) but as its own type: `forge-mud-v3` must not depend on
//! `forge-audio-v3` (that dependency would run backwards — audio doesn't own
//! physics).
//!
//! Landed 2026-08-16 to close a real debugging gap found live tonight: these
//! two functions are the actual ground-truth physics queries this session
//! spent hours diagnosing by hand (screenshots + manual log reading) — a
//! walker falling through terrain that rendered as solid. With this, "what
//! did physics actually find here" is a live, greppable `eprintln!` instead
//! of an inference from a screenshot.

use core::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicU8, Ordering};

const RING_CAP: usize = 16;

/// Lock-free physics query telemetry. `Relaxed` throughout — this is a
/// debug instrument, not a synchronization primitive; approximate ordering
/// across ticks is acceptable (mirrors `forge-audio-v3::telemetry`'s own
/// `Relaxed` choice for the same reason).
pub struct PhysicsTelemetry {
    ground_y_queries: AtomicU64,
    ground_y_misses: AtomicU64,
    body_blocked_queries: AtomicU64,
    body_blocked_hits: AtomicU64,

    miss_ring_tick: [AtomicU64; RING_CAP],
    miss_ring_x: [AtomicI64; RING_CAP],
    miss_ring_z: [AtomicI64; RING_CAP],
    miss_ring_cursor: AtomicU32,
    miss_ring_len: AtomicU8,

    hit_ring_tick: [AtomicU64; RING_CAP],
    hit_ring_x: [AtomicI64; RING_CAP],
    hit_ring_y: [AtomicI64; RING_CAP],
    hit_ring_z: [AtomicI64; RING_CAP],
    hit_ring_cursor: AtomicU32,
    hit_ring_len: AtomicU8,
}

// Repeat-array const-init helper — `[AtomicU64::new(0); RING_CAP]` needs
// `Copy` on the array init expression, which atomics don't have; write it
// out with a macro instead of unsafe transmute tricks.
macro_rules! atomic_ring {
    ($ty:ty, $cap:expr) => {{
        const Z: $ty = <$ty>::new(0);
        [Z; $cap]
    }};
}

impl PhysicsTelemetry {
    /// A fresh telemetry instance, all counters and ring slots zeroed.
    pub const fn new() -> Self {
        Self {
            ground_y_queries: AtomicU64::new(0),
            ground_y_misses: AtomicU64::new(0),
            body_blocked_queries: AtomicU64::new(0),
            body_blocked_hits: AtomicU64::new(0),

            miss_ring_tick: atomic_ring!(AtomicU64, RING_CAP),
            miss_ring_x: atomic_ring!(AtomicI64, RING_CAP),
            miss_ring_z: atomic_ring!(AtomicI64, RING_CAP),
            miss_ring_cursor: AtomicU32::new(0),
            miss_ring_len: AtomicU8::new(0),

            hit_ring_tick: atomic_ring!(AtomicU64, RING_CAP),
            hit_ring_x: atomic_ring!(AtomicI64, RING_CAP),
            hit_ring_y: atomic_ring!(AtomicI64, RING_CAP),
            hit_ring_z: atomic_ring!(AtomicI64, RING_CAP),
            hit_ring_cursor: AtomicU32::new(0),
            hit_ring_len: AtomicU8::new(0),
        }
    }

    /// Record one `ground_y` call. `eprintln!`s on a miss (no solid cell
    /// found in the column) — the exact event that used to require a
    /// screenshot to notice.
    pub fn record_ground_y(&self, x: i64, z: i64, tick: u64, result: Option<i64>) {
        self.ground_y_queries.fetch_add(1, Ordering::Relaxed);
        if result.is_none() {
            self.ground_y_misses.fetch_add(1, Ordering::Relaxed);
            // Print each missing COLUMN once, not once per frame — the same
            // void columns re-missing every tick drowned the console (Sean
            // 2026-08-17). The ring + counters still record every miss.
            let len = (self.miss_ring_len.load(Ordering::Relaxed) as usize).min(RING_CAP);
            let already_seen = (0..len).any(|i| {
                self.miss_ring_x[i].load(Ordering::Relaxed) == x
                    && self.miss_ring_z[i].load(Ordering::Relaxed) == z
            });
            let slot = self.miss_ring_cursor.fetch_add(1, Ordering::Relaxed) as usize % RING_CAP;
            self.miss_ring_tick[slot].store(tick, Ordering::Relaxed);
            self.miss_ring_x[slot].store(x, Ordering::Relaxed);
            self.miss_ring_z[slot].store(z, Ordering::Relaxed);
            if (self.miss_ring_len.load(Ordering::Relaxed) as usize) < RING_CAP {
                self.miss_ring_len.fetch_add(1, Ordering::Relaxed);
            }
            if !already_seen {
                eprintln!("physics: ground_y miss at ({x}, {z}), tick {tick}");
            }
        }
    }

    /// Record one `body_blocked` call. `eprintln!`s on a hit (real
    /// collision) — the walker's position at the moment physics says
    /// "solid here".
    pub fn record_body_blocked(&self, x: i64, y: i64, z: i64, tick: u64, hit: bool) {
        self.body_blocked_queries.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.body_blocked_hits.fetch_add(1, Ordering::Relaxed);
            let slot = self.hit_ring_cursor.fetch_add(1, Ordering::Relaxed) as usize % RING_CAP;
            self.hit_ring_tick[slot].store(tick, Ordering::Relaxed);
            self.hit_ring_x[slot].store(x, Ordering::Relaxed);
            self.hit_ring_y[slot].store(y, Ordering::Relaxed);
            self.hit_ring_z[slot].store(z, Ordering::Relaxed);
            if (self.hit_ring_len.load(Ordering::Relaxed) as usize) < RING_CAP {
                self.hit_ring_len.fetch_add(1, Ordering::Relaxed);
            }
            // No `eprintln!` here (unlike `record_ground_y`'s miss case):
            // `body_blocked` returns `true` on every tick the walker is
            // simply standing on normal ground (~120/sec) — printing every
            // hit would be log spam, not signal. The ring still records it
            // silently for later inspection via `hit_ring_at`.
        }
    }

    /// Total `ground_y` calls recorded (queries + misses combined count).
    pub fn ground_y_query_count(&self) -> u64 {
        self.ground_y_queries.load(Ordering::Relaxed)
    }

    /// Total `ground_y` misses recorded.
    pub fn ground_y_miss_count(&self) -> u64 {
        self.ground_y_misses.load(Ordering::Relaxed)
    }

    /// Total `body_blocked` calls recorded.
    pub fn body_blocked_query_count(&self) -> u64 {
        self.body_blocked_queries.load(Ordering::Relaxed)
    }

    /// Total `body_blocked` hits recorded.
    pub fn body_blocked_hit_count(&self) -> u64 {
        self.body_blocked_hits.load(Ordering::Relaxed)
    }

    /// How many miss-ring slots are populated (`0..=RING_CAP`).
    pub fn miss_ring_len(&self) -> usize {
        self.miss_ring_len.load(Ordering::Relaxed) as usize
    }

    /// How many hit-ring slots are populated (`0..=RING_CAP`).
    pub fn hit_ring_len(&self) -> usize {
        self.hit_ring_len.load(Ordering::Relaxed) as usize
    }

    /// Read miss-ring slot `i` (`i < miss_ring_len()`): `(tick, x, z)`.
    pub fn miss_ring_at(&self, i: usize) -> (u64, i64, i64) {
        (
            self.miss_ring_tick[i].load(Ordering::Relaxed),
            self.miss_ring_x[i].load(Ordering::Relaxed),
            self.miss_ring_z[i].load(Ordering::Relaxed),
        )
    }

    /// Read hit-ring slot `i` (`i < hit_ring_len()`): `(tick, x, y, z)`.
    pub fn hit_ring_at(&self, i: usize) -> (u64, i64, i64, i64) {
        (
            self.hit_ring_tick[i].load(Ordering::Relaxed),
            self.hit_ring_x[i].load(Ordering::Relaxed),
            self.hit_ring_y[i].load(Ordering::Relaxed),
            self.hit_ring_z[i].load(Ordering::Relaxed),
        )
    }
}

impl Default for PhysicsTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

static TELEMETRY: PhysicsTelemetry = PhysicsTelemetry::new();

/// The process-wide physics telemetry singleton.
pub fn telemetry() -> &'static PhysicsTelemetry {
    &TELEMETRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_wraps_past_capacity_without_panicking() {
        let t = PhysicsTelemetry::new();
        for i in 0..(RING_CAP * 3) as i64 {
            t.record_ground_y(i, i, i as u64, None);
        }
        assert_eq!(t.miss_ring_len(), RING_CAP, "ring length caps at RING_CAP, never exceeds it");
        assert_eq!(t.ground_y_miss_count(), (RING_CAP * 3) as u64, "the running counter is uncapped");
    }

    #[test]
    fn ground_y_hit_does_not_populate_the_miss_ring() {
        let t = PhysicsTelemetry::new();
        t.record_ground_y(0, 0, 0, Some(5));
        assert_eq!(t.ground_y_query_count(), 1);
        assert_eq!(t.ground_y_miss_count(), 0);
        assert_eq!(t.miss_ring_len(), 0);
    }

    #[test]
    fn body_blocked_hit_populates_the_hit_ring() {
        let t = PhysicsTelemetry::new();
        t.record_body_blocked(10, 20, 30, 7, true);
        assert_eq!(t.body_blocked_hit_count(), 1);
        assert_eq!(t.hit_ring_len(), 1);
        assert_eq!(t.hit_ring_at(0), (7, 10, 20, 30));
    }

    #[test]
    fn body_blocked_miss_does_not_populate_the_hit_ring() {
        let t = PhysicsTelemetry::new();
        t.record_body_blocked(10, 20, 30, 7, false);
        assert_eq!(t.body_blocked_query_count(), 1);
        assert_eq!(t.body_blocked_hit_count(), 0);
        assert_eq!(t.hit_ring_len(), 0);
    }

    #[test]
    fn the_process_wide_singleton_is_reachable() {
        // Just confirm `telemetry()` compiles and returns a stable reference
        // — the real state-mutation behavior is covered by the tests above
        // against a fresh local instance (a shared static would make test
        // order matter, which these tests deliberately avoid).
        let a = telemetry() as *const PhysicsTelemetry;
        let b = telemetry() as *const PhysicsTelemetry;
        assert_eq!(a, b, "telemetry() always returns the same static");
    }
}
