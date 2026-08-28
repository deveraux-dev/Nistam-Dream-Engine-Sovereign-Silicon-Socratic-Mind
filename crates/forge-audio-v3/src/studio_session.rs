//! RecordingSession — the state machine behind the Recording Studio panel.
//!
//! Owns the current take (mic-recorded, imported, or synth-generated), the
//! Deadpan Linter toggle, and the playhead. Firewall: forge-audio must NOT
//! depend on forge-vocal-corpus (which already depends on forge-audio — a
//! reverse edge would be a cycle), so corpus registration
//! (`forge_vocal_corpus::process_track`) happens one layer up, in forge-gui,
//! right after `save_wav`.

use crate::dsp::{self, AudioBuffer};
// MicCapture import EXCLUDED - crate::mic_capture needs Vec<f32>: ClockPlane, not implemented.
use crate::vocal_studio::{apply_deadpan, DeadpanParams};

/// Fixed session rate — matches `MicCapture`'s WASAPI decimation target, so a
/// mic take and a synth/import take always line up without a resample step.
pub const SESSION_SAMPLE_RATE: u32 = 16_000;

/// Default rumble-cut corner for the record-chain high-pass: kills mains hum,
/// handling thumps, and DC below ~80 Hz on a voice take. One-pole `dsp::highpass`.
pub const DEFAULT_HIGHPASS_CUTOFF_HZ: f32 = 80.0;

/// Where the current take's audio came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeSource {
    Mic,
    Imported,
    Synth,
}

/// The Recording Studio's session state: one take in flight, three sources,
/// one DSP toggle (Deadpan Linter — modulation rack follows later).
pub struct RecordingSession {
    pub buffer: Option<AudioBuffer>,
    pub source: Option<TakeSource>,
    pub deadpan_enabled: bool,
    pub deadpan_params: DeadpanParams,
    pub highpass_enabled: bool,
    pub highpass_cutoff_hz: f32,
    pub playhead: u64,
    pub is_playing: bool,
    // mic: Option<MicCapture> field EXCLUDED - see import note above.
}

impl Default for RecordingSession {
    fn default() -> Self { Self::new() }
}

impl RecordingSession {
    pub fn new() -> Self {
        Self {
            buffer: None,
            source: None,
            deadpan_enabled: false,
            deadpan_params: DeadpanParams::default(),
            highpass_enabled: false,
            highpass_cutoff_hz: DEFAULT_HIGHPASS_CUTOFF_HZ,
            playhead: 0,
            is_playing: false,
        }
    }

    // is_recording/start_recording: EXCLUDED - needs MicCapture (real API-shape gap: Vec<f32>: ClockPlane unimplemented).

    // stop_recording: EXCLUDED - needs MicCapture (see field note above).

    /// Import an existing audio file (or drag-drop drop target) as the current take.
    pub fn import_file(&mut self, path: &str) -> Result<(), String> {
        let ingested = crate::ingest::ingest_file(path).map_err(|e| format!("{path}: {e:?}"))?;
        let buf = match ingested {
            crate::ingest::Ingested::Recorded { audio, .. } => audio,
            crate::ingest::Ingested::Symbolic { .. } => {
                return Err(format!("{path}: MIDI/symbolic source, not a decodable audio take"));
            }
        };
        self.buffer = Some(buf);
        self.source = Some(TakeSource::Imported);
        self.playhead = 0;
        Ok(())
    }

    /// Generate a primitive sine-wave take — the synth source, for music/SFX
    /// assets (not a recorded voice).
    pub fn generate_synth(&mut self, freq_hz: f32, duration_secs: f32) {
        let n = (SESSION_SAMPLE_RATE as f32 * duration_secs).max(1.0) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq_hz * i as f32 / SESSION_SAMPLE_RATE as f32).sin() * 0.5)
            .collect();
        self.buffer = Some(AudioBuffer { samples: vec![samples], sample_rate: SESSION_SAMPLE_RATE });
        self.source = Some(TakeSource::Synth);
        self.playhead = 0;
    }

    // generate_vocal: EXCLUDED - needs vocal_studio::vocal_synth (needs forge_calligraphy, absent).

    pub fn toggle_deadpan(&mut self) { self.deadpan_enabled = !self.deadpan_enabled; }

    pub fn toggle_highpass(&mut self) { self.highpass_enabled = !self.highpass_enabled; }

    /// The take as it would be played/saved. Record-chain order: raw → high-pass
    /// rumble-cut → Deadpan Linter, each stage applied only when its toggle is on.
    pub fn processed_take(&self) -> Option<AudioBuffer> {
        let mut buf = self.buffer.clone()?;
        if self.highpass_enabled {
            buf = dsp::highpass(buf, self.highpass_cutoff_hz);
        }
        if self.deadpan_enabled {
            buf = apply_deadpan(buf, &self.deadpan_params);
        }
        Some(buf)
    }

    pub fn play(&mut self) -> Result<(), String> {
        if self.buffer.is_none() {
            return Err("no take loaded".to_string());
        }
        self.is_playing = true;
        Ok(())
    }

    pub fn pause(&mut self) { self.is_playing = false; }

    /// Seek the playhead to a fraction (0.0..=1.0) of the take's length — the scrub bar.
    pub fn seek(&mut self, fraction: f32) {
        let Some(buf) = &self.buffer else { return };
        let len = buf.len() as f64;
        self.playhead = (fraction.clamp(0.0, 1.0) as f64 * len) as u64;
    }

    pub fn duration_secs(&self) -> f32 {
        self.buffer.as_ref().map(AudioBuffer::duration_secs).unwrap_or(0.0)
    }

    pub fn playhead_secs(&self) -> f32 {
        self.buffer.as_ref().map(|b| self.playhead as f32 / b.sample_rate as f32).unwrap_or(0.0)
    }

    /// Save the processed take to a WAV file on disk. Corpus registration is
    /// the caller's job (see module doc — firewall).
    pub fn save_wav(&self, path: &str) -> Result<(), String> {
        let buf = self.processed_take().ok_or_else(|| "no take loaded".to_string())?;
        dsp::write_wav(path, &buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_empty_and_stopped() {
        let s = RecordingSession::new();
        assert!(s.buffer.is_none());
        // is_recording() call EXCLUDED - needs MicCapture (excluded).
        assert!(!s.is_playing);
        assert_eq!(s.duration_secs(), 0.0);
    }

    #[test]
    fn generate_synth_loads_a_take() {
        let mut s = RecordingSession::new();
        s.generate_synth(220.0, 0.5);
        assert_eq!(s.source, Some(TakeSource::Synth));
        assert!(s.buffer.is_some());
        assert!((s.duration_secs() - 0.5).abs() < 0.01);
    }

    #[test]
    fn play_without_a_take_is_an_honest_error() {
        let mut s = RecordingSession::new();
        assert!(s.play().is_err());
    }

    #[test]
    fn play_pause_toggles_state() {
        let mut s = RecordingSession::new();
        s.generate_synth(220.0, 0.2);
        assert!(s.play().is_ok());
        assert!(s.is_playing);
        s.pause();
        assert!(!s.is_playing);
    }

    #[test]
    fn seek_clamps_into_range() {
        let mut s = RecordingSession::new();
        s.generate_synth(220.0, 1.0);
        let len = s.buffer.as_ref().unwrap().len() as u64;
        s.seek(0.5);
        assert!(s.playhead > 0 && s.playhead <= len);
        s.seek(2.0); // out of range, must clamp
        assert_eq!(s.playhead, len);
        s.seek(-1.0);
        assert_eq!(s.playhead, 0);
    }

    #[test]
    fn deadpan_toggle_changes_processed_output() {
        let mut s = RecordingSession::new();
        s.generate_synth(220.0, 0.5);
        let raw = s.processed_take().unwrap();
        s.toggle_deadpan();
        assert!(s.deadpan_enabled);
        let processed = s.processed_take().unwrap();
        assert_eq!(raw.len(), processed.len());
        assert_ne!(raw.samples[0], processed.samples[0], "Deadpan should audibly change the take");
    }

    #[test]
    fn highpass_toggle_attenuates_sub_corner_tone() {
        // 40 Hz sits well below the 80 Hz corner — the one-pole high-pass must
        // audibly pull its level down when engaged (record-chain rumble cut).
        let mut s = RecordingSession::new();
        s.generate_synth(40.0, 0.5);
        let raw = s.processed_take().unwrap();
        s.toggle_highpass();
        assert!(s.highpass_enabled);
        let filtered = s.processed_take().unwrap();
        assert_eq!(raw.len(), filtered.len(), "high-pass must not change take length");
        let peak = |b: &AudioBuffer| b.samples[0].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            peak(&filtered) < peak(&raw),
            "high-pass must attenuate a 40Hz tone below the 80Hz corner (raw {}, hp {})",
            peak(&raw), peak(&filtered)
        );
    }

    #[test]
    fn highpass_and_deadpan_stack_in_the_chain() {
        // Both stages on: output differs from raw and from either stage alone —
        // proves processed_take() actually threads the buffer through both.
        let mut s = RecordingSession::new();
        s.generate_synth(40.0, 0.5);
        let raw = s.processed_take().unwrap();
        s.toggle_highpass();
        s.toggle_deadpan();
        let both = s.processed_take().unwrap();
        assert_eq!(raw.len(), both.len());
        assert_ne!(raw.samples[0][10], both.samples[0][10], "chain with both FX on must alter the take");
    }

    // generate_vocal_plays_into_deadpan test EXCLUDED - needs generate_vocal (excluded, vocal_synth needs forge_calligraphy).

    // stop_recording_without_start_is_an_honest_error test EXCLUDED - needs stop_recording (excluded).

    #[test]
    fn import_missing_file_is_an_honest_error() {
        let mut s = RecordingSession::new();
        assert!(s.import_file("F:/NewRepo/does_not_exist_deadpan_test.wav").is_err());
    }

    #[test]
    fn save_without_a_take_is_an_honest_error() {
        let s = RecordingSession::new();
        assert!(s.save_wav("F:/NewRepo/.forge/_scratch/should_not_be_written.wav").is_err());
    }

    #[test]
    fn save_writes_a_real_wav_file() {
        let dir = std::env::temp_dir().join("forge_audio_studio_session_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("take.wav");
        let path_str = path.to_str().unwrap();

        let mut s = RecordingSession::new();
        s.generate_synth(220.0, 0.3);
        s.save_wav(path_str).expect("save_wav should succeed with a loaded take");
        assert!(path.exists());
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 44, "WAV file should contain real audio data, not just a header");

        let _ = std::fs::remove_file(&path);
    }
}