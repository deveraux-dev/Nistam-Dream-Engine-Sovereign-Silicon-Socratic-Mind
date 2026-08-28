//! STFT — Short-Time Fourier Transform using realfft.

use num_complex::Complex32;
use realfft::RealFftPlanner;

/// Precomputed Hann window of length N.
fn hann_window(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            0.5 * (1.0 - phase.cos())
        })
        .collect()
}

/// Forward STFT. Returns Vec of complex spectral frames, each of length N/2+1.
pub fn stft_forward(signal: &[f32], frame_size: usize, hop: usize) -> Vec<Vec<Complex32>> {
    let window = hann_window(frame_size);
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(frame_size);
    let mut frames = Vec::new();
    let mut pos = 0;
    while pos + frame_size <= signal.len() {
        let mut buf: Vec<f32> = signal[pos..pos + frame_size]
            .iter()
            .zip(&window)
            .map(|(s, w)| s * w)
            .collect();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut buf, &mut spectrum).expect("FFT failed");
        frames.push(spectrum);
        pos += hop;
    }
    frames
}

/// Inverse STFT via overlap-add. Reconstructs time-domain signal from spectral frames.
pub fn stft_inverse(
    frames: &[Vec<Complex32>],
    frame_size: usize,
    hop: usize,
    output_len: usize,
) -> Vec<f32> {
    let window = hann_window(frame_size);
    let mut planner = RealFftPlanner::<f32>::new();
    let ifft = planner.plan_fft_inverse(frame_size);

    let total_len = (frames.len() - 1) * hop + frame_size;
    let mut output = vec![0.0f32; total_len];
    let mut norm = vec![0.0f32; total_len];

    for (idx, spectrum) in frames.iter().enumerate() {
        let mut spec = spectrum.clone();
        let mut time_buf = ifft.make_output_vec();
        ifft.process(&mut spec, &mut time_buf).expect("IFFT failed");

        let pos = idx * hop;
        let scale = 1.0 / frame_size as f32;
        for j in 0..frame_size {
            if pos + j < total_len {
                output[pos + j] += time_buf[j] * scale * window[j];
                norm[pos + j] += window[j] * window[j];
            }
        }
    }

    for i in 0..total_len {
        if norm[i] > 1e-8 {
            output[i] /= norm[i];
        }
    }

    output.truncate(output_len);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_sine_peak() {
        let sr = 44100;
        let n = 4096;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let frames = stft_forward(&signal, 2048, 512);
        assert!(frames.len() > 1);
        assert_eq!(frames[0].len(), 1025);
        let mags: Vec<f32> = frames[1].iter().map(|c| c.norm()).collect();
        let peak_bin = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert!(
            (peak_bin as i32 - 20).abs() <= 1,
            "Peak at bin {} expected ~20",
            peak_bin
        );
    }

    #[test]
    fn test_stft_frame_count() {
        let signal = vec![0.0f32; 8192];
        let frames = stft_forward(&signal, 2048, 512);
        assert_eq!(frames.len(), 13);
    }

    #[test]
    fn test_stft_roundtrip() {
        let sr = 44100;
        let n = 8192;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let frame_size = 2048;
        let hop = 512;
        let frames = stft_forward(&signal, frame_size, hop);
        let reconstructed = stft_inverse(&frames, frame_size, hop, signal.len());
        let start = frame_size;
        let end = signal.len() - frame_size;
        for i in start..end {
            assert!(
                (reconstructed[i] - signal[i]).abs() < 0.01,
                "Mismatch at {}: got {} expected {}",
                i, reconstructed[i], signal[i]
            );
        }
    }
}
