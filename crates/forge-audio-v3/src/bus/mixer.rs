//! MixerCommandHub — stateful UI/Broski-bridge wrapper that applies MixerCommands
//! and exposes a LiveMixerState for the bus engine.
//!
//! Renamed from `Mixer` 2026-05-18 (FORGE-AUDIO-AUDIT-001) to disambiguate from
//! `forge_audio::mixer::Mixer` — the live 4-deck CANONICAL INTERIM mixer used by
//! `dreadpirateradio`. The two share no code; only the former name collided.

use std::collections::VecDeque;
use std::sync::Arc;

use super::command::MixerCommand;
use super::effect::ActiveEffect;
use super::sequencer::Sequencer;
use super::snapshot::{DeckSnapshot, DeckState, LiveMixerState};
use super::track::TrackInfo;

// ★ what-comment → stripped private field docs; @forge:allow_float on bus-state fields
#[derive(Clone, Debug)]
struct Deck {
    state: DeckState,
    track: Option<TrackInfo>,
    position_secs: f64,   // @forge:allow_float — playback position, bus-level state
    duration_secs: f64,   // @forge:allow_float — track length, bus-level state
    volume: f32,          // @forge:allow_float — fader position, bus-level state
    waveform_peak: f32,   // @forge:allow_float — peak meter, bus-level state
    effects: Vec<ActiveEffect>,
    queue: VecDeque<TrackInfo>,
    next_effect_id: usize,
    error_message: Option<String>,
    pan: f32,    // -1.0..1.0, center = 0.0 (G-AUDIO-03 wire)
    muted: bool,
    solo: bool,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            state: DeckState::Empty,
            track: None,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 1.0,
            waveform_peak: 0.0,
            effects: Vec::new(),
            queue: VecDeque::new(),
            next_effect_id: 0,
            error_message: None,
            pan: 0.0,
            muted: false,
            solo: false,
        }
    }
}

impl Deck {
    fn snapshot(&self) -> DeckSnapshot {
        DeckSnapshot {
            track: self.track.clone(),
            state: self.state.clone(),
            position_secs: self.position_secs,
            duration_secs: self.duration_secs,
            volume: self.volume,
            waveform_peak: self.waveform_peak,
            effects: self.effects.clone(),
            error_message: self.error_message.clone(),
            // Silent-mode placeholders: MixerCommandHub has no access to the
            // realtime audio callback where rms/spectral/beat are computed.
            // Only the live `mixer::Mixer` path (via `LiveMixerState::capture`)
            // can populate these; zeros here are intentional, not missing signal.
            rms_level: 0.0,
            spectral_energy: [0.0; 3],
            eq_bands: [0.0; 3],
            beat_phase: 0.0,
            genre: self.track.as_ref().and_then(|t| t.genre),
            waveform_bands: [[0.0; 3]; 200],
            pan: self.pan,
            muted: self.muted,
            solo: self.solo,
        }
    }
}

/// Stateful UI/Broski-bridge command hub owning 4 decks and master controls.
/// Applies `MixerCommand`s and publishes a `LiveMixerState`. Distinct from
/// `forge_audio::mixer::Mixer` (the live 4-deck CANONICAL INTERIM mixer).
pub struct MixerCommandHub {
    decks: [Deck; 4],
    master_volume: f32,
    crossfader: f32,
    frame: u64,
    underrun_count: u64,
    sfx_oneshot: Option<(Vec<f32>, usize)>,
    pub sequencer: Sequencer,
    ambient_weather_warned: bool,
    // ★ hot-alloc → pre-allocated once; cloned cheaply per snapshot
    silent_waveform: Arc<[f32]>,
    silent_spectrum: Arc<[f32]>,
}

impl MixerCommandHub {
    pub fn new() -> Self {
        Self {
            decks: [
                Deck::default(),
                Deck::default(),
                Deck::default(),
                Deck::default(),
            ],
            master_volume: 1.0,
            crossfader: 0.0,
            frame: 0,
            underrun_count: 0,
            sfx_oneshot: None,
            sequencer: Sequencer::new(48000),
            ambient_weather_warned: false,
            silent_waveform: Arc::from(vec![0.0f32; 1024].into_boxed_slice()),
            silent_spectrum: Arc::from(vec![0.0f32; 512].into_boxed_slice()),
        }
    }

    /// Resolve + apply a `.kit.vixi` binding event in one call (G-AUDIO-03 wire) —
    /// the one seam a VixiKit runtime dispatcher calls per widget event. Returns
    /// `false` for unknown/read-only bindings (e.g. `mixer.channel_rms`).
    pub fn apply_kit_binding(&mut self, binding: &str, value_permyriad: i32, deck: usize) -> bool {
        match super::kit_bridge::resolve_kit_binding(binding, value_permyriad, deck) {
            Some(cmd) => { self.apply_command(cmd); true }
            None => false,
        }
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn crossfader(&self) -> f32 {
        self.crossfader
    }

    /// Current BPM from the first playing deck, or 0.0 if none.
    pub fn current_bpm(&self) -> f32 {
        self.decks
            .iter()
            .find(|d| d.state == DeckState::Playing)
            .and_then(|d| d.track.as_ref())
            .and_then(|t| t.bpm)
            .unwrap_or(0.0)
    }

    pub fn any_deck_playing(&self) -> bool {
        self.decks.iter().any(|d| d.state == DeckState::Playing)
    }

    /// Apply a mixer command, validating parameters and updating state.
    ///
    /// Invalid deck indices (>= 4) are silently ignored.
    /// Volume and crossfader values are clamped to valid ranges.
    pub fn apply_command(&mut self, cmd: MixerCommand) {
        match cmd {
            MixerCommand::Play { deck, track } => {
                if deck >= 4 {
                    return;
                }
                let d = &mut self.decks[deck];
                d.error_message = None;
                d.track = Some(track.clone());
                d.duration_secs = track.duration_secs;
                d.position_secs = 0.0;
                d.state = DeckState::Playing;
            }
            MixerCommand::Pause { deck } => {
                if deck >= 4 {
                    return;
                }
                let d = &mut self.decks[deck];
                if d.state == DeckState::Playing {
                    d.state = DeckState::Paused;
                    // position preserved
                }
            }
            MixerCommand::Stop { deck } => {
                if deck >= 4 {
                    return;
                }
                let d = &mut self.decks[deck];
                d.state = DeckState::Stopped;
                d.position_secs = 0.0;
            }
            MixerCommand::SetVolume { deck, volume } => {
                if deck >= 4 {
                    return;
                }
                self.decks[deck].volume = volume.clamp(0.0, 1.0);
            }
            MixerCommand::SetPan { deck, pan } => {
                if deck >= 4 {
                    return;
                }
                self.decks[deck].pan = pan.clamp(-1.0, 1.0);
            }
            MixerCommand::ToggleMute { deck } => {
                if deck >= 4 {
                    return;
                }
                self.decks[deck].muted = !self.decks[deck].muted;
            }
            MixerCommand::ToggleSolo { deck } => {
                if deck >= 4 {
                    return;
                }
                let now_solo = !self.decks[deck].solo;
                self.decks[deck].solo = now_solo;
                if now_solo {
                    // Solo-defeat: unmute the soloed deck, mute every other
                    // deck that isn't itself soloed.
                    self.decks[deck].muted = false;
                    for (i, d) in self.decks.iter_mut().enumerate() {
                        if i != deck && !d.solo {
                            d.muted = true;
                        }
                    }
                } else if self.decks.iter().all(|d| !d.solo) {
                    // Last solo cleared — unmute everyone.
                    for d in self.decks.iter_mut() {
                        d.muted = false;
                    }
                }
            }
            MixerCommand::SetCrossfader { position } => {
                self.crossfader = position.clamp(-1.0, 1.0);
            }
            MixerCommand::SetMasterVolume { volume } => {
                self.master_volume = volume.clamp(0.0, 1.0);
            }
            MixerCommand::Seek { deck, position_secs } => {
                if deck >= 4 {
                    return;
                }
                let d = &mut self.decks[deck];
                d.position_secs = position_secs.max(0.0).min(d.duration_secs);
            }
            MixerCommand::ApplyEffect { deck, effect } => {
                if deck >= 4 {
                    return;
                }
                let d = &mut self.decks[deck];
                let id = d.next_effect_id;
                d.next_effect_id += 1;
                d.effects.push(ActiveEffect {
                    id,
                    effect,
                    enabled: true,
                });
            }
            MixerCommand::RemoveEffect { deck, effect_id } => {
                if deck >= 4 {
                    return;
                }
                self.decks[deck].effects.retain(|e| e.id != effect_id);
            }
            MixerCommand::Enqueue { deck, track } => {
                if deck >= 4 {
                    return;
                }
                self.decks[deck].queue.push_back(track);
            }
            MixerCommand::Shutdown => {
                // Handled by the feeder loop, not the mixer.
            }
            MixerCommand::SetParam(_, _) | MixerCommand::SetAction(_) => {
                // Handled by engine_adapter, not the stub mixer.
            }
            // Pass-through variants — handled by engine_adapter, not the stub mixer.
            MixerCommand::PlaySfx { buffer } => {
                let mono = buffer.to_mono();
                self.sfx_oneshot = Some((mono, 0));
            }
            MixerCommand::Param { .. }
            | MixerCommand::Action { .. }
            | MixerCommand::LoadDeck { .. }
            | MixerCommand::ToggleFx { .. }
            | MixerCommand::SetFxIntensity { .. }
            | MixerCommand::SetPreset { .. }
            | MixerCommand::SetFxSlot { .. }
            | MixerCommand::SetHotcue { .. }
            | MixerCommand::DeleteHotcue { .. }
            | MixerCommand::ToggleBroadcast
            | MixerCommand::ToggleRecording { .. }
            | MixerCommand::ToggleMic
            | MixerCommand::ToggleMicMonitor
            | MixerCommand::LoadSampler { .. }
            | MixerCommand::TriggerSampler { .. }
            | MixerCommand::StopSampler { .. }
            | MixerCommand::SetSync { .. }
            | MixerCommand::ToggleLoop { .. }
            | MixerCommand::StartRecord { .. }
            | MixerCommand::StopRecord { .. }
            | MixerCommand::QuickExport { .. } => {}
            MixerCommand::SequencerPlay => { self.sequencer.start(); }
            MixerCommand::SequencerStop => { self.sequencer.stop(); }
            MixerCommand::SequencerSetStep { track, step, note } => {
                self.sequencer.set_step(track, step, note);
            }
            MixerCommand::SequencerSetStepVel { track, step, note, velocity } => {
                self.sequencer.set_step_vel(track, step, note, velocity);
            }
            MixerCommand::SequencerSetBpm { bpm } => { self.sequencer.set_bpm(bpm); }
            MixerCommand::AmbientWeather { .. } => {
                // Stub path cannot apply weather — no hardware, no real Mixer.
                // Real path (real_feeder_loop → engine_adapter::translate_command) routes
                // this to Mixer::apply_ambient_weather correctly.
                if !self.ambient_weather_warned {
                    self.ambient_weather_warned = true;
                    eprintln!(
                        "[forge-audio::bus] AmbientWeather — stub path drop \
                         (no audio hardware; real path wired via engine_adapter). \
                         Fix: wire weather bridge through forge-game-systems."
                    );
                }
            }
        }
    }

    /// Advance playback positions by `dt` seconds and handle track completion.
    pub fn tick(&mut self, dt: f64) {
        for deck in &mut self.decks {
            if deck.state == DeckState::Playing {
                deck.position_secs += dt;
                if deck.position_secs >= deck.duration_secs {
                    if let Some(next_track) = deck.queue.pop_front() {
                        deck.duration_secs = next_track.duration_secs;
                        deck.track = Some(next_track);
                        deck.position_secs = 0.0;
                    } else {
                        deck.state = DeckState::Stopped;
                        deck.position_secs = 0.0;
                    }
                }
            }
        }
        self.frame += 1;
    }

    pub fn deck_snapshots(&self) -> [DeckSnapshot; 4] {
        [
            self.decks[0].snapshot(),
            self.decks[1].snapshot(),
            self.decks[2].snapshot(),
            self.decks[3].snapshot(),
        ]
    }

    pub fn waveform_buffer(&self) -> Arc<[f32]> {
        Arc::clone(&self.silent_waveform)
    }

    pub fn compute_spectrum(&self) -> Arc<[f32]> {
        Arc::clone(&self.silent_spectrum)
    }

    pub fn record_underrun(&mut self) {
        self.underrun_count += 1;
    }

    pub fn underrun_count(&self) -> u64 {
        self.underrun_count
    }

    pub fn set_deck_error(&mut self, deck: usize, message: String) {
        if deck >= 4 {
            return;
        }
        let d = &mut self.decks[deck];
        d.state = DeckState::Empty;
        d.track = None;
        d.error_message = Some(message);
    }

    pub fn build_snapshot(&self) -> LiveMixerState {
        LiveMixerState {
            decks: self.deck_snapshots(),
            master_volume: self.master_volume,
            crossfader: self.crossfader,
            bpm: self.current_bpm(),
            is_playing: self.any_deck_playing(),
            frame: self.frame,
            waveform_buffer: self.waveform_buffer(),
            spectrum: self.compute_spectrum(),
            timestamp_ns: 0,
            underrun_count: self.underrun_count,
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
            seq_step: if self.sequencer.playing {
                Some(self.sequencer.current_step())
            } else {
                None
            },
            last_mix_us: 0,
        }
    }
}

impl Default for MixerCommandHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::track::AudioFormat;

    fn test_track() -> TrackInfo {
        TrackInfo {
            path: "test.mp3".into(),
            title: "Test".into(),
            artist: "Artist".into(),
            duration_secs: 180.0,
            bpm: Some(120.0),
            key: None,
            format: AudioFormat::Mp3,
            genre: None,
        }
    }

    #[test]
    fn underrun_count_increments() {
        let mut mixer = MixerCommandHub::new();
        assert_eq!(mixer.underrun_count(), 0);
        mixer.record_underrun();
        mixer.record_underrun();
        assert_eq!(mixer.underrun_count(), 2);
        let snap = mixer.build_snapshot();
        assert_eq!(snap.underrun_count, 2);
    }

    #[test]
    fn set_pan_clamps_and_applies() {
        let mut mixer = MixerCommandHub::new();
        mixer.apply_command(MixerCommand::SetPan { deck: 0, pan: 2.0 });
        assert_eq!(mixer.build_snapshot().decks[0].pan, 1.0);
        mixer.apply_command(MixerCommand::SetPan { deck: 0, pan: -0.5 });
        assert_eq!(mixer.build_snapshot().decks[0].pan, -0.5);
    }

    #[test]
    fn toggle_mute_flips_state() {
        let mut mixer = MixerCommandHub::new();
        assert!(!mixer.build_snapshot().decks[0].muted);
        mixer.apply_command(MixerCommand::ToggleMute { deck: 0 });
        assert!(mixer.build_snapshot().decks[0].muted);
        mixer.apply_command(MixerCommand::ToggleMute { deck: 0 });
        assert!(!mixer.build_snapshot().decks[0].muted);
    }

    #[test]
    fn toggle_solo_mutes_other_decks() {
        let mut mixer = MixerCommandHub::new();
        mixer.apply_command(MixerCommand::ToggleSolo { deck: 1 });
        let snap = mixer.build_snapshot();
        assert!(snap.decks[1].solo);
        assert!(!snap.decks[1].muted);
        assert!(snap.decks[0].muted);
        assert!(snap.decks[2].muted);
        assert!(snap.decks[3].muted);
    }

    #[test]
    fn toggle_solo_off_unmutes_everyone() {
        let mut mixer = MixerCommandHub::new();
        mixer.apply_command(MixerCommand::ToggleSolo { deck: 1 });
        mixer.apply_command(MixerCommand::ToggleSolo { deck: 1 });
        let snap = mixer.build_snapshot();
        assert!(!snap.decks[1].solo);
        for d in &snap.decks {
            assert!(!d.muted);
        }
    }

    #[test]
    fn set_deck_error_resets_to_empty() {
        let mut mixer = MixerCommandHub::new();
        mixer.apply_command(MixerCommand::Play {
            deck: 0,
            track: test_track(),
        });
        assert_eq!(mixer.build_snapshot().decks[0].state, DeckState::Playing);

        mixer.set_deck_error(0, "Cannot decode file".into());
        let snap = mixer.build_snapshot();
        assert_eq!(snap.decks[0].state, DeckState::Empty);
        assert!(snap.decks[0].track.is_none());
        assert_eq!(
            snap.decks[0].error_message.as_deref(),
            Some("Cannot decode file")
        );
    }

    #[test]
    fn set_deck_error_does_not_affect_other_decks() {
        let mut mixer = MixerCommandHub::new();
        mixer.apply_command(MixerCommand::Play {
            deck: 0,
            track: test_track(),
        });
        mixer.apply_command(MixerCommand::Play {
            deck: 1,
            track: test_track(),
        });

        mixer.set_deck_error(0, "bad file".into());
        let snap = mixer.build_snapshot();
        assert_eq!(snap.decks[0].state, DeckState::Empty);
        assert_eq!(snap.decks[1].state, DeckState::Playing);
    }

    #[test]
    fn play_clears_previous_error() {
        let mut mixer = MixerCommandHub::new();
        mixer.set_deck_error(0, "old error".into());
        mixer.apply_command(MixerCommand::Play {
            deck: 0,
            track: test_track(),
        });
        let snap = mixer.build_snapshot();
        assert!(snap.decks[0].error_message.is_none());
        assert_eq!(snap.decks[0].state, DeckState::Playing);
    }
}
