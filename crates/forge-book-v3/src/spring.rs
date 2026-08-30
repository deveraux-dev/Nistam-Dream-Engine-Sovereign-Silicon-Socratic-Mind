//! Spring — an integer smoothing spring: position eases toward a target at a
//! per-mille rate, always making progress (harvested from forge-canvas spring).

use serde::{Deserialize, Serialize};

/// A 1-D integer spring in permyriad space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spring {
    /// Current position in permyriad space.
    pub pos_pmy: i32,
    /// Easing rate as per-mille (1..=1000) per step.
    pub rate: i32,
}

impl Spring {
    /// A spring easing at `rate` per-mille (1..=1000) per step, resting at 0.
    pub fn new(rate: i32) -> Self {
        Self { pos_pmy: 0, rate: rate.clamp(1, 1000) }
    }

    /// Set position to an absolute value.
    pub fn set(&mut self, pos_pmy: i32) {
        self.pos_pmy = pos_pmy;
    }

    /// Step one tick toward `target`; always moves at least 1 until settled.
    pub fn step(&mut self, target: i32) -> i32 {
        let dx = target - self.pos_pmy;
        if dx == 0 {
            return self.pos_pmy;
        }
        let mut d = dx * self.rate / 1000;
        if d == 0 {
            d = dx.signum();
        }
        self.pos_pmy += d;
        self.pos_pmy
    }

    /// Returns true if position has reached the target.
    pub fn settled(&self, target: i32) -> bool {
        self.pos_pmy == target
    }

    /// Run to rest, returning the step count (bounded).
    pub fn settle(&mut self, target: i32) -> u32 {
        let mut n = 0;
        while !self.settled(target) {
            self.step(target);
            n += 1;
            if n > 1_000_000 {
                break;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_toward_target() {
        let mut s = Spring::new(200);
        let first = s.step(10_000);
        assert!(first > 0 && first < 10_000); // partial move
        s.settle(10_000);
        assert!(s.settled(10_000));
    }

    #[test]
    fn always_makes_progress_near_target() {
        let mut s = Spring::new(1); // tiny rate
        s.set(9_999);
        s.step(10_000); // rounding would give 0; must still move 1
        assert_eq!(s.pos_pmy, 10_000);
    }

    #[test]
    fn deterministic_settle_count() {
        let mut a = Spring::new(150);
        let mut b = Spring::new(150);
        assert_eq!(a.settle(10_000), b.settle(10_000));
    }
}
