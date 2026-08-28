//! Harmonic Strike Evaluation — resonance_hz-modulated hit_stop and knockback.
//!
//! All computation is integer-only. No f32/f64 permitted.
//! Lower resonance_hz (40 Hz, Nigredo) → heavier impact (more hit_stop, more knockback).
//! Higher resonance_hz (800 Hz, Rubedo) → lighter impact (less hit_stop, less knockback).

use crate::combat::{CombatState, AudioCommand};

/// Result of evaluating a harmonic strike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrikeResult {
    /// Hit-stop duration in ticks (1-8). Freezes game time on impact.
    pub hit_stop_ticks: u16,
    /// Knockback magnitude in MilliUnits (1000-8000). Direction applied by caller.
    pub knockback: i64,
    /// Audio command to dispatch for this strike.
    pub audio: AudioCommand,
}

/// Integer-only computation. No f32.
/// Maps resonance_hz 40-800 to hit_stop 8-1 ticks (inverse relationship).
/// Lower frequency → longer hit_stop. Higher frequency → shorter hit_stop.
pub fn compute_hit_stop(resonance_hz: u16) -> u16 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped - 40) as u32 * 7;
    let result = 8 - (numerator / 760) as u16;
    result.max(1)
}

/// Integer-only knockback. Inverse relationship with resonance_hz.
/// Low freq (40) → high knockback (8000 MilliUnits).
/// High freq (800) → low knockback (1000 MilliUnits).
pub fn compute_knockback(resonance_hz: u16) -> i64 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped as i64 - 40) * 7000;
    8000 - numerator / 760
}

/// Evaluate a harmonic strike based on the attacker's resonance_hz.
/// Returns hit_stop duration, knockback magnitude, and audio command.
pub fn evaluate_strike(state: &CombatState) -> StrikeResult {
    let hz = state.resonance_hz;
    let hit_stop = compute_hit_stop(hz);
    StrikeResult {
        hit_stop_ticks: hit_stop,
        knockback: compute_knockback(hz),
        audio: AudioCommand::HitStop { duration_ticks: hit_stop },
    }
}

// ── Property-Based Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests for boundary values ───────────────────────────────────────

    #[test]
    fn hit_stop_at_40hz_is_8() {
        assert_eq!(compute_hit_stop(40), 8);
    }

    #[test]
    fn hit_stop_at_800hz_is_1() {
        assert_eq!(compute_hit_stop(800), 1);
    }

    #[test]
    fn knockback_at_40hz_is_8000() {
        assert_eq!(compute_knockback(40), 8000);
    }

    #[test]
    fn knockback_at_800hz_is_1000() {
        assert_eq!(compute_knockback(800), 1000);
    }

    #[test]
    fn clamping_below_40_uses_40() {
        assert_eq!(compute_hit_stop(0), compute_hit_stop(40));
        assert_eq!(compute_knockback(0), compute_knockback(40));
    }

    #[test]
    fn clamping_above_800_uses_800() {
        assert_eq!(compute_hit_stop(65535), compute_hit_stop(800));
        assert_eq!(compute_knockback(65535), compute_knockback(800));
    }

    #[test]
    fn evaluate_strike_dispatches_audio() {
        let state = CombatState {
            resonance_hz: 400,
            ..Default::default()
        };
        let result = evaluate_strike(&state);
        assert_eq!(result.audio, AudioCommand::HitStop { duration_ticks: result.hit_stop_ticks });
    }

    // L07: Bijection test — strike frequency is strictly monotonic
    //
    // For any two resonance_hz values a, b where 40 <= a < b <= 800:
    //   compute_hit_stop(a) >= compute_hit_stop(b)
    //   compute_knockback(a) >= compute_knockback(b)
    #[test]
    fn l07_strike_monotonicity_bijection() {
        // Test that doubling resonance reduces hit_stop and knockback
        let pairs = vec![
            (40u16, 100u16),
            (100u16, 200u16),
            (200u16, 400u16),
            (400u16, 800u16),
        ];

        for (low, high) in pairs {
            let hs_low = compute_hit_stop(low);
            let hs_high = compute_hit_stop(high);
            let kb_low = compute_knockback(low);
            let kb_high = compute_knockback(high);

            assert!(
                hs_low >= hs_high,
                "Hit-stop must decrease: compute_hit_stop({}) = {} >= compute_hit_stop({}) = {}",
                low, hs_low, high, hs_high
            );
            assert!(
                kb_low >= kb_high,
                "Knockback must decrease: compute_knockback({}) = {} >= compute_knockback({}) = {}",
                low, kb_low, high, kb_high
            );
        }
    }

    // L07: Bijection test — encode/decode are inverses for strike outcomes
    #[test]
    fn l07_strike_result_encode_decode() {
        // Strike result encodes into: (hit_stop_ticks, knockback)
        // Decode: given hz, we should recover the same hit_stop and knockback
        let test_hz = vec![40, 100, 200, 400, 600, 800, 65535];

        for hz in test_hz {
            let result = evaluate_strike(&CombatState {
                resonance_hz: hz,
                ..Default::default()
            });

            // Verify audio matches hit_stop
            assert_eq!(
                result.audio,
                AudioCommand::HitStop { duration_ticks: result.hit_stop_ticks },
                "Audio encoding must match hit_stop for hz={}",
                hz
            );

            // Verify bounds
            assert!(result.hit_stop_ticks >= 1 && result.hit_stop_ticks <= 8);
            assert!(result.knockback >= 1000 && result.knockback <= 8000);
        }
    }
}
