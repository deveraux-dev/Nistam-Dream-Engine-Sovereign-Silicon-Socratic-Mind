//! `Morton8` — the 8-byte 5-axis Z-order code, T4 of the forge-vision drain
//! (plan-2-welds.md T4, quarried from 05-poll5d.md's 12-bit/axis Morton
//! scheme — the encoding is new, the discipline is drained).
//!
//! RECEIPT RESOLVED (2026-08-18): the quarry file was recovered at
//! `F:\v3\TODO\quarry-sort\MAGICWORD-MOTION-2026-08-17\poll5d-v2\spatial.rs`
//! and CONFIRMS the brief's account — legacy `AXIS_BITS = 12` (spatial.rs:23,
//! 60-bit words; that layout lives on as core's `MortonKey5D`). This crate's
//! 10-bit/axis form remains the deliberate renegotiation the brief described
//! (50 bits <= 64 with headroom), not an error. The recovered file also holds
//! NO range-splitting/k-NN acceleration (brute-force knn over a ring buffer,
//! heterogeneous distance gauge) — nothing further to port from it.
//!
//! Machine-first (L08): bit-interleave (spread/compact) is integer
//! shift-and-mask only, no float anywhere on the path.
//!
//! ONE HOME (L05): this file is the only definition of `Morton8`.

/// Bits of quantization per axis.
pub const AXIS_BITS: u32 = 10;

/// Inclusive per-axis ceiling: `2^10 - 1 == 1023`.
pub const AXIS_MAX: u16 = (1u16 << AXIS_BITS) - 1;

/// How many axes this Z-order code interleaves.
pub const MORTON_AXES: u32 = 5;

/// Spread a 10-bit value so bit `i` lands at bit `i * MORTON_AXES` — the
/// per-axis stride that leaves room for the other four axes' bits between
/// each of this axis's own bits.
#[inline(always)]
const fn spread_bits(v: u16) -> u64 {
    let mut out: u64 = 0;
    let mut i = 0;
    while i < AXIS_BITS {
        let bit = ((v >> i) & 1) as u64;
        out |= bit << (i * MORTON_AXES);
        i += 1;
    }
    out
}

/// The inverse of `spread_bits`: gather bit `i * MORTON_AXES` of `word`
/// back into bit `i` of a 10-bit value.
#[inline(always)]
const fn compact_bits(word: u64) -> u16 {
    let mut out: u16 = 0;
    let mut i = 0;
    while i < AXIS_BITS {
        let bit = ((word >> (i * MORTON_AXES)) & 1) as u16;
        out |= bit << i;
        i += 1;
    }
    out
}

/// One 5D Z-order code, 8 bytes, exact: five 10-bit axis values (x, y, z,
/// t, s) interleaved into 50 bits, with 14 spare bits held at zero. A
/// `#[repr(transparent)]` newtype over the packed word — five separate
/// `u16` fields would not fit 8 bytes (`5 * 2 == 10`), and the packed word
/// *is* the point of a Morton code: adjacent points in the same octant
/// share a bit prefix in the word itself.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Morton8(pub u64);

impl Morton8 {
    /// The origin: the domain's mid-point on every axis (brief line 41,
    /// "Mid-point origin" — the poll5d coordinate space is centred here,
    /// not at the quantized zero corner).
    pub const ORIGIN: Self = match Self::encode(512, 512, 512, 512, 512) {
        Some(m) => m,
        None => panic!("the mid-point origin is out of the 10-bit axis domain"),
    };

    /// True when the 14 spare bits (50..64) are zero. `decode` reads every
    /// word (compacting always yields an in-domain 10-bit value per axis);
    /// `from_word` is the checked constructor that refuses a live spare bit.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        (self.0 >> (AXIS_BITS * MORTON_AXES)) == 0
    }

    /// Interleave five axis values into a Morton8 code. `None` when any
    /// axis exceeds `AXIS_MAX` — corruption refused at the boundary, not
    /// clamped past it.
    #[inline(always)]
    pub const fn encode(x: u16, y: u16, z: u16, t: u16, s: u16) -> Option<Self> {
        if x > AXIS_MAX || y > AXIS_MAX || z > AXIS_MAX || t > AXIS_MAX || s > AXIS_MAX {
            return None;
        }
        let word = spread_bits(x)
            | (spread_bits(y) << 1)
            | (spread_bits(z) << 2)
            | (spread_bits(t) << 3)
            | (spread_bits(s) << 4);
        Some(Self(word))
    }

    /// De-interleave back into the five axis values. Always well-defined:
    /// `compact_bits` only ever produces a 10-bit value, so every `Morton8`
    /// (valid or not) decodes to an in-domain axis tuple; the spare bits,
    /// if live, are simply not part of any axis.
    #[inline(always)]
    pub const fn decode(self) -> (u16, u16, u16, u16, u16) {
        (
            compact_bits(self.0),
            compact_bits(self.0 >> 1),
            compact_bits(self.0 >> 2),
            compact_bits(self.0 >> 3),
            compact_bits(self.0 >> 4),
        )
    }

    /// The checked constructor from a raw word. `None` when a spare bit
    /// (50..64) is live — corruption refused at the boundary, not silently
    /// carried through decode.
    #[inline(always)]
    pub const fn from_word(word: u64) -> Option<Self> {
        let m = Self(word);
        if m.is_valid() { Some(m) } else { None }
    }

    /// The raw packed word.
    #[inline(always)]
    pub const fn word(self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<Morton8>() == 8);
const _: () = assert!(core::mem::align_of::<Morton8>() == 8);
const _: () = assert!(core::mem::offset_of!(Morton8, 0) == 0);

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the mid-point
// origin, and the origin word carries no live spare bit.
const _: () = {
    assert!(Morton8::ORIGIN.is_valid(), "the origin word left a spare bit live");
    let (x, y, z, t, s) = Morton8::ORIGIN.decode();
    assert!(x == 512 && y == 512 && z == 512 && t == 512 && s == 512, "the origin did not decode to its own axes");
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: the brief's named sample survives an
    /// encode-then-decode round trip exactly (brief line 39).
    #[test]
    fn bijection_holds_over_the_interior_sample() {
        let m = Morton8::encode(128, 256, 512, 768, 256).expect("interior sample refused");
        assert_eq!(m.decode(), (128, 256, 512, 768, 256));
        assert!(m.is_valid());
    }

    /// L07 over the min/max edges: the all-zero corner and the all-max
    /// corner both survive the wire (brief line 40).
    #[test]
    fn bijection_holds_over_the_min_max_edges() {
        let min = Morton8::encode(0, 0, 0, 0, 0).expect("min edge refused");
        assert_eq!(min.decode(), (0, 0, 0, 0, 0));
        assert_eq!(min.word(), 0);

        let max = Morton8::encode(AXIS_MAX, AXIS_MAX, AXIS_MAX, AXIS_MAX, AXIS_MAX).expect("max edge refused");
        assert_eq!(max.decode(), (AXIS_MAX, AXIS_MAX, AXIS_MAX, AXIS_MAX, AXIS_MAX));
        assert_eq!(max.word(), (1u64 << (AXIS_BITS * MORTON_AXES)) - 1, "max edge did not fill exactly 50 bits");
    }

    /// L07 over the mid-point origin (brief line 41): the origin constant
    /// round-trips and matches a fresh encode of the same axes.
    #[test]
    fn bijection_holds_over_the_midpoint_origin() {
        assert_eq!(Morton8::ORIGIN.decode(), (512, 512, 512, 512, 512));
        assert_eq!(Morton8::encode(512, 512, 512, 512, 512), Some(Morton8::ORIGIN));
    }

    /// L07 over the full lattice: every axis independently sweeps its
    /// domain (0, 1, mid, max-1, max) while the others sit at the origin's
    /// mid-point, and every combination survives the wire.
    #[test]
    fn bijection_holds_over_the_lattice() {
        let samples = [0u16, 1, 512, AXIS_MAX - 1, AXIS_MAX];
        for axis in 0..5 {
            for &v in &samples {
                let mut coords = [512u16; 5];
                coords[axis] = v;
                let [x, y, z, t, s] = coords;
                let m = Morton8::encode(x, y, z, t, s).expect("in-domain axis sweep refused");
                assert_eq!(m.decode(), (x, y, z, t, s), "axis={axis} v={v}");
                assert_eq!(Morton8::from_word(m.word()), Some(m));
            }
        }
    }

    /// Z-order locality: two points differing only in axis 0's least
    /// significant bit differ in the packed word only at bit 0 — the
    /// defining property of the interleave (brief line 42, resolved
    /// concretely rather than left as a vague "shares a prefix" claim).
    #[test]
    fn adjacent_points_differ_only_in_the_interleaved_low_bit() {
        let a = Morton8::encode(0, 0, 0, 0, 0).unwrap();
        let b = Morton8::encode(1, 0, 0, 0, 0).unwrap();
        assert_eq!(a.word() ^ b.word(), 1, "x's low bit did not land at word bit 0");

        // Same property holds for each axis's own stride offset.
        let axes: [(u16, u16, u16, u16, u16); 5] = [
            (1, 0, 0, 0, 0),
            (0, 1, 0, 0, 0),
            (0, 0, 1, 0, 0),
            (0, 0, 0, 1, 0),
            (0, 0, 0, 0, 1),
        ];
        for (i, &(x, y, z, t, s)) in axes.iter().enumerate() {
            let m = Morton8::encode(x, y, z, t, s).unwrap();
            assert_eq!(m.word(), 1u64 << i, "axis {i}'s single low bit did not land at word bit {i}");
        }
    }

    /// The boundary refuses corruption: an above-`AXIS_MAX` value on any
    /// axis is refused, and a word with a live spare bit is refused by the
    /// checked constructor.
    #[test]
    fn out_of_domain_values_are_refused() {
        let over = AXIS_MAX + 1;
        assert_eq!(Morton8::encode(over, 0, 0, 0, 0), None, "x over AXIS_MAX was accepted");
        assert_eq!(Morton8::encode(0, over, 0, 0, 0), None, "y over AXIS_MAX was accepted");
        assert_eq!(Morton8::encode(0, 0, over, 0, 0), None, "z over AXIS_MAX was accepted");
        assert_eq!(Morton8::encode(0, 0, 0, over, 0), None, "t over AXIS_MAX was accepted");
        assert_eq!(Morton8::encode(0, 0, 0, 0, over), None, "s over AXIS_MAX was accepted");

        let live_spare_bit = 1u64 << (AXIS_BITS * MORTON_AXES);
        assert_eq!(Morton8::from_word(live_spare_bit), None, "a live spare bit was accepted");
    }
}
