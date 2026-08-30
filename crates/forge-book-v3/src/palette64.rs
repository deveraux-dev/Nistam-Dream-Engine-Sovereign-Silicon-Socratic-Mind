//! Palette64 — the 64 palette chips (from the vixi playground far rail). Each
//! chip is a deterministic OKLCH spread by id.

use crate::colour::Oklch;

/// The 64-chip palette.
pub struct Palette64;

impl Palette64 {
    /// The number of palette chips.
    pub const LEN: u8 = 64;

    /// The colour of chip `id` (0..=63; wraps). Hue spreads the ring; lightness
    /// alternates in two bands so neighbours read apart.
    pub fn chip(id: u8) -> Oklch {
        let i = (id % Self::LEN) as u32;
        let h = i * 360 / Self::LEN as u32;
        let l = if i % 2 == 0 { 6500 } else { 5000 };
        Oklch::new(l, 2000, h)
    }

    /// All 64 chips in order.
    pub fn all() -> Vec<Oklch> {
        (0..Self::LEN).map(Self::chip).collect()
    }
}

/// Grid64 — the AUTHORED 64-colour tactile grid (Sean 2026-07-17), the hand-tuned artist
/// picker distinct from the procedural [`Palette64`] hue-ring above. An 8-family × 8-column
/// matrix (`design/palette/grid64.vibe.vixi` is the `.vixi` twin): ASH values · BLAZE fire ·
/// CLAY ground · WILD growth · WAVE fluid · PLUM twilight · GLOW neon · SOFT glaze. Exact
/// sRGB, no rounding. Flat index = row*8 + col; family = index/8. The tone LADDER of any
/// chip is that colour over the dark ground at descending opacity (WHITE@a → the ASH grays),
/// so shades are composited, never authored — see `overlay_tones`.
pub struct Grid64;

impl Grid64 {
    /// The number of grid chips.
    pub const LEN: u8 = 64;

    /// The 8 family names, row0..row7.
    pub const FAMILIES: [&'static str; 8] =
        ["ASH", "BLAZE", "CLAY", "WILD", "WAVE", "PLUM", "GLOW", "SOFT"];

    /// `(word, sRGB)` for all 64 chips, row-major (row = family, col = 0..7).
    pub const CHIPS: [(&'static str, [u8; 3]); 64] = [
        // row0 ASH — value anchor
        ("WHITE", [255, 255, 255]), ("ICE", [234, 234, 234]), ("CLOUD", [204, 204, 204]),
        ("GRAY", [153, 153, 153]), ("STONE", [102, 102, 102]), ("IRON", [68, 68, 68]),
        ("CHAR", [34, 34, 34]), ("BLACK", [0, 0, 0]),
        // row1 BLAZE — fire
        ("CANDLE", [255, 255, 85]), ("SUN", [255, 170, 0]), ("GOLD", [212, 175, 55]),
        ("ORANGE", [255, 85, 0]), ("FLAME", [255, 34, 0]), ("AMBER", [204, 68, 0]),
        ("RED", [255, 0, 0]), ("BLOOD", [136, 0, 0]),
        // row2 CLAY — ground
        ("BONE", [245, 245, 220]), ("SAND", [238, 220, 130]), ("DUST", [194, 178, 128]),
        ("EARTH", [150, 75, 0]), ("BRICK", [178, 34, 34]), ("MUD", [92, 64, 51]),
        ("CHOCK", [61, 35, 20]), ("BARK", [37, 22, 15]),
        // row3 WILD — growth
        ("LIME", [0, 255, 0]), ("FERN", [127, 255, 0]), ("MOSS", [173, 255, 47]),
        ("LEAF", [0, 170, 0]), ("GRASS", [34, 139, 34]), ("JUNGLE", [0, 85, 0]),
        ("HOLLY", [1, 50, 32]), ("PINE", [11, 47, 29]),
        // row4 WAVE — fluid
        ("MIST", [224, 255, 255]), ("TEAL", [0, 255, 255]), ("OCEAN", [0, 170, 255]),
        ("RIVER", [0, 85, 255]), ("SKY", [135, 206, 235]), ("BLUE", [0, 0, 255]),
        ("SHADOW", [0, 0, 136]), ("DEEP", [0, 0, 51]),
        // row5 PLUM — twilight
        ("BLUSH", [255, 192, 203]), ("PINK", [255, 105, 180]), ("CANDY", [255, 0, 255]),
        ("BERRY", [199, 21, 133]), ("PLUM", [128, 0, 128]), ("VIOLET", [75, 0, 130]),
        ("GRAPE", [49, 0, 74]), ("NIGHT", [26, 0, 44]),
        // row6 GLOW — neon
        ("LASER", [170, 255, 0]), ("SHINE", [0, 255, 170]), ("ELECTRIC", [0, 170, 255]),
        ("LAVA", [255, 69, 0]), ("ACID", [223, 255, 0]), ("MAGENTA", [255, 0, 127]),
        ("ALIEN", [57, 255, 20]), ("HOT", [255, 20, 147]),
        // row7 SOFT — glaze
        ("SILK", [255, 248, 220]), ("CREAM", [253, 245, 230]), ("SAGE", [188, 143, 143]),
        ("OLIVE", [128, 128, 0]), ("DENIM", [70, 130, 180]), ("COAL", [47, 79, 79]),
        ("CLOVER", [85, 107, 47]), ("RUST", [139, 69, 19]),
    ];

    /// sRGB of chip `id` (0..=63; wraps).
    pub fn rgb(id: u8) -> [u8; 3] {
        Self::CHIPS[(id % Self::LEN) as usize].1
    }

    /// The 6yo WORD of chip `id` (WHITE, LAVA, OCEAN, …).
    pub fn word(id: u8) -> &'static str {
        Self::CHIPS[(id % Self::LEN) as usize].0
    }

    /// The family (row) name of chip `id`.
    pub fn family(id: u8) -> &'static str {
        Self::FAMILIES[((id / 8) % 8) as usize]
    }

    /// sRGB of chip `id` composited over the dark ground at Permyriad opacity `a`
    /// (10000 = opaque). The tone ladder is one chip walked down `a` — this is how a
    /// shade is MADE, not authored (WHITE at 5000 over black ≈ GRAY).
    pub fn rgb_over(id: u8, ground: [u8; 3], a_pmy: u16) -> [u8; 3] {
        let c = Self::rgb(id);
        let a = a_pmy.min(10000) as u32;
        let inv = 10000 - a;
        [
            ((c[0] as u32 * a + ground[0] as u32 * inv) / 10000) as u8,
            ((c[1] as u32 * a + ground[1] as u32 * inv) / 10000) as u8,
            ((c[2] as u32 * a + ground[2] as u32 * inv) / 10000) as u8,
        ]
    }

    /// All 64 chips' sRGB in order.
    pub fn all() -> Vec<[u8; 3]> {
        (0..Self::LEN).map(Self::rgb).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_sixty_four_chips() {
        assert_eq!(Palette64::all().len(), 64);
    }

    #[test]
    fn hue_spreads_around_the_ring() {
        assert_eq!(Palette64::chip(0).h_deg, 0);
        assert_eq!(Palette64::chip(32).h_deg, 180);
        assert_eq!(Palette64::chip(64).h_deg, 0); // wraps
    }

    #[test]
    fn lightness_alternates() {
        assert_ne!(Palette64::chip(0).l_pmy, Palette64::chip(1).l_pmy);
    }

    #[test]
    fn grid64_has_sixty_four_authored_chips() {
        assert_eq!(Grid64::all().len(), 64);
        assert_eq!(Grid64::CHIPS.len(), 64);
    }

    #[test]
    fn grid64_families_map_by_row() {
        assert_eq!(Grid64::family(0), "ASH"); // row0 col0 WHITE
        assert_eq!(Grid64::word(0), "WHITE");
        assert_eq!(Grid64::family(8), "BLAZE"); // row1 col0 CANDLE
        assert_eq!(Grid64::family(51), "GLOW"); // row6 (51/8=6) LAVA
        assert_eq!(Grid64::word(51), "LAVA");
        assert_eq!(Grid64::rgb(51), [255, 69, 0]); // LAVA is a real orange, not gray
        assert_eq!(Grid64::rgb(34), [0, 170, 255]); // WAVE OCEAN is a real blue
    }

    #[test]
    fn grid64_tone_is_made_by_opacity_not_authored() {
        // WHITE at 50% over black ≈ mid-gray — the ASH ladder is composited, not a swatch.
        let g = Grid64::rgb_over(0, [0, 0, 0], 5000);
        assert_eq!(g, [127, 127, 127]);
        // Opaque returns the chip; zero returns the ground.
        assert_eq!(Grid64::rgb_over(24, [0, 0, 0], 10000), Grid64::rgb(24));
        assert_eq!(Grid64::rgb_over(24, [12, 12, 12], 0), [12, 12, 12]);
    }
}
