//! DSP Primitives — Oscillators, Filters, Envelopes, SeedRng.
//!
//! All functions are pure, deterministic, and zero-alloc. Designed for
//! real-time audio synthesis on the audio thread.
//!
//! Requirements: 2.1–2.6, 3.1–3.6, 4.1–4.5, 9.1–9.4

use std::f32::consts::PI;

// ---------------------------------------------------------------------------
// SeedRng — Deterministic PRNG (SplitMix64)  [Task 1.2]
// ---------------------------------------------------------------------------

/// Deterministic PRNG based on SplitMix64.
///
/// Used for reproducible noise generation and pitch variation.
/// `next_f32()` returns values in [-1.0, 1.0].
#[derive(Debug, Clone)]
pub struct SeedRng {
    state: u64,
}

impl SeedRng {
    /// Create a new SeedRng from a 64-bit seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance state and return a raw u64 (SplitMix64 algorithm).
    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Return a deterministic f32 in [-1.0, 1.0].
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as f32;
        bits / (1u64 << 23) as f32 - 1.0
    }
}

// ---------------------------------------------------------------------------
// Oscillators  [Tasks 1.3, 1.4, 1.5]
// ---------------------------------------------------------------------------

/// Sine oscillator. Phase in [0.0, 1.0), output in [-1.0, 1.0].
#[inline]
pub fn osc_sine(phase: f32) -> f32 {
    (phase * 2.0 * PI).sin()
}

/// Square wave with configurable duty cycle.
/// Phase in [0.0, 1.0), duty in (0.0, 1.0), output in {-1.0, 1.0}.
#[inline]
pub fn osc_square(phase: f32, duty: f32) -> f32 {
    if phase < duty { 1.0 } else { -1.0 }
}

/// PolyBLEP correction term for band-limited waveforms.
#[inline]
fn poly_blep(t: f32) -> f32 {
    if !(0.0..1.0).contains(&t) {
        0.0
    } else if t < 0.5 {
        let t2 = t * 2.0;
        t2 + t2 - t2 * t2 - 1.0
    } else {
        let t2 = (t - 0.5) * 2.0;
        t2 * t2 - t2 - t2 + 1.0
    }
}

/// Band-limited sawtooth oscillator via PolyBLEP anti-aliasing.
/// Phase in [0.0, 1.0), output in [-1.0, 1.0].
#[inline]
pub fn osc_saw(phase: f32) -> f32 {
    let naive = 2.0 * phase - 1.0;
    naive - poly_blep(phase)
}

// ---------------------------------------------------------------------------
// Noise generators  [Tasks 1.6, 1.7]
// ---------------------------------------------------------------------------

/// White noise from SeedRng. Output in [-1.0, 1.0].
#[inline]
pub fn noise_white(rng: &mut SeedRng) -> f32 {
    rng.next_f32()
}

/// State for Voss-McCartney pink noise generator.
#[derive(Debug, Clone)]
pub struct PinkNoiseState {
    octaves: [f32; 8],
    counter: u32,
    running_sum: f32,
}

impl PinkNoiseState {
    pub fn new() -> Self {
        Self { octaves: [0.0; 8], counter: 0, running_sum: 0.0 }
    }
}

impl Default for PinkNoiseState {
    fn default() -> Self { Self::new() }
}

/// Voss-McCartney pink noise. Output approximately in [-1.0, 1.0].
pub fn noise_pink(state: &mut PinkNoiseState, rng: &mut SeedRng) -> f32 {
    state.counter = state.counter.wrapping_add(1);
    let changed_bits = state.counter ^ state.counter.wrapping_sub(1);
    for k in 0..8u32 {
        if changed_bits & (1 << k) != 0 {
            let old = state.octaves[k as usize];
            let new_val = rng.next_f32();
            state.running_sum += new_val - old;
            state.octaves[k as usize] = new_val;
        }
    }
    let white = rng.next_f32();
    (state.running_sum + white) / 9.0
}

// ---------------------------------------------------------------------------
// Filters  [Tasks 1.8, 1.9]
// ---------------------------------------------------------------------------

/// Biquad filter state (second-order IIR).
#[derive(Debug, Clone)]
pub struct BiquadState {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
}

impl BiquadState {
    pub fn new() -> Self {
        Self { x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    #[inline]
    fn advance(&mut self, x: f32, y: f32) {
        self.x2 = self.x1; self.x1 = x;
        self.y2 = self.y1; self.y1 = y;
    }
}

impl Default for BiquadState {
    fn default() -> Self { Self::new() }
}

#[inline]
fn clamp_cutoff(cutoff: f32) -> f32 {
    if cutoff.is_nan() || cutoff.is_infinite() || cutoff < 20.0 { 20.0 }
    else if cutoff > 20000.0 { 20000.0 }
    else { cutoff }
}

#[inline]
fn clamp_q(q: f32) -> f32 {
    if q.is_nan() || q.is_infinite() || q < 0.5 { 0.5 }
    else if q > 20.0 { 20.0 }
    else { q }
}

#[inline]
fn sanitize(y: f32) -> f32 {
    if y.is_nan() || y.is_infinite() { 0.0 } else { y }
}

/// Second-order lowpass filter.
pub fn filter_lowpass(state: &mut BiquadState, sample: f32, cutoff: f32, q: f32, sr: f32) -> f32 {
    let fc = clamp_cutoff(cutoff);
    let q = clamp_q(q);
    let w0 = 2.0 * PI * fc / sr;
    let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
    let alpha = sin_w0 / (2.0 * q);
    let b0 = (1.0 - cos_w0) / 2.0;
    let b1 = 1.0 - cos_w0;
    let b2 = (1.0 - cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    let inv_a0 = 1.0 / a0;
    let y = (b0 * sample + b1 * state.x1 + b2 * state.x2
        - a1 * state.y1 - a2 * state.y2) * inv_a0;
    let y = sanitize(y);
    state.advance(sample, y);
    y
}

/// Second-order highpass filter.
pub fn filter_highpass(state: &mut BiquadState, sample: f32, cutoff: f32, q: f32, sr: f32) -> f32 {
    let fc = clamp_cutoff(cutoff);
    let q = clamp_q(q);
    let w0 = 2.0 * PI * fc / sr;
    let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
    let alpha = sin_w0 / (2.0 * q);
    let b0 = (1.0 + cos_w0) / 2.0;
    let b1 = -(1.0 + cos_w0);
    let b2 = (1.0 + cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    let inv_a0 = 1.0 / a0;
    let y = (b0 * sample + b1 * state.x1 + b2 * state.x2
        - a1 * state.y1 - a2 * state.y2) * inv_a0;
    let y = sanitize(y);
    state.advance(sample, y);
    y
}

/// Second-order bandpass filter.
pub fn filter_bandpass(state: &mut BiquadState, sample: f32, center: f32, bw: f32, sr: f32) -> f32 {
    let fc = clamp_cutoff(center);
    let bw_safe = if bw.is_nan() || bw.is_infinite() || bw <= 0.0 { 1.0 } else { bw };
    let q = clamp_q(fc / bw_safe);
    let w0 = 2.0 * PI * fc / sr;
    let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
    let alpha = sin_w0 / (2.0 * q);
    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;
    let inv_a0 = 1.0 / a0;
    let y = (b0 * sample + b1 * state.x1 + b2 * state.x2
        - a1 * state.y1 - a2 * state.y2) * inv_a0;
    let y = sanitize(y);
    state.advance(sample, y);
    y
}

/// Resonant lowpass filter.
pub fn filter_resonant_lp(state: &mut BiquadState, sample: f32, cutoff: f32, q: f32, sr: f32) -> f32 {
    filter_lowpass(state, sample, cutoff, q, sr)
}

// ---------------------------------------------------------------------------
// Envelopes  [Tasks 1.10, 1.11]
// ---------------------------------------------------------------------------

/// AR (Attack-Release) envelope generator.
pub fn envelope_ar(t: f32, attack: f32, release: f32) -> f32 {
    let attack = attack.clamp(0.001, 1.0);
    let release = release.clamp(0.001, 10.0);
    if t < 0.0 { return 0.0; }
    if t < attack {
        (t / attack).clamp(0.0, 1.0)
    } else {
        let decay_t = t - attack;
        (-5.0 * decay_t / release).exp().clamp(0.0, 1.0)
    }
}

/// ADSR (Attack-Decay-Sustain-Release) envelope generator.
pub fn envelope_adsr(t: f32, gate_time: f32, a: f32, d: f32, s: f32, r: f32) -> f32 {
    let a = a.clamp(0.001, 1.0);
    let d = d.clamp(0.001, 5.0);
    let s = s.clamp(0.0, 1.0);
    let r = r.clamp(0.001, 10.0);
    let gate_time = gate_time.max(0.0);
    if t < 0.0 { return 0.0; }
    if t < gate_time {
        if t < a {
            (t / a).clamp(0.0, 1.0)
        } else if t < a + d {
            let decay_t = t - a;
            (1.0 - (1.0 - s) * (1.0 - (-5.0 * decay_t / d).exp())).clamp(0.0, 1.0)
        } else {
            s
        }
    } else {
        let level_at_gate = if gate_time < a {
            (gate_time / a).clamp(0.0, 1.0)
        } else if gate_time < a + d {
            let decay_t = gate_time - a;
            (1.0 - (1.0 - s) * (1.0 - (-5.0 * decay_t / d).exp())).clamp(0.0, 1.0)
        } else {
            s
        };
        let release_t = t - gate_time;
        (level_at_gate * (-5.0 * release_t / r).exp()).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_rng_deterministic() {
        let mut a = SeedRng::new(42);
        let mut b = SeedRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_f32().to_bits(), b.next_f32().to_bits());
        }
    }

    #[test]
    fn seed_rng_range() {
        let mut rng = SeedRng::new(12345);
        for _ in 0..1000 {
            let v = rng.next_f32();
            assert!((-1.0..=1.0).contains(&v), "SeedRng out of range: {}", v);
        }
    }

    #[test]
    fn osc_sine_at_zero_and_quarter() {
        assert!((osc_sine(0.0)).abs() < 1e-6);
        assert!((osc_sine(0.25) - 1.0).abs() < 1e-6);
        assert!((osc_sine(0.75) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn osc_square_duty() {
        assert_eq!(osc_square(0.0, 0.5), 1.0);
        assert_eq!(osc_square(0.49, 0.5), 1.0);
        assert_eq!(osc_square(0.5, 0.5), -1.0);
        assert_eq!(osc_square(0.99, 0.5), -1.0);
    }

    #[test]
    fn filter_zero_in_zero_out_lowpass() {
        let mut state = BiquadState::new();
        for _ in 0..100 {
            let y = filter_lowpass(&mut state, 0.0, 1000.0, 1.0, 44100.0);
            assert_eq!(y, 0.0);
        }
    }

    #[test]
    fn envelope_ar_basic() {
        assert!((envelope_ar(0.0, 0.01, 0.1)).abs() < 1e-6);
        let peak = envelope_ar(0.01, 0.01, 0.1);
        assert!((peak - 1.0).abs() < 0.01);
        let later = envelope_ar(0.1, 0.01, 0.1);
        assert!(later < peak);
    }

    #[test]
    fn envelope_adsr_sustain_level() {
        let env = envelope_adsr(0.5, 1.0, 0.01, 0.05, 0.6, 0.1);
        assert!((env - 0.6).abs() < 0.01, "Expected ~0.6 sustain, got {}", env);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p3_oscillator_output_range(freq in 20.0f32..20000.0, seed in any::<u64>()) {
            let sr = 44100.0f32;
            let mut phase = 0.0f32;
            let phase_inc = freq / sr;
            let mut rng = SeedRng::new(seed);
            for _ in 0..256 {
                let s = osc_sine(phase);
                prop_assert!((-1.0..=1.0).contains(&s));
                let sq = osc_square(phase, 0.5);
                prop_assert!((-1.0..=1.0).contains(&sq));
                let saw = osc_saw(phase);
                prop_assert!((-1.0..=1.0).contains(&saw));
                let wn = noise_white(&mut rng);
                prop_assert!((-1.0..=1.0).contains(&wn));
                phase += phase_inc;
                phase -= phase.floor();
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p4_filter_zero_in_zero_out(cutoff in 20.0f32..20000.0, q in 0.5f32..20.0) {
            let sr = 44100.0f32;
            let bw = cutoff / q;
            let mut lp = BiquadState::new();
            let mut hp = BiquadState::new();
            let mut bp = BiquadState::new();
            let mut rlp = BiquadState::new();
            for _ in 0..128 {
                prop_assert!(filter_lowpass(&mut lp, 0.0, cutoff, q, sr) == 0.0);
                prop_assert!(filter_highpass(&mut hp, 0.0, cutoff, q, sr) == 0.0);
                prop_assert!(filter_bandpass(&mut bp, 0.0, cutoff, bw, sr) == 0.0);
                prop_assert!(filter_resonant_lp(&mut rlp, 0.0, cutoff, q, sr) == 0.0);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p5_filter_no_nan(cutoff in any::<f32>(), q in any::<f32>(), sample in -1.0f32..1.0) {
            let sr = 44100.0f32;
            let mut lp = BiquadState::new();
            let mut hp = BiquadState::new();
            let mut bp = BiquadState::new();
            let mut rlp = BiquadState::new();
            for _ in 0..64 {
                let y_lp = filter_lowpass(&mut lp, sample, cutoff, q, sr);
                prop_assert!(!y_lp.is_nan() && !y_lp.is_infinite());
                let y_hp = filter_highpass(&mut hp, sample, cutoff, q, sr);
                prop_assert!(!y_hp.is_nan() && !y_hp.is_infinite());
                let y_bp = filter_bandpass(&mut bp, sample, cutoff, q, sr);
                prop_assert!(!y_bp.is_nan() && !y_bp.is_infinite());
                let y_rlp = filter_resonant_lp(&mut rlp, sample, cutoff, q, sr);
                prop_assert!(!y_rlp.is_nan() && !y_rlp.is_infinite());
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p6_envelope_range(
            t in 0.0f32..20.0, attack in 0.001f32..1.0,
            decay in 0.001f32..5.0, sustain in 0.0f32..=1.0,
            release in 0.001f32..10.0, gate_time in 0.0f32..10.0,
        ) {
            let ar = envelope_ar(t, attack, release);
            prop_assert!((0.0..=1.0).contains(&ar));
            let adsr = envelope_adsr(t, gate_time, attack, decay, sustain, release);
            prop_assert!((0.0..=1.0).contains(&adsr));
        }
    }
}
