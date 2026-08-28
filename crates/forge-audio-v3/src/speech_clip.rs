//! Speech clip editor — cut ums/uhs, trim pauses, quantize to beat grid.
//!
//! Pipeline: `ingest MP3 → detect_speech_clips → remove fillers → quantize → export`
//!
//! Designed for spoken-word tracks that need to sync with a video timeline:
//! 1. Energy-envelope pause detection splits raw speech into clips
//! 2. Short low-energy "filler" clips (ums, uhs, hesitations) are classified and removed
//! 3. Remaining clips are snapped to beat subdivisions for rhythmic video editing
//!
//! LOAD-TIME ONLY — no RT thread usage, heap alloc is fine.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.

use crate::bpm::{self, BeatGrid};
use crate::dsp::AudioBuffer;

// ── Configuration ────────────────────────────────────────────────────────────

/// How to classify and trim speech clips.
#[derive(Debug, Clone)]
pub struct ClipConfig {
    /// Minimum pause duration (seconds) to treat as a clip boundary.
    pub min_pause_secs: f32,
    /// Energy threshold (RMS) below which audio is considered silence/pause.
    /// Range: 0.0–1.0. Typical: 0.02–0.05.
    pub silence_threshold: f32,
    /// Maximum duration (seconds) for a clip to be classified as a filler.
    /// Clips shorter than this AND with low spectral centroid = filler.
    pub max_filler_duration_secs: f32,
    /// Beat subdivision for quantization (1 = whole beat, 2 = half, 4 = quarter, etc.)
    pub beat_subdivision: u32,
    /// Crossfade duration in seconds at clip boundaries (prevents clicks).
    pub crossfade_secs: f32,
    /// Minimum clip duration (seconds) to keep after filler removal.
    pub min_clip_secs: f32,
}

impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            min_pause_secs: 0.3,
            silence_threshold: 0.025,
            max_filler_duration_secs: 0.6,
            beat_subdivision: 4,
            crossfade_secs: 0.005,
            min_clip_secs: 0.15,
        }
    }
}

// ── Core types ───────────────────────────────────────────────────────────────

/// A detected speech clip with its classification.
#[derive(Debug, Clone)]
pub struct SpeechClip {
    /// Start sample in the source buffer.
    pub start: usize,
    /// End sample (exclusive) in the source buffer.
    pub end: usize,
    /// Classification of this clip.
    pub kind: ClipKind,
    /// RMS energy of this clip.
    pub rms: f32,
    /// Spectral centroid (Hz) — low values suggest filler/mumble.
    pub centroid_hz: f32,
}

impl SpeechClip {
    pub fn duration_secs(&self, sample_rate: u32) -> f32 {
        (self.end - self.start) as f32 / sample_rate as f32
    }

    pub fn samples<'a>(&self, mono: &'a [f32]) -> &'a [f32] {
        &mono[self.start..self.end.min(mono.len())]
    }
}

/// Classification of a speech clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipKind {
    /// Normal speech content — keep.
    Speech,
    /// Filler word (um, uh, hesitation) — remove.
    Filler,
    /// Pure silence/pause — remove.
    Pause,
}

/// The result of the full speech-clip pipeline.
#[derive(Debug, Clone)]
pub struct ClipEditResult {
    /// All detected clips (including removed ones, for UI display).
    pub all_clips: Vec<SpeechClip>,
    /// Indices into `all_clips` that were kept (Speech only).
    pub kept_indices: Vec<usize>,
    /// The reassembled output audio (only kept clips, crossfaded, beat-quantized).
    pub output: AudioBuffer,
    /// Beat grid used for quantization.
    pub grid: BeatGrid,
    /// Total removed duration in seconds.
    pub removed_secs: f32,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Full speech clip edit pipeline:
/// 1. Detect clips via energy envelope
/// 2. Classify fillers via duration + spectral centroid
/// 3. Remove fillers and pauses
/// 4. Quantize remaining clips to beat grid
/// 5. Reassemble with crossfades
pub fn edit_speech(buf: &AudioBuffer, bpm: f32, config: &ClipConfig) -> ClipEditResult {
    let sr = buf.sample_rate;
    let mono = buf.to_mono();
    let clips = detect_clips(&mono, sr, config);
    finish_edit(mono, clips, bpm, sr, config)
}

/// Word-aware variant: same energy-envelope clip detection, but the classification
/// (Speech vs Filler) is OVERRIDDEN by a transcript's word spans where they overlap.
/// The energy path only ever GUESSES a filler (short + low spectral centroid); a real
/// STT transcript (Whisper `GhostWord`s → [`WordSpan`]) knows the actual word, so a clip
/// carrying "um"/"uh" is cut and a clip carrying real speech is kept regardless of its
/// acoustics. Clips with no word overlap keep their energy-based kind (pauses/music the
/// transcript never covers). `words` empty ⇒ identical to [`edit_speech`].
pub fn edit_speech_words(
    buf: &AudioBuffer,
    bpm: f32,
    config: &ClipConfig,
    words: &[WordSpan],
) -> ClipEditResult {
    let sr = buf.sample_rate;
    let mono = buf.to_mono();
    let mut clips = detect_clips(&mono, sr, config);
    classify_by_words(&mut clips, words, sr);
    finish_edit(mono, clips, bpm, sr, config)
}

/// Shared tail of the edit pipeline: given classified clips, keep the Speech ones,
/// build the beat grid, reassemble crossfaded + quantized, tally removed time.
fn finish_edit(
    mono: Vec<f32>,
    clips: Vec<SpeechClip>,
    bpm: f32,
    sr: u32,
    config: &ClipConfig,
) -> ClipEditResult {
    let kept_indices: Vec<usize> = clips
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == ClipKind::Speech)
        .map(|(i, _)| i)
        .collect();

    let total_kept_samples: usize = kept_indices.iter().map(|&i| clips[i].end - clips[i].start).sum();
    let grid = BeatGrid::from_bpm(bpm, 0, sr, total_kept_samples);

    let output = assemble_quantized(&mono, &clips, &kept_indices, &grid, sr, config);

    let removed_secs: f32 = clips
        .iter()
        .enumerate()
        .filter(|(i, _)| !kept_indices.contains(i))
        .map(|(_, c)| c.duration_secs(sr))
        .sum();

    ClipEditResult {
        all_clips: clips,
        kept_indices,
        output,
        grid,
        removed_secs,
    }
}

/// A transcript word mapped onto the source timeline. `is_filler` is the caller's
/// verdict (lexicon match on the STT text) — speech_clip is transcript-agnostic and
/// only consumes the timing + this flag, so no STT dep crosses into forge-audio.
#[derive(Debug, Clone)]
pub struct WordSpan {
    pub start_ms: u64,
    pub end_ms: u64,
    pub is_filler: bool,
}

/// Override each clip's `kind` by transcript-word overlap (Whisper truth beats the
/// energy guess). A clip overlapping any real word ⇒ Speech; overlapping only
/// filler words ⇒ Filler; no overlap ⇒ energy-based kind is left intact.
pub fn classify_by_words(clips: &mut [SpeechClip], words: &[WordSpan], sample_rate: u32) {
    if words.is_empty() || sample_rate == 0 {
        return;
    }
    let sr = sample_rate as u64;
    for clip in clips.iter_mut() {
        let clip_lo = (clip.start as u64) * 1000 / sr;
        let clip_hi = (clip.end as u64) * 1000 / sr;
        let mut filler_ms = 0u64;
        let mut speech_ms = 0u64;
        for w in words {
            let lo = clip_lo.max(w.start_ms);
            let hi = clip_hi.min(w.end_ms);
            if hi > lo {
                if w.is_filler {
                    filler_ms += hi - lo;
                } else {
                    speech_ms += hi - lo;
                }
            }
        }
        if speech_ms > 0 {
            clip.kind = ClipKind::Speech;
        } else if filler_ms > 0 {
            clip.kind = ClipKind::Filler;
        }
    }
}

/// Detect speech clips from a mono signal using energy envelope analysis.
pub fn detect_clips(mono: &[f32], sample_rate: u32, config: &ClipConfig) -> Vec<SpeechClip> {
    if mono.is_empty() {
        return vec![];
    }

    let sr = sample_rate as f32;
    // 20ms analysis window, 10ms hop
    let window = (sr * 0.020) as usize;
    let hop = window / 2;

    if window == 0 || mono.len() < window {
        return vec![];
    }

    // Compute RMS envelope per frame
    let n_frames = (mono.len().saturating_sub(window)) / hop + 1;
    let rms_envelope: Vec<f32> = (0..n_frames)
        .map(|i| {
            let start = i * hop;
            let end = (start + window).min(mono.len());
            let sum_sq: f32 = mono[start..end].iter().map(|&s| s * s).sum();
            (sum_sq / (end - start) as f32).sqrt()
        })
        .collect();

    // Classify frames as speech/silence
    let min_pause_frames = (config.min_pause_secs * sr / hop as f32) as usize;
    let mut is_speech: Vec<bool> = rms_envelope
        .iter()
        .map(|&rms| rms > config.silence_threshold)
        .collect();

    // Fill short silence gaps (< min_pause) — they're probably just consonants
    fill_short_gaps(&mut is_speech, min_pause_frames);

    // Extract contiguous speech regions
    let regions = extract_regions(&is_speech);

    // Convert frame regions to sample clips with classification
    let min_clip_frames = (config.min_clip_secs * sr / hop as f32) as usize;

    regions
        .into_iter()
        .filter(|(start, end)| end - start >= min_clip_frames)
        .map(|(frame_start, frame_end)| {
            let start_sample = frame_start * hop;
            let end_sample = (frame_end * hop + window).min(mono.len());
            let clip_mono = &mono[start_sample..end_sample];

            let rms = compute_rms(clip_mono);
            let centroid = spectral_centroid(clip_mono, sample_rate);
            let duration = (end_sample - start_sample) as f32 / sr;

            let kind = classify_clip(duration, rms, centroid, config);

            SpeechClip {
                start: start_sample,
                end: end_sample,
                kind,
                rms,
                centroid_hz: centroid,
            }
        })
        .collect()
}

/// Detect BPM from a speech clip and snap to common video-edit tempos.
/// Returns a tempo suitable for video sync (typically 90–160 BPM).
pub fn detect_speech_tempo(buf: &AudioBuffer) -> f32 {
    let raw_bpm = bpm::detect_bpm(buf);
    bpm::snap_bpm(raw_bpm)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Classify a clip as Speech, Filler, or Pause based on its acoustic properties.
fn classify_clip(duration_secs: f32, rms: f32, centroid_hz: f32, config: &ClipConfig) -> ClipKind {
    // Very low energy = pause
    if rms < config.silence_threshold * 0.5 {
        return ClipKind::Pause;
    }

    // Short + low centroid = filler (ums/uhs have low spectral energy, centered around 200-500 Hz)
    if duration_secs <= config.max_filler_duration_secs && centroid_hz < 800.0 {
        return ClipKind::Filler;
    }

    // Short + very low RMS (just above pause threshold) = likely filler
    if duration_secs <= config.max_filler_duration_secs && rms < config.silence_threshold * 1.5 {
        return ClipKind::Filler;
    }

    ClipKind::Speech
}

/// Compute RMS of a signal slice.
fn compute_rms(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = signal.iter().map(|&s| s * s).sum();
    (sum_sq / signal.len() as f32).sqrt()
}

/// Estimate spectral centroid via zero-crossing rate + energy distribution.
/// Fast approximation: no full FFT, uses band-split energy ratios.
/// Returns Hz estimate of the spectral center of mass.
fn spectral_centroid(signal: &[f32], sample_rate: u32) -> f32 {
    if signal.len() < 256 {
        return 0.0;
    }
    let sr = sample_rate as f32;

    // Zero-crossing rate → rough frequency indicator
    let zcr: usize = signal.windows(2).filter(|w| w[0].signum() != w[1].signum()).count();
    let zcr_freq = (zcr as f32 * sr) / (2.0 * signal.len() as f32);

    // Simple 4-band energy split for centroid estimate
    let chunk_size = signal.len() / 4;
    if chunk_size == 0 {
        return zcr_freq;
    }

    // Use DFT of first 512 samples for a quick spectral peek
    let n = 512.min(signal.len());
    let mut band_energy = [0.0f32; 4];
    let _band_width = n / 8; // each "DFT bin" cluster

    // Compute magnitude via Goertzel for 4 frequency bands
    let band_centers = [
        sr * 0.0625,  // ~low (sub 3kHz band)
        sr * 0.125,   // ~low-mid
        sr * 0.25,    // ~high-mid
        sr * 0.375,   // ~high
    ];

    for (band_idx, &freq) in band_centers.iter().enumerate() {
        let k = (freq * n as f32 / sr).round() as usize;
        if k >= n {
            continue;
        }
        // Goertzel for this bin
        let w = 2.0 * std::f32::consts::PI * k as f32 / n as f32;
        let coeff = 2.0 * w.cos();
        let mut s0 = 0.0f32;
        let mut s1 = 0.0f32;
        let mut s2;
        for i in 0..n {
            s2 = signal[i] + coeff * s0 - s1;
            s1 = s0;
            s0 = s2;
        }
        let power = s0 * s0 + s1 * s1 - coeff * s0 * s1;
        band_energy[band_idx] = power.max(0.0);
    }

    // Weighted centroid
    let total_energy: f32 = band_energy.iter().sum();
    if total_energy < 1e-10 {
        return zcr_freq;
    }

    let weighted_sum: f32 = band_energy
        .iter()
        .zip(band_centers.iter())
        .map(|(&e, &f)| e * f)
        .sum();

    weighted_sum / total_energy
}

/// Fill short gaps in the speech/silence classification.
/// Gaps shorter than `min_frames` are filled (treated as speech).
fn fill_short_gaps(is_speech: &mut [bool], min_frames: usize) {
    if is_speech.is_empty() {
        return;
    }

    let mut i = 0;
    while i < is_speech.len() {
        if !is_speech[i] {
            // Find end of this silence gap
            let gap_start = i;
            while i < is_speech.len() && !is_speech[i] {
                i += 1;
            }
            let gap_len = i - gap_start;
            // If gap is too short, fill it
            if gap_len < min_frames {
                for j in gap_start..i {
                    is_speech[j] = true;
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Extract contiguous `true` regions as (start, end) frame indices.
fn extract_regions(is_speech: &[bool]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut i = 0;
    while i < is_speech.len() {
        if is_speech[i] {
            let start = i;
            while i < is_speech.len() && is_speech[i] {
                i += 1;
            }
            regions.push((start, i));
        } else {
            i += 1;
        }
    }
    regions
}

/// Reassemble kept clips with crossfades, snapped to beat grid subdivisions.
fn assemble_quantized(
    mono: &[f32],
    clips: &[SpeechClip],
    kept_indices: &[usize],
    grid: &BeatGrid,
    sample_rate: u32,
    config: &ClipConfig,
) -> AudioBuffer {
    if kept_indices.is_empty() {
        return AudioBuffer {
            samples: vec![vec![]],
            sample_rate,
        };
    }

    let crossfade_samples = (config.crossfade_secs * sample_rate as f32) as usize;
    let subdivision_samples = grid.beat_interval / config.beat_subdivision.max(1) as usize;

    // Calculate total output length: each clip snapped to next subdivision boundary
    let mut output_len = 0usize;
    for &idx in kept_indices {
        let clip = &clips[idx];
        let clip_len = clip.end - clip.start;
        // Snap clip length up to next subdivision boundary
        let snapped_len = snap_up(clip_len, subdivision_samples);
        output_len += snapped_len;
    }

    let mut output = vec![0.0f32; output_len];
    let mut write_pos = 0usize;

    for (i, &idx) in kept_indices.iter().enumerate() {
        let clip = &clips[idx];
        let clip_samples = &mono[clip.start..clip.end];
        let clip_len = clip_samples.len();

        // Snap the write position to subdivision grid
        if subdivision_samples > 0 {
            write_pos = snap_up(write_pos, subdivision_samples);
        }

        if write_pos + clip_len > output.len() {
            // Safety: don't write past end
            let avail = output.len().saturating_sub(write_pos);
            if avail == 0 {
                break;
            }
            output[write_pos..write_pos + avail].copy_from_slice(&clip_samples[..avail]);
            break;
        }

        // Copy clip samples
        output[write_pos..write_pos + clip_len].copy_from_slice(clip_samples);

        // Apply fade-in at clip start
        let fade_in = crossfade_samples.min(clip_len / 2);
        for j in 0..fade_in {
            let gain = j as f32 / fade_in as f32;
            output[write_pos + j] *= gain;
        }

        // Apply fade-out at clip end
        let fade_out = crossfade_samples.min(clip_len / 2);
        for j in 0..fade_out {
            let gain = 1.0 - (j as f32 / fade_out as f32);
            let pos = write_pos + clip_len - fade_out + j;
            if pos < output.len() {
                output[pos] *= gain;
            }
        }

        // Crossfade overlap with previous clip
        if i > 0 && crossfade_samples > 0 && write_pos >= crossfade_samples {
            // The fade-out of the previous and fade-in of current already handle this
            // via the overlap at the subdivision boundary.
        }

        write_pos += clip_len;
    }

    // Trim trailing silence
    let final_len = output.iter().rposition(|&s| s.abs() > 1e-6).unwrap_or(0) + 1;
    output.truncate(final_len);

    AudioBuffer {
        samples: vec![output],
        sample_rate,
    }
}

/// Snap a value up to the next multiple of `grid_size`.
fn snap_up(value: usize, grid_size: usize) -> usize {
    if grid_size == 0 {
        return value;
    }
    let remainder = value % grid_size;
    if remainder == 0 {
        value
    } else {
        value + grid_size - remainder
    }
}

// ── Convenience: one-shot file → file ───────────────────────────────────────

/// Process an MP3/WAV file: cut fillers, quantize to detected BPM, write WAV.
/// Returns the path of the output file and edit stats.
pub fn process_speech_file(
    input_path: &str,
    output_path: &str,
    config: Option<ClipConfig>,
) -> Result<ClipEditStats, String> {
    let buf = crate::dsp::load_audio(input_path)?;
    let config = config.unwrap_or_default();
    let bpm = detect_speech_tempo(&buf);
    let result = edit_speech(&buf, bpm, &config);

    // Write output WAV
    write_wav(output_path, &result.output)?;

    Ok(edit_stats(buf.duration_secs(), &result, bpm, config.beat_subdivision))
}

/// Transcript-driven variant of [`process_speech_file`]: the caller supplies word
/// spans (e.g. Whisper `GhostWord`s with fillers flagged); filler classification is
/// word-accurate instead of an energy guess. `words` empty ⇒ same as the energy path.
pub fn process_speech_file_words(
    input_path: &str,
    output_path: &str,
    config: Option<ClipConfig>,
    words: &[WordSpan],
) -> Result<ClipEditStats, String> {
    let buf = crate::dsp::load_audio(input_path)?;
    let config = config.unwrap_or_default();
    let bpm = detect_speech_tempo(&buf);
    let result = edit_speech_words(&buf, bpm, &config, words);
    write_wav(output_path, &result.output)?;
    Ok(edit_stats(buf.duration_secs(), &result, bpm, config.beat_subdivision))
}

/// Build the reported stats from a finished edit (shared by both file entries).
fn edit_stats(input_duration_secs: f32, result: &ClipEditResult, bpm: f32, beat_subdivision: u32) -> ClipEditStats {
    ClipEditStats {
        input_duration_secs,
        output_duration_secs: result.output.duration_secs(),
        removed_secs: result.removed_secs,
        total_clips: result.all_clips.len(),
        kept_clips: result.kept_indices.len(),
        fillers_removed: result.all_clips.iter().filter(|c| c.kind == ClipKind::Filler).count(),
        pauses_removed: result.all_clips.iter().filter(|c| c.kind == ClipKind::Pause).count(),
        bpm,
        beat_subdivision,
    }
}

/// Stats from a clip edit operation.
#[derive(Debug, Clone)]
pub struct ClipEditStats {
    pub input_duration_secs: f32,
    pub output_duration_secs: f32,
    pub removed_secs: f32,
    pub total_clips: usize,
    pub kept_clips: usize,
    pub fillers_removed: usize,
    pub pauses_removed: usize,
    pub bpm: f32,
    pub beat_subdivision: u32,
}

/// Write a mono AudioBuffer to a WAV file.
fn write_wav(path: &str, buf: &AudioBuffer) -> Result<(), String> {
    use std::io::Write;

    let mono = buf.to_mono();
    let sr = buf.sample_rate;
    let num_samples = mono.len();
    let bits_per_sample: u16 = 16;
    let num_channels: u16 = 1;
    let byte_rate = sr * (bits_per_sample as u32 / 8) * num_channels as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = (num_samples * 2) as u32; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut file = std::fs::File::create(path).map_err(|e| format!("create {path}: {e}"))?;

    // RIFF header
    file.write_all(b"RIFF").map_err(|e| e.to_string())?;
    file.write_all(&file_size.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(b"WAVE").map_err(|e| e.to_string())?;

    // fmt chunk
    file.write_all(b"fmt ").map_err(|e| e.to_string())?;
    file.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?; // chunk size
    file.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?; // PCM format
    file.write_all(&num_channels.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&sr.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&byte_rate.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&block_align.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&bits_per_sample.to_le_bytes()).map_err(|e| e.to_string())?;

    // data chunk
    file.write_all(b"data").map_err(|e| e.to_string())?;
    file.write_all(&data_size.to_le_bytes()).map_err(|e| e.to_string())?;

    // Write 16-bit PCM samples
    for &sample in &mono {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_val = (clamped * 32767.0) as i16;
        file.write_all(&i16_val.to_le_bytes()).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_buffer(samples: Vec<f32>, sample_rate: u32) -> AudioBuffer {
        AudioBuffer {
            samples: vec![samples],
            sample_rate,
        }
    }

    #[test]
    fn test_detect_clips_empty() {
        let clips = detect_clips(&[], 44100, &ClipConfig::default());
        assert!(clips.is_empty());
    }

    #[test]
    fn test_detect_clips_silence() {
        let silence = vec![0.0f32; 44100]; // 1 second of silence
        let clips = detect_clips(&silence, 44100, &ClipConfig::default());
        // All silence → no clips (or all classified as Pause)
        assert!(clips.iter().all(|c| c.kind == ClipKind::Pause || c.kind == ClipKind::Filler));
    }

    #[test]
    fn test_detect_clips_speech_with_pause() {
        let sr = 44100u32;
        let mut signal = Vec::new();
        // 1 second of "speech" (sine wave at 300 Hz, moderate amplitude)
        for i in 0..(sr as usize) {
            let t = i as f32 / sr as f32;
            signal.push(0.3 * (2.0 * std::f32::consts::PI * 300.0 * t).sin());
        }
        // 0.5 second pause
        signal.extend(vec![0.0f32; sr as usize / 2]);
        // 1 second more speech
        for i in 0..(sr as usize) {
            let t = i as f32 / sr as f32;
            signal.push(0.3 * (2.0 * std::f32::consts::PI * 300.0 * t).sin());
        }

        let clips = detect_clips(&signal, sr, &ClipConfig::default());
        let speech_clips: Vec<_> = clips.iter().filter(|c| c.kind == ClipKind::Speech).collect();
        // Should detect at least one speech region
        assert!(!speech_clips.is_empty());
    }

    #[test]
    fn test_snap_up() {
        assert_eq!(snap_up(0, 100), 0);
        assert_eq!(snap_up(50, 100), 100);
        assert_eq!(snap_up(100, 100), 100);
        assert_eq!(snap_up(101, 100), 200);
    }

    #[test]
    fn test_edit_speech_roundtrip() {
        let sr = 44100u32;
        let mut signal = Vec::new();
        // Generate speech-like signal: voiced + pause + filler + pause + voiced
        // Voiced (1s)
        for i in 0..(sr as usize) {
            let t = i as f32 / sr as f32;
            signal.push(0.3 * (2.0 * std::f32::consts::PI * 400.0 * t).sin());
        }
        // Pause (0.4s)
        signal.extend(vec![0.0f32; (sr as f32 * 0.4) as usize]);
        // Filler-like: short, low frequency, low amplitude (0.3s)
        for i in 0..((sr as f32 * 0.3) as usize) {
            let t = i as f32 / sr as f32;
            signal.push(0.04 * (2.0 * std::f32::consts::PI * 150.0 * t).sin());
        }
        // Pause (0.4s)
        signal.extend(vec![0.0f32; (sr as f32 * 0.4) as usize]);
        // Voiced (1s)
        for i in 0..(sr as usize) {
            let t = i as f32 / sr as f32;
            signal.push(0.3 * (2.0 * std::f32::consts::PI * 350.0 * t).sin());
        }

        let buf = make_test_buffer(signal, sr);
        let result = edit_speech(&buf, 120.0, &ClipConfig::default());

        // Output should be shorter than input (fillers + pauses removed)
        assert!(result.output.duration_secs() < buf.duration_secs());
        assert!(result.removed_secs > 0.0);
    }
}
