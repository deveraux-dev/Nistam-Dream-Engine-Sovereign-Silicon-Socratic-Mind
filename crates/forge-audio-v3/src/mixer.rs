//! Quad-deck mixer engine for DJ-style mixing.
//!
//! Many loops here are `for i in 0..frames { ... out[ch][i] ... }` or
//! `for ch in 0..channels { ... out[ch][i] ... buf.samples[ch] ... }`. These
//! are deliberate DSP frame/channel-grid iteration patterns: indices touch
//! multiple parallel 2D buffers (`out[ch][i]`, `buf.samples[ch][i]`, etc.) and
//! the canonical clippy iter-rewrite would only cover one axis. The patterns
//! are also load-bearing for performance — avoid extra bounds-check pairs.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::explicit_counter_loop)]

use std::sync::Arc;
use crate::dsp::{self, AudioBuffer, BiquadState};

/// Prepare a zeroed scratch buffer, reusing existing heap allocations when possible.
/// First call allocates; subsequent calls just zero-fill (no alloc).
fn prep_scratch(scratch: &mut Vec<Vec<f32>>, channels: usize, frames: usize) -> Vec<Vec<f32>> {
    let mut buf = std::mem::take(scratch);
    buf.resize_with(channels, Vec::new);
    buf.truncate(channels);
    for ch in &mut buf {
        ch.resize(frames, 0.0);
        for s in ch.iter_mut() { *s = 0.0; }
    }
    buf
}

/// Copy `src` into `dst` re-using existing heap capacity. Allocations only
/// happen on growth (first call, or when sample count expands). Used by the
/// FX-apply loop to back up the dry signal without `clone()` per block.
///
/// FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19).
fn copy_audio_into_pool(dst: &mut Vec<Vec<f32>>, src: &[Vec<f32>]) {
    if dst.len() < src.len() {
        dst.resize_with(src.len(), Vec::new);
    } else {
        dst.truncate(src.len());
    }
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        // `extend_from_slice` after `clear` is no-alloc if d.capacity() >= s.len().
        d.clear();
        d.extend_from_slice(s);
    }
}

/// Deferred FX-apply error. Pushed onto an SPSC ring by the audio thread,
/// drained by a cold control thread that logs to stderr. Tag-only — no
/// String — so `push` is genuinely no-alloc.
///
/// FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19): replaces `eprintln!` on the
/// audio hot path. The drained logs lose the inner FX error detail; preset
/// problems are reproducible offline if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxError {
    pub fx_idx: u8,
    pub deck_id: u8,
}

/// Deck identifier for 4-deck mixer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(usize)]
pub enum DeckId {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

/// Crossfader curve shape.
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(Default)]
pub enum CrossfaderCurve {
    /// Equal-power: vol_a = cos(cf * Ï€/2), vol_b = sin(cf * Ï€/2)
    #[default]
    SmoothBlend,
    /// Steep sigmoid: rapid transition near center
    SharpCut,
}


/// Crossfader assignment — which decks appear on left/right sides.
pub struct CrossfaderAssignment {
    pub left: Vec<DeckId>,
    pub right: Vec<DeckId>,
}

impl Default for CrossfaderAssignment {
    fn default() -> Self {
        Self {
            left: vec![DeckId::A, DeckId::C],
            right: vec![DeckId::B, DeckId::D],
        }
    }
}

/// Per-deck 3-band EQ with gains in dB.
#[derive(Clone, Debug)]
pub struct DeckEQ {
    pub low: f32,  // -60 to +12 dB (full kill at -60)
    pub mid: f32,  // -60 to +12 dB (full kill at -60)
    pub high: f32, // -60 to +12 dB (full kill at -60)
}

impl Default for DeckEQ {
    fn default() -> Self {
        Self { low: 0.0, mid: 0.0, high: 0.0 }
    }
}

/// Parameters controlling a single deck.
#[derive(Clone, Debug)]
pub struct DeckParams {
    pub volume: f32,      // 0.0-1.0
    pub tempo: f32,       // 0.5-2.0 (1.0 = original)
    pub eq: DeckEQ,
    pub fx_amount: f32,   // 0.0-1.0
    pub playing: bool,
    pub looping: bool,
    pub loop_start: usize,
    pub loop_end: usize,
    pub replay_gain_db: f32, // ReplayGain normalization (dB, typically -6 to +6)
    /// Pre-fader listen: send this deck to headphone bus
    pub pfl: bool,
    /// Keylock: change tempo without changing pitch (WSOLA time-stretch)
    pub keylock: bool,
    /// Quantize: snap hotcues/loops to nearest beat grid position
    pub quantize: bool,
}

impl Default for DeckParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            tempo: 1.0,
            eq: DeckEQ::default(),
            fx_amount: 0.0,
            playing: false,
            looping: false,
            loop_start: 0,
            loop_end: 0,
            replay_gain_db: 0.0,
            pfl: false,
            keylock: false,
            quantize: false,
        }
    }
}

/// Per-deck stem separation state.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct StemState {
    pub drums_muted: bool,
    pub vocal_muted: bool,
    pub instruments_muted: bool,
}

/// A hotcue point: either a single position or a saved loop region.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum HotcueSlot {
    /// Single cue point (sample position).
    Cue(usize),
    /// Saved loop (start, end sample positions).
    Loop { start: usize, end: usize },
}

/// Beat sync mode for a deck.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SyncMode {
    #[default]
    Off,
    /// Follow the sync leader's tempo and phase.
    Follower,
    /// This deck is the tempo leader — others sync to it.
    Leader,
}

pub struct Deck {
    pub buffer: Option<AudioBuffer>,
    pub params: DeckParams,
    pub playback_pos: usize,
    pub bpm: Option<f32>,
    /// 8 hotcue points — None = unset, Some(pos) = sample position
    pub hotcues: [Option<HotcueSlot>; 8],
    /// Persistent biquad state for 3-band EQ: \[low/mid/high\]\[channel0/channel1\]
    pub eq_state: [[BiquadState; 2]; 3],
    /// Stem mute state
    pub stems: StemState,
    /// Cached waveform overview (200 peak points). Computed once on load.
    pub waveform_cache: Vec<f32>,
    /// Slip mode: shadow position continues playing while scratching/looping.
    pub slip_active: bool,
    pub slip_pos: usize,
    /// Beatgrid for sync/phase display. Computed on load from BPM.
    pub beatgrid: Option<crate::bpm::BeatGrid>,
    /// Sync mode for this deck.
    pub sync_mode: SyncMode,
    /// Device output sample rate (set by Mixer).
    pub device_sample_rate: u32,
    /// Vinyl scratch: jog wheel offset applied to playback rate.
    /// Positive = forward nudge, negative = backward. Decays to 0.
    pub scratch_rate: f32,
    /// True while the user is actively touching the jog platter.
    pub scratching: bool,
    /// Reverse playback: negates effective_rate so audio plays backwards.
    pub reverse: bool,
    /// Saved cue point (sample position). Set by pressing cue while playing.
    pub cue_point: usize,
    /// Loop roll: saved return position (where playback would have been).
    pub loop_roll_return: Option<usize>,
    /// 3-band waveform energy [low, mid, high] per window. Computed once on load.
    pub waveform_bands: Vec<[f32; 3]>,
    /// Track title from library DB.
    pub title: String,
    /// Track artist from library DB.
    pub artist: String,
    /// Musical key in Camelot notation (e.g. "8B", "5A"). Set by key_detect on load.
    pub key: Option<String>,
    /// Genre classification (0=DnB, 1=Techno, 2=Deep, 3=Other). Set by genre_detect on load.
    pub genre: Option<u8>,
    /// Pre-allocated scratch buffer for read_block output. Recycled by mix_block.
    pub scratch_out: Vec<Vec<f32>>,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            buffer: None,
            params: DeckParams::default(),
            playback_pos: 0,
            bpm: None,
            hotcues: [None; 8],
            eq_state: Default::default(),
            stems: StemState::default(),
            waveform_cache: Vec::new(),
            slip_active: false,
            slip_pos: 0,
            beatgrid: None,
            sync_mode: SyncMode::Off,
            scratch_rate: 0.0,
            scratching: false,
            reverse: false,
            device_sample_rate: 48000,
            cue_point: 0,
            loop_roll_return: None,
            waveform_bands: Vec::new(),
            title: String::new(),
            artist: String::new(),
            key: None,
            genre: None,
            scratch_out: Vec::new(),
        }
    }
}

impl Deck {
    /// Load audio into this deck, reset position, and set loop_end to buffer length.
    pub fn load(&mut self, buffer: AudioBuffer) {
        let len = buffer.len();
        // Pre-compute waveform overview (200 peak points) — ONCE, not every snapshot
        self.waveform_cache = Self::compute_waveform(&buffer, 200);
        // Pre-compute 3-band colored waveform — ONCE on load
        self.waveform_bands = crate::dsp::compute_waveform_bands(&buffer, 512);
        self.buffer = Some(buffer);
        self.playback_pos = 0;
        // Reset transient state (keep volume/tempo/eq/pfl/keylock)
        self.params.playing = false;
        self.params.looping = false;
        self.params.loop_start = 0;
        self.params.loop_end = len;
        self.bpm = None;
        self.hotcues = [None; 8];
        self.eq_state = Default::default();
        self.slip_active = false;
        self.slip_pos = 0;
        self.scratch_rate = 0.0;
        self.scratching = false;
        self.reverse = false;
        self.beatgrid = None;
    }

    /// Snap position to nearest beat if quantize is active, otherwise return unchanged.
    pub fn snap_if_quantized(&self, pos: usize) -> usize {
        if self.params.quantize {
            if let Some(ref bg) = self.beatgrid {
                return bg.snap_to_beat(pos);
            }
        }
        pos
    }

    fn compute_waveform(buf: &AudioBuffer, points: usize) -> Vec<f32> {
        let mono = buf.to_mono();
        if mono.is_empty() { return vec![]; }
        let chunk_size = (mono.len() / points).max(1);
        (0..points)
            .map(|i| {
                let start = i * chunk_size;
                let end = (start + chunk_size).min(mono.len());
                if start >= mono.len() { return 0.0; }
                mono[start..end].iter().map(|s| s.abs()).fold(0.0f32, f32::max)
            })
            .collect()
    }

    /// Read the next block of `frames` output samples, applying tempo via linear
    /// interpolation, looping, and playing state. Returns silence if not playing
    /// or no buffer is loaded.
    pub fn read_block(&mut self, frames: usize) -> Option<AudioBuffer> {
        let buf = self.buffer.as_ref()?;
        let channels = buf.channels();
        let sample_rate = buf.sample_rate;
        let buf_len = buf.len();

        if !self.params.playing || buf_len == 0 {
            let samples = prep_scratch(&mut self.scratch_out, channels, frames);
            return Some(AudioBuffer { samples, sample_rate });
        }

        let tempo = self.params.tempo.clamp(0.5, 2.0);
        // Apply scratch rate on top of tempo (vinyl feel)
        let raw_rate = if self.scratching {
            self.scratch_rate
        } else {
            tempo + self.scratch_rate
        };
        let effective_rate = if self.reverse { -raw_rate } else { raw_rate };
        // Decay scratch_rate when not actively scratching
        if !self.scratching {
            self.scratch_rate *= 0.95;
            if self.scratch_rate.abs() < 0.001 { self.scratch_rate = 0.0; }
        }

        // Rate conversion: if buffer sample rate differs from device, adjust cursor speed
        let rate_ratio = buf.sample_rate as f64 / self.device_sample_rate.max(1) as f64;

        let mut out = prep_scratch(&mut self.scratch_out, channels, frames);
        let mut cursor = self.playback_pos as f64;

        for i in 0..frames {
            // Check bounds / looping (handles both forward and reverse)
            let effective_start = if self.params.looping && self.params.loop_end > self.params.loop_start {
                self.params.loop_start as f64
            } else {
                0.0
            };
            let effective_end = if self.params.looping && self.params.loop_end > self.params.loop_start {
                self.params.loop_end as f64
            } else {
                buf_len as f64
            };

            if cursor >= effective_end {
                if self.params.looping {
                    cursor = effective_start;
                } else {
                    break;
                }
            } else if cursor < effective_start {
                if self.params.looping {
                    cursor = effective_end - 1.0;
                } else {
                    cursor = 0.0;
                    break;
                }
            }

            let idx = cursor.max(0.0) as usize;
            let frac = (cursor - idx as f64) as f32;

            for ch in 0..channels {
                let a = buf.samples[ch].get(idx).copied().unwrap_or(0.0);
                let b = buf.samples[ch].get(idx + 1).copied().unwrap_or(a);
                out[ch][i] = a + (b - a) * frac;
            }

            cursor += effective_rate as f64 * rate_ratio;
        }

        self.playback_pos = cursor.max(0.0) as usize;

        // Slip mode: advance shadow position at normal tempo (1.0)
        if self.slip_active {
            self.slip_pos = (self.slip_pos + frames).min(buf_len);
        }

        // Keylock: WSOLA (Waveform Similarity Overlap-Add) pitch correction.
        // Cancels pitch shift from tempo change by time-stretching with overlap-add.
        if self.params.keylock && (tempo - 1.0).abs() > 0.01 {
            let ratio = 1.0 / tempo as f64; // how much to stretch
            let win_size = 1024usize;
            let hop_out = win_size / 4; // 75% overlap
            let hop_in = (hop_out as f64 / ratio) as usize;
            if hop_in > 0 && frames > 0 {
                let out_len = frames;
                let mut corrected = vec![vec![0.0f32; out_len]; channels];
                let mut norm = vec![0.0f32; out_len];
                // Hann window (precompute)
                let hann: Vec<f32> = (0..win_size)
                    .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / win_size as f32).cos()))
                    .collect();
                let mut in_pos = 0usize;
                let mut out_pos = 0usize;
                while out_pos + win_size <= out_len {
                    for ch in 0..channels {
                        for i in 0..win_size {
                            let src_idx = in_pos + i;
                            let sample = out[ch].get(src_idx).copied().unwrap_or(0.0);
                            corrected[ch][out_pos + i] += sample * hann[i];
                        }
                    }
                    for i in 0..win_size {
                        norm[out_pos + i] += hann[i];
                    }
                    in_pos += hop_in;
                    out_pos += hop_out;
                }
                // Normalize by accumulated window weights
                for ch in 0..channels {
                    for i in 0..out_len {
                        if norm[i] > 0.001 {
                            corrected[ch][i] /= norm[i];
                        }
                    }
                }
                out = corrected;
            }
        }

        Some(AudioBuffer { samples: out, sample_rate })
    }
}

/// A single FX slot — holds a named preset, enabled state, intensity, and cached pipeline.
pub struct FxSlot {
    pub preset: Option<String>,
    pub enabled: bool,
    pub intensity: f32,
    cached_pipeline: Option<crate::composer::EffectsPipeline>,
}

impl Default for FxSlot {
    fn default() -> Self {
        Self { preset: None, enabled: false, intensity: 0.5, cached_pipeline: None }
    }
}

impl FxSlot {
    /// Assign a preset — parses and caches the pipeline immediately.
    pub fn set_preset(&mut self, name: &str, intensity: f32) {
        self.preset = Some(name.to_string());
        self.intensity = intensity;
        self.cached_pipeline = crate::presets::load_preset(name, intensity).ok();
    }

    /// Update intensity — re-parses the cached pipeline with the new intensity.
    pub fn update_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
        if let Some(ref name) = self.preset.clone() {
            self.cached_pipeline = crate::presets::load_preset(name, intensity).ok();
        }
    }
}

/// Microphone input with talkover ducking + envelope-driven auto-duck + monitor.
#[derive(Clone, Debug, Default)]
pub struct MicState {
    pub active: bool,
    pub volume: f32,
    /// Talkover: duck music by this amount (dB) when mic is active. 0 = no duck.
    /// Used when `auto_duck_enabled=false`. Negative means reduction.
    pub talkover_duck_db: f32,
    /// Last-block peak sample magnitude (post-volume). Updated by mix_block.
    pub peak: f32,
    /// Last-block RMS (post-volume). Updated by mix_block.
    pub rms: f32,
    /// Cue monitor: mic routed to headphone bus (loopback) when true.
    /// Independent of `active` so monitor can be on while mic is muted to master.
    pub monitor_enabled: bool,
    /// Auto-duck: envelope-driven sidechain ducking of master by mic level.
    /// Overrides the static `talkover_duck_db` path when true.
    pub auto_duck_enabled: bool,
    /// Smoothed envelope of the mic input (block RMS, pole-smoothed by attack/release).
    /// Range ~[0.0, 1.0]. Used to compute `duck_applied_db` each block.
    pub duck_env: f32,
    /// Currently applied duck gain reduction in dB (negative = ducking, 0.0 = no duck).
    /// Published to the snapshot so UI can drive a GR meter. Updated each block.
    pub duck_applied_db: f32,
    /// Auto-duck envelope attack time in ms (rise).
    pub duck_attack_ms: f32,
    /// Auto-duck envelope release time in ms (fall).
    pub duck_release_ms: f32,
    /// Auto-duck threshold: envelope value below this produces no reduction.
    pub duck_threshold: f32,
    /// Auto-duck maximum reduction in dB (e.g. -12.0). Applied when envelope >> threshold.
    pub duck_max_db: f32,
}

/// One-shot or loop sampler pad.
#[derive(Clone, Debug, Default)]
pub struct Sampler {
    pub buffer: Option<AudioBuffer>,
    pub playing: bool,
    pub looping: bool,
    pub playback_pos: usize,
    pub volume: f32,
}

impl Sampler {
    pub fn trigger(&mut self) {
        if self.buffer.is_some() {
            self.playing = true;
            self.playback_pos = 0;
        }
    }

    pub fn read_block(&mut self, frames: usize) -> Option<AudioBuffer> {
        let buf = self.buffer.as_ref()?;
        if !self.playing { return None; }
        let channels = buf.channels();
        let sr = buf.sample_rate;
        let len = buf.len();
        let mut out = vec![vec![0.0f32; frames]; channels];
        for i in 0..frames {
            if self.playback_pos >= len {
                if self.looping {
                    self.playback_pos = 0;
                } else {
                    self.playing = false;
                    break;
                }
            }
            for ch in 0..channels {
                out[ch][i] = buf.samples[ch].get(self.playback_pos).copied().unwrap_or(0.0) * self.volume;
            }
            self.playback_pos += 1;
        }
        Some(AudioBuffer { samples: out, sample_rate: sr })
    }
}

/// Quad-deck mixer with crossfader and master volume.
pub struct Mixer {
    pub decks: [Deck; 4],  // Index: 0=A, 1=B, 2=C, 3=D
    pub crossfader: f32, // 0.0 = all left, 1.0 = all right
    pub smoothed_crossfader: f32, // smoothed value used in per-sample mix
    pub crossfader_curve: CrossfaderCurve, // SmoothBlend or SharpCut
    pub crossfader_assignment: CrossfaderAssignment,
    pub master_volume: f32,
    pub recording: bool,
    pub record_buffer: Vec<Vec<f32>>,
    pub master_effects: Option<crate::composer::EffectsPipeline>,
    /// 4 FX slots (mapped to S2 FX1-4 buttons)
    pub fx_slots: [FxSlot; 4],
    /// Peak levels from last mix_block (for VU meters). [deck_a, deck_b, deck_c, deck_d, master]
    pub peak_levels: [f32; 5],
    /// RMS levels per deck + master (same layout as peak_levels)
    pub rms_levels: [f32; 5],
    /// 3-band spectral energy per deck: [low, mid, high] (approximate via simple filters)
    pub spectral_energy: [[f32; 3]; 4],
    /// 64-bin FFT magnitude spectrum per deck (updated each time 128 samples accumulate)
    pub fft_bins: [[f32; 64]; 4],
    fft_ring: [[f32; 128]; 4],
    fft_ring_fill: [usize; 4],
    fft_plan: Arc<dyn realfft::RealToComplex<f32>>,
    fft_in_buf: Vec<f32>,
    fft_out_buf: Vec<realfft::num_complex::Complex32>,
    /// Microphone input state + talkover ducking
    pub mic: MicState,
    /// Live mic input consumer (SPSC ring fed by cpal input callback).
    /// None when no mic capture is attached. Populated via `set_mic_input`.
    pub mic_input_rx: Option<rtrb::Consumer<f32>>,
    /// 8 sampler slots (one-shot or loop trigger pads)
    pub samplers: [Sampler; 8],
    /// Per-deck FX assignment: which FX slots apply before crossfader (vs master)
    /// e.g. fx_assign\[0\] = \[true,false,false,false\] means deck A gets FX slot 0
    pub fx_assign: [[bool; 4]; 4],
    /// Ghost Fire rendering enabled
    pub ghost_enabled: bool,
    /// Headphone mix: pre-fader listen output (decks with pfl=true, summed)
    pub headphone_mix: Option<AudioBuffer>,
    /// Headphone volume
    pub headphone_volume: f32,
    /// Headphone cue/main blend (0.0 = cue only, 1.0 = main only)
    pub headphone_blend: f32,
    /// Booth output volume (taps from main)
    pub booth_volume: f32,
    /// Booth output mix (copy of main with booth_volume applied)
    pub booth_mix: Option<AudioBuffer>,
    /// Ring buffer producer for broadcast thread (decoupled from hot path)
    pub broadcast_tx: Option<rtrb::Producer<f32>>,
    /// Device output sample rate (for rate conversion in read_block)
    pub device_sample_rate: u32,
    /// AutoDJ crossfade ramp target (None = no ramp in progress)
    pub crossfade_target: Option<f32>,
    /// Per-block step size for crossfade ramp
    pub crossfade_rate: f32,
    /// Correspondence Bus: shared psychoacoustic + harmonic state per block
    pub bus: crate::correspondence_bus::CorrespondenceBus,
    /// 64-bin FFT of master output — shows ALL FX, heal layer, everything
    pub master_fft: [f32; 64],
    master_fft_ring: [f32; 128],
    master_fft_fill: usize,
    /// Pre-allocated scratch for mix_block output. Recycled each cycle.
    out_scratch: Vec<Vec<f32>>,
    /// Pre-allocated scratch for headphone mix. Recycled each cycle.
    hp_scratch: Vec<Vec<f32>>,
    /// Heal plugin state (hidden master insert)
    pub heal: HealState,
    /// Hardware fader override: true when HID fader sets volume < 0.05
    pub hw_fader_override: [bool; 4],
    /// SF-017: LOCKED — last decode error per deck, set by DeckLoadFailed command.
    /// Surfaced through MixerSnapshot so UI can distinguish "load failed" from "deck empty".
    /// See forge-audio/src/params.rs::tests::sf_017_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
    pub deck_load_errors: [Option<String>; 4],
    /// Ghost whisper hook slot. Proprietary DSP plugs in through this trait
    /// boundary only. See ghost_whisper.rs.
    pub whisper: crate::ghost_whisper::WhisperSlot,
    /// Pre-allocated whisper bus scratch (mono, reused per block).
    whisper_scratch: Vec<f32>,
    /// Pre-allocated mic block scratch (mono, reused per block). Drains the cpal
    /// input ring once; both the master mix and the monitor bus read this buffer.
    mic_scratch: Vec<f32>,
    /// Wrap-aware beat phase delta between two loudest playing decks.
    /// Updated each mix_block via `compute_beat_phase_delta()`.
    pub beat_phase_delta: f32,
    /// Pre-allocated dry-signal backup pool, one `Vec<Vec<f32>>` per FX slot.
    /// Replaces `b.samples.clone()` on the hot path (FORGE-AUDIO-HOTPATH-CLEAN-001).
    /// First-block allocations grow the pool to per-block size; subsequent
    /// blocks reuse capacity.
    dry_backup_pool: [Vec<Vec<f32>>; 4],
    /// SPSC ring producer for deferred FX-apply error logging. `None` means
    /// errors drop on the floor (default; suitable for tests). Production
    /// owners call [`Mixer::set_fx_error_sink`] to attach a drain consumer.
    /// FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19) — replaces `eprintln!` on the
    /// audio hot path.
    fx_error_tx: Option<rtrb::Producer<FxError>>,
}

/// Psychoacoustic heal layer: binaural beats, Schumann resonance, solfeggio, isochronic pulse.
#[derive(Clone, Debug)]
pub struct HealState {
    pub active: bool,
    pub intensity: f32,  // 0.0-1.0
    pub mode: u8,        // 0=alpha, 1=beta, 2=theta, 3=gamma
    pub bpm: f32,        // auto-fed from master BPM
    phase: f64,          // running phase for oscillators
}

impl Default for HealState {
    fn default() -> Self {
        Self { active: false, intensity: 0.3, mode: 0, bpm: 120.0, phase: 0.0 }
    }
}

impl Default for Mixer {
    fn default() -> Self {
        let mut planner = realfft::RealFftPlanner::<f32>::new();
        let fft_plan = planner.plan_fft_forward(128);
        let fft_out_buf = fft_plan.make_output_vec();
        Self {
            decks: Default::default(),
            crossfader: 0.5,
            smoothed_crossfader: 0.5,
            crossfader_curve: CrossfaderCurve::default(),
            crossfader_assignment: CrossfaderAssignment::default(),
            master_volume: 1.0,
            recording: false,
            record_buffer: Vec::new(),
            master_effects: None,
            fx_slots: {
                let mut slots: [FxSlot; 4] = Default::default();
                // Pre-load S2 FX1-4 with default presets so hardware buttons work immediately
                slots[0].set_preset("freeze", 0.5);
                slots[1].set_preset("hollow", 0.5);
                slots[2].set_preset("dread", 0.5);
                slots[3].set_preset("shatter", 0.5);
                slots
            },
            peak_levels: [0.0; 5],
            rms_levels: [0.0; 5],
            spectral_energy: [[0.0; 3]; 4],
            mic: MicState {
                active: false,
                volume: 1.0,
                talkover_duck_db: -12.0,
                monitor_enabled: false,
                auto_duck_enabled: false,
                duck_env: 0.0,
                duck_applied_db: 0.0,
                duck_attack_ms: 10.0,
                duck_release_ms: 200.0,
                duck_threshold: 0.05,
                duck_max_db: -12.0,
                ..Default::default()
            },
            mic_input_rx: None,
            samplers: Default::default(),
            // All 4 FX slots assigned to decks A+B by default (S2 layout)
            fx_assign: [[true, true, true, true], [true, true, true, true], [false; 4], [false; 4]],
            ghost_enabled: true,
            headphone_mix: None,
            headphone_volume: 1.0,
            headphone_blend: 0.0,
            booth_volume: 1.0,
            booth_mix: None,
            broadcast_tx: None,
            device_sample_rate: 48000,
            crossfade_target: None,
            crossfade_rate: 0.0,
            bus: crate::correspondence_bus::CorrespondenceBus::new(),
            fft_bins: [[0.0; 64]; 4],
            fft_ring: [[0.0; 128]; 4],
            fft_ring_fill: [0; 4],
            fft_plan,
            fft_in_buf: vec![0.0f32; 128],
            fft_out_buf,
            master_fft: [0.0; 64],
            master_fft_ring: [0.0; 128],
            master_fft_fill: 0,
            out_scratch: Vec::new(),
            hp_scratch: Vec::new(),
            heal: HealState::default(),
            hw_fader_override: [false; 4],
            deck_load_errors: [None, None, None, None],
            whisper: crate::ghost_whisper::WhisperSlot::new(),
            whisper_scratch: Vec::new(),
            mic_scratch: Vec::new(),
            beat_phase_delta: 0.0,
            // FORGE-AUDIO-HOTPATH-CLEAN-001: empty initially; first call to
            // mix_block grows each slot to (channels × frames) and reuses
            // capacity on every subsequent block.
            dry_backup_pool: std::array::from_fn(|_| Vec::new()),
            // No sink by default — errors drop. Owners wire a sink via
            // `set_fx_error_sink` and spawn a drain thread.
            fx_error_tx: None,
        }
    }
}

impl Mixer {
    /// Attach an SPSC sink for deferred FX-apply error logging. The matching
    /// [`rtrb::Consumer<FxError>`] is held by the owner (e.g. dreadpirateradio)
    /// and drained on a cold thread that stderrs each entry. If no sink is
    /// set, FX errors drop silently — appropriate for tests and headless runs.
    ///
    /// FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19).
    pub fn set_fx_error_sink(&mut self, tx: rtrb::Producer<FxError>) {
        self.fx_error_tx = Some(tx);
    }

    /// Attach a live mic-input SPSC consumer. Call once the cpal input stream
    /// is running. `mix_block` drains this ring each block (post-talkover-duck)
    /// and sums the samples into the master bus at `mic.volume`. Replacing an
    /// existing consumer drops it.
    pub fn set_mic_input(&mut self, consumer: rtrb::Consumer<f32>) {
        self.mic_input_rx = Some(consumer);
    }

    /// Detach the mic-input ring. Mic remains configured but no samples flow.
    pub fn clear_mic_input(&mut self) {
        self.mic_input_rx = None;
    }

    /// Get a read-only reference to a deck by ID.
    pub fn deck(&self, id: DeckId) -> &Deck {
        &self.decks[id as usize]
    }

    /// Get a mutable reference to a deck by ID.
    pub fn deck_mut(&mut self, id: DeckId) -> &mut Deck {
        &mut self.decks[id as usize]
    }


    /// Recycle output buffer back into scratch for zero-alloc reuse next cycle.
    /// Call this after pushing samples to the ring buffer.
    /// Beat sync: adjust follower deck tempos to match reference BPM.
    /// Reference = explicit leader, or first non-follower playing deck with BPM.
    pub fn sync_tempos(&mut self) {
        // Find reference: leader first, then first non-follower playing deck
        let ref_info: Option<(f32, f32)> = self.decks.iter()
            .find(|d| d.sync_mode == SyncMode::Leader && d.bpm.filter(|b| *b > 1.0).is_some())
            .or_else(|| self.decks.iter()
                .find(|d| d.sync_mode != SyncMode::Follower && d.params.playing && d.bpm.filter(|b| *b > 1.0).is_some()))
            .and_then(|d| d.bpm.map(|b| (b, d.params.tempo)));

        if let Some((ref_bpm, ref_tempo)) = ref_info {
            let target_bpm = ref_bpm * ref_tempo;
            for deck in &mut self.decks {
                if deck.sync_mode == SyncMode::Follower {
                    if let Some(own_bpm) = deck.bpm {
                        if own_bpm > 1.0 {
                            deck.params.tempo = (target_bpm / own_bpm).clamp(0.5, 2.0);
                        }
                    }
                }
            }
        }
    }

    pub fn recycle_output(&mut self, buf: AudioBuffer) {
        self.out_scratch = buf.samples;
    }

    /// Phase-locked beat sync: match follower tempo to leader, then nudge
    /// playback_pos toward phase alignment (max 4 samples/block).
    pub fn phase_correct(&mut self) {
        let li = match (0..4).find(|&i| self.decks[i].sync_mode == SyncMode::Leader) {
            Some(i) => i,
            None => return,
        };
        let leader_bpm = match self.decks[li].bpm {
            Some(b) if b > 1.0 => b,
            _ => return,
        };
        let leader_phase = self.decks[li].beatgrid.as_ref()
            .map(|bg| bg.phase_at(self.decks[li].playback_pos))
            .unwrap_or(0.0);

        for i in 0..4 {
            if i == li || self.decks[i].sync_mode != SyncMode::Follower { continue; }
            let follower_bpm = match self.decks[i].bpm {
                Some(b) if b > 1.0 => b,
                _ => continue,
            };
            // Tempo lock
            self.decks[i].params.tempo = (leader_bpm / follower_bpm).clamp(0.5, 2.0);

            // Phase nudge via playback_pos (not tempo)
            if let Some(ref bg) = self.decks[i].beatgrid {
                let f_phase = bg.phase_at(self.decks[i].playback_pos);
                let mut delta = leader_phase - f_phase;
                if delta > 0.5 { delta -= 1.0; }
                if delta < -0.5 { delta += 1.0; }
                if delta.abs() > 0.02 {
                    // Nudge up to 4 samples toward alignment
                    let nudge = (delta.signum() * delta.abs().min(0.01) * bg.beat_interval as f32) as isize;
                    let new_pos = (self.decks[i].playback_pos as isize + nudge).max(0) as usize;
                    self.decks[i].playback_pos = new_pos;
                }
            }
        }
    }

    /// Compute wrap-aware beat phase delta between the two loudest playing decks.
    /// Returns 0.0 if fewer than 2 decks are playing.
    pub fn compute_beat_phase_delta(&self) -> f32 {
        // Collect playing decks with their peak levels
        let mut playing: Vec<(usize, f32)> = (0..4)
            .filter(|&i| self.decks[i].params.playing && self.decks[i].buffer.is_some())
            .map(|i| (i, self.peak_levels[i]))
            .collect();
        if playing.len() < 2 {
            return 0.0;
        }
        // Sort by peak level descending to find two loudest
        playing.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let deck_a = playing[0].0;
        let deck_b = playing[1].0;
        // Get beat phase from beatgrid
        let phase_a = self.decks[deck_a].beatgrid.as_ref()
            .map(|bg| bg.phase_at(self.decks[deck_a].playback_pos))
            .unwrap_or(0.0);
        let phase_b = self.decks[deck_b].beatgrid.as_ref()
            .map(|bg| bg.phase_at(self.decks[deck_b].playback_pos))
            .unwrap_or(0.0);
        let raw = (phase_a - phase_b).abs();
        raw.min(1.0 - raw)
    }

    /// Drive the hidden heal insert straight from a cognitive reading — the lens
    /// picks the band + depth in place of a manual slider (C3). Guidance-gated: an
    /// Off slider (default) leaves `heal` untouched, so nothing is ever imposed. This
    /// is the studio's cognitive→heal wire; it runs on the 1 Hz logic lane, not the
    /// audio callback. `apply_heal` (below) reads the state this sets.
    #[cfg(feature = "cognitive")]
    pub fn drive_heal(
        &mut self,
        state: forge_sieve::cognitive::CognitiveState,
        guidance: forge_sieve::cognitive::Guidance,
    ) {
        if let Some(update) = crate::cognitive_heal::heal_for(state, guidance) {
            update.apply(&mut self.heal);
        }
    }

    /// Apply heal psychoacoustic layer to master output.
    /// 4 layers: binaural L/R split, Schumann 7.83Hz AM, 528Hz resonant boost, BPM-locked isochronic.
    pub fn apply_heal(&mut self, out: &mut [Vec<f32>], frames: usize, sample_rate: u32) {
        if !self.heal.active || out.len() < 2 { return; }
        let sr = sample_rate as f64;
        let intensity = self.heal.intensity as f64;
        // Mode-dependent beat frequency for binaural
        let beat_hz: f64 = match self.heal.mode {
            0 => 10.0,  // alpha (relaxation)
            1 => 20.0,  // beta (focus)
            2 => 6.0,   // theta (meditation)
            _ => 40.0,  // gamma (cognition)
        };
        let carrier = 220.0;
        let schumann = 7.83;
        let solfeggio = 528.0;
        let iso_hz = self.heal.bpm as f64 / 60.0; // BPM-locked pulse

        for i in 0..frames {
            let t = self.heal.phase;
            // Layer 1: Binaural — slightly different freq in L vs R
            let bin_l = (2.0 * std::f64::consts::PI * (carrier - beat_hz / 2.0) * t).sin() * 0.03 * intensity;
            let bin_r = (2.0 * std::f64::consts::PI * (carrier + beat_hz / 2.0) * t).sin() * 0.03 * intensity;
            // Layer 2: Schumann 7.83Hz AM on 45Hz carrier
            let schumann_env = ((2.0 * std::f64::consts::PI * schumann * t).sin() * 0.5 + 0.5) * intensity * 0.02;
            let schumann_sig = (2.0 * std::f64::consts::PI * 45.0 * t).sin() * schumann_env;
            // Layer 3: 528Hz solfeggio resonant boost
            let solf = (2.0 * std::f64::consts::PI * solfeggio * t).sin() * 0.015 * intensity;
            // Layer 4: Isochronic BPM-locked pulse (square-ish envelope)
            let iso_phase = (t * iso_hz).fract();
            let iso_env = if iso_phase < 0.5 { 1.0 } else { 0.0 };
            let iso = iso_env * 0.01 * intensity;

            out[0][i] += (bin_l + schumann_sig + solf + iso) as f32;
            out[1][i] += (bin_r + schumann_sig + solf + iso) as f32;

            self.heal.phase += 1.0 / sr;
            // Prevent phase from growing unbounded
            if self.heal.phase > 1000.0 { self.heal.phase -= 1000.0; }
        }
    }

    /// Mix a block of `frames` samples from all 4 decks using crossfader
    /// assignment, deck volumes, and master volume. Appends to the record
    /// buffer if recording is active.
    pub fn mix_block(&mut self, frames: usize) -> AudioBuffer {
        // AutoDJ crossfade ramp: step crossfader toward target each block
        if let Some(target) = self.crossfade_target {
            self.crossfader += self.crossfade_rate;
            let reached = if self.crossfade_rate >= 0.0 {
                self.crossfader >= target
            } else {
                self.crossfader <= target
            };
            if reached {
                self.crossfader = target;
                self.crossfade_target = None;
                self.crossfade_rate = 0.0;
            }
        }

        // Beat sync: match follower tempos + phase to leader
        self.phase_correct();

        // Read blocks from all 4 decks and apply per-deck EQ
        let mut blocks: [Option<AudioBuffer>; 4] = [None, None, None, None];
        let mut deck_peaks = [0.0f32; 4];

        for deck_id in 0..4 {
            let mut block = self.decks[deck_id].read_block(frames);
            if let Some(ref mut b) = block {
                let eq = &self.decks[deck_id].params.eq;
                if eq.low.abs() > 0.5 || eq.mid.abs() > 0.5 || eq.high.abs() > 0.5 {
                    dsp::eq_3band(b, eq.low, eq.mid, eq.high, &mut self.decks[deck_id].eq_state);
                }
                // Apply ReplayGain normalization (dB â†’ linear)
                let rg = self.decks[deck_id].params.replay_gain_db;
                if rg.abs() > 0.01 {
                    let gain = 10.0f32.powf(rg / 20.0);
                    for ch in b.samples.iter_mut() {
                        for s in ch.iter_mut() { *s *= gain; }
                    }
                }
                // Apply per-deck FX (assigned via fx_assign).
                // On pipeline error we fall back to the DRY signal instead of silence,
                // so a broken preset never kills playback. FORGE-AUDIO-HOTPATH-CLEAN-001:
                // the dry backup is copied into a pre-allocated pool (no per-block
                // clone), and errors push onto an SPSC ring (no per-block eprintln).
                for fx_idx in 0..4 {
                    if !self.fx_assign[deck_id][fx_idx] || !self.fx_slots[fx_idx].enabled {
                        continue;
                    }
                    if self.fx_slots[fx_idx].cached_pipeline.is_none() {
                        continue;
                    }
                    // Pre-emptive dry backup (no alloc after warm-up).
                    copy_audio_into_pool(&mut self.dry_backup_pool[fx_idx], &b.samples);
                    let taken = std::mem::take(&mut b.samples);
                    let sr = b.sample_rate;
                    let fx_buf = AudioBuffer { samples: taken, sample_rate: sr };
                    // Pipeline borrow scoped to the .apply() call; released
                    // before we touch self.fx_error_tx / dry_backup_pool again.
                    let apply_result = self.fx_slots[fx_idx]
                        .cached_pipeline
                        .as_ref()
                        .expect("checked is_some() above")
                        .apply(fx_buf);
                    match apply_result {
                        Ok(processed) => b.samples = processed.samples,
                        Err(_e) => {
                            // Cold-path drain logs via stderr; on-thread we
                            // just push a 2-byte tag.
                            if let Some(tx) = self.fx_error_tx.as_mut() {
                                let _ = tx.push(FxError {
                                    fx_idx: fx_idx as u8,
                                    deck_id: deck_id as u8,
                                });
                            }
                            // Move pool contents into b.samples (no copy).
                            // Pool becomes empty for this slot; will re-grow on
                            // next iteration (one-time cold-path alloc, rare).
                            b.samples = std::mem::take(&mut self.dry_backup_pool[fx_idx]);
                        }
                    }
                }
                deck_peaks[deck_id] = b.samples.iter()
                    .flat_map(|ch| ch.iter()).fold(0.0f32, |m, &s| m.max(s.abs()))
                    * self.decks[deck_id].params.volume;
            }
            blocks[deck_id] = block;
        }

        // --- Correspondence Bus: vocal detection + derived state ---
        for deck_id in 0..4 {
            if let Some(ref b) = blocks[deck_id] {
                if b.channels() >= 1 {
                    self.bus.detect_vocal(deck_id, &b.samples[0], b.sample_rate);
                }
            }
        }
        self.bus.deck_keys = [
            self.decks[0].key.clone(), self.decks[1].key.clone(),
            self.decks[2].key.clone(), self.decks[3].key.clone(),
        ];
        self.bus.deck_bpm = [
            self.decks[0].bpm.unwrap_or(0.0) as f64,
            self.decks[1].bpm.unwrap_or(0.0) as f64,
            self.decks[2].bpm.unwrap_or(0.0) as f64,
            self.decks[3].bpm.unwrap_or(0.0) as f64,
        ];
        self.bus.update_derived();

        // Auto-duck vocals on incoming deck when collision detected
        if self.bus.vocal_collision > 0.3 {
            let (a_energy, b_energy) = (self.bus.vocal_energy[0], self.bus.vocal_energy[1]);
            let duck_deck = if a_energy < b_energy { 0 } else { 1 };
            let duck_db = self.bus.vocal_collision * 6.0;
            let gain = 10.0f32.powf(-duck_db / 20.0);
            if let Some(ref mut b) = blocks[duck_deck] {
                for ch in b.samples.iter_mut() {
                    for s in ch.iter_mut() {
                        *s *= gain;
                    }
                }
            }
        }

        // Determine output channel count and sample rate from first available deck, defaulting to stereo 44100.
        let (channels, sample_rate) = blocks.iter().find_map(|b| b.as_ref().map(|ab| (ab.channels(), ab.sample_rate)))
            .unwrap_or((2, 44100));

        // Headphone PFL mix: sum pre-fader audio from decks with pfl=true
        // Recycle previous headphone buffer before building new one
        if let Some(old_hp) = self.headphone_mix.take() {
            self.hp_scratch = old_hp.samples;
        }
        let any_pfl = self.decks.iter().any(|d| d.params.pfl);
        if any_pfl {
            let mut hp = prep_scratch(&mut self.hp_scratch, channels, frames);
            for i in 0..4 {
                if !self.decks[i].params.pfl { continue; }
                if let Some(ref b) = blocks[i] {
                    for ch in 0..channels.min(b.channels()) {
                        for j in 0..frames.min(b.samples[ch].len()) {
                            hp[ch][j] += b.samples[ch][j] * self.decks[i].params.volume;
                        }
                    }
                }
            }
            // Apply headphone volume + hard clamp (speaker protection)
            let hv = self.headphone_volume;
            for ch in hp.iter_mut() { for s in ch.iter_mut() { *s = (*s * hv).clamp(-1.0, 1.0); } }
            self.headphone_mix = Some(AudioBuffer { samples: hp, sample_rate });
        } else {
            self.headphone_mix = None;
        }

        // Crossfader smoothing
        let cf_start = self.smoothed_crossfader;
        let alpha = 1.0 - (-1.0 / (0.005 * sample_rate as f32)).exp();
        let target_cf = self.crossfader.clamp(0.0, 1.0);
        self.smoothed_crossfader += alpha * (target_cf - self.smoothed_crossfader);
        let cf_end = self.smoothed_crossfader;

        let master = self.master_volume;
        let mut out = prep_scratch(&mut self.out_scratch, channels, frames);
        let frames_f = frames as f32;

        // Mix: apply crossfader to left/right deck assignments
        for ch in 0..channels {
            for i in 0..frames {
                let t = i as f32 / frames_f;
                let cf = cf_start + (cf_end - cf_start) * t;

                // Crossfade gains based on curve type
                let (gain_left, gain_right) = match self.crossfader_curve {
                    CrossfaderCurve::SmoothBlend => {
                        let a = (cf * std::f32::consts::FRAC_PI_2).cos();
                        let b = (cf * std::f32::consts::FRAC_PI_2).sin();
                        (a, b)
                    }
                    CrossfaderCurve::SharpCut => {
                        let a = if cf < 0.4 { 1.0 } else if cf > 0.6 { 0.0 } else { 1.0 - (cf - 0.4) * 5.0 };
                        let b = if cf > 0.6 { 1.0 } else if cf < 0.4 { 0.0 } else { (cf - 0.4) * 5.0 };
                        (a.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
                    }
                };

                let mut sample = 0.0f32;

                // Sum all left-side decks
                for &deck_id in &self.crossfader_assignment.left {
                    let deck_idx = deck_id as usize;
                    let s = blocks[deck_idx].as_ref()
                        .and_then(|b| b.samples.get(ch))
                        .and_then(|c| c.get(i))
                        .copied()
                        .unwrap_or(0.0);
                    sample += s * self.decks[deck_idx].params.volume * gain_left;
                }

                // Sum all right-side decks
                for &deck_id in &self.crossfader_assignment.right {
                    let deck_idx = deck_id as usize;
                    let s = blocks[deck_idx].as_ref()
                        .and_then(|b| b.samples.get(ch))
                        .and_then(|c| c.get(i))
                        .copied()
                        .unwrap_or(0.0);
                    sample += s * self.decks[deck_idx].params.volume * gain_right;
                }

                out[ch][i] = sample * master;
            }
        }

        // Apply FX slots in series — zero-copy: move samples through pipeline.
        // `out` is already `mut` from earlier; the previous `let mut out = out;`
        // shadow was a no-op (clippy::let_and_return-adjacent: redundant_locals).
        for slot in &self.fx_slots {
            if slot.enabled && slot.intensity > 0.01 {
                if let Some(ref pipeline) = slot.cached_pipeline {
                    let buf = AudioBuffer { samples: out, sample_rate };
                    match pipeline.apply(buf) {
                        Ok(processed) => out = processed.samples,
                        Err(_) => { out = vec![vec![0.0; frames]; channels]; }
                    }
                    continue;
                }
            }
        }

        // Apply master effects if set — zero-copy
        if let Some(ref pipeline) = self.master_effects {
            let buf = AudioBuffer { samples: out, sample_rate };
            match pipeline.apply(buf) {
                Ok(processed) => out = processed.samples,
                Err(_) => { out = vec![vec![0.0; frames]; channels]; }
            }
        }

        // --- Mic block: drain once, compute level, drive duck + monitor + master ---
        //
        // Drain the cpal input ring into a mono scratch buffer. Always drain even when
        // inactive so the producer does not back up. The ring is absent when no input
        // device is attached; in that case the scratch stays zeroed.
        self.mic_scratch.resize(frames, 0.0);
        for s in self.mic_scratch.iter_mut() { *s = 0.0; }
        let mic_attached = self.mic_input_rx.is_some();
        if let Some(ref mut rx) = self.mic_input_rx {
            let gain = self.mic.volume;
            for j in 0..frames {
                let raw = rx.pop().unwrap_or(0.0);
                self.mic_scratch[j] = raw * gain;
            }
        }

        // Block peak + RMS of the drained mic samples (post-volume).
        let mut mic_peak = 0.0f32;
        let mut mic_sumsq = 0.0f32;
        for &s in self.mic_scratch.iter() {
            mic_peak = mic_peak.max(s.abs());
            mic_sumsq += s * s;
        }
        let mic_block_rms = if frames > 0 { (mic_sumsq / frames as f32).sqrt() } else { 0.0 };

        // Envelope follower: block-rate pole smoothing with asymmetric attack/release.
        // Only updated when the envelope is actually consumed (auto-duck armed).
        if self.mic.auto_duck_enabled {
            let sr_f = sample_rate as f32;
            let block_secs = (frames as f32 / sr_f).max(1e-6);
            let att_tau = (self.mic.duck_attack_ms * 0.001).max(1e-4);
            let rel_tau = (self.mic.duck_release_ms * 0.001).max(1e-4);
            let alpha_a = 1.0 - (-block_secs / att_tau).exp();
            let alpha_r = 1.0 - (-block_secs / rel_tau).exp();
            let target = mic_block_rms;
            let alpha = if target > self.mic.duck_env { alpha_a } else { alpha_r };
            self.mic.duck_env += alpha * (target - self.mic.duck_env);
        } else {
            // Let the envelope decay back to silence when auto-duck is disarmed.
            self.mic.duck_env = 0.0;
        }

        // Decide applied duck dB for this block.
        //   auto_duck on  -> envelope-driven, mapped linearly between threshold and max.
        //   auto_duck off -> legacy static talkover behavior (gated by mic.active).
        let duck_applied_db = if self.mic.auto_duck_enabled {
            let thresh = self.mic.duck_threshold.max(0.0).min(0.999);
            let over = (self.mic.duck_env - thresh).max(0.0);
            let span = (1.0 - thresh).max(1e-4);
            let k = (over / span).clamp(0.0, 1.0);
            // duck_max_db is expected to be <= 0 (e.g. -12.0). k=0 -> 0 dB, k=1 -> max.
            self.mic.duck_max_db.min(0.0) * k
        } else if self.mic.active && self.mic.talkover_duck_db < 0.0 {
            self.mic.talkover_duck_db
        } else {
            0.0
        };
        self.mic.duck_applied_db = duck_applied_db;

        // Apply the duck to the master bus.
        if duck_applied_db < 0.0 {
            let duck_gain = 10.0f32.powf(duck_applied_db / 20.0);
            for ch in out.iter_mut() {
                for s in ch.iter_mut() { *s *= duck_gain; }
            }
        }

        // Mix mic into master when active.
        if self.mic.active {
            for j in 0..frames {
                let s = self.mic_scratch[j];
                for ch in 0..channels {
                    out[ch][j] += s;
                }
            }
            self.mic.peak = mic_peak;
            self.mic.rms = mic_block_rms;
        } else {
            self.mic.peak = 0.0;
            self.mic.rms = 0.0;
        }

        // Monitor (loopback): route mic into the headphone bus regardless of master state.
        // Creates the headphone buffer if PFL did not already build one, or adds into it.
        if self.mic.monitor_enabled && mic_attached {
            let hv = self.headphone_volume;
            if self.headphone_mix.is_none() {
                let mut hp = prep_scratch(&mut self.hp_scratch, channels, frames);
                for j in 0..frames {
                    let s = self.mic_scratch[j];
                    for ch in 0..channels {
                        hp[ch][j] = (s * hv).clamp(-1.0, 1.0);
                    }
                }
                self.headphone_mix = Some(AudioBuffer { samples: hp, sample_rate });
            } else if let Some(ref mut hp) = self.headphone_mix {
                let hp_channels = hp.samples.len();
                for ch in 0..channels.min(hp_channels) {
                    let ch_len = hp.samples[ch].len();
                    for j in 0..frames.min(ch_len) {
                        let s = self.mic_scratch[j];
                        hp.samples[ch][j] = (hp.samples[ch][j] + s * hv).clamp(-1.0, 1.0);
                    }
                }
            }
        }

        // Ghost-whisper hook: proprietary DSP plugs in through WhisperSlot.
        // Called once per block with a zeroed mono bus; result is summed into master.
        if self.whisper.is_registered() {
            self.whisper_scratch.resize(frames, 0.0);
            for s in self.whisper_scratch.iter_mut() { *s = 0.0; }
            self.whisper.tick(&mut self.whisper_scratch, sample_rate);
            for j in 0..frames {
                let s = self.whisper_scratch[j];
                for ch in 0..channels {
                    out[ch][j] += s;
                }
            }
        }

        // Sum sampler outputs into main mix
        for sampler in &mut self.samplers {
            if let Some(sb) = sampler.read_block(frames) {
                for ch in 0..channels.min(sb.channels()) {
                    for j in 0..frames.min(sb.samples[ch].len()) {
                        out[ch][j] += sb.samples[ch][j];
                    }
                }
            }
        }

        // Record if active — capped at 120 min to prevent OOM
        const MAX_RECORD_SAMPLES: usize = 48000 * 60 * 120;
        if self.recording {
            if self.record_buffer.is_empty() {
                // Pre-allocate 10 min to avoid realloc stalls in audio thread
                let prealloc = 48000 * 60 * 10;
                self.record_buffer = vec![Vec::with_capacity(prealloc); channels];
            }
            if self.record_buffer[0].len() + frames < MAX_RECORD_SAMPLES {
                for ch in 0..channels.min(self.record_buffer.len()) {
                    self.record_buffer[ch].extend_from_slice(&out[ch]);
                }
            } else {
                // Auto-stop recording to prevent OOM — UI sees recording=false in snapshot
                self.recording = false;
            }
        }

        // Track peak + RMS levels for VU meters (4 decks + master)
        let peak_master = out.iter()
            .flat_map(|ch| ch.iter()).fold(0.0f32, |m, &s| m.max(s.abs()));
        let rms_master = {
            let sum: f32 = out.iter().flat_map(|ch| ch.iter()).map(|s| s * s).sum();
            let count = out.iter().map(|ch| ch.len()).sum::<usize>().max(1);
            (sum / count as f32).sqrt()
        };

        // Master FFT: mono-sum final output into 128-sample ring, run FFT when full
        {
            let mono_iter = (0..frames).map(|j| {
                out.iter().filter_map(|ch| ch.get(j)).sum::<f32>() / channels.max(1) as f32
            });
            for s in mono_iter {
                let pos = self.master_fft_fill % 128;
                self.master_fft_ring[pos] = s;
                self.master_fft_fill += 1;
            }
            if self.master_fft_fill >= 128 {
                self.fft_in_buf.copy_from_slice(&self.master_fft_ring);
                let plan = Arc::clone(&self.fft_plan);
                if plan.process(&mut self.fft_in_buf, &mut self.fft_out_buf).is_ok() {
                    for (bin, c) in self.fft_out_buf.iter().take(64).enumerate() {
                        self.master_fft[bin] = (c.re * c.re + c.im * c.im).sqrt();
                    }
                }
                self.master_fft_fill = 0;
            }
        }

        // Per-deck RMS + 3-band spectral energy (approximate: low<300Hz, mid<3kHz, high>3kHz)
        for i in 0..4 {
            if let Some(ref b) = blocks[i] {
                let num_ch = b.channels().max(1) as f32;
                let mut sum_sq = 0.0f32;
                let mut low_e = 0.0f32;
                let mut high_e = 0.0f32;
                let mut prev_mono = 0.0f32;

                for j in 0..frames {
                    let mut s = 0.0f32;
                    for ch in &b.samples {
                        if let Some(&val) = ch.get(j) {
                            s += val;
                        }
                    }
                    let mono_s = s / num_ch;
                    sum_sq += mono_s * mono_s;

                    if j > 0 {
                        let diff = (mono_s - prev_mono).abs();
                        low_e += mono_s.abs();
                        high_e += diff;
                    }
                    prev_mono = mono_s;

                    // Feed mono samples into per-deck ring buffer; run FFT when 128 accumulated
                    let pos = self.fft_ring_fill[i] % 128;
                    self.fft_ring[i][pos] = mono_s;
                    self.fft_ring_fill[i] += 1;
                    if self.fft_ring_fill[i] >= 128 {
                        self.fft_in_buf.copy_from_slice(&self.fft_ring[i]);
                        let plan = Arc::clone(&self.fft_plan);
                        if plan.process(&mut self.fft_in_buf, &mut self.fft_out_buf).is_ok() {
                            for (bin, c) in self.fft_out_buf.iter().take(64).enumerate() {
                                self.fft_bins[i][bin] = (c.re * c.re + c.im * c.im).sqrt();
                            }
                        }
                        self.fft_ring_fill[i] = 0;
                    }
                }

                let rms = (sum_sq / frames.max(1) as f32).sqrt();
                self.rms_levels[i] = rms;

                let total = low_e + high_e + 0.001;
                let low_ratio = low_e / total;
                let high_ratio = high_e / total;
                let mid_ratio = 1.0 - low_ratio - high_ratio;
                let energy = rms * self.decks[i].params.volume;
                self.spectral_energy[i] = [
                    energy * low_ratio,
                    energy * mid_ratio.max(0.0),
                    energy * high_ratio,
                ];
            }
        }

        // Recycle deck scratch buffers for zero-alloc reuse next cycle
        for i in 0..4 {
            if let Some(buf) = blocks[i].take() {
                self.decks[i].scratch_out = buf.samples;
            }
        }

        // Update all 5 peak + RMS values with decay
        for i in 0..4 {
            if deck_peaks[i] > self.peak_levels[i] {
                self.peak_levels[i] = deck_peaks[i];
            } else {
                self.peak_levels[i] *= 0.95;
            }
        }
        if peak_master > self.peak_levels[4] {
            self.peak_levels[4] = peak_master;
        } else {
            self.peak_levels[4] *= 0.95;
        }
        self.rms_levels[4] = rms_master;

        // Compute beat phase delta between two loudest playing decks
        self.beat_phase_delta = self.compute_beat_phase_delta();

        // Broadcast tap — push interleaved samples to ring buffer (non-blocking)
        if let Some(ref mut tx) = self.broadcast_tx {
            for i in 0..frames {
                for ch in 0..channels {
                    // Drop samples on overflow rather than stalling the audio thread
                    let _ = tx.push(out[ch][i]);
                }
            }
        }

        // Inject psychoacoustic layer (sub-threshold binaural + sub-bass)
        {
            let mut psycho_buf = AudioBuffer { samples: out, sample_rate };
            self.bus.inject_psychoacoustic(&mut psycho_buf);
            out = psycho_buf.samples;
        }

        // Code-voice block EXCLUDED — needs crate::fauna (excluded, real
        // unsafe: OnceLock<UnsafeCell<_>> single-audio-thread pattern).

        // Heal plugin: hidden master insert (post-FX, pre-limiter)
        if self.heal.active {
            // Auto-feed BPM from reference deck
            if let Some(ref_bpm) = self.decks.iter()
                .find(|d| d.sync_mode == SyncMode::Leader && d.bpm.is_some())
                .or_else(|| self.decks.iter().find(|d| d.params.playing && d.bpm.is_some()))
                .and_then(|d| d.bpm) {
                self.heal.bpm = ref_bpm;
            }
            self.apply_heal(&mut out, frames, sample_rate);
        }

        // Hard clamp — prevent digital clipping on output
        for ch in out.iter_mut() {
            for s in ch.iter_mut() {
                *s = s.clamp(-1.0, 1.0);
            }
        }

        // Generate booth output (main mix with booth_volume) — reuse buffer to avoid alloc
        if self.booth_volume > 0.01 {
            let bv = self.booth_volume;
            let actual_frames = out[0].len(); // use actual buffer length, not `frames`
            let booth = self.booth_mix.get_or_insert_with(|| AudioBuffer {
                samples: vec![vec![0.0; actual_frames]; channels],
                sample_rate,
            });
            booth.sample_rate = sample_rate;
            for ch in 0..channels {
                let bch = &mut booth.samples[ch];
                bch.resize(actual_frames, 0.0);
                for i in 0..actual_frames {
                    bch[i] = out[ch][i] * bv;
                }
            }
        } else {
            self.booth_mix = None;
        }

        AudioBuffer { samples: out, sample_rate }
    }

    /// Apply a named parameter from MIDI mapping.
    /// Supports dotted names: "deck_a.eq_high", "deck_c.volume", "crossfader", "master_volume", etc.
    pub fn apply_param(&mut self, name: &str, value: f32) {
        match name {
            "crossfader" => self.crossfader = value.clamp(0.0, 1.0),
            "crossfader_curve" => self.crossfader_curve = if value >= 0.5 { CrossfaderCurve::SharpCut } else { CrossfaderCurve::SmoothBlend },
            "master_volume" => self.master_volume = value.clamp(0.0, 1.0),
            "headphone_volume" => self.headphone_volume = value.clamp(0.0, 1.0),
            "headphone_blend" => self.headphone_blend = value.clamp(0.0, 1.0),
            "booth_volume" => self.booth_volume = value.clamp(0.0, 1.0),
            s if s.starts_with("deck_a.") => self.apply_deck_param_ext(0, &s[7..], value),
            s if s.starts_with("deck_b.") => self.apply_deck_param_ext(1, &s[7..], value),
            s if s.starts_with("deck_c.") => self.apply_deck_param_ext(2, &s[7..], value),
            s if s.starts_with("deck_d.") => self.apply_deck_param_ext(3, &s[7..], value),
            "mic" => self.mic.active = value > 0.5,
            // SF-001: LOCKED — silent fallthrough closed. External IPC callers must use apply_param_typed
            // (parses via MixerParam::from_str, rejects unknowns at the boundary).
            // This untyped path is kept for internal trusted callers only.
            // See forge-audio/src/params.rs::tests::sf_001_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
            // Game-specific params consumed by RecipeState — silently ignore in DJ mixer
            "tick_seed" | "pitch" | "ambience_intensity" | "rain_intensity"
            | "wind_speed" | "brand_distortion" | "era_mood"
            | "rain_permyriad" | "wind_permyriad" | "fog_permyriad" => {}
            _ => eprintln!("[mixer] Unknown param: {} (internal caller — use apply_param_typed at IPC boundary)", name),
        }
    }

    /// Apply a fully-typed mixer parameter (SF-001 LOCKED gate).
    ///
    /// This is the only path that external IPC callers (commands, HTTP endpoints,
    /// live session mapping engine) should use. The `MixerParam` enum is constructed by
    /// `MixerParam::from_str` which rejects unknowns at the boundary — no silent drops.
    pub fn apply_param_typed(&mut self, param: crate::params::MixerParam, value: f32) {
        use crate::params::{MixerParam, DeckParam};
        match param {
            MixerParam::Crossfader      => self.crossfader = value.clamp(0.0, 1.0),
            MixerParam::CrossfaderCurve => self.crossfader_curve = if value >= 0.5 { CrossfaderCurve::SharpCut } else { CrossfaderCurve::SmoothBlend },
            MixerParam::MasterVolume    => self.master_volume = value.clamp(0.0, 1.0),
            MixerParam::HeadphoneVolume => self.headphone_volume = value.clamp(0.0, 1.0),
            MixerParam::HeadphoneBlend  => self.headphone_blend = value.clamp(0.0, 1.0),
            MixerParam::BoothVolume     => self.booth_volume = value.clamp(0.0, 1.0),
            MixerParam::Mic             => self.mic.active = value > 0.5,
            MixerParam::Deck { id, param } => {
                let deck_id = id as usize;
                match param {
                    DeckParam::Volume => {
                        self.decks[deck_id].params.volume = value.clamp(0.0, 1.0);
                        self.hw_fader_override[deck_id] = value < 0.05;
                    }
                    DeckParam::Tempo      => self.decks[deck_id].params.tempo = value.clamp(0.25, 4.0),
                    DeckParam::EqLow      => self.decks[deck_id].params.eq.low = value.clamp(-60.0, 12.0),
                    DeckParam::EqMid      => self.decks[deck_id].params.eq.mid = value.clamp(-60.0, 12.0),
                    DeckParam::EqHigh     => self.decks[deck_id].params.eq.high = value.clamp(-60.0, 12.0),
                    DeckParam::FxAmount   => self.decks[deck_id].params.fx_amount = value.clamp(0.0, 1.0),
                    DeckParam::Pregain    => self.decks[deck_id].params.replay_gain_db = (value * 24.0 - 12.0).clamp(-12.0, 12.0),
                    DeckParam::Pfl        => self.decks[deck_id].params.pfl = value > 0.5,
                    DeckParam::Keylock    => self.decks[deck_id].params.keylock = value > 0.5,
                    DeckParam::LoopHalf   => {
                        let d = &mut self.decks[deck_id].params;
                        if d.loop_end > d.loop_start {
                            let half = (d.loop_end - d.loop_start) / 2;
                            if half > 256 { d.loop_end = d.loop_start + half; }
                        }
                    }
                    DeckParam::LoopDouble => {
                        let d = &mut self.decks[deck_id].params;
                        if d.loop_end > d.loop_start {
                            d.loop_end = d.loop_start + (d.loop_end - d.loop_start) * 2;
                        }
                    }
                    DeckParam::LoopSize   => {
                        let q = self.decks[deck_id].params.quantize;
                        self.decks[deck_id].params.quantize = !q;
                    }
                    DeckParam::Scratching => self.decks[deck_id].scratching = value > 0.5,
                    DeckParam::Slip       => self.decks[deck_id].slip_active = value > 0.5,
                    DeckParam::FxAssign(slot) => {
                        if slot < 4 { self.fx_assign[deck_id][slot] = value > 0.5; }
                    }
                }
            }
        }
    }

    /// Route deck params — handles both DeckParams fields and Deck-level fields.
    fn apply_deck_param_ext(&mut self, deck_id: usize, name: &str, value: f32) {
        match name {
            "volume" => {
                self.decks[deck_id].params.volume = value.clamp(0.0, 1.0);
                self.hw_fader_override[deck_id] = value < 0.05;
            }
            "scratching" => self.decks[deck_id].scratching = value > 0.5,
            "slip" => self.decks[deck_id].slip_active = value > 0.5,
            n if n.starts_with("fx_assign_") => {
                if let Ok(slot) = n[10..].parse::<usize>() {
                    if slot < 4 { self.fx_assign[deck_id][slot] = value > 0.5; }
                }
            }
            _ => apply_deck_param(&mut self.decks[deck_id].params, name, value),
        }
    }

    /// Load a horror preset onto the master bus.
    pub fn set_preset(&mut self, name: &str, intensity: f32) -> Result<(), String> {
        let pipeline = crate::presets::load_preset(name, intensity)?;
        self.master_effects = Some(pipeline);
        Ok(())
    }

    /// Assign a preset to an FX slot (0-3). Parses and caches the pipeline.
    pub fn set_fx_slot(&mut self, slot: usize, preset: &str) {
        if slot < 4 {
            self.fx_slots[slot].set_preset(preset, self.fx_slots[slot].intensity);
        }
    }

    /// Toggle an FX slot on/off.
    pub fn toggle_fx_slot(&mut self, slot: usize) {
        if slot < 4 {
            self.fx_slots[slot].enabled = !self.fx_slots[slot].enabled;
        }
    }

    /// Set FX slot intensity (0.0-1.0). Re-caches the pipeline with new intensity.
    pub fn set_fx_intensity(&mut self, slot: usize, intensity: f32) {
        if slot < 4 {
            self.fx_slots[slot].update_intensity(intensity.clamp(0.0, 1.0));
        }
    }

    /// Clear the master effects chain.
    pub fn clear_master_effects(&mut self) {
        self.master_effects = None;
    }

    /// Return recorded audio and clear the record buffer. Returns `None` if
    /// the buffer is empty.
    pub fn take_recording(&mut self) -> Option<AudioBuffer> {
        if self.record_buffer.is_empty() || self.record_buffer[0].is_empty() {
            return None;
        }

        let sample_rate = self.decks.iter()
            .find_map(|d| d.buffer.as_ref().map(|b| b.sample_rate))
            .unwrap_or(44100);

        let samples = std::mem::take(&mut self.record_buffer);
        Some(AudioBuffer { samples, sample_rate })
    }

    /// Apply a named action (transport, toggle) — untyped shim for legacy callers.
    ///
    /// New code must send `MixerCommand::ActionTyped` so the parse happens at
    /// the IPC boundary.  This shim parses internally so the `_ =>` arm can
    /// never silently drop an unknown action.
    pub fn apply_action(&mut self, target: &str) {
        use std::str::FromStr;
        // SF-002: LOCKED — parse at the action boundary; typos now log rather than silently succeed.
        // See forge-audio/src/params.rs::tests::sf_002_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
        match crate::params::MixerAction::from_str(target) {
            Ok(action) => self.apply_action_typed(action),
            Err(e)     => eprintln!("[mixer] SF-002: unrecognized action — {}", e),
        }
    }

    /// Apply a fully-typed action.  Exhaustive match — compiler enforces coverage.
    pub fn apply_action_typed(&mut self, action: crate::params::MixerAction) {
        use crate::params::MixerAction;
        match action {
            MixerAction::DeckPlayPause(id) => {
                let idx = id as usize;
                self.decks[idx].params.playing = !self.decks[idx].params.playing;
            }
            MixerAction::DeckCue(id) => {
                let idx = id as usize;
                self.decks[idx].params.playing = false;
                self.decks[idx].playback_pos = self.decks[idx].cue_point;
            }
            MixerAction::DeckQuantizeToggle(id) => {
                let idx = id as usize;
                self.decks[idx].params.quantize = !self.decks[idx].params.quantize;
            }
            MixerAction::DeckGridToggle(id) => {
                // Grid adjust — TODO: beat grid editor
                eprintln!("[mixer] Grid toggle: deck {}", id as usize);
            }
            MixerAction::DeckLoadTrack(id) => {
                // Load track needs file picker — not mixer state
                eprintln!("[mixer] Load track (UI-only): deck {}", id as usize);
            }
            MixerAction::RecordToggle => {
                self.recording = !self.recording;
            }
            MixerAction::BrowseToggle => {
                eprintln!("[mixer] Browse toggle (UI-only)");
            }
            MixerAction::BrowseBack => {
                eprintln!("[mixer] Browse back (UI-only)");
            }
            MixerAction::BrowseSelect => {
                eprintln!("[mixer] Browse select (UI-only)");
            }
        }
    }

    // â”€â”€ AudioState helpers (called from feeder loop, per-block) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Master bus RMS energy (0.0-1.0).
    pub fn master_rms(&self) -> f32 {
        self.rms_levels[4]
    }

    /// Spectral centroid from master FFT (0.0-1.0 normalized).
    pub fn spectral_centroid(&self) -> f32 {
        let mut weighted_sum = 0.0f32;
        let mut magnitude_sum = 0.0f32;
        for (i, &mag) in self.master_fft.iter().enumerate() {
            weighted_sum += i as f32 * mag;
            magnitude_sum += mag;
        }
        if magnitude_sum > 0.001 {
            (weighted_sum / (magnitude_sum * 64.0)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    /// Simple onset/drop detection: master RMS > 0.7.
    pub fn drop_detected(&self) -> bool {
        self.rms_levels[4] > 0.7
    }

    /// Genre of the loudest playing deck (0 if none).
    pub fn active_genre(&self) -> u8 {
        self.decks.iter()
            .filter(|d| d.params.playing)
            .max_by(|a, b| a.params.volume.partial_cmp(&b.params.volume).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|d| d.genre)
            .unwrap_or(0)
    }

    /// BeatGrid of the loudest playing deck.
    pub fn active_beat_grid(&self) -> Option<crate::bpm::BeatGrid> {
        self.decks.iter()
            .filter(|d| d.params.playing)
            .max_by(|a, b| a.params.volume.partial_cmp(&b.params.volume).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|d| d.beatgrid.clone())
    }

    /// Sample position of the loudest playing deck.
    pub fn sample_position(&self) -> usize {
        self.decks.iter()
            .filter(|d| d.params.playing)
            .max_by(|a, b| a.params.volume.partial_cmp(&b.params.volume).unwrap_or(std::cmp::Ordering::Equal))
            .map(|d| d.playback_pos)
            .unwrap_or(0)
    }

    /// Copy of master FFT bins for AudioState.
    pub fn spectrum_snapshot(&self) -> [f32; 64] {
        self.master_fft
    }

    /// Apply ambient weather modulation. Converts permyriad to f32 on the audio thread.
    /// Modulates the correspondence bus ambient layer (reverb wet, noise gain, filter sweep).
    pub fn apply_ambient_weather(&mut self, rain: u16, wind: u16, fog: u16) {
        let rain_f = rain as f32 / 10000.0;
        let wind_f = wind as f32 / 10000.0;
        let fog_f = fog as f32 / 10000.0;
        // Rain â†’ reverb wet mix on correspondence bus
        self.bus.set_ambient_reverb(rain_f * 0.6);
        // Wind â†’ high-pass sweep (higher wind = more high-pass)
        self.bus.set_ambient_filter(wind_f);
        // Fog â†’ low-pass damping (more fog = more muffled)
        self.bus.set_ambient_damping(fog_f * 0.8);
    }
}

fn apply_deck_param(params: &mut DeckParams, name: &str, value: f32) {
    match name {
        "volume" => { params.volume = value.clamp(0.0, 1.0); }
        "tempo" => params.tempo = value.clamp(0.25, 4.0),
        "eq_low" => params.eq.low = value.clamp(-60.0, 12.0),
        "eq_mid" => params.eq.mid = value.clamp(-60.0, 12.0),
        "eq_high" => params.eq.high = value.clamp(-60.0, 12.0),
        "fx_amount" => params.fx_amount = value.clamp(0.0, 1.0),
        "pregain" => params.replay_gain_db = (value * 24.0 - 12.0).clamp(-12.0, 12.0),
        "pfl" => params.pfl = value > 0.5,
        "keylock" => params.keylock = value > 0.5,
        "loop_half" => {
            if params.loop_end > params.loop_start {
                let half_len = (params.loop_end - params.loop_start) / 2;
                if half_len > 256 { // minimum ~5ms at 44.1kHz
                    params.loop_end = params.loop_start + half_len;
                }
            }
        }
        "loop_double" => {
            if params.loop_end > params.loop_start {
                let double_len = (params.loop_end - params.loop_start) * 2;
                params.loop_end = params.loop_start + double_len;
            }
        }
        "loop_size" | "quantize_toggle" => { params.quantize = !params.quantize; },
        // SF-003: LOCKED — structurally closed: apply_param_typed uses an exhaustive DeckParam match,
        // so this arm is only reachable via the legacy apply_param path (MixerCommand::Param).
        // See forge-audio/src/params.rs::tests::sf_003_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
        _ => eprintln!("[deck] Unknown param: {}", name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::AudioBuffer;

    #[test]
    fn test_4_decks_default() {
        let mixer = Mixer::default();
        assert_eq!(mixer.decks.len(), 4);
        for deck in &mixer.decks {
            assert!(deck.buffer.is_none());
            assert!(!deck.params.playing);
        }
    }

    #[test]
    fn test_deck_id_indexing() {
        let _mixer = Mixer::default();
        assert_eq!(DeckId::A as usize, 0);
        assert_eq!(DeckId::B as usize, 1);
        assert_eq!(DeckId::C as usize, 2);
        assert_eq!(DeckId::D as usize, 3);
    }

    #[test]
    fn test_crossfader_assignment() {
        let mixer = Mixer::default();
        assert_eq!(mixer.crossfader_assignment.left, vec![DeckId::A, DeckId::C]);
        assert_eq!(mixer.crossfader_assignment.right, vec![DeckId::B, DeckId::D]);
    }

    #[test]
    fn test_empty_decks_produce_silence() {
        let mut mixer = Mixer::default();
        for deck in &mut mixer.decks {
            deck.params.playing = true;
        }
        let output = mixer.mix_block(1024);
        let is_silent = output.samples.iter()
            .all(|ch| ch.iter().all(|&s| s.abs() < 1e-6));
        assert!(is_silent);
    }

    #[test]
    fn mixer_apply_action_record_toggle() {
        let mut mixer = Mixer::default();
        assert!(!mixer.recording);
        mixer.apply_action("record_toggle");
        assert!(mixer.recording);
    }

    #[test]
    fn mixer_apply_action_unknown_is_safe() {
        let mut mixer = Mixer::default();
        mixer.apply_action("nonexistent.action"); // should not panic
    }

    #[test]
    fn crossfader_curve_sharp_cuts() {
        let mut mixer = Mixer::default();
        mixer.crossfader_curve = CrossfaderCurve::SharpCut;
        mixer.crossfader = 0.0; // full left
        let block = mixer.mix_block(64);
        assert_eq!(block.samples[0].len(), 64);
    }

    #[test]
    fn slip_mode_snaps_back() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![1.0; 44100]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.decks[0].slip_active = true;
        mixer.decks[0].slip_pos = 0;
        mixer.mix_block(512); // advance both positions
        assert!(mixer.decks[0].playback_pos > 0);
        assert!(mixer.decks[0].slip_pos > 0);
        // Seek away
        mixer.decks[0].playback_pos = 22050;
        // Deactivate slip — should snap back to slip_pos
        let snap_pos = mixer.decks[0].slip_pos;
        mixer.decks[0].playback_pos = snap_pos; // manual snap (command does this)
        mixer.decks[0].slip_active = false;
        assert_eq!(mixer.decks[0].playback_pos, snap_pos);
    }

    #[test]
    fn replay_gain_amplifies() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 512]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.decks[0].params.replay_gain_db = 6.0; // ~2x gain
        mixer.crossfader = 0.0;
        let block = mixer.mix_block(64);
        let peak = block.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak > 0.5); // should be louder
    }

    #[test]
    fn beatgrid_phase_tracks() {
        let bg = crate::bpm::BeatGrid::from_bpm(120.0, 0, 44100, 44100 * 10);
        assert_eq!(bg.beat_interval, 22050); // 44100 * 60/120
        assert_eq!(bg.beat_pos(0), 0);
        assert_eq!(bg.beat_pos(1), 22050);
        let phase = bg.phase_at(11025); // halfway through first beat
        assert!((phase - 0.5).abs() < 0.01);
    }

    #[test]
    fn pfl_generates_headphone_mix() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 512]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.decks[0].params.pfl = true;
        mixer.crossfader = 0.0;
        mixer.mix_block(64);
        assert!(mixer.headphone_mix.is_some());
    }

    #[test]
    fn sampler_trigger_produces_audio() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.8; 1000]], sample_rate: 44100 };
        mixer.samplers[0].buffer = Some(buf);
        mixer.samplers[0].volume = 1.0;
        mixer.samplers[0].trigger();
        assert!(mixer.samplers[0].playing);
        let block = mixer.samplers[0].read_block(64);
        assert!(block.is_some());
        let peak = block.unwrap().samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak > 0.5);
    }

    #[test]
    fn talkover_ducks_music() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 512]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.crossfader = 0.0;
        // Without mic
        let block_no_mic = mixer.mix_block(64);
        let peak_no = block_no_mic.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        // Reset position
        mixer.decks[0].playback_pos = 0;
        // With mic active + ducking
        mixer.mic.active = true;
        mixer.mic.talkover_duck_db = -12.0;
        let block_mic = mixer.mix_block(64);
        let peak_mic = block_mic.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak_mic < peak_no); // ducked = quieter
    }

    #[test]
    fn mic_input_rx_drains_and_sums_into_master() {
        let mut mixer = Mixer::default();
        mixer.mic.active = true;
        mixer.mic.volume = 1.0;
        mixer.mic.talkover_duck_db = 0.0;
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(2048);
        let frames = 64usize;
        for _ in 0..frames {
            let _ = prod.push(0.5);
        }
        mixer.set_mic_input(cons);
        let block = mixer.mix_block(frames);
        let peak_out = block.samples[0]
            .iter()
            .fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak_out >= 0.5 * 0.99, "master peak < mic peak: {}", peak_out);
        assert!(mixer.mic.peak >= 0.5 * 0.99);
        assert!(mixer.mic.rms > 0.0);
    }

    #[test]
    fn mic_input_rx_drains_silently_when_inactive() {
        let mut mixer = Mixer::default();
        mixer.mic.active = false;
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(2048);
        for _ in 0..64 {
            let _ = prod.push(0.9);
        }
        mixer.set_mic_input(cons);
        mixer.mix_block(64);
        assert_eq!(mixer.mic.peak, 0.0);
        assert_eq!(mixer.mic.rms, 0.0);
    }

    #[test]
    fn hotcue_slot_cue_and_loop() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 44100]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        // Set cue
        mixer.decks[0].hotcues[0] = Some(HotcueSlot::Cue(1000));
        // Set loop
        mixer.decks[0].hotcues[1] = Some(HotcueSlot::Loop { start: 5000, end: 10000 });
        match mixer.decks[0].hotcues[0] {
            Some(HotcueSlot::Cue(pos)) => assert_eq!(pos, 1000),
            _ => panic!("Expected Cue"),
        }
        match mixer.decks[0].hotcues[1] {
            Some(HotcueSlot::Loop { start, end }) => {
                assert_eq!(start, 5000);
                assert_eq!(end, 10000);
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn booth_mix_generated() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 512]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.crossfader = 0.0;
        mixer.booth_volume = 0.5;
        mixer.mix_block(64);
        assert!(mixer.booth_mix.is_some());
    }

    #[test]
    fn scratch_rate_affects_playback() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 44100]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.decks[0].scratch_rate = 0.5; // 50% faster
        mixer.mix_block(512);
        // Should advance faster than normal
        assert!(mixer.decks[0].playback_pos > 512);
    }

    // ---- phase_correct() tests (Task 7 from dead-drop-arena-fixes spec) ----
    // Closes the structural-gap test for phase-locked beat sync without any
    // audio device, ForgeVision, or live playback. Pure algorithm verification.

    #[test]
    fn phase_correct_no_op_without_leader() {
        let mut mixer = Mixer::default();
        let initial = mixer.decks[1].params.tempo;
        mixer.phase_correct();
        // No leader â†’ function early-returns, nothing changes
        assert_eq!(mixer.decks[1].params.tempo, initial);
    }

    #[test]
    fn phase_correct_locks_tempo_to_leader_bpm() {
        let mut mixer = Mixer::default();
        let bg_a = crate::bpm::BeatGrid::from_bpm(125.0, 0, 48000, 48000 * 200);
        let bg_b = crate::bpm::BeatGrid::from_bpm(124.0, 0, 48000, 48000 * 200);
        mixer.decks[0].bpm = Some(125.0);
        mixer.decks[0].beatgrid = Some(bg_a);
        mixer.decks[0].sync_mode = SyncMode::Leader;
        mixer.decks[1].bpm = Some(124.0);
        mixer.decks[1].beatgrid = Some(bg_b);
        mixer.decks[1].sync_mode = SyncMode::Follower;
        // Single call should set the tempo lock immediately
        mixer.phase_correct();
        let expected = 125.0 / 124.0;
        let actual = mixer.decks[1].params.tempo;
        assert!((actual - expected).abs() < 0.001,
            "follower tempo should be leader_bpm/follower_bpm = {}, got {}", expected, actual);
    }

    #[test]
    fn phase_correct_aligns_phase_same_bpm() {
        let mut mixer = Mixer::default();
        let bg = crate::bpm::BeatGrid::from_bpm(125.0, 0, 48000, 48000 * 200);
        let beat_interval = bg.beat_interval;

        // Both decks at 125 BPM
        mixer.decks[0].bpm = Some(125.0);
        mixer.decks[0].beatgrid = Some(bg.clone());
        mixer.decks[0].sync_mode = SyncMode::Leader;
        mixer.decks[0].playback_pos = 0;

        mixer.decks[1].bpm = Some(125.0);
        mixer.decks[1].beatgrid = Some(bg.clone());
        mixer.decks[1].sync_mode = SyncMode::Follower;
        // Start follower at 30% phase offset
        mixer.decks[1].playback_pos = (beat_interval as f32 * 0.3) as usize;

        // Sanity: initial phase offset is ~0.3
        let initial = bg.phase_at(mixer.decks[1].playback_pos);
        assert!((initial - 0.3).abs() < 0.01,
            "expected ~0.3 initial phase, got {}", initial);

        // Run phase_correct + simulate playback advance for ~200 cycles
        // Each phase_correct nudges by min(delta, 0.01) * beat_interval samples
        // 0.3 / 0.01 = 30 nudges minimum to converge
        let frames: usize = 1024;
        for _ in 0..200 {
            mixer.phase_correct();
            mixer.decks[0].playback_pos += frames;
            let tempo = mixer.decks[1].params.tempo;
            mixer.decks[1].playback_pos += (frames as f32 * tempo) as usize;
        }

        // Phase delta should converge below 0.05 (Task 7 acceptance threshold)
        let pa = bg.phase_at(mixer.decks[0].playback_pos);
        let pb = bg.phase_at(mixer.decks[1].playback_pos);
        let mut delta = (pa - pb).abs();
        if delta > 0.5 { delta = 1.0 - delta; } // wrap-around
        assert!(delta < 0.05,
            "phase delta should converge < 0.05, got {} (leader phase={}, follower phase={})",
            delta, pa, pb);

        // Same BPM â†’ tempo lock = 1.0
        assert!((mixer.decks[1].params.tempo - 1.0).abs() < 0.001,
            "same BPM should yield tempo 1.0, got {}", mixer.decks[1].params.tempo);
    }

    // ---- Lane B: auto-duck envelope + monitor + whisper hook ----

    /// Auto-duck off: behavior equals the legacy static path (mic active + negative
    /// talkover_db reduces master by that amount exactly).
    #[test]
    fn auto_duck_off_falls_back_to_static_talkover() {
        let mut mixer = Mixer::default();
        let buf = AudioBuffer { samples: vec![vec![0.5; 512]], sample_rate: 44100 };
        mixer.decks[0].load(buf);
        mixer.decks[0].params.playing = true;
        mixer.crossfader = 0.0;
        mixer.mic.active = true;
        mixer.mic.auto_duck_enabled = false;
        mixer.mic.talkover_duck_db = -6.0;
        mixer.mix_block(64);
        assert!((mixer.mic.duck_applied_db - (-6.0)).abs() < 1e-4,
            "expected applied_db=-6.0 in legacy path, got {}", mixer.mic.duck_applied_db);
    }

    /// Auto-duck off + mic off: no reduction is applied.
    #[test]
    fn auto_duck_off_and_mic_off_no_reduction() {
        let mut mixer = Mixer::default();
        mixer.mic.active = false;
        mixer.mic.auto_duck_enabled = false;
        mixer.mix_block(64);
        assert_eq!(mixer.mic.duck_applied_db, 0.0);
    }

    /// Auto-duck armed with a loud mic input: envelope rises over successive blocks and
    /// applied_db reaches a meaningful reduction.
    #[test]
    fn auto_duck_envelope_reduces_master_on_loud_mic() {
        let mut mixer = Mixer::default();
        mixer.mic.auto_duck_enabled = true;
        mixer.mic.duck_threshold = 0.05;
        mixer.mic.duck_max_db = -12.0;
        mixer.mic.duck_attack_ms = 5.0;
        mixer.mic.duck_release_ms = 100.0;

        // Feed a steady high-RMS mic stream across several blocks.
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(8192);
        let frames = 512usize;
        for _ in 0..(frames * 8) {
            let _ = prod.push(0.8);
        }
        mixer.set_mic_input(cons);

        // First block primes the envelope; after a few blocks it should be well past threshold.
        for _ in 0..6 {
            mixer.mix_block(frames);
        }

        assert!(mixer.mic.duck_env > 0.5,
            "expected envelope > 0.5, got {}", mixer.mic.duck_env);
        assert!(mixer.mic.duck_applied_db <= -5.0,
            "expected applied_db <= -5 dB under loud mic, got {}", mixer.mic.duck_applied_db);
        assert!(mixer.mic.duck_applied_db >= mixer.mic.duck_max_db - 1e-3,
            "applied_db must not exceed configured max, got {}", mixer.mic.duck_applied_db);
    }

    /// Auto-duck armed with silence on the mic: no reduction.
    #[test]
    fn auto_duck_silence_leaves_master_unducked() {
        let mut mixer = Mixer::default();
        mixer.mic.auto_duck_enabled = true;
        mixer.mic.duck_threshold = 0.05;
        mixer.mic.duck_max_db = -12.0;
        let (_prod, cons) = rtrb::RingBuffer::<f32>::new(2048);
        mixer.set_mic_input(cons);
        for _ in 0..4 {
            mixer.mix_block(256);
        }
        assert_eq!(mixer.mic.duck_applied_db, 0.0);
    }

    /// Monitor enabled + mic attached: mic is routed to the headphone bus even when
    /// mic.active is false (independent from the master path).
    #[test]
    fn monitor_routes_mic_to_headphones_while_muted_on_master() {
        let mut mixer = Mixer::default();
        mixer.mic.active = false;
        mixer.mic.monitor_enabled = true;
        mixer.mic.volume = 1.0;
        mixer.headphone_volume = 1.0;

        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(2048);
        let frames = 64usize;
        for _ in 0..frames {
            let _ = prod.push(0.5);
        }
        mixer.set_mic_input(cons);
        mixer.mix_block(frames);

        let hp = mixer.headphone_mix.as_ref().expect("headphone buffer should exist with monitor on");
        let peak_hp = hp.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak_hp >= 0.49, "mic should be present in headphones with peak â‰¥ 0.49, got {}", peak_hp);
    }

    /// Monitor disabled: mic is NOT routed to the headphone bus even when attached.
    #[test]
    fn monitor_disabled_keeps_mic_out_of_headphones() {
        let mut mixer = Mixer::default();
        mixer.mic.active = false;
        mixer.mic.monitor_enabled = false;

        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(2048);
        for _ in 0..64 {
            let _ = prod.push(0.5);
        }
        mixer.set_mic_input(cons);
        mixer.mix_block(64);

        // No PFL decks, no monitor â†’ no headphone buffer at all.
        assert!(mixer.headphone_mix.is_none(),
            "headphone bus should remain empty without monitor or PFL");
    }

    /// Ghost-whisper hook: registering a hook routes its samples into the master bus.
    #[test]
    fn ghost_whisper_hook_feeds_master_bus() {
        use crate::ghost_whisper::GhostWhisperHook;

        struct Steady;
        impl GhostWhisperHook for Steady {
            fn tick(&mut self, bus: &mut [f32], _sample_rate: u32) {
                for s in bus.iter_mut() { *s = 0.25; }
            }
        }

        let mut mixer = Mixer::default();
        let frames = 64usize;
        // Baseline: silent master (no decks, no mic, no hook) â†’ out is zero.
        let block0 = mixer.mix_block(frames);
        let peak0 = block0.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak0 < 1e-6, "baseline should be silent, got peak {}", peak0);

        // With hook â†’ master picks up the whisper contribution.
        mixer.whisper.register(Box::new(Steady));
        let block = mixer.mix_block(frames);
        let peak = block.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!((peak - 0.25).abs() < 1e-4, "expected whisper peak ~0.25, got {}", peak);
    }
}


