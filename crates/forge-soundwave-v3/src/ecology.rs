//! `EcologyPCM8` — the 8-byte soundwave-ecology word, T7 of the forge-vision
//! drain (plan-2-welds.md T7, quarried from v2's vision_spectral /
//! vision_pcm / vision_defects boundary encoding — the encoding is new, the
//! discipline is drained; see brief-queue/T7-ecologypcm8-BRIEF.md).
//!
//! Machine-first (L08): every channel is an exact integer permyriad or an
//! opaque bit word; no float ever touches the encode/decode path.
//!
//! ONE HOME (L05): this file is the only definition of `EcologyPCM8`.

/// Full scale for the permyriad channels (altitude, slope): `0..=10_000`.
/// Matches `ColourTrit8::PMY_MAX` [observed forge-colour-v3/src/trit.rs:23].
pub const PMY_MAX: u16 = 10_000;

/// Slope's neutral point — permyriad 5_000 encodes slope 0.0 (flat), 0
/// encodes -1.0, `PMY_MAX` encodes +1.0. `[ASSUMED]` the brief marks the
/// 5000-neutral offset encoding as an assumption (T7-ecologypcm8-BRIEF.md
/// section 3/"Design notes"); no quarried source pins this constant, it is
/// chosen for symmetry with the permyriad domain.
pub const SLOPE_NEUTRAL: u16 = 5_000;

/// One soundwave-ecology texel, 8 bytes, exact: altitude, slope, and an
/// opaque event-flag word. Field order is offset order — every byte is a
/// field, no padding hole. The offsets below are locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EcologyPCM8 {
    /// Depth/altitude in permyriad, `0..=PMY_MAX`, `[0, 1]`.
    pub altitude_pmy: u16,
    /// Slope in permyriad, `0..=PMY_MAX`, offset-encoded around
    /// `SLOPE_NEUTRAL` for the `[-1, +1]` domain. `[ASSUMED]` encoding — see
    /// `SLOPE_NEUTRAL`.
    pub slope_pmy: u16,
    /// Habitat-discontinuity event flags. Opaque u16: this word roundtrips
    /// whatever bits arrive and invents no meaning for them. event_flags
    /// schema is deferred by ARCH000 per L17 (T7-ecologypcm8-BRIEF.md
    /// "Design notes" / "Effort") — every bit pattern `0..=65535` is valid,
    /// none is refused, until a schema names the bits.
    pub event_flags: u16,
    /// Reserved. Must be zero — a nonzero reserved word is corruption, never
    /// forward-compatibility (L05 one-home, no hidden channel).
    pub reserved: u16,
}

impl EcologyPCM8 {
    /// The origin: altitude 0, slope neutral, no event flags, no reserved
    /// bits. Named in the brief's "Bijection Gate" section 3 as the
    /// compile-time E0080 gate point.
    pub const ORIGIN: Self =
        Self { altitude_pmy: 0, slope_pmy: SLOPE_NEUTRAL, event_flags: 0, reserved: 0 };

    /// True when every channel is inside its domain: altitude and slope are
    /// `<= PMY_MAX`, reserved is zero. `event_flags` is opaque and always
    /// valid — no bit pattern is refused until a schema constrains it.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.altitude_pmy <= PMY_MAX && self.slope_pmy <= PMY_MAX && self.reserved == 0
    }

    /// Pack into one little-endian u64 word. Byte layout is the struct
    /// layout: altitude, slope, event_flags, reserved.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.altitude_pmy as u64
            | (self.slope_pmy as u64) << 16
            | (self.event_flags as u64) << 32
            | (self.reserved as u64) << 48
    }

    /// Unpack a word. `None` for anything outside the valid domain — an
    /// out-of-range permyriad channel or a live reserved word is corruption
    /// refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self {
            altitude_pmy: word as u16,
            slope_pmy: (word >> 16) as u16,
            event_flags: (word >> 32) as u16,
            reserved: (word >> 48) as u16,
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<EcologyPCM8>() == 8);
const _: () = assert!(core::mem::align_of::<EcologyPCM8>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 8 while
// silently reinterpreting every stored texel.
const _: () = assert!(core::mem::offset_of!(EcologyPCM8, altitude_pmy) == 0);
const _: () = assert!(core::mem::offset_of!(EcologyPCM8, slope_pmy) == 2);
const _: () = assert!(core::mem::offset_of!(EcologyPCM8, event_flags) == 4);
const _: () = assert!(core::mem::offset_of!(EcologyPCM8, reserved) == 6);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(2 + 2 + 2 + 2 == core::mem::size_of::<EcologyPCM8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match EcologyPCM8::decode(EcologyPCM8::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.altitude_pmy == 0 && w.slope_pmy == SLOPE_NEUTRAL);
            assert!(w.event_flags == 0 && w.reserved == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 over the interior: altitude and slope lattice samples, crossed
    /// with the event-flag samples named in the brief's "Bijection Gate"
    /// section 1.
    #[test]
    fn bijection_holds_over_the_interior() {
        let altitudes = [1u16, 2_500, 5_000, 7_500, 9_999];
        let slopes = [1u16, 2_500, 5_000, 7_500, 9_999];
        let flags = [0u16, 1, 256, 512, 1024, 4095, 8191, 16383];
        for altitude in altitudes {
            for slope in slopes {
                for flag in flags {
                    let e = EcologyPCM8 {
                        altitude_pmy: altitude,
                        slope_pmy: slope,
                        event_flags: flag,
                        reserved: 0,
                    };
                    assert_eq!(
                        EcologyPCM8::decode(e.encode()),
                        Some(e),
                        "altitude={altitude} slope={slope} flags={flag}"
                    );
                }
            }
        }
    }

    /// L07 over the sentinels: altitude 0/PMY_MAX x slope 0 (-1 edge) /
    /// SLOPE_NEUTRAL / PMY_MAX (+1 edge) x event_flags 0 and 65535 (the
    /// opaque word's full-bit sentinel).
    #[test]
    fn bijection_holds_over_the_sentinels() {
        for altitude in [0u16, PMY_MAX] {
            for slope in [0u16, SLOPE_NEUTRAL, PMY_MAX] {
                for flag in [0u16, u16::MAX] {
                    let e = EcologyPCM8 {
                        altitude_pmy: altitude,
                        slope_pmy: slope,
                        event_flags: flag,
                        reserved: 0,
                    };
                    assert_eq!(
                        EcologyPCM8::decode(e.encode()),
                        Some(e),
                        "altitude={altitude} slope={slope} flags={flag}"
                    );
                }
            }
        }
    }

    /// The origin: altitude 0, slope neutral, no flags, round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        assert_eq!(EcologyPCM8::decode(EcologyPCM8::ORIGIN.encode()), Some(EcologyPCM8::ORIGIN));
    }

    /// L07 over the slope +-1 edge: slope saturates at 0 (-1) and PMY_MAX
    /// (+1) [ASSUMED encoding, SLOPE_NEUTRAL doc comment] and survives its
    /// wire crossed with the altitude interior samples.
    #[test]
    fn slope_edges_survive_the_wire_over_the_altitude_interior() {
        for slope in [0u16, PMY_MAX] {
            for altitude in [1u16, 2_500, 5_000, 7_500, 9_999] {
                let e = EcologyPCM8 { altitude_pmy: altitude, slope_pmy: slope, event_flags: 0, reserved: 0 };
                assert_eq!(EcologyPCM8::decode(e.encode()), Some(e), "altitude={altitude} slope={slope}");
            }
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None.
    /// `event_flags` carries no refusal row — it is opaque and its schema is
    /// deferred (see the field's doc comment) — every bit pattern is valid.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = EcologyPCM8 { altitude_pmy: 5_000, slope_pmy: 5_000, event_flags: 4_095, reserved: 0 };
        assert!(EcologyPCM8::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            EcologyPCM8 { altitude_pmy: PMY_MAX + 1, ..good },
            EcologyPCM8 { slope_pmy: PMY_MAX + 1, ..good },
            EcologyPCM8 { reserved: 1, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(EcologyPCM8::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }

    /// W04 Mythos-anchor: Void Marshes terrain ecology (lore tie).
    /// The Void Marshes, a named world zone, encodes its terrain as a
    /// permyriad altitude (3000, half-depth) and neutral slope. This test
    /// anchors that lore claim: the ecology word survives its wire.
    /// [OBSERVED] fabric: EcologyPCM8 encode/decode roundtrip, L07 bijection.
    #[test]
    fn void_marshes_ecology_lore_tie_survives_wire() {
        let void_marshes = EcologyPCM8 {
            altitude_pmy: 3_000,  // Half-depth into the void
            slope_pmy: SLOPE_NEUTRAL,  // Flat terrain
            event_flags: 0,  // No events yet
            reserved: 0,  // Must be zero
        };
        assert!(void_marshes.is_valid(), "Void Marshes ecology is out of domain");
        let encoded = void_marshes.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(void_marshes), "Void Marshes ecology failed round-trip");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// Thornhaven Market terrain ecology (lore tie), distinct named zone
    /// from `void_marshes_ecology_lore_tie_survives_wire` above — asset ref
    /// `assets/ironroot/Good/location/thornhaven_market_golden.png`. A
    /// surface-level town square encodes near-zero altitude (not void-depth)
    /// and neutral (flat, paved) slope. [OBSERVED] fabric: EcologyPCM8
    /// encode/decode roundtrip, L07 bijection.
    #[test]
    fn thornhaven_market_ecology_lore_tie_survives_wire() {
        let thornhaven_market = EcologyPCM8 {
            altitude_pmy: 500,  // Near-surface, a town square, not void-depth
            slope_pmy: SLOPE_NEUTRAL,  // Flat, paved market ground
            event_flags: 0,  // No events yet
            reserved: 0,  // Must be zero
        };
        assert!(thornhaven_market.is_valid(), "Thornhaven Market ecology is out of domain");
        let encoded = thornhaven_market.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(thornhaven_market), "Thornhaven Market ecology failed round-trip");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// Hollowden Pack territory ecology (lore tie), a third distinct named
    /// zone from the two above — asset ref `assets/ironroot/Good/
    /// faction-banner/hollowden_pack_banner_golden.png`. Unlike Void Marshes
    /// (flat, mid-depth) and Thornhaven Market (flat, surface), the Pack's
    /// forested ridge territory is the first lore tie to exercise a
    /// non-neutral slope — an uphill grade, `SLOPE_NEUTRAL`-offset toward
    /// the `+1` (uphill) edge, not the flat plateau the prior two zones
    /// share. [OBSERVED] fabric: EcologyPCM8 encode/decode roundtrip, L07
    /// bijection over the slope domain (already proven generically by
    /// `slope_edges_survive_the_wire_over_the_altitude_interior` above; this
    /// test is the first to tie a non-neutral slope to a NAMED zone).
    #[test]
    fn hollowden_pack_ecology_lore_tie_survives_wire() {
        let hollowden_pack = EcologyPCM8 {
            altitude_pmy: 6_000,  // Elevated forest ridge, above surface-level
            slope_pmy: 7_000,     // Uphill grade toward the +1 edge (not flat)
            event_flags: 0,       // No events yet
            reserved: 0,          // Must be zero
        };
        assert!(hollowden_pack.is_valid(), "Hollowden Pack ecology is out of domain");
        assert!(hollowden_pack.slope_pmy != SLOPE_NEUTRAL, "this brick's whole point is a non-neutral slope");
        let encoded = hollowden_pack.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(hollowden_pack), "Hollowden Pack ecology failed round-trip");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// the Cinderfall Breach — a fourth distinct named zone, and the first
    /// lore tie to exercise a NONZERO `event_flags` word. The three zones
    /// above are all flag-quiet (`event_flags: 0`, "no events yet"); this
    /// one carries an active habitat-discontinuity event. Per the field's
    /// own doc comment the bit schema is deferred (ARCH000/L17) — this test
    /// claims only that a nonzero event word is present and opaque, never
    /// what any specific bit means, so it doesn't invent a schema. [OBSERVED]
    /// fabric: `EcologyPCM8` encode/decode roundtrip, `event_flags` opacity
    /// (every bit pattern valid, `is_valid`/`decode` never refuse on this
    /// field), same as `out_of_domain_words_are_refused` already proves.
    #[test]
    fn cinderfall_breach_ecology_lore_tie_carries_an_active_event() {
        let cinderfall_breach = EcologyPCM8 {
            altitude_pmy: 2_500,  // Shallow crater rim, below surface-level
            slope_pmy: 3_000,     // Downhill toward the -1 edge, into the breach
            event_flags: 1,       // A habitat-discontinuity event is active; bit meaning undeclared
            reserved: 0,          // Must be zero
        };
        assert!(cinderfall_breach.is_valid(), "Cinderfall Breach ecology is out of domain");
        assert_ne!(cinderfall_breach.event_flags, 0, "this brick's whole point is an active event flag");
        let encoded = cinderfall_breach.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(cinderfall_breach), "Cinderfall Breach ecology failed round-trip, event flag lost");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// the Skyreach Pinnacle — a fifth distinct named zone, and the first
    /// lore tie to sit at the domain's extreme edge (`PMY_MAX`) rather than
    /// an interior sample. The four zones above (Void Marshes, Thornhaven
    /// Market, Hollowden Pack, Cinderfall Breach) all used interior
    /// altitude/slope values; a mountain's true summit is the one place the
    /// channel's own ceiling is the correct, literal encoding — not a
    /// rounding accident. [OBSERVED] fabric: `EcologyPCM8` encode/decode
    /// roundtrip, `is_valid`'s own `<= PMY_MAX` boundary (already proven
    /// generically by `bijection_holds_over_the_sentinels`; this test is the
    /// first to tie that exact sentinel to a NAMED zone).
    #[test]
    fn skyreach_pinnacle_ecology_lore_tie_survives_wire_at_the_ceiling() {
        let skyreach_pinnacle = EcologyPCM8 {
            altitude_pmy: PMY_MAX, // the literal summit — the channel's own ceiling, not a rounding accident
            slope_pmy: 9_500,      // a near-vertical final approach
            event_flags: 0,        // no events yet
            reserved: 0,           // must be zero
        };
        assert!(skyreach_pinnacle.is_valid(), "Skyreach Pinnacle ecology is out of domain");
        assert_eq!(skyreach_pinnacle.altitude_pmy, PMY_MAX, "this brick's whole point is the literal ceiling");
        let encoded = skyreach_pinnacle.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(skyreach_pinnacle), "Skyreach Pinnacle ecology failed round-trip at the ceiling");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11): a
    /// corrupted scribe's rumor about the abyss — the first lore tie built
    /// around the REFUSAL law rather than a successful round-trip. The five
    /// zones above all encode real, valid places; a garbled secondhand
    /// account (a rumor, not a survey) is the honest in-world reason a
    /// record might carry an out-of-domain altitude — and the channel must
    /// refuse it rather than silently accept corrupted lore as canon.
    /// Anchors to the already-landed `is_valid`/`decode` boundary refusal.
    /// [OBSERVED] fabric: `EcologyPCM8::decode`, already proven generically
    /// by `out_of_domain_words_are_refused`; this test is the first to frame
    /// that refusal as a named in-world claim rather than an abstract bound.
    #[test]
    fn corrupted_scribes_rumor_ecology_lore_tie_is_correctly_refused() {
        let garbled_rumor = EcologyPCM8 {
            altitude_pmy: PMY_MAX + 1, // the scribe misheard "half-depth" as "past the floor of the world"
            slope_pmy: SLOPE_NEUTRAL,
            event_flags: 0,
            reserved: 0,
        };
        assert!(!garbled_rumor.is_valid(), "a garbled rumor must not read as a valid ecology record");
        let decoded = EcologyPCM8::decode(garbled_rumor.encode());
        assert_eq!(decoded, None, "the channel must refuse corrupted lore, never silently canonize it");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// the Precipice of Null — an eighth named zone, and the first lore tie
    /// to sit at the literal `slope_pmy: 0` sentinel (the domain's `-1`
    /// edge, a sheer downward drop) rather than an interior slope sample.
    /// The prior slope-bearing ties (Hollowden Pack 7000, Cinderfall Breach
    /// 3000, Drowned Gate 4000) were all interior values; none touched this
    /// exact edge. HONEST NOTE: `EcologyPCM8`'s three meaningful fields
    /// (altitude/slope/event_flags) now have every literal edge AND the
    /// refusal path named to a real zone — this is very likely the last
    /// distinct angle available in this 8-byte struct without genuinely
    /// repeating a prior claim; a future Audio-lane tick should treat an
    /// uncited module or crate as owed before adding a ninth tie here.
    /// [OBSERVED] fabric: `EcologyPCM8` encode/decode roundtrip, the `0`
    /// slope sentinel already proven generically by
    /// `bijection_holds_over_the_sentinels`.
    #[test]
    fn precipice_of_null_ecology_lore_tie_survives_wire_at_the_sheer_edge() {
        let precipice_of_null = EcologyPCM8 {
            altitude_pmy: 5_000, // mid-height, right at the lip
            slope_pmy: 0,        // the literal -1 edge — a sheer, vertical drop
            event_flags: 0,
            reserved: 0,
        };
        assert!(precipice_of_null.is_valid(), "Precipice of Null ecology is out of domain");
        assert_eq!(precipice_of_null.slope_pmy, 0, "this brick's whole point is the sheer-edge sentinel");
        let encoded = precipice_of_null.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(precipice_of_null), "Precipice of Null ecology failed round-trip at the sheer edge");
    }

    /// W04 Mythos-anchor (world-builder brick, Audio lane float per W11):
    /// the Drowned Gate — a seventh distinct named zone, and the first lore
    /// tie to sit at the literal `altitude_pmy: 0` floor rather than any
    /// interior or top-sentinel sample. The six ties above range from 500
    /// through `PMY_MAX`; none touched the domain's other edge. A gate at
    /// exactly sea-level/surface-zero is the one place `0` is the correct,
    /// literal encoding of "right at the waterline," not an uninitialized
    /// default. [OBSERVED] fabric: `EcologyPCM8` encode/decode roundtrip,
    /// the `0` sentinel already proven generically by
    /// `bijection_holds_over_the_sentinels`; this test is the first to tie
    /// it to a NAMED zone.
    #[test]
    fn drowned_gate_ecology_lore_tie_survives_wire_at_the_floor() {
        let drowned_gate = EcologyPCM8 {
            altitude_pmy: 0,   // exactly at the waterline — the literal floor, not a default
            slope_pmy: 4_000,  // a gentle downward slope into the water
            event_flags: 0,
            reserved: 0,
        };
        assert!(drowned_gate.is_valid(), "Drowned Gate ecology is out of domain");
        assert_eq!(drowned_gate.altitude_pmy, 0, "this brick's whole point is the literal floor");
        let encoded = drowned_gate.encode();
        let decoded = EcologyPCM8::decode(encoded);
        assert_eq!(decoded, Some(drowned_gate), "Drowned Gate ecology failed round-trip at the floor");
    }
}
