//! Timeline-anchored Catmull-Rom for bone keyframes.
//!
//! Author the motion on a **timeline first** (keyframes at ticks), *before* the
//! wireframe lands — the mesh then rides the timeline at the play tick. You keep
//! control: place / move / drop keyframes. Two snaps to the timeline:
//!
//! 1. **Authoring snap** — a keyframe's tick quantizes to the grid (beat / frame)
//!    on insert (`set_key` → `snap_tick`).
//! 2. **Curve snap** — Catmull-Rom *interpolates*, so the sampled curve passes
//!    exactly through every keyframe (which sit on the grid). The flow between
//!    them stays smooth (C1), but it never drifts off a timeline knot.
//!
//! Grid-agnostic: the host supplies `grid` (e.g. `forge-hal` ticks-per-beat), so
//! this stays decoupled from the clock. Integer `MilliUnit`; cold authoring path.

use crate::bone_spline::{catmull_rom_point, SPLINE_T};
use forge_core_v3::fixed_point::MilliUnit;

/// One keyframe: a bone position anchored at a timeline tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyframe {
    pub tick: u32,
    pub pos: [MilliUnit; 3],
}

/// Snap a tick to the nearest grid line (`grid` = ticks per beat/frame).
/// `grid == 0` means free (no snap).
#[inline]
pub fn snap_tick(tick: u32, grid: u32) -> u32 {
    if grid == 0 {
        return tick;
    }
    let g = grid as u64;
    let t = tick as u64;
    (((t + g / 2) / g) * g) as u32
}

/// A Catmull-Rom keyframe track for one bone, anchored to a timeline grid.
/// Keyframes are kept sorted by tick.
#[derive(Clone, Debug, Default)]
pub struct BoneTimeline {
    keys: Vec<Keyframe>,
    /// Snap grid in ticks (0 = free). Public so the host can retune it.
    pub grid: u32,
}

impl BoneTimeline {
    pub fn new(grid: u32) -> Self {
        Self { keys: Vec::new(), grid }
    }

    pub fn keys(&self) -> &[Keyframe] {
        &self.keys
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Place (or replace) a keyframe. Its tick **snaps to the grid**; if a key
    /// already sits on that snapped tick it is overwritten. Returns the snapped
    /// tick. This is the author's control surface.
    pub fn set_key(&mut self, tick: u32, pos: [MilliUnit; 3]) -> u32 {
        let st = snap_tick(tick, self.grid);
        match self.keys.binary_search_by_key(&st, |k| k.tick) {
            Ok(i) => self.keys[i].pos = pos,
            Err(i) => self.keys.insert(i, Keyframe { tick: st, pos }),
        }
        st
    }

    /// Remove the keyframe on the snapped tick, if any. Returns whether one went.
    pub fn remove_key(&mut self, tick: u32) -> bool {
        let st = snap_tick(tick, self.grid);
        match self.keys.binary_search_by_key(&st, |k| k.tick) {
            Ok(i) => {
                self.keys.remove(i);
                true
            }
            Err(_) => false,
        }
    }

    /// Sample the bone position at `tick` — Catmull-Rom across the surrounding
    /// keyframes, clamped at the ends. Lands exactly on a keyframe's value when
    /// `tick` is on that keyframe (snap), smooth in between.
    pub fn sample(&self, tick: u32) -> Option<[MilliUnit; 3]> {
        let n = self.keys.len();
        if n == 0 {
            return None;
        }
        if n == 1 || tick <= self.keys[0].tick {
            return Some(self.keys[0].pos);
        }
        if tick >= self.keys[n - 1].tick {
            return Some(self.keys[n - 1].pos);
        }

        let i = match self.keys.binary_search_by_key(&tick, |k| k.tick) {
            Ok(idx) => return Some(self.keys[idx].pos), // exactly on a knot
            Err(idx) => idx,
        };
        // keys[i-1].tick < tick < keys[i].tick → segment starts at s = i-1.
        let s = i - 1;
        let p1 = self.keys[s].pos;
        let p2 = self.keys[s + 1].pos;
        let p0 = self.keys[s.saturating_sub(1)].pos;
        let p3 = self.keys[(s + 2).min(n - 1)].pos;

        let span = (self.keys[s + 1].tick - self.keys[s].tick).max(1) as u64;
        let into = (tick - self.keys[s].tick) as u64;
        let tp = ((into * SPLINE_T as u64) / span) as i32;
        Some(catmull_rom_point(p0, p1, p2, p3, tp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mu(x: i64, y: i64, z: i64) -> [MilliUnit; 3] {
        [MilliUnit(x), MilliUnit(y), MilliUnit(z)]
    }

    #[test]
    fn snaps_to_grid() {
        assert_eq!(snap_tick(7, 4), 8);
        assert_eq!(snap_tick(5, 4), 4);
        assert_eq!(snap_tick(6, 4), 8); // round half up
        assert_eq!(snap_tick(10, 0), 10); // free
    }

    #[test]
    fn set_key_snaps_sorts_and_replaces() {
        let mut t = BoneTimeline::new(10);
        assert_eq!(t.set_key(13, mu(0, 0, 0)), 10); // 13 → 10
        assert_eq!(t.set_key(28, mu(100, 0, 0)), 30); // 28 → 30
        assert_eq!(t.len(), 2);
        assert_eq!(t.keys()[0].tick, 10);
        assert_eq!(t.keys()[1].tick, 30);
        // 11 also snaps to 10 → replaces, not a new key
        t.set_key(11, mu(50, 0, 0));
        assert_eq!(t.len(), 2);
        assert_eq!(t.keys()[0].pos, mu(50, 0, 0));
    }

    #[test]
    fn sample_lands_on_every_keyframe() {
        let mut t = BoneTimeline::new(10);
        t.set_key(0, mu(0, 0, 0));
        t.set_key(10, mu(100, 200, 0));
        t.set_key(20, mu(200, 0, 0));
        t.set_key(30, mu(300, 100, 0));
        for k in t.keys() {
            assert_eq!(t.sample(k.tick), Some(k.pos), "curve must snap to keyframe @ {}", k.tick);
        }
    }

    #[test]
    fn sample_midpoint_is_between_two_keys() {
        let mut t = BoneTimeline::new(0);
        t.set_key(0, mu(0, 0, 0));
        t.set_key(10, mu(100, 100, 0));
        // 2-key Catmull-Rom (clamped) → exact midpoint at the middle tick.
        assert_eq!(t.sample(5), Some(mu(50, 50, 0)));
    }

    #[test]
    fn sample_clamps_outside_range() {
        let mut t = BoneTimeline::new(0);
        t.set_key(10, mu(7, 0, 0));
        t.set_key(20, mu(9, 0, 0));
        assert_eq!(t.sample(0), Some(mu(7, 0, 0))); // before first
        assert_eq!(t.sample(999), Some(mu(9, 0, 0))); // after last
    }

    #[test]
    fn empty_samples_none() {
        let t = BoneTimeline::new(4);
        assert_eq!(t.sample(0), None);
    }

    #[test]
    fn remove_and_deterministic() {
        let mut t = BoneTimeline::new(8);
        t.set_key(8, mu(1, 0, 0));
        t.set_key(16, mu(2, 0, 0));
        assert!(t.remove_key(15)); // 15 → 16
        assert_eq!(t.len(), 1);
        assert!(!t.remove_key(100));
        // deterministic sampling
        let mut a = BoneTimeline::new(0);
        for (k, v) in [(0, 0), (10, 100), (20, 50), (30, 80)] {
            a.set_key(k, mu(v, 0, 0));
        }
        assert_eq!(a.sample(13), a.sample(13));
    }
}
