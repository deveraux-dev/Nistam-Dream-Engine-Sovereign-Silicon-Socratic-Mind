//! Formant meter — HPSS + LPC vocal-formant-energy worker fed by the
//! loopback RT callback's `rtrb` producer.
//!
//! Same publish contract as `LoopbackMeter`: one `AtomicU32` Permyriad,
//! relaxed loads, any thread. Unlike the band-split meter, HPSS + LPC are
//! not RT-safe (`hpss_separate` allocates and is non-causal over a window),
//! so this runs off the RT thread entirely: the callback pushes raw mono
//! samples into an `rtrb` ring (best-effort, drops on a full ring, never
//! blocks); a dedicated worker drains it into a rolling buffer and re-runs
//! the analysis on a fixed cadence.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::alchemy::{formant, hpss};

/// RT-side ring capacity — generous headroom over one hop's worth of samples
/// so a slow worker wakeup never forces the callback to drop live audio.
const RING_CAPACITY: usize = 16_384;
/// Rolling analysis window fed to `hpss_separate` each hop.
const ROLLING_SECONDS: f32 = 1.5;
/// Worker wake cadence.
const HOP_MS: u64 = 100;
/// Harmonic-output tail analyzed for formant energy (skips buffer-start
/// median-filter edge artifacts).
const TAIL_MS: f32 = 250.0;
/// Below this many samples, HPSS's 2048-frame STFT can't form a full frame.
const MIN_SAMPLES_FOR_ANALYSIS: usize = 2048;

/// The shared meter — one writer (the worker thread), any readers.
pub struct FormantMeter {
    energy_pmy: AtomicU32,
}

impl FormantMeter {
    /// A zeroed meter, ready to hand to [`spawn`].
    pub fn new() -> Arc<Self> {
        Arc::new(Self { energy_pmy: AtomicU32::new(0) })
    }

    /// Current vocal-formant energy, Permyriad (0..10000) — relaxed load.
    pub fn energy_pmy(&self) -> u32 {
        self.energy_pmy.load(Ordering::Relaxed)
    }
}

/// Fixed-capacity circular sample buffer — O(1) push, no shifting, no
/// per-sample allocation. `snapshot` linearizes oldest-to-newest once per
/// worker hop (a bounded, infrequent copy, not a hot path).
struct RollingBuffer {
    data: Vec<f32>,
    write: usize,
    filled: usize,
}

impl RollingBuffer {
    fn new(cap: usize) -> Self {
        Self { data: vec![0.0; cap.max(1)], write: 0, filled: 0 }
    }

    fn push(&mut self, sample: f32) {
        let cap = self.data.len();
        self.data[self.write] = sample;
        self.write = (self.write + 1) % cap;
        self.filled = (self.filled + 1).min(cap);
    }

    fn snapshot(&self) -> Vec<f32> {
        let cap = self.data.len();
        if self.filled < cap {
            self.data[..self.filled].to_vec() // @forge:allow_alloc worker-hop snapshot, not RT
        } else {
            let mut out = Vec::with_capacity(cap); // @forge:allow_alloc worker-hop snapshot, not RT
            out.extend_from_slice(&self.data[self.write..]);
            out.extend_from_slice(&self.data[..self.write]);
            out
        }
    }
}

/// Create the RT-side producer and spawn the analysis worker. The caller
/// (`loopback.rs`'s audio callback) pushes mono samples into the returned
/// producer; a full ring is dropped silently, never blocks the RT thread.
pub fn spawn(meter: Arc<FormantMeter>, sample_rate: u32) -> rtrb::Producer<f32> {
    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
    let sr = sample_rate.max(8_000);
    std::thread::spawn(move || {
        let cap = ((sr as f32) * ROLLING_SECONDS) as usize;
        let mut rolling = RollingBuffer::new(cap);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(HOP_MS));
            while let Ok(sample) = consumer.pop() {
                rolling.push(sample);
            }
            let window = rolling.snapshot();
            if window.len() < MIN_SAMPLES_FOR_ANALYSIS {
                continue;
            }
            let (harmonic, _percussive) = hpss::hpss_separate(&window, sr);
            let tail_len = (((sr as f32) * TAIL_MS / 1000.0) as usize).max(1);
            let tail_start = harmonic.len().saturating_sub(tail_len);
            let tail = &harmonic[tail_start..];
            let pmy = formant::formant_energy_pmy(tail, sr).clamp(0, 10_000) as u32;
            meter.energy_pmy.store(pmy, Ordering::Relaxed);
        }
    });
    producer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_starts_zero() {
        let m = FormantMeter::new();
        assert_eq!(m.energy_pmy(), 0);
    }

    #[test]
    fn rolling_buffer_linearizes_before_wrap() {
        let mut rb = RollingBuffer::new(4);
        rb.push(1.0);
        rb.push(2.0);
        assert_eq!(rb.snapshot(), vec![1.0, 2.0]);
    }

    #[test]
    fn rolling_buffer_linearizes_after_wrap_oldest_first() {
        let mut rb = RollingBuffer::new(4);
        for v in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
            rb.push(v);
        }
        // Capacity 4, six pushes: the last four values, oldest-to-newest.
        assert_eq!(rb.snapshot(), vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn spawn_returns_a_usable_producer() {
        let meter = FormantMeter::new();
        let mut producer = spawn(meter, 44_100);
        assert!(producer.push(0.1).is_ok());
    }
}
