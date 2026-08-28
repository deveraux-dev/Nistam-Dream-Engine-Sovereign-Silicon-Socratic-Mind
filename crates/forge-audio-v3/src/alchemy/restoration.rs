//! Spectral restoration — geometric spectral subtraction with minimum statistics noise estimation.

/// Minimum Statistics noise floor estimator.
///
/// Tracks the minimum power spectrum over a sliding window (~1.5s).
/// Since archival recordings have no clean silence, we estimate noise from the
/// quietest moments in each frequency band.
///
/// Uses asymmetric smoothing: fast drop to follow quiet moments, slow rise to
/// avoid signal bursts inflating the noise estimate. The noise floor is the
/// per-bin minimum across the window, with bias correction.
pub struct MinStatistics {
    n_bins: usize,
    window_size: usize,
    history: Vec<Vec<f32>>,
    write_pos: usize,
    smoothed: Vec<f32>,
    alpha: f32,
    frames_seen: usize,
}

impl MinStatistics {
    pub fn new(n_bins: usize, window_frames: usize) -> Self {
        Self {
            n_bins,
            window_size: window_frames,
            history: vec![vec![f32::MAX; n_bins]; window_frames],
            write_pos: 0,
            smoothed: vec![0.0; n_bins],
            alpha: 0.1,
            frames_seen: 0,
        }
    }

    /// Feed a new power spectrum frame into the tracker.
    pub fn update(&mut self, power: &[f32]) {
        assert_eq!(power.len(), self.n_bins);
        if self.frames_seen == 0 {
            self.smoothed.copy_from_slice(power);
        } else {
            for k in 0..self.n_bins {
                let target = power[k];
                if target < self.smoothed[k] {
                    self.smoothed[k] = 0.5 * target + 0.5 * self.smoothed[k];
                } else {
                    self.smoothed[k] =
                        self.alpha * target + (1.0 - self.alpha) * self.smoothed[k];
                }
            }
        }
        self.history[self.write_pos].copy_from_slice(&self.smoothed);
        self.write_pos = (self.write_pos + 1) % self.window_size;
        self.frames_seen += 1;
    }

    /// Return the current noise floor estimate with bias correction.
    /// The bias (1.5x) compensates for minimum statistics underestimation.
    pub fn noise_floor(&self) -> Vec<f32> {
        let mut floor = self.raw_floor();
        let bias = 1.5;
        for k in 0..self.n_bins {
            floor[k] *= bias;
        }
        floor
    }

    /// Return the raw minimum across the sliding window (no bias correction).
    fn raw_floor(&self) -> Vec<f32> {
        let mut floor = vec![f32::MAX; self.n_bins];
        let frames_stored = self.frames_seen.min(self.window_size);
        for i in 0..frames_stored {
            for k in 0..self.n_bins {
                floor[k] = floor[k].min(self.history[i][k]);
            }
        }
        floor
    }
}

/// Full spectral restoration pipeline.
/// Takes a noisy mono signal, returns a cleaned mono signal.
///
/// Uses MinStatistics for per-bin noise floor estimation, then applies
/// geometric spectral subtraction with conservative undersubtraction to
/// avoid distorting tonal content while still reducing broadband noise.
pub fn restore_spectral(signal: &[f32], sample_rate: u32) -> Vec<f32> {
    use crate::alchemy::stft;

    let frame_size = 2048;
    let hop = 512;

    let frames = stft::stft_forward(signal, frame_size, hop);
    let n_bins = frame_size / 2 + 1;

    // Compute power spectra
    let power_frames: Vec<Vec<f32>> = frames
        .iter()
        .map(|f| f.iter().map(|c| c.norm_sqr()).collect())
        .collect();

    // MinStatistics noise floor estimation
    let window_frames = ((sample_rate as f32 / hop as f32) * 1.5) as usize;
    let mut tracker = MinStatistics::new(n_bins, window_frames.max(10));
    for power in &power_frames {
        tracker.update(power);
    }
    let noise_floor = tracker.raw_floor();

    // Subtraction factor: controls aggressiveness.
    // Lower values preserve more signal but remove less noise.
    // For archival recordings where we want to preserve tonal content,
    // a conservative factor prevents musical noise artifacts.
    let subtraction_alpha = 0.08;

    // Apply geometric spectral subtraction
    let mut cleaned_frames = Vec::with_capacity(frames.len());
    for frame in &frames {
        let mut out = frame.clone();
        for k in 0..n_bins {
            let y_power = frame[k].norm_sqr();
            let noise_est = noise_floor[k].max(1e-10);
            let gamma = y_power / noise_est;

            // Parametric spectral subtraction:
            // gain = sqrt(max(0, 1 - alpha / gamma))
            //
            // When gamma >> 1 (signal well above noise): gain → 1.0
            // When gamma ≈ 1 (at noise floor): gain = sqrt(1 - alpha) ≈ 0.96
            // When gamma < alpha (below noise floor): gain = spectral floor
            let gain = if gamma > subtraction_alpha {
                (1.0 - subtraction_alpha / gamma).sqrt()
            } else {
                0.05
            };
            out[k] = frame[k] * gain;
        }
        cleaned_frames.push(out);
    }

    stft::stft_inverse(&cleaned_frames, frame_size, hop, signal.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_stats_constant_noise() {
        let n_bins = 1025;
        let noise_power = vec![0.01f32; n_bins];
        let window_frames = 10;
        let mut tracker = MinStatistics::new(n_bins, window_frames);
        for _ in 0..20 {
            tracker.update(&noise_power);
        }
        let estimate = tracker.noise_floor();
        for i in 0..n_bins {
            assert!(
                (estimate[i] - 0.01 * 1.5).abs() < 0.005,
                "Bin {}: got {} expected ~0.015",
                i,
                estimate[i]
            );
        }
    }

    #[test]
    fn test_min_stats_signal_plus_noise() {
        let n_bins = 1025;
        let window_frames = 10;
        let mut tracker = MinStatistics::new(n_bins, window_frames);
        for frame in 0..30 {
            let mut power = vec![0.01f32; n_bins];
            if frame % 3 == 0 {
                for k in 18..23 {
                    power[k] = 1.0;
                }
            }
            tracker.update(&power);
        }
        let estimate = tracker.noise_floor();
        for k in 18..23 {
            assert!(
                estimate[k] < 0.1,
                "Bin {}: noise estimate {} should be near floor, not signal",
                k,
                estimate[k]
            );
        }
    }

    #[test]
    fn test_spectral_subtraction_reduces_noise() {
        let sr = 44100;
        let n = 44100;
        let signal: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let noisy: Vec<f32> = signal
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let noise = (i as f32 * 0.7123 + 0.3).sin() * 0.1;
                s + noise
            })
            .collect();
        let cleaned = restore_spectral(&noisy, sr as u32);
        let rms_before: f32 =
            noisy.iter().zip(&signal).map(|(n, s)| (n - s).powi(2)).sum::<f32>() / n as f32;
        let rms_after: f32 =
            cleaned.iter().zip(&signal).map(|(c, s)| (c - s).powi(2)).sum::<f32>() / n as f32;
        assert!(
            rms_after < rms_before,
            "Restoration should reduce noise: before={:.6} after={:.6}",
            rms_before,
            rms_after
        );
    }
}
