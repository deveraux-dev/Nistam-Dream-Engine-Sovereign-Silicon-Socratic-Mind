//! Alchemical resurrection — co-op revive system.

use serde::{Deserialize, Serialize};
use super::config::ticks_from_secs;

pub const CORPSE_DRAG_PROXIMITY_SQ_MM: i64 = 3_600_000_000;
pub const DRAG_SPEED_MULTIPLIER_PERMYRIAD: u32 = 4000;
pub const UROSCOPY_CHANNEL_TICKS: u16 = ticks_from_secs(3) as u16;
pub const TRANSMUTATION_CHANNEL_TICKS: u16 = ticks_from_secs(3) as u16;
pub const TRANSMUTATION_HP_COST_PERMYRIAD: u32 = 5000;
pub const SCAR_WEIGHT_GRAMS: u32 = 5000;
pub const SCAR_HP_PENALTY_PERMYRIAD: u32 = 500;
pub const NO_DRAG_TARGET: u8 = 255;
/// Max tether length squared before corpse gets pulled (100px -> 100_000mm, mm² scale).
pub const MAX_TETHER_LENGTH_SQ_MM: i64 = 10_000_000_000;
/// Lerp factor for tether pull, permyriad (1500 = 0.15, deterministic, no sqrt).
pub const TETHER_PULL_FACTOR_PERMYRIAD: u32 = 1500;
/// Vicious Tongue ping cooldown (distinct from half_hanged's AoE-attack cooldown).
pub const VICIOUS_TONGUE_PING_COOLDOWN: u8 = super::config::ticks_from_ms(500) as u8;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResurrectionPhase { Uroscopy, Transmutation }

pub fn is_near_corpse_mm(px: i64, py: i64, cx: i64, cy: i64) -> bool {
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy < CORPSE_DRAG_PROXIMITY_SQ_MM
}

pub fn is_safe_zone(_x: i64, _y: i64) -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_near_corpse_within_range() {
        // dist_mm = (40_000, 30_000) -> dist_sq = 1.6e9 + 0.9e9 = 2.5e9 < 3.6e9
        assert!(is_near_corpse_mm(100_000, 200_000, 140_000, 230_000));
    }

    #[test]
    fn is_near_corpse_out_of_range() {
        // dist_sq = 1e10 + 1e10 = 2e10 > 3.6e9
        assert!(!is_near_corpse_mm(0, 0, 100_000, 100_000));
    }

    #[test]
    fn is_safe_zone_stub_always_true() {
        assert!(is_safe_zone(0, 0));
        assert!(is_safe_zone(999_000, -999_000));
    }
}
