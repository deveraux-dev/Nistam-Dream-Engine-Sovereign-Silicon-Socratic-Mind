//! Beat quality scoring and progression ledger recording.
//! Ported from forge-daemon/harness.rs (lines 133-156).
//!
//! A PASS is scored by how much of the board is green; FAIL scores 0.
//! Permyriad (1-10000) integer scaling.

/// XP for one beat, permyriad. A PASS is scored by how much of the board is green,
/// so the harness cannot farm rank by running an empty workspace; FAIL scores 0.
///
/// # Examples
/// - Empty board (total=0): 0
/// - Pure green (green=100, total=100): 10000
/// - Half green (green=50, total=100): 5000
pub fn beat_quality(verdict: &str, green: i64, red: i64, unwired: i64) -> u16 {
    let total = green + red + unwired;
    if verdict != "PASS" || total <= 0 {
        return 0;
    }
    ((green.max(0) as u128 * 10_000 / total as u128) as u64).min(10_000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_quality_empty_board() {
        assert_eq!(beat_quality("PASS", 0, 0, 0), 0);
    }

    #[test]
    fn test_beat_quality_pure_green() {
        assert_eq!(beat_quality("PASS", 100, 0, 0), 10000);
    }

    #[test]
    fn test_beat_quality_half_green() {
        assert_eq!(beat_quality("PASS", 50, 50, 0), 5000);
    }

    #[test]
    fn test_beat_quality_fail_verdict() {
        assert_eq!(beat_quality("FAIL", 100, 0, 0), 0);
    }

    #[test]
    fn test_beat_quality_blind_verdict() {
        assert_eq!(beat_quality("BLIND", 100, 0, 0), 0);
    }

    #[test]
    fn test_beat_quality_with_unwired() {
        // 50 green out of 100 total (25 red + 25 unwired) = 5000
        assert_eq!(beat_quality("PASS", 50, 25, 25), 5000);
    }
}
