#![allow(clippy::disallowed_types)] // @forge:allow_alloc — cold-path module, init-time allocations permitted
// SyncMonitor and sync_health — audio-game clock drift monitoring.

use std::sync::atomic::Ordering;

use super::viz_buffer::AudioVizBuffer;

/// Sync health status based on audio-game clock drift.
///
/// - `Green`:  drift < 10 ms  (10,000 μs)
/// - `Yellow`: 10 ms ≤ drift < 20 ms  (20,000 μs)
/// - `Red`:    drift ≥ 20 ms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    Green,
    Yellow,
    Red,
}

/// Pure function: compute sync status from the audio and game clocks
/// stored in the shared [`AudioVizBuffer`].
///
/// Reads `audio_clock_us` and `game_tick_us` with `Relaxed` ordering
/// (stale reads are acceptable for a UI health indicator).
pub fn sync_health(viz: &AudioVizBuffer) -> SyncStatus {
    sync_health_clocks(
        viz.audio_clock_us.load(Ordering::Relaxed),
        viz.game_tick_us.load(Ordering::Relaxed),
    )
}

/// [`sync_health`] on the two clocks alone. The health verdict only ever needed
/// this pair — a host that owns its own viz buffer reads its clocks and calls here
/// rather than converting a whole buffer type across the crate boundary.
pub fn sync_health_clocks(audio_us: u64, game_us: u64) -> SyncStatus {
    match audio_us.abs_diff(game_us) {
        d if d < 10_000 => SyncStatus::Green,
        d if d < 20_000 => SyncStatus::Yellow,
        _ => SyncStatus::Red,
    }
}

/// Continuously compares audio and game clocks, maintains a rolling window
/// of 64 drift samples, and computes max/avg drift for the sync indicator.
///
/// Called once per game tick from the game thread only.
pub struct SyncMonitor {
    /// Rolling window of signed drift samples (audio_us − game_us) in microseconds.
    drift_history: [i64; 64],
    /// Next write index into `drift_history`, advances modulo 64.
    drift_idx: usize,
    /// Current sync status derived from the latest drift measurement.
    pub status: SyncStatus,
    /// Maximum absolute drift observed in the current window.
    pub max_drift_us: i64,
    /// Mean of absolute drift values in the current window.
    pub avg_drift_us: i64,
}

impl SyncMonitor {
    /// Create a new `SyncMonitor` with all drift history zeroed and status Green.
    pub fn new() -> Self {
        Self {
            drift_history: [0i64; 64],
            drift_idx: 0,
            status: SyncStatus::Green,
            max_drift_us: 0,
            avg_drift_us: 0,
        }
    }

    /// Record the current audio-game drift and recompute statistics.
    ///
    /// 1. Reads `audio_clock_us` and `game_tick_us` from the viz buffer.
    /// 2. Computes signed drift (`audio_us as i64 - game_us as i64`).
    /// 3. Stores drift in the rolling window and advances the index.
    /// 4. Recomputes `max_drift_us` (max of absolute values) and
    ///    `avg_drift_us` (mean of absolute values) over the full window.
    /// 5. Updates `status` via [`sync_health`].
    pub fn update(&mut self, viz: &AudioVizBuffer) {
        self.update_clocks(
            viz.audio_clock_us.load(Ordering::Relaxed),
            viz.game_tick_us.load(Ordering::Relaxed),
        );
    }

    /// [`update`](Self::update) on the two clocks alone — the whole body of the
    /// drift monitor. Hosts with their own viz buffer feed it directly.
    pub fn update_clocks(&mut self, audio_us: u64, game_us: u64) {
        let drift = audio_us as i64 - game_us as i64;

        // Store in rolling window and advance index.
        self.drift_history[self.drift_idx] = drift;
        self.drift_idx = (self.drift_idx + 1) % 64;

        // Recompute max and avg over the full 64-element window.
        let mut max_abs: i64 = 0;
        let mut sum_abs: i64 = 0;
        for &d in &self.drift_history {
            let abs_d = d.abs();
            if abs_d > max_abs {
                max_abs = abs_d;
            }
            sum_abs += abs_d;
        }
        self.max_drift_us = max_abs;
        self.avg_drift_us = sum_abs / 64;

        self.status = sync_health_clocks(audio_us, game_us);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn green_when_clocks_equal() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(50_000, Ordering::Relaxed);
        viz.game_tick_us.store(50_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Green);
    }

    #[test]
    fn green_when_drift_below_threshold() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000 - 9_999, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Green);
    }

    #[test]
    fn yellow_at_exact_10ms_boundary() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000 - 10_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Yellow);
    }

    #[test]
    fn yellow_when_drift_in_range() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000 - 15_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Yellow);
    }

    #[test]
    fn red_at_exact_20ms_boundary() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000 - 20_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Red);
    }

    #[test]
    fn red_when_drift_large() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.audio_clock_us.store(1_000_000, Ordering::Relaxed);
        viz.game_tick_us.store(0, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Red);
    }

    #[test]
    fn drift_is_absolute_game_ahead_of_audio() {
        let viz = AudioVizBuffer::new(1024, 256);
        // game clock ahead of audio clock by 15ms → Yellow
        viz.audio_clock_us.store(50_000, Ordering::Relaxed);
        viz.game_tick_us.store(65_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Yellow);
    }

    // -------------------------------------------------------------------
    // Property 4: Sync Drift Threshold Correctness
    // **Validates: Requirements 3.4, 3.5, 3.6**
    //
    // For arbitrary (audio_us, game_us) u64 pairs, sync_health classifies
    // drift into the correct SyncStatus bucket:
    //   Green  if drift < 10,000 μs
    //   Yellow if 10,000 ≤ drift < 20,000 μs
    //   Red    if drift ≥ 20,000 μs
    // -------------------------------------------------------------------

    proptest! {
        #[test]
        fn prop_sync_drift_threshold_correctness(
            audio_us in any::<u64>(),
            game_us in any::<u64>(),
        ) {
            let viz = AudioVizBuffer::new(1024, 256);
            viz.audio_clock_us.store(audio_us, Ordering::Relaxed);
            viz.game_tick_us.store(game_us, Ordering::Relaxed);

            let drift = audio_us.abs_diff(game_us);
            let expected = match drift {
                d if d < 10_000 => SyncStatus::Green,
                d if d < 20_000 => SyncStatus::Yellow,
                _ => SyncStatus::Red,
            };

            let actual = sync_health(&viz);
            prop_assert_eq!(actual, expected,
                "audio_us={}, game_us={}, drift={}: expected {:?}, got {:?}",
                audio_us, game_us, drift, expected, actual);
        }
    }

    // -------------------------------------------------------------------
    // SyncMonitor unit tests
    // -------------------------------------------------------------------

    #[test]
    fn sync_monitor_new_defaults() {
        let sm = SyncMonitor::new();
        assert_eq!(sm.status, SyncStatus::Green);
        assert_eq!(sm.max_drift_us, 0);
        assert_eq!(sm.avg_drift_us, 0);
    }

    #[test]
    fn sync_monitor_update_records_drift() {
        let mut sm = SyncMonitor::new();
        let viz = AudioVizBuffer::new(1024, 256);

        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(95_000, Ordering::Relaxed);
        sm.update(&viz);

        // drift = 100_000 - 95_000 = 5_000 → abs = 5_000
        // All other slots are 0, so max = 5_000, avg = 5_000 / 64
        assert_eq!(sm.max_drift_us, 5_000);
        assert_eq!(sm.avg_drift_us, 5_000 / 64);
        assert_eq!(sm.status, SyncStatus::Green);
    }

    #[test]
    fn sync_monitor_update_negative_drift() {
        let mut sm = SyncMonitor::new();
        let viz = AudioVizBuffer::new(1024, 256);

        // game ahead of audio → negative signed drift
        viz.audio_clock_us.store(50_000, Ordering::Relaxed);
        viz.game_tick_us.store(65_000, Ordering::Relaxed);
        sm.update(&viz);

        // drift = 50_000 - 65_000 = -15_000 → abs = 15_000
        assert_eq!(sm.max_drift_us, 15_000);
        assert_eq!(sm.status, SyncStatus::Yellow);
    }

    #[test]
    fn sync_monitor_rolling_window_wraps() {
        let mut sm = SyncMonitor::new();
        let viz = AudioVizBuffer::new(1024, 256);

        // Fill all 64 slots with drift = 1_000
        for _ in 0..64 {
            viz.audio_clock_us.store(101_000, Ordering::Relaxed);
            viz.game_tick_us.store(100_000, Ordering::Relaxed);
            sm.update(&viz);
        }

        assert_eq!(sm.max_drift_us, 1_000);
        assert_eq!(sm.avg_drift_us, 1_000);

        // Now overwrite one slot with drift = 5_000
        viz.audio_clock_us.store(105_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000, Ordering::Relaxed);
        sm.update(&viz);

        assert_eq!(sm.max_drift_us, 5_000);
        // avg = (63 * 1_000 + 5_000) / 64 = 68_000 / 64 = 1_062
        assert_eq!(sm.avg_drift_us, (63 * 1_000 + 5_000) / 64);
    }

    #[test]
    fn sync_monitor_idx_wraps_at_64() {
        let mut sm = SyncMonitor::new();
        let viz = AudioVizBuffer::new(1024, 256);

        viz.audio_clock_us.store(100_000, Ordering::Relaxed);
        viz.game_tick_us.store(100_000, Ordering::Relaxed);

        // Call update 65 times — idx should wrap back to 1
        for _ in 0..65 {
            sm.update(&viz);
        }

        // All drifts are 0, so stats should be 0
        assert_eq!(sm.max_drift_us, 0);
        assert_eq!(sm.avg_drift_us, 0);
    }

    // -------------------------------------------------------------------
    // Property 5: SyncMonitor Rolling Window Correctness
    // **Validates: Requirement 3.7**
    //
    // For arbitrary drift sequences (1..=128 pairs), after feeding them
    // through SyncMonitor::update(), max_drift_us equals the max of
    // absolute drifts in the full 64-element window, and avg_drift_us
    // equals the mean (integer division). The window is always 64 slots
    // initialized to 0, so for N < 64 updates the remaining slots are 0.
    // -------------------------------------------------------------------

    proptest! {
        #[test]
        fn prop_sync_monitor_rolling_window_correctness(
            pairs in proptest::collection::vec(
                (0u32..200_000u32, 0u32..200_000u32),
                1..=128usize,
            ),
        ) {
            let mut sm = SyncMonitor::new();
            let viz = AudioVizBuffer::new(1024, 256);

            // Collect signed drifts as we feed them through the monitor.
            let mut all_drifts: Vec<i64> = Vec::new();

            for &(audio_us, game_us) in &pairs {
                viz.audio_clock_us.store(audio_us as u64, Ordering::Relaxed);
                viz.game_tick_us.store(game_us as u64, Ordering::Relaxed);
                sm.update(&viz);

                let drift = audio_us as i64 - game_us as i64;
                all_drifts.push(drift);
            }

            // Build the expected 64-element window:
            // The monitor always computes over the full [i64; 64] array.
            // The last min(N, 64) drifts occupy slots; the rest are 0.
            let n = all_drifts.len();
            let mut window = [0i64; 64];
            let recent = &all_drifts[n.saturating_sub(64)..];
            // Place recent drifts into the window at the positions the
            // monitor would have written them. The monitor writes
            // sequentially starting at index 0, wrapping at 64.
            // After N updates, the window contains:
            //   - If N <= 64: slots 0..N filled, slots N..64 still 0
            //   - If N > 64: all 64 slots filled with the last 64 drifts
            //     (but rotated). Either way, the set of values in the
            //     window is exactly `recent` plus zero-padding.
            if n <= 64 {
                for (i, &d) in recent.iter().enumerate() {
                    window[i] = d;
                }
                // remaining slots stay 0
            } else {
                // When N > 64, the window is fully overwritten.
                // The monitor writes at idx = N % 64 after all updates,
                // so the window is rotated. But max/avg don't depend on
                // order — just the multiset of values matters.
                for (i, &d) in recent.iter().enumerate() {
                    window[i] = d;
                }
            }

            let expected_max = window.iter().map(|d| d.abs()).max().unwrap();
            let expected_avg = window.iter().map(|d| d.abs()).sum::<i64>() / 64;

            prop_assert_eq!(
                sm.max_drift_us, expected_max,
                "max_drift_us mismatch after {} updates", n,
            );
            prop_assert_eq!(
                sm.avg_drift_us, expected_avg,
                "avg_drift_us mismatch after {} updates", n,
            );
        }
    }
}
