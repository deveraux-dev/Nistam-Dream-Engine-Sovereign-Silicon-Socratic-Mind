//! BPM detection via autocorrelation of onset envelope.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.

use crate::dsp::AudioBuffer;

const COMMON_BPMS: &[f32] = &[
    60.0, 66.0, 70.0, 72.0, 75.0, 78.0, 80.0, 84.0, 85.0, 88.0, 90.0, 92.0, 95.0, 96.0, 100.0,
    104.0, 108.0, 110.0, 112.0, 115.0, 116.0, 118.0, 120.0, 122.0, 124.0, 126.0, 128.0, 130.0,
    132.0, 134.0, 136.0, 138.0, 140.0, 144.0, 148.0, 150.0, 152.0, 155.0, 156.0, 160.0, 164.0,
    168.0, 170.0, 172.0, 175.0, 176.0, 180.0, 184.0, 188.0, 190.0, 192.0, 196.0, 200.0,
];

/// Detect the BPM of an audio buffer via autocorrelation of the onset envelope.
pub fn detect_bpm(buf: &AudioBuffer) -> f32 {
    let full_mono = buf.to_mono();
    let sr = buf.sample_rate as f32;
    let mono = &full_mono[..];

    let window_samples = (sr * 0.010).round() as usize;
    let hop_samples = window_samples / 2;

    if window_samples == 0 || mono.len() < window_samples {
        return 120.0;
    }

    let num_frames = (mono.len().saturating_sub(window_samples)) / hop_samples + 1;
    let mut envelope = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let start = i * hop_samples;
        let end = (start + window_samples).min(mono.len());
        let energy: f32 = mono[start..end].iter().map(|&s| s * s).sum();
        envelope.push(energy);
    }

    let mut onset = vec![0.0f32; envelope.len()];
    for i in 1..envelope.len() {
        let diff = envelope[i] - envelope[i - 1];
        onset[i] = diff.max(0.0);
    }

    let envelope_sr = sr / hop_samples as f32;
    let min_lag = (envelope_sr * 60.0 / 200.0).round() as usize;
    let max_lag = ((envelope_sr * 60.0 / 60.0).round() as usize).min(onset.len() - 1);

    if min_lag >= max_lag || max_lag >= onset.len() {
        return 120.0;
    }

    let mut best_lag = min_lag;
    let mut best_corr = f32::NEG_INFINITY;
    for lag in min_lag..=max_lag {
        let n = onset.len() - lag;
        let corr: f32 = (0..n).map(|i| onset[i] * onset[i + lag]).sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    let mut bpm = envelope_sr * 60.0 / best_lag as f32;
    if bpm < 80.0 {
        bpm *= 2.0;
    }
    bpm
}

/// Snap a raw BPM value to the nearest common tempo.
pub fn snap_bpm(bpm: f32) -> f32 {
    let mut best = COMMON_BPMS[0];
    let mut best_dist = (bpm - best).abs();
    for &t in &COMMON_BPMS[1..] {
        let d = (bpm - t).abs();
        if d < best_dist {
            best_dist = d;
            best = t;
        }
    }
    best
}

/// Beat grid: evenly-spaced beat positions derived from BPM + first-beat offset.
#[derive(Clone, Debug)]
pub struct BeatGrid {
    pub bpm: f32, // @forge:allow_float
    pub first_beat: usize,
    pub beat_interval: usize,
    pub beat_count: usize,
}

impl BeatGrid {
    pub fn from_bpm(bpm: f32, first_beat: usize, sample_rate: u32, total_samples: usize) -> Self {
        let interval = (sample_rate as f64 * 60.0 / bpm as f64) as usize;
        let count = total_samples
            .saturating_sub(first_beat)
            .checked_div(interval)
            .unwrap_or(0);
        Self { bpm, first_beat, beat_interval: interval.max(1), beat_count: count }
    }

    pub fn beat_pos(&self, n: usize) -> usize {
        self.first_beat + n * self.beat_interval
    }

    pub fn phase_at(&self, pos: usize) -> f32 {
        if pos < self.first_beat || self.beat_interval == 0 {
            return 0.0;
        }
        let offset = pos - self.first_beat;
        (offset % self.beat_interval) as f32 / self.beat_interval as f32
    }

    pub fn nearest_beat(&self, pos: usize) -> usize {
        if pos <= self.first_beat {
            return 0;
        }
        let offset = pos - self.first_beat;
        (offset + self.beat_interval / 2) / self.beat_interval
    }

    pub fn snap_to_beat(&self, pos: usize) -> usize {
        let beat_idx = self.nearest_beat(pos);
        self.beat_pos(beat_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_bpm_nearest() {
        assert_eq!(snap_bpm(120.4), 120.0);
        assert_eq!(snap_bpm(121.5), 122.0);
        assert_eq!(snap_bpm(128.0), 128.0);
        assert_eq!(snap_bpm(61.0), 60.0);
        assert_eq!(snap_bpm(199.5), 200.0);
    }

    #[test]
    fn beat_grid_120bpm_48k() {
        let g = BeatGrid::from_bpm(120.0, 0, 48_000, 48_000);
        assert_eq!(g.beat_interval, 24_000);
        assert_eq!(g.beat_pos(2), 48_000);
    }
}
