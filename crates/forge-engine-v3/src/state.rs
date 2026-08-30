//! `SpineState64` — the 64-byte engine spine state word
//! (ENGINE-SPINE-BRIEF.md "SpineState64"): an `EngineTick8`, a `Morton8` 5D
//! position (`forge-poll5d-v3`), an FNV-1a digest of the consumed
//! `InputFrame64`, a packed mechanism word (scroll permyriad + pulse
//! count), and 32 reserved zero bytes.
//!
//! Machine-first (L08): `input_digest` is computed by `fnv1a`, integer
//! FNV-1a only (`offset_basis 0xcbf29ce484222325`, `prime 0x100000001b3`) —
//! never a float hash.
//!
//! ONE HOME (L05): this file is the only definition of `SpineState64`.

#[cfg(feature = "sky-mount")]
use crate::forge_poll5d_v3::Morton8;
#[cfg(not(feature = "sky-mount"))]
use forge_poll5d_v3::Morton8;

use crate::tick::EngineTick8;

/// Highest valid `scroll_pmy`: `0..=10_000` permyriad, matching the other
/// permyriad channels across the drain.
pub const SCROLL_PMY_MAX: u32 = 10_000;

/// FNV-1a 64-bit offset basis, the brief's exact constant.
pub const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime, the brief's exact constant.
pub const FNV_PRIME: u64 = 0x100000001b3;

/// FNV-1a over `bytes`, integer-only: `hash = (hash ^ byte) * FNV_PRIME`,
/// starting from `FNV_OFFSET_BASIS`.
#[inline(always)]
pub const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// Pack `scroll_pmy`/`pulse_count` into one `mech` word: `scroll_pmy` in the
/// low 32 bits, `pulse_count` in the high 32 bits. `None` when `scroll_pmy`
/// exceeds `SCROLL_PMY_MAX` — corruption refused at the boundary, not
/// clamped past it.
#[inline(always)]
pub const fn pack_mech(scroll_pmy: u32, pulse_count: u32) -> Option<u64> {
    if scroll_pmy > SCROLL_PMY_MAX {
        return None;
    }
    Some(scroll_pmy as u64 | (pulse_count as u64) << 32)
}

/// One spine state, 64 bytes, exact. Field order is offset order — every
/// byte is a field or the named reserved run, no padding hole. The offsets
/// below are locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpineState64 {
    /// The tick this state was computed at.
    pub tick: EngineTick8,
    /// The 5D position (x, y, z, t, s), poll5d Z-order encoded.
    pub pos: Morton8,
    /// FNV-1a digest (`fnv1a`) of the `InputFrame64` bytes consumed this
    /// step.
    pub input_digest: u64,
    /// Packed mechanism word: low u32 `scroll_pmy` (`0..=SCROLL_PMY_MAX`
    /// permyriad), high u32 `pulse_count`. See [`pack_mech`].
    pub mech: u64,
    /// Reserved. Must be all zero — a live reserved byte is corruption,
    /// never forward-compatibility.
    pub reserved: [u8; 32],
}

impl SpineState64 {
    /// The origin: the tick origin, the poll5d mid-point origin, zero
    /// digest, zero mechanism, no reserved bits.
    pub const ORIGIN: Self = Self {
        tick: EngineTick8::ORIGIN,
        pos: Morton8::ORIGIN,
        input_digest: 0,
        mech: 0,
        reserved: [0; 32],
    };

    /// The `scroll_pmy` low word of `mech`.
    #[inline(always)]
    pub const fn scroll_pmy(self) -> u32 {
        self.mech as u32
    }

    /// The `pulse_count` high word of `mech`.
    #[inline(always)]
    pub const fn pulse_count(self) -> u32 {
        (self.mech >> 32) as u32
    }

    /// True when every channel is inside its domain: `tick.is_valid()`,
    /// `pos.is_valid()`, `scroll_pmy() <= SCROLL_PMY_MAX`, and `reserved` is
    /// all zero. `input_digest` and `pulse_count` are opaque full-range
    /// words — no bit pattern is refused.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.tick.is_valid()
            && self.pos.is_valid()
            && self.scroll_pmy() <= SCROLL_PMY_MAX
            && !any_reserved_byte_is_set(&self.reserved)
    }

    /// Pack into a 64-byte little-endian array. Byte layout is the struct
    /// layout, field for field, offset order.
    #[inline(always)]
    pub const fn encode(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        write_u64(&mut out, 0, self.tick.encode_word());
        write_u64(&mut out, 8, self.pos.word());
        write_u64(&mut out, 16, self.input_digest);
        write_u64(&mut out, 24, self.mech);
        let mut i = 0;
        while i < 32 {
            out[32 + i] = self.reserved[i];
            i += 1;
        }
        out
    }

    /// Unpack a 64-byte array. `None` for anything outside the valid
    /// domain — an invalid tick, an invalid Morton8 (a live spare bit), an
    /// out-of-range `scroll_pmy`, or a live reserved byte is corruption
    /// refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(bytes: [u8; 64]) -> Option<Self> {
        let tick = match EngineTick8::decode_word(read_u64(&bytes, 0)) {
            Some(t) => t,
            None => return None,
        };
        let pos = match Morton8::from_word(read_u64(&bytes, 8)) {
            Some(p) => p,
            None => return None,
        };
        let mut reserved = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            reserved[i] = bytes[32 + i];
            i += 1;
        }
        let c = Self { tick, pos, input_digest: read_u64(&bytes, 16), mech: read_u64(&bytes, 24), reserved };
        if c.is_valid() {
            Some(c)
        } else {
            None
        }
    }
}

#[inline(always)]
const fn any_reserved_byte_is_set(reserved: &[u8; 32]) -> bool {
    let mut i = 0;
    while i < 32 {
        if reserved[i] != 0 {
            return true;
        }
        i += 1;
    }
    false
}

#[inline(always)]
const fn write_u64(out: &mut [u8; 64], at: usize, v: u64) {
    let b = v.to_le_bytes();
    let mut i = 0;
    while i < 8 {
        out[at + i] = b[i];
        i += 1;
    }
}

#[inline(always)]
const fn read_u64(bytes: &[u8; 64], at: usize) -> u64 {
    u64::from_le_bytes([
        bytes[at],
        bytes[at + 1],
        bytes[at + 2],
        bytes[at + 3],
        bytes[at + 4],
        bytes[at + 5],
        bytes[at + 6],
        bytes[at + 7],
    ])
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<SpineState64>() == 64);
const _: () = assert!(core::mem::align_of::<SpineState64>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 64 while
// silently reinterpreting every stored state.
const _: () = assert!(core::mem::offset_of!(SpineState64, tick) == 0);
const _: () = assert!(core::mem::offset_of!(SpineState64, pos) == 8);
const _: () = assert!(core::mem::offset_of!(SpineState64, input_digest) == 16);
const _: () = assert!(core::mem::offset_of!(SpineState64, mech) == 24);
const _: () = assert!(core::mem::offset_of!(SpineState64, reserved) == 32);

// Every one of the 64 bytes is a field — no padding hole.
const _: () = assert!(8 + 8 + 8 + 8 + 32 == core::mem::size_of::<SpineState64>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match SpineState64::decode(SpineState64::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.tick.frame == 0 && w.input_digest == 0 && w.mech == 0);
            assert!(!any_reserved_byte_is_set(&w.reserved));
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    fn base() -> SpineState64 {
        SpineState64 {
            tick: EngineTick8::encode(37, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap(),
            pos: forge_poll5d_v3::Morton8::encode(1, 2, 3, 4, 5).unwrap(),
            input_digest: fnv1a(b"engine-spine"),
            mech: pack_mech(2_500, 9).unwrap(),
            reserved: [0; 32],
        }
    }

    /// L07 over the interior: a representative state survives its own wire
    /// exactly.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        let w = base();
        assert_eq!(SpineState64::decode(w.encode()), Some(w));
    }

    /// L07 over the sentinels: tick frame 0/29/30/`u32::MAX`, Morton8
    /// min/mid/max corners, digest/mech `0`/`u64::MAX`-derived edges.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        let ticks = [
            EngineTick8::encode(0, crate::tick::RUN_STATE_HALT, crate::tick::REGISTER_INFERNO).unwrap(),
            EngineTick8::encode(29, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap(),
            EngineTick8::encode(30, crate::tick::RUN_STATE_REPLAY, crate::tick::REGISTER_PARADISO).unwrap(),
            EngineTick8::encode(u32::MAX, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap(),
        ];
        let positions = [
            forge_poll5d_v3::Morton8::encode(0, 0, 0, 0, 0).unwrap(),
            forge_poll5d_v3::Morton8::ORIGIN,
            forge_poll5d_v3::Morton8::encode(1023, 1023, 1023, 1023, 1023).unwrap(),
        ];
        for tick in ticks {
            for pos in positions {
                let w = SpineState64 {
                    tick,
                    pos,
                    input_digest: u64::MAX,
                    mech: pack_mech(SCROLL_PMY_MAX, u32::MAX).unwrap(),
                    reserved: [0; 32],
                };
                assert_eq!(SpineState64::decode(w.encode()), Some(w));
            }
        }
    }

    /// The origin: `ORIGIN` round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        assert_eq!(SpineState64::decode(SpineState64::ORIGIN.encode()), Some(SpineState64::ORIGIN));
    }

    /// The boundary refuses corruption: each invalid state decodes to None.
    #[test]
    fn out_of_domain_states_are_refused() {
        let good = base();
        assert!(SpineState64::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let mut bad_tick = good;
        bad_tick.tick.phase = 29; // frame 37 % 30 == 7, so 29 is inconsistent.

        let mut bad_pos = good;
        bad_pos.pos = forge_poll5d_v3::Morton8(1u64 << 50); // a live spare bit (50..64).

        let mut bad_scroll = good;
        bad_scroll.mech = pack_mech(0, 0).unwrap();
        bad_scroll.mech |= (SCROLL_PMY_MAX as u64) + 1; // overwrite low u32 past the max directly.

        let mut bad_reserved = good;
        bad_reserved.reserved[0] = 1;

        for (i, b) in [bad_tick, bad_pos, bad_scroll, bad_reserved].iter().enumerate() {
            assert_eq!(SpineState64::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }

    /// `scroll_pmy`/`pulse_count` round-trip through `pack_mech`.
    #[test]
    fn mech_accessors_read_back_what_pack_mech_wrote() {
        let mech = pack_mech(3_333, 77).unwrap();
        let w = SpineState64 { mech, ..SpineState64::ORIGIN };
        assert_eq!(w.scroll_pmy(), 3_333);
        assert_eq!(w.pulse_count(), 77);
        assert_eq!(pack_mech(SCROLL_PMY_MAX + 1, 0), None, "scroll_pmy past the max was accepted");
    }

    /// FNV-1a matches the RFC test vector for the empty string (offset
    /// basis itself) and produces distinct digests for distinct inputs.
    #[test]
    fn fnv1a_matches_the_known_offset_basis_and_differs_by_input() {
        assert_eq!(fnv1a(b""), FNV_OFFSET_BASIS);
        assert_ne!(fnv1a(b"a"), fnv1a(b"b"));
    }
}
