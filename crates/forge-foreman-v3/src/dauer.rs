//! Allostatic OODA hypervisor survival mode (dauer).
//! Ported from forge-daemon/harness.rs (lines 10-31).
//!
//! DAUER = the build has been FAIL/BLIND for DAUER_THRESHOLD+ beats:
//! hold mutations, durable state only.

/// Consecutive non-PASS beats before the observer declares survival mode
/// (mirrors the gate's [STRIKE 1/3] ladder — sustained load, never one spike).
pub const DAUER_THRESHOLD: u32 = 3;

/// dauer_state — the Allostatic OODA Hypervisor survival mode.
/// `Dauer` = the build has been FAIL/BLIND for [`DAUER_THRESHOLD`]+ beats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DauerState {
    /// The build is currently passing (streak < DAUER_THRESHOLD).
    Active,
    /// The build has failed DAUER_THRESHOLD+ consecutive times; survival mode active.
    Dauer {
        /// Number of consecutive failures.
        streak: u32
    },
}

/// Pure rung: streak -> state.
pub fn dauer_state(fail_streak: u32) -> DauerState {
    if fail_streak >= DAUER_THRESHOLD {
        DauerState::Dauer { streak: fail_streak }
    } else {
        DauerState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dauer_state_active_zero() {
        assert_eq!(dauer_state(0), DauerState::Active);
    }

    #[test]
    fn test_dauer_state_active_one() {
        assert_eq!(dauer_state(1), DauerState::Active);
    }

    #[test]
    fn test_dauer_state_active_two() {
        assert_eq!(dauer_state(2), DauerState::Active);
    }

    #[test]
    fn test_dauer_state_dauer_at_threshold() {
        match dauer_state(3) {
            DauerState::Dauer { streak } => assert_eq!(streak, 3),
            _ => panic!("Expected Dauer state"),
        }
    }

    #[test]
    fn test_dauer_state_dauer_above_threshold() {
        match dauer_state(5) {
            DauerState::Dauer { streak } => assert_eq!(streak, 5),
            _ => panic!("Expected Dauer state"),
        }
    }
}
