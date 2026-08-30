//! Astrolabe Resonance & Celestial World Clock.
//!
//! Ported & hardened from `F:\AKWEB\forge-starpy-v3\resonance_engine.py` into fixed-point
//! permyriad integer math. Computes astrological aspect resonance, 13-Moons phase cycles,
//! and DM encounter volatility modifiers.

#![deny(unsafe_code)]

use forge_mud_v3::ironroot::brand_aspect::angular_distance_mdeg;
use serde::{Deserialize, Serialize};

/// Classical major astrological aspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CelestialAspect {
    /// 0° - Fusion (Highest harmonic resonance, 9,000 pmy).
    Conjunct,
    /// 120° - Flow / Harmony (8,500 pmy).
    Trine,
    /// 60° - Opportunity (8,000 pmy).
    Sextile,
    /// 180° - Tension / Polarization (7,500 pmy).
    Oppose,
    /// 90° - Friction / Conflict (7,000 pmy).
    Square,
}

impl CelestialAspect {
    /// Aspect multiplier in permyriad (1.0 = 10,000 pmy).
    pub const fn multiplier_pmy(&self) -> u32 {
        match self {
            Self::Conjunct => 9_000,
            Self::Trine => 8_500,
            Self::Sextile => 8_000,
            Self::Oppose => 7_500,
            Self::Square => 7_000,
        }
    }

    /// Exact separation this aspect names, in milli-degrees.
    pub const fn exact_mdeg(&self) -> i32 {
        match self {
            Self::Conjunct => 0,
            Self::Sextile => 60_000,
            Self::Square => 90_000,
            Self::Trine => 120_000,
            Self::Oppose => 180_000,
        }
    }

    /// Orb allowed either side of `exact_mdeg`, in milli-degrees.
    pub const fn orb_mdeg(&self) -> i32 {
        match self {
            Self::Conjunct => 8_000,
            Self::Trine => 8_000,
            Self::Oppose => 8_000,
            Self::Square => 7_000,
            Self::Sextile => 6_000,
        }
    }
}

/// The five aspects `CelestialAspect` names, in ascending exact separation.
pub const MAJOR_ASPECTS: [CelestialAspect; 5] = [
    CelestialAspect::Conjunct,
    CelestialAspect::Sextile,
    CelestialAspect::Square,
    CelestialAspect::Trine,
    CelestialAspect::Oppose,
];

/// Celestial bodies tracked by the Astrolabe engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CelestialBody {
    /// The solar core.
    Sun,
    /// The lunar mirror.
    Moon,
    /// The messenger and swift logic.
    Mercury,
    /// The harmonic balancer.
    Venus,
    /// The warrior and drive catalyst.
    Mars,
    /// The expansive sovereign.
    Jupiter,
    /// The boundary and timekeeper.
    Saturn,
    /// The rising horizon orientation.
    Ascendant,
}

/// A weighted planetary influence (weight in permyriad 0..10,000).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanetaryInfluence {
    /// The active celestial body.
    pub body: CelestialBody,
    /// Weight in permyriad (e.g. 10,000 = 1.0, 5,000 = 0.5).
    pub weight_pmy: u32,
}

/// 13-Moons calendar phase (0..12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonPhase {
    /// Moon index in the 13-moon annual cycle (0..12).
    pub moon_index: u8,
    /// Day within the 28-day lunar cycle (1..28).
    pub lunar_day: u8,
}

impl MoonPhase {
    /// Creates a new moon phase validated against the 13-Moons invariant.
    pub fn new(moon_index: u8, lunar_day: u8) -> Self {
        Self {
            moon_index: moon_index.clamp(0, 12),
            lunar_day: lunar_day.clamp(1, 28),
        }
    }

    /// Returns the lunar quarter (0 = New, 1 = Waxing, 2 = Full, 3 = Waning).
    pub fn quarter(&self) -> u8 {
        (self.lunar_day - 1) / 7
    }

    /// Whether the moon is at full apex (days 14..16).
    pub fn is_full_apex(&self) -> bool {
        self.lunar_day >= 14 && self.lunar_day <= 16
    }
}

// Free-longitude aspect producer. Angle folding is reused from
// forge-mud-v3 `ironroot::brand_aspect`; its single 1000 mdeg tolerance is not,
// because that orb is sized for exact 30-degree seats, not for real positions.

/// A body's ecliptic longitude in milli-degrees from 0 degrees Aries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyLongitude {
    /// The body standing at this longitude.
    pub body: CelestialBody,
    /// Ecliptic longitude, milli-degrees; any integer, folded on use.
    pub lon_mdeg: i32,
}

/// One resolved aspect between two bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectHit {
    /// The aspect the separation resolved to.
    pub aspect: CelestialAspect,
    /// Folded separation between the two longitudes, 0..=180,000 mdeg.
    pub separation_mdeg: i32,
    /// Distance from the aspect's exact angle, 0..=`orb_mdeg`.
    pub orb_deviation_mdeg: i32,
}

/// Resolves two ecliptic longitudes to a named aspect. Tightest orb wins;
/// `None` when the separation falls in no aspect's orb.
pub fn aspect_between_mdeg(a_lon_mdeg: i32, b_lon_mdeg: i32) -> Option<AspectHit> {
    let separation_mdeg = angular_distance_mdeg(a_lon_mdeg, b_lon_mdeg);
    let mut best: Option<AspectHit> = None;

    for aspect in MAJOR_ASPECTS {
        let orb_deviation_mdeg = (separation_mdeg - aspect.exact_mdeg()).abs();
        if orb_deviation_mdeg > aspect.orb_mdeg() {
            continue;
        }
        let tighter = match best {
            Some(held) => orb_deviation_mdeg < held.orb_deviation_mdeg,
            None => true,
        };
        if tighter {
            best = Some(AspectHit { aspect, separation_mdeg, orb_deviation_mdeg });
        }
    }

    best
}

/// Writes every pairwise aspect found in `bodies` into `out`, returning the
/// count written. Stops at `out.len()`; allocates nothing.
pub fn chart_aspects(bodies: &[BodyLongitude], out: &mut [AspectHit]) -> usize {
    let mut written = 0;

    for (i, a) in bodies.iter().enumerate() {
        for b in &bodies[i + 1..] {
            if written == out.len() {
                return written;
            }
            if let Some(hit) = aspect_between_mdeg(a.lon_mdeg, b.lon_mdeg) {
                out[written] = hit;
                written += 1;
            }
        }
    }

    written
}

/// Evaluates total celestial resonance across active aspects and planetary weights.
///
/// Output is clamped to 0..10,000 permyriad.
pub fn calculate_resonance_pmy(
    aspects: &[CelestialAspect],
    influences: &[PlanetaryInfluence],
) -> u32 {
    if aspects.is_empty() || influences.is_empty() {
        return 0;
    }

    // 1. Average aspect multiplier in permyriad
    let aspect_sum: u64 = aspects.iter().map(|a| a.multiplier_pmy() as u64).sum();
    let avg_aspect_pmy = (aspect_sum / aspects.len() as u64) as u32;

    // 2. Average planet weight in permyriad
    let weight_sum: u64 = influences.iter().map(|p| p.weight_pmy as u64).sum();
    let avg_weight_pmy = (weight_sum / influences.len() as u64) as u32;

    // 3. Composite score = (avg_weight_pmy * avg_aspect_pmy) / 10,000
    let final_pmy = ((avg_weight_pmy as u64 * avg_aspect_pmy as u64) / 10_000) as u32;
    final_pmy.min(10_000)
}

/// DM Encounter volatility modifier calculated from celestial resonance and moon phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmCelestialModifiers {
    /// Resonance score in permyriad (0..10,000).
    pub resonance_pmy: u32,
    /// Predator/Wolf aggression scale in permyriad (10,000 = baseline 100%).
    pub aggression_scale_pmy: u32,
    /// Trader barter generosity scale in permyriad (10,000 = baseline 100%).
    pub barter_scale_pmy: u32,
    /// Narrative tension tier (Low, Balanced, High, Peak).
    pub tension_tier: &'static str,
}

/// Computes DM modifiers given the current celestial configuration.
pub fn compute_dm_modifiers(
    aspects: &[CelestialAspect],
    influences: &[PlanetaryInfluence],
    moon: &MoonPhase,
) -> DmCelestialModifiers {
    let res_pmy = calculate_resonance_pmy(aspects, influences);

    // Full moon adds +2,000 pmy aggression, high resonance stabilizes barter
    let full_moon_boost = if moon.is_full_apex() { 2_000 } else { 0 };
    let aggression = 10_000 + full_moon_boost + (10_000u32.saturating_sub(res_pmy) / 5);
    let barter = 8_000 + (res_pmy / 5);

    let tension_tier = if aggression >= 12_500 {
        "Peak Volatility (Full Moon Hunt)"
    } else if aggression >= 11_000 {
        "High Tension"
    } else if res_pmy >= 8_000 {
        "Harmonic Alignment"
    } else {
        "Balanced Baseline"
    };

    DmCelestialModifiers {
        resonance_pmy: res_pmy,
        aggression_scale_pmy: aggression,
        barter_scale_pmy: barter,
        tension_tier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit_buffer<const N: usize>() -> [AspectHit; N] {
        [AspectHit {
            aspect: CelestialAspect::Conjunct,
            separation_mdeg: 0,
            orb_deviation_mdeg: 0,
        }; N]
    }

    #[test]
    fn exact_angles_resolve_to_their_own_aspect() {
        for aspect in MAJOR_ASPECTS {
            let hit = aspect_between_mdeg(0, aspect.exact_mdeg())
                .unwrap_or_else(|| panic!("{aspect:?} must resolve at its exact angle"));
            assert_eq!(hit.aspect, aspect);
            assert_eq!(hit.orb_deviation_mdeg, 0);
            assert_eq!(hit.separation_mdeg, aspect.exact_mdeg());
        }
    }

    #[test]
    fn the_orb_edge_holds_and_one_mdeg_past_it_does_not() {
        for aspect in MAJOR_ASPECTS {
            let edge = aspect.exact_mdeg() - aspect.orb_mdeg();
            if edge >= 0 {
                let hit = aspect_between_mdeg(0, edge).expect("the orb edge is inside");
                assert_eq!(hit.aspect, aspect);
                assert_eq!(hit.orb_deviation_mdeg, aspect.orb_mdeg());
            }
            let past = aspect.exact_mdeg() - aspect.orb_mdeg() - 1;
            if past >= 0 {
                let outside = aspect_between_mdeg(0, past);
                assert!(
                    !matches!(outside, Some(h) if h.aspect == aspect),
                    "{aspect:?} must not claim an angle one mdeg past its orb"
                );
            }
        }
    }

    #[test]
    fn the_orbs_never_overlap_so_a_separation_names_one_aspect() {
        for a in MAJOR_ASPECTS {
            for b in MAJOR_ASPECTS {
                if a == b {
                    continue;
                }
                let gap = (a.exact_mdeg() - b.exact_mdeg()).abs();
                assert!(
                    gap > a.orb_mdeg() + b.orb_mdeg(),
                    "{a:?} and {b:?} orbs overlap"
                );
            }
        }
    }

    #[test]
    fn separations_outside_every_orb_are_no_aspect_at_all() {
        // 30 (semi-sextile), 45 (semi-square), 150 (quincunx) are not in the
        // five-aspect set; CelestialAspect has no variant to hold them.
        assert!(aspect_between_mdeg(0, 30_000).is_none());
        assert!(aspect_between_mdeg(0, 45_000).is_none());
        assert!(aspect_between_mdeg(0, 150_000).is_none());
    }

    #[test]
    fn the_wheel_folds_so_the_aspect_is_symmetric_and_wrap_safe() {
        let direct = aspect_between_mdeg(10_000, 130_000).expect("trine");
        assert_eq!(direct.aspect, CelestialAspect::Trine);
        assert_eq!(aspect_between_mdeg(130_000, 10_000), Some(direct));
        // Across 0 Aries, and a full turn on top of it.
        assert_eq!(aspect_between_mdeg(350_000, 110_000), Some(direct));
        assert_eq!(aspect_between_mdeg(10_000 + 360_000, 130_000), Some(direct));
        assert_eq!(aspect_between_mdeg(10_000 - 360_000, 130_000), Some(direct));
    }

    #[test]
    fn chart_aspects_writes_every_pair_it_finds() {
        let bodies = [
            BodyLongitude { body: CelestialBody::Sun, lon_mdeg: 0 },
            BodyLongitude { body: CelestialBody::Moon, lon_mdeg: 120_000 },
            BodyLongitude { body: CelestialBody::Mars, lon_mdeg: 240_000 },
        ];
        let mut out = hit_buffer::<8>();
        let n = chart_aspects(&bodies, &mut out);
        assert_eq!(n, 3, "a grand trine is three pairs");
        assert!(out[..n].iter().all(|h| h.aspect == CelestialAspect::Trine));
    }

    #[test]
    fn chart_aspects_stops_at_the_buffer_and_never_overruns() {
        let bodies = [
            BodyLongitude { body: CelestialBody::Sun, lon_mdeg: 0 },
            BodyLongitude { body: CelestialBody::Moon, lon_mdeg: 120_000 },
            BodyLongitude { body: CelestialBody::Mars, lon_mdeg: 240_000 },
        ];
        let mut out = hit_buffer::<2>();
        assert_eq!(chart_aspects(&bodies, &mut out), 2);
        let mut none = hit_buffer::<0>();
        assert_eq!(chart_aspects(&bodies, &mut none), 0);
    }

    #[test]
    fn the_producer_feeds_the_resonance_consumer() {
        let bodies = [
            BodyLongitude { body: CelestialBody::Sun, lon_mdeg: 0 },
            BodyLongitude { body: CelestialBody::Mars, lon_mdeg: 3_000 },
        ];
        let mut out = hit_buffer::<4>();
        let n = chart_aspects(&bodies, &mut out);
        assert_eq!(n, 1);

        let aspects: Vec<CelestialAspect> = out[..n].iter().map(|h| h.aspect).collect();
        let influences = [
            PlanetaryInfluence { body: CelestialBody::Sun, weight_pmy: 10_000 },
            PlanetaryInfluence { body: CelestialBody::Mars, weight_pmy: 10_000 },
        ];
        assert_eq!(calculate_resonance_pmy(&aspects, &influences), 9_000);
    }

    #[test]
    fn test_empty_aspects_returns_zero() {
        assert_eq!(calculate_resonance_pmy(&[], &[]), 0);
    }

    #[test]
    fn test_conjunct_aspect_resonance() {
        let aspects = [CelestialAspect::Conjunct];
        let influences = [
            PlanetaryInfluence { body: CelestialBody::Sun, weight_pmy: 10_000 },
            PlanetaryInfluence { body: CelestialBody::Mars, weight_pmy: 10_000 },
        ];
        // 10,000 * 9,000 / 10,000 = 9,000 pmy
        assert_eq!(calculate_resonance_pmy(&aspects, &influences), 9_000);
    }

    #[test]
    fn test_dm_modifiers_under_full_moon() {
        let aspects = [CelestialAspect::Square];
        let influences = [PlanetaryInfluence {
            body: CelestialBody::Mars,
            weight_pmy: 8_000,
        }];
        let moon = MoonPhase::new(5, 15); // Full apex
        assert!(moon.is_full_apex());

        let mods = compute_dm_modifiers(&aspects, &influences, &moon);
        assert!(mods.aggression_scale_pmy >= 12_000);
        assert_eq!(mods.tension_tier, "Peak Volatility (Full Moon Hunt)");
    }
}
