//! Ported 2026-08-17 from F:\NewRepo\crates\technothesia\src\player.rs — Technothesia winamp-lite, Sean's 'find our old winamp music player'
//!
//! Winamp-lite file player — in-loop state machine tied to unified.rs's existing
//! `sing::Output` cpal stream. No second cpal device opens; the F9 device picker
//! controls both the additive synth AND the player because they share one ring.
//!
//! The render loop calls `Player::pump` each frame to mix decoded PCM into the
//! shared `rtrb::Producer<f32>`, then `Player::snapshot` to publish state.
//!
//! Sound Gate: decode is load-time (heap alloc fine; forge-audio carve-out).
//! Lock-Free Gate: `ArcSwap::store()` only — no Mutex in the snapshot path.

use std::sync::Arc;

use arc_swap::ArcSwap;

// ── Player snapshot types (inlined from v2 snapshot.rs) ────────────────────

/// Track metadata decoded from file tags.
#[derive(Debug, Clone, Default)]
pub struct TrackMeta {
    pub title:         String,
    pub artist:        String,
    pub album:         String,
    pub duration_secs: f32,
}

/// Full playback state snapshot — written by the player thread, read by the render thread.
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub playing:       bool,
    pub position_secs: f32,
    pub duration_secs: f32,
    pub volume:        f32,
    pub rms:           f32,
    /// 64-bin FFT magnitudes (0.0–1.0). Fed into OrbSwarm::update each frame.
    pub fft:           Vec<f32>,
    pub meta:          Option<TrackMeta>,
    pub loaded:        bool,
    pub init_failed:   Option<String>,
    pub load_error:    Option<String>,
    pub track_ended:   bool,
    pub peak_l:        f32,
    pub peak_r:        f32,
    /// Ring-buffer fill 0.0–1.0.
    pub buffer_fill:   f32,
    pub underruns:     u64,
    pub device_rate:   u32,
    pub file_rate:     u32,
    // ── Spectral semantic layer (DAG reasoning) ─────────────────────────────
    /// Weighted mean of FFT bins (0.0 = bass-heavy, 1.0 = treble-heavy).
    pub spectral_centroid: f32,
    /// RMS of ~300–3400 Hz band (vocal presence).
    pub vocal_energy:      f32,
    /// Index 0–7 into the loudest CE band.
    pub dominant_band:     usize,
    /// −1.0 = bass-heavy, +1.0 = treble-heavy.
    pub spectral_tilt:     f32,
    /// Beat phase 0.0–1.0.
    pub beat_phase:        f32,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            playing:       false,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume:        1.0,
            rms:           0.0,
            fft:           vec![0.0; 64],
            meta:          None,
            loaded:        false,
            init_failed:   None,
            load_error:    None,
            track_ended:   false,
            peak_l:        0.0,
            peak_r:        0.0,
            buffer_fill:   0.0,
            underruns:     0,
            device_rate:   0,
            file_rate:     0,
            spectral_centroid: 0.5,
            vocal_energy:  0.0,
            dominant_band: 0,
            spectral_tilt: 0.0,
            beat_phase:    0.0,
        }
    }
}

// ── Commands (queued externally; applied on the next `pump` call) ─────────────

pub enum PlayerCmd {
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(f32),
    SetVolume(f32),
}

// ── Decoded deck (load-time only) ─────────────────────────────────────────────

struct Deck {
    /// Interleaved stereo (or mono) samples at the OUTPUT device sample-rate.
    /// Resampling is done post-decode by forge-audio `resample_to`.
    samples:     Vec<f32>,
    cursor:      usize,
    sample_rate: u32,
    channels:    usize,
    meta:        TrackMeta,
}

impl Deck {
    fn duration_secs(&self) -> f32 {
        let denom = (self.sample_rate as usize).max(1) * self.channels.max(1);
        self.samples.len() as f32 / denom as f32
    }
    fn position_secs(&self) -> f32 {
        let denom = (self.sample_rate as usize).max(1) * self.channels.max(1);
        self.cursor as f32 / denom as f32
    }
    fn seek(&mut self, secs: f32) {
        let frame = (secs * self.sample_rate as f32) as usize * self.channels;
        self.cursor = frame.min(self.samples.len());
    }
    fn ended(&self) -> bool { self.cursor >= self.samples.len() }
}

// ── Player ────────────────────────────────────────────────────────────────────

pub struct Player {
    deck:       Option<Deck>,
    pub playing: bool,
    pub volume:  f32,
    pub underruns: u64,
    /// Published each frame into the shared ArcSwap for the render thread.
    pub snapshot: Arc<ArcSwap<PlayerSnapshot>>,
    /// Pending commands delivered by the render loop.
    pending: Vec<PlayerCmd>,
    /// RMS over last pump window.
    rms: f32,
}

impl Player {
    pub fn new() -> Self {
        Self {
            deck:      None,
            playing:   false,
            volume:    1.0,
            underruns: 0,
            snapshot:  Arc::new(ArcSwap::from_pointee(PlayerSnapshot::default())),
            pending:   Vec::new(),
            rms:       0.0,
        }
    }

    pub fn push_cmd(&mut self, cmd: PlayerCmd) { self.pending.push(cmd); }

    /// Apply queued commands. Call once per render frame with the current device config.
    pub fn apply_cmds(&mut self, out_channels: usize) {
        let cmds: Vec<PlayerCmd> = self.pending.drain(..).collect();
        for cmd in cmds {
            match cmd {
                PlayerCmd::Load(path) => self.do_load(&path, out_channels),
                PlayerCmd::Play       => self.playing = true,
                PlayerCmd::Pause      => self.playing = false,
                PlayerCmd::Stop       => {
                    self.playing = false;
                    if let Some(d) = &mut self.deck { d.cursor = 0; }
                }
                PlayerCmd::Seek(s) => {
                    if let Some(d) = &mut self.deck { d.seek(s); }
                }
                PlayerCmd::SetVolume(v) => self.volume = v.clamp(0.0, 2.0),
            }
        }
    }

    /// Mix `n` decoded samples (interleaved, `channels` wide) into `block`.
    /// `block` is the same mono/stereo render buffer the synth fills; call
    /// AFTER `lane.render` so the player adds on top. Safe to call when not playing.
    pub fn mix_into_block(&mut self, block: &mut [f32], channels: usize) {
        if !self.playing { self.rms = 0.0; return; }
        let Some(deck) = &mut self.deck else { self.rms = 0.0; return };

        let n = block.len().min(deck.samples.len().saturating_sub(deck.cursor));
        let mut sq = 0.0f32;
        for i in 0..n {
            let src_ch  = deck.channels.max(1);
            let frame   = (deck.cursor + i) / channels.max(1);
            let out_c   = i % channels.max(1);
            let src_idx = frame * src_ch + out_c.min(src_ch - 1);
            let s = deck.samples.get(src_idx).copied().unwrap_or(0.0) * self.volume;
            sq += s * s;
            block[i] += s;
        }
        deck.cursor += n;
        self.rms = if n > 0 { (sq / n as f32).sqrt() } else { 0.0 };
        if deck.ended() { self.playing = false; }
    }

    fn do_load(&mut self, path: &str, out_channels: usize) {
        match load_and_prepare(path, out_channels) {
            Ok(deck) => {
                self.deck    = Some(deck);
                self.playing = false;
            }
            Err(e) => {
                self.snapshot.store(Arc::new(PlayerSnapshot {
                    load_error: Some(e),
                    ..PlayerSnapshot::default()
                }));
            }
        }
    }

    /// Publish the current state. Call once per frame after `pump`.
    pub fn publish(&self) {
        let (playing, loaded, ended) = match &self.deck {
            None    => (false, false, false),
            Some(d) => (self.playing, true, d.ended()),
        };
        let mut s = PlayerSnapshot {
            playing,
            loaded,
            track_ended:   ended,
            volume:        self.volume,
            underruns:     self.underruns,
            rms:           self.rms,
            peak_l:        self.rms,
            peak_r:        self.rms,
            ..PlayerSnapshot::default()
        };
        if let Some(d) = &self.deck {
            s.position_secs = d.position_secs();
            s.duration_secs = d.duration_secs();
            s.device_rate   = d.sample_rate;
            s.meta          = Some(d.meta.clone());
        }
        self.snapshot.store(Arc::new(s));
    }

    pub fn load_snapshot(&self) -> Arc<PlayerSnapshot> { self.snapshot.load_full() }
}

impl Default for Player {
    fn default() -> Self { Self::new() }
}

// ── Decode + optional channel adapt ──────────────────────────────────────────

fn load_and_prepare(path: &str, out_channels: usize) -> Result<Deck, String> {
    match crate::ingest::ingest_file(path) {
        Ok(crate::ingest::Ingested::Recorded { audio, meta }) => {
            let tm = TrackMeta {
                title:         meta.title,
                artist:        meta.artist,
                album:         meta.album,
                duration_secs: meta.duration_secs,
            };
            // audio.samples is Vec<Vec<f32>>: [channel][frame]
            let sr      = audio.sample_rate;
            let n_ch    = audio.samples.len().max(1);
            let n_fr    = audio.samples[0].len();
            let out_ch  = out_channels.max(1);
            // interleave + adapt channels
            let mut flat = Vec::with_capacity(n_fr * out_ch);
            for fr in 0..n_fr {
                for c in 0..out_ch {
                    let src_c = c.min(n_ch - 1);
                    flat.push(audio.samples[src_c][fr]);
                }
            }
            Ok(Deck { samples: flat, cursor: 0, sample_rate: sr, channels: out_ch, meta: tm })
        }
        Ok(crate::ingest::Ingested::Symbolic { .. }) => {
            Err("MIDI playback not yet wired — use an audio file".into())
        }
        Err(e) => Err(format!("{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_default_snapshot_clean() {
        let p = Player::new();
        let s = p.load_snapshot();
        assert!(!s.playing);
        assert!(!s.loaded);
        assert_eq!(s.volume, 1.0);
    }

    #[test]
    fn mix_no_panic_without_deck() {
        let mut p = Player::new();
        let mut block = vec![0.0f32; 64];
        p.mix_into_block(&mut block, 2); // must not panic
    }
}
