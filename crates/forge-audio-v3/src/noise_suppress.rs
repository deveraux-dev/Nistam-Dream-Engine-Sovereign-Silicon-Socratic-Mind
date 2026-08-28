// @forge:allow_float — DDSP leaf; spectral audio arithmetic is inherently f32.
//! noise_suppress — streaming spectral background suppression (the "NVIDIA RTX
//! Voice" step of the Broadcast Booth).
//!
//! `alchemy::restoration::restore_spectral` is a one-shot over a whole signal;
//! a live booth needs a STREAMING suppressor that tracks the noise floor as the
//! mic runs and gates each STFT frame in flight. This drains that same
//! Minimum-Statistics + geometric-spectral-subtraction math into an overlap-add
//! streaming engine with a single `strength` knob and per-bin gain smoothing
//! (temporal + spectral) to kill musical-noise artifacts — the difference
//! between "noise gate" and the smooth RTX hush.
//!
//! Runs on the logic/record lane (off the RT callback), so heap use at `::new`
//! and per-frame scratch is inside the forge-audio zero-alloc carve-out.

use std::collections::VecDeque;
use std::sync::Arc;

use num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use crate::alchemy::restoration::MinStatistics;

/// Streaming spectral suppressor. Feed arbitrary-length blocks through
/// [`NoiseSuppressor::process_block`]; output is the cleaned signal delayed by `frame_size`
/// (constant priming latency), sample-count preserved.
pub struct NoiseSuppressor {
    frame_size: usize,
    hop: usize,
    n_bins: usize,
    window: Vec<f32>,

    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,

    // Analysis / synthesis scratch (sized once, reused every frame).
    in_frame: Vec<f32>,
    spectrum: Vec<Complex32>,
    time_out: Vec<f32>,
    power: Vec<f32>,

    tracker: MinStatistics,
    smoothed_gain: Vec<f32>,
    sp: Vec<f32>, // variance-reduced (EMA) power spectrum

    // Streaming buffers.
    in_accum: VecDeque<f32>,   // input samples not yet framed
    ola: Vec<f32>,             // overlap-add accumulator (len frame_size)
    ola_norm: Vec<f32>,        // WOLA normalisation accumulator (len frame_size)
    out_ready: VecDeque<f32>,  // finalised output samples awaiting the caller

    strength: f32,     // 0.0 (bypass) .. 1.0 (aggressive hush)
    gain_smooth: f32,  // per-frame temporal smoothing factor
    primed: bool,
}

impl NoiseSuppressor {
    /// `sample_rate` sizes the min-statistics history (~1.5 s). `strength` in
    /// `0.0..=1.0`: 0 = transparent, ~0.85 = the smooth broadcast hush.
    pub fn new(sample_rate: u32, strength: f32) -> Self {
        // 1024/256 = 75% overlap: low latency (~21 ms @ 48 kHz), smooth OLA.
        let frame_size = 1024usize;
        let hop = 256usize;
        let n_bins = frame_size / 2 + 1;

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(frame_size);
        let ifft = planner.plan_fft_inverse(frame_size);

        let window = hann(frame_size);

        // ~1.5 s of history, floored so short sessions still track something.
        let window_frames = (((sample_rate as f32 / hop as f32) * 1.5) as usize).max(16);

        let mut out_ready = VecDeque::with_capacity(frame_size * 4);
        // Prime the output FIFO with the frame_size latency of silence so that
        // every call can always return exactly as many samples as it was given.
        for _ in 0..frame_size {
            out_ready.push_back(0.0);
        }

        let spectrum = fft.make_output_vec();
        let in_frame = fft.make_input_vec();
        let time_out = ifft.make_output_vec();

        Self {
            frame_size,
            hop,
            n_bins,
            window,
            fft,
            ifft,
            in_frame,
            spectrum,
            time_out,
            power: vec![0.0; n_bins],
            tracker: MinStatistics::new(n_bins, window_frames),
            smoothed_gain: vec![1.0; n_bins],
            sp: vec![0.0; n_bins],
            in_accum: VecDeque::with_capacity(frame_size * 2),
            ola: vec![0.0; frame_size],
            ola_norm: vec![0.0; frame_size],
            out_ready,
            strength: strength.clamp(0.0, 1.0),
            gain_smooth: 0.6,
            primed: false,
        }
    }

    /// Suppression aggressiveness, `0.0..=1.0`. 0 hands the signal through clean.
    pub fn set_strength(&mut self, s: f32) {
        self.strength = s.clamp(0.0, 1.0);
    }

    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Pre-learn the noise floor from a captured "room tone" sample (optional —
    /// the streaming tracker also learns continuously). Improves the first second.
    pub fn learn_noise(&mut self, room_tone: &[f32]) {
        let mut pos = 0;
        while pos + self.frame_size <= room_tone.len() {
            for i in 0..self.frame_size {
                self.in_frame[i] = room_tone[pos + i] * self.window[i];
            }
            if self.fft.process(&mut self.in_frame, &mut self.spectrum).is_ok() {
                for k in 0..self.n_bins {
                    self.power[k] = self.spectrum[k].norm_sqr();
                }
                self.tracker.update(&self.power);
            }
            pos += self.hop;
        }
        self.primed = true;
    }

    /// Clean `buf` in place. Output is delayed by `frame_size` samples.
    pub fn process_block(&mut self, buf: &mut [f32]) {
        // Fast bypass — still costs the latency, so keep the FIFO balanced.
        for &s in buf.iter() {
            self.in_accum.push_back(s);
        }
        while self.in_accum.len() >= self.frame_size {
            self.process_one_frame();
            for _ in 0..self.hop {
                self.in_accum.pop_front();
            }
        }
        // Emit exactly buf.len() samples (front-pad with silence if we somehow
        // ran short during priming for an oversized first block).
        let need = buf.len();
        while self.out_ready.len() < need {
            self.out_ready.push_front(0.0);
        }
        for s in buf.iter_mut() {
            *s = self.out_ready.pop_front().unwrap_or(0.0);
        }
    }

    fn process_one_frame(&mut self) {
        // Copy the leading frame_size samples of in_accum into the windowed frame.
        for (i, dst) in self.in_frame.iter_mut().enumerate() {
            *dst = self.in_accum[i] * self.window[i];
        }
        if self.fft.process(&mut self.in_frame, &mut self.spectrum).is_err() {
            return;
        }

        // Track the noise floor from this frame's power spectrum.
        for k in 0..self.n_bins {
            self.power[k] = self.spectrum[k].norm_sqr();
        }
        // Variance-reduced power (EMA) — stabilises the periodogram before the
        // min-statistics floor and the gain decision (this is what turns a harsh
        // spectral gate into the smooth RTX-style hush).
        let beta = 0.6f32;
        for k in 0..self.n_bins {
            self.sp[k] = beta * self.sp[k] + (1.0 - beta) * self.power[k];
        }
        self.tracker.update(&self.sp);
        let noise = self.tracker.noise_floor(); // bias-corrected per-bin floor

        // Berouti-style oversubtraction with a spectral floor, one strength knob.
        // strength 0 -> os 0 and floor 1  =>  gain == 1 (exact bypass).
        // A bin well above the tracked floor (voice) keeps gain ~1; a bin sitting
        // at the floor (background) is pulled down to the spectral floor.
        let os = self.strength * 4.0;
        let floor = (1.0 - self.strength).max(0.02);
        let floor2 = floor * floor;
        for k in 0..self.n_bins {
            let p = self.sp[k].max(1e-12);
            let sub = p - os * noise[k];
            let g2 = (sub / p).clamp(floor2, 1.0);
            let g = g2.sqrt();
            // temporal smoothing to suppress musical noise
            self.smoothed_gain[k] =
                self.gain_smooth * self.smoothed_gain[k] + (1.0 - self.gain_smooth) * g;
        }
        // light spectral (frequency) smoothing — 3-bin moving average
        spectral_smooth(&mut self.smoothed_gain);

        for k in 0..self.n_bins {
            self.spectrum[k] *= self.smoothed_gain[k];
        }
        // realfft: DC and Nyquist bins must stay purely real for the inverse.
        self.spectrum[0].im = 0.0;
        self.spectrum[self.n_bins - 1].im = 0.0;

        if self.ifft.process(&mut self.spectrum, &mut self.time_out).is_err() {
            return;
        }

        // Overlap-add with WOLA normalisation (mirrors alchemy::stft::stft_inverse).
        let scale = 1.0 / self.frame_size as f32;
        for j in 0..self.frame_size {
            self.ola[j] += self.time_out[j] * scale * self.window[j];
            self.ola_norm[j] += self.window[j] * self.window[j];
        }

        // Finalise the leading `hop` samples, then slide the accumulators.
        for j in 0..self.hop {
            let n = self.ola_norm[j];
            let out = if n > 1e-8 { self.ola[j] / n } else { 0.0 };
            self.out_ready.push_back(out);
        }
        self.ola.rotate_left(self.hop);
        self.ola_norm.rotate_left(self.hop);
        let base = self.frame_size - self.hop;
        for j in base..self.frame_size {
            self.ola[j] = 0.0;
            self.ola_norm[j] = 0.0;
        }
    }

    pub fn reset(&mut self) {
        self.in_accum.clear();
        self.out_ready.clear();
        for _ in 0..self.frame_size {
            self.out_ready.push_back(0.0);
        }
        self.ola.iter_mut().for_each(|x| *x = 0.0);
        self.ola_norm.iter_mut().for_each(|x| *x = 0.0);
        self.smoothed_gain.iter_mut().for_each(|x| *x = 1.0);
        self.sp.iter_mut().for_each(|x| *x = 0.0);
    }
}

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            0.5 * (1.0 - phase.cos())
        })
        .collect()
}

/// In-place 3-bin moving average over the gain curve (kills musical noise).
fn spectral_smooth(gain: &mut [f32]) {
    let n = gain.len();
    if n < 3 {
        return;
    }
    let mut prev = gain[0];
    for k in 1..n - 1 {
        let cur = gain[k];
        let avg = (prev + cur + gain[k + 1]) / 3.0;
        prev = cur;
        gain[k] = avg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|&s| s * s).sum::<f32>() / b.len().max(1) as f32).sqrt()
    }

    // Deterministic pseudo-noise so the test is reproducible.
    fn pseudo_noise(n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let x = (i as f32 * 12.9898).sin() * 43758.547;
                amp * (2.0 * (x - x.floor()) - 1.0)
            })
            .collect()
    }

    #[test]
    fn bypass_at_zero_strength_reconstructs() {
        let sr = 48_000;
        let sig: Vec<f32> = (0..8192)
            .map(|i| 0.4 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let mut ns = NoiseSuppressor::new(sr, 0.0);
        let mut out = sig.clone();
        ns.process_block(&mut out);
        // Compare accounting for the frame_size (1024) latency.
        let lat = 1024usize;
        let a = &sig[1024..sig.len() - lat];
        let b = &out[1024 + lat..];
        let err: f32 = a
            .iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            / a.len() as f32;
        assert!(err.sqrt() < 0.05, "zero-strength OLA should reconstruct: rms err {}", err.sqrt());
    }

    #[test]
    fn suppresses_broadband_noise() {
        let sr = 48_000;
        // A stationary broadband bed — pure background, no signal.
        let noise = pseudo_noise(48_000, 0.2);
        let before = rms(&noise[2048..]);
        let mut ns = NoiseSuppressor::new(sr, 0.9);
        let mut out = noise.clone();
        ns.process_block(&mut out);
        // Skip the priming region; the steady-state hush should be deep.
        let after = rms(&out[12_000..]);
        assert!(
            after < 0.5 * before,
            "streaming suppressor should hush a pure noise bed: {before} -> {after}"
        );
    }

    #[test]
    fn preserves_syllabic_voice_over_noise() {
        let sr = 48_000;
        // A 440 Hz "voice" gated into ~3 Hz syllables (clear gaps let the floor
        // tracker learn the background), over a light broadband bed.
        let sf = sr as f32;
        let mixed: Vec<f32> = (0..48_000)
            .map(|i| {
                let t = i as f32 / sf;
                let syllable = (2.0 * std::f32::consts::PI * 3.0 * t).sin().max(0.0);
                let tone = 0.7 * syllable * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
                let x = (i as f32 * 12.9898).sin() * 43758.547;
                let bed = 0.03 * (2.0 * (x - x.floor()) - 1.0);
                tone + bed
            })
            .collect();
        let voiced_before = rms(&mixed[4096..44_000]);
        let mut ns = NoiseSuppressor::new(sr, 0.8);
        let mut out = mixed.clone();
        ns.process_block(&mut out);
        // Latency is 1024 samples; the loud syllables must survive the hush.
        let voiced_after = rms(&out[5120..44_000]);
        assert!(
            voiced_after > 0.5 * voiced_before,
            "syllabic voice must survive suppression: {voiced_before} -> {voiced_after}"
        );
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn preserves_sample_count() {
        let sr = 48_000;
        let mut ns = NoiseSuppressor::new(sr, 0.8);
        for block_len in [133usize, 512, 2048, 300] {
            let mut buf = vec![0.1f32; block_len];
            ns.process_block(&mut buf);
            assert_eq!(buf.len(), block_len, "block length must be preserved");
        }
    }
}
