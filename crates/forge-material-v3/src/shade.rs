//! `MaterialShade16` — the 16-byte shading-complete material word, tranche A
//! of the material tier ladder (`.forge/brief-queue/A-material-ladder-BRIEF.md`).
//! Composition of `MaterialTrit8` (discrete material identity, this crate)
//! and `NormalAlbedo8` (continuous energy gradient, forge-photometric-v3).
//! The composition is a third thing and lives here — neither home is
//! disturbed (L05).
//!
//! Pattern copied from `material.rs`: layout locks measured by rustc (L02),
//! `Option` refusal on decode, the E0080 const origin gate, bijection tests
//! (L07).

#[cfg(feature = "sky-mount")]
use crate::forge_photometric_v3::NormalAlbedo8;
#[cfg(not(feature = "sky-mount"))]
use forge_photometric_v3::NormalAlbedo8;

use crate::material::MaterialTrit8;

/// One shading-complete material texel, 16 bytes, exact: the discrete
/// material word followed by the photometric word. Field order is offset
/// order — every byte is a field, no padding hole. The offsets below are
/// locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialShade16 {
    /// The discrete material identity layer.
    pub trit: MaterialTrit8,
    /// The continuous photometric layer.
    pub shade: NormalAlbedo8,
}

impl MaterialShade16 {
    /// The origin: absence (`MaterialTrit8::EMPTY`) under a flat,
    /// mirror-smooth white shading layer (`NormalAlbedo8::FLAT`).
    pub const FLAT_NONE: Self = Self { trit: MaterialTrit8::EMPTY, shade: NormalAlbedo8::FLAT };

    /// True when both layers are inside their own domain.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.trit.is_valid() && self.shade.is_valid()
    }

    /// Pack into two little-endian u64 words: the material word, then the
    /// shading word.
    #[inline(always)]
    pub const fn encode(self) -> [u64; 2] {
        [self.trit.encode(), self.shade.encode()]
    }

    /// Unpack two words. `None` unless both halves decode — a texel with one
    /// corrupt layer is corruption refused whole, not half-accepted.
    #[inline(always)]
    pub const fn decode(words: [u64; 2]) -> Option<Self> {
        let trit = match MaterialTrit8::decode(words[0]) {
            Some(t) => t,
            None => return None,
        };
        let shade = match NormalAlbedo8::decode(words[1]) {
            Some(s) => s,
            None => return None,
        };
        Some(Self { trit, shade })
    }

    /// LOD downshift: drop the photometric layer, keep the material word
    /// exactly. Exact over the retained channels by construction — no
    /// transform, a plain field read.
    #[inline(always)]
    pub const fn downshift_8(self) -> MaterialTrit8 {
        self.trit
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<MaterialShade16>() == 16);
const _: () = assert!(core::mem::align_of::<MaterialShade16>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping the two layers keeps size 16
// while silently reinterpreting every stored texel.
const _: () = assert!(core::mem::offset_of!(MaterialShade16, trit) == 0);
const _: () = assert!(core::mem::offset_of!(MaterialShade16, shade) == 8);

// Every one of the 16 bytes is a field — no padding hole.
const _: () = assert!(
    core::mem::size_of::<MaterialTrit8>() + core::mem::size_of::<NormalAlbedo8>()
        == core::mem::size_of::<MaterialShade16>()
);

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match MaterialShade16::decode(MaterialShade16::FLAT_NONE.encode()) {
        Some(w) => {
            assert!(w.trit.fill_pmy == 0 && w.shade.oct_u == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;
    use crate::material::{MATERIAL_ASH, MATERIAL_ID_MAX, MATERIAL_IRON, MATERIAL_STONE};

    /// L07 over the interior: composed words survive their own wire exactly.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        let samples = [
            (MaterialTrit8 { fill_pmy: 2_500, material_id: MATERIAL_IRON, craft: [1, 40, 121, 200, 241] },
             NormalAlbedo8 { oct_u: 12_345, oct_v: 54_321, albedo_pmy: 2_500, roughness_pmy: 7_500 }),
            (MaterialTrit8 { fill_pmy: 7_500, material_id: MATERIAL_STONE, craft: [0, 121, 242, 60, 180] },
             NormalAlbedo8 { oct_u: 0x7fff, oct_v: 0x8001, albedo_pmy: 9_999, roughness_pmy: 1 }),
        ];
        for (trit, shade) in samples {
            let w = MaterialShade16 { trit, shade };
            assert_eq!(MaterialShade16::decode(w.encode()), Some(w));
        }
    }

    /// L07 over the sentinels: both layer extremes together.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for material_id in [0u8, MATERIAL_ASH] {
            for fill_pmy in [0u16, 10_000] {
                for (albedo, roughness) in [(0u16, 0u16), (10_000, 10_000)] {
                    let w = MaterialShade16 {
                        trit: MaterialTrit8 { fill_pmy, material_id, craft: [0; 5] },
                        shade: NormalAlbedo8 { oct_u: 0, oct_v: 0, albedo_pmy: albedo, roughness_pmy: roughness },
                    };
                    assert_eq!(MaterialShade16::decode(w.encode()), Some(w));
                }
            }
        }
    }

    /// The boundary refuses corruption: a bad layer on either side decodes to
    /// `None` for the whole word.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = MaterialShade16 {
            trit: MaterialTrit8 { fill_pmy: 5_000, material_id: MATERIAL_STONE, craft: [10, 20, 30, 40, 50] },
            shade: NormalAlbedo8 { oct_u: 100, oct_v: 200, albedo_pmy: 5_000, roughness_pmy: 5_000 },
        };
        assert!(MaterialShade16::decode(good.encode()).is_some(), "the baseline itself is invalid");

        // A sentinel material_id (`MATERIAL_ID_MAX + 1`) is in-domain now — a
        // craft byte past `CRAFT_BYTE_MAX` is what makes the layer corrupt.
        let bad_trit = MaterialShade16 {
            trit: MaterialTrit8 { craft: [243, 20, 30, 40, 50], ..good.trit },
            ..good
        };
        let bad_shade = MaterialShade16 {
            shade: NormalAlbedo8 { oct_u: 0x8000, ..good.shade },
            ..good
        };
        assert_eq!(MaterialShade16::decode(bad_trit.encode()), None);
        assert_eq!(MaterialShade16::decode(bad_shade.encode()), None);

        let sentinel_trit = MaterialShade16 {
            trit: MaterialTrit8 { material_id: MATERIAL_ID_MAX + 1, ..good.trit },
            ..good
        };
        assert!(MaterialShade16::decode(sentinel_trit.encode()).is_some(), "a sentinel id must decode");
    }

    /// The origin: `FLAT_NONE` round-trips and reads back as no-material
    /// under a flat shading layer.
    #[test]
    fn the_origin_survives_its_wire() {
        let w = MaterialShade16::FLAT_NONE;
        assert_eq!(MaterialShade16::decode(w.encode()), Some(w));
    }

    /// Downshift to `MaterialTrit8` is exact over the retained channel: the
    /// material word is unchanged, the photometric word is simply dropped.
    #[test]
    fn downshift_8_is_exact_over_the_retained_material_word() {
        let trit = MaterialTrit8 { fill_pmy: 4_321, material_id: MATERIAL_IRON, craft: [5, 15, 25, 35, 45] };
        let w = MaterialShade16 { trit, shade: NormalAlbedo8::FLAT };
        assert_eq!(w.downshift_8(), trit);
    }
}
