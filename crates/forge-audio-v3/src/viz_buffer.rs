//! AudioVizBuffer — lock-free shared memory between audio and render threads.
//!
//! Audio thread writes PCM samples, RMS, peak, phase. Render thread reads
//! for waveform, VU meters, phase display. Zero mutex, zero allocation on hot path.
//!
//! Ported from ironroot-edict quarry (2026-05-18 13forge-baseline).

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

/// Atomic f32 wrapper (store/load via u32 bit pattern).
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self { Self(AtomicU32::new(v.to_bits())) }
    pub fn load(&self) -> f32 { f32::from_bits(self.0.load(Ordering::Relaxed)) }
    pub fn store(&self, v: f32) { self.0.store(v.to_bits(), Ordering::Relaxed); }
}

/// Single-producer single-consumer lock-free ring buffer.
/// Audio thread pushes, render thread reads latest. Lossy on overflow.
/// INVARIANT: only one thread may call push_slice at a time (SPSC).
/// Generic since 2026-07-31: the f32 sample ring is the audio SPECIALIZATION
/// ([`SampleRing`]), not the only shape. A host ring over its own `Copy` payload
/// (tuning constants, control frames) is the same buffer, one home.
pub struct AtomicRingBuffer<T: Copy + Default = f32> {
    buf: Box<[T]>,
    capacity: usize,
    head: AtomicU64,
}

/// The audio sample ring — the f32 specialization the viz buffer runs on.
pub type SampleRing = AtomicRingBuffer<f32>;

impl<T: Copy + Default> AtomicRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two().max(256);
        let buf = vec![T::default(); cap].into_boxed_slice(); // @forge:allow_alloc -- one boxed slice at construction; push/read never allocate
        Self { buf, capacity: cap, head: AtomicU64::new(0) }
    }

    /// Push values. Producer thread only. No allocation on hot path.
    #[allow(unsafe_code)]
    pub fn push_slice(&self, data: &[T]) {
        let head = self.head.load(Ordering::Relaxed) as usize;
        let mask = self.capacity - 1;
        for (i, &value) in data.iter().enumerate() {
            let idx = (head + i) & mask;
            // SAFETY: SPSC invariant — single-producer contract enforced at API level.
            // idx is masked to 0..capacity (power of 2 wrap), always in bounds.
            // self.buf is initialized to capacity elements, never reallocated (Box).
            // Only one thread calls push_slice() at a time, so no data race.
            unsafe {
                let ptr = self.buf.as_ptr() as *mut T;
                ptr.add(idx).write(value);
            }
        }
        self.head.store((head + data.len()) as u64, Ordering::Release);
    }

    /// Read the most recent `count` values. Consumer thread only.
    pub fn read_latest(&self, count: usize, out: &mut [T]) -> usize {
        let head = self.head.load(Ordering::Acquire) as usize;
        let n = count.min(self.capacity).min(out.len());
        let mask = self.capacity - 1;
        let start = head.saturating_sub(n);
        let actual = head.saturating_sub(start);
        for i in 0..actual {
            let idx = (start + i) & mask;
            out[i] = self.buf[idx];
        }
        actual
    }

    pub fn capacity(&self) -> usize { self.capacity }
}

/// Shared visualization buffer between audio and render threads.
pub struct AudioVizBuffer {
    /// PCM stereo interleaved samples.
    pub samples: AtomicRingBuffer,
    /// FFT magnitude bins.
    pub fft_bins: AtomicRingBuffer,
    /// RMS levels (300ms integration window).
    pub rms_left: AtomicF32,
    pub rms_right: AtomicF32,
    /// Peak levels (1.7s hold, then decay).
    pub peak_left: AtomicF32,
    pub peak_right: AtomicF32,
    /// Phase correlation coefficient (-1.0 to 1.0).
    pub phase_correlation: AtomicF32,
    /// Audio thread clock (microseconds since start).
    pub audio_clock_us: AtomicU64,
    /// Game tick clock (microseconds since start).
    pub game_tick_us: AtomicU64,
    /// Active lane indicator (0=synth, 1=mixer, 2=playback).
    pub active_lane: AtomicU8,
    /// Buffer underrun counter.
    pub underrun_count: AtomicU32,
    /// Smoothed vibe-modulation state (G-AUDIO-01 wire) — published by
    /// `VibeAudioModulation::store_to_viz`, read by a vibe-monitor overlay.
    pub vibe_fog: AtomicF32,
    pub vibe_aberration: AtomicF32,
    pub vibe_glow: AtomicF32,
    pub vibe_distortion: AtomicF32,
}

impl AudioVizBuffer {
    pub fn new(sample_capacity: usize, fft_capacity: usize) -> Self {
        Self {
            samples: AtomicRingBuffer::new(sample_capacity),
            fft_bins: AtomicRingBuffer::new(fft_capacity),
            rms_left: AtomicF32::new(0.0),
            rms_right: AtomicF32::new(0.0),
            peak_left: AtomicF32::new(0.0),
            peak_right: AtomicF32::new(0.0),
            phase_correlation: AtomicF32::new(0.0),
            audio_clock_us: AtomicU64::new(0),
            game_tick_us: AtomicU64::new(0),
            active_lane: AtomicU8::new(0),
            underrun_count: AtomicU32::new(0),
            vibe_fog: AtomicF32::new(0.0),
            vibe_aberration: AtomicF32::new(0.0),
            vibe_glow: AtomicF32::new(0.0),
            vibe_distortion: AtomicF32::new(0.0),
        }
    }

    /// Compute RMS from the last `window` stereo interleaved samples and store.
    /// Called by audio thread — heap alloc permitted (not in realtime callback).
    pub fn compute_and_store_rms(&self, window: usize) {
        let n = window * 2;
        let mut buf = vec![0.0f32; n];
        let read = self.samples.read_latest(n, &mut buf);
        if read < 2 { return; }
        let (mut sum_l, mut sum_r) = (0.0f64, 0.0f64);
        let mut count = 0usize;
        for chunk in buf[..read].chunks_exact(2) {
            sum_l += (chunk[0] as f64) * (chunk[0] as f64);
            sum_r += (chunk[1] as f64) * (chunk[1] as f64);
            count += 1;
        }
        if count > 0 {
            self.rms_left.store((sum_l / count as f64).sqrt() as f32);
            self.rms_right.store((sum_r / count as f64).sqrt() as f32);
        }
    }

    /// Update peak hold with decay. Call periodically from audio thread.
    pub fn update_peak_hold(&self) {
        const DECAY: f32 = 0.0003;
        let rms_l = self.rms_left.load();
        let rms_r = self.rms_right.load();
        let cur_l = self.peak_left.load();
        let cur_r = self.peak_right.load();
        self.peak_left.store(if rms_l > cur_l { rms_l } else { (cur_l - DECAY).max(0.0) });
        self.peak_right.store(if rms_r > cur_r { rms_r } else { (cur_r - DECAY).max(0.0) });
    }
}

/// Sync health status from audio↔game clock drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus { Green, Yellow, Red }

/// Compute sync status from viz buffer clocks.
pub fn sync_health(viz: &AudioVizBuffer) -> SyncStatus {
    let audio = viz.audio_clock_us.load(Ordering::Relaxed);
    let game = viz.game_tick_us.load(Ordering::Relaxed);
    let drift = audio.abs_diff(game);
    if drift < 10_000 { SyncStatus::Green }
    else if drift < 20_000 { SyncStatus::Yellow }
    else { SyncStatus::Red }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generic lane must actually carry a non-f32 payload — a ring that only
    /// ever sees `SampleRing` has an unexercised type parameter (root#rank).
    #[test]
    fn ring_carries_a_non_f32_copy_payload() {
        #[derive(Clone, Copy, Default, Debug, PartialEq)]
        struct Tuning {
            gain_q: i32,
            tick: u64,
        }

        let rb: AtomicRingBuffer<Tuning> = AtomicRingBuffer::new(4);
        rb.push_slice(&[
            Tuning { gain_q: 7_500, tick: 1 },
            Tuning { gain_q: 10_000, tick: 2 },
        ]);
        let mut out = [Tuning::default(); 2];
        assert_eq!(rb.read_latest(2, &mut out), 2);
        assert_eq!(out[0], Tuning { gain_q: 7_500, tick: 1 });
        assert_eq!(out[1], Tuning { gain_q: 10_000, tick: 2 });
    }

    #[test]
    fn ring_buffer_push_read() {
        let rb = AtomicRingBuffer::new(16);
        rb.push_slice(&[1.0, 2.0, 3.0, 4.0]);
        let mut out = [0.0f32; 4];
        let n = rb.read_latest(4, &mut out);
        assert_eq!(n, 4);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn ring_buffer_wraps() {
        let rb = AtomicRingBuffer::new(4);
        rb.push_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let mut out = [0.0f32; 4];
        let n = rb.read_latest(4, &mut out);
        assert_eq!(n, 4);
        assert_eq!(out[2], 5.0);
        assert_eq!(out[3], 6.0);
    }

    #[test]
    fn atomic_f32_roundtrip() {
        let a = AtomicF32::new(0.42);
        assert!((a.load() - 0.42).abs() < 1e-6);
        a.store(0.99);
        assert!((a.load() - 0.99).abs() < 1e-6);
    }

    #[test]
    fn viz_buffer_rms() {
        let viz = AudioVizBuffer::new(1024, 256);
        let silence = vec![0.0f32; 200];
        viz.samples.push_slice(&silence);
        viz.compute_and_store_rms(100);
        assert_eq!(viz.rms_left.load(), 0.0);
        assert_eq!(viz.rms_right.load(), 0.0);
    }

    #[test]
    fn viz_buffer_peak_decay() {
        let viz = AudioVizBuffer::new(1024, 256);
        viz.peak_left.store(0.5);
        viz.rms_left.store(0.1);
        viz.update_peak_hold();
        assert!(viz.peak_left.load() < 0.5);
    }

    #[test]
    fn sync_health_green() {
        let viz = AudioVizBuffer::new(256, 64);
        viz.audio_clock_us.store(1_000_000, Ordering::Relaxed);
        viz.game_tick_us.store(1_000_500, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Green);
    }

    #[test]
    fn sync_health_red() {
        let viz = AudioVizBuffer::new(256, 64);
        viz.audio_clock_us.store(1_000_000, Ordering::Relaxed);
        viz.game_tick_us.store(1_030_000, Ordering::Relaxed);
        assert_eq!(sync_health(&viz), SyncStatus::Red);
    }
}
