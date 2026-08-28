//! ShadowSieve Pattern Observation — fixed-array rival AI.
//!
//! Ported by translation from forge-cart-brain::sieve. Tracks enemy attack patterns
//! via direction and aspect frequency arrays. Provides prediction confidence as
//! Permyriad (0-10000). Degrades via bit-shift when total_observations reaches 60000.
//!
//! No floats. All arithmetic is integer-only. Zero heap allocations.

/// Fixed-size rival AI pattern observation map. Zero heap allocations.
/// Degrades via bit-shift at 60K observations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PatternMap {
    /// Attack direction frequencies (8 cardinal/ordinal directions).
    pub direction_freq: [u16; 8],
    /// Attack aspect frequencies (8 aspect categories).
    pub aspect_freq: [u16; 8],
    /// Sum of direction_freq entries. Triggers degradation at 60000.
    pub total_observations: u16,
}

impl PatternMap {
    /// Create a new empty pattern map.
    pub fn new() -> Self {
        Self::default()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_attack_increments_frequencies() {
        let mut map = PatternMap::default();
        map.observe_attack(2, 5);
        assert_eq!(map.direction_freq[2], 1);
        assert_eq!(map.aspect_freq[5], 1);
        assert_eq!(map.total_observations, 1);
    }

    #[test]
    fn prediction_confidence_zero_when_empty() {
        let map = PatternMap::default();
        assert_eq!(map.prediction_confidence(), 0);
    }

    #[test]
    fn prediction_confidence_formula() {
        let mut map = PatternMap::default();
        for _ in 0..5000 {
            map.observe_attack(1, 0);
        }
        for _ in 0..5000 {
            map.observe_attack(2, 0);
        }
        let conf = map.prediction_confidence();
        let expected = (5000 * 10000) / 10000;
        assert_eq!(conf, expected as i32);
    }

    #[test]
    fn observe_attack_masks_direction_and_aspect() {
        let mut map = PatternMap::default();
        map.observe_attack(0xFF, 0xFF);  // All bits set
        assert_eq!(map.direction_freq[7], 1, "direction masked to & 7");
        assert_eq!(map.aspect_freq[7], 1, "aspect masked to & 7");
    }

    #[test]
    fn degradation_triggers_at_60000() {
        let mut map = PatternMap::default();
        for i in 0..8 {
            map.direction_freq[i] = 7500;
        }
        map.total_observations = 59999;

        map.observe_attack(0, 0);
        assert!(
            map.total_observations <= 30000,
            "degradation should halve the total; got {}",
            map.total_observations
        );
    }

    #[test]
    fn inject_noise_reduces_confidence() {
        let mut map = PatternMap::default();
        // Set one dominant slot
        map.direction_freq[0] = 9000;
        for i in 1..8 {
            map.direction_freq[i] = 100;
        }
        map.total_observations = 9700;

        let conf_before = map.prediction_confidence();
        assert!(conf_before > 7000, "high confidence before noise");

        map.inject_noise(42, 1000);
        let conf_after = map.prediction_confidence();
        assert!(conf_after < 7000, "noise should reduce confidence below 7000; got {}", conf_after);
    }

    #[test]
    fn inject_noise_is_deterministic() {
        let mut map1 = PatternMap::default();
        let mut map2 = PatternMap::default();

        map1.inject_noise(42, 100);
        map2.inject_noise(42, 100);

        assert_eq!(map1.direction_freq, map2.direction_freq, "same seed produces same direction noise");
        assert_eq!(map1.aspect_freq, map2.aspect_freq, "same seed produces same aspect noise");
    }

    #[test]
    fn saturating_add_prevents_overflow() {
        let mut map = PatternMap::default();
        map.direction_freq[0] = u16::MAX;
        // Set total_observations just below degradation threshold
        map.total_observations = 59999;

        let freq_before = map.direction_freq[0];
        map.observe_attack(0, 0);
        // After observe_attack, total becomes 60000, triggering degradation (halving).
        // So freq[0] goes from u16::MAX to u16::MAX >> 1 = 32767
        assert_eq!(map.direction_freq[0], freq_before >> 1, "degradation halves freq at threshold");
        // total_observations is recomputed as sum of halved freqs = 32767
        assert_eq!(map.total_observations, 32767, "total recomputed after degradation");
    }
}
