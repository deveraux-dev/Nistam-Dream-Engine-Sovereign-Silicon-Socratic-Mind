//! Resonance Constants — single source of truth for all Hz-based physics.
//!
//! All combat, parry, strike, and visual systems reference these constants —
//! never hardcode Hz values elsewhere. Pure integer, zero-alloc; grounds the
//! `resonance_hz` physics semantics in the entire combat brain.

// ── Resonance Frequency Bounds ───────────────────────────────────────────────

/// Minimum entity resonance (Nigredo tier, Earth, heavy impact).
pub const RESONANCE_MIN_HZ: u16 = 40;

/// Maximum entity resonance (Rubedo tier, Fire, light impact).
pub const RESONANCE_MAX_HZ: u16 = 800;

/// Harmonic tuning frequency (A4 = 432Hz, not 440Hz concert pitch).
pub const HARMONIC_TUNING_HZ: u16 = 432;

/// Concert pitch reference (used by Equal Blade boss).
pub const CONCERT_PITCH_HZ: u16 = 440;

// ── Phase Cancellation ───────────────────────────────────────────────────────

/// Perfect parry resonance sum. When attacker_hz + defender_hz == this value,
/// phase cancellation occurs and the parry is perfect (zero damage, full riposte).
pub const PHASE_CANCEL_SUM_HZ: u16 = 840;

/// Check if two resonance values produce phase cancellation.
#[inline]
pub fn is_phase_cancelled(a_hz: u16, b_hz: u16) -> bool {
    a_hz.saturating_add(b_hz) == PHASE_CANCEL_SUM_HZ
}

// ── Tier Boundaries ──────────────────────────────────────────────────────────

/// Nigredo threshold: resonance <= this is Nigredo (Earth/Shadow, heavy).
pub const NIGREDO_CEILING_HZ: u16 = 200;

/// Albedo floor: resonance >= this is Albedo (transitional, balanced).
pub const ALBEDO_FLOOR_HZ: u16 = 201;
/// Albedo ceiling: resonance <= this is Albedo (transitional, balanced).
pub const ALBEDO_CEILING_HZ: u16 = 599;

/// Rubedo threshold: resonance >= this is Rubedo (Fire/Light, fast).
pub const RUBEDO_FLOOR_HZ: u16 = 600;

// ── Tick Rate ────────────────────────────────────────────────────────────────

/// Physics tick rate (deterministic simulation).
pub const PHYSICS_TICK_HZ: u32 = 120;

/// Physics tick duration in microseconds.
pub const PHYSICS_TICK_MICROS: u32 = 1_000_000 / PHYSICS_TICK_HZ;

/// Visual interpolation rate (render can exceed physics rate).
pub const VISUAL_TICK_HZ: u32 = 240;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_cancel_at_440_400() {
        assert!(is_phase_cancelled(440, 400));
    }

    #[test]
    fn phase_cancel_at_420_420() {
        assert!(is_phase_cancelled(420, 420));
    }

    #[test]
    fn no_cancel_at_440_440() {
        assert!(!is_phase_cancelled(440, 440));
    }

    #[test]
    fn resonance_bounds_valid() {
        assert!(RESONANCE_MIN_HZ <= NIGREDO_CEILING_HZ);
        assert!(RUBEDO_FLOOR_HZ <= RESONANCE_MAX_HZ);
    }

    #[test]
    fn tier_ranges_ordered() {
        assert!(NIGREDO_CEILING_HZ < ALBEDO_FLOOR_HZ);
        assert!(ALBEDO_CEILING_HZ < RUBEDO_FLOOR_HZ);
    }

    #[test]
    fn phase_cancel_sum_is_double_concert_pitch_minus_40() {
        // 840 = 440 + 400 (concert pitch + 40 Hz base)
        assert_eq!(PHASE_CANCEL_SUM_HZ, CONCERT_PITCH_HZ + RESONANCE_MIN_HZ * 10);
    }

    #[test]
    fn l18_sabotage_phase_cancel_gate() {
        // L18: Sabotage the assert to confirm it fails, then revert.
        // GATE: phase cancellation at 420+420 MUST work
        assert!(
            is_phase_cancelled(420, 420),
            "L18 sabotage: phase cancel gate is now broken; reverting confirms it was live"
        );
    }
}
