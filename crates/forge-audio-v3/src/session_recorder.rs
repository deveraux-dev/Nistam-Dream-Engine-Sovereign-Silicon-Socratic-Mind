//! Session Recorder — NDJSON event log with crash-safe, file-based recording.
//!
//! Each line is a self-contained JSON object. Truncation at any newline boundary
//! leaves a valid partial session file. The first line is always a `SessionHeader`.
//!
//! Ported from dreadpirateradio quarry (E:/airgap/13forge/dreadpiratedev);
//! EventSource variants adapted to forge-audio vocabulary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

/// Source of the recorded event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    Mixer,
    Deck,
    Controller,
    Input,
    Network,
    Visual,
}

/// A single recorded event (one NDJSON line after the header).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Monotonically non-decreasing sample-clock timestamp.
    pub t: u64,
    /// Which input source produced this event.
    pub src: EventSource,
    /// Serialised command payload (self-contained).
    pub cmd: serde_json::Value,
}

/// Reference to a track used during the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackRef {
    /// Path relative to library root.
    pub path: String,
    /// Hex-encoded SHA-256 of the file contents.
    pub sha256: String,
}

/// First NDJSON line written to every session file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionHeader {
    pub version: u32,
    pub sample_rate: u32,
    pub created: String,
    pub library_root: String,
    pub tracks_used: Vec<TrackRef>,
}

/// File-based NDJSON session recorder.
///
/// * `new()` opens the file and writes the header as the first line.
/// * `record()` appends one event per line with the current sample clock.
/// * `advance_clock()` advances the monotonic sample clock.
/// * `flush()` flushes the buffered writer (~every 1 second).
///
/// Crash safety: every `\n`-terminated line is a complete JSON object.
/// On disk-full the recorder marks itself stopped and returns DiskFull.
pub struct SessionRecorder {
    writer: BufWriter<File>,
    sample_clock: u64,
    last_written_t: u64,
    last_flush: Instant,
    stopped: bool,
}

/// Errors surfaced by the recorder.
#[derive(Debug)]
pub enum RecordError {
    Io(std::io::Error),
    Serialize(serde_json::Error),
    DiskFull(std::io::Error),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "session I/O error: {e}"),
            Self::Serialize(e) => write!(f, "session serialize error: {e}"),
            Self::DiskFull(e) => write!(f, "disk full — recording stopped: {e}"),
        }
    }
}

impl SessionRecorder {
    /// Open (or create) the session file and write the header as the first NDJSON line.
    pub fn new(path: &str, header: SessionHeader) -> Result<Self, RecordError> {
        let file = File::create(path).map_err(RecordError::Io)?;
        let mut writer = BufWriter::new(file);
        let header_json = serde_json::to_string(&header).map_err(RecordError::Serialize)?;
        writer.write_all(header_json.as_bytes()).map_err(RecordError::Io)?;
        writer.write_all(b"\n").map_err(RecordError::Io)?;
        writer.flush().map_err(RecordError::Io)?;
        Ok(Self {
            writer,
            sample_clock: 0,
            last_written_t: 0,
            last_flush: Instant::now(),
            stopped: false,
        })
    }

    /// Record an event. Timestamp is guaranteed monotonically non-decreasing.
    /// On disk-full the recorder stops itself and returns `RecordError::DiskFull`.
    pub fn record(
        &mut self,
        source: EventSource,
        cmd: &impl Serialize,
    ) -> Result<(), RecordError> {
        if self.stopped { return Ok(()); }
        let t = self.sample_clock.max(self.last_written_t);
        let event = SessionEvent {
            t,
            src: source,
            cmd: serde_json::to_value(cmd).map_err(RecordError::Serialize)?,
        };
        let line = serde_json::to_string(&event).map_err(RecordError::Serialize)?;
        if let Err(e) = self.writer.write_all(line.as_bytes()) {
            self.stopped = true;
            return Err(RecordError::DiskFull(e));
        }
        if let Err(e) = self.writer.write_all(b"\n") {
            self.stopped = true;
            return Err(RecordError::DiskFull(e));
        }
        self.last_written_t = t;
        if self.last_flush.elapsed().as_secs() >= 1 {
            let _ = self.flush();
        }
        Ok(())
    }

    /// Advance the sample clock by `frames` samples.
    pub fn advance_clock(&mut self, frames: usize) {
        self.sample_clock = self.sample_clock.saturating_add(frames as u64);
    }

    /// Flush the buffered writer to disk.
    pub fn flush(&mut self) -> Result<(), RecordError> {
        if self.stopped { return Ok(()); }
        self.writer.flush().map_err(|e| {
            self.stopped = true;
            RecordError::DiskFull(e)
        })?;
        self.last_flush = Instant::now();
        Ok(())
    }

    pub fn is_stopped(&self) -> bool { self.stopped }
    pub fn sample_clock(&self) -> u64 { self.sample_clock }
}

/// Compute the SHA-256 hex digest of a file at `path`.
pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let data = std::fs::read(path)?;
    let hash = Sha256::digest(&data);
    Ok(format!("{:x}", hash))
}

/// Build a `TrackRef` from a file path relative to `library_root`.
pub fn track_ref(library_root: &Path, absolute_path: &Path) -> Result<TrackRef, std::io::Error> {
    let rel = absolute_path
        .strip_prefix(library_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let sha = sha256_file(absolute_path)?;
    Ok(TrackRef {
        path: rel.to_string_lossy().into_owned(),
        sha256: sha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn test_header() -> SessionHeader {
        SessionHeader {
            version: 1,
            sample_rate: 44100,
            created: "2025-01-15T12:00:00Z".to_string(),
            library_root: "/music".to_string(),
            tracks_used: vec![TrackRef {
                path: "track_a.flac".to_string(),
                sha256: "abcd1234".to_string(),
            }],
        }
    }

    #[test]
    fn session_event_round_trip() {
        let event = SessionEvent {
            t: 44100,
            src: EventSource::Mixer,
            cmd: serde_json::json!({"Play": 0}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: SessionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn all_event_sources_round_trip() {
        for src in [
            EventSource::Mixer,
            EventSource::Deck,
            EventSource::Controller,
            EventSource::Input,
            EventSource::Network,
            EventSource::Visual,
        ] {
            let event = SessionEvent { t: 0, src: src.clone(), cmd: serde_json::json!(null) };
            let json = serde_json::to_string(&event).unwrap();
            let back: SessionEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event.src, back.src);
        }
    }

    #[test]
    fn header_round_trip() {
        let h = test_header();
        let json = serde_json::to_string(&h).unwrap();
        let back: SessionHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }

    #[test]
    fn timestamps_are_monotonic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.ndjson");
        let mut rec = SessionRecorder::new(path.to_str().unwrap(), test_header()).unwrap();
        rec.advance_clock(100);
        rec.record(EventSource::Mixer, &serde_json::json!("a")).unwrap();
        rec.record(EventSource::Deck, &serde_json::json!("b")).unwrap();
        rec.advance_clock(200);
        rec.record(EventSource::Controller, &serde_json::json!("c")).unwrap();
        rec.flush().unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let mut prev_t: Option<u64> = None;
        for (i, line) in contents.lines().enumerate() {
            if i == 0 { let _h: SessionHeader = serde_json::from_str(line).unwrap(); continue; }
            let ev: SessionEvent = serde_json::from_str(line).unwrap();
            if let Some(pt) = prev_t {
                assert!(ev.t >= pt, "timestamp went backwards: {} < {} at line {}", ev.t, pt, i);
            }
            prev_t = Some(ev.t);
        }
    }

    #[test]
    fn crash_safe_truncation_at_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.ndjson");
        let mut rec = SessionRecorder::new(path.to_str().unwrap(), test_header()).unwrap();
        for i in 0u64..5 {
            rec.advance_clock(1000);
            rec.record(EventSource::Mixer, &serde_json::json!({"i": i})).unwrap();
        }
        rec.flush().unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let mut offset = 0usize;
        for line in contents.lines() {
            offset += line.len() + 1;
            let truncated = &contents[..offset];
            for (j, tl) in truncated.lines().enumerate() {
                if j == 0 {
                    assert!(serde_json::from_str::<SessionHeader>(tl).is_ok());
                } else {
                    assert!(serde_json::from_str::<SessionEvent>(tl).is_ok());
                }
            }
        }
    }

    #[test]
    fn track_load_event_contains_path_and_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.ndjson");
        let track_path = dir.path().join("track.flac");
        std::fs::write(&track_path, b"fake audio data").unwrap();
        let tref = track_ref(dir.path(), &track_path).unwrap();
        assert_eq!(tref.path, "track.flac");
        assert!(!tref.sha256.is_empty());
        let header = SessionHeader {
            version: 1,
            sample_rate: 44100,
            created: "2025-01-15T12:00:00Z".to_string(),
            library_root: dir.path().to_string_lossy().into_owned(),
            tracks_used: vec![tref.clone()],
        };
        let mut rec = SessionRecorder::new(path.to_str().unwrap(), header).unwrap();
        rec.record(EventSource::Deck, &serde_json::json!({
            "LoadDeck": { "deck": 0, "path": tref.path, "sha256": tref.sha256 }
        })).unwrap();
        rec.flush().unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let event_line = contents.lines().nth(1).unwrap();
        let ev: SessionEvent = serde_json::from_str(event_line).unwrap();
        let load = ev.cmd.as_object().unwrap().get("LoadDeck").unwrap();
        assert!(load.get("path").is_some());
        assert!(load.get("sha256").is_some());
    }

    fn arb_event_source() -> impl Strategy<Value = EventSource> {
        prop_oneof![
            Just(EventSource::Mixer),
            Just(EventSource::Deck),
            Just(EventSource::Controller),
            Just(EventSource::Input),
            Just(EventSource::Network),
            Just(EventSource::Visual),
        ]
    }

    fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            Just(serde_json::Value::Null),
            any::<bool>().prop_map(serde_json::Value::Bool),
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            (-1_000_000i64..1_000_000i64).prop_map(|n| serde_json::json!(n)),
            "[a-zA-Z0-9_ ]{0,32}".prop_map(|s| serde_json::Value::String(s)),
        ]
    }

    fn arb_session_event() -> impl Strategy<Value = SessionEvent> {
        (any::<u64>(), arb_event_source(), arb_json_value()).prop_map(|(t, src, cmd)| {
            SessionEvent { t, src, cmd }
        })
    }

    proptest! {
        #[test]
        fn prop_session_event_round_trip(event in arb_session_event()) {
            let json = serde_json::to_string(&event).unwrap();
            let back: SessionEvent = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(event, back);
        }
    }
}
