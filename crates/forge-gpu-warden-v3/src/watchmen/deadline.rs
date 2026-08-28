use crate::manifest::Priority;
use forge_watchmen_v3::{HealthSignal, Watchman};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Tracks P0 audio miss count. Vetoes P1+ when misses exceed threshold.
pub struct DeadlineMiss {
    /// Shared counter the caller increments on each missed deadline.
    pub miss_count: Arc<AtomicU32>,
    /// Miss count at or above which this watchman fires/vetoes.
    pub threshold: u32,
}

impl DeadlineMiss {
    /// Build a watchman and the shared counter callers increment. Returns
    /// `(watchman, counter)` so the caller retains a handle to drive it.
    pub fn new(threshold: u32) -> (Self, Arc<AtomicU32>) {
        let counter = Arc::new(AtomicU32::new(0));
        let watchman = Self { miss_count: counter.clone(), threshold };
        (watchman, counter)
    }
}

impl Watchman for DeadlineMiss {
    fn name(&self) -> &'static str { "deadline" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let misses = self.miss_count.load(Ordering::Relaxed);
        if misses >= self.threshold {
            Some(HealthSignal::DeadlineMissed { miss_count: misses })
        } else {
            None
        }
    }

    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        if lane == Priority::P0Audio as u8 { return None; }
        let misses = self.miss_count.load(Ordering::Relaxed);
        if misses >= self.threshold {
            Some(("deadline", HealthSignal::DeadlineMissed { miss_count: misses }))
        } else {
            None
        }
    }
}
