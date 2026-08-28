//! MixerSnapshot — lightweight read-only state for UI/controller threads.
//! Updated by the audio worker thread every ~50ms. Never written by UI.

use crate::mixer::{Mixer, StemState};
use serde::Serialize;

/// Snapshot of a single deck's state
#[derive(Clone, Debug, Default, Serialize)]
pub struct DeckSnapshot {
    pub playing: bool,
    pub pos: f64,
    pub duration: f64,
    pub bpm: Option<f32>,
    pub volume: f32,
    pub tempo: f32,
    pub looping: bool,
    pub hotcues: [Option<crate::mixer::HotcueSlot>; 8],
    pub stems: StemState,
    pub waveform: Vec<f32>,
    pub eq: [f32; 3],  // high, mid, low in dB
    pub peak_level: f32,
    pub rms_level: f32,
    pub spectral_energy: [f32; 3], // [low, mid, high]
    #[serde(skip)]
    pub fft_bins: Vec<f32>,        // 64-bin FFT magnitude spectrum (runtime-computed, not serialized)
    pub slip_active: bool,
    pub replay_gain_db: f32,
    pub beat_phase: f32, // 0.0-1.0 within current beat
    pub has_beatgrid: bool,
    pub sync_mode: crate::mixer::SyncMode,
    pub pfl: bool,
    pub keylock: bool,
    pub scratching: bool,
    pub reverse: bool,
    pub sample_rate: u32,
    pub quantize: bool,
    pub waveform_bands: Vec<[f32; 3]>,  // [low, mid, high] per window
    pub loop_start_frac: f64,  // 0.0-1.0 fraction of track
    pub loop_end_frac: f64,
    pub title: String,
    pub artist: String,
    pub key: Option<String>,
    pub genre: Option<u8>,
}

/// Read-only snapshot of mixer state for UI display.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MixerSnapshot {
    pub decks: [DeckSnapshot; 4],
    pub crossfader: f32,
    pub crossfader_curve: f32,
    pub master_volume: f32,
    pub master_peak: f32,
    pub recording: bool,
    pub fx_slots: [(bool, f32, Option<String>); 4],
    // Correspondence Bus state
    pub vocal_energy: [f32; 4],
    pub harmonic_compat: f32,
    pub vocal_collision: f32,
    pub groove_lock: f32,
    /// 64-bin master output FFT — shows all FX, heal, everything
    pub master_fft: Vec<f32>,
    pub heal_active: bool,
    pub heal_intensity: f32,
    pub heal_mode: u8,
    pub hw_fader_override: [bool; 4],
    /// SF-017: LOCKED — last decode error per deck. None = no error (deck empty or loaded ok).
    /// Set by DeckLoadFailed command; cleared by LoadDeck on success.
    pub deck_load_errors: [Option<String>; 4],
    /// Mic channel state (surfaced for UI meters + toggle feedback).
    pub mic_active: bool,
    pub mic_volume: f32,
    pub mic_peak: f32,
    pub mic_rms: f32,
    pub mic_talkover_db: f32,
    pub mic_attached: bool,
    /// Envelope-driven auto-duck armed.
    pub mic_auto_duck_enabled: bool,
    /// Monitor (loopback) routing armed: mic → headphone bus.
    pub mic_monitor_enabled: bool,
    /// Currently applied gain reduction in dB on the master bus (negative = ducking, 0 = no duck).
    /// Drives the UI GR meter. Refreshed each mixer block.
    /// Headphone cue volume (0.0..1.0)
    pub headphone_volume: f32,
    /// Headphone cue/main blend (0.0 = cue only, 1.0 = main only)
    pub headphone_blend: f32,
    pub mic_duck_applied_db: f32,
}

impl MixerSnapshot {
    /// Capture current mixer state into a snapshot. Called by audio worker thread.
    pub fn capture(mixer: &Mixer) -> Self {
        let mut decks = [
            DeckSnapshot::default(),
            DeckSnapshot::default(),
            DeckSnapshot::default(),
            DeckSnapshot::default(),
        ];

        for (i, deck_data) in mixer.decks.iter().enumerate() {
            let duration = deck_data.buffer.as_ref()
                .map(|b| b.len() as f64 / b.sample_rate as f64)
                .unwrap_or(0.0);

            decks[i] = DeckSnapshot {
                playing: deck_data.params.playing,
                pos: deck_data.buffer.as_ref()
                    .map(|b| deck_data.playback_pos as f64 / b.sample_rate as f64)
                    .unwrap_or(0.0),
                duration,
                bpm: deck_data.bpm,
                volume: deck_data.params.volume,
                tempo: deck_data.params.tempo,
                looping: deck_data.params.looping,
                hotcues: deck_data.hotcues,
                stems: deck_data.stems.clone(),
                waveform: deck_data.waveform_cache.clone(),
                eq: [
                    deck_data.params.eq.high,
                    deck_data.params.eq.mid,
                    deck_data.params.eq.low,
                ],
                peak_level: mixer.peak_levels[i],
                rms_level: mixer.rms_levels[i],
                spectral_energy: mixer.spectral_energy[i],
                fft_bins: mixer.fft_bins[i].to_vec(),
                slip_active: deck_data.slip_active,
                replay_gain_db: deck_data.params.replay_gain_db,
                beat_phase: deck_data.beatgrid.as_ref()
                    .map(|bg| bg.phase_at(deck_data.playback_pos))
                    .unwrap_or(0.0),
                has_beatgrid: deck_data.beatgrid.is_some(),
                sync_mode: deck_data.sync_mode,
                pfl: deck_data.params.pfl,
                keylock: deck_data.params.keylock,
                scratching: deck_data.scratching,
                reverse: deck_data.reverse,
                sample_rate: deck_data.buffer.as_ref().map(|b| b.sample_rate).unwrap_or(44100),
                quantize: deck_data.params.quantize,
                waveform_bands: deck_data.waveform_bands.clone(),
                loop_start_frac: if duration > 0.0 {
                    deck_data.params.loop_start as f64 / (duration * deck_data.buffer.as_ref().map(|b| b.sample_rate).unwrap_or(44100) as f64)
                } else { 0.0 },
                loop_end_frac: if duration > 0.0 {
                    deck_data.params.loop_end as f64 / (duration * deck_data.buffer.as_ref().map(|b| b.sample_rate).unwrap_or(44100) as f64)
                } else { 1.0 },
                title: deck_data.title.clone(),
                artist: deck_data.artist.clone(),
                key: deck_data.key.clone(),
                genre: deck_data.genre,
            };
        }

        Self {
            decks,
            crossfader: mixer.crossfader,
            crossfader_curve: match mixer.crossfader_curve {
                crate::mixer::CrossfaderCurve::SmoothBlend => 0.0,
                crate::mixer::CrossfaderCurve::SharpCut => 1.0,
            },
            master_volume: mixer.master_volume,
            master_peak: mixer.peak_levels[4],
            recording: mixer.recording,
            fx_slots: [
                (mixer.fx_slots[0].enabled, mixer.fx_slots[0].intensity, mixer.fx_slots[0].preset.clone()),
                (mixer.fx_slots[1].enabled, mixer.fx_slots[1].intensity, mixer.fx_slots[1].preset.clone()),
                (mixer.fx_slots[2].enabled, mixer.fx_slots[2].intensity, mixer.fx_slots[2].preset.clone()),
                (mixer.fx_slots[3].enabled, mixer.fx_slots[3].intensity, mixer.fx_slots[3].preset.clone()),
            ],
            vocal_energy: mixer.bus.vocal_energy,
            harmonic_compat: mixer.bus.harmonic_compat,
            vocal_collision: mixer.bus.vocal_collision,
            groove_lock: mixer.bus.groove_lock,
            master_fft: mixer.master_fft.to_vec(),
            heal_active: mixer.heal.active,
            heal_intensity: mixer.heal.intensity,
            heal_mode: mixer.heal.mode,
            hw_fader_override: mixer.hw_fader_override,
            deck_load_errors: mixer.deck_load_errors.clone(),
            mic_active: mixer.mic.active,
            mic_volume: mixer.mic.volume,
            mic_peak: mixer.mic.peak,
            mic_rms: mixer.mic.rms,
            mic_talkover_db: mixer.mic.talkover_duck_db,
            mic_attached: mixer.mic_input_rx.is_some(),
            mic_auto_duck_enabled: mixer.mic.auto_duck_enabled,
            mic_monitor_enabled: mixer.mic.monitor_enabled,
            headphone_volume: mixer.headphone_volume,
            headphone_blend: mixer.headphone_blend,
            mic_duck_applied_db: mixer.mic.duck_applied_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_default_mixer() {
        let mixer = Mixer::default();
        let snap = MixerSnapshot::capture(&mixer);
        for deck in &snap.decks {
            assert!(!deck.playing);
        }
        assert_eq!(snap.crossfader, 0.5);
    }

    #[test]
    fn capture_reflects_changes() {
        let mut mixer = Mixer::default();
        mixer.decks[0].params.playing = true;
        mixer.decks[2].params.playing = true;
        mixer.crossfader = 0.7;
        let snap = MixerSnapshot::capture(&mixer);
        assert!(snap.decks[0].playing);
        assert!(!snap.decks[1].playing);
        assert!(snap.decks[2].playing);
        assert!(!snap.decks[3].playing);
        assert_eq!(snap.crossfader, 0.7);
    }

    #[test]
    fn snapshot_captures_all_decks() {
        let mut mixer = Mixer::default();
        for (i, deck) in mixer.decks.iter_mut().enumerate() {
            deck.params.volume = 0.25 + i as f32 * 0.1;
            deck.params.tempo = 1.0 + i as f32 * 0.1;
            deck.params.playing = i % 2 == 0;
        }
        let snap = MixerSnapshot::capture(&mixer);
        for (i, deck) in snap.decks.iter().enumerate() {
            assert_eq!(deck.volume, 0.25 + i as f32 * 0.1);
            assert_eq!(deck.tempo, 1.0 + i as f32 * 0.1);
            assert_eq!(deck.playing, i % 2 == 0);
        }
    }
}
