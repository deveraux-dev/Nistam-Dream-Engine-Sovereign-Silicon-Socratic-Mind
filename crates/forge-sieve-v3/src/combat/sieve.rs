//! Ported verbatim from E:\.airgap\2026-05-17-dsp-hrtf-p00-loop\ironroot-edict\game\src\combat\sieve.rs (2026-08-17 fake-enum-audit lineage port).
//!
//! ShadowSieve Pattern Observation — fixed-array nemesis AI.
//!
//! Tracks enemy attack patterns via direction and aspect frequency arrays.
//! Provides prediction confidence as Permyriad (0-10000).
//! Degrades via bit-shift when total_observations reaches 60000.
//!
//! No f32/f64 permitted. All arithmetic is integer-only.
//! Zero heap allocations. Fixed-size [u16; 8] arrays only.

use super::PatternMap;

impl PatternMap {
    /// Record an attack observation. Saturating increment of the corresponding
    /// direction and aspect frequency slots.
    ///
    /// Triggers degradation if total_observations reaches 60000.
    ///
    /// # Arguments
    /// - `direction`: Attack direction (masked to 0-7 via `& 7`)
    /// - `aspect`: Attack aspect (masked to 0-7 via `& 7`)
    pub fn observe_attack(&mut self, direction: u8, aspect: u8) {
        let dir_idx = (direction & 7) as usize;
        let asp_idx = (aspect & 7) as usize;

        self.direction_freq[dir_idx] = self.direction_freq[dir_idx].saturating_add(1);
        self.aspect_freq[asp_idx] = self.aspect_freq[asp_idx].saturating_add(1);
        self.total_observations = self.total_observations.saturating_add(1);

        if self.total_observations >= 60000 {
            self.degrade();
        }
    }

    /// Prediction confidence as Permyriad (0-10000).
    ///
    /// Formula: `(max(direction_freq) * 10000) / total_observations`
    /// Returns 0 if no observations recorded.
    pub fn prediction_confidence(&self) -> i32 {
        if self.total_observations == 0 {
            return 0;
        }
        let max_freq = self.direction_freq.iter().copied().max().unwrap_or(0);
        ((max_freq as u32 * 10000) / self.total_observations as u32) as i32
    }

    /// Flood arrays with pseudo-random values derived from seed.
    /// Destroys prediction confidence by distributing values roughly evenly.
    ///
    /// Uses a simple LCG (Linear Congruential Generator) for deterministic
    /// pseudo-random number generation. No external RNG dependency.
    ///
    /// # Arguments
    /// - `seed`: Initial RNG state
    /// - `intensity`: Maximum value for each frequency slot (modulo bound)
    pub fn inject_noise(&mut self, seed: u32, intensity: u16) {
        let mut rng = seed;
        for i in 0..8 {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            self.direction_freq[i] = (rng >> 16) as u16 % intensity;
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            self.aspect_freq[i] = (rng >> 16) as u16 % intensity;
        }
        // Recompute total from direction_freq
        self.total_observations = self.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));
    }

    /// Right-shift all frequencies by 1, halving values.
    /// Preserves relative ratios while reclaiming headroom.
    /// Recomputes total_observations as sum of halved direction_freq.
    fn degrade(&mut self) {
        for freq in self.direction_freq.iter_mut() {
            *freq >>= 1;
        }
        for freq in self.aspect_freq.iter_mut() {
            *freq >>= 1;
        }
        self.total_observations = self.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));
    }
}

// ── Property-Based Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Property 13: Noise Injection Reduces Confidence ──────────────────

    // Feature: combat-system, Property 13: Noise Injection Reduces Confidence
    // For any PatternMap with prediction_confidence > 7000, after
    // inject_noise(seed, 10000), prediction_confidence SHALL be < 7000.
    //
    // Rationale: noise distributes values roughly evenly across 8 slots,
    // so max/total ≈ 1/8 = 1250 Permyriad, well below 7000.
    //
    // **Validates: Requirements 5.6**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn noise_injection_reduces_confidence(
            seed in any::<u32>(),
            dominant_slot in 0usize..8,
            dominant_value in 8000u16..=60000,
            other_value in 0u16..=500,
        ) {
            // Build a PatternMap with high confidence (one dominant slot)
            let mut map = PatternMap::default();
            for i in 0..8 {
                if i == dominant_slot {
                    map.direction_freq[i] = dominant_value;
                } else {
                    map.direction_freq[i] = other_value;
                }
                map.aspect_freq[i] = other_value;
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            // Verify precondition: confidence > 7000
            let confidence_before = map.prediction_confidence();
            prop_assume!(confidence_before > 7000,
                "Precondition: confidence must be > 7000, got {}", confidence_before);

            // Inject noise
            map.inject_noise(seed, 10000);

            // Postcondition: confidence < 7000
            let confidence_after = map.prediction_confidence();
            prop_assert!(confidence_after < 7000,
                "After inject_noise, confidence should be < 7000, got {} (was {})",
                confidence_after, confidence_before);
        }
    }

    // ── Property 15: PatternMap Observation Recording ─────────────────────

    // Feature: combat-system, Property 15: PatternMap Observation Recording
    // For any call to observe_attack(direction, aspect) where direction and
    // aspect are in [0, 7], the corresponding direction_freq[direction] SHALL
    // increase by 1 and aspect_freq[aspect] SHALL increase by 1
    // (unless degradation triggers).
    //
    // **Validates: Requirements 7.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn pattern_map_observation_recording(
            direction in 0u8..8,
            aspect in 0u8..8,
            // Use low initial values to avoid triggering degradation
            initial_freqs in proptest::array::uniform8(0u16..1000),
            initial_aspects in proptest::array::uniform8(0u16..1000),
        ) {
            let mut map = PatternMap {
                direction_freq: initial_freqs,
                aspect_freq: initial_aspects,
                total_observations: initial_freqs.iter().copied().fold(0u16, |a, x| a.saturating_add(x)),
            };

            // Ensure we won't trigger degradation (total < 60000)
            prop_assume!(map.total_observations < 59999);

            let dir_before = map.direction_freq[direction as usize];
            let asp_before = map.aspect_freq[aspect as usize];
            let total_before = map.total_observations;

            map.observe_attack(direction, aspect);

            // direction_freq[direction] incremented by 1
            prop_assert_eq!(
                map.direction_freq[direction as usize],
                dir_before.saturating_add(1),
                "direction_freq[{}] should increment: {} -> {}",
                direction, dir_before, map.direction_freq[direction as usize]
            );

            // aspect_freq[aspect] incremented by 1
            prop_assert_eq!(
                map.aspect_freq[aspect as usize],
                asp_before.saturating_add(1),
                "aspect_freq[{}] should increment: {} -> {}",
                aspect, asp_before, map.aspect_freq[aspect as usize]
            );

            // total_observations incremented by 1
            prop_assert_eq!(
                map.total_observations,
                total_before.saturating_add(1),
                "total_observations should increment: {} -> {}",
                total_before, map.total_observations
            );
        }
    }

    // ── Property 16: PatternMap Confidence Formula ────────────────────────

    // Feature: combat-system, Property 16: PatternMap Confidence Formula
    // For any PatternMap state where total_observations > 0,
    // prediction_confidence() SHALL equal
    // (max(direction_freq) * 10000) / total_observations.
    //
    // **Validates: Requirements 7.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn pattern_map_confidence_formula(
            freqs in proptest::array::uniform8(0u16..10000),
            aspects in proptest::array::uniform8(0u16..10000),
        ) {
            let total: u16 = freqs.iter().copied().fold(0u16, |a, x| a.saturating_add(x));
            prop_assume!(total > 0);

            let map = PatternMap {
                direction_freq: freqs,
                aspect_freq: aspects,
                total_observations: total,
            };

            let max_freq = freqs.iter().copied().max().unwrap();
            let expected = ((max_freq as u32 * 10000) / total as u32) as i32;
            let actual = map.prediction_confidence();

            prop_assert_eq!(actual, expected,
                "Confidence formula mismatch: expected {}, got {} (max_freq={}, total={})",
                expected, actual, max_freq, total);
        }
    }

    // ── Property 17: PatternMap Degradation Invariant ─────────────────────

    // Feature: combat-system, Property 17: PatternMap Degradation Invariant
    // For any sequence of observe_attack() calls with direction in [0, 7] and
    // aspect in [0, 7], total_observations SHALL never exceed 60000.
    //
    // **Validates: Requirements 7.6, 7.7, 7.8**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn pattern_map_degradation_invariant(
            // Generate a sequence of observations
            observations in proptest::collection::vec((0u8..8, 0u8..8), 1..500),
            // Start near the degradation threshold to exercise the boundary
            initial_total in 59000u16..=59999,
        ) {
            // Build initial state near threshold
            let per_slot = initial_total / 8;
            let remainder = initial_total % 8;
            let mut map = PatternMap::default();
            for i in 0..8 {
                map.direction_freq[i] = per_slot + if (i as u16) < remainder { 1 } else { 0 };
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            // Apply all observations
            for (dir, asp) in observations {
                map.observe_attack(dir, asp);

                // Invariant: total_observations never exceeds 60000
                prop_assert!(map.total_observations <= 60000,
                    "total_observations exceeded 60000: got {}",
                    map.total_observations);
            }
        }
    }
}
