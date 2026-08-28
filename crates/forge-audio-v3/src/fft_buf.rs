//! `AudioFftBuf` — 128-sample ring buffer → 64-bin magnitude FFT.
//!
//! Mirrors the Mixer's per-deck FFT path (mixer.rs:699-1368) as a
//! reusable struct for any mono synthesis lane (e.g. technothesia AudioLane).
//! `push_block` is zero-alloc after construction; the plan and buffers are
//! heap-allocated once at `new()`. Benchmark receipts against this hot path
//! (or any future no_std FFT engine) validate against `fft_bench_receipt.schema.json` (2026-08-22, unimplemented — schema only, no receipts emitted yet).

use std::sync::Arc;
use realfft::RealToComplex;

const RING_LEN: usize = 128;
const BINS: usize = 64;

/// 128-sample ring buffer + realfft forward plan → 64-bin magnitude spectrum.
///
/// Zero-alloc on the hot path after `new()`. Call `push_block` with each
/// synthesized audio block; it returns `true` when a new FFT frame is ready
/// (every 128 samples → ~2.67 ms at 48 kHz, well under 60 Hz frame budget).
pub struct AudioFftBuf {
    ring: [f32; RING_LEN],
    fill: usize,
    plan: Arc<dyn RealToComplex<f32>>,
    in_buf: Vec<f32>,
    out_buf: Vec<realfft::num_complex::Complex32>,
    /// Current 64-bin magnitude spectrum. Updated each time `push_block`
    /// returns `true`. Reads between updates return the last computed frame.
    pub bins: [f32; BINS],
}

impl AudioFftBuf {
    /// Allocate the FFT plan and buffers. Called once at startup (cold path).
    pub fn new() -> Self {
        let mut planner = realfft::RealFftPlanner::<f32>::new();
        let plan = planner.plan_fft_forward(RING_LEN);
        let in_buf = plan.make_input_vec();
        let out_buf = plan.make_output_vec();
        Self {
            ring: [0.0; RING_LEN],
            fill: 0,
            plan,
            in_buf,
            out_buf,
            bins: [0.0; BINS],
        }
    }

    /// Feed mono samples from an audio block. Returns `true` when a complete
    /// 128-sample window was processed and `bins` was updated.
    /// Zero-alloc: samples accumulate in `ring`; FFT runs on `in_buf`/`out_buf`
    /// which are pre-allocated at `new()`.
    pub fn push_block(&mut self, block: &[f32]) -> bool {
        let mut updated = false;
        for &s in block {
            let pos = self.fill % RING_LEN;
            self.ring[pos] = s;
            self.fill += 1;
            if self.fill % RING_LEN == 0 {
                self.in_buf.copy_from_slice(&self.ring);
                if self.plan.process(&mut self.in_buf, &mut self.out_buf).is_ok() {
                    for (bin, c) in self.out_buf.iter().take(BINS).enumerate() {
                        self.bins[bin] = (c.re * c.re + c.im * c.im).sqrt();
                    }
                    updated = true;
                }
            }
        }
        updated
    }
}

impl Default for AudioFftBuf {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_block_produces_zero_bins() {
        let mut buf = AudioFftBuf::new();
        let block = vec![0.0f32; 128];
        buf.push_block(&block);
        assert!(buf.bins.iter().all(|&b| b < 1e-6), "silence → near-zero bins");
    }

    #[test]
    fn push_block_returns_true_after_128_samples() {
        let mut buf = AudioFftBuf::new();
        // 127 samples → no update yet
        let partial = vec![0.0f32; 127];
        assert!(!buf.push_block(&partial));
        // 1 more → triggers FFT
        assert!(buf.push_block(&[0.0f32; 1]));
    }

    #[test]
    fn tone_at_quarter_nyquist_dominates_mid_bins() {
        let mut buf = AudioFftBuf::new();
        // Quarter-Nyquist: +1,0,-1,0 repeating (period 4 samples = 12 kHz at 48 kHz).
        // Energy concentrates at bin 32 (12000/48000*128=32), well within 0..64 capture.
        let block: Vec<f32> = (0..128usize).map(|i| match i % 4 {
            0 => 1.0, 2 => -1.0, _ => 0.0,
        }).collect();
        buf.push_block(&block);
        let mid: f32 = buf.bins[24..40].iter().sum();
        let low: f32 = buf.bins[..8].iter().sum();
        assert!(mid > low, "quarter-nyquist tone: mid bins dominate ({mid} vs {low})");
    }
}
