//! Wave pressure (chain H5): the bell rings, the arena answers.
//!
//! THE LOOP: the vgl `bell_pulse` EventHook fires on the audio side → its
//! `intensity_q` permyriad lands here as a bare integer (deliberately NO
//! forge-vgl type edge into game-systems — the caller passes the number, the
//! firewall stays clean) → pressure accumulates → the NEXT wave derives
//! through the astrakey sieve with a pressure-keyed context (different
//! context = different `DerivedSeed` = a genuinely different wave) plus a
//! pressure-scaled mob budget and a pressure-weighted rarity floor → casting
//! the wave VENTS pressure back toward neutral, so the loop breathes.
//!
//! Integer-permyriad throughout, deterministic (same prime + same wave index
//! + same bells = the same wave, DET gate). Allocation only at wave cast
//! (the context `String` — cold path, one per wave).

use super::astrakey_sieve::derivation::{derive_seed, weighted_rarity};
use super::astrakey_sieve::types::{RarityTier, SystemID};

/// Neutral pressure (1.0) — waves at neutral are the authored baseline.
pub const PRESSURE_NEUTRAL_Q: i32 = 10_000;
/// Ceiling (3.0): bell spam saturates here — no overflow, no runaway arena.
pub const PRESSURE_CEILING_Q: i32 = 30_000;
/// Each bell contributes a quarter of its intensity (a full 10_000 bell = +2_500).
const BELL_DAMP_DIV: i32 = 4;
/// Pressure buckets of 2_500 permyriad key the sieve context — coarse enough
/// that jitter never rerolls a wave, fine enough that real pressure does.
const PRESSURE_BUCKET_Q: i32 = 2_500;

/// The bell→wave pressure accumulator. One per arena run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavePressure {
    pressure_q: i32,
}

impl Default for WavePressure {
    fn default() -> Self {
        Self::new()
    }
}

/// One derived wave order — everything the spawner needs, sieve-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveOrder {
    /// Sieve-derived seed for this wave's composition RNG.
    pub seed_value: u64,
    /// Mob budget: `base_budget × pressure` (integer permyriad, floored at 1).
    pub mob_budget: i32,
    /// Rarity floor for elites, pressure-weighted through the sieve.
    pub rarity: RarityTier,
    /// Pressure at cast (permyriad) — a receipt for the debug inspectors.
    pub pressure_at_cast_q: i32,
}

impl WavePressure {
    pub fn new() -> Self {
        Self { pressure_q: PRESSURE_NEUTRAL_Q }
    }

    /// Current pressure, permyriad (10_000 = neutral).
    pub fn pressure_q(&self) -> i32 {
        self.pressure_q
    }

    /// A bell rang: feed its hook intensity (vgl `EventHook.intensity_q`,
    /// 0..=10_000, over-range clamped). Dampened ×¼, saturating at the
    /// ceiling. Zero or negative intensity is a no-op — a silent bell moves
    /// nothing.
    pub fn ring_bell(&mut self, intensity_q: i32) {
        if intensity_q <= 0 {
            return;
        }
        let add = intensity_q.min(10_000) / BELL_DAMP_DIV;
        self.pressure_q = (self.pressure_q + add).min(PRESSURE_CEILING_Q);
    }

    /// Derive the NEXT wave through the astrakey sieve, then VENT (half the
    /// head above neutral releases with the cast). Deterministic and total.
    pub fn next_wave(&mut self, master_prime: u64, wave_index: usize, base_budget: i32) -> WaveOrder {
        let bucket = (self.pressure_q - PRESSURE_NEUTRAL_Q) / PRESSURE_BUCKET_Q;
        let context = format!("arena.wave:{wave_index}:pressure:{bucket}");
        let seed = derive_seed(master_prime, wave_index, SystemID::Bosses, &context);
        // Pressure floors the rarity draw: neutral rolls floor 0 of 8, the
        // ceiling rolls floor 7 of 8 — elites thicken as the bell tolls.
        let floor = bucket.clamp(0, 7) as u32;
        let rarity = weighted_rarity(&seed, floor, 8);
        let mob_budget = ((base_budget as i64 * self.pressure_q as i64) / 10_000).max(1) as i32;
        let order = WaveOrder {
            seed_value: seed.seed_value,
            mob_budget,
            rarity,
            pressure_at_cast_q: self.pressure_q,
        };
        self.pressure_q = PRESSURE_NEUTRAL_Q + (self.pressure_q - PRESSURE_NEUTRAL_Q) / 2;
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_pressure_modifies_the_next_wave_deterministically() {
        let mut quiet = WavePressure::new();
        let mut loud = WavePressure::new();
        loud.ring_bell(10_000);
        loud.ring_bell(10_000);
        assert_eq!(loud.pressure_q(), 15_000, "two full bells = +5_000 head");

        let w_quiet = quiet.next_wave(104_729, 3, 100);
        let w_loud = loud.next_wave(104_729, 3, 100);
        assert_ne!(
            w_quiet.seed_value, w_loud.seed_value,
            "pressure re-keys the sieve derivation — the bell CHANGES the wave"
        );
        assert_eq!(w_quiet.mob_budget, 100, "neutral pressure = authored budget");
        assert_eq!(w_loud.mob_budget, 150, "1.5× pressure = 1.5× budget, integer-exact");

        // DET gate: replay the same bells against the same prime = same wave.
        let mut loud_replay = WavePressure::new();
        loud_replay.ring_bell(10_000);
        loud_replay.ring_bell(10_000);
        assert_eq!(loud_replay.next_wave(104_729, 3, 100), w_loud);
    }

    #[test]
    fn pressure_vents_at_cast_and_breathes_back_to_baseline() {
        let mut p = WavePressure::new();
        p.ring_bell(10_000);
        p.ring_bell(10_000);
        let _ = p.next_wave(7, 0, 10);
        assert_eq!(p.pressure_q(), 12_500, "cast vents half the head");
        let _ = p.next_wave(7, 1, 10);
        let _ = p.next_wave(7, 2, 10);
        let _ = p.next_wave(7, 3, 10);
        assert!(
            p.pressure_q() - PRESSURE_NEUTRAL_Q < 1_000,
            "pressure breathes back toward neutral without bells"
        );
        // Once the head decays under one bucket, waves match a fresh arena's.
        let mut fresh = WavePressure::new();
        assert_eq!(
            p.next_wave(7, 9, 10).seed_value,
            fresh.next_wave(7, 9, 10).seed_value,
            "vented arena rolls baseline waves again"
        );
    }

    #[test]
    fn bell_spam_saturates_at_the_ceiling_and_silent_bells_are_noops() {
        let mut p = WavePressure::new();
        for _ in 0..50 {
            p.ring_bell(10_000);
        }
        assert_eq!(p.pressure_q(), PRESSURE_CEILING_Q, "spam clamps at the ceiling");
        p.ring_bell(i32::MAX); // over-range intensity clamps, no overflow
        assert_eq!(p.pressure_q(), PRESSURE_CEILING_Q);
        p.ring_bell(0);
        p.ring_bell(-9_999);
        assert_eq!(p.pressure_q(), PRESSURE_CEILING_Q, "silent/negative bells move nothing");
        // Ceiling waves stay integer-exact: 3.0× budget.
        assert_eq!(p.next_wave(13, 0, 40).mob_budget, 120);
    }
}
