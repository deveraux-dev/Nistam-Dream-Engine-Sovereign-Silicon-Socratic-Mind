//! `FrameHeader8` — the 8-byte frame descriptor, T3 of the forge-vision
//! drain (plan-2-welds.md T3, quarried from dxgi.rs's `RawCapture` struct —
//! the discipline is drained, the encoding is new).
//!
//! D1 exempt-hybrid (decisions.log [ASSUMED: ARCH000 ruling recorded, not
//! sighted]): raw BGRA/RGBA pixel bulk stays zero-copy binary BEHIND this
//! header; it is never re-encoded, packed, or trit-ified. This header
//! describes the bulk — it does not touch it.
//!
//! ONE HOME (L05): this file is the only definition of `FrameHeader8`.

/// One frame descriptor, 8 bytes, exact: width/height/stride in pixels or
/// bytes, and the pixel format tag. Field order is offset order — every
/// byte is a field, no padding hole. The offsets below are locked by rustc,
/// not prose.
#[repr(C, align(2))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameHeader8 {
    /// Pixel width. `0` = zero-dim sentinel, `u16::MAX` = max edge
    /// (dxgi.rs:71-76 accepts usize; stored here as a u16 sentinel).
    pub width: u16,
    /// Pixel height. Same sentinel scheme as `width`.
    pub height: u16,
    /// Bytes per row. DXGI stride = width × 4, no padding (dxgi.rs:71-76).
    pub stride: u16,
    /// Pixel format: 0 = BGRA8_UNORM, 1 = RGBA8_UNORM, 2..=255 reserved.
    pub format: u8,
    /// Padding field — zero at origin, live once a second format axis is
    /// needed.
    pub _reserved: u8,
}

impl FrameHeader8 {
    /// BGRA8_UNORM pixel format tag.
    pub const FORMAT_BGRA8_UNORM: u8 = 0;
    /// RGBA8_UNORM pixel format tag.
    pub const FORMAT_RGBA8_UNORM: u8 = 1;

    /// The origin: the zero-dim sentinel frame — no width, no height, no
    /// stride, BGRA8_UNORM format.
    pub const ZERO_DIM: Self =
        Self { width: 0, height: 0, stride: 0, format: Self::FORMAT_BGRA8_UNORM, _reserved: 0 };

    /// True when the format tag names one of the 2 live formats and the
    /// reserved lane is zero. `width`/`height`/`stride` carry no domain
    /// restriction beyond their integer width — 0 and `u16::MAX` are both
    /// named sentinels, not corruption.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.format <= Self::FORMAT_RGBA8_UNORM && self._reserved == 0
    }

    /// True when both `width` and `height` are the zero-dim sentinel.
    #[inline(always)]
    pub const fn is_zero_dim(self) -> bool {
        self.width == 0 && self.height == 0
    }

    /// Bulk pixel payload size in bytes: `height * stride`, per dxgi.rs's
    /// stride-aligned raw capture. The payload itself is never held here
    /// (D1 exempt-hybrid) — this is only the size a caller's `&[u8]`/`Vec<u8>`
    /// must carry.
    #[inline(always)]
    pub const fn payload_len(self) -> u32 {
        self.height as u32 * self.stride as u32
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: width, height, stride, format, _reserved.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.width as u64
            | (self.height as u64) << 16
            | (self.stride as u64) << 32
            | (self.format as u64) << 48
            | (self._reserved as u64) << 56
    }

    /// Unpack a word. `None` for anything outside the valid domain — an
    /// unnamed format tag or a live reserved byte is corruption refused at
    /// the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            width: word as u16,
            height: (word >> 16) as u16,
            stride: (word >> 32) as u16,
            format: (word >> 48) as u8,
            _reserved: (word >> 56) as u8,
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<FrameHeader8>() == 8);
const _: () = assert!(core::mem::align_of::<FrameHeader8>() == 2);

// OFFSET LOCKS. Size alone is weak: reordering two fields keeps size 8
// while silently reinterpreting every stored header.
const _: () = assert!(core::mem::offset_of!(FrameHeader8, width) == 0);
const _: () = assert!(core::mem::offset_of!(FrameHeader8, height) == 2);
const _: () = assert!(core::mem::offset_of!(FrameHeader8, stride) == 4);
const _: () = assert!(core::mem::offset_of!(FrameHeader8, format) == 6);
const _: () = assert!(core::mem::offset_of!(FrameHeader8, _reserved) == 7);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 2 + 1 + 1 == core::mem::size_of::<FrameHeader8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the zero-dim
// origin.
const _: () = {
    match FrameHeader8::decode(FrameHeader8::ZERO_DIM.encode()) {
        Some(w) => {
            assert!(w.width == 0 && w.height == 0 && w.stride == 0);
            assert!(w.format == FrameHeader8::FORMAT_BGRA8_UNORM && w._reserved == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 over the interior: representative width/height/stride/format
    /// combinations round-trip exactly.
    #[test]
    fn bijection_holds_over_the_interior() {
        for width in [1u16, 640, 1_920, 65_534] {
            for height in [1u16, 480, 1_080, 65_534] {
                for format in [FrameHeader8::FORMAT_BGRA8_UNORM, FrameHeader8::FORMAT_RGBA8_UNORM] {
                    let stride = width.wrapping_mul(4);
                    let h = FrameHeader8 { width, height, stride, format, _reserved: 0 };
                    assert_eq!(FrameHeader8::decode(h.encode()), Some(h), "w={width} h={height} fmt={format}");
                }
            }
        }
    }

    /// L07 over the zero-dim sentinel: `width == 0` or `height == 0`, both
    /// formats, stride pinned to 0.
    #[test]
    fn bijection_holds_over_the_zero_dim_sentinel() {
        for (width, height) in [(0u16, 0u16), (0, 1_080), (1_920, 0)] {
            for format in [FrameHeader8::FORMAT_BGRA8_UNORM, FrameHeader8::FORMAT_RGBA8_UNORM] {
                let h = FrameHeader8 { width, height, stride: 0, format, _reserved: 0 };
                assert_eq!(FrameHeader8::decode(h.encode()), Some(h), "w={width} h={height} fmt={format}");
            }
        }
    }

    /// L07 over the max-u16 edge: width/height/stride all pinned to
    /// `u16::MAX`, both formats.
    #[test]
    fn bijection_holds_over_the_max_u16_edge() {
        for format in [FrameHeader8::FORMAT_BGRA8_UNORM, FrameHeader8::FORMAT_RGBA8_UNORM] {
            let h = FrameHeader8 { width: u16::MAX, height: u16::MAX, stride: u16::MAX, format, _reserved: 0 };
            assert_eq!(FrameHeader8::decode(h.encode()), Some(h), "fmt={format}");
        }
    }

    /// The origin: `ZERO_DIM` round-trips, is valid, and reports zero-dim.
    #[test]
    fn the_origin_survives_its_wire() {
        assert!(FrameHeader8::ZERO_DIM.is_valid());
        assert!(FrameHeader8::ZERO_DIM.is_zero_dim());
        assert_eq!(FrameHeader8::decode(FrameHeader8::ZERO_DIM.encode()), Some(FrameHeader8::ZERO_DIM));
    }

    /// `payload_len` is `height * stride`, checked against the DXGI
    /// stride-aligned convention (stride = width × 4).
    #[test]
    fn payload_len_matches_height_times_stride() {
        let h = FrameHeader8 { width: 1_920, height: 1_080, stride: 1_920 * 4, format: 0, _reserved: 0 };
        assert_eq!(h.payload_len(), 1_080 * 1_920 * 4);
        assert_eq!(FrameHeader8::ZERO_DIM.payload_len(), 0);
    }

    /// The boundary refuses corruption: an unnamed format tag and a live
    /// reserved byte each decode to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = FrameHeader8 { width: 100, height: 200, stride: 400, format: 0, _reserved: 0 };
        assert!(FrameHeader8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            FrameHeader8 { format: 2, ..good },
            FrameHeader8 { format: 255, ..good },
            FrameHeader8 { _reserved: 1, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(FrameHeader8::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }
}
