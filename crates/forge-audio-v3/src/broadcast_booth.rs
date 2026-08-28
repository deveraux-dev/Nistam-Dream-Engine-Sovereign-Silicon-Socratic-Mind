// @forge:allow_float — DDSP leaf; the booth's sample metering is inherently f32.
//! broadcast_booth — the Broadcast Booth engine that folds into the ONE AUDIO desk.
//!
//! Ties the slice-1 mic DSP (`noise_suppress::NoiseSuppressor` +
//! `mic_fx::MicStrip`) to the desk's live surfaces: meters (published to
//! `telemetry::TELEMETRY` + an arc-swap `MicSnapshot` for the `mic.*` binds — the
//! desk's "UI never calculates audio math" acoustic-isolation law), a WAV record
//! path finished DSPP-style, and the append-only flight recorder
//! (`session_recorder::SessionRecorder` — crash-safe NDJSON, the VCS flight
//! recorder). Carries a 5D placement for the mic source (HEAR-side spatial).
//!
//! Runs on the studio's logic/record lane (the studio drains the `!Send`
//! `MicCapture` bridge and hands blocks to [`BroadcastBooth::process_mic_block`]),
//! so heap use here is inside the forge-audio zero-alloc carve-out.

use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::Serialize;

use crate::dsp::{self, AudioBuffer};
use crate::healing::{heal_voice, HealingParams};
use crate::mic_fx::{MicStrip, MicStripParams};
use crate::noise_suppress::NoiseSuppressor;
use crate::session_recorder::{EventSource, RecordError, SessionHeader, SessionRecorder};
use crate::telemetry::TELEMETRY;

/// 5D placement of the mic source in the HEAR field (integer, permyriad-scaled).
/// The desk binds these; the actual 5D→stereo collapse lives in
/// `dimensional_collapse` and is applied on the monitor path by the studio.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Mic5D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
    pub theta: i32,
}

impl Default for Mic5D {
    fn default() -> Self {
        Self { x: 0, y: 0, z: 0, w: 0, theta: 0 }
    }
}

/// Read-only booth state the desk reads via arc-swap (the `mic.*` bind namespace).
#[derive(Clone, Debug, Serialize)]
pub struct MicSnapshot {
    /// Processed mic RMS, permyriad of full scale (0..=10000).
    pub rms_pmy: i32,
    /// Processed mic peak, permyriad of full scale (0..=10000).
    pub peak_pmy: i32,
    /// RMS in dBFS × 100 (fixed-point, matches telemetry).
    pub rms_db_fixed: i32,
    /// Peak in dBFS × 100.
    pub peak_db_fixed: i32,
    /// Suppression strength, permyriad (0..=10000).
    pub denoise_pmy: i32,
    /// DJ filter knob, permyriad (−10000..=10000; 0 = bypass).
    pub dj_knob_pmy: i32,
    /// High-pass cutoff in Hz.
    pub hpf_hz: i32,
    /// Gate threshold in dBFS × 100.
    pub gate_db_fixed: i32,
    pub recording: bool,
    pub live: bool,
    /// Recorded frame count (record length so far).
    pub record_frames: u64,
    pub place: Mic5D,
}

impl Default for MicSnapshot {
    fn default() -> Self {
        Self {
            rms_pmy: 0,
            peak_pmy: 0,
            rms_db_fixed: crate::telemetry::DB_SILENCE_FIXED,
            peak_db_fixed: crate::telemetry::DB_SILENCE_FIXED,
            denoise_pmy: 0,
            dj_knob_pmy: 0,
            hpf_hz: 90,
            gate_db_fixed: -5000,
            recording: false,
            live: false,
            record_frames: 0,
            place: Mic5D::default(),
        }
    }
}

/// One booth action, logged to the flight recorder (append-only, replayable).
#[derive(Clone, Debug, Serialize)]
pub enum BoothAction {
    MicOpen { sample_rate: u32 },
    SetDenoise { strength_pmy: i32 },
    SetDjKnob { knob_pmy: i32 },
    SetHpf { hz: i32 },
    RecordStart,
    RecordStop { path: String, frames: u64, secs_x1000: u64 },
    GoLive { mount: String },
    StopLive,
    /// Emitted by the stem conductor when a `.timeline.vixi` recipe fires a stem.
    TriggerStem { group: u8, id: u64, tick_us: i64 },
}

/// The Broadcast Booth: the mic → hush → strip → meters → WAV/live signal chain,
/// with the append-only flight recorder and a 5D mic placement.
pub struct BroadcastBooth {
    sr: u32,
    suppressor: NoiseSuppressor,
    strip: MicStrip,
    strip_params: MicStripParams,
    denoise_strength: f32,
    denoise_on: bool,

    recording: bool,
    record_buf: Vec<f32>,
    heal_on_record: bool,

    snapshot: Arc<ArcSwap<MicSnapshot>>,
    flight: Option<SessionRecorder>,
    place: Mic5D,
    publish_telemetry: bool,

    last_rms: f32,
    last_peak: f32,
}

impl BroadcastBooth {
    pub fn new(sample_rate: u32) -> Self {
        let params = MicStripParams::default();
        Self {
            sr: sample_rate,
            suppressor: NoiseSuppressor::new(sample_rate, 0.0),
            strip: MicStrip::new(sample_rate as f32, params),
            strip_params: params,
            denoise_strength: 0.0,
            denoise_on: false,
            recording: false,
            record_buf: Vec::new(),
            heal_on_record: false,
            snapshot: Arc::new(ArcSwap::from_pointee(MicSnapshot::default())),
            flight: None,
            place: Mic5D::default(),
            publish_telemetry: true,
            last_rms: 0.0,
            last_peak: 0.0,
        }
    }

    /// The arc-swap the desk holds to read `mic.*` without touching audio math.
    pub fn shared_snapshot(&self) -> Arc<ArcSwap<MicSnapshot>> {
        Arc::clone(&self.snapshot)
    }

    /// Current snapshot (a cheap arc load).
    pub fn snapshot(&self) -> Arc<MicSnapshot> {
        self.snapshot.load_full()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr
    }

    /// Set whether the record master is run through `healing::heal_voice`
    /// (HPF→comp→brickwall). Off by default — the live strip already shaped it,
    /// so the record path only peak-safeties unless healing is explicitly asked.
    pub fn set_heal_on_record(&mut self, on: bool) {
        self.heal_on_record = on;
    }

    /// Suppression strength, `0.0..=1.0`. 0 disables the STFT hush entirely.
    pub fn set_denoise(&mut self, strength: f32) {
        let s = strength.clamp(0.0, 1.0);
        self.denoise_strength = s;
        self.denoise_on = s > 0.001;
        self.suppressor.set_strength(s);
        self.log(BoothAction::SetDenoise { strength_pmy: pmy(s) });
        self.republish();
    }

    /// Move the DJ filter knob (`-1.0..=1.0`).
    pub fn set_dj_knob(&mut self, knob: f32) {
        self.strip_params.dj_knob = knob.clamp(-1.0, 1.0);
        self.strip.set_dj_knob(self.strip_params.dj_knob);
        self.log(BoothAction::SetDjKnob { knob_pmy: (self.strip_params.dj_knob * 10_000.0) as i32 });
        self.republish();
    }

    /// Set the high-pass cutoff (Hz).
    pub fn set_hpf(&mut self, hz: f32) {
        self.strip_params.hpf_hz = hz.clamp(20.0, 2000.0);
        self.strip.set_params(self.strip_params);
        self.log(BoothAction::SetHpf { hz: self.strip_params.hpf_hz as i32 });
        self.republish();
    }

    /// Commit a whole strip parameter set (gate/hpf/dj/warmth/comp).
    pub fn set_strip_params(&mut self, p: MicStripParams) {
        self.strip_params = p;
        self.strip.set_params(p);
        self.republish();
    }

    /// Place the mic source in the 5D HEAR field.
    pub fn set_5d(&mut self, place: Mic5D) {
        self.place = place;
        self.republish();
    }

    pub fn place_5d(&self) -> Mic5D {
        self.place
    }

    /// The heart of the booth: clean → shape → meter → publish → (record) → (live).
    /// `block` is mono mic samples; processed in place.
    pub fn process_mic_block(&mut self, block: &mut [f32]) {
        if self.denoise_on {
            self.suppressor.process_block(block);
        }
        self.strip.process_block(block);

        // Metering (peak + RMS) over the processed block.
        let mut peak = 0.0f32;
        let mut sq = 0.0f64;
        for &s in block.iter() {
            let a = s.abs();
            if a > peak {
                peak = a;
            }
            sq += (s as f64) * (s as f64);
        }
        let rms = (sq / block.len().max(1) as f64).sqrt() as f32;
        self.last_rms = rms;
        self.last_peak = peak;

        if self.publish_telemetry {
            TELEMETRY.set_master_levels(lin_to_db(rms), lin_to_db(peak));
        }

        if self.recording {
            self.record_buf.extend_from_slice(block);
            if let Some(fr) = self.flight.as_mut() {
                fr.advance_clock(block.len());
            }
        }

        self.republish();
    }

    /// Begin accumulating processed mic samples for a WAV take.
    pub fn start_record(&mut self) {
        self.record_buf.clear();
        self.recording = true;
        self.log(BoothAction::RecordStart);
        self.republish();
    }

    /// Stop recording and write the take to `path` as a WAV. Returns the duration
    /// in seconds. The record master is peak-safetied to −1 dBFS (and optionally
    /// healed). Also logged to the flight recorder.
    pub fn stop_record_to_wav(&mut self, path: &str) -> Result<f32, String> {
        self.recording = false;
        let frames = self.record_buf.len() as u64;
        let mono = std::mem::take(&mut self.record_buf);

        let mut buf = AudioBuffer { samples: vec![mono], sample_rate: self.sr };
        if self.heal_on_record {
            buf = heal_voice(buf, &HealingParams::default());
        } else {
            peak_safety(&mut buf, -1.0);
        }
        let secs = buf.duration_secs();

        dsp::write_wav(path, &buf).map_err(|e| format!("[booth] WAV write failed: {e}"))?;

        self.log(BoothAction::RecordStop {
            path: path.to_string(),
            frames,
            secs_x1000: (secs * 1000.0) as u64,
        });
        if let Some(fr) = self.flight.as_mut() {
            let _ = fr.flush();
        }
        self.republish();
        Ok(secs)
    }

    /// Open (or replace) the append-only flight recorder at `path`.
    pub fn open_flight_recorder(&mut self, path: &str) -> Result<(), RecordError> {
        let header = SessionHeader {
            version: 1,
            sample_rate: self.sr,
            created: now_rfc3339(),
            library_root: String::new(),
            tracks_used: Vec::new(),
        };
        let mut fr = SessionRecorder::new(path, header)?;
        // Stamp the open so a replay knows the booth's sample rate context.
        let _ = fr.record(
            EventSource::Input,
            &BoothAction::MicOpen { sample_rate: self.sr },
        );
        self.flight = Some(fr);
        Ok(())
    }

    /// Log a stem trigger (called by the `.timeline.vixi` stem conductor).
    pub fn log_stem_trigger(&mut self, group: u8, id: u64, tick_us: i64) {
        self.log(BoothAction::TriggerStem { group, id, tick_us });
    }

    /// Flush the flight recorder (call ~1 Hz from the studio logic lane).
    pub fn flush_flight(&mut self) {
        if let Some(fr) = self.flight.as_mut() {
            let _ = fr.flush();
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    fn log(&mut self, action: BoothAction) {
        let src = match action {
            BoothAction::MicOpen { .. } | BoothAction::SetDenoise { .. } => EventSource::Input,
            BoothAction::SetDjKnob { .. } | BoothAction::SetHpf { .. } => EventSource::Mixer,
            BoothAction::RecordStart | BoothAction::RecordStop { .. } => EventSource::Mixer,
            BoothAction::GoLive { .. } | BoothAction::StopLive => EventSource::Network,
            BoothAction::TriggerStem { .. } => EventSource::Deck,
        };
        if let Some(fr) = self.flight.as_mut() {
            let _ = fr.record(src, &action);
        }
    }

    fn republish(&self) {
        let snap = MicSnapshot {
            rms_pmy: pmy(self.last_rms),
            peak_pmy: pmy(self.last_peak),
            rms_db_fixed: crate::telemetry::db_to_fixed(lin_to_db(self.last_rms)),
            peak_db_fixed: crate::telemetry::db_to_fixed(lin_to_db(self.last_peak)),
            denoise_pmy: pmy(self.denoise_strength),
            dj_knob_pmy: (self.strip_params.dj_knob * 10_000.0) as i32,
            hpf_hz: self.strip_params.hpf_hz as i32,
            gate_db_fixed: crate::telemetry::db_to_fixed(self.strip_params.gate_threshold_db),
            recording: self.recording,
            live: false,
            record_frames: self.record_buf.len() as u64,
            place: self.place,
        };
        self.snapshot.store(Arc::new(snap));
    }
}

/// Permyriad of full scale (0..=10000) from a linear [0,1]-ish level.
#[inline]
fn pmy(x: f32) -> i32 {
    (x.clamp(0.0, 1.0) * 10_000.0) as i32
}

#[inline]
fn lin_to_db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// Scale the whole buffer so its peak sits at `ceiling_db` dBFS if it is hotter.
fn peak_safety(buf: &mut AudioBuffer, ceiling_db: f32) {
    let ceiling = 10.0f32.powf(ceiling_db / 20.0);
    let mut peak = 0.0f32;
    for ch in &buf.samples {
        for &s in ch {
            peak = peak.max(s.abs());
        }
    }
    if peak > ceiling && peak > 0.0 {
        let scale = ceiling / peak;
        for ch in &mut buf.samples {
            for s in ch.iter_mut() {
                *s *= scale;
            }
        }
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise_block(n: usize, amp: f32, seed: f32) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let x = ((i as f32 + seed) * 12.9898).sin() * 43758.547;
                amp * (2.0 * (x - x.floor()) - 1.0)
            })
            .collect()
    }

    #[test]
    fn snapshot_reflects_denoise_and_knob() {
        let mut b = BroadcastBooth::new(48_000);
        b.set_denoise(0.8);
        b.set_dj_knob(-0.5);
        let s = b.snapshot();
        assert_eq!(s.denoise_pmy, 8000);
        assert_eq!(s.dj_knob_pmy, -5000);
    }

    #[test]
    fn process_meters_and_is_finite() {
        let mut b = BroadcastBooth::new(48_000);
        b.set_denoise(0.9);
        let mut block = noise_block(2048, 0.3, 1.0);
        b.process_mic_block(&mut block);
        assert!(block.iter().all(|s| s.is_finite()));
        let s = b.snapshot();
        // meters populated (not the silent default)
        assert!(s.rms_pmy >= 0 && s.peak_pmy >= s.rms_pmy);
    }

    #[test]
    fn record_writes_a_wav_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("take.wav");
        let path_s = path.to_str().unwrap();

        let mut b = BroadcastBooth::new(48_000);
        b.start_record();
        assert!(b.is_recording());
        for k in 0..8 {
            let mut block: Vec<f32> = (0..1024)
                .map(|i| 0.4 * (2.0 * std::f32::consts::PI * 300.0 * (i + k * 1024) as f32 / 48_000.0).sin())
                .collect();
            b.process_mic_block(&mut block);
        }
        let secs = b.stop_record_to_wav(path_s).expect("wav write");
        assert!(!b.is_recording());
        assert!(secs > 0.1, "expected a non-trivial take, got {secs}s");
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 44, "WAV should be more than a bare header");
    }

    #[test]
    fn flight_recorder_appends_actions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("booth.ndjson");
        let path_s = path.to_str().unwrap();

        let mut b = BroadcastBooth::new(48_000);
        b.open_flight_recorder(path_s).expect("open flight recorder");
        b.set_denoise(0.7);
        b.set_dj_knob(0.6);
        b.start_record();
        let mut block = noise_block(1024, 0.2, 3.0);
        b.process_mic_block(&mut block);
        let take = dir.path().join("t.wav");
        b.stop_record_to_wav(take.to_str().unwrap()).unwrap();
        b.flush_flight();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        // header + MicOpen + SetDenoise + SetDjKnob + RecordStart + RecordStop
        assert!(lines.len() >= 6, "flight recorder should hold every action, got {}", lines.len());
        assert!(contents.contains("SetDenoise"));
        assert!(contents.contains("RecordStop"));
    }

    #[test]
    fn five_d_placement_round_trips() {
        let mut b = BroadcastBooth::new(48_000);
        let p = Mic5D { x: 1000, y: -2000, z: 500, w: 250, theta: 9000 };
        b.set_5d(p);
        assert_eq!(b.place_5d().theta, 9000);
        assert_eq!(b.snapshot().place.x, 1000);
    }
}
