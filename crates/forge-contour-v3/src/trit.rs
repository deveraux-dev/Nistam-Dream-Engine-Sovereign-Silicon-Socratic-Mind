//! `ContourTrit8` — the 8-byte quantized contour profile point, T5 of the
//! forge-vision drain (plan-2-welds.md T5, quarried from 07 geometry's
//! `contour.rs` — the encoding is new, the discipline is drained).
//!
//! Machine-first (L08): this word is the exact integer encoding; the
//! normalized `(f32, f32)` profile point `contour.rs:18-20` produced is
//! DERIVED from it by a consumer (see `forge_contour_v3::to_normalized`)
//! and never stored a second time.
//!
//! ONE HOME (L05): this file is the only definition of `ContourTrit8`.

/// One 2D profile point `(u, v)` in normalized `[0..1]` space, 8 bytes,
/// exact. Every byte is a field — no padding hole. The offsets below are
/// locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContourTrit8 {
    /// Normalized x-coordinate: `0..=u16::MAX` maps `0.0..=1.0`.
    pub u: u16,
    /// Normalized y-coordinate: `0..=u16::MAX` maps `0.0..=1.0`.
    pub v: u16,
    /// Reserved. Zero until claimed by a later tranche — a nonzero reserved
    /// byte today is corruption, never forward-compatibility.
    pub reserved: [u8; 4],
}

impl ContourTrit8 {
    /// The origin: `(0, 0)`, the anchor.
    pub const ORIGIN: Self = Self { u: 0, v: 0, reserved: [0; 4] };

    /// A profile point at the given quantized coordinates. Reserved is
    /// pinned to zero by construction.
    #[inline(always)]
    pub const fn new(u: u16, v: u16) -> Self {
        Self { u, v, reserved: [0; 4] }
    }

    /// True when `u == 0 && v == 0` — the anchor point.
    #[inline(always)]
    pub const fn is_origin(self) -> bool {
        self.u == 0 && self.v == 0
    }

    /// True when reserved is all-zero — the only invalid domain this word
    /// has today.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.reserved[0] == 0 && self.reserved[1] == 0 && self.reserved[2] == 0 && self.reserved[3] == 0
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: u, v, reserved.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.u as u64
            | (self.v as u64) << 16
            | (self.reserved[0] as u64) << 32
            | (self.reserved[1] as u64) << 40
            | (self.reserved[2] as u64) << 48
            | (self.reserved[3] as u64) << 56
    }

    /// Unpack a word. `None` when reserved is non-zero — corruption refused
    /// at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            u: word as u16,
            v: (word >> 16) as u16,
            reserved: [(word >> 32) as u8, (word >> 40) as u8, (word >> 48) as u8, (word >> 56) as u8],
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<ContourTrit8>() == 8);
const _: () = assert!(core::mem::align_of::<ContourTrit8>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping u and v keeps size 8 while
// silently reinterpreting every stored point.
const _: () = assert!(core::mem::offset_of!(ContourTrit8, u) == 0);
const _: () = assert!(core::mem::offset_of!(ContourTrit8, v) == 2);
const _: () = assert!(core::mem::offset_of!(ContourTrit8, reserved) == 4);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 4 == core::mem::size_of::<ContourTrit8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match ContourTrit8::decode(ContourTrit8::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.u == 0 && w.v == 0);
            assert!(w.reserved[0] == 0 && w.reserved[1] == 0 && w.reserved[2] == 0 && w.reserved[3] == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
    assert!(ContourTrit8::ORIGIN.is_origin(), "the origin constant is not its own anchor");
};

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 over the interior: multiple interior steps on both axes survive
    /// their own wire exactly.
    #[test]
    fn bijection_holds_over_the_interior() {
        for u in [1u16, 16_384, 32_768, 49_151, 65_534] {
            for v in [1u16, 16_384, 32_768, 49_151, 65_534] {
                let p = ContourTrit8::new(u, v);
                assert_eq!(ContourTrit8::decode(p.encode()), Some(p), "u={u} v={v}");
            }
        }
    }

    /// L07 over the edges: `0` and `u16::MAX` on both axes, every
    /// combination.
    #[test]
    fn bijection_holds_over_the_edges() {
        for u in [0u16, u16::MAX] {
            for v in [0u16, u16::MAX] {
                let p = ContourTrit8::new(u, v);
                assert_eq!(ContourTrit8::decode(p.encode()), Some(p), "u={u} v={v}");
            }
        }
    }

    /// The sentinel rows named in the brief: origin, both edges, the
    /// corner, and the midpoint.
    #[test]
    fn bijection_holds_over_the_sentinels() {
        let sentinels = [
            ContourTrit8::new(0, 0),
            ContourTrit8::new(u16::MAX, 0),
            ContourTrit8::new(0, u16::MAX),
            ContourTrit8::new(u16::MAX, u16::MAX),
            ContourTrit8::new(32_768, 32_768),
        ];
        for p in sentinels {
            assert_eq!(ContourTrit8::decode(p.encode()), Some(p), "u={} v={}", p.u, p.v);
        }
    }

    /// The origin: `(0, 0)` round-trips and reports itself as the anchor.
    #[test]
    fn the_origin_survives_its_wire() {
        assert!(ContourTrit8::ORIGIN.is_origin());
        assert_eq!(ContourTrit8::decode(ContourTrit8::ORIGIN.encode()), Some(ContourTrit8::ORIGIN));
        assert_eq!(ContourTrit8::new(0, 0), ContourTrit8::ORIGIN);
    }

    /// The boundary refuses corruption: any nonzero reserved byte decodes to
    /// None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = ContourTrit8::new(100, 200);
        assert!(ContourTrit8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            ContourTrit8 { reserved: [1, 0, 0, 0], ..good },
            ContourTrit8 { reserved: [0, 1, 0, 0], ..good },
            ContourTrit8 { reserved: [0, 0, 1, 0], ..good },
            ContourTrit8 { reserved: [0, 0, 0, 1], ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(ContourTrit8::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }
}
