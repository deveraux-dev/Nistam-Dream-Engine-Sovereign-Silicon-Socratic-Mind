//! HPSS — Harmonic-Percussive Source Separation via median filtering.

use num_complex::Complex32;
use crate::alchemy::stft;

/// Insert into an ascending window, keeping it sorted.
fn insert_sorted(window: &mut Vec<f32>, v: f32) {
    let pos = window.partition_point(|&x| x < v);
    window.insert(pos, v);
}

/// Remove one occurrence of `v` from an ascending window.
fn remove_sorted(window: &mut Vec<f32>, v: f32) {
    let pos = window.partition_point(|&x| x < v);
    debug_assert!(pos < window.len(), "sliding median dropped a sample it never held");
    window.remove(pos);
}

/// Running median over a sliding window that stays sorted between samples.
///
/// The window advances by one insert and one remove — both binary-search
/// positioned — instead of copying and re-sorting `kernel_size` samples per
/// output. Compares drop from O(n·k·log k) to O(n·log k); the residual cost is
/// the `Vec::insert`/`remove` memmove, a contiguous block copy the compiler
/// vectorises. Output is bit-identical to the naive filter.
fn running_median(data: &[f32], kernel_size: usize) -> Vec<f32> {
    let half = kernel_size / 2;
    let n = data.len();
    if n == 0 {
        return Vec::new(); // @forge:allow_alloc load-time STFT stage, not the RT callback
    }
    let mut result = vec![0.0f32; n]; // @forge:allow_alloc load-time STFT stage (zero-alloc law = RT callback only)
    let mut window: Vec<f32> = Vec::with_capacity(kernel_size + 1); // @forge:allow_alloc one window for the whole pass

    let mut start = 0usize;
    let mut end = (half + 1).min(n);
    window.extend_from_slice(&data[start..end]);
    window.sort_by(|a, b| a.partial_cmp(b).unwrap());
    result[0] = window[window.len() / 2];

    for i in 1..n {
        let new_start = i.saturating_sub(half);
        let new_end = (i + half + 1).min(n);
        while end < new_end {
            insert_sorted(&mut window, data[end]);
            end += 1;
        }
        while start < new_start {
            remove_sorted(&mut window, data[start]);
            start += 1;
        }
        result[i] = window[window.len() / 2];
    }
    result
}

/// Separate a mono signal into harmonic and percussive components.
/// Returns (harmonic, percussive) as mono f32 vectors.
pub fn hpss_separate(signal: &[f32], sample_rate: u32) -> (Vec<f32>, Vec<f32>) {
    let _ = sample_rate;
    let frame_size = 2048;
    let hop = 512;
    let l_h = 17; // horizontal median filter length (time axis, for harmonic)
    let l_p = 17; // vertical median filter length (frequency axis, for percussive)

    let frames = stft::stft_forward(signal, frame_size, hop);
    let n_bins = frame_size / 2 + 1;
    let n_frames = frames.len();

    if n_frames == 0 {
        return (signal.to_vec(), vec![0.0; signal.len()]);
    }

    // Build magnitude spectrogram: [n_bins][n_frames]
    let mut mag = vec![vec![0.0f32; n_frames]; n_bins];
    for (t, frame) in frames.iter().enumerate() {
        for k in 0..n_bins {
            mag[k][t] = frame[k].norm();
        }
    }

    // Horizontal median filter (across time) → harmonic
    let mut h_mag = vec![vec![0.0f32; n_frames]; n_bins];
    for k in 0..n_bins {
        h_mag[k] = running_median(&mag[k], l_h);
    }

    // Vertical median filter (across frequency) → percussive
    let mut p_mag = vec![vec![0.0f32; n_frames]; n_bins];
    for t in 0..n_frames {
        let col: Vec<f32> = (0..n_bins).map(|k| mag[k][t]).collect();
        let filtered = running_median(&col, l_p);
        for k in 0..n_bins {
            p_mag[k][t] = filtered[k];
        }
    }

    // Wiener-like soft masks with p=2
    let mut h_frames = Vec::with_capacity(n_frames);
    let mut p_frames = Vec::with_capacity(n_frames);
    for t in 0..n_frames {
        let mut h_frame = vec![Complex32::new(0.0, 0.0); n_bins];
        let mut p_frame = vec![Complex32::new(0.0, 0.0); n_bins];
        for k in 0..n_bins {
            let hh = h_mag[k][t] * h_mag[k][t];
            let pp = p_mag[k][t] * p_mag[k][t];
            let denom = hh + pp + 1e-10;
            let mask_h = hh / denom;
            let mask_p = pp / denom;
            h_frame[k] = frames[t][k] * mask_h;
            p_frame[k] = frames[t][k] * mask_p;
        }
        h_frames.push(h_frame);
        p_frames.push(p_frame);
    }

    let harmonic = stft::stft_inverse(&h_frames, frame_size, hop, signal.len());
    let percussive = stft::stft_inverse(&p_frames, frame_size, hop, signal.len());

    (harmonic, percussive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpss_separates_click_from_tone() {
        let sr = 44100;
        let n = 44100;
        let mut signal: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        // Add impulsive click at sample 22050
        for i in 22050..22150 {
            signal[i] += 0.8;
        }

        let (harmonic, percussive) = hpss_separate(&signal, sr as u32);

        let h_energy: f32 = harmonic.iter().map(|s| s * s).sum();
        let p_energy: f32 = percussive.iter().map(|s| s * s).sum();
        assert!(h_energy > p_energy * 2.0,
            "Harmonic energy {} should dominate percussive {}", h_energy, p_energy);

        let click_region_p: f32 = percussive[22000..22200].iter().map(|s| s.abs()).sum();
        let click_region_h: f32 = harmonic[22000..22200].iter().map(|s| s.abs()).sum();
        assert!(click_region_p > click_region_h * 0.5,
            "Percussive click region {} should be significant vs harmonic {}", click_region_p, click_region_h);
    }

    /// The naive filter the sliding window replaced — kept here as the oracle.
    fn naive_median(data: &[f32], kernel_size: usize) -> Vec<f32> {
        let half = kernel_size / 2;
        let n = data.len();
        let mut result = vec![0.0f32; n]; // @forge:allow_alloc test oracle
        for i in 0..n {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(n);
            let mut window: Vec<f32> = data[start..end].to_vec(); // @forge:allow_alloc test oracle
            window.sort_by(|a, b| a.partial_cmp(b).unwrap());
            result[i] = window[window.len() / 2];
        }
        result
    }

    #[test]
    fn sliding_median_is_bit_identical_to_the_naive_filter() {
        // Deterministic pseudo-random field with plateaus, so duplicate samples
        // exercise the equal-value insert/remove path.
        let mut state: u32 = 0x1357_9bdf;
        let mut data = [0.0f32; 1024];
        for slot in data.iter_mut() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *slot = ((state >> 20) % 7) as f32 * 0.25;
        }
        for k in [1usize, 2, 3, 17, 33, 129] {
            let fast = running_median(&data, k);
            let slow = naive_median(&data, k);
            assert_eq!(fast, slow, "sliding median diverged from the oracle at kernel {k}");
        }
        assert!(running_median(&[], 17).is_empty(), "empty input must stay empty");
    }

    #[test]
    fn test_hpss_energy_conservation() {
        let sr = 44100;
        let n = 22050;
        let signal: Vec<f32> = (0..n)
            .map(|i| 0.3 * (2.0 * std::f32::consts::PI * 300.0 * i as f32 / sr as f32).sin())
            .collect();
        let (h, p) = hpss_separate(&signal, sr as u32);
        let orig_energy: f32 = signal.iter().map(|s| s * s).sum();
        let sum_energy: f32 = h.iter().zip(&p).map(|(a, b)| (a + b).powi(2)).sum::<f32>();
        let ratio = sum_energy / orig_energy.max(1e-10);
        assert!(ratio > 0.5 && ratio < 2.0,
            "Energy ratio {} should be roughly 1.0", ratio);
    }
}
