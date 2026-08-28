//! Day-quality scoring — `ORACLE-C-DREAM-DIAMONDS-EUX.md:231-232`:
//! `day-quality = balance*0.3 + energy*0.4 + beat*0.3`, plus the rough-patch
//! watch (`§13`: quality <0.3 sustained 5s=600 ticks @120Hz is remembered).
//!
//! Integer Permyriad throughout (0..=10000 = 0.0..=1.0), the house convention
//! (`cdk.rs`, `entropy.rs`) — no floats in the scored path.

/// Fixed-point weights, out of 10_000 — 0.3 / 0.4 / 0.3.
const WEIGHT_BALANCE_PMY: u32 = 3_000;
const WEIGHT_ENERGY_PMY: u32 = 4_000;
const WEIGHT_BEAT_PMY: u32 = 3_000;

/// Ticks a sustained rough patch must run before the world remembers it
/// (`ORACLE-C-DREAM-DIAMONDS-EUX.md:233`, `§13`: 5s @ 120Hz).
pub const ROUGH_PATCH_TICKS: u64 = 600;

/// The safe rest window in ticks: `wake` inside this window seals `Attested`;
/// past it, the `EphemeralEnvelope` falls through to `Expired` — the
/// Sleeping-Beauty boundary (sleep past the deadline, the rest turns risky).
pub const SLEEP_TTL_TICKS: u64 = 600;

/// Quality floor below which a moment counts toward a rough patch (`§8`: <0.3).
pub const ROUGH_PATCH_FLOOR_PMY: u32 = 3_000;

/// `day_quality = balance*0.3 + energy*0.4 + beat*0.3`, each input and the
/// result in Permyriad (0..=10000). Inputs are clamped, not asserted — a
/// caller feeding an out-of-range signal gets a clamped answer, not a panic.
pub fn day_quality_pmy(balance_pmy: u32, energy_pmy: u32, beat_pmy: u32) -> u32 {
    let b = balance_pmy.min(10_000) as u64;
    let e = energy_pmy.min(10_000) as u64;
    let t = beat_pmy.min(10_000) as u64;
    let weighted = b * WEIGHT_BALANCE_PMY as u64
        + e * WEIGHT_ENERGY_PMY as u64
        + t * WEIGHT_BEAT_PMY as u64;
    (weighted / 10_000) as u32
}

/// Tracks a running below-floor streak in ticks; reports once the streak
/// crosses [`ROUGH_PATCH_TICKS`] (fires once per streak, not once per tick
/// past the threshold).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoughPatchWatch {
    streak_ticks: u64,
    reported: bool,
}

impl RoughPatchWatch {
    /// A fresh watch, no streak yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold in one tick's quality reading. Returns `true` exactly once, on
    /// the tick the streak first crosses [`ROUGH_PATCH_TICKS`].
    pub fn observe(&mut self, quality_pmy: u32) -> bool {
        if quality_pmy < ROUGH_PATCH_FLOOR_PMY {
            self.streak_ticks += 1;
        } else {
            self.streak_ticks = 0;
            self.reported = false;
        }
        if !self.reported && self.streak_ticks >= ROUGH_PATCH_TICKS {
            self.reported = true;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_quality_weights_match_the_spec_exactly() {
        // balance=1.0, energy=0.0, beat=0.0 -> 0.3
        assert_eq!(day_quality_pmy(10_000, 0, 0), 3_000);
        // balance=0.0, energy=1.0, beat=0.0 -> 0.4
        assert_eq!(day_quality_pmy(0, 10_000, 0), 4_000);
        // balance=0.0, energy=0.0, beat=1.0 -> 0.3
        assert_eq!(day_quality_pmy(0, 0, 10_000), 3_000);
        // all full scale -> 1.0
        assert_eq!(day_quality_pmy(10_000, 10_000, 10_000), 10_000);
        // all zero -> 0.0
        assert_eq!(day_quality_pmy(0, 0, 0), 0);
    }

    #[test]
    fn day_quality_clamps_out_of_range_inputs() {
        assert_eq!(day_quality_pmy(20_000, 20_000, 20_000), 10_000);
    }

    #[test]
    fn rough_patch_fires_once_at_exactly_600_ticks() {
        let mut watch = RoughPatchWatch::new();
        for _ in 0..(ROUGH_PATCH_TICKS - 1) {
            assert!(!watch.observe(1_000), "must not fire before the threshold");
        }
        assert!(watch.observe(1_000), "must fire on the tick that crosses the threshold");
        assert!(!watch.observe(1_000), "must not fire twice for the same streak");
    }

    #[test]
    fn rough_patch_resets_on_recovery() {
        let mut watch = RoughPatchWatch::new();
        for _ in 0..300 {
            watch.observe(1_000);
        }
        watch.observe(9_000); // recovers above the floor
        for _ in 0..(ROUGH_PATCH_TICKS - 1) {
            assert!(!watch.observe(1_000), "a recovered streak must restart from zero");
        }
        assert!(watch.observe(1_000));
    }

    #[test]
    fn quality_at_or_above_floor_never_counts() {
        let mut watch = RoughPatchWatch::new();
        for _ in 0..(ROUGH_PATCH_TICKS * 2) {
            assert!(!watch.observe(ROUGH_PATCH_FLOOR_PMY), "exactly-at-floor is not below it");
        }
    }
}
