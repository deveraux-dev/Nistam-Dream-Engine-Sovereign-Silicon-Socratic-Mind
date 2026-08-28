//! Run Summary — stats captured at the end of an arena quest.
//!
//! Ported by translation from forge-game-systems::run_summary. Integer-only
//! representation; no serde (the mud is pure machine, never crosses the serde
//! boundary).

/// Statistics from a completed arena run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Number of enemies defeated.
    pub kills: u32,
    /// Number of times the player died.
    pub deaths: u16,
    /// Brand corruption accumulation (0-255).
    pub brand_corruption: u8,
    /// Tithe debt accumulation (0-255).
    pub tithe_debt: u8,
    /// Final wave number reached.
    pub wave_number: u32,
    /// Total ticks survived.
    pub ticks_survived: u64,
}

impl Default for RunSummary {
    fn default() -> Self {
        Self {
            kills: 0,
            deaths: 0,
            brand_corruption: 0,
            tithe_debt: 0,
            wave_number: 0,
            ticks_survived: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_summary_default_is_empty() {
        let s = RunSummary::default();
        assert_eq!(s.kills, 0);
        assert_eq!(s.deaths, 0);
        assert_eq!(s.ticks_survived, 0);
    }

    #[test]
    fn run_summary_equality() {
        let s1 = RunSummary {
            kills: 42,
            deaths: 3,
            brand_corruption: 128,
            tithe_debt: 64,
            wave_number: 7,
            ticks_survived: 12345,
        };
        let s2 = RunSummary {
            kills: 42,
            deaths: 3,
            brand_corruption: 128,
            tithe_debt: 64,
            wave_number: 7,
            ticks_survived: 12345,
        };
        assert_eq!(s1, s2);
    }

    #[test]
    fn run_summary_inequality() {
        let s1 = RunSummary { kills: 42, ..Default::default() };
        let s2 = RunSummary { kills: 43, ..Default::default() };
        assert_ne!(s1, s2);
    }
}
