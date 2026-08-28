//! AudioContext — centralized audio-visual bridge.
//!
//! Wraps `AudioBusHandle` and exposes all derived visual bridge fields
//! (rms, spectrum, beat_phase, sub_bass_ratio, energy_multiplier, bpm, genre, now_playing).
//! Called once per frame via `poll()` to read the latest `MixerSnapshot`.

use super::bus::AudioBusHandle;
use super::snapshot::{DeckSnapshot, DeckState};

/// Persistent player state — transport metadata for any mode.
#[derive(Clone, Debug, Default)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub deck: String,
    pub playing: bool,
    pub file: String,
}

/// Centralized audio-visual bridge. Wraps AudioBusHandle and exposes
/// all derived fields needed by the shader pipeline and Dead Drop effects.
///
/// Clone + Send: multiple subsystems can hold independent copies.
/// The AudioBusHandle inside is Clone (Arc + crossbeam Sender).
#[derive(Clone)]
pub struct AudioContext {
    /// Layer 1 handle — command channel + lock-free snapshot.
    pub bus: AudioBusHandle,

    // ── Bridge fields (populated by poll()) ──
    /// RMS level of the loudest playing deck (0.0–1.0).
    pub rms: f32,
    /// Spectral energy bands [low, mid, high] (0.0–1.0 each).
    pub spectrum: [f32; 3],
    /// Beat phase within current beat (0.0–1.0).
    pub beat_phase: f32,
    /// Sub-bass energy ratio: low / total (0.0–1.0).
    pub sub_bass_ratio: f32,
    /// Cross-deck energy stacking multiplier (0.0–1.0).
    pub energy_multiplier: f32,
    /// BPM from the mixer snapshot.
    pub bpm: f32,
    /// Genre classification string ("DnB", "Techno", "Deep", "Other").
    pub genre: String,
    /// Persistent now-playing transport metadata.
    pub now_playing: NowPlaying,
    /// ForgeVision warning active — triggers Dead Drop amber pulse.
    pub warning_active: bool,
    /// Timestamp when last warning was set (for 5-second auto-clear).
    warning_set_time: Option<std::time::Instant>,
}

impl AudioContext {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self {
            bus,
            rms: 0.0,
            spectrum: [0.0; 3],
            beat_phase: 0.0,
            sub_bass_ratio: 0.0,
            energy_multiplier: 0.0,
            bpm: 0.0,
            genre: String::new(),
            now_playing: NowPlaying::default(),
            warning_active: false,
            warning_set_time: None,
        }
    }

    /// Read the latest MixerSnapshot and populate all bridge fields.
    /// Called once per frame in render_shell().
    pub fn poll(&mut self) {
        let snap = self.bus.snapshot.load();

        let primary_idx = snap
            .decks
            .iter()
            .enumerate()
            .filter(|(_, d)| d.state == DeckState::Playing)
            .max_by(|(_, a), (_, b)| {
                a.waveform_peak
                    .partial_cmp(&b.waveform_peak)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let deck = &snap.decks[primary_idx];

        self.rms = deck.waveform_peak.clamp(0.0, 1.0);

        self.spectrum = [
            deck.spectral_energy[0].clamp(0.0, 1.0),
            deck.spectral_energy[1].clamp(0.0, 1.0),
            deck.spectral_energy[2].clamp(0.0, 1.0),
        ];

        self.beat_phase = deck.beat_phase.clamp(0.0, 1.0);

        self.bpm = snap.bpm;

        self.genre = match deck.genre {
            Some(0) => "DnB".to_string(),
            Some(1) => "Techno".to_string(),
            Some(2) => "Deep".to_string(),
            _ => "Other".to_string(),
        };

        let total = self.spectrum[0] + self.spectrum[1] + self.spectrum[2];
        self.sub_bass_ratio = if total > 0.001 {
            (self.spectrum[0] / total).clamp(0.0, 1.0)
        } else {
            0.0
        };

        self.energy_multiplier = compute_energy_multiplier(&snap.decks);

        let any_playing = snap
            .decks
            .iter()
            .any(|d| d.state == DeckState::Playing);

        if any_playing {
            if let Some(ref track) = deck.track {
                self.now_playing.title = track.title.clone();
                self.now_playing.artist = track.artist.clone();
                self.now_playing.deck = format!("{}", (b'A' + primary_idx as u8) as char);
                self.now_playing.playing = true;
                self.now_playing.file = track.path.clone();
            }
        } else {
            self.now_playing.playing = false;
        }

        if self.warning_active {
            if let Some(set_time) = self.warning_set_time {
                if set_time.elapsed().as_secs_f32() > 5.0 {
                    self.warning_active = false;
                    self.warning_set_time = None;
                }
            }
        }
    }

    pub fn set_warning(&mut self) {
        self.warning_active = true;
        self.warning_set_time = Some(std::time::Instant::now());
    }

    /// Get the amber pulse intensity for Dead Drop shader (0.0–1.0).
    pub fn warning_pulse_intensity(&self) -> f32 {
        if !self.warning_active { return 0.0; }
        let t = self.warning_set_time
            .map(|s| s.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        ((t * std::f32::consts::TAU).sin() + 1.0) * 0.5
    }
}

// ★ hot-alloc → zero-alloc pairwise scan over fixed array
fn compute_energy_multiplier(decks: &[DeckSnapshot; 4]) -> f32 {
    let mut max_mult: f32 = 0.0;
    for i in 0..4 {
        if decks[i].state != DeckState::Playing || decks[i].waveform_peak <= 0.01 { continue; }
        for j in (i + 1)..4 {
            if decks[j].state != DeckState::Playing || decks[j].waveform_peak <= 0.01 { continue; }
            let mult = (decks[i].waveform_peak * decks[j].waveform_peak).powf(1.5);
            if mult > max_mult { max_mult = mult; }
        }
    }
    max_mult.clamp(0.0, 1.0)
}
