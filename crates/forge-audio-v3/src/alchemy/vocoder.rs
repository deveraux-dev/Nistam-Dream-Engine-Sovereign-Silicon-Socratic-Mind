//! Phase Vocoder — pitch-preserving time stretch with phase locking.

use num_complex::Complex32;
use crate::alchemy::stft;

/// Time-stretch a mono signal by `stretch_factor` without changing pitch.
pub fn phase_vocoder(signal: &[f32], sample_rate: u32, stretch_factor: f32) -> Vec<f32> {
    let frame_size = 2048;
    let hop_a = 512; // analysis hop
    let hop_s = (hop_a as f32 * stretch_factor) as usize; // synthesis hop
    let n_bins = frame_size / 2 + 1;

    let frames = stft::stft_forward(signal, frame_size, hop_a);
    let n_frames = frames.len();

    if n_frames < 2 {
        return signal.to_vec();
    }

    let _ = sample_rate; // reserved for future sample-rate-dependent processing

    let mut synth_frames: Vec<Vec<Complex32>> = Vec::with_capacity(n_frames);
    let mut prev_phase_a = vec![0.0f32; n_bins];
    let mut synth_phase = vec![0.0f32; n_bins];

    let omega: Vec<f32> = (0..n_bins)
        .map(|k| 2.0 * std::f32::consts::PI * k as f32 / frame_size as f32 * hop_a as f32)
        .collect();

    for (idx, frame) in frames.iter().enumerate() {
        let mag: Vec<f32> = frame.iter().map(|c| c.norm()).collect();
        let phase_a: Vec<f32> = frame.iter().map(|c| c.arg()).collect();

        if idx == 0 {
            synth_phase = phase_a.clone();
        } else {
            for k in 0..n_bins {
                let dp = phase_a[k] - prev_phase_a[k] - omega[k];
                let dp_wrapped = dp - (dp / (2.0 * std::f32::consts::PI)).round()
                    * 2.0 * std::f32::consts::PI;
                let inst_freq = omega[k] + dp_wrapped;
                synth_phase[k] += inst_freq * (hop_s as f32 / hop_a as f32);
            }

            // Phase locking (Laroche-Dolson)
            let peaks = find_spectral_peaks(&mag);
            for &peak in &peaks {
                let peak_rotation = synth_phase[peak] - phase_a[peak];
                let start = peak.saturating_sub(3);
                let end = (peak + 4).min(n_bins);
                for k in start..end {
                    if k != peak {
                        synth_phase[k] = phase_a[k] + peak_rotation;
                    }
                }
            }
        }

        prev_phase_a = phase_a;

        let mut synth_frame: Vec<Complex32> = mag.iter().zip(&synth_phase)
            .map(|(&m, &p)| Complex32::from_polar(m, p))
            .collect();
        // realfft requires DC and Nyquist bins to be purely real
        synth_frame[0] = Complex32::new(synth_frame[0].re, 0.0);
        if let Some(last) = synth_frame.last_mut() {
            *last = Complex32::new(last.re, 0.0);
        }
        synth_frames.push(synth_frame);
    }

    let output_len = (n_frames - 1) * hop_s + frame_size;
    stft::stft_inverse(&synth_frames, frame_size, hop_s, output_len)
}

fn find_spectral_peaks(mag: &[f32]) -> Vec<usize> {
    let mut peaks = Vec::new();
    for i in 1..mag.len() - 1 {
        if mag[i] > mag[i - 1] && mag[i] > mag[i + 1] && mag[i] > 0.001 {
            peaks.push(i);
        }
    }
    peaks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pvoc_identity() {
        let sr = 44100;
        let n = 22050;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let result = phase_vocoder(&signal, sr as u32, 1.0);
        let len_ratio = result.len() as f32 / signal.len() as f32;
        assert!((len_ratio - 1.0).abs() < 0.1, "Length ratio {} should be ~1.0", len_ratio);
        let orig_e: f32 = signal.iter().map(|s| s * s).sum();
        let result_e: f32 = result.iter().map(|s| s * s).sum();
        let e_ratio = result_e / orig_e;
        assert!(e_ratio > 0.7 && e_ratio < 1.3, "Energy ratio {} should be ~1.0", e_ratio);
    }

    #[test]
    fn test_pvoc_stretch_doubles_length() {
        let sr = 44100;
        let n = 22050;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let result = phase_vocoder(&signal, sr as u32, 2.0);
        let len_ratio = result.len() as f32 / signal.len() as f32;
        assert!((len_ratio - 2.0).abs() < 0.2, "Length ratio {} should be ~2.0", len_ratio);
    }

    #[test]
    fn test_pvoc_preserves_pitch() {
        use crate::alchemy::pitch;
        let sr = 44100u32;
        let n = 44100;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let stretched = phase_vocoder(&signal, sr, 1.5);
        let pitches = pitch::yin_track(&stretched, sr, 2048, 512, 0.15);
        let valid: Vec<f32> = pitches.iter().filter(|&&p| p > 100.0).copied().collect();
        if !valid.is_empty() {
            let avg: f32 = valid.iter().sum::<f32>() / valid.len() as f32;
            assert!((avg - 440.0).abs() < 20.0,
                "Pitch {} should be preserved at ~440Hz", avg);
        }
    }
}
