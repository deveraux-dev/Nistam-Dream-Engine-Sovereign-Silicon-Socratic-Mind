//! Easing — integer easing curves (permyriad t -> permyriad value). Deterministic
//! shaping for the fold / page-turn animation; no float.

use serde::{Deserialize, Serialize};

/// An easing curve over `t` in `0..=10000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ease {
    /// Linear interpolation; no acceleration or deceleration.
    Linear,
    /// Quadratic ease-in; slow start, fast finish.
    InQuad,
    /// Quadratic ease-out; fast start, slow finish.
    OutQuad,
    /// Quadratic ease-in-out; slow start and finish, fast middle.
    InOutQuad,
    /// Cubic ease-in; slow start, fast finish.
    InCubic,
    /// Cubic ease-out; fast start, slow finish.
    OutCubic,
}

impl Ease {
    /// Ease `t` (permyriad) to an output (permyriad).
    pub fn at(&self, t: u32) -> u32 {
        let t = t.min(10_000) as u64;
        let out = match self {
            Ease::Linear => t,
            Ease::InQuad => t * t / 10_000,
            Ease::OutQuad => {
                let u = 10_000 - t;
                10_000 - u * u / 10_000
            }
            Ease::InOutQuad => {
                if t < 5_000 {
                    2 * t * t / 10_000
                } else {
                    let u = 10_000 - t;
                    10_000 - 2 * u * u / 10_000
                }
            }
            Ease::InCubic => t * t / 10_000 * t / 10_000,
            Ease::OutCubic => {
                let u = 10_000 - t;
                10_000 - u * u / 10_000 * u / 10_000
            }
        };
        out.min(10_000) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed() {
        for e in [Ease::Linear, Ease::InQuad, Ease::OutQuad, Ease::InOutQuad, Ease::InCubic, Ease::OutCubic] {
            assert_eq!(e.at(0), 0, "{e:?} at 0");
            assert_eq!(e.at(10_000), 10_000, "{e:?} at 1");
        }
    }

    #[test]
    fn quad_curves_below_and_above_linear() {
        assert!(Ease::InQuad.at(5_000) < 5_000); // slow start
        assert!(Ease::OutQuad.at(5_000) > 5_000); // fast start
        assert_eq!(Ease::Linear.at(5_000), 5_000);
    }

    #[test]
    fn output_never_exceeds_range() {
        for t in (0..=10_000).step_by(137) {
            for e in [Ease::InCubic, Ease::OutCubic, Ease::InOutQuad] {
                assert!(e.at(t) <= 10_000);
            }
        }
    }
}
