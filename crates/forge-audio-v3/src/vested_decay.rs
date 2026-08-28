//! VestedDecay: a bounded, self-pruning memory that models graceful forgetting —
//! strength VESTS on reinforcement and DECAYS on a half-life curve; the store
//! never outgrows its capacity nor outlives its TTL. Pure integer permyriad, no
//! float, no disk — a reasonable amount, and then it forgets.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-core\src\vested_decay.rs` (v2
//! Crate Zero). Lands here, not in `forge-core-v3`, because v3's Crate Zero is a
//! strict zero-dependency floor (bytemuck the one blessed exception) and this
//! module's only real consumer anywhere in `F:\v3` is `forge-audio-v3::bus::panels`
//! — confirmed by grep before landing, not assumed.

/// Full strength, permyriad.
pub const STRENGTH_MAX: u32 = 10_000;

#[derive(Clone, Copy, Debug)]
struct Entry<K> {
    key: K,
    last: u64,     // tick of last reinforcement — decay measures from here
    strength: u32, // permyriad AT `last`
}

/// A keyed forgetting store. `K` is a small copyable tag (event kind, cell id…).
/// Reads always decay live off `now`; `prune` is what bounds *storage*.
#[derive(Clone, Debug)]
pub struct VestedDecay<K: Copy + PartialEq> {
    entries: Vec<Entry<K>>,
    capacity: usize,
    half_life: u64, // ticks for strength to halve
    ttl: u64,       // hard age cap — older than this is forgotten outright
    floor: u32,     // permyriad below which an entry is forgotten
}

impl<K: Copy + PartialEq> VestedDecay<K> {
    pub fn new(capacity: usize, half_life: u64, ttl: u64, floor: u32) -> Self {
        Self {
            entries: Vec::new(),
            capacity: capacity.max(1),
            half_life: half_life.max(1),
            ttl: ttl.max(1),
            floor: floor.min(STRENGTH_MAX),
        }
    }

    /// Half-life decay of `s` over `dt` ticks — deterministic, integer, monotone
    /// non-increasing: `full` exact halvings, then a linear tail across the band.
    fn decay(s: u32, dt: u64, half_life: u64) -> u32 {
        if s == 0 {
            return 0;
        }
        let hl = half_life.max(1);
        let full = (dt / hl).min(20) as u32; // >20 halvings ⇒ forgotten
        let mut v = s >> full;
        let rem = dt % hl;
        if rem > 0 && v > 0 {
            let drop = ((v as u64 - (v as u64 >> 1)) * rem / hl) as u32;
            v = v.saturating_sub(drop);
        }
        v
    }

    /// Reinforce (or first-see) `key` at `now` with `gain` permyriad, then prune.
    /// Vesting: the prior strength decays to `now`, then `gain` adds (saturating).
    pub fn observe(&mut self, key: K, now: u64, gain: u32) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.key == key) {
            let cur = Self::decay(e.strength, now.saturating_sub(e.last), self.half_life);
            e.strength = cur.saturating_add(gain).min(STRENGTH_MAX);
            e.last = now;
        } else {
            self.entries.push(Entry {
                key,
                last: now,
                strength: gain.min(STRENGTH_MAX),
            });
        }
        self.prune(now);
    }

    /// Drop the forgotten (past TTL or below floor at `now`), then evict the
    /// weakest until within capacity. The store never bloats into a dump.
    pub fn prune(&mut self, now: u64) {
        let (hl, ttl, floor) = (self.half_life, self.ttl, self.floor);
        self.entries.retain(|e| {
            let dt = now.saturating_sub(e.last);
            dt < ttl && Self::decay(e.strength, dt, hl) >= floor
        });
        if self.entries.len() > self.capacity {
            self.entries
                .sort_by_key(|e| Self::decay(e.strength, now.saturating_sub(e.last), hl));
            let overflow = self.entries.len() - self.capacity;
            self.entries.drain(0..overflow);
        }
    }

    /// Decayed strength of `key` at `now` (0 if forgotten/absent).
    pub fn strength(&self, key: K, now: u64) -> u32 {
        self.entries
            .iter()
            .find(|e| e.key == key)
            .map(|e| Self::decay(e.strength, now.saturating_sub(e.last), self.half_life))
            .unwrap_or(0)
    }

    /// Live entries at `now` — still above the floor and within TTL.
    pub fn live(&self, now: u64) -> usize {
        let (hl, ttl, floor) = (self.half_life, self.ttl, self.floor);
        self.entries
            .iter()
            .filter(|e| {
                let dt = now.saturating_sub(e.last);
                dt < ttl && Self::decay(e.strength, dt, hl) >= floor
            })
            .count()
    }

    /// Summed decayed strength at `now`, saturating — the whole memory's weight.
    pub fn total(&self, now: u64) -> u32 {
        let hl = self.half_life;
        self.entries.iter().fold(0u32, |a, e| {
            a.saturating_add(Self::decay(e.strength, now.saturating_sub(e.last), hl))
        })
    }

    /// Stored-entry count (storage size, NOT the live/forgotten view).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_halves_over_one_half_life() {
        assert_eq!(VestedDecay::<u16>::decay(STRENGTH_MAX, 0, 100), STRENGTH_MAX);
        assert_eq!(VestedDecay::<u16>::decay(STRENGTH_MAX, 100, 100), 5_000);
        assert_eq!(VestedDecay::<u16>::decay(STRENGTH_MAX, 200, 100), 2_500);
        assert_eq!(VestedDecay::<u16>::decay(STRENGTH_MAX, 100_000, 100), 0);
    }

    #[test]
    fn decay_is_monotone_non_increasing() {
        let mut prev = STRENGTH_MAX + 1;
        for dt in 0..=1_000u64 {
            let v = VestedDecay::<u16>::decay(STRENGTH_MAX, dt, 128);
            assert!(v <= prev, "decay must never rise: dt={dt} v={v} prev={prev}");
            prev = v;
        }
    }

    #[test]
    fn observe_vests_and_reads_decayed() {
        let mut m = VestedDecay::<u16>::new(8, 100, 10_000, 100);
        m.observe(1, 0, 6_000);
        assert_eq!(m.strength(1, 0), 6_000);
        assert_eq!(m.strength(1, 100), 3_000); // one half-life
        m.observe(1, 100, 6_000); // vests on the decayed remainder
        assert_eq!(m.strength(1, 100), 9_000); // 3_000 + 6_000
    }

    #[test]
    fn ttl_forgets_and_prune_drops_storage() {
        let mut m = VestedDecay::<u16>::new(8, 50, 200, 1);
        m.observe(7, 0, STRENGTH_MAX);
        assert_eq!(m.live(0), 1);
        m.prune(500); // past TTL
        assert_eq!(m.len(), 0, "TTL prunes storage");
        assert_eq!(m.strength(7, 500), 0);
    }

    #[test]
    fn floor_forgets_weak_entries() {
        let mut m = VestedDecay::<u16>::new(8, 100, 100_000, 500);
        m.observe(3, 0, 1_000);
        m.prune(200); // 1_000 -> 250 < 500 floor
        assert_eq!(m.len(), 0, "sub-floor strength is forgotten");
    }

    #[test]
    fn capacity_caps_storage_evicting_weakest() {
        let mut m = VestedDecay::<u16>::new(3, 1_000, 100_000, 1);
        for (k, g) in [(1u16, 1_000u32), (2, 2_000), (3, 3_000), (4, 4_000), (5, 5_000)] {
            m.observe(k, 0, g);
        }
        assert!(m.len() <= 3, "never a memory dump: capped at capacity");
        assert_eq!(m.strength(1, 0), 0, "weakest forgotten");
        assert_eq!(m.strength(2, 0), 0, "next-weakest forgotten");
        assert_eq!(m.strength(5, 0), 5_000, "strongest survives");
    }
}
