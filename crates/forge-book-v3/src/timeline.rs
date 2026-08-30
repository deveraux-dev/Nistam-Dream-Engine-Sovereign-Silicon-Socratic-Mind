//! Timeline — tick-driven keyframes for the fold / page-turn animation. Integer
//! ticks, integer lerp; deterministic sampling with no float.

use serde::{Deserialize, Serialize};

/// One keyframe: a permyriad value at an integer tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyframe {
    /// Tick at which this keyframe occurs.
    pub tick: u32,
    /// Value in permyriad units (0–10000 range).
    pub value_pmy: u32,
}

/// An ordered set of keyframes, sampled by integer lerp.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    /// Keyframes sorted by tick; maintained via the `key()` method.
    pub frames: Vec<Keyframe>,
}

impl Timeline {
    /// Create a new empty timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a keyframe at `tick`, keeping frames tick-sorted.
    pub fn key(&mut self, tick: u32, value_pmy: u32) -> &mut Self {
        let kf = Keyframe { tick, value_pmy: value_pmy.min(10_000) };
        match self.frames.binary_search_by_key(&tick, |f| f.tick) {
            Ok(i) => self.frames[i] = kf,
            Err(i) => self.frames.insert(i, kf),
        }
        self
    }

    /// Duration of the timeline: tick of the last keyframe, or 0 if empty.
    pub fn duration(&self) -> u32 {
        self.frames.last().map(|f| f.tick).unwrap_or(0)
    }

    /// Sample the value at `tick` — clamped before the first / after the last
    /// key, integer-lerped between the surrounding keys.
    pub fn sample(&self, tick: u32) -> u32 {
        if self.frames.is_empty() {
            return 0;
        }
        if tick <= self.frames[0].tick {
            return self.frames[0].value_pmy;
        }
        let last = self.frames[self.frames.len() - 1];
        if tick >= last.tick {
            return last.value_pmy;
        }
        // find the bracketing pair
        let hi = self.frames.iter().position(|f| f.tick >= tick).unwrap();
        let a = self.frames[hi - 1];
        let b = self.frames[hi];
        let span = (b.tick - a.tick) as u64;
        let into = (tick - a.tick) as u64;
        let lo = a.value_pmy as i64;
        let delta = b.value_pmy as i64 - lo;
        (lo + delta * into as i64 / span as i64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> Timeline {
        let mut t = Timeline::new();
        t.key(0, 0).key(100, 10_000);
        t
    }

    #[test]
    fn samples_endpoints_and_midpoint() {
        let t = ramp();
        assert_eq!(t.sample(0), 0);
        assert_eq!(t.sample(100), 10_000);
        assert_eq!(t.sample(50), 5_000);
        assert_eq!(t.duration(), 100);
    }

    #[test]
    fn clamps_outside_range() {
        let t = ramp();
        assert_eq!(t.sample(999), 10_000);
    }

    #[test]
    fn key_replaces_in_place() {
        let mut t = Timeline::new();
        t.key(10, 3000).key(10, 7000);
        assert_eq!(t.frames.len(), 1);
        assert_eq!(t.sample(10), 7000);
    }

    #[test]
    fn empty_samples_zero() {
        assert_eq!(Timeline::new().sample(5), 0);
    }
}
