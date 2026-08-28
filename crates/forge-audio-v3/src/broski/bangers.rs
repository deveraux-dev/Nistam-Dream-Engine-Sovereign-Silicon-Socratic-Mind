//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\bangers.rs (191 LOC).
//!
//! Broski Banger Tracker — learns which tracks slap. In-memory, ephemeral.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BangerScore {
    pub path: String,
    pub play_count: u32,
    pub skip_count: u32,
    pub avg_peak_energy: f64,
    pub ghost_delta: i32,
    pub avg_play_percentage: f64,
    pub energy_stacking_count: u32,
    pub composite_score: f64,
}

fn compute_composite(stats: &BangerScore) -> f64 {
    let play_score = (stats.play_count as f64 / 20.0).min(1.0);
    let energy_score = stats.avg_peak_energy.min(1.0);
    let ghost_score = ((stats.ghost_delta as f64) / 10.0).clamp(0.0, 1.0);
    let completion_score = stats.avg_play_percentage;
    let stacking_score = (stats.energy_stacking_count as f64 / 10.0).min(1.0);
    let skip_penalty = (stats.skip_count as f64 / (stats.play_count.max(1) as f64)).min(0.5);
    (play_score * 0.25 + energy_score * 0.25 + ghost_score * 0.15
        + completion_score * 0.20 + stacking_score * 0.15 - skip_penalty)
        .clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Default)]
struct RawStats {
    play_count: u32,
    skip_count: u32,
    total_peak_energy: f64,
    total_ghost_delta: i32,
    total_play_percentage: f64,
    energy_stacking_count: u32,
}

pub struct BangerTracker {
    stats: HashMap<String, RawStats>,
}

impl Default for BangerTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BangerTracker {
    pub fn new() -> Self { Self { stats: HashMap::new() } }

    pub fn record_play(&mut self, path: &str, peak_energy: f64, ghost_delta: i32, play_percentage: f64, was_stacking: bool) {
        let s = self.stats.entry(path.to_string()).or_default();
        s.play_count += 1;
        s.total_peak_energy += peak_energy;
        s.total_ghost_delta += ghost_delta;
        s.total_play_percentage += play_percentage;
        if was_stacking { s.energy_stacking_count += 1; }
    }

    pub fn record_skip(&mut self, path: &str) {
        self.stats.entry(path.to_string()).or_default().skip_count += 1;
    }

    pub fn score_for(&self, path: &str) -> Option<BangerScore> {
        let s = self.stats.get(path)?;
        let pc = s.play_count.max(1) as f64;
        let mut score = BangerScore {
            path: path.to_string(), play_count: s.play_count, skip_count: s.skip_count,
            avg_peak_energy: s.total_peak_energy / pc, ghost_delta: s.total_ghost_delta,
            avg_play_percentage: s.total_play_percentage / pc,
            energy_stacking_count: s.energy_stacking_count, composite_score: 0.0,
        };
        score.composite_score = compute_composite(&score);
        Some(score)
    }

    pub fn top_bangers(&self, limit: usize) -> Vec<BangerScore> {
        let mut scores: Vec<_> = self.stats.keys()
            .filter_map(|p| self.score_for(p)).collect();
        scores.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());
        scores.truncate(limit);
        scores
    }

    pub fn duds(&self, limit: usize) -> Vec<BangerScore> {
        let mut all = self.top_bangers(1000);
        all.reverse();
        all.retain(|s| s.composite_score < 0.2);
        all.truncate(limit);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_score_range() {
        let s = BangerScore { path: String::new(), play_count: 100, skip_count: 50,
            avg_peak_energy: 2.0, ghost_delta: -100, avg_play_percentage: 0.0,
            energy_stacking_count: 0, composite_score: 0.0 };
        let c = compute_composite(&s);
        assert!((0.0..=1.0).contains(&c));
    }

    #[test]
    fn test_high_play_count_scores_well() {
        let high = BangerScore { path: String::new(), play_count: 20, skip_count: 0,
            avg_peak_energy: 0.5, ghost_delta: 0, avg_play_percentage: 0.8,
            energy_stacking_count: 0, composite_score: 0.0 };
        let low = BangerScore { play_count: 2, ..high.clone() };
        assert!(compute_composite(&high) > compute_composite(&low));
    }

    #[test]
    fn test_skip_penalty() {
        let clean = BangerScore { path: String::new(), play_count: 10, skip_count: 0,
            avg_peak_energy: 0.5, ghost_delta: 0, avg_play_percentage: 0.8,
            energy_stacking_count: 0, composite_score: 0.0 };
        let skipped = BangerScore { skip_count: 8, ..clean.clone() };
        assert!(compute_composite(&clean) > compute_composite(&skipped));
    }

    #[test]
    fn test_top_bangers_ordered() {
        let mut t = BangerTracker::new();
        t.record_play("/banger.mp3", 0.9, 5, 0.95, true);
        t.record_play("/banger.mp3", 0.9, 5, 0.95, true);
        t.record_play("/meh.mp3", 0.2, 0, 0.3, false);
        let top = t.top_bangers(10);
        assert!(top.len() >= 2);
        assert!(top[0].composite_score >= top[1].composite_score);
    }

    #[test]
    fn test_record_play_updates() {
        let mut t = BangerTracker::new();
        t.record_play("/x.mp3", 0.8, 2, 0.9, false);
        t.record_play("/x.mp3", 0.6, 1, 0.7, true);
        let s = t.score_for("/x.mp3").unwrap();
        assert_eq!(s.play_count, 2);
        assert!((s.avg_peak_energy - 0.7).abs() < 0.01);
    }

    use proptest::prelude::*;

    // Validates: Requirements 14.3
    // Property 12: Banger score bounded — composite_score is always in [0.0, 1.0]
    proptest! {
        #[test]
        fn prop_banger_score_bounded(
            play_count in 0u32..1000,
            skip_count in 0u32..1000,
            avg_peak_energy in -10.0f64..10.0,
            ghost_delta in -100i32..100,
            avg_play_percentage in -1.0f64..2.0,
            energy_stacking_count in 0u32..100,
        ) {
            let score = BangerScore {
                path: String::new(),
                play_count,
                skip_count,
                avg_peak_energy,
                ghost_delta,
                avg_play_percentage,
                energy_stacking_count,
                composite_score: 0.0,
            };
            let c = compute_composite(&score);
            prop_assert!((0.0..=1.0).contains(&c),
                "composite_score {} out of bounds for play={}, skip={}, energy={}, ghost={}, play%={}, stacking={}",
                c, play_count, skip_count, avg_peak_energy, ghost_delta, avg_play_percentage, energy_stacking_count);
        }
    }

    // Validates: Requirements 14.4
    // Property 13: Top bangers sorted — top_bangers() returns descending composite_score
    proptest! {
        #[test]
        fn prop_top_bangers_sorted(
            tracks in prop::collection::vec(
                (
                    1u32..50,       // play_count
                    0u32..20,       // skip_count
                    0.0f64..1.0,    // peak_energy
                    -10i32..10,     // ghost_delta
                    0.0f64..1.0,    // play_percentage
                    prop::bool::ANY, // was_stacking
                ),
                1..20,
            )
        ) {
            let mut tracker = BangerTracker::new();
            for (i, (plays, skips, energy, ghost, pct, stacking)) in tracks.iter().enumerate() {
                let path = format!("/track_{}.mp3", i);
                for _ in 0..*plays {
                    tracker.record_play(&path, *energy, *ghost, *pct, *stacking);
                }
                for _ in 0..*skips {
                    tracker.record_skip(&path);
                }
            }
            let top = tracker.top_bangers(100);
            for w in top.windows(2) {
                prop_assert!(w[0].composite_score >= w[1].composite_score,
                    "top_bangers not sorted: {} < {}", w[0].composite_score, w[1].composite_score);
            }
        }
    }
}
