//! Carrier → `Point5D` axis bindings — Gap #1 of `compiler_doctrine.md` v1.0.
//!
//! INV-7: the carrier→axis mapping is explicit in code, not lore. Layer 4 authority
//! (Rank 1, authored): this table IS the registry entry. Integer-only (INV-4).

use crate::dimensional_collapse::Point5D;

/// The five axes a carrier can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis5D {
    /// `x_mu` — spatial horizontal / crowd spread.
    X,
    /// `y_mu` — distance / depth / terrain.
    Y,
    /// `z_semantic` — identity / pitch / meaning.
    Z,
    /// `w_tick` — time / lineage / tempo.
    W,
    /// `theta_mdeg` — harmonic codeword / timbre.
    Theta,
}

/// An acoustic carrier family. Named by the doctrine's carrier table, not derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcousticCarrier {
    Voice,
    Bell,
    Drum,
    Horn,
    String,
    FluteReed,
    AnvilHammer,
    BonePipe,
    StoneHarp,
}

impl AcousticCarrier {
    /// Every carrier, in doctrine order — the registry is enumerable or it isn't one.
    pub const ALL: [Self; 9] = [
        Self::Voice,
        Self::Bell,
        Self::Drum,
        Self::Horn,
        Self::String,
        Self::FluteReed,
        Self::AnvilHammer,
        Self::BonePipe,
        Self::StoneHarp,
    ];

    /// The axes this carrier drives. `AnvilHammer` drives two (Z + W); the rest one.
    pub const fn axes(self) -> &'static [Axis5D] {
        match self {
            Self::Voice | Self::BonePipe => &[Axis5D::Z],
            Self::Bell => &[Axis5D::X],
            Self::Drum => &[Axis5D::W],
            Self::Horn | Self::StoneHarp => &[Axis5D::Y],
            Self::String | Self::FluteReed => &[Axis5D::Theta],
            Self::AnvilHammer => &[Axis5D::Z, Axis5D::W],
        }
    }

    /// The primary axis — the first one driven.
    pub const fn primary_axis(self) -> Axis5D {
        self.axes()[0]
    }
}

/// The emitting event's 5D seat. Axes the carrier does not drive pass straight
/// through, so one event under two carriers differs only where the carrier speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CarrierContext {
    pub x_mu: i64,
    pub y_mu: i64,
    pub z_semantic: i32,
    pub w_tick: u64,
    pub theta_mdeg: i32,
    /// Event magnitude, permyriad 0..=10000. Clamped, never wrapped.
    pub drive_pmy: i32,
}

// ── axis spans: how far full drive (10000 pmy) moves each axis, in that axis's units ──
const SPREAD_SPAN_MU: i64 = 10_000; // X: ±10 world units, the pan half-width
const DEPTH_SPAN_MU: i64 = 20_000; // Y: 20 world units of recede
const SEMANTIC_SPAN_DEG: i32 = 24; // Z: two octaves of scale degree
/// W: one quarter of lineage. The tick rate is NOT redeclared here — it is
/// `forge_harmonics::synthxml::MUSIC_TICKS_PER_QUARTER`, the one music-tick source.
const LINEAGE_SPAN_TICKS: u64 = forge_harmonics::synthxml::MUSIC_TICKS_PER_QUARTER as u64;
const TIMBRE_SPAN_MDEG: i32 = 180_000; // θ: half a turn of the codeword
/// Flute/reed sits lower in the overtone stack than a bowed string (doctrine:
/// "θ, lower overtone_pmy"), so it takes a third of the timbre span.
const REED_SPAN_MDEG: i32 = TIMBRE_SPAN_MDEG / 3;
/// Bone pipe is the low register — memory / decay — one octave under the root.
const BONE_PIPE_DROP_DEG: i32 = -12;

/// Scale `span` by `drive_pmy` (0..=10000), integer-exact, truncating toward zero.
const fn scaled_i64(span: i64, drive_pmy: i32) -> i64 {
    let d = if drive_pmy < 0 {
        0
    } else if drive_pmy > 10_000 {
        10_000
    } else {
        drive_pmy
    } as i64;
    span * d / 10_000
}

const fn scaled_i32(span: i32, drive_pmy: i32) -> i32 {
    scaled_i64(span as i64, drive_pmy) as i32
}

/// A carrier event → the `Point5D` that `collapse_5d_to_stereo` consumes directly.
///
/// Pure and deterministic: same carrier + same context ⇒ same point, no float, no
/// clock read. The carrier writes ONLY its own axes (INV-7); everything else is the
/// context's seat.
pub fn carrier_to_point5d(carrier: AcousticCarrier, ctx: CarrierContext) -> Point5D {
    let mut p = Point5D {
        x_mu: ctx.x_mu,
        y_mu: ctx.y_mu,
        z_semantic: ctx.z_semantic,
        w_tick: ctx.w_tick,
        theta_mdeg: ctx.theta_mdeg,
    };
    let d = ctx.drive_pmy;
    for axis in carrier.axes() {
        match axis {
            Axis5D::X => p.x_mu = ctx.x_mu.saturating_add(scaled_i64(SPREAD_SPAN_MU, d)),
            Axis5D::Y => p.y_mu = ctx.y_mu.saturating_add(scaled_i64(DEPTH_SPAN_MU, d)),
            Axis5D::Z => {
                let drop =
                    if matches!(carrier, AcousticCarrier::BonePipe) { BONE_PIPE_DROP_DEG } else { 0 };
                p.z_semantic = ctx
                    .z_semantic
                    .saturating_add(scaled_i32(SEMANTIC_SPAN_DEG, d))
                    .saturating_add(drop);
            }
            Axis5D::W => {
                p.w_tick = ctx.w_tick.saturating_add(scaled_i64(LINEAGE_SPAN_TICKS as i64, d) as u64)
            }
            Axis5D::Theta => {
                let span = if matches!(carrier, AcousticCarrier::FluteReed) {
                    REED_SPAN_MDEG
                } else {
                    TIMBRE_SPAN_MDEG
                };
                p.theta_mdeg = ctx.theta_mdeg.saturating_add(scaled_i32(span, d)).rem_euclid(360_000)
            }
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimensional_collapse::collapse_5d_to_stereo;

    fn ctx(drive_pmy: i32) -> CarrierContext {
        CarrierContext { z_semantic: 12, w_tick: 100, drive_pmy, ..Default::default() }
    }

    /// The doctrine table, read back off the code. If this drifts, INV-7 is dead.
    #[test]
    fn carrier_5d_table_matches_the_doctrine() {
        use AcousticCarrier::*;
        use Axis5D::*;
        let want: [(AcousticCarrier, &[Axis5D]); 9] = [
            (Voice, &[Z]),
            (Bell, &[X]),
            (Drum, &[W]),
            (Horn, &[Y]),
            (String, &[Theta]),
            (FluteReed, &[Theta]),
            (AnvilHammer, &[Z, W]),
            (BonePipe, &[Z]),
            (StoneHarp, &[Y]),
        ];
        for (c, axes) in want {
            assert_eq!(c.axes(), axes, "{c:?} drives the wrong axis");
        }
        assert_eq!(AcousticCarrier::ALL.len(), want.len(), "a carrier is missing from ALL");
    }

    /// Every carrier lands a point the collapse can consume, and lands the SAME one twice.
    #[test]
    fn carrier_5d_is_total_and_deterministic() {
        for c in AcousticCarrier::ALL {
            let p = carrier_to_point5d(c, ctx(5_000));
            assert_eq!(p, carrier_to_point5d(c, ctx(5_000)), "{c:?} is not deterministic");
            let f = collapse_5d_to_stereo(p, 48_000);
            assert!(f.root_freq_mhz > 0, "{c:?} collapsed to a dead fundamental");
        }
    }

    /// A carrier writes its own axes and nothing else — the seat is the context's.
    #[test]
    fn carrier_5d_touches_only_its_own_axes() {
        let base = ctx(0);
        for c in AcousticCarrier::ALL {
            let p = carrier_to_point5d(c, ctx(10_000));
            let drives = |a: Axis5D| c.axes().contains(&a);
            if !drives(Axis5D::X) {
                assert_eq!(p.x_mu, base.x_mu, "{c:?} moved X");
            }
            if !drives(Axis5D::Y) {
                assert_eq!(p.y_mu, base.y_mu, "{c:?} moved Y");
            }
            if !drives(Axis5D::Z) {
                assert_eq!(p.z_semantic, base.z_semantic, "{c:?} moved Z");
            }
            if !drives(Axis5D::W) {
                assert_eq!(p.w_tick, base.w_tick, "{c:?} moved W");
            }
            if !drives(Axis5D::Theta) {
                assert_eq!(p.theta_mdeg, base.theta_mdeg, "{c:?} moved θ");
            }
        }
    }

    /// Anvil/hammer is the two-axis case: resonance pitch AND rhythmic lineage.
    #[test]
    fn carrier_5d_anvil_drives_pitch_and_lineage() {
        let p = carrier_to_point5d(AcousticCarrier::AnvilHammer, ctx(10_000));
        assert_eq!(p.z_semantic, 12 + SEMANTIC_SPAN_DEG);
        assert_eq!(p.w_tick, 100 + LINEAGE_SPAN_TICKS);
    }

    /// Bone pipe is the low register: same axis as Voice, an octave under it.
    #[test]
    fn carrier_5d_bone_pipe_sits_an_octave_below_voice() {
        let voice = carrier_to_point5d(AcousticCarrier::Voice, ctx(2_500));
        let bone = carrier_to_point5d(AcousticCarrier::BonePipe, ctx(2_500));
        assert_eq!(bone.z_semantic, voice.z_semantic + BONE_PIPE_DROP_DEG);
    }

    /// Reed sits lower in the overtone stack than the bowed string (doctrine).
    #[test]
    fn carrier_5d_reed_is_a_shorter_timbre_arc_than_string() {
        let s = carrier_to_point5d(AcousticCarrier::String, ctx(10_000));
        let r = carrier_to_point5d(AcousticCarrier::FluteReed, ctx(10_000));
        assert!(r.theta_mdeg < s.theta_mdeg, "reed {r:?} should trail string {s:?}");
    }

    /// Drive is clamped, never wrapped: out-of-range input cannot fling an axis.
    #[test]
    fn carrier_5d_drive_is_clamped() {
        let hi = carrier_to_point5d(AcousticCarrier::Drum, ctx(999_999));
        let full = carrier_to_point5d(AcousticCarrier::Drum, ctx(10_000));
        assert_eq!(hi.w_tick, full.w_tick);
        let lo = carrier_to_point5d(AcousticCarrier::Drum, ctx(-500));
        assert_eq!(lo.w_tick, 100, "negative drive moved the lineage");
    }
}
