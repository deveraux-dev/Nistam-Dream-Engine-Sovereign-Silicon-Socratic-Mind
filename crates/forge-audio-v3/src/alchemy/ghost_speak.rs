//! Ghost Speak — vocal energy envelope extraction via HPSS.
//!
//! Pure DSP. Takes the harmonic component from HPSS and computes
//! a downsampled energy envelope for Ghost Words opacity mapping.

use super::hpss;
use super::stft;
use num_complex::Complex32;

/// Detect regions with vocal-range energy (300Hz-4kHz).
///
/// Returns a list of (start_sample, end_sample) ranges where vocal-frequency
/// energy exceeds the threshold. Uses cheap FFT bandpass — no HPSS needed.
/// Window size is ~1 second. Adjacent hot windows are merged.
pub fn detect_vocal_regions(signal: &[f32], sample_rate: u32, threshold: f32) -> Vec<(usize, usize)> {
    if signal.is_empty() {
        return vec![];
    }

    let frame_size = 2048;
    let hop = 512;
    let bin_hz = sample_rate as f32 / frame_size as f32;
    let vocal_lo = 300.0f32;
    let vocal_hi = 4000.0f32;

    // Cheap STFT — only measure vocal-band energy per frame
    let frames = stft::stft_forward(signal, frame_size, hop);
    let frame_energies: Vec<f32> = frames.iter().map(|frame| {
        let mut energy = 0.0f32;
        for (bin, val) in frame.iter().enumerate() {
            let freq = bin as f32 * bin_hz;
            if freq >= vocal_lo && freq <= vocal_hi {
                energy += val.norm_sqr();
            }
        }
        energy.sqrt()
    }).collect();

    // Normalize
    let peak = frame_energies.iter().cloned().fold(0.0f32, f32::max);
    if peak == 0.0 {
        return vec![];
    }

    // Group frames into ~1-second windows, check if any frame exceeds threshold
    let frames_per_sec = sample_rate as usize / hop;
    let window_size = frames_per_sec.max(1);
    let mut hot_windows: Vec<bool> = Vec::new();
    for chunk in frame_energies.chunks(window_size) {
        let max_e = chunk.iter().cloned().fold(0.0f32, f32::max) / peak;
        hot_windows.push(max_e >= threshold);
    }

    // Merge consecutive hot windows into regions
    let samples_per_window = window_size * hop;
    let mut regions = Vec::new();
    let mut in_region = false;
    let mut start = 0usize;
    for (i, &hot) in hot_windows.iter().enumerate() {
        if hot && !in_region {
            start = i * samples_per_window;
            in_region = true;
        } else if !hot && in_region {
            let end = (i * samples_per_window).min(signal.len());
            regions.push((start, end));
            in_region = false;
        }
    }
    if in_region {
        regions.push((start, signal.len()));
    }

    regions
}

/// Split a mono signal into 3 frequency bands: low (<300Hz), mid (300-5kHz), vocal (HPSS harmonic).
///
/// Low and mid use FFT bin zeroing via STFT (full signal).
/// Vocal uses HPSS only on detected vocal regions (skips intros/outros/breakdowns).
/// Designed for offline use.
pub fn band_split_3way(signal: &[f32], sample_rate: u32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    if signal.is_empty() {
        return (vec![], vec![], vec![]);
    }

    let frame_size = 2048;
    let hop = 512;
    let low_cutoff = 300.0f32;
    let mid_cutoff = 5000.0f32;
    let bin_hz = sample_rate as f32 / frame_size as f32;

    // Forward STFT — low + mid bands (cheap, full signal)
    let frames = stft::stft_forward(signal, frame_size, hop);

    let mut low_frames = Vec::with_capacity(frames.len());
    let mut mid_frames = Vec::with_capacity(frames.len());

    for frame in &frames {
        let mut low = vec![Complex32::new(0.0, 0.0); frame.len()];
        let mut mid = vec![Complex32::new(0.0, 0.0); frame.len()];

        for (bin, &val) in frame.iter().enumerate() {
            let freq = bin as f32 * bin_hz;
            if freq < low_cutoff {
                low[bin] = val;
            } else if freq < mid_cutoff {
                mid[bin] = val;
            }
        }

        low_frames.push(low);
        mid_frames.push(mid);
    }

    let low_signal = stft::stft_inverse(&low_frames, frame_size, hop, signal.len());
    let mid_signal = stft::stft_inverse(&mid_frames, frame_size, hop, signal.len());

    // Vocal = HPSS harmonic, but ONLY on vocal regions (skip silent/instrumental sections)
    let regions = detect_vocal_regions(signal, sample_rate, 0.15);
    let mut vocal_signal = vec![0.0f32; signal.len()];

    if regions.is_empty() {
        // No vocal regions detected — run HPSS on full signal as fallback
        let (harmonic, _) = hpss::hpss_separate(signal, sample_rate);
        return (low_signal, mid_signal, harmonic);
    }

    for (start, end) in &regions {
        let segment = &signal[*start..*end];
        if segment.len() < frame_size {
            // Too short for STFT — copy raw samples
            vocal_signal[*start..*end].copy_from_slice(segment);
            continue;
        }
        let (harmonic, _) = hpss::hpss_separate(segment, sample_rate);
        let copy_len = harmonic.len().min(end - start);
        vocal_signal[*start..*start + copy_len].copy_from_slice(&harmonic[..copy_len]);
    }

    (low_signal, mid_signal, vocal_signal)
}

/// Compute a vocal energy envelope from an audio signal.
///
/// Runs HPSS to isolate the harmonic (vocal) component, then computes
/// RMS energy in windows, downsampled to `num_bins` bins.
///
/// Returns a Vec of f32 values in [0.0, 1.0] representing normalized
/// vocal energy at each bin position.
pub fn vocal_energy_envelope(signal: &[f32], sample_rate: u32, num_bins: usize) -> Vec<f32> {
    if signal.is_empty() || num_bins == 0 {
        return vec![0.0; num_bins];
    }

    // HPSS — harmonic component contains vocals + tonal content
    let (harmonic, _percussive) = hpss::hpss_separate(signal, sample_rate);

    downsample_rms(&harmonic, num_bins)
}

/// Compute RMS energy in `num_bins` equal-width windows, normalized to [0, 1].
/// Public entry point for external callers (commands.rs).
pub fn downsample_rms_pub(signal: &[f32], num_bins: usize) -> Vec<f32> {
    downsample_rms(signal, num_bins)
}

fn downsample_rms(signal: &[f32], num_bins: usize) -> Vec<f32> {
    let n = signal.len();
    if n == 0 || num_bins == 0 {
        return vec![0.0; num_bins];
    }

    let bin_size = n / num_bins;
    if bin_size == 0 {
        return vec![0.0; num_bins];
    }

    let mut envelope = Vec::with_capacity(num_bins);
    for i in 0..num_bins {
        let start = i * bin_size;
        let end = if i == num_bins - 1 { n } else { start + bin_size };
        let rms: f32 = signal[start..end]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / (end - start) as f32;
        envelope.push(rms.sqrt());
    }

    // Normalize to [0, 1]
    let peak = envelope.iter().cloned().fold(0.0f32, f32::max);
    if peak > 0.0 {
        for v in &mut envelope {
            *v /= peak;
        }
    }

    envelope
}

/// Serialize a vocal energy envelope to bytes (f32 little-endian).
pub fn vocal_energy_to_blob(envelope: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(envelope.len() * 4);
    for &v in envelope {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

/// Deserialize a vocal energy envelope from bytes (f32 little-endian).
pub fn vocal_energy_from_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Save a mono f32 signal as 16kHz 16-bit WAV for whisper input.
/// Resamples from source rate to 16000 Hz via linear interpolation.
pub fn save_stem_wav(samples: &[f32], source_rate: u32, path: &str) -> Result<(), String> {
    let target_rate = 16000u32;
    let ratio = source_rate as f64 / target_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;

    let mut resampled = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = samples.get(idx).copied().unwrap_or(0.0);
        let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
        resampled.push(s0 + frac * (s1 - s0));
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: target_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("Failed to create WAV: {e}"))?;
    for &s in &resampled {
        let i16_val = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(i16_val).map_err(|e| format!("WAV write error: {e}"))?;
    }
    writer.finalize().map_err(|e| format!("WAV finalize error: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downsample_rms_basic() {
        // 1000 samples of a sine wave, downsample to 10 bins
        let signal: Vec<f32> = (0..1000)
            .map(|i| (2.0 * std::f32::consts::PI * 4.0 * i as f32 / 1000.0).sin())
            .collect();
        let envelope = downsample_rms(&signal, 10);
        assert_eq!(envelope.len(), 10);
        // All bins should have similar energy for a uniform sine
        for v in &envelope {
            assert!(*v > 0.5, "Bin energy {} should be > 0.5 for uniform sine", v);
        }
    }

    #[test]
    fn test_downsample_rms_silence_then_signal() {
        let mut signal = vec![0.0f32; 500];
        signal.extend(vec![0.5f32; 500]);
        let envelope = downsample_rms(&signal, 2);
        assert_eq!(envelope.len(), 2);
        assert!(envelope[0] < 0.01, "Silent half should be near zero");
        assert!((envelope[1] - 1.0).abs() < 0.01, "Signal half should be 1.0 after normalization");
    }

    #[test]
    fn test_blob_roundtrip() {
        let envelope = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let blob = vocal_energy_to_blob(&envelope);
        let restored = vocal_energy_from_blob(&blob);
        assert_eq!(envelope, restored);
    }

    #[test]
    fn test_vocal_energy_envelope_empty() {
        let envelope = vocal_energy_envelope(&[], 44100, 200);
        assert_eq!(envelope.len(), 200);
        assert!(envelope.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_band_split_3way_empty() {
        let (low, mid, vocal) = band_split_3way(&[], 44100);
        assert!(low.is_empty());
        assert!(mid.is_empty());
        assert!(vocal.is_empty());
    }

    #[test]
    fn test_band_split_3way_lengths() {
        // 4096 samples of mixed frequencies — enough for 1 STFT frame
        let signal: Vec<f32> = (0..4096)
            .map(|i| {
                let t = i as f32 / 44100.0;
                // 100Hz (low) + 1000Hz (mid) + 200Hz (also low)
                (2.0 * std::f32::consts::PI * 100.0 * t).sin()
                    + (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5
            })
            .collect();
        let (low, mid, vocal) = band_split_3way(&signal, 44100);
        // All outputs should be same length as input
        assert_eq!(low.len(), signal.len());
        assert_eq!(mid.len(), signal.len());
        assert_eq!(vocal.len(), signal.len());
    }

    #[test]
    fn test_detect_vocal_regions_empty() {
        let regions = detect_vocal_regions(&[], 44100, 0.15);
        assert!(regions.is_empty());
    }

    #[test]
    fn test_detect_vocal_regions_silence() {
        let signal = vec![0.0f32; 44100 * 5]; // 5 seconds of silence
        let regions = detect_vocal_regions(&signal, 44100, 0.15);
        assert!(regions.is_empty());
    }

    #[test]
    fn test_detect_vocal_regions_finds_vocal_band() {
        // 3 seconds: 1s silence + 1s 1kHz tone (vocal band) + 1s silence
        let sr = 44100;
        let mut signal = vec![0.0f32; sr * 3];
        for i in sr..(sr * 2) {
            let t = i as f32 / sr as f32;
            signal[i] = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.8;
        }
        let regions = detect_vocal_regions(&signal, sr as u32, 0.15);
        assert!(!regions.is_empty(), "Should detect the 1kHz tone as a vocal region");
        // Region should be roughly in the middle second
        let (start, end) = regions[0];
        assert!(start < sr * 2, "Region should start before 2s mark");
        assert!(end > sr, "Region should extend past 1s mark");
    }
}
