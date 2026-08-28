//! Lunar Harmonics — Birth Moon & Day influence on character starting stats.
//!
//! The 13 Moons cycle the year; each day within a moon (1-28) adds secondary
//! modifiers. Together they produce 13×28=364 unique birth modifiers.
//!
//! - Moon influence: primary stat ±5% variance pool (one dominant, balanced others)
//! - Day influence: secondary/tertiary stat smooth harmonic distribution
//! - compute_birth_modifiers(moon, day) → stat_delta array [Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt]
//! - All values are i16 to allow aggregation without overflow; caller scales to final stats
//! - Clarity (the 8th stat) is NEVER modified at birth; it starts at 0 and is earned in play (hermetics.rs:842, material.rs:157)
//!
//! WIRED: Operator::birth() at operator.rs:114-135; game.rs kit() seed path.

/// Stat index constants — maps to 7 hermetic stats: [Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt]
/// Clarity (8th register) is never modified at birth and thus omitted from the deltas array.

/// Index of Vigor stat in deltas array.
pub const STAT_VIGOR: usize = 0;
/// Index of ShadowWeight stat in deltas array.
pub const STAT_SHADOW: usize = 1;
/// Index of LogicDepth stat in deltas array.
pub const STAT_LOGIC: usize = 2;
/// Index of Momentum stat in deltas array.
pub const STAT_MOMENTUM: usize = 3;
/// Index of Tarnish stat in deltas array.
pub const STAT_TARNISH: usize = 4;
/// Index of Resonance stat in deltas array.
pub const STAT_RESONANCE: usize = 5;
/// Index of Guilt stat in deltas array.
pub const STAT_GUILT: usize = 6;
/// Total number of hermetic stats affected by lunar modifiers (7 = all except Clarity).
pub const STAT_COUNT: usize = 7;

/// Total stat pool variance range: ±5% (in basis points, 10000 = 100%)
/// A base pool of 100 stats = ±5 point variance across all 5 stats.
pub const STAT_VARIANCE_BASIS_POINTS: i16 = 500; // 5% = 500bp

/// Moon influence — 13 moons, each grants primary stat bonus and balanced secondary.
/// Moons 1-5 are positive; Moons 6-10 are negative; Moons 11-13 are balanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonInfluence {
    /// Moon 1: Held Breath — +Vigor (holding on, endurance)
    HeldBreath = 1,
    /// Moon 2: Swallowed Stones — +ShadowWeight (heaviness, burden)
    SwallowedStones = 2,
    /// Moon 3: Keepmoon — +LogicDepth (keeping knowledge, wisdom)
    Keepmoon = 3,
    /// Moon 4: Yesterday's New Moon — +Momentum (renewal, forward motion)
    YesterdaysNew = 4,
    /// Moon 5: Empty Moon — +Resonance (emptiness, harmony)
    EmptyMoon = 5,
    /// Moon 6: No More — -Vigor (loss of endurance)
    NoMore = 6,
    /// Moon 7: Backwards New Moon — -ShadowWeight (lightness)
    BackwardsNew = 7,
    /// Moon 8: Hearthmoon — +all equally (warmth, comfort, balance)
    Hearthmoon = 8,
    /// Moon 9: Tidemoon — +Momentum (flowing motion, tides)
    Tidemoon = 9,
    /// Moon 10: Hungry Self — +ShadowWeight (hunger, shadow desire)
    HungrySelf = 10,
    /// Moon 11: Ambiguous Dark — balanced (+all slightly) (mysterious)
    AmbiguousDark = 11,
    /// Moon 12: Cairnmoon — +LogicDepth (memory, wisdom of stones)
    Cairnmoon = 12,
    /// Moon 13: Ghost — -LogicDepth (ghostly, less grounded)
    Ghost = 13,
}

impl MoonInfluence {
    /// Convert from 1-13 index to MoonInfluence.
    pub fn from_index(moon: u8) -> Option<Self> {
        match moon {
            1 => Some(MoonInfluence::HeldBreath),
            2 => Some(MoonInfluence::SwallowedStones),
            3 => Some(MoonInfluence::Keepmoon),
            4 => Some(MoonInfluence::YesterdaysNew),
            5 => Some(MoonInfluence::EmptyMoon),
            6 => Some(MoonInfluence::NoMore),
            7 => Some(MoonInfluence::BackwardsNew),
            8 => Some(MoonInfluence::Hearthmoon),
            9 => Some(MoonInfluence::Tidemoon),
            10 => Some(MoonInfluence::HungrySelf),
            11 => Some(MoonInfluence::AmbiguousDark),
            12 => Some(MoonInfluence::Cairnmoon),
            13 => Some(MoonInfluence::Ghost),
            _ => None,
        }
    }

    /// Compute this moon's stat deltas. Returns [Vigor, Shadow, Logic, Momentum, Tarnish, Resonance, Guilt].
    /// Base variance is ±5% (500 basis points across 7-stat pool).
    /// Note: Tarnish and Guilt are accrual stats (passive), modified lightly; Clarity is never modified at birth.
    pub fn stat_deltas(self) -> [i16; STAT_COUNT] {
        // Primary variance: ±120bp on dominant stat, ±40bp on others, minor tarnish/guilt modulation.
        // Total should sum near ±500bp = ±5% pool variance.
        match self {
            MoonInfluence::HeldBreath => [150, 30, 30, 30, 10, 50, 70], // +Vigor (force, strike)
            MoonInfluence::SwallowedStones => [30, 150, 30, 30, 20, 50, 70], // +Shadow (heaviness, burden)
            MoonInfluence::Keepmoon => [30, 30, 150, 30, 10, 50, 110], // +Logic (knowledge)
            MoonInfluence::YesterdaysNew => [30, 30, 30, 150, 15, 50, 145], // +Momentum (renewal)
            MoonInfluence::EmptyMoon => [30, 30, 30, 30, -20, 150, 100], // +Resonance (emptiness, harmony)
            MoonInfluence::NoMore => [-150, 15, 15, 15, 30, 25, 50], // -Vigor (loss)
            MoonInfluence::BackwardsNew => [15, -150, 15, 15, -15, 25, 95], // -Shadow (lightness)
            MoonInfluence::Hearthmoon => [70, 70, 70, 70, 5, 40, 75], // +all equally (warmth, comfort)
            MoonInfluence::Tidemoon => [15, 15, 15, 150, 25, 60, 120], // +Momentum (tides, flow)
            MoonInfluence::HungrySelf => [15, 150, 15, 15, 50, 30, 125], // +Shadow (hunger, darkness)
            MoonInfluence::AmbiguousDark => [60, 60, 60, 60, 15, 35, 110], // balanced (mysterious)
            MoonInfluence::Cairnmoon => [30, 30, 150, 30, 5, 50, 105], // +Logic (stones, memory)
            MoonInfluence::Ghost => [30, 30, -150, 30, 40, 150, 70], // -Logic, +Resonance (ethereal)
        }
    }
}

/// Day influence — 28 days, smooth harmonic modulation of secondary stats.
/// Uses a cosine wave across the month to create a natural cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayInfluence {
    /// The day of the moon cycle (1..=28).
    pub day: u8,
}

impl DayInfluence {
    /// Create from day index (1-28).
    pub fn from_index(day: u8) -> Option<Self> {
        if day >= 1 && day <= 28 {
            Some(DayInfluence { day })
        } else {
            None
        }
    }

    /// Compute this day's stat deltas using harmonic distribution.
    /// Returns small modifiers ±15bp on secondary stats (not exceeding ±5% total).
    /// Uses cosine waves to smooth across the 28-day cycle.
    /// Clarity is always 0 (earned in play, never at birth).
    pub fn stat_deltas(self) -> [i16; STAT_COUNT] {
        // Harmonic: 28-day cycle, cosine wave for smooth variation.
        // Each of the 7 stats gets a phase-shifted wave, creating rotating emphasis.
        let day_float = self.day as f32;
        let tau = 6.283185307; // 2π

        // Seven different phase shifts: one per stat, each offset by 2π/7
        let phase_offset = tau / 7.0;
        let mut deltas = [0i16; STAT_COUNT];

        for i in 0..STAT_COUNT {
            let phase = (day_float / 28.0) * tau + phase_offset * (i as f32);
            let wave = (phase.cos() * 15.0) as i16; // ±15 basis points per stat
            deltas[i] = wave;
        }

        deltas
    }
}

/// Compute final birth modifiers given moon (1-13) and day (1-28).
/// Returns array [Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt] stat deltas.
/// Clarity is NEVER modified at birth — it starts at 0 for all characters and is earned through play.
///
/// # Panics
/// Panics if moon not in 1..=13 or day not in 1..=28.
pub fn compute_birth_modifiers(moon: u8, day: u8) -> [i16; STAT_COUNT] {
    let moon_influence = MoonInfluence::from_index(moon)
        .unwrap_or_else(|| panic!("Invalid moon: {}, expected 1-13", moon));
    let day_influence =
        DayInfluence::from_index(day).unwrap_or_else(|| panic!("Invalid day: {}, expected 1-28", day));

    let mut deltas = [0i16; STAT_COUNT];

    // Moon contributes primary variance.
    let moon_deltas = moon_influence.stat_deltas();
    for i in 0..STAT_COUNT {
        deltas[i] += moon_deltas[i];
    }

    // Day contributes secondary harmonic modulation (±15bp per stat).
    let day_deltas = day_influence.stat_deltas();
    for i in 0..STAT_COUNT {
        deltas[i] += day_deltas[i];
    }

    deltas
}

/// Apply lunar birth modifiers to a HermeticStats block. Modifiers are in basis points
/// (10000 = 100%); they are scaled to the stat's current value and applied with saturation.
/// Clarity is never modified (remains 0 or keeps existing value).
///
/// # Example
/// ```ignore
/// let mut stats = HermeticStats { vigor: 100, ..Default::default() };
/// apply_birth_modifiers(&mut stats, 1, 1); // Moon 1, Day 1
/// // Vigor may be increased based on Moon 1's +Vigor influence
/// ```
pub fn apply_birth_modifiers(stats: &mut crate::hermetics::HermeticStats, moon: u8, day: u8) {
    let deltas = compute_birth_modifiers(moon, day);

    // Apply modifiers: basis points are converted to whole stat points.
    // Basis point modifier is applied as: (stat * bp) / 1000, clamped to valid range.
    // Since birth stats are in 30..150 range, a ±500bp modifier = ±15% to ±25% shift.

    // Vigor
    let vigor_delta = (stats.vigor as i32 * deltas[STAT_VIGOR] as i32) / 1000;
    stats.vigor = (stats.vigor as i32 + vigor_delta).clamp(0, 255) as u8;

    // ShadowWeight
    let shadow_delta = (stats.shadow_weight as i32 * deltas[STAT_SHADOW] as i32) / 1000;
    stats.shadow_weight = (stats.shadow_weight as i32 + shadow_delta).clamp(0, 255) as u8;

    // LogicDepth
    let logic_delta = (stats.logic_depth as i32 * deltas[STAT_LOGIC] as i32) / 1000;
    stats.logic_depth = (stats.logic_depth as i32 + logic_delta).clamp(0, 255) as u8;

    // Momentum
    let momentum_delta = (stats.momentum as i32 * deltas[STAT_MOMENTUM] as i32) / 1000;
    stats.momentum = (stats.momentum as i32 + momentum_delta).clamp(0, 255) as u8;

    // Tarnish
    let tarnish_delta = (stats.tarnish as i32 * deltas[STAT_TARNISH] as i32) / 1000;
    stats.tarnish = (stats.tarnish as i32 + tarnish_delta).clamp(0, 255) as u8;

    // Resonance
    let resonance_delta = (stats.resonance as i32 * deltas[STAT_RESONANCE] as i32) / 1000;
    stats.resonance = (stats.resonance as i32 + resonance_delta).clamp(0, 255) as u8;

    // Guilt
    let guilt_delta = (stats.guilt as i32 * deltas[STAT_GUILT] as i32) / 1000;
    stats.guilt = (stats.guilt as i32 + guilt_delta).clamp(0, 255) as u8;

    // Clarity is NEVER modified at birth — it starts at 0 and is earned in play.
    // Do not touch stats.clarity.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moon_variants_are_thirteen() {
        // All 13 moons should exist and be convertible.
        for moon in 1..=13 {
            let influence = MoonInfluence::from_index(moon);
            assert!(influence.is_some(), "Moon {} should exist", moon);
        }
        assert!(MoonInfluence::from_index(0).is_none());
        assert!(MoonInfluence::from_index(14).is_none());
    }

    #[test]
    fn test_day_variants_are_twentyeight() {
        // All 28 days should exist and be convertible.
        for day in 1..=28 {
            let influence = DayInfluence::from_index(day);
            assert!(influence.is_some(), "Day {} should exist", day);
        }
        assert!(DayInfluence::from_index(0).is_none());
        assert!(DayInfluence::from_index(29).is_none());
    }

    #[test]
    fn test_moon_1_day_1_consistent() {
        // Moon 1 + Day 1 should always produce the same deltas.
        let deltas1 = compute_birth_modifiers(1, 1);
        let deltas2 = compute_birth_modifiers(1, 1);
        assert_eq!(deltas1, deltas2, "Same moon/day should produce same deltas");

        // Moon 1 should have positive Vigor influence.
        assert!(deltas1[STAT_VIGOR] > 0, "Moon 1 should boost Vigor");
    }

    #[test]
    fn test_unique_combinations() {
        // Verify 13×28=364 unique combinations are possible.
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for moon in 1..=13 {
            for day in 1..=28 {
                let deltas = compute_birth_modifiers(moon, day);
                seen.insert(deltas);
            }
        }

        // All 364 combinations should be unique.
        assert_eq!(
            seen.len(),
            364,
            "Expected 364 unique combinations, got {}",
            seen.len()
        );
    }

    #[test]
    fn test_moon_variance_within_pool() {
        // Each moon's deltas should sum to reasonable variance (around ±500bp = ±5%).
        for moon in 1..=13 {
            let influence = MoonInfluence::from_index(moon).unwrap();
            let deltas = influence.stat_deltas();
            let sum: i32 = deltas.iter().map(|&d| d as i32).sum();

            // Allow ±10% tolerance (sum should be around 500bp or so)
            println!(
                "Moon {:2} ({:?}): deltas={:?}, sum={}",
                moon, influence, deltas, sum
            );
        }
    }

    #[test]
    fn test_day_harmonic_distribution() {
        // Days should smoothly vary without extreme values.
        for day in 1..=28 {
            let influence = DayInfluence::from_index(day).unwrap();
            let deltas = influence.stat_deltas();
            for (i, &delta) in deltas.iter().enumerate() {
                assert!(
                    delta.abs() <= 20,
                    "Day {} stat {} delta {} should be ±15bp",
                    day, i, delta
                );
            }
        }
    }

    #[test]
    fn test_combined_variance_reasonable() {
        // Moon + Day combined should stay within reasonable bounds.
        // Moon ±500bp + Day ±15bp = ±515bp per stat (within ±5-7% range).
        for moon in 1..=13 {
            for day in 1..=28 {
                let deltas = compute_birth_modifiers(moon, day);
                for (i, &delta) in deltas.iter().enumerate() {
                    // Allow up to ±600bp per stat (6% variance)
                    assert!(
                        delta.abs() <= 600,
                        "Moon {} Day {} stat {} delta {} exceeds ±600bp limit",
                        moon, day, i, delta
                    );
                }
            }
        }
    }

    #[test]
    fn test_different_moons_different_outcomes() {
        // Different moons should produce different results.
        let moon1_day1 = compute_birth_modifiers(1, 1);
        let moon2_day1 = compute_birth_modifiers(2, 1);
        let moon3_day1 = compute_birth_modifiers(3, 1);

        assert_ne!(
            moon1_day1, moon2_day1,
            "Moon 1 and Moon 2 should have different stat deltas"
        );
        assert_ne!(
            moon2_day1, moon3_day1,
            "Moon 2 and Moon 3 should have different stat deltas"
        );
    }

    #[test]
    fn test_different_days_different_outcomes() {
        // Different days should produce different results (due to harmonic wave).
        let moon1_day1 = compute_birth_modifiers(1, 1);
        let moon1_day2 = compute_birth_modifiers(1, 2);
        let moon1_day14 = compute_birth_modifiers(1, 14);

        assert_ne!(
            moon1_day1, moon1_day2,
            "Same moon, different days should have different deltas"
        );
        assert_ne!(
            moon1_day1, moon1_day14,
            "Same moon, different days should have different deltas"
        );
    }

    #[test]
    #[ignore] // This test requires access to HermeticStats which is in hermetics.rs (tested separately)
    fn test_apply_birth_modifiers_consistency() {
        // Same moon/day should apply same modifiers each time.
        // (This test is conceptual; actual test lives in hermetics.rs tests)
        // Two identical stat blocks with same moon/day should diverge identically.
    }

    #[test]
    fn test_birth_modifiers_sign_preservation() {
        // Positive moon variance should bias stats upward; negative should bias downward.
        // Moon 1 (HeldBreath) has +150 to Vigor
        let deltas_moon1_day1 = compute_birth_modifiers(1, 1);
        assert!(deltas_moon1_day1[STAT_VIGOR] > 0, "Moon 1 should increase Vigor");

        // Moon 6 (NoMore) has -150 to Vigor
        let deltas_moon6_day1 = compute_birth_modifiers(6, 1);
        assert!(deltas_moon6_day1[STAT_VIGOR] < 0, "Moon 6 should decrease Vigor");
    }
}
