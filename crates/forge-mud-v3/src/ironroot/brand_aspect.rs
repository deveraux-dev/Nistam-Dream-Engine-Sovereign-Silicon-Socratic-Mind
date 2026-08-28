//! Horary aspect geometry on the twelve-seat Brand wheel — which Brands can
//! see each other, and the yod's apex/base triad. Integer milli-degrees only;
//! the seats are exact 30° divisions, so no trig is needed to classify them.

use super::brand::BRAND_COUNT;

/// The full circle in milli-degrees.
pub const CIRCLE_MDEG: i32 = 360_000;

/// One Brand seat: the circle divided by the zodiac's twelve.
pub const BRAND_STEP_MDEG: i32 = CIRCLE_MDEG / BRAND_COUNT as i32;

/// Orb allowed either side of an exact aspect (1°, the tight horary orb).
pub const ASPECT_TOLERANCE_MDEG: i32 = 1_000;

/// A classical two-body aspect between Brand seats.
///
/// Deliberately NOT `combat_brain::dissonance::AspectGeometry`, whose seventh
/// variant is `Yod`. A yod is a THREE-body figure (an apex quincunx to two
/// bases sextile each other) and cannot be the verdict of a two-body angle
/// classification. The seventh case here is `Aversion` — the classical name
/// for two signs that simply cannot see one another — which is what a
/// fall-through actually means. Same seven-slot shape, different final term,
/// so the two types are not twins to be merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrandAspect {
    /// Same seat (0°).
    Conjunction,
    /// Two seats (60°).
    Sextile,
    /// Three seats (90°).
    Square,
    /// Four seats (120°).
    Trine,
    /// Five seats (150°) — the yod's leg.
    Quincunx,
    /// Six seats (180°).
    Opposition,
    /// No aspect at all: the seats cannot see each other.
    Aversion,
}

/// Where a Brand sits on the wheel, in milli-degrees from Aries.
pub fn brand_angle_mdeg(index: usize) -> i32 {
    (index % BRAND_COUNT) as i32 * BRAND_STEP_MDEG
}

/// Smallest absolute separation between two wheel angles.
pub fn angular_distance_mdeg(a_mdeg: i32, b_mdeg: i32) -> i32 {
    let d = (b_mdeg - a_mdeg).rem_euclid(CIRCLE_MDEG);
    d.min(CIRCLE_MDEG - d)
}

fn within(value: i32, target: i32) -> bool {
    (value - target).abs() <= ASPECT_TOLERANCE_MDEG
}

/// Classify a raw separation. Anything matching no classical angle is
/// `Aversion` — the absence of aspect, not a lesser one.
pub fn classify_aspect_mdeg(delta_mdeg: i32) -> BrandAspect {
    let d = delta_mdeg.rem_euclid(CIRCLE_MDEG);
    let a = d.min(CIRCLE_MDEG - d);

    if within(a, 0) {
        BrandAspect::Conjunction
    } else if within(a, 60_000) {
        BrandAspect::Sextile
    } else if within(a, 90_000) {
        BrandAspect::Square
    } else if within(a, 120_000) {
        BrandAspect::Trine
    } else if within(a, 150_000) {
        BrandAspect::Quincunx
    } else if within(a, 180_000) {
        BrandAspect::Opposition
    } else {
        BrandAspect::Aversion
    }
}

/// The aspect between two Brand seats.
pub fn aspect_between(a: usize, b: usize) -> BrandAspect {
    classify_aspect_mdeg(angular_distance_mdeg(brand_angle_mdeg(a), brand_angle_mdeg(b)))
}

/// True when two seats are five or seven apart — the quincunx, the leg a yod
/// is built from.
pub fn is_quincunx(a: usize, b: usize) -> bool {
    let d = (b as i16 - a as i16).rem_euclid(BRAND_COUNT as i16);
    d == 5 || d == 7
}

/// The two base seats of a yod whose apex is `apex`: five and seven seats on,
/// which are themselves sextile to each other. The classical finger of god.
pub fn yod_bases_for_apex(apex: usize) -> (usize, usize) {
    let i = apex % BRAND_COUNT;
    ((i + 5) % BRAND_COUNT, (i + 7) % BRAND_COUNT)
}

/// Of two seats, the one holding superior dexter — the earlier seat on the
/// wheel, which throws its ray with the circle's own direction.
pub fn superior_dexter(a: usize, b: usize) -> usize {
    let (a, b) = (a % BRAND_COUNT, b % BRAND_COUNT);
    if a <= b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_divides_into_twelve_exact_seats() {
        assert_eq!(BRAND_STEP_MDEG, 30_000);
        assert_eq!(BRAND_STEP_MDEG * BRAND_COUNT as i32, CIRCLE_MDEG);
        assert_eq!(brand_angle_mdeg(0), 0, "Aries opens the wheel");
        assert_eq!(brand_angle_mdeg(BRAND_COUNT), 0, "the wheel closes on itself");
    }

    #[test]
    fn seat_separation_reads_the_classical_aspects() {
        assert_eq!(aspect_between(0, 0), BrandAspect::Conjunction);
        assert_eq!(aspect_between(0, 2), BrandAspect::Sextile);
        assert_eq!(aspect_between(0, 3), BrandAspect::Square);
        assert_eq!(aspect_between(0, 4), BrandAspect::Trine);
        assert_eq!(aspect_between(0, 5), BrandAspect::Quincunx);
        assert_eq!(aspect_between(0, 6), BrandAspect::Opposition);
    }

    /// Aversion is REACHABLE from seats, and that is the whole point: signs
    /// one seat apart (30°, the semi-sextile) are not in aspect at all in the
    /// classical set — they cannot see each other. A seventh variant of `Yod`
    /// would have no verdict to give here, because a yod is three bodies.
    #[test]
    fn neighbouring_seats_are_in_aversion_not_in_aspect() {
        for a in 0..BRAND_COUNT {
            let next = (a + 1) % BRAND_COUNT;
            let prev = (a + BRAND_COUNT - 1) % BRAND_COUNT;
            assert_eq!(
                aspect_between(a, next),
                BrandAspect::Aversion,
                "seat {a} and its neighbour {next} are 30 degrees apart — no aspect"
            );
            assert_eq!(aspect_between(a, prev), BrandAspect::Aversion);
        }
        // And off-lattice angles fall through the same way.
        assert_eq!(classify_aspect_mdeg(45_000), BrandAspect::Aversion);
        assert_eq!(classify_aspect_mdeg(100_000), BrandAspect::Aversion);
    }

    /// Exactly which seat separations are averse: the semi-sextile only —
    /// one seat either way. Every other separation on a twelve-wheel lands on
    /// a classical angle (7 apart is 210°, which folds to the 150° quincunx).
    #[test]
    fn only_the_neighbouring_seats_are_averse() {
        let averse: Vec<usize> = (0..BRAND_COUNT)
            .filter(|&d| aspect_between(0, d) == BrandAspect::Aversion)
            .collect();
        assert_eq!(averse, vec![1, 11], "only the semi-sextile fails to see");
    }

    #[test]
    fn the_aspect_is_symmetric_across_the_wheel() {
        for a in 0..BRAND_COUNT {
            for b in 0..BRAND_COUNT {
                assert_eq!(aspect_between(a, b), aspect_between(b, a));
            }
        }
    }

    /// A yod is three bodies: the apex is quincunx to BOTH bases, and the
    /// bases are sextile to each other. That triad is the whole figure.
    #[test]
    fn the_yod_triad_holds_for_every_apex() {
        for apex in 0..BRAND_COUNT {
            let (base_a, base_b) = yod_bases_for_apex(apex);
            assert_eq!(aspect_between(apex, base_a), BrandAspect::Quincunx);
            assert_eq!(aspect_between(apex, base_b), BrandAspect::Quincunx);
            assert_eq!(
                aspect_between(base_a, base_b),
                BrandAspect::Sextile,
                "the two bases of a yod must see each other by sextile"
            );
            assert!(is_quincunx(apex, base_a) && is_quincunx(apex, base_b));
        }
    }

    #[test]
    fn superior_dexter_is_the_earlier_seat_either_way_round() {
        assert_eq!(superior_dexter(2, 9), 2);
        assert_eq!(superior_dexter(9, 2), 2);
        assert_eq!(superior_dexter(4, 4), 4);
    }

    /// The wheel is the Brand wheel, not a private copy of one.
    #[test]
    fn the_seats_are_the_landed_brands() {
        assert_eq!(BRAND_COUNT, 12);
        let (a, b) = yod_bases_for_apex(0);
        assert_eq!(
            (super::super::brand::BRANDS[a].name, super::super::brand::BRANDS[b].name),
            ("Virgo", "Scorpio"),
            "Aries apex takes its bases from Virgo and Scorpio"
        );
    }
}
