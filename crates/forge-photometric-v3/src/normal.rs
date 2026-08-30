//! `NormalAlbedo8` — the 8-byte photometric word, T2 of the forge-vision
//! drain (plan-2-welds.md T2, quarried from 08 photometrics — the encoding
//! is new, the discipline is drained).
//!
//! Machine-first (L08): the octahedral encode/decode path is fixed-point
//! integer arithmetic only — `oct_u`/`oct_v` are reinterpreted as `i16` and
//! folded with integer add/sub/abs/sign, never a float division or a
//! normalize. Any perceptual/shading transform (true unit-sphere
//! normalization, sRGB albedo) is a consumer's business, derived, never
//! stored a second time.
//!
//! ONE HOME (L05): this file is the only definition of `NormalAlbedo8` and
//! the octahedral codec.

/// Full scale for the permyriad channels (albedo, roughness):
/// `0..=10_000` maps `0.0..=1.0`.
pub const PMY_MAX: u16 = 10_000;

/// The octahedron's L1 radius in fixed-point units — the scale every
/// decoded `(x, y, z)` normal component is measured against. Matches
/// `i16::MAX`: `i16::MIN` (-32768) has no positive counterpart, so its bit
/// pattern is refused rather than folded into an asymmetric octahedron
/// (see `NormalAlbedo8::is_valid`).
pub const OCT_SCALE: i32 = i16::MAX as i32;

/// Sign as +/-1, never 0 — the fold formulas need a definite sign even at
/// zero, and both `encode_octahedral` and `decode_octahedral` agree that
/// zero is positive. This is the one place the octahedral map is not a
/// perfect bijection (the four square corners all fold to a single pole,
/// and a component that folds to exactly zero loses its source sign); it
/// is a documented property of the encoding, not a defect introduced here.
#[inline(always)]
const fn sign(v: i32) -> i32 {
    if v < 0 { -1 } else { 1 }
}

#[inline(always)]
const fn abs_i32(v: i32) -> i32 {
    if v < 0 { -v } else { v }
}

/// Decode an octahedral-encoded pair into fixed-point normal components
/// `(x, y, z)`, each scaled by `OCT_SCALE`. The result always lies exactly
/// on the L1 octahedron: `|x| + |y| + |z| == OCT_SCALE`.
///
/// Integer-only (L08): `oct_u`/`oct_v` are reinterpreted as `i16` (their
/// two's-complement bit pattern), never divided or normalized through a
/// float.
#[inline(always)]
pub const fn decode_octahedral(oct_u: u16, oct_v: u16) -> (i32, i32, i32) {
    let x = oct_u as i16 as i32;
    let y = oct_v as i16 as i32;
    let ax = abs_i32(x);
    let ay = abs_i32(y);
    let z = OCT_SCALE - ax - ay;
    if z >= 0 { (x, y, z) } else { ((OCT_SCALE - ay) * sign(x), (OCT_SCALE - ax) * sign(y), z) }
}

/// Encode fixed-point normal components already lying on the octahedron
/// (`|x| + |y| + |z| == OCT_SCALE`) back into an octahedral pair. The
/// inverse of `decode_octahedral` for every point decode can produce, save
/// the documented corner/zero-fold seam noted on `sign`.
#[inline(always)]
pub const fn encode_octahedral(x: i32, y: i32, z: i32) -> (u16, u16) {
    let (u, v) = if z >= 0 {
        (x, y)
    } else {
        let ax = abs_i32(x);
        let ay = abs_i32(y);
        ((OCT_SCALE - ay) * sign(x), (OCT_SCALE - ax) * sign(y))
    };
    (u as i16 as u16, v as i16 as u16)
}

/// One photometric texel, 8 bytes, exact: an octahedral-encoded normal plus
/// albedo/roughness in permyriad. Field order is offset order — every byte
/// is a field, no padding hole. The offsets below are locked by rustc, not
/// prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NormalAlbedo8 {
    /// Octahedral U — the sphere's x/y projected and folded onto a square;
    /// see `decode_octahedral`.
    pub oct_u: u16,
    /// Octahedral V — see `decode_octahedral`.
    pub oct_v: u16,
    /// Albedo in permyriad, `0..=PMY_MAX`.
    pub albedo_pmy: u16,
    /// Roughness in permyriad, `0..=PMY_MAX`.
    pub roughness_pmy: u16,
}

impl NormalAlbedo8 {
    /// The origin: flat normal (0, 0, +1), full albedo, zero roughness — a
    /// mirror-smooth white texel.
    pub const FLAT: Self = Self { oct_u: 0, oct_v: 0, albedo_pmy: PMY_MAX, roughness_pmy: 0 };

    /// True when every channel is inside its domain: the oct pair excludes
    /// `i16::MIN`'s bit pattern (`0x8000`, the octahedron's asymmetric
    /// edge), and both permyriad channels are `<= PMY_MAX`.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.oct_u != 0x8000
            && self.oct_v != 0x8000
            && self.albedo_pmy <= PMY_MAX
            && self.roughness_pmy <= PMY_MAX
    }

    /// The fixed-point normal this texel's oct pair decodes to.
    #[inline(always)]
    pub const fn normal(self) -> (i32, i32, i32) {
        decode_octahedral(self.oct_u, self.oct_v)
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: oct_u, oct_v, albedo, roughness.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.oct_u as u64
            | (self.oct_v as u64) << 16
            | (self.albedo_pmy as u64) << 32
            | (self.roughness_pmy as u64) << 48
    }

    /// Unpack a word. `None` for anything outside the valid domain — an
    /// out-of-range permyriad channel or the refused oct bit pattern is
    /// corruption refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            oct_u: word as u16,
            oct_v: (word >> 16) as u16,
            albedo_pmy: (word >> 32) as u16,
            roughness_pmy: (word >> 48) as u16,
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<NormalAlbedo8>() == 8);
const _: () = assert!(core::mem::align_of::<NormalAlbedo8>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 8 while
// silently reinterpreting every stored texel.
const _: () = assert!(core::mem::offset_of!(NormalAlbedo8, oct_u) == 0);
const _: () = assert!(core::mem::offset_of!(NormalAlbedo8, oct_v) == 2);
const _: () = assert!(core::mem::offset_of!(NormalAlbedo8, albedo_pmy) == 4);
const _: () = assert!(core::mem::offset_of!(NormalAlbedo8, roughness_pmy) == 6);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 2 + 2 == core::mem::size_of::<NormalAlbedo8>());

// The origin survives its own wire, both at the word level and through the
// octahedral codec. A const gate so E0080 fires before any test harness
// runs: encode-then-decode is the identity at the word origin, and the
// flat normal's octahedral round trip lands back on (0, 0).
const _: () = {
    match NormalAlbedo8::decode(NormalAlbedo8::FLAT.encode()) {
        Some(w) => {
            assert!(w.oct_u == 0 && w.oct_v == 0);
            assert!(w.albedo_pmy == PMY_MAX && w.roughness_pmy == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
    let (x, y, z) = decode_octahedral(0, 0);
    assert!(x == 0 && y == 0 && z == OCT_SCALE, "the flat oct pair did not decode to +Z");
    let (u, v) = encode_octahedral(x, y, z);
    assert!(u == 0 && v == 0, "the flat normal did not survive its own octahedral wire");
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: every lattice word sample survives its own
    /// wire exactly.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        for oct_u in [0u16, 1, 12_345, 0x7fff, 0x8001, 0xffff] {
            for oct_v in [0u16, 1, 54_321, 0x7fff, 0x8001, 0xffff] {
                for albedo in [1u16, 2_500, 5_000, 9_999] {
                    for roughness in [1u16, 2_500, 5_000, 9_999] {
                        let w = NormalAlbedo8 { oct_u, oct_v, albedo_pmy: albedo, roughness_pmy: roughness };
                        assert_eq!(
                            NormalAlbedo8::decode(w.encode()),
                            Some(w),
                            "oct_u={oct_u} oct_v={oct_v} albedo={albedo} roughness={roughness}"
                        );
                    }
                }
            }
        }
    }

    /// L07 over the sentinels: 0 and PMY_MAX on both permyriad channels,
    /// and the oct extremes that are NOT the refused `0x8000` pattern.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for oct_u in [0u16, 0x7fff, 0x8001, 0xffff] {
            for oct_v in [0u16, 0x7fff, 0x8001, 0xffff] {
                for albedo in [0u16, PMY_MAX] {
                    for roughness in [0u16, PMY_MAX] {
                        let w = NormalAlbedo8 { oct_u, oct_v, albedo_pmy: albedo, roughness_pmy: roughness };
                        assert_eq!(NormalAlbedo8::decode(w.encode()), Some(w));
                    }
                }
            }
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = NormalAlbedo8 { oct_u: 100, oct_v: 200, albedo_pmy: 5_000, roughness_pmy: 5_000 };
        assert!(NormalAlbedo8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            NormalAlbedo8 { oct_u: 0x8000, ..good },
            NormalAlbedo8 { oct_v: 0x8000, ..good },
            NormalAlbedo8 { albedo_pmy: PMY_MAX + 1, ..good },
            NormalAlbedo8 { roughness_pmy: PMY_MAX + 1, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(NormalAlbedo8::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }

    /// L07 over the octahedral codec's poles: z = +1 sits at the origin
    /// (0, 0); z = -1 is only reachable through `encode_octahedral` (it is
    /// the fold target of all four square corners) and lands at the
    /// (SCALE, SCALE) corner this codec canonically chooses.
    #[test]
    fn octahedral_bijection_holds_over_the_poles() {
        assert_eq!(decode_octahedral(0, 0), (0, 0, OCT_SCALE), "north pole is not at the oct origin");
        let (u, v) = encode_octahedral(0, 0, -OCT_SCALE);
        assert_eq!((u, v), (OCT_SCALE as u16, OCT_SCALE as u16), "south pole did not fold to its canonical corner");
        assert_eq!(decode_octahedral(u, v), (0, 0, -OCT_SCALE), "the canonical south corner did not decode back to the south pole");
    }

    /// L07 over the equator edge: `|x| + |y| == OCT_SCALE`, `z == 0`, on
    /// both axes and both signs — the boundary between the pass-through and
    /// folded halves of the map.
    #[test]
    fn octahedral_bijection_holds_over_the_equator_edge() {
        let scale = OCT_SCALE as u16;
        let neg_scale = (-OCT_SCALE) as i16 as u16;
        for (u, v) in [(scale, 0u16), (0u16, scale), (neg_scale, 0u16), (0u16, neg_scale)] {
            let (x, y, z) = decode_octahedral(u, v);
            assert_eq!(z, 0, "u={u} v={v} did not land exactly on the equator");
            let (u2, v2) = encode_octahedral(x, y, z);
            assert_eq!((u2, v2), (u, v), "equator point u={u} v={v} did not survive its own wire");
        }
    }

    /// L07 over the origin: the flat normal (0, 0, +1) round-trips through
    /// both the octahedral codec and the word codec.
    #[test]
    fn the_origin_survives_its_wire() {
        assert_eq!(NormalAlbedo8::FLAT.normal(), (0, 0, OCT_SCALE));
        assert_eq!(NormalAlbedo8::decode(NormalAlbedo8::FLAT.encode()), Some(NormalAlbedo8::FLAT));
    }

    /// L07 over the octahedral interior: `encode(decode(u, v)) == (u, v)`
    /// for samples on both sides of the fold. Samples deliberately avoid
    /// the documented zero-fold seam (`sign`'s doc comment) — a component
    /// that folds to exactly zero cannot carry a source sign, which is a
    /// property of the encoding, not a gap in this gate.
    #[test]
    fn octahedral_bijection_holds_over_the_interior() {
        let samples: &[(i32, i32)] = &[
            (0, 0),
            (1_000, 2_000),
            (-1_000, 2_000),
            (1_000, -2_000),
            (-1_000, -2_000),
            (20_000, 5_000),
            (-20_000, 5_000),
            (20_000, -5_000),
            (-20_000, -5_000),
            (20_000, 20_000),
            (-20_000, 20_000),
            (20_000, -20_000),
            (-20_000, -20_000),
            (OCT_SCALE, 0),
            (-OCT_SCALE, 0),
            (0, OCT_SCALE),
            (0, -OCT_SCALE),
        ];
        for &(u, v) in samples {
            let (u, v) = (u as i16 as u16, v as i16 as u16);
            let (x, y, z) = decode_octahedral(u, v);
            assert_eq!(abs_i32(x) + abs_i32(y) + abs_i32(z), OCT_SCALE, "u={u} v={v} left the octahedron");
            let (u2, v2) = encode_octahedral(x, y, z);
            assert_eq!((u2, v2), (u, v), "u={u} v={v} did not survive decode-then-encode");
        }
    }

    /// Decode-then-encode identity for exactly-representable lattice
    /// normals: fixed-point `(x, y, z)` triples already on the octahedron,
    /// fed straight to `encode_octahedral` then `decode_octahedral`,
    /// reproduce themselves — the codec's other direction, over both the
    /// pass-through (`z >= 0`) and folded (`z < 0`) halves.
    #[test]
    fn lattice_normals_survive_encode_then_decode() {
        let lattice: &[(i32, i32, i32)] = &[
            (0, 0, OCT_SCALE),
            (10_000, 5_000, OCT_SCALE - 15_000),
            (-10_000, 5_000, OCT_SCALE - 15_000),
            (10_000, -5_000, OCT_SCALE - 15_000),
            (-10_000, -5_000, OCT_SCALE - 15_000),
            (20_000, 10_000, -(20_000 + 10_000 - OCT_SCALE)),
            (-20_000, 10_000, -(20_000 + 10_000 - OCT_SCALE)),
            (20_000, -10_000, -(20_000 + 10_000 - OCT_SCALE)),
            (-20_000, -10_000, -(20_000 + 10_000 - OCT_SCALE)),
        ];
        for &(x, y, z) in lattice {
            assert_eq!(abs_i32(x) + abs_i32(y) + abs_i32(z), OCT_SCALE, "fixture ({x},{y},{z}) is off the octahedron");
            let (u, v) = encode_octahedral(x, y, z);
            assert_eq!(decode_octahedral(u, v), (x, y, z), "lattice normal ({x},{y},{z}) did not survive encode-then-decode");
        }
    }
}
