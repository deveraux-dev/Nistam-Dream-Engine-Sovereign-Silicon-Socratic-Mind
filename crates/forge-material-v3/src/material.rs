//! `MaterialTrit8` — the 8-byte material word, T8 of the forge-vision drain
//! (plan-2-welds.md T8, re-founded from the forgeatom `material_traits.rs`
//! `material_id` line) plus the ARCH000 2026-08-11 material-canvas
//! directive: a 25-trit ternary craft plane, 5 trits/byte in base-3.
//!
//! ## The `[ASSUMED]` on the id line is DISCHARGED (2026-08-11)
//!
//! This header previously carried `[ASSUMED]` because the source file was not
//! present in the tree that session. It is present and was re-observed this
//! session: `F:\NewRepo\crates\forge-vision\src\scan\material_traits.rs:15-21`
//! declares `TRAIT_MATERIAL_{NONE,VOID,SHADOW,IRON,STONE,BONE,ASH} = 0..=6`,
//! matching the seven consts below exactly. The quarried line was correct.
//!
//! Two further facts from that file, both load-bearing and now observed:
//!   - `:3-5` — the v2 pack was `[material_id: 4 bits][percent_permyriad: 12
//!     bits]`, `0..4095`. `FILL_MAX` here is the permyriad upgrade of that
//!     channel, so the widening is deliberate, not a re-reading.
//!   - `:8` — "Sum of percent across overlapping materials per voxel must be
//!     <= 10000; overflow fires VISION_GATE_TRAIT_OVERFLOW." Overlapping
//!     materials per voxel are therefore v2 doctrine, not a v3 invention, and
//!     10_000 was already the semantic ceiling. The layered stack tranche
//!     inherits that invariant and refuses overflow rather than clamping it.
//!
//! Machine-first (L08): this word is the exact integer encoding; no
//! perceptual or shading transform lives here — `NormalAlbedo8`
//! (forge-photometric-v3) carries the continuous energy gradient and is
//! composed with this word at shade time, never merged into one home.
//!
//! ONE HOME (L05): this file is the only definition of `MaterialTrit8` and
//! the 5-trit base-3 byte codec.

/// Highest valid `material_id`: `242`, so the line carries **243 materials**.
///
/// ARCH000 2026-08-11 took the extension the previous ceiling was holding open
/// — but not to the 64-slot registry that comment anticipated. `243 == 3^5`, so
/// a `material_id` is exactly **one base-3 byte: five balanced trits**, read by
/// the same [`unpack5`] this file already defines for the craft plane. One
/// codec, two channels, no second home (L05).
///
/// That also makes the refusal rule one rule instead of two: `243..=255` are
/// refused for a `material_id` for precisely the reason they are refused for a
/// craft byte — those bit patterns have no balanced-trit reading. And it is the
/// last ternary step that fits a byte at all (`3^6 == 729` needs two), so this
/// is the ceiling, not a waypoint.
///
/// The seven forgeatom names below keep their values `0..=6` unchanged; they
/// are now named members of a 243-wide line rather than the whole of it.
pub const MATERIAL_ID_MAX: u8 = CRAFT_BYTE_MAX;

/// How many material ids the line carries: `243 == 3^5`, one per 5-trit word.
pub const MATERIAL_COUNT: u16 = MATERIAL_ID_MAX as u16 + 1;

/// The five axes of the `material_id` coordinate, least-significant trit
/// first — the same order [`unpack5`]/[`pack5`] already use. Axis order is
/// fixed per the B1 brief: `0 PHASE, 1 ORIGIN, 2 CONDUCTANCE, 3 DENSITY,
/// 4 NOBILITY`. Full axis semantics live in the sprint doc; this file names
/// the axis slots and the codec, not the physical meaning of each trit
/// value — that stays out of scope here (not re-derived this session).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialAxis {
    /// Trit slot 0.
    Phase = 0,
    /// Trit slot 1.
    Origin = 1,
    /// Trit slot 2.
    Conductance = 2,
    /// Trit slot 3.
    Density = 3,
    /// Trit slot 4.
    Nobility = 4,
}

impl MaterialAxis {
    /// All five axes, in trit-slot order.
    pub const ALL: [MaterialAxis; 5] =
        [Self::Phase, Self::Origin, Self::Conductance, Self::Density, Self::Nobility];

    /// This axis's slot in the trit array `unpack5`/`pack5` already use.
    #[inline(always)]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// `material_id = 2` — ash, a coordinate on the axis model, not the old flat
/// index. Value asserted against [`MaterialTrit8::from_axes`] output below
/// (L07), not hand-trusted.
pub const MATERIAL_ASH: u8 = 2;
/// `material_id = 80` — iron.
pub const MATERIAL_IRON: u8 = 80;
/// `material_id = 110` — bone.
pub const MATERIAL_BONE: u8 = 110;
/// `material_id = 143` — stone.
pub const MATERIAL_STONE: u8 = 143;
/// `material_id = 165` — gas.
pub const MATERIAL_GAS: u8 = 165;
/// `material_id = 242` — gold, the physical ceiling itself.
pub const MATERIAL_GOLD: u8 = 242;

// `MATERIAL_NONE`, `MATERIAL_VOID` and `MATERIAL_SHADOW` are DELETED per the
// B1 brief: absence is the PHASE axis and sentinel 246; shadow is sentinel
// 243. Neither is a physical coordinate any more.
//
// Ids `0..=242` are the open physical line: valid, roundtrippable, and
// unnamed beyond the six coordinates above until something names them.

/// Full scale for the fill channel: `0..=FILL_MAX` maps `0.0..=1.0`,
/// upgraded from the quarry's 12-bit `0..=4095` pack to permyriad, matching
/// the other T-words' fill/albedo/roughness channels.
pub const FILL_MAX: u16 = 10_000;

/// Highest valid craft byte: 5 trits base-3 top out at `2+2*3+2*9+2*27+2*81
/// == 242`. `243..=255` are refused — those bit patterns have no balanced-
/// trit reading.
pub const CRAFT_BYTE_MAX: u8 = 242;

/// Highest physical `material_id` — the axis-model name for [`MATERIAL_ID_MAX`],
/// which is itself `== CRAFT_BYTE_MAX` (asserted below, L05: one boundary,
/// not a second one under a new name).
pub const MATERIAL_PHYSICAL_MAX: u8 = MATERIAL_ID_MAX;
/// How many physical material ids the line carries — the axis-model name for
/// [`MATERIAL_COUNT`].
pub const MATERIAL_PHYSICAL_COUNT: u16 = MATERIAL_COUNT;
const _: () = assert!(MATERIAL_PHYSICAL_MAX == CRAFT_BYTE_MAX);
const _: () = assert!(MATERIAL_PHYSICAL_COUNT == 243);

/// The 13 `material_id` sentinels, `243..=255` — bytes with no balanced-trit
/// reading. This names the SAME envelope `forge-core-v3::atom::TritCell5D`
/// already asserts (13 == 256 - 243); it does not mint a second one (L05).
///
/// `NO MEMSET POISON` (ARCH000 2026-08-11, withdrawn justification): `HaltRecycle`
/// is a slot explicitly marked cleared and ready for overwrite, and nothing
/// more — not a poison-fill value, not a fill/sweep/garbage-detection path,
/// not a claim that lifecycle is inferred from memory contents.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentinel {
    /// Optical interrupt — photon deficit / occlusion mask; 0 radiance
    /// write, geometry still hit-tests.
    Shadow = 243,
    /// Procedural field — bypass static LUT; evaluate real-time 5D noise
    /// kernels.
    Chaos = 244,
    /// Material decay — thermal/weathering delta; blend neighbour cells
    /// over t.
    Entropy = 245,
    /// Absence — absolute vacuum; prune downstream node passes.
    Void = 246,
    /// Node blend — weighted dual-material blend across vertex/noise
    /// boundary.
    Superposition = 247,
    /// Fallback — unbound material catch; must never panic.
    Untraced = 248,
    /// Energy source — bypass BRDF; cell is a radiator.
    Emissive = 249,
    /// Geometric gate — re-route to reflection/refraction vectors.
    Mirror = 250,
    /// Pipeline flag — alpha skip; defer to underlying layers.
    Passthrough = 251,
    /// Physics gate — slot stores kinematic vectors, not material mass.
    ForceField = 252,
    /// Custom script — hand to user compute kernel.
    UserReserved = 253,
    /// Boundary gate — lock-free ring-buffer boundary for RenderGate64.
    IpcSync = 254,
    /// Memory gate — slot explicitly marked cleared, ready for overwrite.
    HaltRecycle = 255,
}

impl Sentinel {
    /// All 13 sentinel variants, in byte order `243..=255`.
    pub const ALL: [Sentinel; 13] = [
        Self::Shadow, Self::Chaos, Self::Entropy, Self::Void, Self::Superposition, Self::Untraced,
        Self::Emissive, Self::Mirror, Self::Passthrough, Self::ForceField, Self::UserReserved,
        Self::IpcSync, Self::HaltRecycle,
    ];

    /// The sentinel a byte reads as, if any. `None` for `0..=242`.
    #[inline(always)]
    pub const fn from_u8(byte: u8) -> Option<Sentinel> {
        match byte {
            243 => Some(Self::Shadow),
            244 => Some(Self::Chaos),
            245 => Some(Self::Entropy),
            246 => Some(Self::Void),
            247 => Some(Self::Superposition),
            248 => Some(Self::Untraced),
            249 => Some(Self::Emissive),
            250 => Some(Self::Mirror),
            251 => Some(Self::Passthrough),
            252 => Some(Self::ForceField),
            253 => Some(Self::UserReserved),
            254 => Some(Self::IpcSync),
            255 => Some(Self::HaltRecycle),
            _ => None,
        }
    }

    /// This sentinel's byte value.
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Flat sentinel consts, `u8` — downstream crates bind these names directly
/// rather than going through the [`Sentinel`] enum.
pub const SENTINEL_SHADOW: u8 = 243;
/// See [`Sentinel::Chaos`].
pub const SENTINEL_CHAOS: u8 = 244;
/// See [`Sentinel::Entropy`].
pub const SENTINEL_ENTROPY: u8 = 245;
/// See [`Sentinel::Void`].
pub const SENTINEL_VOID: u8 = 246;
/// See [`Sentinel::Superposition`].
pub const SENTINEL_SUPERPOSITION: u8 = 247;
/// See [`Sentinel::Untraced`].
pub const SENTINEL_UNTRACED: u8 = 248;
/// See [`Sentinel::Emissive`].
pub const SENTINEL_EMISSIVE: u8 = 249;
/// See [`Sentinel::Mirror`].
pub const SENTINEL_MIRROR: u8 = 250;
/// See [`Sentinel::Passthrough`].
pub const SENTINEL_PASSTHROUGH: u8 = 251;
/// See [`Sentinel::ForceField`].
pub const SENTINEL_FORCE_FIELD: u8 = 252;
/// See [`Sentinel::UserReserved`].
pub const SENTINEL_USER_RESERVED: u8 = 253;
/// See [`Sentinel::IpcSync`].
pub const SENTINEL_IPC_SYNC: u8 = 254;
/// See [`Sentinel::HaltRecycle`].
pub const SENTINEL_HALT_RECYCLE: u8 = 255;

/// How many trits one craft byte carries.
const TRITS_PER_BYTE: usize = 5;

/// Pack 5 balanced trits (`-1, 0, +1`, one per array slot, least-significant
/// first) into one base-3 byte, `0..=CRAFT_BYTE_MAX`. Stored digit `d in
/// 0..=2` maps to trit `d - 1`, so packing is the inverse of `unpack5`'s
/// digit read.
#[inline(always)]
pub const fn pack5(trits: [i8; TRITS_PER_BYTE]) -> u8 {
    let mut value: u16 = 0;
    let mut place: u16 = 1;
    let mut i = 0;
    while i < TRITS_PER_BYTE {
        let digit = (trits[i] + 1) as u16;
        value += digit * place;
        place *= 3;
        i += 1;
    }
    value as u8
}

/// Unpack a base-3 byte into 5 balanced trits. `None` for `243..=255` — a
/// byte with no balanced-trit reading is corruption refused at the
/// boundary, not clamped past it.
#[inline(always)]
pub const fn unpack5(byte: u8) -> Option<[i8; TRITS_PER_BYTE]> {
    if byte > CRAFT_BYTE_MAX {
        return None;
    }
    let mut v = byte as u16;
    let mut trits = [0i8; TRITS_PER_BYTE];
    let mut i = 0;
    while i < TRITS_PER_BYTE {
        let digit = (v % 3) as i8;
        trits[i] = digit - 1;
        v /= 3;
        i += 1;
    }
    Some(trits)
}

/// One material texel, 8 bytes, exact: the forgeatom `material_id`, a fill
/// channel in permyriad, and the 25-trit ternary craft plane. Field order is
/// chosen so every byte is a field and offsets fall on natural alignment —
/// `fill_pmy` (align 2) leads at offset 0, `material_id` and `craft` fill
/// the remaining single bytes with no hole. The offsets below are locked by
/// rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialTrit8 {
    /// Fill in permyriad, `0..=FILL_MAX`.
    pub fill_pmy: u16,
    /// The material line: `0..=MATERIAL_ID_MAX` (`0..=242`) is the physical
    /// axis-coordinate line, six of whose points are named
    /// (`MATERIAL_ASH`..`MATERIAL_GOLD`); `243..=255` are the 13 named
    /// [`Sentinel`] values. The rest of the physical line is open and
    /// unnamed.
    pub material_id: u8,
    /// The ternary craft plane, 5 bytes x 5 trits/byte base-3 = 25 trits.
    /// Trits 0 and 1 (byte 0, positions 0-1) are the HERMETIC METAL lane;
    /// trit 3 (byte 0, position 3) is the METALNESS trit; every other trit
    /// index (2, and 4..25) is an open craft lane, roundtripped opaquely —
    /// no invented meaning beyond the codec (L17).
    pub craft: [u8; 5],
}

impl MaterialTrit8 {
    /// The origin: absence (`SENTINEL_VOID`, per the brief: absence is
    /// sentinel 246, not a physical coordinate), no fill, craft plane
    /// all-zero (every trit reads `-1`, the D1 "mute" reading, at the
    /// all-zero byte pattern).
    pub const EMPTY: Self = Self { fill_pmy: 0, material_id: SENTINEL_VOID, craft: [0; 5] };

    /// True when every channel is inside its domain: `material_id` is now
    /// always in-domain (`0..=242` physical, `243..=255` sentinel — every one
    /// of the 256 byte values is a legal id), `fill_pmy <= FILL_MAX`, and
    /// every craft byte has a balanced-trit reading (`<= CRAFT_BYTE_MAX`).
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.fill_pmy <= FILL_MAX
            && self.craft[0] <= CRAFT_BYTE_MAX
            && self.craft[1] <= CRAFT_BYTE_MAX
            && self.craft[2] <= CRAFT_BYTE_MAX
            && self.craft[3] <= CRAFT_BYTE_MAX
            && self.craft[4] <= CRAFT_BYTE_MAX
    }

    /// Read one craft trit by its flat index `0..25`. `None` for an index
    /// out of range or a craft byte with no balanced-trit reading.
    #[inline(always)]
    pub const fn craft_trit(self, idx: u8) -> Option<i8> {
        if idx >= 25 {
            return None;
        }
        let byte_idx = (idx / TRITS_PER_BYTE as u8) as usize;
        let pos = (idx % TRITS_PER_BYTE as u8) as usize;
        match unpack5(self.craft[byte_idx]) {
            Some(trits) => Some(trits[pos]),
            None => None,
        }
    }

    /// The HERMETIC METAL lane, trits 0-1 folded back to `0..=8`: 9 states
    /// for the 7 hermetic metals plus `None` plus one reserve slot.
    ///
    /// Ordering is **observed**, not assumed (2026-08-11): `seven.rs` does not
    /// exist as a file, but the sevenfold line is live at
    /// `crates/forge-mud-v3/src/hermetics.rs:120-127`, where `SEVENFOLD`
    /// declares its correspondences in exactly this order — Iron(Mars),
    /// Lead(Saturn), Quicksilver(Mercury), Silver(Luna), Copper(Venus),
    /// Gold(Sol), Tin(Jupiter). The previous `[ASSUMED]` on this mapping is
    /// discharged against that file; `hermetics.rs` is the one home for the
    /// ordering and this lane must never restate it (L05).
    ///
    /// Mapping: `0` Iron, `1` Lead, `2` Quicksilver, `3` Silver, `4` Copper,
    /// `5` Gold, `6` Tin, `7` None, `8` reserve.
    #[inline(always)]
    pub const fn metal_lane(self) -> Option<u8> {
        let t0 = match self.craft_trit(0) {
            Some(t) => t,
            None => return None,
        };
        let t1 = match self.craft_trit(1) {
            Some(t) => t,
            None => return None,
        };
        Some(((t0 + 1) as u8) + ((t1 + 1) as u8) * 3)
    }

    /// This id's five axis trits, `[PHASE, ORIGIN, CONDUCTANCE, DENSITY,
    /// NOBILITY]`. `None` for a sentinel id (`243..=255`) — a sentinel has no
    /// axis reading, by the same boundary as [`unpack5`].
    #[inline(always)]
    pub const fn axes(self) -> Option<[i8; TRITS_PER_BYTE]> {
        self.material_trits()
    }

    /// One axis trit by name. `None` for a sentinel id.
    #[inline(always)]
    pub const fn axis(self, axis: MaterialAxis) -> Option<i8> {
        match self.axes() {
            Some(a) => Some(a[axis.index()]),
            None => None,
        }
    }

    /// Build a physical `material_id` from its five axis trits. `None` if any
    /// trit is outside `-1..=1` — refused, never clamped.
    #[inline(always)]
    pub const fn from_axes(axes: [i8; TRITS_PER_BYTE]) -> Option<u8> {
        let mut i = 0;
        while i < TRITS_PER_BYTE {
            if axes[i] < -1 || axes[i] > 1 {
                return None;
            }
            i += 1;
        }
        Some(pack5(axes))
    }

    /// This id's sentinel, if it is one (`243..=255`). `None` for a physical
    /// id.
    #[inline(always)]
    pub const fn sentinel(self) -> Option<Sentinel> {
        Sentinel::from_u8(self.material_id)
    }

    /// True for a physical id, `0..=242`.
    #[inline(always)]
    pub const fn is_physical(self) -> bool {
        self.material_id <= MATERIAL_PHYSICAL_MAX
    }

    /// True for a sentinel id, `243..=255` — exactly the complement of
    /// [`Self::is_physical`].
    #[inline(always)]
    pub const fn is_sentinel(self) -> bool {
        !self.is_physical()
    }

    /// One trit product, `{-1, 0, +1}`, mapped to permyriad `{0, 5_000,
    /// 10_000}`. Exact: `10_000` is even, so the midpoint halves clean.
    #[inline(always)]
    const fn trit_product_pmy(product: i8) -> u16 {
        (product as i16 + 1) as u16 * 5_000
    }

    /// Derived mass: `PHASE x DENSITY`. `None` for a sentinel id. Not a
    /// stored field — a derivation over the axes.
    #[inline(always)]
    pub const fn mass_pmy(self) -> Option<u16> {
        match self.axes() {
            Some(a) => Some(Self::trit_product_pmy(a[MaterialAxis::Phase as usize] * a[MaterialAxis::Density as usize])),
            None => None,
        }
    }

    /// Derived ring: `PHASE x CONDUCTANCE x ORIGIN`. `None` for a sentinel id.
    #[inline(always)]
    pub const fn ring_pmy(self) -> Option<u16> {
        match self.axes() {
            Some(a) => Some(Self::trit_product_pmy(
                a[MaterialAxis::Phase as usize] * a[MaterialAxis::Conductance as usize] * a[MaterialAxis::Origin as usize],
            )),
            None => None,
        }
    }

    /// Derived attack: `PHASE x DENSITY x NOBILITY`. `None` for a sentinel id.
    #[inline(always)]
    pub const fn attack_pmy(self) -> Option<u16> {
        match self.axes() {
            Some(a) => Some(Self::trit_product_pmy(
                a[MaterialAxis::Phase as usize] * a[MaterialAxis::Density as usize] * a[MaterialAxis::Nobility as usize],
            )),
            None => None,
        }
    }

    /// The `material_id` read as its five balanced trits, least-significant
    /// first. `None` only for `243..=255` — the ids with no trit reading, which
    /// `is_valid` already refuses.
    ///
    /// This is the whole point of the 243 ceiling: a material's identity is a
    /// ternary word, not an opaque index, and it is read by the same codec as
    /// the craft plane. The five trits carry no assigned meaning yet — naming a
    /// lane here is ARCH000's call, exactly as it is for craft trits 4..24, and
    /// inventing one would be the kind of unearned semantics L17 refuses.
    #[inline(always)]
    pub const fn material_trits(self) -> Option<[i8; TRITS_PER_BYTE]> {
        unpack5(self.material_id)
    }

    /// The METALNESS trit, craft index 3: `-1` dielectric, `0` hybrid, `+1`
    /// conductor.
    #[inline(always)]
    pub const fn metalness_trit(self) -> Option<i8> {
        self.craft_trit(3)
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: fill lo/hi, material_id, craft[0..5].
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.fill_pmy as u64
            | (self.material_id as u64) << 16
            | (self.craft[0] as u64) << 24
            | (self.craft[1] as u64) << 32
            | (self.craft[2] as u64) << 40
            | (self.craft[3] as u64) << 48
            | (self.craft[4] as u64) << 56
    }

    /// Unpack a word. `None` for anything outside the valid domain — an
    /// out-of-range `material_id`, an out-of-range `fill_pmy`, or a craft
    /// byte with no balanced-trit reading is corruption refused at the
    /// boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            fill_pmy: word as u16,
            material_id: (word >> 16) as u8,
            craft: [
                (word >> 24) as u8,
                (word >> 32) as u8,
                (word >> 40) as u8,
                (word >> 48) as u8,
                (word >> 56) as u8,
            ],
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
const _: () = assert!(core::mem::size_of::<MaterialTrit8>() == 8);
const _: () = assert!(core::mem::align_of::<MaterialTrit8>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping fields keeps size 8 while
// silently reinterpreting every stored word.
const _: () = assert!(core::mem::offset_of!(MaterialTrit8, fill_pmy) == 0);
const _: () = assert!(core::mem::offset_of!(MaterialTrit8, material_id) == 2);
const _: () = assert!(core::mem::offset_of!(MaterialTrit8, craft) == 3);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(2 + 1 + 5 == core::mem::size_of::<MaterialTrit8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match MaterialTrit8::decode(MaterialTrit8::EMPTY.encode()) {
        Some(w) => {
            assert!(w.fill_pmy == 0 && w.material_id == SENTINEL_VOID);
            assert!(w.craft[0] == 0 && w.craft[1] == 0 && w.craft[2] == 0 && w.craft[3] == 0 && w.craft[4] == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
    // The trit codec's own origin: byte 0 unpacks to all -1, and packs back.
    match unpack5(0) {
        Some(trits) => {
            assert!(
                trits[0] == -1 && trits[1] == -1 && trits[2] == -1 && trits[3] == -1 && trits[4] == -1
            );
            assert!(pack5(trits) == 0);
        }
        None => panic!("byte 0 failed to unpack — the trit codec is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: every lattice word sample survives its own
    /// wire exactly, across all 7 material ids and interior fill/craft
    /// samples.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        for material_id in [MATERIAL_BONE, MATERIAL_ASH, 7, 100, MATERIAL_ID_MAX] {
            for fill_pmy in [1u16, 2_500, 5_000, 7_500, 9_999] {
                for craft_byte in [1u8, 40, 121, 200, 241] {
                    let w = MaterialTrit8 { fill_pmy, material_id, craft: [craft_byte; 5] };
                    assert_eq!(
                        MaterialTrit8::decode(w.encode()),
                        Some(w),
                        "material_id={material_id} fill={fill_pmy} craft={craft_byte}"
                    );
                }
            }
        }
    }

    /// L07 over the sentinels: 0 and FILL_MAX on fill, 0/121/242 on every
    /// craft byte (all-`-1`, all-`0`, all-`+1` trits), both material_id
    /// extremes.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for material_id in [MATERIAL_BONE, MATERIAL_ASH, MATERIAL_ID_MAX] {
            for fill_pmy in [0u16, FILL_MAX] {
                for craft_byte in [0u8, 121, CRAFT_BYTE_MAX] {
                    let w = MaterialTrit8 { fill_pmy, material_id, craft: [craft_byte; 5] };
                    assert_eq!(MaterialTrit8::decode(w.encode()), Some(w));
                }
            }
        }
    }

    /// L07 over mixed craft bytes: each of the 5 craft bytes independently
    /// varied, not just uniform fill.
    #[test]
    fn word_bijection_holds_over_mixed_craft() {
        let w = MaterialTrit8 { fill_pmy: 3_333, material_id: MATERIAL_IRON, craft: [0, 60, 121, 180, 242] };
        assert_eq!(MaterialTrit8::decode(w.encode()), Some(w));
    }

    /// The boundary refuses corruption: each invalid word decodes to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = MaterialTrit8 { fill_pmy: 5_000, material_id: MATERIAL_STONE, craft: [10, 20, 30, 40, 50] };
        assert!(MaterialTrit8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        // `material_id: MATERIAL_ID_MAX + 1` is NOT a bad row any more: it is
        // a sentinel id (`243..=255`), and every one of the 256 material_id
        // byte values is now in-domain (brief §3) — only a craft byte past
        // `CRAFT_BYTE_MAX` or a fill past `FILL_MAX` is corruption.
        let bad = [
            MaterialTrit8 { fill_pmy: FILL_MAX + 1, ..good },
            MaterialTrit8 { craft: [243, 20, 30, 40, 50], ..good },
            MaterialTrit8 { craft: [10, 255, 30, 40, 50], ..good },
            MaterialTrit8 { craft: [10, 20, 30, 40, 243], ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(MaterialTrit8::decode(b.encode()), None, "bad row {i} was accepted");
        }

        // A sentinel material_id, otherwise-valid word, is still accepted.
        let sentinel_word = MaterialTrit8 { material_id: MATERIAL_ID_MAX + 1, ..good };
        assert!(MaterialTrit8::decode(sentinel_word.encode()).is_some(), "a sentinel id must decode");
    }

    /// The origin: EMPTY round-trips and reads back as absence,
    /// no-fill, all craft trits `-1`.
    #[test]
    fn the_origin_survives_its_wire() {
        let w = MaterialTrit8::EMPTY;
        assert_eq!(MaterialTrit8::decode(w.encode()), Some(w));
        for idx in 0..25 {
            assert_eq!(w.craft_trit(idx), Some(-1), "trit {idx} was not -1 at the origin");
        }
    }

    /// L07 over the trit codec: `pack5(unpack5(b).unwrap()) == b` for every
    /// valid byte `0..=CRAFT_BYTE_MAX`, and every unpacked trit is in
    /// `{-1, 0, 1}`.
    #[test]
    fn trit_codec_bijection_holds_over_every_valid_byte() {
        for b in 0..=CRAFT_BYTE_MAX {
            let trits = unpack5(b).unwrap_or_else(|| panic!("byte {b} should unpack"));
            for t in trits {
                assert!(t == -1 || t == 0 || t == 1, "byte {b} produced an out-of-domain trit {t}");
            }
            assert_eq!(pack5(trits), b, "byte {b} did not survive unpack-then-pack");
        }
    }

    /// The whole 243-material line survives its own wire — every id, not a
    /// sample. This is the test the widened ceiling is worth: if any id in
    /// `0..=242` failed to roundtrip, the line would be a claim rather than a
    /// capacity.
    #[test]
    fn every_one_of_the_two_hundred_and_forty_three_materials_roundtrips() {
        assert_eq!(MATERIAL_COUNT, 243);
        assert_eq!(MATERIAL_ID_MAX, CRAFT_BYTE_MAX, "the id ceiling IS the 5-trit ceiling");

        for material_id in 0..=MATERIAL_ID_MAX {
            let w = MaterialTrit8 { fill_pmy: 4_242, material_id, craft: [7; 5] };
            assert!(w.is_valid(), "material_id {material_id} must be inside the line");
            assert_eq!(MaterialTrit8::decode(w.encode()), Some(w), "material_id {material_id}");

            // The id is a ternary word, and it is the SAME codec as the craft
            // plane — not a parallel one.
            let trits = w.material_trits().unwrap_or_else(|| panic!("id {material_id} has no trit reading"));
            for t in trits {
                assert!(t == -1 || t == 0 || t == 1, "id {material_id} produced an out-of-domain trit {t}");
            }
            assert_eq!(pack5(trits), material_id, "id {material_id} did not survive unpack-then-pack");
        }
    }

    /// Sentinel ids (`243..=255`) have no axis/trit reading, but they are
    /// still in-domain material ids (`is_valid`/`decode` accept them) — the
    /// refusal boundary belongs to the trit codec (`material_trits`,
    /// `unpack5`), not to `is_valid`. This replaces the old flat-index
    /// "ids past the ceiling are refused" test, which assumed a basis (a
    /// flat `material_id` line with no sentinel range) that the B1 axis
    /// model superseded.
    #[test]
    fn sentinel_ids_are_in_domain_but_have_no_trit_reading() {
        for material_id in (MATERIAL_ID_MAX + 1)..=255 {
            let w = MaterialTrit8 { fill_pmy: 0, material_id, craft: [0; 5] };
            assert!(w.is_valid(), "sentinel material_id {material_id} must still be in-domain");
            assert!(MaterialTrit8::decode(w.encode()).is_some(), "sentinel material_id {material_id} must decode");
            assert_eq!(w.material_trits(), None, "id {material_id} must have no trit reading");
            assert_eq!(unpack5(material_id), None, "the id and craft refusals must agree at {material_id}");
            assert!(w.is_sentinel(), "id {material_id} must read as a sentinel");
            assert!(w.sentinel().is_some(), "id {material_id} must resolve to a Sentinel variant");
        }
    }

    /// ONE ENVELOPE, TWO NAMERS (L05). Crate Zero already asserts this control
    /// envelope from the other side — `forge-core-v3/src/atom.rs:141-143`,
    /// `assert_eq!(sentinels, 13, "256 - 243 = 13; the control envelope is not
    /// negotiable")` — over the SAME radix-3 5-trit byte. B1's `Sentinel` enum
    /// NAMES that envelope; it does not mint a second one.
    ///
    /// This test is what makes the doc comment above the enum a law instead of
    /// a promise: if the two ever disagree on a single byte, the build goes red
    /// here rather than drifting silently into two homes for one boundary.
    #[test]
    fn the_sentinel_envelope_is_the_same_one_crate_zero_asserts() {
        use forge_core_v3::TritCell5D;

        for b in 0u16..=255 {
            let byte = b as u8;
            assert_eq!(
                Sentinel::from_u8(byte).is_some(),
                TritCell5D(byte).is_sentinel(),
                "byte {byte}: forge-material-v3 and forge-core-v3 disagree on whether \
                 this is a sentinel — the control envelope has drifted into two homes"
            );
        }

        // And the boundary itself, from both directions.
        assert_eq!(
            Sentinel::ALL.len(),
            (0u16..=255).filter(|b| TritCell5D(*b as u8).is_sentinel()).count(),
            "the named sentinels must exhaust Crate Zero's envelope, no more and no less"
        );
        assert!(
            !TritCell5D(MATERIAL_PHYSICAL_MAX).is_sentinel(),
            "MATERIAL_PHYSICAL_MAX must be the LAST physical byte, not the first sentinel"
        );
        assert!(
            TritCell5D(MATERIAL_PHYSICAL_MAX + 1).is_sentinel(),
            "the byte after MATERIAL_PHYSICAL_MAX must be the first sentinel"
        );
    }

    /// The trit codec refuses `243..=255` — no balanced-trit reading exists
    /// past the 5-digit base-3 ceiling.
    #[test]
    fn trit_codec_refuses_bytes_past_the_ceiling() {
        for b in (CRAFT_BYTE_MAX + 1)..=255 {
            assert_eq!(unpack5(b), None, "byte {b} should have no trit reading");
        }
    }

    /// The metal lane folds trits 0-1 to `0..=8` and round-trips through
    /// every base-3 pair.
    #[test]
    fn metal_lane_covers_all_nine_states() {
        let mut seen = std::collections::BTreeSet::new();
        for d0 in 0..3i8 {
            for d1 in 0..3i8 {
                let mut trits = [0i8; 5];
                trits[0] = d0 - 1;
                trits[1] = d1 - 1;
                let w = MaterialTrit8 { fill_pmy: 0, material_id: MATERIAL_ASH, craft: [pack5(trits), 0, 0, 0, 0] };
                let lane = w.metal_lane().expect("valid craft byte must yield a metal lane");
                assert!(lane <= 8);
                seen.insert(lane);
            }
        }
        assert_eq!(seen.len(), 9, "the metal lane did not cover all 9 states");
    }

    /// The metalness trit reads craft index 3 directly.
    #[test]
    fn metalness_trit_reads_index_three() {
        for (t3, expect) in [(-1i8, -1i8), (0, 0), (1, 1)] {
            let mut trits = [0i8; 5];
            trits[3] = t3;
            let w = MaterialTrit8 { fill_pmy: 0, material_id: MATERIAL_ASH, craft: [pack5(trits), 0, 0, 0, 0] };
            assert_eq!(w.metalness_trit(), Some(expect));
        }
    }

    /// `craft_trit` refuses an out-of-range index.
    #[test]
    fn craft_trit_refuses_index_past_twenty_five() {
        assert_eq!(MaterialTrit8::EMPTY.craft_trit(25), None);
        assert_eq!(MaterialTrit8::EMPTY.craft_trit(255), None);
    }
}
