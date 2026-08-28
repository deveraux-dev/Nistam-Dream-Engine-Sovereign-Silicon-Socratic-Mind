//! MixerCommand — the command vocabulary for the audio feeder thread.

use serde::{Deserialize, Serialize};

use super::effect::EffectType;
use super::track::TrackInfo;
use crate::params::{MixerParam, MixerAction};

/// Every command the audio feeder thread understands.
/// All panels speak this same language via `crossbeam_channel::Sender<MixerCommand>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MixerCommand {
    /// Load and play a track on a specific deck.
    Play { deck: usize, track: TrackInfo },
    /// Pause a specific deck (preserves playback position).
    Pause { deck: usize },
    /// Stop a specific deck and reset position to zero.
    Stop { deck: usize },
    /// Set sync mode for a deck ("off", "follower", "leader").
    SetSync { deck: usize, mode: String },
    /// Toggle loop on a deck.
    ToggleLoop { deck: usize },
    /// Set volume for a deck (0.0..=1.0).
    SetVolume { deck: usize, volume: f32 },
    /// Set pan for a deck (-1.0 = left, 1.0 = right, center = 0.0). G-AUDIO-03 wire.
    SetPan { deck: usize, pan: f32 },
    /// Toggle mute on a deck. G-AUDIO-03 wire.
    ToggleMute { deck: usize },
    /// Toggle solo on a deck (solo-defeat: unmutes this deck, mutes others with
    /// no solo active). G-AUDIO-03 wire.
    ToggleSolo { deck: usize },
    /// Set crossfader position (-1.0 = left, 1.0 = right).
    SetCrossfader { position: f32 },
    /// Set master volume (0.0..=1.0).
    SetMasterVolume { volume: f32 },
    /// Seek to position in seconds on a deck.
    Seek { deck: usize, position_secs: f64 },
    /// Apply an effect to a deck.
    ApplyEffect { deck: usize, effect: EffectType },
    /// Remove an effect from a deck by effect id.
    RemoveEffect { deck: usize, effect_id: usize },
    /// Enqueue a track for gapless playback on a deck.
    Enqueue { deck: usize, track: TrackInfo },
    /// Shutdown the audio thread gracefully.
    Shutdown,

    // ── Pass-through variants for forge-audio low-level commands ──────
    // These are forwarded directly to forge-audio's mixer_cmd system
    // by the engine adapter. Panels that need fine-grained mixer control
    // (EQ, per-deck params, actions) use these.

    /// Set a named parameter (e.g. "crossfader", "deck_a_volume", "master_volume").
    Param { target: String, value: f32 },
    /// Trigger a named action (e.g. "deck_a.play_pause", "deck_a.cue").
    Action { target: String },
    /// Set a fully-typed mixer parameter.
    SetParam(MixerParam, f32),
    /// Trigger a fully-typed mixer action.
    SetAction(MixerAction),
    /// Load a decoded PCM buffer directly into a deck (used by library browser).
    #[serde(skip)]
    LoadDeck { deck: String, buffer: crate::dsp::AudioBuffer, title: String, artist: String },
    /// Toggle FX slot on/off.
    ToggleFx { slot: usize },
    /// Set FX slot intensity.
    SetFxIntensity { slot: usize, intensity: f32 },
    /// Set a named preset with intensity.
    SetPreset { name: String, intensity: f32 },
    /// Set FX slot preset from TOML string.
    SetFxSlot { slot: usize, preset: String },
    /// Set a hotcue on a deck (slot 0-7, position in samples).
    SetHotcue { deck: usize, slot: u8, position: usize },
    /// Delete a hotcue on a deck (slot 0-7).
    DeleteHotcue { deck: usize, slot: u8 },
    /// Toggle broadcast live/offline state.
    ToggleBroadcast,
    /// Toggle recording on/off.
    ToggleRecording { output_dir: String },
    /// Toggle mic on/off.
    ToggleMic,
    /// Toggle mic monitor (loopback to headphones).
    ToggleMicMonitor,
    /// Play a one-shot SFX buffer (material-driven procedural synth from SoundQueue).
    #[serde(skip)]
    PlaySfx { buffer: crate::dsp::AudioBuffer },

    // ── Sampler variants (reconciled from ironroot's mixer_cmd usage) ─────

    /// Load audio into a sampler slot (0-7).
    #[serde(skip)]
    LoadSampler { slot: usize, buffer: crate::dsp::AudioBuffer },
    /// Trigger a sampler pad (0-7).
    TriggerSampler { slot: usize },
    /// Stop a sampler pad (0-7).
    StopSampler { slot: usize },
    /// Start recording mic input into a sampler slot (0-7).
    StartRecord { slot: usize },
    /// Stop recording mic input.
    StopRecord { slot: usize },
    /// Capture the master bus for N seconds and flush to a .wav file.
    QuickExport { duration_secs: f32, output_path: String },

    // ── Step Sequencer ───────────────────────────────────────────────────
    /// Start the step sequencer.
    SequencerPlay,
    /// Stop the step sequencer.
    SequencerStop,
    /// Set a step in the sequencer grid. note: -1 = off, 0-127 = MIDI note.
    SequencerSetStep { track: usize, step: usize, note: i8 },
    /// Set a step in the sequencer grid with an explicit MIDI velocity.
    /// Used by the OMR -> sequencer quantizer (OMR-STUDIO-SEQUENCER-BIND-001)
    /// to preserve scanned note dynamics instead of the hardcoded default.
    /// note: -1 = off, 0-127 = MIDI note. velocity: 0-127.
    SequencerSetStepVel { track: usize, step: usize, note: i8, velocity: u8 },
    /// Set sequencer BPM (30-300).
    SequencerSetBpm { bpm: f32 },

    // ── Game Audio (ironroot) ────────────────────────────────────────────
    /// Set ambient weather audio parameters (rain, wind, fog as Permyriad 0-10000).
    AmbientWeather { rain_permyriad: u16, wind_permyriad: u16, fog_permyriad: u16 },
}
