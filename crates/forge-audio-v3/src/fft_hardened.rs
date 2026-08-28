//! `fft_hardened` — Zero-heap, `no_std`-ready in-place FFT engine.
//!
//! Provides deterministic in-place radix-2 Cooley-Tukey Fast Fourier Transforms
//! for audio DSP and spectral analysis. All operations run directly in caller-provided
//! slices with zero dynamic heap allocations (`hotpath_heap_bytes = 0`).

#![allow(clippy::needless_range_loop)]

/// 2D Complex number representation for no_std DSP pipelines.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    #[inline(always)]
    pub const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    #[inline(always)]
    pub fn norm_sq(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    #[inline(always)]
    pub fn norm(self) -> f32 {
        self.norm_sq().sqrt()
    }

    #[inline(always)]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }

    #[inline(always)]
    pub fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }

    #[inline(always)]
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

/// Bit-reversal permutation table helper for power-of-two sizes.
#[inline(always)]
pub fn bit_reverse_indices(mut x: usize, log2_n: u32) -> usize {
    let mut rev = 0;
    for _ in 0..log2_n {
        rev = (rev << 1) | (x & 1);
        x >>= 1;
    }
    rev
}

/// Reorders complex slice in-place according to bit-reversal permutation.
#[inline]
pub fn bit_reverse_permute(buf: &mut [Complex32]) {
    let n = buf.len();
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two(), "FFT length must be a power of two");
    let log2_n = n.trailing_zeros();

    for i in 0..n {
        let j = bit_reverse_indices(i, log2_n);
        if i < j {
            buf.swap(i, j);
        }
    }
}

/// In-place Radix-2 Decimation-in-Time Complex-to-Complex FFT.
///
/// If `inverse` is true, performs the inverse transform and normalizes by `1/N`.
/// Operates in-place on `buf` with 0 heap bytes allocated.
pub fn c2c_in_place(buf: &mut [Complex32], inverse: bool) {
    let n = buf.len();
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two(), "FFT size must be power of two");

    bit_reverse_permute(buf);

    let pi = core::f32::consts::PI;
    let sign = if inverse { 1.0 } else { -1.0 };

    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let theta = sign * 2.0 * pi / len as f32;
        let w_step = Complex32::new(theta.cos(), theta.sin());

        let mut i = 0;
        while i < n {
            let mut w = Complex32::new(1.0, 0.0);
            for j in 0..half {
                let u = buf[i + j];
                let v = buf[i + j + half].mul(w);
                buf[i + j] = u.add(v);
                buf[i + j + half] = u.sub(v);
                w = w.mul(w_step);
            }
            i += len;
        }
        len <<= 1;
    }

    if inverse {
        let scale = 1.0 / n as f32;
        for c in buf.iter_mut() {
            c.re *= scale;
            c.im *= scale;
        }
    }
}

/// In-place Real-to-Complex FFT wrapper using standard complex buffer.
/// `real_in` is loaded into `buf` (with imaginary part zeroed) and transformed in-place.
pub fn rfft_in_place(real_in: &[f32], buf: &mut [Complex32]) {
    let n = real_in.len();
    debug_assert_eq!(buf.len(), n);
    for (c, &r) in buf.iter_mut().zip(real_in.iter()) {
        c.re = r;
        c.im = 0.0;
    }
    c2c_in_place(buf, false);
}

/// In-place Complex-to-Real Inverse FFT.
/// Inverse transforms `buf` and copies the real part into `real_out`.
pub fn irfft_in_place(buf: &mut [Complex32], real_out: &mut [f32]) {
    let n = buf.len();
    debug_assert_eq!(real_out.len(), n);
    c2c_in_place(buf, true);
    for (&c, r) in buf.iter().zip(real_out.iter_mut()) {
        *r = c.re;
    }
}

/// Stack-backed fixed-capacity RFFT processor for real-time audio threads.
/// Guaranteed 0 heap allocations across life cycle.
pub struct HardenedFft<const N: usize> {
    pub buffer: [Complex32; N],
}

impl<const N: usize> Default for HardenedFft<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> HardenedFft<N> {
    pub const fn new() -> Self {
        assert!(N.is_power_of_two(), "FFT size must be power of two");
        Self {
            buffer: [Complex32::zero(); N],
        }
    }

    /// Forward in-place real transform of `input` ([N] samples).
    #[inline]
    pub fn forward_real(&mut self, input: &[f32; N]) {
        for i in 0..N {
            self.buffer[i] = Complex32::new(input[i], 0.0);
        }
        c2c_in_place(&mut self.buffer, false);
    }

    /// Inverse in-place transform back into `output` ([N] samples).
    #[inline]
    pub fn inverse_real(&mut self, output: &mut [f32; N]) {
        c2c_in_place(&mut self.buffer, true);
        for i in 0..N {
            output[i] = self.buffer[i].re;
        }
    }

    /// Extract magnitude spectrum into caller-provided bins ([N/2+1] or [N/2]).
    #[inline]
    pub fn compute_magnitudes(&self, bins: &mut [f32]) {
        let count = bins.len().min(N / 2 + 1);
        for i in 0..count {
            bins[i] = self.buffer[i].norm();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_response_is_flat_spectrum() {
        let mut fft = HardenedFft::<128>::new();
        let mut impulse = [0.0f32; 128];
        impulse[0] = 1.0;

        fft.forward_real(&impulse);

        let mut bins = [0.0f32; 65];
        fft.compute_magnitudes(&mut bins);

        for (idx, &mag) in bins.iter().enumerate() {
            assert!(
                (mag - 1.0).abs() < 1e-5,
                "Impulse response magnitude at bin {idx} should be 1.0, got {mag}"
            );
        }
    }

    #[test]
    fn forward_inverse_roundtrip_mse() {
        let n = 256;
        let mut fft = HardenedFft::<256>::new();
        let mut original = [0.0f32; 256];
        for i in 0..n {
            original[i] = (2.0 * core::f32::consts::PI * 5.0 * i as f32 / n as f32).sin()
                + 0.5 * (2.0 * core::f32::consts::PI * 17.0 * i as f32 / n as f32).cos();
        }

        fft.forward_real(&original);

        let mut reconstructed = [0.0f32; 256];
        fft.inverse_real(&mut reconstructed);

        let mut mse = 0.0f32;
        for i in 0..n {
            let diff = original[i] - reconstructed[i];
            mse += diff * diff;
        }
        mse /= n as f32;

        assert!(
            mse < 1e-12,
            "Round-trip MSE must be < 1e-12, got {mse}"
        );
    }

    #[test]
    fn pure_sine_peak_detection() {
        let mut fft = HardenedFft::<128>::new();
        let mut signal = [0.0f32; 128];
        let target_bin = 16;
        for i in 0..128 {
            signal[i] = (2.0 * core::f32::consts::PI * target_bin as f32 * i as f32 / 128.0).sin();
        }

        fft.forward_real(&signal);

        let mut bins = [0.0f32; 65];
        fft.compute_magnitudes(&mut bins);

        let max_bin = bins
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        assert_eq!(max_bin, target_bin, "Peak detected at wrong bin");
    }
}
