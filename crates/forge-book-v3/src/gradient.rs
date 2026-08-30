//! Gradient — a multi-stop Oklch gradient sampled at permyriad t. Integer lerp
//! per channel; the page's colour wash.

use crate::colour::Oklch;
use serde::{Deserialize, Serialize};

/// One colour stop at position `t_pmy` (`0..=10000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stop {
    /// Position along the gradient, measured in perymyriad (0 to 10000).
    pub t_pmy: u32,
    /// The colour at this position in Oklch colour space.
    pub colour: Oklch,
}

/// An ordered multi-stop gradient.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gradient {
    /// Ordered collection of colour stops by position, maintained sorted.
    pub stops: Vec<Stop>,
}

fn lerp(a: u32, b: u32, num: u32, den: u32) -> u32 {
    let a = a as i64;
    let b = b as i64;
    (a + (b - a) * num as i64 / den.max(1) as i64) as u32
}

impl Gradient {
    /// Create a new empty gradient with no stops.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a stop, keeping stops sorted by position.
    pub fn stop(&mut self, t_pmy: u32, colour: Oklch) -> &mut Self {
        let s = Stop { t_pmy: t_pmy.min(10_000), colour };
        match self.stops.binary_search_by_key(&s.t_pmy, |x| x.t_pmy) {
            Ok(i) => self.stops[i] = s,
            Err(i) => self.stops.insert(i, s),
        }
        self
    }

    /// Sample the gradient at `t` — clamped before first / after last stop.
    pub fn sample(&self, t: u32) -> Option<Oklch> {
        if self.stops.is_empty() {
            return None;
        }
        let first = self.stops[0];
        if t <= first.t_pmy {
            return Some(first.colour);
        }
        let last = self.stops[self.stops.len() - 1];
        if t >= last.t_pmy {
            return Some(last.colour);
        }
        let hi = self.stops.iter().position(|s| s.t_pmy >= t).unwrap();
        let a = self.stops[hi - 1];
        let b = self.stops[hi];
        let num = t - a.t_pmy;
        let den = b.t_pmy - a.t_pmy;
        Some(Oklch {
            l_pmy: lerp(a.colour.l_pmy, b.colour.l_pmy, num, den),
            c_pmy: lerp(a.colour.c_pmy, b.colour.c_pmy, num, den),
            h_deg: lerp(a.colour.h_deg, b.colour.h_deg, num, den),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad() -> Gradient {
        let mut g = Gradient::new();
        g.stop(0, Oklch::new(0, 0, 0)).stop(10_000, Oklch::new(10_000, 4000, 200));
        g
    }

    #[test]
    fn samples_endpoints_and_midpoint() {
        let g = grad();
        assert_eq!(g.sample(0).unwrap().l_pmy, 0);
        assert_eq!(g.sample(10_000).unwrap().l_pmy, 10_000);
        assert_eq!(g.sample(5_000).unwrap().l_pmy, 5_000);
        assert_eq!(g.sample(5_000).unwrap().h_deg, 100);
    }

    #[test]
    fn empty_gradient_samples_none() {
        assert!(Gradient::new().sample(5_000).is_none());
    }

    #[test]
    fn clamps_beyond_range() {
        assert_eq!(grad().sample(99_999).unwrap().l_pmy, 10_000);
    }
}
