//! Colour — OKLCH harmony for the Atlas (harvested from forge-colour). Integer
//! lightness/chroma (permyriad) and hue (degrees); harmony schemes by rotation.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// An OKLCH colour with integer channels: L/C permyriad, hue degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Oklch {
    /// Lightness in permyriad (0–10,000).
    pub l_pmy: u32,
    /// Chroma in permyriad (0–10,000).
    pub c_pmy: u32,
    /// Hue in degrees (0–359).
    pub h_deg: u32,
}

impl Oklch {
    /// Create a new Oklch colour, clamping L/C to 10,000 and hue to 0–359.
    pub fn new(l_pmy: u32, c_pmy: u32, h_deg: u32) -> Self {
        Self { l_pmy: l_pmy.min(10_000), c_pmy: c_pmy.min(10_000), h_deg: h_deg % 360 }
    }

    /// Rotate hue by `deg` degrees (wraps).
    pub fn rotated(&self, deg: u32) -> Oklch {
        Oklch { h_deg: (self.h_deg + deg) % 360, ..*self }
    }

    /// The complement (hue + 180).
    pub fn complement(&self) -> Oklch {
        self.rotated(180)
    }

    /// The triad — base + two 120-apart hues.
    pub fn triad(&self) -> [Oklch; 3] {
        [*self, self.rotated(120), self.rotated(240)]
    }

    /// Analogous — base flanked by +/-30.
    pub fn analogous(&self) -> [Oklch; 3] {
        [self.rotated(330), *self, self.rotated(30)]
    }
}

/// Bind a base colour's harmonies into a Colour chapter.
pub fn to_chapter(base: Oklch, title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Colour".into()));
    ch.add_lore(format!("base L{} C{} H{}", base.l_pmy, base.c_pmy, base.h_deg));
    ch.add_lore(format!("complement H{}", base.complement().h_deg));
    let t = base.triad();
    ch.add_lore(format!("triad H{} H{} H{}", t[0].h_deg, t[1].h_deg, t[2].h_deg));
    let a = base.analogous();
    ch.add_lore(format!("analogous H{} H{} H{}", a[0].h_deg, a[1].h_deg, a[2].h_deg));
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complement_is_180_away() {
        let c = Oklch::new(6000, 1500, 30);
        assert_eq!(c.complement().h_deg, 210);
    }

    #[test]
    fn triad_is_120_apart() {
        let t = Oklch::new(6000, 1500, 0).triad();
        assert_eq!(t[1].h_deg, 120);
        assert_eq!(t[2].h_deg, 240);
    }

    #[test]
    fn analogous_wraps_below_zero() {
        let a = Oklch::new(6000, 1500, 10).analogous();
        assert_eq!(a[0].h_deg, 340); // 10 - 30 wraps
        assert_eq!(a[2].h_deg, 40);
    }

    #[test]
    fn channels_clamp() {
        let c = Oklch::new(99_999, 99_999, 725);
        assert_eq!(c.l_pmy, 10_000);
        assert_eq!(c.h_deg, 5); // 725 % 360
    }
}
