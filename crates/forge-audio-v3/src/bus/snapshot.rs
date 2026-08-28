//! LiveMixerState, DeckSnapshot, DeckState — lock-free state published by the feeder thread.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::effect::ActiveEffect;
use super::track::TrackInfo;

/// State of a single mixer deck.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum DeckState {
    /// No track loaded.
    #[default]
    Empty,
    /// Track is being decoded / buffered.
    Loading,
    /// Track is actively playing.
    Playing,
    /// Playback paused — position preserved.
    Paused,
    /// Playback stopped — position reset to 0.
    Stopped,
}

/// Lightweight beatgrid info for snapshot (no methods, just data).
#[derive(Clone, Debug, Default)]
pub struct BeatGridInfo {
    pub bpm: f32,
    pub first_beat_frac: f64,
    pub beat_interval_frac: f64,
}

/// Snapshot of a single deck, embedded in [`LiveMixerState`].
#[derive(Clone, Debug)]
pub struct DeckSnapshot {
    /// Currently loaded track metadata, `None` when deck is [`DeckState::Empty`].
    pub track: Option<TrackInfo>,
    /// Current deck state.
    pub state: DeckState,
    /// Current playback position in seconds.
    pub position_secs: f64,
    /// Total track duration in seconds (0.0 when empty).
    pub duration_secs: f64,
    /// Per-deck volume (0.0–1.0).
    pub volume: f32,
    /// Peak waveform amplitude for this deck (0.0–1.0).
    pub waveform_peak: f32,
    pub effects: Vec<ActiveEffect>,
    /// Error message if the last operation failed (e.g. undecodable file).
    pub error_message: Option<String>,
    /// RMS metering level (0.0–1.0).
    pub rms_level: f32,
    /// Spectral energy bands [low, mid, high] (0.0–1.0 each).
    pub spectral_energy: [f32; 3],
    /// Current EQ gains in dB [high, mid, low].
    pub eq_bands: [f32; 3],
    /// Beat phase within current beat (0.0–1.0).
    pub beat_phase: f32,
    /// Genre classification byte (0=DnB, 1=Techno, 2=Deep, 3=Other).
    pub genre: Option<u8>,
    /// 200-point 3-band energy overview [low, mid, high] for coloring.
    pub waveform_bands: [[f32; 3]; 200],
    /// Pan position, -1.0 (left) .. 1.0 (right), center = 0.0 (G-AUDIO-03 wire).
    pub pan: f32,
    /// Muted flag (G-AUDIO-03 wire).
    pub muted: bool,
    /// Solo flag (G-AUDIO-03 wire).
    pub solo: bool,
}


impl Default for DeckSnapshot {
    fn default() -> Self {
        Self {
            track: None,
            state: DeckState::Empty,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 1.0,
            waveform_peak: 0.0,
            effects: Vec::new(),
            error_message: None,
            rms_level: 0.0,
            spectral_energy: [0.0; 3],
            eq_bands: [0.0; 3],
            beat_phase: 0.0,
            genre: None,
            waveform_bands: [[0.0; 3]; 200],
            pan: 0.0,
            muted: false,
            solo: false,
        }
    }
}

/// Live mixer state published via `ArcSwap` by the feeder thread after every
/// mix cycle. Panels call `snapshot.load()` to get an `Arc<LiveMixerState>`
/// with zero contention.
///
/// Distinct from `crate::snapshot::MixerSnapshot` (the serializable capture
/// face built from `mixer::Mixer::capture`). This type is the bus-level
/// real-time read face — `Clone` only, no `Serialize`.
#[derive(Clone, Debug)]
pub struct LiveMixerState {
    pub decks: [DeckSnapshot; 4],
    /// Master output volume (0.0–1.0).
    pub master_volume: f32,
    /// Crossfader position (-1.0 = left, 1.0 = right).
    pub crossfader: f32,
    /// Current BPM (beats per minute).
    pub bpm: f32,
    pub is_playing: bool,
    /// Monotonic frame counter for change detection.
    pub frame: u64,
    /// PCM samples for visualisers (last N frames).
    pub waveform_buffer: Arc<[f32]>,
    /// FFT magnitudes for spectrum analysers.
    pub spectrum: Arc<[f32]>,
    /// Timestamp of snapshot creation in nanoseconds.
    pub timestamp_ns: u64,
    /// Number of audio buffer underruns detected by the feeder thread.
    pub underrun_count: u64,
    /// 200-point waveform overview per deck for scrolling display.
    pub waveform_overviews: [[f32; 200]; 4],
    /// 200-point 3-band energy overview per deck for coloring.
    pub waveform_bands_overviews: [[[f32; 3]; 200]; 4],
    /// Beatgrid info per deck (None if no beatgrid detected).
    pub beatgrid_info: [Option<BeatGridInfo>; 4],
    /// Wrap-aware beat phase delta between two loudest playing decks.
    pub beat_phase_delta: f32,
    /// Pre-fader listen active per deck.
    pub pfl_active: [bool; 4],
    // -- Mic channel --
    pub mic_active: bool,
    pub mic_volume: f32,
    pub mic_peak: f32,
    pub mic_rms: f32,
    pub mic_talkover_db: f32,
    pub mic_attached: bool,
    pub mic_auto_duck_enabled: bool,
    pub mic_monitor_enabled: bool,
    pub mic_duck_applied_db: f32,
    // -- Step Sequencer --
    /// Current step position (0-15), None if sequencer not active.
    pub seq_step: Option<usize>,
    /// Last mix_block processing time in microseconds (for profiler).
    pub last_mix_us: u64,
}

impl Default for LiveMixerState {
    fn default() -> Self {
        Self {
            decks: [
                DeckSnapshot::default(),
                DeckSnapshot::default(),
                DeckSnapshot::default(),
                DeckSnapshot::default(),
            ],
            master_volume: 1.0,
            crossfader: 0.0,
            bpm: 0.0,
            is_playing: false,
            frame: 0,
            waveform_buffer: Arc::from(Vec::new().into_boxed_slice()),
            spectrum: Arc::from(Vec::new().into_boxed_slice()),
            timestamp_ns: 0,
            underrun_count: 0,
            waveform_overviews: [[0.0; 200]; 4],
            waveform_bands_overviews: [[[0.0; 3]; 200]; 4],
            beatgrid_info: [None, None, None, None],
            beat_phase_delta: 0.0,
            pfl_active: [false; 4],
            mic_active: false,
            mic_volume: 1.0,
            mic_peak: 0.0,
            mic_rms: 0.0,
            mic_talkover_db: 0.0,
            mic_attached: false,
            mic_auto_duck_enabled: false,
            mic_monitor_enabled: false,
            mic_duck_applied_db: 0.0,
            seq_step: None,
            last_mix_us: 0,
        }
    }
}
