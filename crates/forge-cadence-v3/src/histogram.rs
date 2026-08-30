//! Per-phase duration distributions, for gauging the `PERMYRIAD_BUDGET`
//! (external — not defined in this crate) split against what a session actually spent.
//!
//! Ported from aspire.rs:324 (CADENCE lane, `phase-histogram`, NEXT) — minus
//! the `hdrhistogram` dependency the aspire row's mechanism names. Three
//! phases at session-close cadence (not a hot-path/governor-tick counter)
//! do not clear L19's bar for a logarithmic-bucket histogram crate; a sorted
//! sample vec and exact quantile is cheaper here and needs no external dep.
//!
//! No wall-clock lives here (C14 firewall): every sample is a caller-timed
//! `u32` millisecond duration, recorded off the tape's own timestamp
//! columns (`forge_vcs_v3::tape::TapeRow::timestamp_ms`), never
//! `Instant::now()`.

use crate::Phase;

/// Per-phase duration samples, indexed by [`Phase::as_u8`].
#[derive(Debug, Clone, Default)]
pub struct PhaseHistogram {
    samples: [Vec<u32>; 3],
}

impl PhaseHistogram {
    /// An empty histogram.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one caller-timed duration (milliseconds) spent in `phase`.
    pub fn record(&mut self, phase: Phase, duration_ms: u32) {
        self.samples[phase.as_u8() as usize].push(duration_ms);
    }

    /// How many samples `phase` has recorded.
    pub fn count(&self, phase: Phase) -> usize {
        self.samples[phase.as_u8() as usize].len()
    }

    /// The `q`-th percentile (`0..=100`) duration for `phase`, or `None` if
    /// no samples were recorded. `q` is clamped to `100`.
    pub fn quantile(&self, phase: Phase, q: u8) -> Option<u32> {
        let bucket = &self.samples[phase.as_u8() as usize];
        if bucket.is_empty() {
            return None;
        }
        let mut sorted = bucket.clone();
        sorted.sort_unstable();
        let q = q.min(100) as usize;
        let idx = (sorted.len() - 1) * q / 100;
        Some(sorted[idx])
    }

    /// This phase's share of total recorded time, in permyriad (parts per
    /// 10000) across all three phases — the value to gauge against
    /// `PERMYRIAD_BUDGET` (external). `None` if nothing has been recorded yet.
    pub fn measured_permyriad(&self, phase: Phase) -> Option<u16> {
        let total: u64 = self.samples.iter().flatten().map(|&v| v as u64).sum();
        if total == 0 {
            return None;
        }
        let this: u64 = self.samples[phase.as_u8() as usize]
            .iter()
            .map(|&v| v as u64)
            .sum();
        Some(((this * 10_000) / total) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Phase::*;

    #[test]
    fn an_empty_histogram_reports_nothing() {
        let h = PhaseHistogram::new();
        assert_eq!(h.count(Floor), 0);
        assert_eq!(h.quantile(Floor, 50), None);
        assert_eq!(h.measured_permyriad(Floor), None);
    }

    #[test]
    fn quantile_is_exact_on_a_known_sample_set() {
        let mut h = PhaseHistogram::new();
        for ms in [10, 20, 30, 40, 50] {
            h.record(Circuit, ms);
        }
        assert_eq!(h.count(Circuit), 5);
        assert_eq!(h.quantile(Circuit, 0), Some(10));
        assert_eq!(h.quantile(Circuit, 50), Some(30));
        assert_eq!(h.quantile(Circuit, 100), Some(50));
        assert_eq!(h.quantile(Circuit, 255 as u8 % 101), h.quantile(Circuit, 255 % 101));
    }

    #[test]
    fn quantile_input_order_does_not_matter() {
        let mut a = PhaseHistogram::new();
        let mut b = PhaseHistogram::new();
        for ms in [30, 10, 50, 20, 40] {
            a.record(Surface, ms);
        }
        for ms in [10, 20, 30, 40, 50] {
            b.record(Surface, ms);
        }
        for q in [0, 25, 50, 75, 100] {
            assert_eq!(a.quantile(Surface, q), b.quantile(Surface, q));
        }
    }

    #[test]
    fn measured_permyriad_reflects_an_even_three_way_split() {
        let mut h = PhaseHistogram::new();
        h.record(Floor, 100);
        h.record(Circuit, 100);
        h.record(Surface, 100);
        assert_eq!(h.measured_permyriad(Floor), Some(3333));
        assert_eq!(h.measured_permyriad(Circuit), Some(3333));
        assert_eq!(h.measured_permyriad(Surface), Some(3333));
    }

    #[test]
    fn a_phase_with_no_samples_measures_zero_permyriad_once_others_have_run() {
        let mut h = PhaseHistogram::new();
        h.record(Floor, 100);
        assert_eq!(h.measured_permyriad(Circuit), Some(0));
    }
}
