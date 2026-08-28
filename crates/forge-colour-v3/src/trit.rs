//! `ColourTrit8` — the 8-byte Munsell colour word, T1 of the forge-vision
//! drain (plan-2-welds.md T1, quarried from v2's forge-colour/surface_ledger
//! ledger doctrine — the encoding is new, the discipline is drained).
//!
//! Machine-first (L08): this word is the exact integer encoding; every
//! perceptual form (OKLCH, sRGB, a swatch on glass) is DERIVED from it by a
//! consumer and never stored a second time.
//!
//! ONE HOME (L05): the OKLCH bridge lives beside this file in `lib.rs`; the
//! HUD swatch strip (xtask, zero-dependency by its own law) mounts this same
//! FILE via `#[path]` — the sky.rs precedent. This module is therefore
//! deliberately self-contained: no `crate::` reference may enter, or the
//! mount stops compiling. The `sky-mount` feature marks the xtask ride and
//! compiles the file data-only, so no test ever has two homes.

/// How many hues the Munsell wheel carries: 10 families (R YR Y GY G BG B PB
/// P RP) × 4 steps (2.5, 5, 7.5, 10). The wrap modulus, in one place.
pub const MUNSELL_HUES: u8 = 40;

/// Full scale for the permyriad channels. Value `0..=10_000` maps Munsell
/// value `0.0..=10.0`; chroma `0..=10_000` maps `0.0..=1.0` of the gamut
/// ceiling (the ceiling itself is the bridge's business, not this word's).
pub const PMY_MAX: u16 = 10_000;

/// One Munsell colour, 8 bytes, exact. Field order deviates from the plan's
/// listing (hue, value, chroma, alpha, reserved) so that every byte is a
/// field — a `u8, u16` head would open a padding hole for a ninth channel to
/// hide in. The offsets below are locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColourTrit8 {
    /// Munsell hue step, `0..MUNSELL_HUES`. Achromatic colours pin it to 0 —
    /// two greys of equal value must compare equal, and they cannot if hue
    /// is free.
    pub hue_idx: u8,
    /// Alpha as a flag: 0 = transparent, 1 = opaque. A third state is
    /// corruption, not translucency — blending arrives with its own tranche.
    pub alpha_flag: u8,
    /// Munsell value in permyriad, `0..=PMY_MAX`.
    pub value_pmy: u16,
    /// Munsell chroma in permyriad of the gamut ceiling, `0..=PMY_MAX`.
    pub chroma_pmy: u16,
    /// Reserved trit tags. Zero until a tranche defines them — a nonzero
    /// reserved byte today is corruption, never forward-compatibility.
    pub tags: [u8; 2],
}

impl ColourTrit8 {
    /// The origin: achromatic white — full value, no chroma, hue pinned,
    /// opaque.
    pub const WHITE: Self =
        Self { hue_idx: 0, alpha_flag: 1, value_pmy: PMY_MAX, chroma_pmy: 0, tags: [0; 2] };

    /// Opaque achromatic grey at the given value. Hue and chroma are pinned
    /// to 0 by construction.
    ///
    /// In the xtask mount the living callers are the crate's bijection tests
    /// and the OKLCH bridge, invisible to the mount's dead-code lint (the
    /// sky.rs precedent).
    #[allow(dead_code)]
    #[inline(always)]
    pub const fn achromatic(value_pmy: u16) -> Self {
        Self { hue_idx: 0, alpha_flag: 1, value_pmy, chroma_pmy: 0, tags: [0; 2] }
    }

    /// Step the hue around the wheel. Wraps at `MUNSELL_HUES` — step 39 plus
    /// one is step 0, the 359°→0° edge in degree terms.
    ///
    /// Living caller in the xtask mount: the crate's wheel-walk test (sky.rs
    /// precedent — allow scoped to the mount's blindness, not to disuse).
    #[allow(dead_code)]
    #[inline(always)]
    pub const fn rotate_hue(self, steps: u8) -> Self {
        Self { hue_idx: (self.hue_idx as u16 + steps as u16).rem_euclid(MUNSELL_HUES as u16) as u8, ..self }
    }

    /// True when chroma is zero — the colour carries no hue information.
    ///
    /// Living callers in the xtask mount: the crate's origin tests and the
    /// OKLCH bridge (sky.rs precedent).
    #[allow(dead_code)]
    #[inline(always)]
    pub const fn is_achromatic(self) -> bool {
        self.chroma_pmy == 0
    }

    /// True when every channel is inside its domain and the achromatic hue
    /// pin holds. `encode` is only a bijection over words where this is true.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.hue_idx < MUNSELL_HUES
            && self.alpha_flag <= 1
            && self.value_pmy <= PMY_MAX
            && self.chroma_pmy <= PMY_MAX
            && self.tags[0] == 0
            && self.tags[1] == 0
            && (self.chroma_pmy != 0 || self.hue_idx == 0)
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: hue, alpha, value lo/hi, chroma lo/hi, tags.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.hue_idx as u64
            | (self.alpha_flag as u64) << 8
            | (self.value_pmy as u64) << 16
            | (self.chroma_pmy as u64) << 32
            | (self.tags[0] as u64) << 48
            | (self.tags[1] as u64) << 56
    }

    /// Unpack a word. `None` for anything outside the valid domain — an
    /// out-of-range channel, a live reserved byte, or a grey with a live hue
    /// is corruption refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            hue_idx: word as u8,
            alpha_flag: (word >> 8) as u8,
            value_pmy: (word >> 16) as u16,
            chroma_pmy: (word >> 32) as u16,
            tags: [(word >> 48) as u8, (word >> 56) as u8],
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<ColourTrit8>() == 8);
const _: () = assert!(core::mem::align_of::<ColourTrit8>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping value and chroma keeps size 8
// while silently reinterpreting every stored colour.
const _: () = assert!(core::mem::offset_of!(ColourTrit8, hue_idx) == 0);
const _: () = assert!(core::mem::offset_of!(ColourTrit8, alpha_flag) == 1);
const _: () = assert!(core::mem::offset_of!(ColourTrit8, value_pmy) == 2);
const _: () = assert!(core::mem::offset_of!(ColourTrit8, chroma_pmy) == 4);
const _: () = assert!(core::mem::offset_of!(ColourTrit8, tags) == 6);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(1 + 1 + 2 + 2 + 2 == core::mem::size_of::<ColourTrit8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode∘decode is the identity at the origin.
const _: () = {
    match ColourTrit8::decode(ColourTrit8::WHITE.encode()) {
        Some(w) => {
            assert!(w.hue_idx == 0 && w.alpha_flag == 1);
            assert!(w.value_pmy == PMY_MAX && w.chroma_pmy == 0);
            assert!(w.tags[0] == 0 && w.tags[1] == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: every lattice sample survives its own wire
    /// exactly. Hue sweeps the whole wheel; value and chroma sample interior
    /// points on both sides of centre.
    #[test]
    fn bijection_holds_over_the_interior() {
        for hue in 0..MUNSELL_HUES {
            for value in [1u16, 2_500, 5_000, 7_500, 9_999] {
                for chroma in [1u16, 2_500, 5_000, 9_999] {
                    let c = ColourTrit8 {
                        hue_idx: hue,
                        alpha_flag: 1,
                        value_pmy: value,
                        chroma_pmy: chroma,
                        tags: [0; 2],
                    };
                    assert_eq!(ColourTrit8::decode(c.encode()), Some(c), "hue={hue} v={value} c={chroma}");
                }
            }
        }
    }

    /// L07 over the sentinels: 0 and PMY_MAX on both permyriad channels, both
    /// alpha states, chroma-zero rows with the hue pinned.
    #[test]
    fn bijection_holds_over_the_sentinels() {
        for value in [0u16, PMY_MAX] {
            for chroma in [0u16, PMY_MAX] {
                for alpha in [0u8, 1] {
                    let hue = if chroma == 0 { 0 } else { MUNSELL_HUES - 1 };
                    let c = ColourTrit8 {
                        hue_idx: hue,
                        alpha_flag: alpha,
                        value_pmy: value,
                        chroma_pmy: chroma,
                        tags: [0; 2],
                    };
                    assert_eq!(ColourTrit8::decode(c.encode()), Some(c), "v={value} c={chroma} a={alpha}");
                }
            }
        }
    }

    /// The 359°→0° edge: the last hue step plus one is step zero, a full
    /// wheel of single steps is the identity, and every step of the walk
    /// survives the wire.
    #[test]
    fn hue_wraps_at_the_wheel_edge_and_the_walk_round_trips() {
        let last = ColourTrit8 {
            hue_idx: MUNSELL_HUES - 1,
            alpha_flag: 1,
            value_pmy: 5_000,
            chroma_pmy: 5_000,
            tags: [0; 2],
        };
        assert_eq!(last.rotate_hue(1).hue_idx, 0, "the wheel did not close");

        let mut walker = last;
        for _ in 0..MUNSELL_HUES {
            walker = walker.rotate_hue(1);
            assert_eq!(ColourTrit8::decode(walker.encode()), Some(walker));
        }
        assert_eq!(walker, last, "a full turn of the wheel is not the identity");
    }

    /// The origin: achromatic white round-trips and is achromatic.
    #[test]
    fn the_origin_survives_its_wire() {
        assert!(ColourTrit8::WHITE.is_achromatic());
        assert_eq!(ColourTrit8::decode(ColourTrit8::WHITE.encode()), Some(ColourTrit8::WHITE));
        for l in [0u16, 1, 5_000, PMY_MAX] {
            let g = ColourTrit8::achromatic(l);
            assert!(g.is_achromatic());
            assert_eq!(g.hue_idx, 0, "a grey with a live hue compares unequal to its own twin");
            assert_eq!(ColourTrit8::decode(g.encode()), Some(g));
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None,
    /// and each is invalid for exactly the reason named.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good =
            ColourTrit8 { hue_idx: 3, alpha_flag: 1, value_pmy: 5_000, chroma_pmy: 5_000, tags: [0; 2] };
        assert!(ColourTrit8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            ColourTrit8 { hue_idx: MUNSELL_HUES, ..good },
            ColourTrit8 { alpha_flag: 2, ..good },
            ColourTrit8 { value_pmy: PMY_MAX + 1, ..good },
            ColourTrit8 { chroma_pmy: PMY_MAX + 1, ..good },
            ColourTrit8 { tags: [1, 0], ..good },
            ColourTrit8 { tags: [0, 1], ..good },
            ColourTrit8 { chroma_pmy: 0, hue_idx: 1, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(ColourTrit8::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }
}
