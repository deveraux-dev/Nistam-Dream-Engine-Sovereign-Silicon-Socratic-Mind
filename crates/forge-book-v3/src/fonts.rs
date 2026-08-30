//! Fonts — the type-ramp section (font sandbox). A ramp is N integer sizes; the
//! author picks a rung, never a raw pixel. MilliUnit throughout.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// A quantized type ramp — ascending MilliUnit sizes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeRamp {
    /// Sorted vector of type sizes in MilliUnits.
    pub sizes_mu: Vec<i64>,
}

impl TypeRamp {
    /// Creates a new type ramp from a vector of MilliUnit sizes, automatically sorted.
    pub fn new(sizes_mu: Vec<i64>) -> Self {
        let mut s = sizes_mu;
        s.sort_unstable();
        Self { sizes_mu: s }
    }

    /// The default six-rung ramp (11..46 MilliUnit-scaled).
    pub fn default_ramp() -> Self {
        Self::new(vec![11_000, 14_000, 18_000, 24_000, 32_000, 46_000])
    }

    /// Returns the number of rungs in this type ramp.
    pub fn len(&self) -> usize {
        self.sizes_mu.len()
    }
    /// Returns `true` if this type ramp has no rungs.
    pub fn is_empty(&self) -> bool {
        self.sizes_mu.is_empty()
    }

    /// The size at rung `n` (clamped to the ramp's ends).
    pub fn rung(&self, n: usize) -> i64 {
        if self.sizes_mu.is_empty() {
            return 0;
        }
        let n = n.min(self.sizes_mu.len() - 1);
        self.sizes_mu[n]
    }

    /// The nearest rung index for a raw MilliUnit size.
    pub fn snap(&self, size_mu: i64) -> usize {
        self.sizes_mu
            .iter()
            .enumerate()
            .min_by_key(|(_, s)| (**s - size_mu).abs())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Converts this type ramp into an atlas chapter documenting all rungs.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Fonts".into()));
        for (i, s) in self.sizes_mu.iter().enumerate() {
            ch.add_lore(format!("ramp[{i}] = {s}mu"));
        }
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ramp_is_sorted_six() {
        let r = TypeRamp::default_ramp();
        assert_eq!(r.len(), 6);
        assert_eq!(r.rung(0), 11_000);
        assert_eq!(r.rung(5), 46_000);
        assert_eq!(r.rung(99), 46_000); // clamps
    }

    #[test]
    fn snap_finds_nearest_rung() {
        let r = TypeRamp::default_ramp();
        assert_eq!(r.snap(12_000), 0); // closest to 11000
        assert_eq!(r.snap(30_000), 4); // closest to 32000
    }

    #[test]
    fn ramp_binds_to_chapter() {
        assert_eq!(TypeRamp::default_ramp().to_chapter("Fonts").lore_count(), 6);
    }
}
