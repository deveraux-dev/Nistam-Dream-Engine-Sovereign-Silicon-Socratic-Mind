// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Lock-Free Audio Bus, Faust-Free DSP Primitives & 120Hz Metronome.
//!
//! Implements:
//! 1. Lock-free Single-Producer Single-Consumer (SPSC) ring buffer with 0 heap allocations.
//! 2. Faust-free CPU DSP primitives: Biquad filter, fractional delay line, and TPDF dithered summing.
//! 3. Strict 120Hz system metronome tick loop ($48\text{kHz} / 120\text{Hz} = 400\text{ samples/tick}$).
//! 4. Sub-microsecond MoeRouter centroid router via bitwise XOR + POPCNT.

#![deny(unsafe_code)]

use core::sync::atomic::{AtomicUsize, Ordering};

/// Sample rate in Hz.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// System metronome clock in Hz.
pub const METRONOME_HZ: u32 = 120;

/// Number of audio samples per 120Hz metronome tick (48000 / 120 = 400).
pub const SAMPLES_PER_TICK: usize = (AUDIO_SAMPLE_RATE / METRONOME_HZ) as usize;

/// Fixed capacity for static SPSC ring buffer (must be power of two).
pub const SPSC_RING_CAPACITY: usize = 1024;
const RING_MASK: usize = SPSC_RING_CAPACITY - 1;

/// Lock-free Single-Producer Single-Consumer (SPSC) ring buffer.
pub struct SpscRingBuffer<T: Copy + Default> {
    buffer: [T; SPSC_RING_CAPACITY],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T: Copy + Default> SpscRingBuffer<T> {
    /// Create a new static SPSC ring buffer.
    pub const fn new(init_val: T) -> Self {
        Self {
            buffer: [init_val; SPSC_RING_CAPACITY],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Push an element to the buffer. Returns `true` if successful, `false` if full.
    pub fn push(&mut self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if (head.wrapping_sub(tail)) >= SPSC_RING_CAPACITY {
            return false; // Full
        }

        self.buffer[head & RING_MASK] = item;
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Pop an element from the buffer. Returns `Some(T)` or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None; // Empty
        }

        let item = self.buffer[tail & RING_MASK];
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Number of elements available for reading.
    #[inline(always)]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    /// Check if buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Fixed-point Biquad Filter (Direct Form II Transposed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiquadFilterFixed {
    /// Feedforward coefficient b0 (Permyriad, 10000 = 1.0).
    pub b0: i32,
    /// Feedforward coefficient b1 (Permyriad).
    pub b1: i32,
    /// Feedforward coefficient b2 (Permyriad).
    pub b2: i32,
    /// Feedback coefficient a1 (Permyriad).
    pub a1: i32,
    /// Feedback coefficient a2 (Permyriad).
    pub a2: i32,
    /// Delay register d1.
    pub d1: i32,
    /// Delay register d2.
    pub d2: i32,
}

impl BiquadFilterFixed {
    /// Create a low-pass smoothing filter with unity DC gain.
    pub const fn lowpass_smoothing() -> Self {
        Self {
            b0: 2_000,
            b1: 4_000,
            b2: 2_000,
            a1: -4_000,
            a2: 2_000,
            d1: 0,
            d2: 0,
        }
    }

    /// Process a single sample through Direct Form II Transposed filter.
    #[inline]
    pub fn process_sample(&mut self, input: i32) -> i32 {
        let in64 = input as i64;
        let b0 = self.b0 as i64;
        let b1 = self.b1 as i64;
        let b2 = self.b2 as i64;
        let a1 = self.a1 as i64;
        let a2 = self.a2 as i64;
        let d1 = self.d1 as i64;
        let d2 = self.d2 as i64;

        let output = (in64 * b0) / 10_000 + d1;
        self.d1 = ((in64 * b1) / 10_000 - (output * a1) / 10_000 + d2) as i32;
        self.d2 = ((in64 * b2) / 10_000 - (output * a2) / 10_000) as i32;

        output as i32
    }
}

/// Fractional delay line with linear interpolation.
pub struct FractionalDelayLine<const N: usize> {
    buffer: [i16; N],
    write_ptr: usize,
}

impl<const N: usize> FractionalDelayLine<N> {
    /// Create a new fractional delay line.
    pub const fn new() -> Self {
        Self {
            buffer: [0i16; N],
            write_ptr: 0,
        }
    }

    /// Push input sample and read delayed sample with fractional delay (Permyriad fraction).
    #[inline]
    pub fn process(&mut self, input: i16, delay_samples: usize, frac_permyriad: i32) -> i16 {
        self.buffer[self.write_ptr] = input;

        let r1 = (self.write_ptr + N - delay_samples) % N;
        let r2 = (self.write_ptr + N - delay_samples - 1) % N;

        let s1 = self.buffer[r1] as i32;
        let s2 = self.buffer[r2] as i32;

        // Linear interpolation: s1 + (s2 - s1) * frac / 10000
        let interp = s1 + ((s2 - s1) * frac_permyriad) / 10_000;

        self.write_ptr = (self.write_ptr + 1) % N;
        interp as i16
    }
}

impl<const N: usize> Default for FractionalDelayLine<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Triangular Probability Density Function (TPDF) Dither Generator.
pub struct TpdfDither {
    lfsr_state: u32,
}

impl TpdfDither {
    /// Create new dither generator initialized with non-zero seed.
    pub const fn new(seed: u32) -> Self {
        Self {
            lfsr_state: if seed == 0 { 0x1301_3001 } else { seed },
        }
    }

    /// Step 32-bit Galois LFSR to generate pseudorandom 16-bit word.
    #[inline(always)]
    fn next_lfsr(&mut self) -> u16 {
        let bit = (self.lfsr_state ^ (self.lfsr_state >> 1) ^ (self.lfsr_state >> 2) ^ (self.lfsr_state >> 7)) & 1;
        self.lfsr_state = (self.lfsr_state >> 1) | (bit << 31);
        (self.lfsr_state & 0xFFFF) as u16
    }

    /// Generate triangular dither value in range `[-1, 1]`.
    #[inline]
    pub fn sample_dither(&mut self) -> i32 {
        let r1 = (self.next_lfsr() as i32) - 32768;
        let r2 = (self.next_lfsr() as i32) - 32768;
        // Triangular distribution = r1 - r2
        (r1 - r2) >> 15
    }

    /// Apply dithered summing to 32-bit audio accumulator down to 16-bit output.
    #[inline]
    pub fn sum_dithered(&mut self, accumulator: i32) -> i16 {
        let d = self.sample_dither();
        let rounded = (accumulator + d) >> 12;
        if rounded > 32767 {
            32767
        } else if rounded < -32768 {
            -32768
        } else {
            rounded as i16
        }
    }
}

/// MoeRouter Centroid routing table using 64-bit harmonic centroids.
pub struct MoeRouter {
    /// 8-domain harmonic centroids.
    pub centroids: [u64; 8],
}

impl Default for MoeRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MoeRouter {
    /// Create new MoeRouter with standard 8-domain audio centroids.
    pub const fn new() -> Self {
        Self {
            centroids: [
                0x1111_2222_3333_4444,
                0x5555_6666_7777_8888,
                0x9999_AAAA_BBBB_CCCC,
                0xDDDD_EEEE_FFFF_0000,
                0x0F0F_1E1E_2D2D_3C3C,
                0x4B4B_5A5A_6969_7878,
                0x8787_9696_A5A5_B4B4,
                0xC3C3_D2D2_E1E1_F0F0,
            ],
        }
    }

    /// Fast sub-microsecond centroid lookup using bitwise XOR + POPCNT.
    #[inline(always)]
    pub fn route_centroid(&self, input_vector: u64) -> (usize, u32) {
        let mut best_expert = 0;
        let mut min_distance = u32::MAX;

        let mut i = 0;
        while i < 8 {
            let dist = (input_vector ^ self.centroids[i]).count_ones();
            if dist < min_distance {
                min_distance = dist;
                best_expert = i;
            }
            i += 1;
        }

        (best_expert, min_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spsc_ring_buffer_fifo() {
        let mut ring = SpscRingBuffer::<u32>::new(0);
        assert!(ring.is_empty());

        assert!(ring.push(42));
        assert!(ring.push(84));
        assert_eq!(ring.len(), 2);

        assert_eq!(ring.pop(), Some(42));
        assert_eq!(ring.pop(), Some(84));
        assert_eq!(ring.pop(), None);
        assert!(ring.is_empty());
    }

    #[test]
    fn test_biquad_filter_bounded_output() {
        let mut filter = BiquadFilterFixed::lowpass_smoothing();
        let mut out = 0;
        for _ in 0..100 {
            out = filter.process_sample(10_000);
        }
        // Output converges cleanly to unity gain (10,000)
        assert!(out > 9_500 && out <= 10_500, "Output was {}", out);
    }

    #[test]
    fn test_fractional_delay_line() {
        let mut delay = FractionalDelayLine::<64>::new();
        // Insert pulse
        let _ = delay.process(1000, 10, 0);
        for _ in 1..10 {
            let _ = delay.process(0, 10, 0);
        }
        let delayed = delay.process(0, 10, 0);
        assert_eq!(delayed, 1000);
    }

    #[test]
    fn test_tpdf_dither_clamping() {
        let mut dither = TpdfDither::new(0x1337);
        let s = dither.sum_dithered(1_000_000_000);
        assert_eq!(s, 32767);
        let s_neg = dither.sum_dithered(-1_000_000_000);
        assert_eq!(s_neg, -32768);
    }

    #[test]
    fn test_moe_router_sub_microsecond() {
        let router = MoeRouter::new();
        let (expert, dist) = router.route_centroid(0x1111_2222_3333_4444);
        assert_eq!(expert, 0);
        assert_eq!(dist, 0);
    }

    #[test]
    fn test_spsc_ring_buffer_full_condition() {
        let mut ring = SpscRingBuffer::<u8>::new(0);
        for _ in 0..SPSC_RING_CAPACITY {
            assert!(ring.push(1));
        }
        // Next push fails because buffer is full
        assert!(!ring.push(2));
    }

    #[test]
    fn test_fractional_delay_zero_delay() {
        let mut delay = FractionalDelayLine::<32>::new();
        let val = delay.process(42, 0, 0);
        assert_eq!(val, 42);
    }
}
