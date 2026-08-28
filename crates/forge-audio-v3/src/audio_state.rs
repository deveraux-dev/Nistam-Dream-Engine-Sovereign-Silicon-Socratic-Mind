//! AudioState — sovereign vibe-matrix snapshot for the render thread.
//!
//! Published by the audio feeder once per buffer via `ArcSwap`. Consumed
//! by the render thread once per frame to drive `vibe_matrix::vibe_tick`.
//!
//! Distinct from `MixerSnapshot` (UI-facing, ~50 ms cadence). `AudioState`
//! is hot-path: per-buffer publish, lock-free read, fixed-size payload.
//!
//! Bit-depth: f32 throughout. This snapshot crosses the audio→render
//! boundary; f32 is the GPU side of the ladder.

use std::sync::Arc;
use arc_swap::ArcSwap;
use crate::bpm::BeatGrid;

/// Number of FFT bins exposed in the published spectrum. Matches the
/// existing analyzer cadence in dreadpiratedev (`fft = vec![0.0f32; 64]`).
pub const SPECTRUM_BINS: usize = 64;

/// Lock-free snapshot of audio state for vibe matrix consumption.
#[derive(Clone, Debug)]
pub struct AudioState {
    /// Beat grid of the active deck. None if no track loaded yet.
    /// Pair with `sample_pos` for grid-locked rhythm via `BeatGrid::phase_at`.
    pub beat_grid: Option<BeatGrid>,
    /// Monotonic sample position from feeder start. Single global clock
    /// so the render thread can lock to the audio grid without drift.
    pub sample_pos: usize,
    /// Master-bus RMS energy, 0.0-1.0.
    pub energy: f32,
    /// Spectral centroid, 0.0-1.0. Hint for vibe preset selection.
    pub spectral_centroid: f32,
    /// Set true for one buffer when an onset spike crosses the drop
    /// threshold. Use for impact-frame triggers (chromatic, shake).
    pub drop_detected: bool,
    /// 64-bin master FFT magnitude, 0.0-1.0 normalized.
    pub spectrum: [f32; SPECTRUM_BINS],
    /// Genre hint (0-7), maps to fog tints / vibe presets.
    pub genre: u8,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            beat_grid: None,
            sample_pos: 0,
            energy: 0.0,
            spectral_centroid: 0.5,
            drop_detected: false,
            spectrum: [0.0f32; SPECTRUM_BINS],
            genre: 0,
        }
    }
}

/// Lock-free publisher handle. Audio thread holds one; render thread holds clones.
pub type AudioStatePublisher = Arc<ArcSwap<AudioState>>;

/// Build a fresh publisher seeded with default state.
pub fn new_publisher() -> AudioStatePublisher {
    Arc::new(ArcSwap::from_pointee(AudioState::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpm::BeatGrid;

    #[test]
    fn default_state_is_safe() {
        let s = AudioState::default();
        assert_eq!(s.sample_pos, 0);
        assert_eq!(s.energy, 0.0);
        assert!(s.beat_grid.is_none());
        assert_eq!(s.spectrum.len(), SPECTRUM_BINS);
        assert!(!s.drop_detected);
    }

    #[test]
    fn phase_at_via_beat_grid_half_beat() {
        // 120 BPM at 48kHz = 24000 samples per beat.
        let grid = BeatGrid::from_bpm(120.0, 0, 48_000, 48_000);
        let mut state = AudioState::default();
        state.beat_grid = Some(grid);
        state.sample_pos = 12_000;
        let phase = state.beat_grid.as_ref().unwrap().phase_at(state.sample_pos);
        assert!((phase - 0.5).abs() < 0.01, "phase={phase}");
    }

    #[test]
    fn phase_at_via_beat_grid_on_beat() {
        let grid = BeatGrid::from_bpm(120.0, 0, 48_000, 48_000);
        let mut state = AudioState::default();
        state.beat_grid = Some(grid);
        state.sample_pos = 24_000;
        let phase = state.beat_grid.as_ref().unwrap().phase_at(state.sample_pos);
        assert!(!(0.01..=0.99).contains(&phase), "phase={phase}");
    }

    #[test]
    fn publisher_load_store_roundtrip() {
        let p = new_publisher();
        let mut s = (**p.load()).clone();
        s.energy = 0.7;
        s.sample_pos = 999;
        s.drop_detected = true;
        s.spectrum[10] = 0.42;
        p.store(Arc::new(s));
        let loaded = p.load();
        assert_eq!(loaded.energy, 0.7);
        assert_eq!(loaded.sample_pos, 999);
        assert!(loaded.drop_detected);
        assert_eq!(loaded.spectrum[10], 0.42);
    }

    #[test]
    fn publisher_clone_shares_state() {
        let p1 = new_publisher();
        let p2 = p1.clone();
        let mut s = (**p1.load()).clone();
        s.energy = 0.5;
        p1.store(Arc::new(s));
        assert_eq!(p2.load().energy, 0.5);
    }
}
