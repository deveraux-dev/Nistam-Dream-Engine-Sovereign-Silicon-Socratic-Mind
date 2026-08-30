//! `MaterialStack32` — the 32-byte two-layer material stack (substrate,
//! coat) joined by one `ReactionSeam16`, tranche A of the material tier
//! ladder (`.forge/brief-queue/A-material-ladder-BRIEF.md`).
//!
//! Pattern copied from `material.rs`: layout locks measured by rustc (L02),
//! `Option` refusal on decode, the E0080 const origin gate, bijection tests
//! (L07). The layer-sum refusal is the stack invariant observed at
//! `F:\NewRepo\crates\forge-vision\src\scan\material_traits.rs:8` — a
//! stack's overlapping layer fills may not exceed 10_000, and overflow is
//! refused, never clamped.
//!
//! ## `MaterialStack64` — the blocked layout, and how it was resolved
//!
//! The tranche-A brief listed three components for this word —
//! `[MaterialTrit8; 4]` (32B), `[ReactionSeam16; 2]` (32B) and `Coeffs16`
//! (16B) — then asserted "that is 32 + 32 == 64". That arithmetic silently
//! dropped `Coeffs16`: the three components sum to **80B**, and 80B does not
//! round onto a 64B align-64 cache line either. The weld correctly refused to
//! guess which correction was meant and stopped (the truth gate: do not pad
//! silently, do not widen the word, do not drop a field).
//!
//! **Resolution (ARCH000 2026-08-11): `Coeffs16` was never per-texel.**
//! Density, friction, restitution, cents, soul and essence are properties of a
//! *material*, not of a *texel* — they are looked up by `material_id`, not
//! stored once per pixel. Inlining them here is precisely the mistake the
//! HOTPATH ISOLATION LAW forbids at `forge-core-v3/src/soul.rs:206-231`, which
//! asserts `size_of::<SoulIdentity>() > size_of::<Pexil>()` so a 12-byte
//! identity can never be inlined into the 8-byte hot atom and collapse the
//! cache line from 8 atoms to 3 (`soul.rs:225-231`). The same law applies:
//! this word is hot per-texel data, `Coeffs16` is cold per-material data.
//!
//! So `Coeffs16` keeps its own home in `coeffs.rs` as a **registry entry keyed
//! by `material_id`**, and the word closes exactly:
//! `[MaterialStack32; 2]` = `[MaterialTrit8; 4]` + `[ReactionSeam16; 2]`
//! = 32 + 32 = **64B, align 64 — one L1 line**. Nothing padded, nothing
//! dropped, and the rule is inherited rather than invented.

use crate::material::MaterialTrit8;
use crate::seam::ReactionSeam16;

#[cfg(feature = "sky-mount")]
use crate::forge_photometric_v3::NormalAlbedo8;
#[cfg(not(feature = "sky-mount"))]
use forge_photometric_v3::NormalAlbedo8;

/// A two-layer material stack, 32 bytes, exact: substrate and coat layers
/// joined by one reaction seam. Field order is offset order — every byte is
/// a field, no padding hole. The offsets below are locked by rustc, not
/// prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialStack32 {
    /// The two layers, substrate (`[0]`) then coat (`[1]`).
    pub layers: [MaterialTrit8; 2],
    /// What crosses the boundary between the two layers.
    pub seam: ReactionSeam16,
}

impl MaterialStack32 {
    /// The origin: both layers empty, seam at rest.
    pub const NONE: Self = Self { layers: [MaterialTrit8::EMPTY; 2], seam: ReactionSeam16::NONE };

    /// True when both layers are valid, the seam is valid, and the layer
    /// fills sum to `<= 10_000` — the stack invariant (fact 6 of the brief):
    /// overflow is refused, never clamped.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.layers[0].is_valid()
            && self.layers[1].is_valid()
            && self.seam.is_valid()
            && self.layers[0].fill_pmy as u32 + self.layers[1].fill_pmy as u32 <= 10_000
    }

    /// Pack into four little-endian u64 words: layer 0, layer 1, then the
    /// seam's two words.
    #[inline(always)]
    pub const fn encode(self) -> [u64; 4] {
        let seam = self.seam.encode();
        [self.layers[0].encode(), self.layers[1].encode(), seam[0], seam[1]]
    }

    /// Unpack four words. `None` unless every part decodes and the layer-sum
    /// invariant holds — corruption refused at the boundary, not clamped
    /// past it.
    #[inline(always)]
    pub const fn decode(words: [u64; 4]) -> Option<Self> {
        let l0 = match MaterialTrit8::decode(words[0]) {
            Some(l) => l,
            None => return None,
        };
        let l1 = match MaterialTrit8::decode(words[1]) {
            Some(l) => l,
            None => return None,
        };
        let seam = match ReactionSeam16::decode([words[2], words[3]]) {
            Some(s) => s,
            None => return None,
        };
        let c = Self { layers: [l0, l1], seam };
        if c.is_valid() {
            Some(c)
        } else {
            None
        }
    }

    /// LOD downshift: drop the coat layer and the seam, keep the substrate
    /// layer composed with the supplied shading layer. Exact over the
    /// retained material channel — the substrate's fields are unchanged.
    #[inline(always)]
    pub const fn downshift_16(self, shade: NormalAlbedo8) -> crate::shade::MaterialShade16 {
        crate::shade::MaterialShade16 { trit: self.layers[0], shade }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<MaterialStack32>() == 32);
const _: () = assert!(core::mem::align_of::<MaterialStack32>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping the layers and the seam keeps
// size 32 while silently reinterpreting every stored stack.
const _: () = assert!(core::mem::offset_of!(MaterialStack32, layers) == 0);
const _: () = assert!(core::mem::offset_of!(MaterialStack32, seam) == 16);

// Every one of the 32 bytes is a field — no padding hole.
const _: () = assert!(
    2 * core::mem::size_of::<MaterialTrit8>() + core::mem::size_of::<ReactionSeam16>()
        == core::mem::size_of::<MaterialStack32>()
);

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match MaterialStack32::decode(MaterialStack32::NONE.encode()) {
        Some(w) => {
            assert!(w.layers[0].fill_pmy == 0 && w.layers[1].fill_pmy == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

/// The full four-layer material stack, 64 bytes, exact — two
/// [`MaterialStack32`] halves, so four layers joined by two reaction seams.
/// `align(64)` puts one whole stack on one L1 cache line, the `PexilLine`
/// precedent (`forge-core-v3/src/soul.rs:225-231`).
///
/// `Coeffs16` is deliberately NOT a field here — see this module's header.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialStack64 {
    /// The two halves: layers 0-1 with seam 0, then layers 2-3 with seam 1.
    pub halves: [MaterialStack32; 2],
}

impl MaterialStack64 {
    /// The origin: four empty layers, both seams at rest.
    pub const NONE: Self = Self { halves: [MaterialStack32::NONE; 2] };

    /// The stack invariant's ceiling, in permyriad. One home for the constant
    /// both this word and [`MaterialStack32`] are bound by.
    pub const FILL_SUM_MAX: u32 = 10_000;

    /// The sum of all four layer fills, in permyriad. Cannot overflow: four
    /// `u16` maxima sum to 262_140, well inside `u32`.
    #[inline(always)]
    pub const fn fill_sum(self) -> u32 {
        self.halves[0].layers[0].fill_pmy as u32
            + self.halves[0].layers[1].fill_pmy as u32
            + self.halves[1].layers[0].fill_pmy as u32
            + self.halves[1].layers[1].fill_pmy as u32
    }

    /// True when both halves are valid AND all four layer fills together sum
    /// to `<= FILL_SUM_MAX`.
    ///
    /// This is **strictly stronger** than the two halves' own checks: each
    /// half only bounds its own pair, so two individually-valid halves can
    /// still carry 20_000 permyriad between them. The observed invariant
    /// (`F:\NewRepo\crates\forge-vision\src\scan\material_traits.rs:8`) bounds
    /// the sum across *every* overlapping material in one voxel, and all four
    /// layers overlap in one voxel — so the total is the quantity that must be
    /// checked. Overflow is refused, never clamped.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.halves[0].is_valid()
            && self.halves[1].is_valid()
            && self.fill_sum() <= Self::FILL_SUM_MAX
    }

    /// Pack into eight little-endian u64 words: half 0's four, then half 1's.
    #[inline(always)]
    pub const fn encode(self) -> [u64; 8] {
        let a = self.halves[0].encode();
        let b = self.halves[1].encode();
        [a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]]
    }

    /// Unpack eight words. `None` unless both halves decode and the four-layer
    /// sum invariant holds — corruption refused at the boundary.
    #[inline(always)]
    pub const fn decode(words: [u64; 8]) -> Option<Self> {
        let a = match MaterialStack32::decode([words[0], words[1], words[2], words[3]]) {
            Some(h) => h,
            None => return None,
        };
        let b = match MaterialStack32::decode([words[4], words[5], words[6], words[7]]) {
            Some(h) => h,
            None => return None,
        };
        let c = Self { halves: [a, b] };
        if c.is_valid() {
            Some(c)
        } else {
            None
        }
    }

    /// LOD downshift: keep the first half (layers 0-1 and seam 0), drop the
    /// second. Exact over every retained channel — the returned half is the
    /// stored half, unmodified.
    #[inline(always)]
    pub const fn downshift_32(self) -> MaterialStack32 {
        self.halves[0]
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS for the 64-byte word. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<MaterialStack64>() == 64);
const _: () = assert!(core::mem::align_of::<MaterialStack64>() == 64);
const _: () = assert!(core::mem::offset_of!(MaterialStack64, halves) == 0);

// Every one of the 64 bytes is a field — no padding hole, and no room for a
// `Coeffs16` to hide in. This is the arithmetic that the brief got wrong,
// stated as a gate so it cannot be got wrong again.
const _: () = assert!(2 * core::mem::size_of::<MaterialStack32>() == core::mem::size_of::<MaterialStack64>());
const _: () = assert!(
    4 * core::mem::size_of::<MaterialTrit8>() + 2 * core::mem::size_of::<ReactionSeam16>()
        == core::mem::size_of::<MaterialStack64>()
);

// The HOTPATH ISOLATION LAW, restated as arithmetic for this word (the
// `soul.rs:206-231` precedent): adding `Coeffs16` would push the stack past
// one cache line, which is the whole reason it lives in a per-material
// registry instead. If a future edit inlines it, this stops compiling.
//
// Not under the HUD mount: `coeffs.rs` reaches `forge_core_v3::EssenceId`,
// which lives in `soul.rs` and pulls `arch.rs` + `atom.rs` behind it. The HUD
// face mounts only the four files the 64-byte word is built from, so this one
// gate is compiled in the crate's own build — where it is the gate that
// matters — and skipped under the mount. The lock is not weakened: it still
// fires from `cargo test -p forge-material-v3`.
#[cfg(not(feature = "sky-mount"))]
const _: () =
    assert!(core::mem::size_of::<MaterialStack64>() + core::mem::size_of::<crate::coeffs::Coeffs16>() > 64);

// One whole stack per L1 line — the PexilLine precedent (soul.rs:225-231).
const _: () = assert!(64 / core::mem::size_of::<MaterialStack64>() == 1);

// The origin survives its own wire, as an E0080 gate before any test runs.
const _: () = {
    match MaterialStack64::decode(MaterialStack64::NONE.encode()) {
        Some(w) => {
            assert!(w.fill_sum() == 0);
        }
        None => panic!("the 64-byte origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;
    use crate::material::{MATERIAL_IRON, MATERIAL_STONE};

    fn layer(fill_pmy: u16, material_id: u8) -> MaterialTrit8 {
        MaterialTrit8 { fill_pmy, material_id, craft: [0; 5] }
    }

    /// L07 over the interior: composed stacks survive their own wire
    /// exactly.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        let w = MaterialStack32 {
            layers: [layer(3_000, MATERIAL_IRON), layer(4_000, MATERIAL_STONE)],
            seam: ReactionSeam16 { conduct_therm_pmy: 2_500, ..ReactionSeam16::NONE },
        };
        assert_eq!(MaterialStack32::decode(w.encode()), Some(w));
    }

    /// L07 over the sum-boundary sentinel: layer fills summing to exactly
    /// 10_000 (the closed boundary) survive the wire.
    #[test]
    fn word_bijection_holds_at_the_sum_boundary() {
        let w = MaterialStack32 {
            layers: [layer(10_000, MATERIAL_IRON), layer(0, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        };
        assert_eq!(MaterialStack32::decode(w.encode()), Some(w));

        let w2 = MaterialStack32 {
            layers: [layer(5_000, MATERIAL_IRON), layer(5_000, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        };
        assert_eq!(MaterialStack32::decode(w2.encode()), Some(w2));
    }

    /// The stack invariant (fact 6): a layer-fill sum of 10_001 is refused,
    /// not clamped. This row is mandatory.
    #[test]
    fn a_stack_whose_layer_fills_sum_past_ten_thousand_is_refused() {
        let w = MaterialStack32 {
            layers: [layer(10_000, MATERIAL_IRON), layer(1, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        };
        assert!(!w.is_valid(), "10_001 must not validate");
        assert_eq!(MaterialStack32::decode(w.encode()), None, "10_001 must decode to None, not clamp");
    }

    /// The boundary refuses corruption on any of the three parts.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = MaterialStack32 {
            layers: [layer(3_000, MATERIAL_IRON), layer(3_000, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        };
        assert!(MaterialStack32::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad_layer = MaterialStack32 { layers: [layer(10_001, MATERIAL_IRON), good.layers[1]], ..good };
        let bad_seam = MaterialStack32 { seam: ReactionSeam16 { react_residual: 255, ..ReactionSeam16::NONE }, ..good };
        assert_eq!(MaterialStack32::decode(bad_layer.encode()), None);
        assert_eq!(MaterialStack32::decode(bad_seam.encode()), None);
    }

    /// The origin: `NONE` round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        let w = MaterialStack32::NONE;
        assert_eq!(MaterialStack32::decode(w.encode()), Some(w));
    }

    /// Downshift to `MaterialShade16` is exact over the retained substrate
    /// layer: the coat layer and seam are dropped, the substrate's fields
    /// are unchanged, and the supplied shading layer is composed in as-is.
    #[test]
    fn downshift_16_is_exact_over_the_retained_substrate_layer() {
        let substrate = layer(6_000, MATERIAL_IRON);
        let w = MaterialStack32 {
            layers: [substrate, layer(2_000, MATERIAL_STONE)],
            seam: ReactionSeam16 { conduct_therm_pmy: 1_234, ..ReactionSeam16::NONE },
        };
        let shade = NormalAlbedo8 { oct_u: 111, oct_v: 222, albedo_pmy: 5_000, roughness_pmy: 5_000 };
        let shade16 = w.downshift_16(shade);
        assert_eq!(shade16.trit, substrate);
        assert_eq!(shade16.shade, shade);
    }

    // ---- the 64-byte word -------------------------------------------------

    fn half(f0: u16, f1: u16) -> MaterialStack32 {
        MaterialStack32 {
            layers: [layer(f0, MATERIAL_IRON), layer(f1, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        }
    }

    /// L07 over the interior: a four-layer stack survives its own wire.
    #[test]
    fn stack64_bijection_holds_over_the_interior() {
        let w = MaterialStack64 {
            halves: [
                MaterialStack32 {
                    layers: [layer(2_000, MATERIAL_IRON), layer(3_000, MATERIAL_STONE)],
                    seam: ReactionSeam16 { conduct_therm_pmy: 1_500, ..ReactionSeam16::NONE },
                },
                MaterialStack32 {
                    layers: [layer(1_000, MATERIAL_STONE), layer(4_000, MATERIAL_IRON)],
                    seam: ReactionSeam16 { conduct_elec_pmy: 9_999, ..ReactionSeam16::NONE },
                },
            ],
        };
        assert_eq!(w.fill_sum(), 10_000);
        assert_eq!(MaterialStack64::decode(w.encode()), Some(w));
    }

    /// The origin round-trips, and the E0080 gate's claim holds at runtime too.
    #[test]
    fn stack64_origin_survives_its_wire() {
        let w = MaterialStack64::NONE;
        assert_eq!(w.fill_sum(), 0);
        assert_eq!(MaterialStack64::decode(w.encode()), Some(w));
    }

    /// The four-layer sum invariant is STRICTER than the halves' own checks.
    /// This is the bug the test exists for: two halves that are each
    /// individually valid (5_000 + 5_000 = 10_000 apiece) still carry 20_000
    /// permyriad between them, which the voxel invariant forbids.
    #[test]
    fn two_individually_valid_halves_can_still_break_the_four_layer_sum() {
        let a = half(5_000, 5_000);
        let b = half(5_000, 5_000);
        assert!(a.is_valid(), "half a must be valid on its own");
        assert!(b.is_valid(), "half b must be valid on its own");

        let w = MaterialStack64 { halves: [a, b] };
        assert_eq!(w.fill_sum(), 20_000);
        assert!(!w.is_valid(), "20_000 across four layers must not validate");
        assert_eq!(MaterialStack64::decode(w.encode()), None, "must refuse, not clamp");
    }

    /// The closed boundary: exactly 10_000 across four layers is legal, and
    /// 10_001 is refused.
    #[test]
    fn the_four_layer_sum_boundary_is_closed_at_ten_thousand() {
        let ok = MaterialStack64 { halves: [half(2_500, 2_500), half(2_500, 2_500)] };
        assert_eq!(ok.fill_sum(), 10_000);
        assert_eq!(MaterialStack64::decode(ok.encode()), Some(ok));

        let over = MaterialStack64 { halves: [half(2_500, 2_500), half(2_500, 2_501)] };
        assert_eq!(over.fill_sum(), 10_001);
        assert_eq!(MaterialStack64::decode(over.encode()), None);
    }

    /// A corrupt part in either half is refused.
    #[test]
    fn stack64_refuses_a_corrupt_half() {
        let good = MaterialStack64 { halves: [half(1_000, 1_000), half(1_000, 1_000)] };
        assert!(MaterialStack64::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad_seam = MaterialStack32 {
            seam: ReactionSeam16 { react_residual: 255, ..ReactionSeam16::NONE },
            ..half(1_000, 1_000)
        };
        let w = MaterialStack64 { halves: [good.halves[0], bad_seam] };
        assert_eq!(MaterialStack64::decode(w.encode()), None);
    }

    /// Downshift 64 -> 32 is exact: the returned half is the stored half.
    #[test]
    fn downshift_32_is_exact_over_the_retained_half() {
        let first = MaterialStack32 {
            layers: [layer(3_000, MATERIAL_IRON), layer(1_000, MATERIAL_STONE)],
            seam: ReactionSeam16 { conduct_therm_pmy: 4_321, ..ReactionSeam16::NONE },
        };
        let w = MaterialStack64 { halves: [first, half(1_000, 1_000)] };
        assert_eq!(w.downshift_32(), first);
    }

    /// The whole ladder, top to bottom: 64 -> 32 -> 16 -> 8, exact at every
    /// step over the channels each tier retains.
    #[test]
    fn the_full_ladder_downshifts_exactly() {
        let substrate = layer(6_000, MATERIAL_IRON);
        let first = MaterialStack32 {
            layers: [substrate, layer(2_000, MATERIAL_STONE)],
            seam: ReactionSeam16::NONE,
        };
        let w64 = MaterialStack64 { halves: [first, half(1_000, 1_000)] };
        let shade = NormalAlbedo8 { oct_u: 7, oct_v: 9, albedo_pmy: 4_242, roughness_pmy: 1_111 };

        let w32 = w64.downshift_32();
        assert_eq!(w32, first);

        let w16 = w32.downshift_16(shade);
        assert_eq!(w16.trit, substrate);
        assert_eq!(w16.shade, shade);

        assert_eq!(w16.downshift_8(), substrate);
    }
}
