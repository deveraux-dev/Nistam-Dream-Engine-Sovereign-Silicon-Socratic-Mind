//! Two Drums: Drum-1 (integer tick/watchdog, deterministic) & Drum-2 (wall-clock speculative, real-time).
//! Doctrine instantiated over CollisionBridge.

use forge_hal_clockspine::{CollisionBridge, Permyriad, ResonanceImpulse, SimTick};

/// Two-drum orchestration wrapping CollisionBridge.
///
/// Drum-1 (Alpha) side: Publishes tick-stamped watchdog N×IPR metrics.
/// Drum-2 (Beta) side: Subscribes to cross-domain N×IPR scaling for speculative acceptance.
/// Never blocks; a miss on either side reuses the last published impulse.
pub struct TwoDrums {
    bridge: CollisionBridge,
    /// Last Alpha gen seen by this instance (used to track freshness on Beta side).
    last_alpha_gen: u64,
    /// Last Beta gen seen by this instance (used to track freshness on Alpha side).
    last_beta_gen: u64,
}

impl TwoDrums {
    /// Create a new two-drum orchestration.
    pub fn new() -> Self {
        Self {
            bridge: CollisionBridge::new(),
            last_alpha_gen: 0,
            last_beta_gen: 0,
        }
    }

    /// Drum-1 (Alpha) publishes a tick and watchdog N×IPR metric.
    /// Returns the old impulse recycled to the caller (for reuse).
    pub fn drum1_publish(&mut self, tick: SimTick, n_ipr: f32) -> ResonanceImpulse {
        let mag_pmy = scale_n_ipr_to_permyriad(n_ipr);
        let impulse = ResonanceImpulse { idx: tick.0, mag_pmy, lane: 0 };
        self.bridge.alpha_publish(impulse)
    }

    /// Drum-2 (Beta) tries to take the latest Drum-1 impulse.
    /// Returns `Some((impulse, scale))` with fresh scaling, or `None` to reuse last scale.
    /// The caller should cache `last_scale` and use it on `None`.
    pub fn drum2_take(&mut self) -> Option<(ResonanceImpulse, f32)> {
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        if let Some(gen) = self.bridge.beta_take(self.last_beta_gen, &mut impulse) {
            self.last_beta_gen = gen;
            let scale = permyriad_to_scale(impulse.mag_pmy);
            Some((impulse, scale))
        } else {
            None
        }
    }

    /// Drum-1 tries to take any liveness pulse from Drum-2. Used for heartbeat detection.
    /// Returns `Some(impulse)` if fresh, `None` to reuse last pulse.
    pub fn drum1_take_liveness(&mut self) -> Option<ResonanceImpulse> {
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        if let Some(gen) = self.bridge.alpha_take(self.last_alpha_gen, &mut impulse) {
            self.last_alpha_gen = gen;
            Some(impulse)
        } else {
            None
        }
    }

    /// Drum-2 publishes a liveness pulse (heartbeat) to Drum-1.
    pub fn drum2_publish_heartbeat(&self, wall_clock_index: u64) -> ResonanceImpulse {
        let impulse = ResonanceImpulse { idx: wall_clock_index, mag_pmy: Permyriad::MAX, lane: 1 };
        self.bridge.beta_publish(impulse)
    }
}

impl Default for TwoDrums {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert N×IPR (float) to Permyriad range [0, 10_000].
/// Clamps to valid range; scales are normalized per ARCH-009 doctrine.
#[inline]
fn scale_n_ipr_to_permyriad(n_ipr: f32) -> Permyriad {
    let clamped = n_ipr.max(1.0).min(200.0);
    let scale = ((clamped - 1.0) / 199.0) * 10_000.0;
    Permyriad::clamp(scale as i32)
}

/// Convert Permyriad to a float scaling factor for acceptance probability.
/// Result is in [0.0, 1.0] for direct multiplication against acceptance rates.
#[inline]
fn permyriad_to_scale(pmy: Permyriad) -> f32 {
    (pmy.0 as f32) / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drum1_publish_and_drum2_take() {
        let mut two_drums = TwoDrums::new();
        let tick = SimTick(42);
        let n_ipr = 50.0;

        two_drums.drum1_publish(tick, n_ipr);

        let (impulse, scale) = two_drums.drum2_take().expect("drum2 should see fresh impulse");
        assert_eq!(impulse.idx, 42);
        assert!(scale > 0.0 && scale <= 1.0, "scale should be in valid range");
    }

    #[test]
    fn drum2_take_miss_returns_none() {
        let mut two_drums = TwoDrums::new();
        let tick = SimTick(10);
        let n_ipr = 100.0;

        two_drums.drum1_publish(tick, n_ipr);
        let (_, _scale1) = two_drums.drum2_take().expect("first take fresh");

        assert!(two_drums.drum2_take().is_none(), "second take should miss (no update)");
    }

    #[test]
    fn drum2_heartbeat_to_drum1() {
        let mut two_drums = TwoDrums::new();
        let wall_clock = 123u64;

        two_drums.drum2_publish_heartbeat(wall_clock);

        let impulse = two_drums.drum1_take_liveness().expect("drum1 should see heartbeat");
        assert_eq!(impulse.idx, wall_clock);
        assert_eq!(impulse.mag_pmy, Permyriad::MAX);
        assert_eq!(impulse.lane, 1);
    }

    #[test]
    fn scale_n_ipr_extremes() {
        assert_eq!(permyriad_to_scale(scale_n_ipr_to_permyriad(1.0)), 0.0, "n_ipr=1.0 → 0");
        let max_scale = permyriad_to_scale(scale_n_ipr_to_permyriad(200.0));
        assert!(
            (max_scale - 1.0).abs() < 0.01,
            "n_ipr=200.0 should scale close to 1.0"
        );
    }

    #[test]
    fn scale_n_ipr_midpoint() {
        let mid_scale = permyriad_to_scale(scale_n_ipr_to_permyriad(100.5));
        assert!(mid_scale > 0.4 && mid_scale < 0.6, "n_ipr=100.5 should be near 0.5");
    }
}
