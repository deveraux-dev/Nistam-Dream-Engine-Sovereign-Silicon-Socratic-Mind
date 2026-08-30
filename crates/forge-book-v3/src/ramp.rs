//! Ramp — a discrete N-step colour ramp sampled evenly from a Gradient.

use crate::colour::Oklch;
use crate::gradient::Gradient;

/// Sample `g` at `steps` evenly spaced positions (endpoints inclusive).
pub fn ramp(g: &Gradient, steps: usize) -> Vec<Oklch> {
    if steps == 0 {
        return Vec::new();
    }
    (0..steps)
        .filter_map(|i| {
            let t = if steps == 1 { 0 } else { (i as u32 * 10_000) / (steps as u32 - 1) };
            g.sample(t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad() -> Gradient {
        let mut g = Gradient::new();
        g.stop(0, Oklch::new(0, 0, 0)).stop(10_000, Oklch::new(10_000, 0, 0));
        g
    }

    #[test]
    fn ramp_spans_endpoints() {
        let r = ramp(&grad(), 5);
        assert_eq!(r.len(), 5);
        assert_eq!(r[0].l_pmy, 0);
        assert_eq!(r[4].l_pmy, 10_000);
        assert_eq!(r[2].l_pmy, 5_000);
    }

    #[test]
    fn zero_steps_empty() {
        assert!(ramp(&grad(), 0).is_empty());
    }
}
