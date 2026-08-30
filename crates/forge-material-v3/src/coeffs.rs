//! `Coeffs16` — bulk physical coefficients for the 64-tier material stack,
//! tranche A of the material tier ladder
//! (`.forge/brief-queue/A-material-ladder-BRIEF.md`).
//!
//! Pattern copied from `material.rs`: layout locks measured by rustc (L02),
//! `Option` refusal on decode, the E0080 const origin gate, bijection tests
//! (L07).

use forge_core_v3::EssenceId;

/// Full scale for the permyriad channels (friction, restitution):
/// `0..=10_000` maps `0.0..=1.0`.
pub const COEFFS_PMY_MAX: u16 = 10_000;

/// Bulk physical coefficients, 16 bytes, exact: density, friction and
/// restitution in permyriad, a signed monetary `cents` channel, a soul
/// handle, and an essence pillar. Field order is offset order — every byte
/// is a field, no padding hole. The offsets below are locked by rustc, not
/// prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coeffs16 {
    /// Density in milligrams per cubic millimetre. Full `u16` range is
    /// valid — no permyriad ceiling applies to this channel.
    pub density_mg_mm3: u16,
    /// Friction in permyriad, `0..=COEFFS_PMY_MAX`.
    pub friction_pmy: u16,
    /// Restitution in permyriad, `0..=COEFFS_PMY_MAX`.
    pub restitution_pmy: u16,
    /// Signed monetary value in cents. Full `i16` range is valid.
    pub cents: i16,
    /// The soul handle. Every `u16` bit pattern is a valid `SoulId`,
    /// `SoulId::ROOT` included.
    pub soul: forge_core_v3::SoulId,
    /// The essence pillar ordinal, `0..=4`. Must decode via
    /// `EssenceId::from_u8` or the word is refused.
    pub essence: u8,
    /// Zero until a later tranche defines these bytes — a nonzero reserved
    /// byte today is corruption, never forward-compatibility.
    pub reserved: [u8; 5],
}

impl Coeffs16 {
    /// The origin: zero density, zero friction, zero restitution, zero
    /// cents, the root soul, the `Canvas` pillar (ordinal 0), zero reserved
    /// tail.
    pub const ZERO: Self = Self {
        density_mg_mm3: 0,
        friction_pmy: 0,
        restitution_pmy: 0,
        cents: 0,
        soul: forge_core_v3::SoulId::ROOT,
        essence: 0,
        reserved: [0; 5],
    };

    /// True when every channel is inside its domain: `friction_pmy` and
    /// `restitution_pmy` are `<= COEFFS_PMY_MAX`, `essence` decodes via
    /// `EssenceId::from_u8`, and every reserved byte is zero.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.friction_pmy <= COEFFS_PMY_MAX
            && self.restitution_pmy <= COEFFS_PMY_MAX
            && EssenceId::from_u8(self.essence).is_some()
            && self.reserved[0] == 0
            && self.reserved[1] == 0
            && self.reserved[2] == 0
            && self.reserved[3] == 0
            && self.reserved[4] == 0
    }

    /// Pack into two little-endian u64 words. Byte layout is the struct
    /// layout: word 0 is density/friction/restitution/cents; word 1 is
    /// soul, essence, then the reserved tail.
    #[inline(always)]
    pub const fn encode(self) -> [u64; 2] {
        let w0 = self.density_mg_mm3 as u64
            | (self.friction_pmy as u64) << 16
            | (self.restitution_pmy as u64) << 32
            | (self.cents as u16 as u64) << 48;
        let mut w1 = self.soul.0 as u64 | (self.essence as u64) << 16;
        let mut i = 0;
        while i < 5 {
            w1 |= (self.reserved[i] as u64) << (24 + i * 8);
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
        let mut reserved = [0u8; 5];
        let mut i = 0;
        while i < 5 {
            reserved[i] = (w1 >> (24 + i * 8)) as u8;
            i += 1;
        }
        let c = Self {
            density_mg_mm3: w0 as u16,
            friction_pmy: (w0 >> 16) as u16,
            restitution_pmy: (w0 >> 32) as u16,
            cents: (w0 >> 48) as u16 as i16,
            soul: forge_core_v3::SoulId(w1 as u16),
            essence: (w1 >> 16) as u8,
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
const _: () = assert!(core::mem::size_of::<Coeffs16>() == 16);
const _: () = assert!(core::mem::align_of::<Coeffs16>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping fields keeps size 16 while
// silently reinterpreting every stored word.
const _: () = assert!(core::mem::offset_of!(Coeffs16, density_mg_mm3) == 0);
const _: () = assert!(core::mem::offset_of!(Coeffs16, friction_pmy) == 2);
const _: () = assert!(core::mem::offset_of!(Coeffs16, restitution_pmy) == 4);
const _: () = assert!(core::mem::offset_of!(Coeffs16, cents) == 6);
const _: () = assert!(core::mem::offset_of!(Coeffs16, soul) == 8);
const _: () = assert!(core::mem::offset_of!(Coeffs16, essence) == 10);
const _: () = assert!(core::mem::offset_of!(Coeffs16, reserved) == 11);

// Every one of the 16 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 2 + 2 + 2 + 1 + 5 == core::mem::size_of::<Coeffs16>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match Coeffs16::decode(Coeffs16::ZERO.encode()) {
        Some(w) => {
            assert!(w.density_mg_mm3 == 0 && w.cents == 0 && w.essence == 0);
            assert!(w.soul.is_root());
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::SoulId;

    /// L07 over the interior: every lattice word sample survives its own
    /// wire exactly, across every essence pillar.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        for essence in 0u8..=4 {
            for cents in [-1_000i16, 0, 1_000] {
                let w = Coeffs16 {
                    density_mg_mm3: 12_345,
                    friction_pmy: 2_500,
                    restitution_pmy: 7_500,
                    cents,
                    soul: SoulId(4_242),
                    essence,
                    reserved: [0; 5],
                };
                assert_eq!(Coeffs16::decode(w.encode()), Some(w), "essence={essence} cents={cents}");
            }
        }
    }

    /// L07 over the sentinels: 0 and COEFFS_PMY_MAX on both permyriad
    /// channels, `i16::MIN`/`i16::MAX` on cents, `SoulId::ROOT`/`MAX`.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for pmy in [0u16, COEFFS_PMY_MAX] {
            for cents in [i16::MIN, i16::MAX] {
                for soul in [SoulId::ROOT, SoulId::MAX] {
                    let w = Coeffs16 {
                        density_mg_mm3: u16::MAX,
                        friction_pmy: pmy,
                        restitution_pmy: pmy,
                        cents,
                        soul,
                        essence: 4,
                        reserved: [0; 5],
                    };
                    assert_eq!(Coeffs16::decode(w.encode()), Some(w));
                }
            }
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = Coeffs16 {
            density_mg_mm3: 500,
            friction_pmy: 5_000,
            restitution_pmy: 5_000,
            cents: 100,
            soul: SoulId(7),
            essence: 2,
            reserved: [0; 5],
        };
        assert!(Coeffs16::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            Coeffs16 { friction_pmy: COEFFS_PMY_MAX + 1, ..good },
            Coeffs16 { restitution_pmy: COEFFS_PMY_MAX + 1, ..good },
            Coeffs16 { essence: 5, ..good },
            Coeffs16 { essence: 255, ..good },
            Coeffs16 { reserved: [1, 0, 0, 0, 0], ..good },
            Coeffs16 { reserved: [0, 0, 0, 0, 1], ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(Coeffs16::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }

    /// The origin: `ZERO` round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        let w = Coeffs16::ZERO;
        assert_eq!(Coeffs16::decode(w.encode()), Some(w));
    }

    /// `essence` must decode via `EssenceId::from_u8` — every pillar ordinal
    /// is accepted, `5..=255` is refused.
    #[test]
    fn essence_refuses_past_the_pillar_ceiling() {
        for essence in 5u16..=255 {
            let w = Coeffs16 { essence: essence as u8, ..Coeffs16::ZERO };
            assert_eq!(Coeffs16::decode(w.encode()), None, "essence {essence} should be refused");
        }
    }
}
