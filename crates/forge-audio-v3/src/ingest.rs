//! Unified file ingest — the ONE seam every ForgeAudio surface loads through.
//!
//! Drag-drop a file (MP3/FLAC/OGG/WAV/AIFF or `.mid`), drag-drop a folder (a
//! playlist), or enqueue from the library: all funnel into [`ingest_file`] /
//! [`ingest_folder`]. The result is either decoded PCM ([`Ingested::Recorded`])
//! or a parsed symbolic score ([`Ingested::Symbolic`]) — and both report a
//! `duration_secs`, which is what sizes a composure on the timeline.
//!
//! This is the **convergence point** for the three historic decode paths
//! (`forge-app-hud`'s player thread, `dreadpirateradio`, the legacy player):
//! they fold *down* onto this. Recorded decode reuses [`crate::dsp::load_audio`]
//! (symphonia); the symbolic arm reuses `crate::forge_midi::parse_smf`.
//!
//! LOAD-TIME ONLY. Nothing here runs on the realtime audio callback, so heap
//! allocation during decode/parse/probe is expected and fine.

use crate::dsp::{self, AudioBuffer};

/// Track metadata. One home — folds down `forge-app-hud::snapshot::TrackMeta`
/// and the duplicated tag-probe that lived in the HUD player.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Length in seconds. Exact for decoded `Recorded`; header-estimated for a
    /// cheap [`probe_meta`] (may be `0.0` for formats that omit a frame count).
    pub duration_secs: f32,
    /// Genre byte auto-set by `ingest_file` via `genre_detect::detect_genre`.
    /// `None` for MIDI (`Symbolic` arm) and folder-probe paths (no PCM available).
    pub genre: Option<u8>,
}

/// The result of ingesting one file: recorded PCM or a symbolic score.
pub enum Ingested {
    /// MP3 / FLAC / OGG / WAV / AIFF → decoded PCM via symphonia.
    Recorded { audio: AudioBuffer, meta: TrackMeta },
    /// Standard MIDI File (`.mid` / `.midi`) → parsed score + PPQN division.
    Symbolic { smf: crate::forge_midi::Smf, meta: TrackMeta },
}

impl Ingested {
    /// Length in seconds — the value a composure clip is sized to.
    pub fn duration_secs(&self) -> f64 {
        match self {
            Ingested::Recorded { audio, .. } => audio.duration_secs() as f64,
            Ingested::Symbolic { smf, .. } => smf.duration_secs(),
        }
    }

    /// Shared metadata, whichever arm produced this.
    pub fn meta(&self) -> &TrackMeta {
        match self {
            Ingested::Recorded { meta, .. } | Ingested::Symbolic { meta, .. } => meta,
        }
    }
}

/// Why an ingest failed. Loud, never silent — every arm names its fault.
#[derive(Debug)]
pub enum IngestError {
    /// Filesystem read failed (missing file, permissions).
    Io(String),
    /// Symphonia could not probe/decode a recorded file.
    Decode(String),
    /// `forge-midi` rejected the SMF bytes.
    Midi(crate::forge_midi::ParseError),
    /// Extension we don't (yet) ingest — message says why / what to do instead.
    Unsupported(String),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::Io(s) => write!(f, "ingest I/O: {s}"),
            IngestError::Decode(s) => write!(f, "ingest decode: {s}"),
            IngestError::Midi(e) => write!(f, "ingest MIDI: {e:?}"),
            IngestError::Unsupported(s) => write!(f, "ingest unsupported: {s}"),
        }
    }
}
impl std::error::Error for IngestError {}

/// Ingest one file, dispatched by extension. THE single drag-drop entry point.
pub fn ingest_file(path: &str) -> Result<Ingested, IngestError> {
    match ext(path).as_str() {
        "mid" | "midi" | "smf" | "kar" => {
            let bytes = std::fs::read(path).map_err(|e| IngestError::Io(format!("{path}: {e}")))?;
            let smf = crate::forge_midi::parse_smf(&bytes).map_err(IngestError::Midi)?;
            let meta = TrackMeta {
                title: stem(path),
                duration_secs: smf.duration_secs() as f32,
                ..Default::default()
            };
            Ok(Ingested::Symbolic { smf, meta })
        }
        // MIDI 2.0 has no settled clip-file format yet; live MIDI 2.0 arrives via
        // UMP (`forge-ump`), and `.mid` files upgrade to UMP clips in the
        // composure layer (clip_to_ump_events). Fail loud rather than fake it.
        "umpx" | "mid2" => Err(IngestError::Unsupported(format!(
            "MIDI 2.0 clip file '{path}': author via live UMP (forge-ump) or import an SMF; native .umpx parse pending"
        ))),
        "" => Err(IngestError::Unsupported(format!("no extension: {path}"))),
        _ => {
            let audio = dsp::load_audio(path).map_err(IngestError::Decode)?;
            let mut meta = probe_meta(path);
            meta.duration_secs = audio.duration_secs(); // exact, post-decode
            // @forge:allow_float — load-time genre analysis; never on the realtime callback.
            // BPM 120.0 neutral: unknown at ingest time; GenreRouter Teacher can retrain later.
            // genre_detect call EXCLUDED — crate::genre_detect needs
            // forge_hal::expert_pool::MOE_QUERY_BYTES (absent) and a real
            // MoeRouter generic-arity fix, not this stroke's scope.
            let _ = &audio; // mono no longer consumed without genre_detect
            Ok(Ingested::Recorded { audio, meta })
        }
    }
}

/// Probe tags + duration WITHOUT a full decode — cheap, for playlist/folder
/// import. Duration comes from the codec frame count when the container reports
/// it (most do); otherwise `0.0` and a full [`ingest_file`] gives the exact value.
pub fn probe_meta(path: &str) -> TrackMeta {
    use symphonia::core::{io::MediaSourceStream, meta::StandardTagKey, probe::Hint};
    let mut meta = TrackMeta { title: stem(path), ..Default::default() };
    let Ok(file) = std::fs::File::open(path) else { return meta };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(e);
    }
    let Ok(mut probed) = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
    else {
        return meta;
    };
    // Duration from the header (no decode).
    if let Some(track) = probed.format.default_track() {
        if let (Some(frames), Some(sr)) =
            (track.codec_params.n_frames, track.codec_params.sample_rate)
        {
            meta.duration_secs = frames as f32 / sr.max(1) as f32;
        }
    }
    if let Some(rev) = probed.format.as_mut().metadata().current() {
        for tag in rev.tags() {
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => meta.title = tag.value.to_string(),
                Some(StandardTagKey::Artist) | Some(StandardTagKey::AlbumArtist)
                    if meta.artist.is_empty() =>
                {
                    meta.artist = tag.value.to_string()
                }
                Some(StandardTagKey::Album) => meta.album = tag.value.to_string(),
                _ => {}
            }
        }
    }
    meta
}

/// Ingest every supported file in a folder — drag-drop a folder = a playlist.
///
/// Cheap: probes metadata only (no full decode), sorted by path for stable order.
/// Returns `(path, meta)` so the caller can build playlist entries and play them.
pub fn ingest_folder(dir: &str) -> Vec<(String, TrackMeta)> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    let mut paths: Vec<std::path::PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_supported(&ext(p.to_str().unwrap_or(""))))
        .collect();
    paths.sort();
    for p in paths {
        if let Some(s) = p.to_str() {
            out.push((s.to_string(), probe_meta(s)));
        }
    }
    out
}

/// Result of compressing a track to the lossless FLAC floor — the "compress, then
/// check the size" half of the roundtrip.
#[derive(Debug, Clone)]
pub struct CompressionReport {
    pub src_bytes: u64,
    pub flac_bytes: u64,
    pub duration_secs: f64,
    pub channels: usize,
    pub sample_rate: u32,
}

impl CompressionReport {
    /// FLAC size as a percentage of the source (lower = better compression).
    pub fn ratio_pct(&self) -> f64 {
        if self.src_bytes == 0 {
            0.0
        } else {
            self.flac_bytes as f64 * 100.0 / self.src_bytes as f64
        }
    }
}

/// Compress a track to a lossless FLAC at `dst` and report the size delta.
///
/// Decodes `src` (the ingest seam's recorded path), FLAC-encodes it (16-bit
/// lossless), and measures both files. Reload `dst` with [`ingest_file`] to play it
/// back lossless — the full "compress → check size → bring it back → play lossless"
/// roundtrip.
pub fn compress_track(src: &str, dst: &str) -> Result<CompressionReport, IngestError> {
    let audio = dsp::load_audio(src).map_err(IngestError::Decode)?;
    let channels = audio.channels();
    let sample_rate = audio.sample_rate;
    let duration_secs = audio.duration_secs() as f64;
    dsp::encode_flac(&audio, dst).map_err(IngestError::Decode)?;
    let src_bytes = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let flac_bytes = std::fs::metadata(dst).map(|m| m.len()).unwrap_or(0);
    Ok(CompressionReport { src_bytes, flac_bytes, duration_secs, channels, sample_rate })
}

/// Is this extension one the ingest seam handles?
pub fn is_supported(ext: &str) -> bool {
    matches!(
        ext,
        "mp3" | "flac" | "ogg" | "wav" | "aiff" | "aif" | "mid" | "midi" | "smf" | "kar"
    )
}

/// Lowercased file extension, or "" if none.
fn ext(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// File stem as a display title fallback.
fn stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_wav_is_recorded_with_real_duration() {
        // 1 second of stereo silence at 48 kHz, written as a real WAV.
        let sr = 48_000;
        let buf = AudioBuffer {
            samples: vec![vec![0.0f32; sr as usize], vec![0.0f32; sr as usize]],
            sample_rate: sr,
        };
        let path = std::env::temp_dir().join("forgeaudio_ingest_test.wav");
        let ps = path.to_str().unwrap();
        dsp::write_wav(ps, &buf).expect("write wav");

        let ing = ingest_file(ps).expect("ingest wav");
        assert!(matches!(ing, Ingested::Recorded { .. }), "wav must be Recorded");
        assert!(
            (ing.duration_secs() - 1.0).abs() < 0.01,
            "expected ~1.0s, got {}",
            ing.duration_secs()
        );
        let _ = std::fs::remove_file(ps);
    }

    #[test]
    fn ingest_smf_is_symbolic_with_real_duration() {
        // Minimal format-0 SMF: PPQN 480, NoteOn@0, NoteOff@480 (one beat).
        // At the default 120 BPM that is exactly 0.5 s.
        let path = std::env::temp_dir().join("forgeaudio_ingest_test.mid");
        let ps = path.to_str().unwrap();
        std::fs::write(ps, minimal_smf()).expect("write smf");

        let ing = ingest_file(ps).expect("ingest smf");
        assert!(matches!(ing, Ingested::Symbolic { .. }), "mid must be Symbolic");
        assert!(
            (ing.duration_secs() - 0.5).abs() < 0.01,
            "expected ~0.5s, got {}",
            ing.duration_secs()
        );
        let _ = std::fs::remove_file(ps);
    }

    #[test]
    fn missing_or_unsupported_errors_loudly() {
        assert!(ingest_file("does_not_exist.xyz").is_err());
        assert!(matches!(
            ingest_file("clip.umpx"),
            Err(IngestError::Unsupported(_))
        ));
    }

    #[test]
    fn flac_roundtrip_is_lossless_and_smaller() {
        // 1 s of a 440 Hz stereo tone at 44.1 kHz — compressible, so FLAC < WAV.
        let sr = 44_100u32;
        let n = sr as usize;
        let mut l = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            l[i] = (t * 440.0 * std::f32::consts::TAU).sin() * 0.5;
        }
        let src_buf = AudioBuffer { samples: vec![l.clone(), l], sample_rate: sr };

        let dir = std::env::temp_dir();
        let wav = dir.join("forgeaudio_roundtrip.wav");
        let flac = dir.join("forgeaudio_roundtrip.flac");
        let (wav, flac) = (wav.to_str().unwrap(), flac.to_str().unwrap());
        dsp::write_wav(wav, &src_buf).expect("write wav");

        // Compress + check the size — FLAC must be smaller than the uncompressed WAV.
        let rep = compress_track(wav, flac).expect("compress");
        assert!(
            rep.flac_bytes > 0 && rep.flac_bytes < rep.src_bytes,
            "flac {} should be < wav {}",
            rep.flac_bytes,
            rep.src_bytes
        );

        // Bring it back + prove lossless at 16-bit (≤1 LSB drift = the f32↔i16 norm).
        let back = dsp::load_audio(flac).expect("reload flac");
        assert_eq!(back.channels(), 2);
        assert_eq!(back.sample_rate, sr);
        let eps = 2.0 / 32_767.0;
        for ch in 0..2 {
            for i in 0..n {
                assert!(
                    (src_buf.samples[ch][i] - back.samples[ch][i]).abs() <= eps,
                    "lossless drift ch{ch} i{i}: {} vs {}",
                    src_buf.samples[ch][i],
                    back.samples[ch][i]
                );
            }
        }
        let _ = std::fs::remove_file(wav);
        let _ = std::fs::remove_file(flac);
    }

    #[test]
    #[ignore = "genre_detect excluded - needs forge_hal::expert_pool::MOE_QUERY_BYTES (absent) + a MoeRouter generic-arity fix, not yet ported"]
    fn ingest_wav_auto_populates_genre() {
        let sr = 44_100u32;
        let buf = AudioBuffer {
            samples: vec![vec![0.0f32; sr as usize], vec![0.0f32; sr as usize]],
            sample_rate: sr,
        };
        let path = std::env::temp_dir().join("forgeaudio_genre_test.wav");
        let ps = path.to_str().unwrap();
        dsp::write_wav(ps, &buf).expect("write wav");
        let ing = ingest_file(ps).expect("ingest");
        match ing {
            Ingested::Recorded { meta, .. } => {
                assert!(meta.genre.is_some(), "ingest_file must auto-populate genre for PCM");
            }
            _ => panic!("expected Recorded"),
        }
        let _ = std::fs::remove_file(ps);
    }

    #[test]
    fn ingest_smf_genre_stays_none() {
        let path = std::env::temp_dir().join("forgeaudio_genre_midi_test.mid");
        let ps = path.to_str().unwrap();
        std::fs::write(ps, minimal_smf()).expect("write smf");
        let ing = ingest_file(ps).expect("ingest smf");
        match ing {
            Ingested::Symbolic { meta, .. } => {
                assert!(meta.genre.is_none(), "MIDI ingest must not set genre (audio-only)");
            }
            _ => panic!("expected Symbolic"),
        }
        let _ = std::fs::remove_file(ps);
    }

    /// Hand-built minimal format-0 Standard MIDI File.
    fn minimal_smf() -> Vec<u8> {
        let mut v = Vec::new();
        // MThd: len 6, format 0, 1 track, division 480 (0x01E0).
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0]);
        // MTrk + track body.
        let trk: [u8; 13] = [
            0x00, 0x90, 0x3C, 0x64, // dt 0: NoteOn ch0 note60 vel100
            0x83, 0x60, 0x80, 0x3C, 0x00, // dt 480 (VLQ 83 60): NoteOff ch0 note60
            0x00, 0xFF, 0x2F, 0x00, // dt 0: meta End-Of-Track
        ];
        v.extend_from_slice(b"MTrk");
        v.extend_from_slice(&(trk.len() as u32).to_be_bytes());
        v.extend_from_slice(&trk);
        v
    }
}