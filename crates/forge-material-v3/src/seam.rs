//! `ReactionSeam16` — what crosses the boundary between two material
//! layers, tranche A of the material tier ladder
//! (`.forge/brief-queue/A-material-ladder-BRIEF.md`). THE NEW HOME for this
//! word and for the reagent residual pair codec (L05).
//!
//! Pattern copied from `material.rs`: layout locks measured by rustc (L02),
//! `Option` refusal on decode, the E0080 const origin gate, bijection tests
//! (L07), and the `ColourTrit8::tags` reserved-zero precedent
//! (`forge-colour-v3/src/trit.rs:43-45`) for the reserved tail.
//!
//! The `react_residual` field is an exactness residual — the exact-division
//! remainder that makes the `u8 -> pmy` conversion in `to_pair` a
//! bijection. It is *shaped* to be consumed as `forge_core_v3::LeakyPermyriad`
//! debt by a later physics tranche; nothing here integrates it (L01/L12 — an
//! unearned "decay" claim is exactly the corruption the prime law refuses).
//!
//! Cross-references in this file are deliberately plain code spans, not
//! intra-doc links: the HUD face mounts this file textually under a different
//! crate root (`xtask/src/main.rs`), where a `[`link`]` to a sibling item or to
//! `forge-core-v3` cannot resolve and turns the doc gate red.

#[cfg(feature = "sky-mount")]
use crate::forge_core_v3::vixel_automata::{
    FLAG_ALIVE, FLAG_BURNING, FLAG_FLAMMABLE, FLAG_FLUID, FLAG_LIT, FLAG_UI,
};
#[cfg(not(feature = "sky-mount"))]
use forge_core_v3::vixel_automata::{FLAG_ALIVE, FLAG_BURNING, FLAG_FLAMMABLE, FLAG_FLUID, FLAG_LIT, FLAG_UI};

/// Full scale for every permyriad channel on this word: `0..=10_000` maps
/// `0.0..=1.0`. Same ceiling as `MaterialTrit8::FILL_MAX` and
/// `NormalAlbedo8::PMY_MAX`, restated here because this word has no other
/// home to import it from without pulling in an unrelated crate for one
/// constant.
pub const SEAM_PMY_MAX: u16 = 10_000;

/// Highest valid `react_residual`: `255` is refused — see `to_pair`.
pub const REACT_RESIDUAL_MAX: u8 = 254;

/// The union of every known `vixel_automata` flag bit, imported rather than
/// restated (L05). A `flags` byte with a bit outside this mask has no known
/// reading and is refused.
const KNOWN_FLAG_MASK: u8 = (FLAG_ALIVE | FLAG_LIT | FLAG_FLUID | FLAG_FLAMMABLE | FLAG_BURNING | FLAG_UI) as u8;

/// How many reserved tail bytes `ReactionSeam16` carries: `16 - (2+2+2+2+1+1)
/// == 6`.
const RESERVED_LEN: usize = 6;

/// Encode one reagent byte as an exact `(react_pmy, react_residual)` pair.
/// `prod = b * 10_000` is exact in `u32` for every `b in 0..=255`
/// (`255 * 10_000 == 2_550_000`, well under `u32::MAX`); dividing by `255`
/// gives the permyriad quotient and `react_residual` is the remainder that
/// makes [`from_pair`] its exact inverse.
///
/// Verified this session for all 256 bytes: 0 failures, max residual 250,
/// `react_pmy <= 10_000` always. Worked rows: `0 -> (0, 0)`,
/// `128 -> (5_019, 155)`, `255 -> (10_000, 0)`.
#[inline(always)]
pub const fn to_pair(b: u8) -> (u16, u8) {
    let prod: u32 = b as u32 * 10_000;
    let react_pmy = (prod / 255) as u16;
    let react_residual = (prod % 255) as u8;
    (react_pmy, react_residual)
}

/// Decode a `(react_pmy, react_residual)` pair back to the reagent byte it
/// came from. `None` when the pair is not reachable from any byte: the
/// reconstructed product must divide `10_000` (wait — must divide by
/// `10_000` cleanly, i.e. `q * 255 + r` must be a multiple of `10_000`) and
/// the resulting byte must fit `u8`.
#[inline(always)]
pub const fn from_pair(q: u16, r: u8) -> Option<u8> {
    let prod = q as u32 * 255 + r as u32;
    if prod % 10_000 != 0 {
        return None;
    }
    let b = prod / 10_000;
    if b > 255 {
        return None;
    }
    Some(b as u8)
}

/// What crosses the boundary between two material layers, 16 bytes, exact:
/// thermal and electrical conductance, the reagent residual pair, a leak
/// channel feeding `forge_core_v3::LeakyPermyriad`, the `vixel_automata`
/// flag byte, and a zero reserved tail. Field order is offset order — every
/// byte is a field, no padding hole. The offsets below are locked by rustc,
/// not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReactionSeam16 {
    /// Thermal conductance in permyriad, `0..=SEAM_PMY_MAX`.
    pub conduct_therm_pmy: u16,
    /// Electrical conductance in permyriad, `0..=SEAM_PMY_MAX`.
    pub conduct_elec_pmy: u16,
    /// The reagent quotient half of the residual pair — see `to_pair`.
    pub react_pmy: u16,
    /// Feeds `forge_core_v3::LeakyPermyriad::new`'s `leak` parameter,
    /// `0..=SEAM_PMY_MAX`.
    pub leak_pmy: u16,
    /// The reagent remainder half of the residual pair, `0..=REACT_RESIDUAL_MAX`
    /// — an exactness residual, not a decay term (see module docs).
    pub react_residual: u8,
    /// `vixel_automata` flag bits (`FLAG_ALIVE` etc, imported), restricted to
    /// `KNOWN_FLAG_MASK`.
    pub flags: u8,
    /// Zero until a later tranche defines these bytes — a nonzero reserved
    /// byte today is corruption, never forward-compatibility.
    pub reserved: [u8; RESERVED_LEN],
}

impl ReactionSeam16 {
    /// The origin: no conductance, no reaction, no leak, no flags, zero
    /// reserved tail.
    pub const NONE: Self = Self {
        conduct_therm_pmy: 0,
        conduct_elec_pmy: 0,
        react_pmy: 0,
        leak_pmy: 0,
        react_residual: 0,
        flags: 0,
        reserved: [0; RESERVED_LEN],
    };

    /// True when every channel is inside its domain: every permyriad channel
    /// `<= SEAM_PMY_MAX`, `react_residual <= REACT_RESIDUAL_MAX`, `flags`
    /// has no bit outside `KNOWN_FLAG_MASK`, and every reserved byte is zero.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.conduct_therm_pmy <= SEAM_PMY_MAX
            && self.conduct_elec_pmy <= SEAM_PMY_MAX
            && self.react_pmy <= SEAM_PMY_MAX
            && self.leak_pmy <= SEAM_PMY_MAX
            && self.react_residual <= REACT_RESIDUAL_MAX
            && (self.flags & !KNOWN_FLAG_MASK) == 0
            && self.reserved[0] == 0
            && self.reserved[1] == 0
            && self.reserved[2] == 0
            && self.reserved[3] == 0
            && self.reserved[4] == 0
            && self.reserved[5] == 0
    }

    /// Pack into two little-endian u64 words. Byte layout is the struct
    /// layout: word 0 is the four permyriad channels; word 1 is
    /// `react_residual`, `flags`, then the reserved tail.
    #[inline(always)]
    pub const fn encode(self) -> [u64; 2] {
        let w0 = self.conduct_therm_pmy as u64
            | (self.conduct_elec_pmy as u64) << 16
            | (self.react_pmy as u64) << 32
            | (self.leak_pmy as u64) << 48;
        let mut w1 = self.react_residual as u64 | (self.flags as u64) << 8;
        let mut i = 0;
        while i < RESERVED_LEN {
            w1 |= (self.reserved[i] as u64) << (16 + i * 8);
            i += 1;
        }
        [w0, w1]
    }

    /// Unpack two words. `None` for anything outside the valid domain —
    /// corruption refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(words: [u64; 2]) -> Option<Self> {
        let w0 = words[0];
        let w1 = words[1];
        let mut reserved = [0u8; RESERVED_LEN];
        let mut i = 0;
        while i < RESERVED_LEN {
            reserved[i] = (w1 >> (16 + i * 8)) as u8;
            i += 1;
        }
        let c = Self {
            conduct_therm_pmy: w0 as u16,
            conduct_elec_pmy: (w0 >> 16) as u16,
            react_pmy: (w0 >> 32) as u16,
            leak_pmy: (w0 >> 48) as u16,
            react_residual: w1 as u8,
            flags: (w1 >> 8) as u8,
            reserved,
        };
        if c.is_valid() {
            Some(c)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<ReactionSeam16>() == 16);
const _: () = assert!(core::mem::align_of::<ReactionSeam16>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping fields keeps size 16 while
// silently reinterpreting every stored word.
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, conduct_therm_pmy) == 0);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, conduct_elec_pmy) == 2);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, react_pmy) == 4);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, leak_pmy) == 6);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, react_residual) == 8);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, flags) == 9);
const _: () = assert!(core::mem::offset_of!(ReactionSeam16, reserved) == 10);

// Every one of the 16 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 2 + 2 + 1 + 1 + RESERVED_LEN == core::mem::size_of::<ReactionSeam16>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin, and
// the reagent pair codec's own origin (byte 0) round-trips.
const _: () = {
    match ReactionSeam16::decode(ReactionSeam16::NONE.encode()) {
        Some(w) => {
            assert!(w.conduct_therm_pmy == 0 && w.react_residual == 0 && w.flags == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
    let (q, r) = to_pair(0);
    assert!(q == 0 && r == 0, "byte 0 did not encode to (0, 0)");
    match from_pair(q, r) {
        Some(b) => assert!(b == 0, "the reagent pair origin did not survive its own wire"),
        None => panic!("the reagent pair origin failed to decode"),
    }
    // The three worked rows this brief was verified against, asserted verbatim.
    let (q128, r128) = to_pair(128);
    assert!(q128 == 5_019 && r128 == 155, "byte 128 did not match the verified worked row");
    let (q255, r255) = to_pair(255);
    assert!(q255 == 10_000 && r255 == 0, "byte 255 did not match the verified worked row");
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: every lattice word sample survives its own
    /// wire exactly.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        for therm in [1u16, 2_500, 5_000, 9_999] {
            for elec in [1u16, 3_333, 6_666, 9_999] {
                for react in [1u16, 4_000, 8_000, 9_999] {
                    for leak in [1u16, 2_000, 7_000, 9_999] {
                        let w = ReactionSeam16 {
                            conduct_therm_pmy: therm,
                            conduct_elec_pmy: elec,
                            react_pmy: react,
                            leak_pmy: leak,
                            react_residual: 100,
                            flags: FLAG_ALIVE as u8 | FLAG_LIT as u8,
                            reserved: [0; RESERVED_LEN],
                        };
                        assert_eq!(ReactionSeam16::decode(w.encode()), Some(w));
                    }
                }
            }
        }
    }

    /// L07 over the sentinels: 0 and SEAM_PMY_MAX on every permyriad
    /// channel, 0 and REACT_RESIDUAL_MAX on the residual byte.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for pmy in [0u16, SEAM_PMY_MAX] {
            for residual in [0u8, REACT_RESIDUAL_MAX] {
                let w = ReactionSeam16 {
                    conduct_therm_pmy: pmy,
                    conduct_elec_pmy: pmy,
                    react_pmy: pmy,
                    leak_pmy: pmy,
                    react_residual: residual,
                    flags: KNOWN_FLAG_MASK,
                    reserved: [0; RESERVED_LEN],
                };
                assert_eq!(ReactionSeam16::decode(w.encode()), Some(w));
            }
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = ReactionSeam16 {
            conduct_therm_pmy: 5_000,
            conduct_elec_pmy: 5_000,
            react_pmy: 5_000,
            leak_pmy: 5_000,
            react_residual: 10,
            flags: FLAG_ALIVE as u8,
            reserved: [0; RESERVED_LEN],
        };
        assert!(ReactionSeam16::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            ReactionSeam16 { conduct_therm_pmy: SEAM_PMY_MAX + 1, ..good },
            ReactionSeam16 { conduct_elec_pmy: SEAM_PMY_MAX + 1, ..good },
            ReactionSeam16 { react_pmy: SEAM_PMY_MAX + 1, ..good },
            ReactionSeam16 { leak_pmy: SEAM_PMY_MAX + 1, ..good },
            ReactionSeam16 { react_residual: 255, ..good },
            ReactionSeam16 { flags: 0x40, ..good },
            ReactionSeam16 { reserved: [1, 0, 0, 0, 0, 0], ..good },
            ReactionSeam16 { reserved: [0, 0, 0, 0, 0, 1], ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(ReactionSeam16::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }

    /// The origin: `NONE` round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        let w = ReactionSeam16::NONE;
        assert_eq!(ReactionSeam16::decode(w.encode()), Some(w));
    }

    /// The reagent residual pair is a bijection over all 256 bytes:
    /// `from_pair(to_pair(b)) == Some(b)`, `react_pmy <= 10_000` and
    /// `react_residual <= REACT_RESIDUAL_MAX` always.
    #[test]
    fn reagent_pair_bijection_holds_over_every_byte() {
        let mut max_residual = 0u8;
        for b in 0u16..=255 {
            let b = b as u8;
            let (q, r) = to_pair(b);
            assert!(q <= 10_000, "byte {b} produced react_pmy {q} above 10_000");
            assert!(r <= REACT_RESIDUAL_MAX, "byte {b} produced residual {r} above {REACT_RESIDUAL_MAX}");
            assert_eq!(from_pair(q, r), Some(b), "byte {b} did not survive to_pair-then-from_pair");
            max_residual = max_residual.max(r);
        }
        assert_eq!(max_residual, 250, "the verified max residual over all 256 bytes was 250");
    }

    /// The three worked rows this brief was verified against.
    #[test]
    fn reagent_pair_worked_rows() {
        assert_eq!(to_pair(0), (0, 0));
        assert_eq!(to_pair(128), (5_019, 155));
        assert_eq!(to_pair(255), (10_000, 0));
    }

    /// `from_pair` refuses an unreachable pair: a `(q, r)` whose reconstructed
    /// product is not a multiple of 10_000, and one whose byte would exceed
    /// `u8::MAX`.
    #[test]
    fn from_pair_refuses_unreachable_pairs() {
        assert_eq!(from_pair(1, 0), None, "q=1,r=0 has product 255, not a multiple of 10_000");
        // q=10_039, r=55: product 2_560_000 is an exact multiple of 10_000,
        // but the reconstructed byte is 256 — past u8::MAX.
        assert_eq!(10_039u32 * 255 + 55, 2_560_000, "fixture arithmetic check");
        assert_eq!(from_pair(10_039, 55), None, "the reconstructed byte would exceed u8::MAX");
    }
}
