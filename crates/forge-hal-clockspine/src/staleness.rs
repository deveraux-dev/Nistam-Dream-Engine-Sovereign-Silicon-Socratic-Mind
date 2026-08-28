//! Staleness tier model — adapted from sf-wasm::ripple.rs's 4-zone cascade.
//! Instead of zone health driven by extraction/regen, this models channel staleness
//! driven by generation age (ticks since last publish).
//!
//! Source: E:\13forge-super\crates\sf-wasm\src\ripple.rs (15 tests, all passing).
//! Ported for scrub-bar feedback channel staleness tracking.

/// Staleness tier — maps generation age to readability/urgency.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StalenessTier {
    /// 0-25: just published, high readability.
    Fresh,
    /// 26-50: a few generations old, normal readability.
    Aging,
    /// 51-75: notably old, reduced readability.
    Stale,
    /// 76-100: very old, urgent re-publish needed.
    Deceased,
}

impl StalenessTier {
    /// Map staleness score (0-100) to tier.
    pub fn from_score(score: u8) -> Self {
        if score >= 76 {
            StalenessTier::Deceased
        } else if score >= 51 {
            StalenessTier::Stale
        } else if score >= 26 {
            StalenessTier::Aging
        } else {
            StalenessTier::Fresh
        }
    }

    /// Visual intensity multiplier (permyriad, 0-10000).
    /// Fresh channels are barely visible; Deceased channels are urgent.
    pub fn urgency_mult(self) -> u16 {
        match self {
            StalenessTier::Fresh => 5000,     // dim
            StalenessTier::Aging => 7500,     // normal
            StalenessTier::Stale => 10000,    // bright
            StalenessTier::Deceased => 15000, // urgent/flashing
        }
    }
}

/// Compute staleness score from generation age.
///
/// Each generation tick ages the staleness by a fixed rate.
/// At generation_age == 0 (fresh), staleness is 0.
/// At generation_age == 10, staleness reaches ~50 (half-stale).
/// At generation_age == 20, staleness is clamped at 100 (deceased).
///
/// The formula is: `staleness = min(100, age_ticks * 5)`.
/// This maps 20 ticks to 100%, consistent with ripple.rs's decay-per-cycle math.
pub fn score_from_age(generation_age: u64) -> u8 {
    let age_score = (generation_age as u32).saturating_mul(5).min(100);
    age_score as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_at_zero_age() {
        let score = score_from_age(0);
        assert_eq!(score, 0);
        assert_eq!(StalenessTier::from_score(score), StalenessTier::Fresh);
    }

    #[test]
    fn aging_at_middle_age() {
        let score = score_from_age(6); // 6*5 = 30
        assert_eq!(score, 30);
        assert_eq!(StalenessTier::from_score(score), StalenessTier::Aging);
    }

    #[test]
    fn stale_at_older_age() {
        let score = score_from_age(11); // 11*5 = 55
        assert_eq!(score, 55);
        assert_eq!(StalenessTier::from_score(score), StalenessTier::Stale);
    }

    #[test]
    fn deceased_at_very_old_age() {
        let score = score_from_age(20); // 20*5 = 100
        assert_eq!(score, 100);
        assert_eq!(StalenessTier::from_score(score), StalenessTier::Deceased);
    }

    #[test]
    fn clamped_at_100() {
        let score = score_from_age(50); // 50*5 = 250, clamped to 100
        assert_eq!(score, 100);
    }

    #[test]
    fn tier_boundaries_exact() {
        assert_eq!(StalenessTier::from_score(0), StalenessTier::Fresh);
        assert_eq!(StalenessTier::from_score(25), StalenessTier::Fresh);
        assert_eq!(StalenessTier::from_score(26), StalenessTier::Aging);
        assert_eq!(StalenessTier::from_score(50), StalenessTier::Aging);
        assert_eq!(StalenessTier::from_score(51), StalenessTier::Stale);
        assert_eq!(StalenessTier::from_score(75), StalenessTier::Stale);
        assert_eq!(StalenessTier::from_score(76), StalenessTier::Deceased);
        assert_eq!(StalenessTier::from_score(100), StalenessTier::Deceased);
    }

    #[test]
    fn urgency_escalates_with_staleness() {
        assert!(StalenessTier::Fresh.urgency_mult() < StalenessTier::Aging.urgency_mult());
        assert!(StalenessTier::Aging.urgency_mult() < StalenessTier::Stale.urgency_mult());
        assert!(StalenessTier::Stale.urgency_mult() < StalenessTier::Deceased.urgency_mult());
    }

    #[test]
    fn round_trip_age_to_tier() {
        for age in 0..=25u64 {
            let score = score_from_age(age);
            let tier = StalenessTier::from_score(score);
            if age <= 5 {
                assert_eq!(tier, StalenessTier::Fresh, "age {}", age);
            }
        }
    }
}
