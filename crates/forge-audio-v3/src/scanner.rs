//! Library scanner — recursively scan directories for audio files, index metadata, BPM, and waveforms.

use std::fs;
use std::path::Path;
use std::io::Read;
use sha2::{Sha256, Digest};
#[cfg(feature = "radio-db")]
use crate::radio_db::RadioDb;

/// Progress callback data during scanning.
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub total: usize,
    pub scanned: usize,
    pub skipped: usize,
    pub errors: usize,
    pub current_file: String,
}

/// Result of a complete scan operation.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub total: usize,
    pub new_tracks: usize,
    pub skipped: usize,
    pub errors: Vec<(String, String)>, // (path, error message)
    pub duration_secs: f64,
}

use std::sync::{Arc, Mutex};

/// Scanner for populating the library database.
pub struct LibraryScanner {
    db: Arc<Mutex<RadioDb>>,
}

impl LibraryScanner {
    /// Create a new scanner with a database connection.
    pub fn new(db: Arc<Mutex<RadioDb>>) -> Self {
        Self { db }
    }

    /// Scan a directory recursively for audio files and index them.
    pub fn scan_directory(
        &self,
        path: &Path,
        fast: bool,
        thermal_state: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
        on_progress: impl Fn(&ScanProgress) + Sync + Send + Clone + 'static,
    ) -> Result<ScanResult, String> {
        let start = std::time::Instant::now();
        
        let mut progress = ScanProgress {
            total: 0,
            scanned: 0,
            skipped: 0,
            errors: 0,
            current_file: String::new(),
        };

        let mut errors: Vec<(String, String)> = Vec::new();
        let mut new_count = 0;

        let mut audio_files = Vec::new();
        self.collect_audio_files(path, &mut audio_files)?;
        progress.total = audio_files.len();

        let (tx, rx) = crossbeam_channel::bounded::<String>(8); // Max 8 in-flight backpressure
        let db_clone = self.db.clone();
        
        // Spawn 8 worker threads
        let mut handles = Vec::new();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<(String, Result<bool, String>)>();
        
        for _ in 0..8 {
            let rx = rx.clone();
            let res_tx = res_tx.clone();
            let scanner = LibraryScanner::new(db_clone.clone());
            let thermal = thermal_state.clone();
            
            handles.push(std::thread::spawn(move || {
                while let Ok(file_path) = rx.recv() {
                    // Thermal Governance: Pause if temp >= 78C, resume < 72C
                    if let Some(ref t) = thermal {
                        loop {
                            let temp = t.load(std::sync::atomic::Ordering::Relaxed);
                            if temp >= 78 {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                            } else if temp < 72 {
                                break;
                            } else {
                                // If between 72 and 78 after being hot, wait until it drops below 72
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                        }
                    }
                    
                    let res = scanner.process_file(&file_path, fast);
                    let _ = res_tx.send((file_path, res));
                }
            }));
        }

        // Drop the extra transmitter so the receiver knows when to stop
        drop(res_tx);

        // Feed the queue
        let tx_thread = std::thread::spawn(move || {
            for file_path in audio_files {
                if tx.send(file_path).is_err() { break; }
            }
        });

        // Collect results
        while let Ok((file_path, res)) = res_rx.recv() {
            progress.current_file = file_path.clone();
            on_progress(&progress);

            match res {
                Ok(inserted) => {
                    if inserted {
                        new_count += 1;
                        progress.scanned += 1;
                    } else {
                        progress.skipped += 1;
                    }
                }
                Err(e) => {
                    progress.errors += 1;
                    errors.push((file_path, e));
                }
            }
        }

        let _ = tx_thread.join();
        for h in handles { let _ = h.join(); }

        let duration_secs = start.elapsed().as_secs_f64();

        Ok(ScanResult {
            total: progress.total,
            new_tracks: new_count,
            skipped: progress.skipped,
            errors,
            duration_secs,
        })
    }

    /// Recursively collect all audio file paths.
    fn collect_audio_files(&self, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                self.collect_audio_files(&path, files)?;
            } else if self.is_audio_file(&path) {
                files.push(path.to_string_lossy().to_string());
            }
        }

        Ok(())
    }

    /// Check if a file has an audio extension.
    fn is_audio_file(&self, path: &Path) -> bool {
        const AUDIO_EXTS: &[&str] = &["mp3", "flac", "wav", "ogg", "aiff", "aac", "m4a", "wma"];
        
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        AUDIO_EXTS.contains(&ext.as_str())
    }

    /// Process a single audio file: hash, check if exists, extract metadata, compute peaks, detect BPM.
    fn process_file(&self, path: &str, fast: bool) -> Result<bool, String> {
        // Lock-Free Gate: never call .lock().unwrap() — use try_lock() with spin so a
        // poisoned or contended mutex surfaces as an Err, not a silent panic.
        #[inline]
        fn db_try_lock(db: &std::sync::Arc<std::sync::Mutex<crate::radio_db::RadioDb>>)
            -> Result<std::sync::MutexGuard<'_, crate::radio_db::RadioDb>, String>
        {
            loop {
                match db.try_lock() {
                    Ok(g) => return Ok(g),
                    Err(std::sync::TryLockError::WouldBlock) => {
                        std::thread::yield_now();
                    }
                    Err(std::sync::TryLockError::Poisoned(e)) => {
                        return Err(format!("DB mutex poisoned: {e}"));
                    }
                }
            }
        }
        // Fast path: skip if this file path is already in DB (no file I/O needed)
        let path_exists = db_try_lock(&self.db)?
            .track_exists_by_path(path)
            .map_err(|e| e.to_string())?;
        if path_exists { return Ok(false); }

        // Compute hash for dedup (same file at different path)
        let hash = compute_fast_hash(path)?;
        let hash_exists = db_try_lock(&self.db)?
            .track_exists_by_hash(&hash)
            .map_err(|e| e.to_string())?;
        if hash_exists { return Ok(false); }

        // Read metadata from tags
        let tags = read_audio_tags(path).unwrap_or_default();

        // Read audio header
        let (duration_secs, sample_rate, channels, format) = read_audio_header(path)?;

        // Analysis pass — skip entirely in fast mode (tags-only scan)
        let (bpm, musical_key, detected_genre, peaks_blob) = if !fast {
            let samples = load_samples_for_analysis(path, sample_rate)?;

            let bpm = if !samples.is_empty() {
                let buf = crate::dsp::AudioBuffer {
                    samples: vec![samples.clone()],
                    sample_rate,
                };
                let detected = crate::bpm::detect_bpm(&buf);
                if detected > 0.0 { Some(detected as f64) } else { None }
            } else {
                None
            };

            let key_result = if !samples.is_empty() {
                crate::key_detect::detect_key(&samples, sample_rate)
            } else {
                None
            };
            let musical_key = key_result.as_ref().map(|k| k.camelot.clone());

            // genre_detect LANDED 2026-08-19 (GenreRouter over MetaRouter/.s13,
            // see genre_detect.rs). The heuristic classifier (`detect_genre`)
            // always runs; the `.s13`-LUT refinement layer is separate and not
            // wired here yet (no shipped `.s13` genre model exists on disk —
            // that's authorship/training, a real follow-on, not this wire).
            let genre_result = if let Some(bpm_val) = bpm {
                if !samples.is_empty() {
                    Some(crate::genre_detect::detect_genre(&samples, sample_rate, bpm_val))
                } else {
                    None
                }
            } else {
                None
            };
            let detected_genre = genre_result.as_ref().map(|g| g.genre.name().to_string());

            let peaks_blob = waveform_to_blob(&peaks_from_samples(&samples, 200));

            (bpm, musical_key, detected_genre, peaks_blob)
        } else {
            (None, None, None, Vec::new())
        };

        // Get file size
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Failed to stat file: {}", e))?;
        let size_bytes = metadata.len() as i64;

        // Insert into database with key + genre. DB boundary is integer
        // (2026-08-19): duration_secs (f64, from read_audio_header) and bpm
        // (Option<f64>, from detect_bpm) convert to milliseconds/milli-BPM here.
        let duration_ms = (duration_secs * 1000.0).round() as i32;
        let bpm_mbpm = bpm.map(|b| (b * 1000.0).round() as i32);
        db_try_lock(&self.db)?.insert_track_full(
            path,
            size_bytes,
            &hash,
            tags.artist.as_deref(),
            tags.album.as_deref(),
            tags.title.as_deref(),
            tags.genre.as_deref(),
            tags.year,
            duration_ms,
            sample_rate as i32,
            channels as i32,
            &format,
            bpm_mbpm,
            musical_key.as_deref(),
            detected_genre.as_deref(),
            if peaks_blob.is_empty() { None } else { Some(&peaks_blob) },
        ).map_err(|e| e.to_string())?;

        Ok(true) // Successfully inserted
    }
}

/// Metadata extracted from audio tags (lofty: ID3 / Vorbis / MP4 …).
#[derive(Debug, Default, Clone)]
pub struct AudioTags {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
}

/// Extract metadata from audio file tags using lofty. The library indexer
/// (`library::index_dir`) calls this for real artist/title/album/year/genre.
pub fn read_audio_tags(path: &str) -> Result<AudioTags, String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged_file = match Probe::open(path)
        .map_err(|e| format!("Failed to open for tags: {}", e))?
        .read()
    {
        Ok(f) => f,
        Err(_) => return Ok(AudioTags::default()),
    };

    let tag = match tagged_file.primary_tag().or(tagged_file.first_tag()) {
        Some(t) => t,
        None => return Ok(AudioTags::default()),
    };

    Ok(AudioTags {
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        title: tag.title().map(|s| s.to_string()),
        genre: tag.genre().map(|s| s.to_string()),
        year: tag.year().map(|y| y as i32),
    })
}

/// Extract audio header info using symphonia.
pub fn read_audio_header(path: &str) -> Result<(f64, u32, u16, String), String> {
    use std::fs::File;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::probe::Hint;

    let file = File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension() {
        if let Some(ext_str) = ext.to_str() {
            hint.with_extension(ext_str);
        }
    }

    let probe_result = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| format!("Failed to probe format: {:?}", e))?;

    let mut duration_secs = 0.0;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut format_str = "unknown".to_string();

    // Get codec parameters from the probed format
    let mut first_track_params = None;
    for (i, track) in probe_result.format.tracks().iter().enumerate() {
        if i == 0 {
            first_track_params = Some(track.codec_params.clone());
            break;
        }
    }

    if let Some(codec_params) = first_track_params {
        if let Some(sr) = codec_params.sample_rate {
            sample_rate = sr;
        }
        if let Some(ch) = codec_params.channels {
            channels = ch.count() as u16;
        }
        if let Some(n_frames) = codec_params.n_frames {
            if sample_rate > 0 {
                duration_secs = n_frames as f64 / sample_rate as f64;
            }
        }
    }

    // Guess format from extension
    if let Some(ext) = Path::new(path).extension() {
        format_str = ext.to_string_lossy().to_lowercase();
    }

    Ok((duration_secs, sample_rate, channels, format_str))
}

/// Compute a fast hash: SHA-256 of (first 64KB + file size bytes).
pub fn compute_fast_hash(path: &str) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024];
    let mut bytes_read = 0;
    const CHUNK_SIZE: usize = 64 * 1024;

    // Hash first 64KB
    loop {
        let n = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 || bytes_read >= CHUNK_SIZE {
            break;
        }
        let to_hash = std::cmp::min(n, CHUNK_SIZE - bytes_read);
        hasher.update(&buffer[..to_hash]);
        bytes_read += to_hash;
    }

    // Hash file size
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Failed to stat file: {}", e))?;
    let size_bytes = metadata.len().to_le_bytes();
    hasher.update(size_bytes);

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Compute peak overview from already-decoded mono f32 samples — zero extra I/O.
/// Prefer this over `compute_waveform_peaks` when samples are already in hand.
pub fn peaks_from_samples(samples: &[f32], num_points: usize) -> Vec<f32> {
    if samples.is_empty() || num_points == 0 {
        return vec![0.0; num_points];
    }
    let chunk = (samples.len() + num_points - 1) / num_points;
    let mut out: Vec<f32> = samples
        .chunks(chunk)
        .take(num_points)
        .map(|c| c.iter().map(|s| s.abs()).fold(0.0f32, f32::max))
        .collect();
    out.resize(num_points, 0.0);
    out
}

/// Compute a waveform peak overview by decoding the audio file.
/// Only use this when samples are not already decoded — it pays a full decode cost.
/// Inside the scanner, call `peaks_from_samples` with the already-loaded samples.
pub fn compute_waveform_peaks(path: &str, num_points: usize) -> Result<Vec<f32>, String> {
    let samples = load_samples_for_analysis(path, 44100)?;
    Ok(peaks_from_samples(&samples, num_points))
}

/// Convert waveform peaks (f32 slice) to little-endian blob.
pub fn waveform_to_blob(peaks: &[f32]) -> Vec<u8> {
    peaks.iter()
        .flat_map(|f| f.to_le_bytes().to_vec())
        .collect()
}

/// Convert blob back to waveform peaks.
pub fn blob_to_waveform(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Load first 30 seconds of audio as mono f32 samples for analysis.
pub fn load_samples_for_analysis(path: &str, sample_rate: u32) -> Result<Vec<f32>, String> {
    use std::fs::File;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::probe::Hint;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;

    let file = File::open(path).map_err(|e| format!("Failed to open: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probe = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| format!("Probe failed: {:?}", e))?;

    let mut format_reader = probe.format;
    let track = format_reader.tracks().first()
        .ok_or("No audio track found")?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Decoder failed: {:?}", e))?;

    let max_samples = (sample_rate as usize) * 30; // 30 seconds
    let mut samples = Vec::with_capacity(max_samples);

    while let Ok(packet) = format_reader.next_packet() {
        if packet.track_id() != track_id { continue; }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let buf = sample_buf.samples();

        // Mix to mono
        let ch = spec.channels.count();
        for frame in buf.chunks(ch) {
            let mono: f32 = frame.iter().sum::<f32>() / ch as f32;
            samples.push(mono);
            if samples.len() >= max_samples { break; }
        }
        if samples.len() >= max_samples { break; }
    }

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_scanner() -> LibraryScanner {
        let conn = Connection::open_in_memory().unwrap();
        let db = crate::radio_db::RadioDb::from_connection(conn).unwrap();
        LibraryScanner::new(Arc::new(Mutex::new(db)))
    }

    #[test]
    fn test_scan_empty_directory() {
        let scanner = test_scanner();
        let temp_dir = std::env::temp_dir().join("forge_scan_test_empty");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        
        let result = scanner.scan_directory(&temp_dir, false, None, |_| {}).unwrap();
        assert_eq!(result.total, 0);
        assert_eq!(result.new_tracks, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors.len(), 0);
        
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_waveform_blob_roundtrip() {
        let original = vec![0.1, 0.2, 0.5, 1.0, 0.0];
        let blob = waveform_to_blob(&original);
        let decoded = blob_to_waveform(&blob);
        
        for (a, b) in original.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.0001);
        }
    }

    #[test]
    fn test_fast_hash_consistency() {
        let temp_dir = std::env::temp_dir().join("forge_hash_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        
        let test_file = temp_dir.join("test.data");
        fs::write(&test_file, b"test data").unwrap();
        
        let hash1 = compute_fast_hash(test_file.to_str().unwrap()).unwrap();
        let hash2 = compute_fast_hash(test_file.to_str().unwrap()).unwrap();
        
        assert_eq!(hash1, hash2);
        
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_audio_file() {
        let scanner = test_scanner();
        
        assert!(scanner.is_audio_file(Path::new("track.mp3")));
        assert!(scanner.is_audio_file(Path::new("song.FLAC")));
        assert!(scanner.is_audio_file(Path::new("audio.wav")));
        assert!(!scanner.is_audio_file(Path::new("image.jpg")));
        assert!(!scanner.is_audio_file(Path::new("document.txt")));
    }
}
