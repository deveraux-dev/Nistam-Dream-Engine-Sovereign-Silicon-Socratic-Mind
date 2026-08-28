//! RDDA — Resonance-Driven Dynamic Asymmetry.
//!
//! Tracks inter-attack timing to detect rhythmic play. Consistent rhythm
//! amplifies damage windows; arrhythmic play narrows them.
//!
//! All integer. Zero-alloc (fixed-size ring buffer). No deps — WASM-clean.
//!
//! Ported from `forge-game-systems/src/arena_core/combat.rs`.

/// Number of intervals tracked for rhythm detection.
const RDDA_WINDOW: usize = 4;

/// Resonance state per player. Stored alongside [`CombatState`].
///
/// [`CombatState`]: super::CombatState
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResonanceState {
    /// Ring buffer of tick intervals between consecutive attacks.
    intervals: [u32; RDDA_WINDOW],
    /// Write index into the ring buffer.
    write_idx: u8,
    /// Number of valid entries (0..=RDDA_WINDOW).
    count: u8,
    /// Last attack tick (for computing interval).
    last_attack_tick: u32,
}

impl ResonanceState {
    /// Record an attack at the given tick. Call when an attack startup begins.
    pub fn record_attack(&mut self, current_tick: u32) {
        if self.last_attack_tick > 0 && current_tick > self.last_attack_tick {
            let interval = current_tick - self.last_attack_tick;
            self.intervals[self.write_idx as usize] = interval;
            self.write_idx = ((self.write_idx + 1) as usize % RDDA_WINDOW) as u8;
            if self.count < RDDA_WINDOW as u8 {
                self.count += 1;
            }
        }
        self.last_attack_tick = current_tick;
    }

    /// Resonance score in Permyriad (0–10000).
    ///
    /// 10000 = perfectly rhythmic, 0 = completely arrhythmic.
    /// Uses coefficient of variation: lower variance relative to mean = higher resonance.
    /// Returns 5000 (neutral) until two intervals are recorded.
    pub fn resonance_permyriad(&self) -> u16 {
        if self.count < 2 {
            return 5000; // neutral until enough data
        }

        let n = self.count as u32;
        let sum: u32 = self.intervals[..n as usize].iter().sum();
        let mean = sum / n;
        if mean == 0 {
            return 5000;
        }

        // Variance (integer): sum of |interval − mean|² / n
        let mut var_sum: u64 = 0;
        for i in 0..n as usize {
            let diff = self.intervals[i] as i64 - mean as i64;
            var_sum += (diff * diff) as u64;
        }
        let variance = var_sum / n as u64;

        // CV² = variance / mean². Map to Permyriad: low CV = high resonance.
        // CV² of 0 → 10000, CV² >= mean² → 0.
        let mean_sq = (mean as u64) * (mean as u64);
        if variance >= mean_sq {
            return 0;
        }

        ((mean_sq - variance) * 10_000 / mean_sq) as u16
    }

    /// Damage multiplier in Permyriad based on resonance.
    ///
    /// Resonance 10000 → 12500 (125%), Resonance 0 → 7500 (75%), Neutral (5000) → 10000 (100%).
    /// Linear map: `7500 + (resonance * 5000) / 10000`.
    pub fn damage_multiplier_permyriad(&self) -> u16 {
        let r = self.resonance_permyriad() as u32;
        (7500 + (r * 5000) / 10_000) as u16
    }
}

/// Apply RDDA damage scaling. `base_damage × multiplier / 10000`. Integer-only.
#[inline]
pub fn rdda_scale_damage(base_damage: u16, resonance: &ResonanceState) -> u16 {
    let mult = resonance.damage_multiplier_permyriad() as u32;
    ((base_damage as u32 * mult) / 10_000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_with_no_data() {
        let r = ResonanceState::default();
        assert_eq!(r.resonance_permyriad(), 5000);
        assert_eq!(r.damage_multiplier_permyriad(), 10_000);
    }

    #[test]
    fn perfect_rhythm_gives_max_resonance() {
        let mut r = ResonanceState::default();
        // Attacks every 20 ticks — perfectly rhythmic (zero variance).
        for i in 1..=5 {
            r.record_attack(i * 20);
        }
        assert_eq!(r.resonance_permyriad(), 10_000);
        assert_eq!(r.damage_multiplier_permyriad(), 12_500);
    }

    #[test]
    fn chaotic_rhythm_gives_low_resonance() {
        let mut r = ResonanceState::default();
        r.record_attack(10);
        r.record_attack(15);  // interval 5
        r.record_attack(100); // interval 85
        r.record_attack(105); // interval 5
        r.record_attack(200); // interval 95
        // Highly variable intervals → low resonance.
        assert!(r.resonance_permyriad() < 3000);
        assert!(r.damage_multiplier_permyriad() < 9000);
    }

    #[test]
    fn scale_damage_neutral_is_unchanged() {
        let r = ResonanceState::default();
        assert_eq!(rdda_scale_damage(100, &r), 100);
    }

    #[test]
    fn scale_damage_amplified_at_perfect_rhythm() {
        let mut r = ResonanceState::default();
        for i in 1..=5 { r.record_attack(i * 20); }
        assert_eq!(rdda_scale_damage(100, &r), 125);
    }

    #[test]
    fn ring_buffer_wraps_without_panic() {
        let mut r = ResonanceState::default();
        // Drive far more attacks than RDDA_WINDOW entries.
        for i in 1..=20u32 {
            r.record_attack(i * 15);
        }
        // Should not panic; count is capped at RDDA_WINDOW.
        assert!(r.count <= RDDA_WINDOW as u8);
        assert!(r.resonance_permyriad() <= 10_000);
    }

    #[test]
    fn single_attack_stays_neutral() {
        let mut r = ResonanceState::default();
        r.record_attack(50);
        assert_eq!(r.resonance_permyriad(), 5000, "need ≥2 intervals");
    }
}
