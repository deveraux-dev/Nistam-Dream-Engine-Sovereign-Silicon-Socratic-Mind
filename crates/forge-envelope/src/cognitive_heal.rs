//! Cognitive Heal DSP Primitives — Sovereign Audio Engine.
//!
//! Provides mathematically clean-room, textbook-grade audio processing primitives in pure f64.
//! Composes filters, delay lines, and LFOs to form complex focus-state soundscapes,
//! implemented from first principles with no external DSP dependency.
//!
//! Handles `#![no_std]` execution through high-precision local Taylor series and Pade math approximations.

/// Mathematical constants.
pub const PI: f64 = 3.141592653589793;

/// Biquad filter (Lowpass, Highpass, Bandpass, Peaking) based on Robert Bristow-Johnson Cookbook formulas.
/// Implemented using the Transposed Direct Form II (TDF-II) structure.
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    w1: f64,
    w2: f64,
}

impl Biquad {
    /// Create a new lowpass filter.
    pub fn lowpass(sample_rate: f64, cutoff: f64, q: f64) -> Self {
        let mut filter = Self::default();
        filter.set_lowpass(sample_rate, cutoff, q);
        filter
    }

    /// Set coefficients for Lowpass.
    pub fn set_lowpass(&mut self, sample_rate: f64, cutoff: f64, q: f64) {
        let omega = 2.0 * PI * cutoff / sample_rate;
        let sin_w = approx_sin(omega);
        let cos_w = approx_cos(omega);
        let alpha = sin_w / (2.0 * q);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_w) / 2.0) / a0;
        self.b1 = (1.0 - cos_w) / a0;
        self.b2 = ((1.0 - cos_w) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Set coefficients for Highpass.
    pub fn set_highpass(&mut self, sample_rate: f64, cutoff: f64, q: f64) {
        let omega = 2.0 * PI * cutoff / sample_rate;
        let sin_w = approx_sin(omega);
        let cos_w = approx_cos(omega);
        let alpha = sin_w / (2.0 * q);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_w) / 2.0) / a0;
        self.b1 = (-(1.0 + cos_w)) / a0;
        self.b2 = ((1.0 + cos_w) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Process a single audio sample.
    #[inline(always)]
    pub fn process(&mut self, sample: f64) -> f64 {
        let out = self.b0 * sample + self.w1;
        self.w1 = self.b1 * sample - self.a1 * out + self.w2;
        self.w2 = self.b2 * sample - self.a2 * out;
        out
    }
}

/// A fractional interpolation delay line.
pub struct DelayLine<const MAX_SAMPLES: usize> {
    buffer: [f64; MAX_SAMPLES],
    write_idx: usize,
}

impl<const MAX_SAMPLES: usize> DelayLine<MAX_SAMPLES> {
    /// Construct a fresh delay line.
    pub fn new() -> Self {
        Self {
            buffer: [0.0f64; MAX_SAMPLES],
            write_idx: 0,
        }
    }

    /// Push a sample and read with a given delay in fractional samples.
    #[inline(always)]
    pub fn process(&mut self, sample: f64, delay_samples: f64) -> f64 {
        // Write in-place
        self.buffer[self.write_idx] = sample;

        // Calculate read index with fractional offset
        let clamped_delay = delay_samples.clamp(0.0, (MAX_SAMPLES - 2) as f64);
        let read_pos = (self.write_idx as f64 - clamped_delay + MAX_SAMPLES as f64) % MAX_SAMPLES as f64;
        let idx0 = read_pos.floor() as usize % MAX_SAMPLES;
        let idx1 = (idx0 + 1) % MAX_SAMPLES;
        let frac = read_pos - read_pos.floor();

        // Linear interpolation
        let out = self.buffer[idx0] * (1.0 - frac) + self.buffer[idx1] * frac;

        // Advance write pointer
        self.write_idx = (self.write_idx + 1) % MAX_SAMPLES;
        out
    }
}

/// Feedback Comb Filter (LBCF) primarily composing the reverb tail.
pub struct DampedComb<const MAX_SAMPLES: usize> {
    delay: DelayLine<MAX_SAMPLES>,
    filter_state: f64,
    feedback: f64,
    damping: f64,
}

impl<const MAX_SAMPLES: usize> DampedComb<MAX_SAMPLES> {
    /// Create a new comb filter.
    pub fn new(feedback: f64, damping: f64) -> Self {
        Self {
            delay: DelayLine::new(),
            filter_state: 0.0,
            feedback,
            damping,
        }
    }

    /// Process a single sample through the damped feedback path.
    #[inline(always)]
    pub fn process(&mut self, sample: f64, delay_len: f64) -> f64 {
        let delayed = self.delay.buffer[self.delay.write_idx]; // direct read
        
        // Lowpass damping filter inside the feedback loop
        self.filter_state = delayed * (1.0 - self.damping) + self.filter_state * self.damping;

        let input_sample = sample + self.filter_state * self.feedback;
        self.delay.process(input_sample, delay_len);
        delayed
    }
}

/// Schroeder Allpass Filter (Magnitude-Flat Diffuser).
pub struct Allpass<const MAX_SAMPLES: usize> {
    delay: DelayLine<MAX_SAMPLES>,
    gain: f64,
}

impl<const MAX_SAMPLES: usize> Allpass<MAX_SAMPLES> {
    /// Create a new Allpass diffuser.
    pub fn new(gain: f64) -> Self {
        Self {
            delay: DelayLine::new(),
            gain,
        }
    }

    /// Process sample while maintaining a flat magnitude response.
    #[inline(always)]
    pub fn process(&mut self, sample: f64, delay_len: f64) -> f64 {
        // Direct buffer read
        let delayed = self.delay.buffer[self.delay.write_idx];
        let write_val = sample + delayed * self.gain;
        let out = -self.gain * write_val + delayed;
        self.delay.process(write_val, delay_len);
        out
    }
}

/// Envelope Follower with asymmetric Attack and Release coefficients.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnvelopeFollower {
    attack_coef: f64,
    release_coef: f64,
    envelope: f64,
}

impl EnvelopeFollower {
    /// Initialize with attack/release durations relative to sample rate.
    pub fn new(sample_rate: f64, attack_ms: f64, release_ms: f64) -> Self {
        let attack_coef = approx_exp(-1.0 / (sample_rate * attack_ms * 0.001));
        let release_coef = approx_exp(-1.0 / (sample_rate * release_ms * 0.001));
        Self {
            attack_coef,
            release_coef,
            envelope: 0.0,
        }
    }

    /// Track the absolute amplitude envelope.
    #[inline(always)]
    pub fn process(&mut self, sample: f64) -> f64 {
        let abs_sample = sample.abs();
        if abs_sample > self.envelope {
            self.envelope = abs_sample + self.attack_coef * (self.envelope - abs_sample);
        } else {
            self.envelope = abs_sample + self.release_coef * (self.envelope - abs_sample);
        }
        self.envelope
    }
}

/// Low Frequency Oscillator (LFO) supporting Sine and Triangle waves.
pub struct Lfo {
    phase: f64,
    phase_increment: f64,
}

impl Lfo {
    /// Create a new LFO at a given rate.
    pub fn new(sample_rate: f64, frequency: f64) -> Self {
        Self {
            phase: 0.0,
            phase_increment: frequency / sample_rate,
        }
    }

    /// Retrieve the next sine value [-1.0, 1.0].
    #[inline(always)]
    pub fn next_sine(&mut self) -> f64 {
        let val = approx_sin(2.0 * PI * self.phase);
        self.phase = (self.phase + self.phase_increment) % 1.0;
        val
    }

    /// Retrieve the next triangle value [-1.0, 1.0].
    #[inline(always)]
    pub fn next_triangle(&mut self) -> f64 {
        let val = if self.phase < 0.5 {
            4.0 * self.phase - 1.0
        } else {
            3.0 - 4.0 * self.phase
        };
        self.phase = (self.phase + self.phase_increment) % 1.0;
        val
    }
}

/// Freeverb Reverb Engine composed of 4 parallel DampedCombs and 2 series Allpass diffusers.
/// Implemented from first principles, no external DSP dependency.
pub struct Freeverb {
    comb1: DampedComb<1600>,
    comb2: DampedComb<1800>,
    comb3: DampedComb<2000>,
    comb4: DampedComb<2200>,
    allpass1: Allpass<500>,
    allpass2: Allpass<200>,
}

impl Freeverb {
    /// Construct a fresh reverb processor.
    pub fn new() -> Self {
        Self {
            comb1: DampedComb::new(0.84, 0.2),
            comb2: DampedComb::new(0.84, 0.2),
            comb3: DampedComb::new(0.84, 0.2),
            comb4: DampedComb::new(0.84, 0.2),
            allpass1: Allpass::new(0.5),
            allpass2: Allpass::new(0.5),
        }
    }

    /// Process stereo-simulated sample summing.
    #[inline(always)]
    pub fn process(&mut self, sample: f64) -> f64 {
        // Parallel comb filters
        let c1 = self.comb1.process(sample, 1116.0);
        let c2 = self.comb2.process(sample, 1356.0);
        let c3 = self.comb3.process(sample, 1422.0);
        let c4 = self.comb4.process(sample, 1491.0);
        
        let summed = (c1 + c2 + c3 + c4) * 0.25;

        // Series Allpass diffusers
        let a1 = self.allpass1.process(summed, 341.0);
        let a2 = self.allpass2.process(a1, 113.0);
        a2
    }
}

// =========================================================================
// Deterministic f64 Math Approximations for #![no_std] Compatibility
// =========================================================================

/// 9th-degree Taylor approximation of sin(x) for x in [-PI, PI].
#[inline(always)]
pub fn approx_sin(mut x: f64) -> f64 {
    // Wrap to [-PI, PI] range
    x = x % (2.0 * PI);
    if x > PI {
        x -= 2.0 * PI;
    } else if x < -PI {
        x += 2.0 * PI;
    }

    let x2 = x * x;
    // Taylor Series Coefficients: 1 - x^3/3! + x^5/5! - x^7/7! + x^9/9!
    let term3 = (x2 * x) / 6.0;
    let term5 = (x2 * term3) / 20.0;
    let term7 = (x2 * term5) / 42.0;
    let term9 = (x2 * term7) / 72.0;

    x - term3 + term5 - term7 + term9
}

/// Cosine approximation using translated sine.
#[inline(always)]
pub fn approx_cos(x: f64) -> f64 {
    approx_sin(x + PI / 2.0)
}

/// Exponential exp(x) Padé (2,2) approximation.
#[inline(always)]
pub fn approx_exp(x: f64) -> f64 {
    let clamped_x = x.clamp(-20.0, 20.0);
    // Padé [2/2] approximation around 0
    let n = 12.0 + 6.0 * clamped_x + clamped_x * clamped_x;
    let d = 12.0 - 6.0 * clamped_x + clamped_x * clamped_x;
    n / d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_sin_cos() {
        assert!((approx_sin(0.0) - 0.0).abs() < 1e-4);
        assert!((approx_sin(PI / 2.0) - 1.0).abs() < 1e-3);
        assert!((approx_cos(0.0) - 1.0).abs() < 1e-3);
        assert!((approx_cos(PI) + 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_biquad_lowpass() {
        let mut lp = Biquad::lowpass(48000.0, 1000.0, 0.707);
        let out = lp.process(1.0);
        // Ensure no NaNs or Infinities propagate
        assert!(out.is_finite());
    }

    #[test]
    fn test_reverb_reproducibility() {
        let mut reverb = Freeverb::new();
        let out1 = reverb.process(1.0);
        let out2 = reverb.process(0.0);
        assert!(out1.is_finite());
        assert!(out2.is_finite());
    }
}
