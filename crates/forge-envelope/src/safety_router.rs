//! Safety Router — Grammar-Guided State Validation & Audit Gate.
//!
//! Evaluates and intercepts anomalous Sieve-13 state tokens, orchestrates
//! multi-expert debate fallbacks, and restricts generative vocabulary spaces
//! to secure, compile-time constitutional enums.

use crate::weaver::ArbitrationVerdict;

/// Structural safety router enforcing strict grammar-guided output boundaries.
pub struct SafetyRouter {
    max_severity_limit: i32,
}

impl SafetyRouter {
    /// Create a fresh safety router.
    pub fn new(max_severity_limit: i32) -> Self {
        Self { max_severity_limit }
    }

    /// Validates an incoming Sieve-13 balanced-ternary token.
    ///
    /// Verifies that every lane is bounded within strict ternary constraints [-1, 0, +1].
    /// Returns `true` if the token is structurally sound.
    pub fn validate_s13_structure(&self, s13_token: &[i8; 13]) -> bool {
        for &lane in s13_token.iter() {
            if lane < -1 || lane > 1 {
                return false; // Out-of-bounds state corruption
            }
        }
        true
    }

    /// Evaluates structural safety of a verified S13 token.
    ///
    /// If the composite physical drift exceeds the severity limit, it triggers a
    /// 2-expert debate fallback protocol.
    pub fn evaluate_state_safety(&self, s13_token: &[i8; 13], verdict: ArbitrationVerdict) -> bool {
        if !self.validate_s13_structure(s13_token) {
            return false; // Structurally unsafe
        }

        // Calculate composite physical forces
        let mut composite_drift = 0;
        for &lane in s13_token.iter() {
            composite_drift += lane.abs() as i32;
        }

        // If drift is too high or arbitration verdict is critical, trigger safety fallback
        if composite_drift > self.max_severity_limit || verdict == ArbitrationVerdict::CriticalEscalation {
            return false; // Security trigger activated: require 2-expert debate escalation
        }

        true // Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_router_validation() {
        let router = SafetyRouter::new(6);
        let valid_s13 = [0i8; 13];
        assert!(router.validate_s13_structure(&valid_s13));

        let mut invalid_s13 = [0i8; 13];
        invalid_s13[5] = 2; // corrupt state
        assert!(!router.validate_s13_structure(&invalid_s13));
    }

    #[test]
    fn test_safety_router_evaluation() {
        let router = SafetyRouter::new(6);
        let s13 = [1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0]; // 7 anomalies
        assert!(!router.evaluate_state_safety(&s13, ArbitrationVerdict::ScheduledMaintenance));
    }
}
