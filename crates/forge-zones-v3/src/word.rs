//! `UlamCell64` — the 64-byte spiral/biome word, T9 of the forge-vision drain
//! (plan-2-welds.md T9, ULAMCELL64 SPEC DRAFT banked in SESSION-HANDOFF.md
//! "2026-08-11 EVENING": `spiral_index u64 + chunk_seed u64 + voxel_type u8 +
//! factor_count u8 + local_pos 3xu8 + biome_hint u8 + avg_density u8 +
//! flags u8 + 40B reserved`, ARCH000 D4 unblock — Ulam IS DONE, quarry weld
//! not fresh authoring).
//!
//! Machine-first (L08): this word is the exact integer encoding; a spiral
//! walk (`UlamSpiral3D`), a biome table lookup, or a rendered trace on glass
//! are all DERIVED from it by a consumer and never stored a second time.
//!
//! ONE HOME (L05): this file is the only definition of `UlamCell64`. The
//! HUD trace mount (xtask, zero-dependency by its own law) rides this same
//! FILE via `#[path]`, the `sky.rs`/T1 precedent; the `sky-mount` feature
//! compiles it data-only there, so no test ever has two homes.
//!
//! Field domains: `spiral_index` and `chunk_seed` are opaque full-range
//! `u64` — every spiral position and every SplitMix64 seed [observed
//! resonance_bridge.rs:78] is a valid word. `voxel_type`, `factor_count`,
//! `local_pos`, `biome_hint`, `avg_density`, and `flags` are likewise opaque
//! full-range bytes — the donor's biome table (prime `factor_count` -> 5
//! density levels [observed]) and scatter-layer flags are consumer-side
//! interpretation, not a domain this word enforces, matching the
//! `EcologyPCM8::event_flags` precedent [observed
//! forge-soundwave-v3/src/ecology.rs — opaque per L17]. Only `reserved` has
//! an enforced domain: all-zero, headroom for scatter-layer refs / lore
//! binding per the spec draft, never a live byte.

/// One Ulam-spiral cell, 64 bytes, exact. Field order is offset order — every
/// byte is a field or the named reserved run, no padding hole. The offsets
/// below are locked by rustc, not prose.
///
/// | Offset | Field | Type | Size |
/// |---|---|---|---|
/// | 0 | `spiral_index` | `u64` | 8B |
/// | 8 | `chunk_seed` | `u64` | 8B |
/// | 16 | `voxel_type` | `u8` | 1B |
/// | 17 | `factor_count` | `u8` | 1B |
/// | 18 | `local_pos` | `[u8; 3]` | 3B |
/// | 21 | `biome_hint` | `u8` | 1B |
/// | 22 | `avg_density` | `u8` | 1B |
/// | 23 | `flags` | `u8` | 1B |
/// | 24 | `reserved` | `[u8; 40]` | 40B |
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UlamCell64 {
    /// Linear index into the Ulam spiral this cell occupies [observed
    /// ulam.rs `index_to_3d`'s `index` parameter]. Opaque — every `u64` is a
    /// reachable spiral position.
    pub spiral_index: u64,
    /// SplitMix64 chunk seed [observed resonance_bridge.rs:78]. Opaque —
    /// every `u64` is a valid seed.
    pub chunk_seed: u64,
    /// Voxel/material kind at this cell. Opaque byte; the enum table is a
    /// consumer's concern, not this word's.
    pub voxel_type: u8,
    /// Prime factor count of `spiral_index`, the biome-density driver
    /// [observed: "biome = prime factor_count -> density 5 levels"]. Opaque
    /// byte — the 5-level mapping is derived, not stored twice.
    pub factor_count: u8,
    /// Local position within the chunk (x, y, z), opaque bytes.
    pub local_pos: [u8; 3],
    /// Biome table hint, opaque byte.
    pub biome_hint: u8,
    /// Average density, opaque byte (permyriad or byte-scale is a consumer
    /// bridge's decision, not this word's — `[ASSUMED]`, no quarried source
    /// pins the scale).
    pub avg_density: u8,
    /// Scatter/lore flags, opaque byte, matching the `EcologyPCM8::event_flags`
    /// precedent [observed forge-soundwave-v3/src/ecology.rs].
    pub flags: u8,
    /// Reserved headroom for scatter-layer refs / lore binding per the spec
    /// draft. Must be all zero — a live reserved byte is corruption, never
    /// forward-compatibility.
    pub reserved: [u8; 40],
}

impl UlamCell64 {
    /// The origin: spiral index 0 (the Ulam spiral's own centre, `(0,0)` per
    /// `UlamSpiral3D::compute_2d`), zero seed, zero voxel/factor/biome/
    /// density/flags, zero local position, no reserved bits.
    pub const ORIGIN: Self = Self {
        spiral_index: 0,
        chunk_seed: 0,
        voxel_type: 0,
        factor_count: 0,
        local_pos: [0; 3],
        biome_hint: 0,
        avg_density: 0,
        flags: 0,
        reserved: [0; 40],
    };

    /// True when the reserved run is all zero. Every other field is opaque
    /// full-range, so this is the word's entire enforced domain.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        !any_reserved_byte_is_set(&self.reserved)
    }

    /// Pack into a 64-byte little-endian array. Byte layout is the struct
    /// layout, field for field, offset order.
    #[inline(always)]
    pub const fn encode(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        write_u64(&mut out, 0, self.spiral_index);
        write_u64(&mut out, 8, self.chunk_seed);
        out[16] = self.voxel_type;
        out[17] = self.factor_count;
        out[18] = self.local_pos[0];
        out[19] = self.local_pos[1];
        out[20] = self.local_pos[2];
        out[21] = self.biome_hint;
        out[22] = self.avg_density;
        out[23] = self.flags;
        let mut i = 0;
        while i < 40 {
            out[24 + i] = self.reserved[i];
            i += 1;
        }
        out
    }

    /// Unpack a 64-byte array. `None` for anything outside the valid
    /// domain — a live reserved byte is corruption refused at the boundary,
    /// not clamped past it.
    #[inline(always)]
    pub const fn decode(bytes: [u8; 64]) -> Option<Self> {
        let mut reserved = [0u8; 40];
        let mut i = 0;
        while i < 40 {
            reserved[i] = bytes[24 + i];
            i += 1;
        }
        let c = Self {
            spiral_index: read_u64(&bytes, 0),
            chunk_seed: read_u64(&bytes, 8),
            voxel_type: bytes[16],
            factor_count: bytes[17],
            local_pos: [bytes[18], bytes[19], bytes[20]],
            biome_hint: bytes[21],
            avg_density: bytes[22],
            flags: bytes[23],
            reserved,
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

#[inline(always)]
const fn any_reserved_byte_is_set(reserved: &[u8; 40]) -> bool {
    let mut i = 0;
    while i < 40 {
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
const _: () = assert!(core::mem::size_of::<UlamCell64>() == 64);
const _: () = assert!(core::mem::align_of::<UlamCell64>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 64 while
// silently reinterpreting every stored cell.
const _: () = assert!(core::mem::offset_of!(UlamCell64, spiral_index) == 0);
const _: () = assert!(core::mem::offset_of!(UlamCell64, chunk_seed) == 8);
const _: () = assert!(core::mem::offset_of!(UlamCell64, voxel_type) == 16);
const _: () = assert!(core::mem::offset_of!(UlamCell64, factor_count) == 17);
const _: () = assert!(core::mem::offset_of!(UlamCell64, local_pos) == 18);
const _: () = assert!(core::mem::offset_of!(UlamCell64, biome_hint) == 21);
const _: () = assert!(core::mem::offset_of!(UlamCell64, avg_density) == 22);
const _: () = assert!(core::mem::offset_of!(UlamCell64, flags) == 23);
const _: () = assert!(core::mem::offset_of!(UlamCell64, reserved) == 24);

// Every one of the 64 bytes is a field — no padding hole.
const _: () = assert!(8 + 8 + 1 + 1 + 3 + 1 + 1 + 1 + 40 == core::mem::size_of::<UlamCell64>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match UlamCell64::decode(UlamCell64::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.spiral_index == 0 && w.chunk_seed == 0);
            assert!(w.voxel_type == 0 && w.factor_count == 0);
            assert!(w.local_pos[0] == 0 && w.local_pos[1] == 0 && w.local_pos[2] == 0);
            assert!(w.biome_hint == 0 && w.avg_density == 0 && w.flags == 0);
            assert!(!any_reserved_byte_is_set(&w.reserved));
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    fn base() -> UlamCell64 {
        UlamCell64 {
            spiral_index: 12_345,
            chunk_seed: 0x9E3779B97F4A7C15,
            voxel_type: 3,
            factor_count: 4,
            local_pos: [7, 8, 9],
            biome_hint: 2,
            avg_density: 200,
            flags: 0b1010_0101,
            reserved: [0; 40],
        }
    }

    /// L07 over the interior: every field survives its own wire exactly.
    #[test]
    fn bijection_holds_over_the_interior() {
        let c = base();
        assert_eq!(UlamCell64::decode(c.encode()), Some(c));
    }

    /// L07 over the sentinels: 0 and max on every opaque byte and both u64
    /// fields, min and max local-position bytes.
    #[test]
    fn bijection_holds_over_the_sentinels() {
        for word in [0u64, 1, u64::MAX] {
            for byte in [0u8, 1, u8::MAX] {
                let c = UlamCell64 {
                    spiral_index: word,
                    chunk_seed: word,
                    voxel_type: byte,
                    factor_count: byte,
                    local_pos: [byte, byte, byte],
                    biome_hint: byte,
                    avg_density: byte,
                    flags: byte,
                    reserved: [0; 40],
                };
                assert_eq!(UlamCell64::decode(c.encode()), Some(c), "word={word} byte={byte}");
            }
        }
    }

    /// The origin: spiral index 0, everything else at rest, round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        assert_eq!(UlamCell64::decode(UlamCell64::ORIGIN.encode()), Some(UlamCell64::ORIGIN));
    }

    /// Each local_pos axis is independently addressable and survives the
    /// wire — the edge where x, y, z diverge.
    #[test]
    fn local_pos_axes_are_independent() {
        let c = UlamCell64 { local_pos: [1, 2, 3], ..base() };
        let d = UlamCell64::decode(c.encode()).expect("valid word");
        assert_eq!(d.local_pos, [1, 2, 3]);
    }

    /// The boundary refuses corruption: a live reserved byte, first or last
    /// in the run, decodes to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = base();
        assert!(UlamCell64::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            UlamCell64 { reserved: { let mut r = [0u8; 40]; r[0] = 1; r }, ..good },
            UlamCell64 { reserved: { let mut r = [0u8; 40]; r[39] = 1; r }, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(UlamCell64::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }
}
