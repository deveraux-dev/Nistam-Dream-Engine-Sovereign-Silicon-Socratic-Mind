//! Gate for the APERTURE: ecliptic longitude -> orb -> named aspect, and the
//! wire from that producer into the resonance consumer it was missing.

use forge_book_v3::astrolabe_resonance::{
    aspect_between_mdeg, calculate_resonance_pmy, chart_aspects, compute_dm_modifiers,
    AspectHit, BodyLongitude, CelestialAspect, CelestialBody, MoonPhase, PlanetaryInfluence,
    MAJOR_ASPECTS,
};
use forge_mud_v3::ironroot::brand_aspect::angular_distance_mdeg;

fn buffer<const N: usize>() -> [AspectHit; N] {
    [AspectHit {
        aspect: CelestialAspect::Conjunct,
        separation_mdeg: 0,
        orb_deviation_mdeg: 0,
    }; N]
}

#[test]
fn every_named_aspect_is_reachable_from_a_longitude_pair() {
    for aspect in MAJOR_ASPECTS {
        let hit = aspect_between_mdeg(15_000, 15_000 + aspect.exact_mdeg())
            .unwrap_or_else(|| panic!("{aspect:?} unreachable"));
        assert_eq!(hit.aspect, aspect);
        assert_eq!(hit.orb_deviation_mdeg, 0);
    }
}

#[test]
fn the_fold_is_the_landed_one_not_a_private_copy() {
    for lon in (0..360_000).step_by(7_919) {
        let folded = angular_distance_mdeg(0, lon);
        if let Some(hit) = aspect_between_mdeg(0, lon) {
            assert_eq!(hit.separation_mdeg, folded);
            assert!(hit.separation_mdeg >= 0 && hit.separation_mdeg <= 180_000);
            assert!(hit.orb_deviation_mdeg <= hit.aspect.orb_mdeg());
        }
    }
}

#[test]
fn a_grand_trine_drives_the_dm_modifiers_end_to_end() {
    let bodies = [
        BodyLongitude { body: CelestialBody::Sun, lon_mdeg: 0 },
        BodyLongitude { body: CelestialBody::Moon, lon_mdeg: 121_000 },
        BodyLongitude { body: CelestialBody::Mars, lon_mdeg: 239_000 },
    ];
    let mut out = buffer::<8>();
    let n = chart_aspects(&bodies, &mut out);
    assert_eq!(n, 3);
    assert!(out[..n].iter().all(|h| h.aspect == CelestialAspect::Trine));

    let aspects: Vec<CelestialAspect> = out[..n].iter().map(|h| h.aspect).collect();
    let influences: Vec<PlanetaryInfluence> = bodies
        .iter()
        .map(|b| PlanetaryInfluence { body: b.body, weight_pmy: 10_000 })
        .collect();

    assert_eq!(calculate_resonance_pmy(&aspects, &influences), 8_500);

    let mods = compute_dm_modifiers(&aspects, &influences, &MoonPhase::new(5, 3));
    assert_eq!(mods.resonance_pmy, 8_500);
    assert_eq!(mods.tension_tier, "Harmonic Alignment");
}

#[test]
fn an_unaspected_chart_produces_nothing_and_resonance_stays_zero() {
    let bodies = [
        BodyLongitude { body: CelestialBody::Sun, lon_mdeg: 0 },
        BodyLongitude { body: CelestialBody::Saturn, lon_mdeg: 30_000 },
    ];
    let mut out = buffer::<4>();
    assert_eq!(chart_aspects(&bodies, &mut out), 0);
    assert_eq!(calculate_resonance_pmy(&[], &[]), 0);
}
