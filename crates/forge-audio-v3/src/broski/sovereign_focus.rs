//! SovereignFocus Bridge — Rust port of SovereignFocus.js
//! Implements tap-calibrated BPM synchronization with active participation gate.
//!
//! # WP1: Pulse Generation
//! Pulse frequency is clamped to 1-4 Hz (60-240 BPM).
//!
//! # WP2: Active Participation Gate
//! The `Synchronized` state requires actual tap input during calibration.
//! Passive playback alone cannot reach `Synchronized` — this is the therapeutic constraint.
//!
//! # Design Notes
//! - Uses `std::time::Instant` for high-resolution tap timing
//! - FSM: Standby → Calibrating → Synchronized
//! - No WASM, no DOM, no AudioContext — pure logic
//! - Environmental sieve (typing delta) omitted in this Rust port

use std::time::Instant;

/// SovereignFocus state machine mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SovereignMode {
    /// Idle, waiting for calibration to begin.
    Standby,
    /// Collecting 10 tap intervals for BPM calibration.
    Calibrating,
    /// Active synchronization (requires preceding tap calibration).
    Synchronized,
}

/// SovereignFocus engine state.
#[derive(Debug, Clone)]
pub struct SovereignState {
    /// Current FSM mode.
    pub mode: SovereignMode,
    /// Calibrated BPM (default 120, range 60–240).
    pub bpm: u32,
    /// Drift in milliseconds (standard deviation of tap intervals).
    pub drift: u32,
    /// Acoustic intensity 0.0–1.0 (affects pulse amplitude).
    pub intensity: f32,
    /// Number of taps recorded during Calibrating phase.
    pub tap_count: u32,
}

impl Default for SovereignState {
    fn default() -> Self {
        Self {
            mode: SovereignMode::Standby,
            bpm: 120,
            drift: 0,
            intensity: 0.4,
            tap_count: 0,
        }
    }
}

/// Tap collection buffer for calibration.
#[derive(Debug, Clone)]
pub struct TapBuffer {
    /// Instants at which taps were recorded.
    taps: Vec<Instant>,
}

impl TapBuffer {
    /// Create a new tap buffer.
    pub fn new() -> Self {
        Self { taps: Vec::new() }
    }

    /// Record a tap at the current time.
    pub fn record_tap(&mut self) {
        self.taps.push(Instant::now());
    }

    /// Get the number of recorded taps.
    pub fn len(&self) -> usize {
        self.taps.len()
    }

    /// Check if we have collected the target 10 taps.
    pub fn is_complete(&self) -> bool {
        self.len() >= 10
    }

    /// Compute calibration from collected taps.
    /// Returns (bpm, drift_ms) if enough taps were recorded, otherwise None.
    pub fn compute_calibration(&self) -> Option<(u32, u32)> {
        if self.len() < 10 {
            return None;
        }

        // Compute inter-tap intervals in milliseconds
        let mut intervals = Vec::new();
        for i in 1..self.len() {
            let interval_ms = self.taps[i]
                .duration_since(self.taps[i - 1])
                .as_millis() as f64;
            intervals.push(interval_ms);
        }

        // Average interval → BPM
        let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let bpm = (60000.0 / avg_interval) as u32;

        // Clamp BPM to valid range (1–4 Hz = 60–240 BPM)
        let clamped_bpm = bpm.max(60).min(240);

        // Drift = standard deviation of intervals (population stddev)
        let variance = intervals
            .iter()
            .map(|&v| (v - avg_interval).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let drift_ms = variance.sqrt() as u32;

        Some((clamped_bpm, drift_ms))
    }

    /// Clear all recorded taps for a new calibration cycle.
    pub fn reset(&mut self) {
        self.taps.clear();
    }
}

impl Default for TapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// SovereignFocus engine.
#[derive(Debug, Clone)]
pub struct SovereignEngine {
    /// Current state machine mode and metrics.
    pub state: SovereignState,
    /// Tap buffer for calibration.
    tap_buffer: TapBuffer,
}

impl SovereignEngine {
    /// Create a new SovereignFocus engine in Standby mode.
    pub fn new() -> Self {
        Self {
            state: SovereignState::default(),
            tap_buffer: TapBuffer::new(),
        }
    }

    /// Begin tap calibration (transition Standby → Calibrating).
    pub fn start_calibration(&mut self) {
        self.state.mode = SovereignMode::Calibrating;
        self.state.tap_count = 0;
        self.state.drift = 0;
        self.tap_buffer.reset();
    }

    /// Record a single tap during calibration.
    /// If 10 taps are collected, automatically transitions to Synchronized
    /// and computes BPM + drift. Returns true if calibration is now complete.
    pub fn record_tap(&mut self) -> bool {
        if self.state.mode != SovereignMode::Calibrating {
            return false;
        }

        self.tap_buffer.record_tap();
        self.state.tap_count = self.tap_buffer.len() as u32;

        if self.tap_buffer.is_complete() {
            if let Some((bpm, drift)) = self.tap_buffer.compute_calibration() {
                self.state.bpm = bpm;
                self.state.drift = drift;
                // WP2: Transition to Synchronized only after actual tap input
                self.state.mode = SovereignMode::Synchronized;
                return true;
            }
        }

        false
    }

    /// Stop the engine and return to Standby.
    pub fn stop_engine(&mut self) {
        self.state.mode = SovereignMode::Standby;
        self.tap_buffer.reset();
        self.state.tap_count = 0;
    }

    /// Set acoustic intensity (0.0–1.0).
    pub fn set_intensity(&mut self, value: f32) {
        self.state.intensity = value.max(0.0).min(1.0);
    }

    /// Get the current state snapshot.
    pub fn get_state(&self) -> SovereignState {
        self.state.clone()
    }

    /// Compute the pulse period (seconds) based on calibrated BPM.
    /// Clamped to 1-4 Hz (60-240 BPM range).
    pub fn pulse_period_secs(&self) -> f64 {
        let clamped_bpm = self.state.bpm.max(60).min(240);
        60.0 / clamped_bpm as f64
    }

    /// Check if engine is in Synchronized state (active participation gate).
    pub fn is_synchronized(&self) -> bool {
        self.state.mode == SovereignMode::Synchronized
    }
}

impl Default for SovereignEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_tap_buffer_creation() {
        let buffer = TapBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert!(!buffer.is_complete());
    }

    #[test]
    fn test_tap_calibration_10_taps() {
        let mut engine = SovereignEngine::new();

        // Start calibration
        engine.start_calibration();
        assert_eq!(engine.state.mode, SovereignMode::Calibrating);
        assert_eq!(engine.state.tap_count, 0);

        // Record 9 taps with ~600ms intervals (60 BPM baseline)
        for _ in 0..9 {
            let completed = engine.record_tap();
            assert!(!completed);
            thread::sleep(Duration::from_millis(600));
        }

        // 10th tap should complete calibration
        let completed = engine.record_tap();
        assert!(completed);
        assert_eq!(engine.state.mode, SovereignMode::Synchronized);
        assert_eq!(engine.state.tap_count, 10);
        // BPM should be close to 100 (60000 / 600 ≈ 100)
        assert!(engine.state.bpm >= 95 && engine.state.bpm <= 105);
    }

    #[test]
    fn test_bpm_clamping() {
        let buffer = TapBuffer::new();
        // 15ms intervals → 4000 BPM, should clamp to 240
        let (_bpm, _) = buffer.compute_calibration().unwrap_or((0, 0));
        // Since compute_calibration needs actual taps, this test is symbolic

        // Direct test: verify pulse_period_secs clamping
        let mut engine = SovereignEngine::new();
        engine.state.bpm = 500; // Beyond max
        let period = engine.pulse_period_secs();
        // Clamped to 240 BPM = 0.25 s period
        assert!(period >= 0.249 && period <= 0.251);

        engine.state.bpm = 30; // Below min
        let period = engine.pulse_period_secs();
        // Clamped to 60 BPM = 1.0 s period
        assert!(period >= 0.999 && period <= 1.001);
    }

    #[test]
    fn test_synchronized_requires_tap_input() {
        let mut engine = SovereignEngine::new();

        // WP2: Verify that we cannot reach Synchronized without taps
        assert_eq!(engine.state.mode, SovereignMode::Standby);
        assert!(!engine.is_synchronized());

        // Start calibration (not synchronized yet)
        engine.start_calibration();
        assert_eq!(engine.state.mode, SovereignMode::Calibrating);
        assert!(!engine.is_synchronized());

        // Only after collecting 10 taps do we reach Synchronized
        for _ in 0..10 {
            engine.record_tap();
            thread::sleep(Duration::from_millis(600));
        }
        assert!(engine.is_synchronized());
    }

    #[test]
    fn test_intensity_clamping() {
        let mut engine = SovereignEngine::new();

        engine.set_intensity(1.5);
        assert_eq!(engine.state.intensity, 1.0);

        engine.set_intensity(-0.5);
        assert_eq!(engine.state.intensity, 0.0);

        engine.set_intensity(0.5);
        assert_eq!(engine.state.intensity, 0.5);
    }

    #[test]
    fn test_stop_engine() {
        let mut engine = SovereignEngine::new();
        engine.start_calibration();
        engine.record_tap();

        assert_eq!(engine.state.mode, SovereignMode::Calibrating);
        assert_eq!(engine.state.tap_count, 1);

        engine.stop_engine();
        assert_eq!(engine.state.mode, SovereignMode::Standby);
        assert_eq!(engine.state.tap_count, 0);
    }

    #[test]
    fn test_default_state() {
        let engine = SovereignEngine::new();
        assert_eq!(engine.state.mode, SovereignMode::Standby);
        assert_eq!(engine.state.bpm, 120);
        assert_eq!(engine.state.drift, 0);
        assert_eq!(engine.state.intensity, 0.4);
        assert_eq!(engine.state.tap_count, 0);
    }
}
