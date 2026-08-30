//! Bezier — an integer cubic Bézier over permyriad t (De Casteljau with integer
//! lerp). Animation paths for the fold / page-turn without float.

use serde::{Deserialize, Serialize};

/// A 1-D cubic Bézier with four control values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cubic {
    /// First control point.
    pub p0: i32,
    /// Second control point.
    pub p1: i32,
    /// Third control point.
    pub p2: i32,
    /// Fourth control point.
    pub p3: i32,
}

fn lerp(a: i32, b: i32, t: i32) -> i32 {
    a + (b - a) * t / 10_000
}

impl Cubic {
    /// Construct a new cubic Bézier from four control points.
    pub fn new(p0: i32, p1: i32, p2: i32, p3: i32) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Sample at `t_pmy` (0..=10000) via De Casteljau.
    pub fn at(&self, t_pmy: u32) -> i32 {
        let t = t_pmy.min(10_000) as i32;
        let a = lerp(self.p0, self.p1, t);
        let b = lerp(self.p1, self.p2, t);
        let c = lerp(self.p2, self.p3, t);
        let d = lerp(a, b, t);
        let e = lerp(b, c, t);
        lerp(d, e, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        let c = Cubic::new(0, 3000, 7000, 10_000);
        assert_eq!(c.at(0), 0);
        assert_eq!(c.at(10_000), 10_000);
    }

    #[test]
    fn ease_curve_is_monotonic() {
        let c = Cubic::new(0, 0, 10_000, 10_000); // ease-in-out
        let mut last = -1;
        for t in (0..=10_000).step_by(500) {
            let v = c.at(t);
            assert!(v >= last, "monotonic at {t}");
            last = v;
        }
    }
}
