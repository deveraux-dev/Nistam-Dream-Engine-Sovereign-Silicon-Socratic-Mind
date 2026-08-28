//! Production-grade audio metering: true peak, RMS, phase correlation, LUFS.
//!
//! LUFS implements ITU-R BS.1770-4 K-weighting (pre-filter + RLB) for
//! integrated loudness measurement per EBU R128.

/// K-weighting biquad filter state (2 cascaded stages per channel).
/// Stage 1: high-shelf pre-filter (+4 dB above ~1.5 kHz)
/// Stage 2: revised low-frequency (RLB) high-pass (~60 Hz)
#[derive(Clone, Debug)]
pub struct KWeightFilter {
    // Stage 1 (pre-filter) coefficients
    s1_b: [f64; 3],
    s1_a: [f64; 2],
    // Stage 2 (RLB) coefficients
    s2_b: [f64; 3],
    s2_a: [f64; 2],
    // State (biquad direct form II transposed)
    s1_z: [f64; 2],
    s2_z: [f64; 2],
}

impl KWeightFilter {
    /// Create K-weight filter for given sample rate (BS.1770-4 coefficients).
    pub fn new(sample_rate: f64) -> Self {
        // Stage 1: Pre-filter (high shelf, +4 dB above ~1.5 kHz)
        // Coefficients derived from BS.1770-4 Table 1 for 48 kHz,
        // recalculated via bilinear transform for other rates.
        let (s1_b, s1_a) = if (sample_rate - 48000.0).abs() < 1.0 {
            // Exact BS.1770-4 coefficients for 48 kHz
            ([1.53512485958697, -2.69169618940638, 1.19839281085285],
             [-1.69065929318241, 0.73248077421585])
        } else {
            // Bilinear transform approximation for other rates
            Self::prefilter_coeffs(sample_rate)
        };

        // Stage 2: RLB high-pass (~60 Hz, 2nd order)
        let (s2_b, s2_a) = if (sample_rate - 48000.0).abs() < 1.0 {
            ([1.0, -2.0, 1.0],
             [-1.99004745483398, 0.99007225036621])
        } else {
            Self::rlb_coeffs(sample_rate)
        };

        Self {
            s1_b, s1_a, s2_b, s2_a,
            s1_z: [0.0; 2],
            s2_z: [0.0; 2],
        }
    }

    fn prefilter_coeffs(fs: f64) -> ([f64; 3], [f64; 2]) {
        // Peaking shelf approximation via bilinear transform
        let f0 = 1681.974450955533;
        let g = 3.999843853973347_f64; // +4 dB
        let q = 0.7071752369554196;
        let a = 10.0_f64.powf(g / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
        let (sin_w, cos_w) = (w0.sin(), w0.cos());
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        let b = [
            (1.0 + alpha * a) / a0,
            (-2.0 * cos_w) / a0,
            (1.0 - alpha * a) / a0,
        ];
        let a_coeff = [
            (-2.0 * cos_w) / a0,
            (1.0 - alpha / a) / a0,
        ];
        (b, a_coeff)
    }

    fn rlb_coeffs(fs: f64) -> ([f64; 3], [f64; 2]) {
        // 2nd-order Butterworth HPF at ~38 Hz (RLB weighting)
        let f0 = 38.13547087602444;
        let q = 0.5003270373238773;
        let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
        let (sin_w, cos_w) = (w0.sin(), w0.cos());
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b = [
            ((1.0 + cos_w) / 2.0) / a0,
            (-(1.0 + cos_w)) / a0,
            ((1.0 + cos_w) / 2.0) / a0,
        ];
        let a_coeff = [
            (-2.0 * cos_w) / a0,
            (1.0 - alpha) / a0,
        ];
        (b, a_coeff)
    }

    /// Process one sample through both stages.
    pub fn process(&mut self, x: f32) -> f32 {
        let x = x as f64;
        // Stage 1
        let y1 = self.s1_b[0] * x + self.s1_z[0];
        self.s1_z[0] = self.s1_b[1] * x - self.s1_a[0] * y1 + self.s1_z[1];
        self.s1_z[1] = self.s1_b[2] * x - self.s1_a[1] * y1;
        // Stage 2
        let y2 = self.s2_b[0] * y1 + self.s2_z[0];
        self.s2_z[0] = self.s2_b[1] * y1 - self.s2_a[0] * y2 + self.s2_z[1];
        self.s2_z[1] = self.s2_b[2] * y1 - self.s2_a[1] * y2;
        y2 as f32
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.s1_z = [0.0; 2];
        self.s2_z = [0.0; 2];
    }
}

/// LUFS integrated loudness meter (EBU R128 / ITU-R BS.1770-4).
/// Operates on 400ms gating windows with -70 LUFS absolute gate
/// and -10 LU relative gate.
#[derive(Clone, Debug)]
pub struct LufsMeter {
    filter_l: KWeightFilter,
    filter_r: KWeightFilter,
    // sample_rate intentionally not stored — its only role is computing
    // `window_size` (= sample_rate * 0.4) at construction time. The
    // K-weighting filters capture the rate-dependent coefficients
    // internally. If future BS.1770 short-term / true-peak modes need
    // it, re-introduce + wire into a public mode-switch API rather
    // than suppress with `#[allow(dead_code)]`.
    /// Accumulator for current 400ms window (sum of squared K-weighted samples).
    window_sum_sq: f64,
    window_samples: usize,
    window_size: usize,
    /// All gating block loudness values (for integrated calculation).
    block_loudness: Vec<f64>,
    /// Current momentary loudness (last 400ms block).
    pub momentary_lufs: f64,
    /// Integrated loudness (gated mean across all blocks).
    pub integrated_lufs: f64,
}

impl LufsMeter {
    pub fn new(sample_rate: f64) -> Self {
        let window_size = (sample_rate * 0.4) as usize; // 400ms
        Self {
            filter_l: KWeightFilter::new(sample_rate),
            filter_r: KWeightFilter::new(sample_rate),
            window_sum_sq: 0.0,
            window_samples: 0,
            window_size,
            block_loudness: Vec::new(),
            momentary_lufs: -70.0,
            integrated_lufs: -70.0,
        }
    }

    /// Process interleaved stereo samples [L, R, L, R, ...].
    pub fn process_interleaved(&mut self, samples: &[f32]) {
        let frames = samples.len() / 2;
        for i in 0..frames {
            let l = self.filter_l.process(samples[i * 2]);
            let r = self.filter_r.process(samples[i * 2 + 1]);
            // Mean power of channels (BS.1770-4 §3: sum of channel powers)
            self.window_sum_sq += (l * l + r * r) as f64;
            self.window_samples += 1;

            if self.window_samples >= self.window_size {
                self.complete_window();
            }
        }
    }

    /// Process mono samples.
    pub fn process_mono(&mut self, samples: &[f32]) {
        for &s in samples {
            let k = self.filter_l.process(s);
            self.window_sum_sq += (k * k) as f64;
            self.window_samples += 1;

            if self.window_samples >= self.window_size {
                self.complete_window();
            }
        }
    }

    fn complete_window(&mut self) {
        let mean_sq = self.window_sum_sq / self.window_samples as f64;
        let lufs = if mean_sq > 0.0 {
            -0.691 + 10.0 * mean_sq.log10()
        } else {
            -70.0
        };
        self.momentary_lufs = lufs;
        self.block_loudness.push(lufs);
        self.integrated_lufs = self.compute_integrated();
        self.window_sum_sq = 0.0;
        self.window_samples = 0;
    }

    fn compute_integrated(&self) -> f64 {
        if self.block_loudness.is_empty() { return -70.0; }

        // Absolute gate: -70 LUFS
        let above_abs: Vec<f64> = self.block_loudness.iter()
            .copied().filter(|&l| l > -70.0).collect();
        if above_abs.is_empty() { return -70.0; }

        // Relative gate: mean of above-absolute, then -10 LU
        let mean_abs = above_abs.iter().sum::<f64>() / above_abs.len() as f64;
        let relative_gate = mean_abs - 10.0;

        let above_rel: Vec<f64> = above_abs.iter()
            .copied().filter(|&l| l > relative_gate).collect();
        if above_rel.is_empty() { return -70.0; }

        above_rel.iter().sum::<f64>() / above_rel.len() as f64
    }

    /// Get LUFS normalized to 0.0-1.0 range (maps -70..0 LUFS to 0..1).
    pub fn normalized(&self) -> f32 {
        ((self.integrated_lufs + 70.0) / 70.0).clamp(0.0, 1.0) as f32
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.filter_l.reset();
        self.filter_r.reset();
        self.window_sum_sq = 0.0;
        self.window_samples = 0;
        self.block_loudness.clear();
        self.momentary_lufs = -70.0;
        self.integrated_lufs = -70.0;
    }
}

/// Parametric EQ band (biquad).
#[derive(Clone, Debug)]
pub struct EqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
    // Biquad state
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl EqBand {
    pub fn new(freq_hz: f32, gain_db: f32, q: f32, sample_rate: f32) -> Self {
        let mut band = Self { freq_hz, gain_db, q, b: [0.0; 3], a: [0.0; 2], z: [0.0; 2] };
        band.recalc(sample_rate);
        band
    }

    /// Recalculate coefficients (peaking EQ).
    pub fn recalc(&mut self, sample_rate: f32) {
        let a = 10.0_f64.powf(self.gain_db as f64 / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * self.freq_hz as f64 / sample_rate as f64;
        let (sin_w, cos_w) = (w0.sin(), w0.cos());
        let alpha = sin_w / (2.0 * self.q as f64);
        let a0 = 1.0 + alpha / a;
        self.b = [
            (1.0 + alpha * a) / a0,
            (-2.0 * cos_w) / a0,
            (1.0 - alpha * a) / a0,
        ];
        self.a = [
            (-2.0 * cos_w) / a0,
            (1.0 - alpha / a) / a0,
        ];
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let x = x as f64;
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y as f32
    }

    pub fn reset(&mut self) { self.z = [0.0; 2]; }
}

/// N-band parametric EQ.
#[derive(Clone, Debug)]
pub struct ParametricEq {
    pub bands: Vec<EqBand>,
    pub sample_rate: f32,
}

impl ParametricEq {
    pub fn new(sample_rate: f32) -> Self {
        Self { bands: Vec::new(), sample_rate }
    }

    pub fn add_band(&mut self, freq_hz: f32, gain_db: f32, q: f32) {
        self.bands.push(EqBand::new(freq_hz, gain_db, q, self.sample_rate));
    }

    pub fn process(&mut self, mut sample: f32) -> f32 {
        for band in &mut self.bands {
            sample = band.process(sample);
        }
        sample
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}

/// Sidechain compressor with attack/release envelope.
#[derive(Clone, Debug)]
pub struct SidechainCompressor {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
    envelope: f32,
    sample_rate: f32,
}

impl SidechainCompressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            makeup_db: 0.0,
            envelope: 0.0,
            sample_rate,
        }
    }

    /// Compute gain reduction in dB from sidechain input level.
    fn gain_reduction_db(&self, sidechain_db: f32) -> f32 {
        if sidechain_db <= self.threshold_db {
            0.0
        } else {
            let over = sidechain_db - self.threshold_db;
            -(over - over / self.ratio)
        }
    }

    /// Process: apply compression to `signal` driven by `sidechain` level.
    /// Both buffers are the same length. Returns gain reduction in dB (for metering).
    pub fn process_block(&mut self, signal: &mut [f32], sidechain: &[f32]) -> f32 {
        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();
        let makeup_lin = 10.0_f32.powf(self.makeup_db / 20.0);
        let mut max_gr = 0.0f32;

        for (sig, &sc) in signal.iter_mut().zip(sidechain.iter()) {
            let sc_db = if sc.abs() > 1e-8 { 20.0 * sc.abs().log10() } else { -80.0 };
            let target_gr = self.gain_reduction_db(sc_db);
            let target_env = -target_gr; // positive envelope tracks compression amount

            let coeff = if target_env > self.envelope { attack_coeff } else { release_coeff };
            self.envelope = coeff * self.envelope + (1.0 - coeff) * target_env;

            let gr_db = -self.envelope;
            let gr_lin = 10.0_f32.powf(gr_db / 20.0);
            *sig *= gr_lin * makeup_lin;
            max_gr = max_gr.max(self.envelope);
        }
        -max_gr // return as negative dB
    }

    pub fn reset(&mut self) { self.envelope = 0.0; }
}

/// Distance attenuation model for game audio spatialization.
/// Implements inverse-distance with configurable rolloff + occlusion filter.
#[derive(Clone, Debug)]
pub struct DistanceAttenuation {
    /// Reference distance (no attenuation below this).
    pub ref_distance: f32,
    /// Maximum distance (fully attenuated beyond this).
    pub max_distance: f32,
    /// Rolloff factor (1.0 = realistic inverse, 2.0 = exaggerated).
    pub rolloff: f32,
}

impl DistanceAttenuation {
    pub fn new(ref_distance: f32, max_distance: f32, rolloff: f32) -> Self {
        Self { ref_distance, max_distance, rolloff }
    }

    /// Compute linear gain [0.0, 1.0] for a given distance.
    pub fn gain(&self, distance: f32) -> f32 {
        if distance <= self.ref_distance { return 1.0; }
        if distance >= self.max_distance { return 0.0; }
        let d = distance.clamp(self.ref_distance, self.max_distance);
        (self.ref_distance / (self.ref_distance + self.rolloff * (d - self.ref_distance)))
            .clamp(0.0, 1.0)
    }
}

/// Occlusion low-pass filter for muffled sound behind walls.
/// Simple 1-pole LPF with occlusion factor controlling cutoff.
#[derive(Clone, Debug)]
pub struct OcclusionFilter {
    /// Occlusion amount: 0.0 = clear, 1.0 = fully occluded.
    pub occlusion: f32,
    /// Minimum cutoff Hz when fully occluded.
    pub min_cutoff_hz: f32,
    /// Maximum cutoff Hz when clear (effectively bypass).
    pub max_cutoff_hz: f32,
    // 1-pole state
    prev: f32,
    sample_rate: f32,
}

impl OcclusionFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            occlusion: 0.0,
            min_cutoff_hz: 400.0,
            max_cutoff_hz: 20000.0,
            prev: 0.0,
            sample_rate,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        if self.occlusion < 0.001 { return sample; } // bypass when clear
        let cutoff = self.max_cutoff_hz - self.occlusion * (self.max_cutoff_hz - self.min_cutoff_hz);
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / self.sample_rate;
        let alpha = dt / (rc + dt);
        self.prev += alpha * (sample - self.prev);
        self.prev
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        if self.occlusion < 0.001 { return; }
        for s in samples.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) { self.prev = 0.0; }
}

/// Map an authored z-plane depth (MilliUnit, the ᐍ U+140D `stroke_z_mu` slot)
/// to `(distance_gain, occlusion)` — session 2026-07-03 §D.6.
///
/// The DET side hands over ONE integer (the z-sorted walk stays integer,
/// `VixelAtom.pos_z` is already MilliUnit); the float conversion happens here
/// at the audio boundary only. z is the listener-distance axis: the authored
/// world IS the ray result — deeper plane, softer and more muffled. Occlusion
/// is the gain complement until per-atom intermediate-z events exist.
pub fn z_plane_spatial(z_mu: i32, atten: &DistanceAttenuation) -> (f32, f32) {
    let distance_px = z_mu.max(0) as f32 / 1000.0;
    let gain = atten.gain(distance_px);
    (gain, (1.0 - gain).clamp(0.0, 1.0))
}

/// Catmull-Rom cubic interpolation — produces overshoot that reveals inter-sample peaks.
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// True peak via 4x Catmull-Rom oversampling.
/// Catches inter-sample peaks that standard peak meters miss.
pub fn true_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let n = samples.len();
    let mut max = 0.0f32;
    for i in 0..n {
        max = max.max(samples[i].abs());
        if i + 1 < n {
            let p0 = if i > 0 { samples[i - 1] } else { samples[i] };
            let p1 = samples[i];
            let p2 = samples[i + 1];
            let p3 = if i + 2 < n { samples[i + 2] } else { samples[i + 1] };
            // 3 interpolated points between each sample pair
            for k in 1..4 {
                let t = k as f32 / 4.0;
                let interp = catmull_rom(p0, p1, p2, p3, t);
                max = max.max(interp.abs());
            }
        }
    }
    max
}

/// RMS level over the entire buffer.
pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// RMS with NaN/Inf detection. Returns (rms, corrupted).
pub fn rms_level_checked(samples: &[f32]) -> (f32, bool) {
    if samples.is_empty() { return (0.0, false); }
    let mut sum_sq: f32 = 0.0;
    let mut corrupted = false;
    for &s in samples {
        if !s.is_finite() { corrupted = true; continue; }
        sum_sq += s * s;
    }
    ((sum_sq / samples.len() as f32).sqrt(), corrupted)
}

/// Boundary converter: linear RMS amplitude (0.0–1.0) → Permyriad (0–10000).
///
/// dB-mapped: −60 dB → 0, 0 dBFS → 10000 (`20·log10(rms)` clamped to [−60, 0] dB).
/// This is the ONE float→integer crossing on the metering path — RMS is inherently
/// `f32` (PCM), and the Permyriad output is what crosses to the visual lane
/// (`level_glyph`, `progress_bar`). Metering/viz only — never feed this into the
/// DET-CLOCK sim. Zero-alloc; safe to call per-frame on the render thread.
/// Seam: G-AUDIO-02 (level_glyph ⟷ rms_level ⟷ progress_bar).
pub fn rms_to_permyriad(rms: f32) -> u32 {
    if !rms.is_finite() || rms <= 0.0 {
        return 0;
    }
    db_to_permyriad(20.0 * rms.log10() as f64, RMS_FLOOR_DB) // negative for rms < 1.0
}

/// The metering floor `rms_to_permyriad` maps from: −60 dB reads as silence.
pub const RMS_FLOOR_DB: i64 = -60;

/// Boundary converter, dB domain: `floor_db`→0, 0 dBFS→10000, integer out.
///
/// The SAME crossing as [`rms_to_permyriad`], for the taps that already publish
/// dB rather than raw amplitude (`telemetry::master_rms_db`). Callers bring
/// their own floor because the floor is an authored choice — the star field
/// holds still below −48 dB while the meter still reads down to −60 — but the
/// ARITHMETIC is one fn, so two surfaces reading one bus cannot disagree about
/// what half-scale means. Seam: G-AUDIO-02.
pub fn db_to_permyriad(db: f64, floor_db: i64) -> u32 {
    let span = -floor_db as f64;
    if !db.is_finite() || span <= 0.0 {
        return 0;
    }
    let over = (db - floor_db as f64).max(0.0);
    ((over * 10_000.0 / span) as u32).min(10_000)
}

/// Stereo phase correlation: +1.0 = mono, 0.0 = uncorrelated, -1.0 = inverted.
pub fn phase_correlation(left: &[f32], right: &[f32]) -> f32 {
    let n = left.len().min(right.len());
    if n == 0 { return 0.0; }
    let mut sum_lr = 0.0f64;
    let mut sum_ll = 0.0f64;
    let mut sum_rr = 0.0f64;
    for i in 0..n {
        let l = left[i] as f64;
        let r = right[i] as f64;
        sum_lr += l * r;
        sum_ll += l * l;
        sum_rr += r * r;
    }
    let denom = (sum_ll * sum_rr).sqrt();
    if denom < 1e-12 { return 0.0; }
    (sum_lr / denom) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_peak_finds_inter_sample() {
        // Two adjacent near-full-scale samples with steep approach/departure
        // Catmull-Rom overshoot between 0.95 and 0.95 exceeds 0.95
        let samples = vec![0.0, 0.95, 0.95, 0.0];
        let peak = true_peak(&samples);
        assert!(peak > 0.95, "Expected inter-sample peak > 0.95, got {}", peak);
    }

    #[test]
    fn true_peak_of_silence_is_zero() {
        let samples = vec![0.0; 1024];
        assert_eq!(true_peak(&samples), 0.0);
    }

    #[test]
    fn true_peak_full_scale() {
        let samples = vec![0.0, 1.0, 0.0];
        assert!((true_peak(&samples) - 1.0).abs() < 0.01);
    }

    #[test]
    fn rms_of_sine() {
        let samples: Vec<f32> = (0..44100).map(|i| {
            (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 44100.0).sin()
        }).collect();
        let rms = rms_level(&samples);
        assert!((rms - 0.707).abs() < 0.01);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms_level(&[0.0; 1024]), 0.0);
    }

    #[test]
    fn phase_correlation_mono_is_one() {
        let signal: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.1).sin()).collect();
        let corr = phase_correlation(&signal, &signal);
        assert!((corr - 1.0).abs() < 0.01);
    }

    #[test]
    fn phase_correlation_inverted_is_negative_one() {
        let signal: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.1).sin()).collect();
        let inverted: Vec<f32> = signal.iter().map(|s| -s).collect();
        let corr = phase_correlation(&signal, &inverted);
        assert!((corr - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn phase_correlation_uncorrelated_near_zero() {
        let a: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.1).sin()).collect();
        let b: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.1731).sin()).collect();
        let corr = phase_correlation(&a, &b);
        assert!(corr.abs() < 0.2);
    }

    #[test]
    fn rms_level_detects_nan() {
        let mut buf = vec![0.5f32; 100];
        buf[50] = f32::NAN;
        let (_, corrupted) = rms_level_checked(&buf);
        assert!(corrupted);
    }

    #[test]
    fn rms_level_clean_buffer() {
        let buf: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        let (rms, corrupted) = rms_level_checked(&buf);
        assert!(!corrupted);
        assert!(rms > 0.0);
    }

    #[test]
    fn rms_level_detects_inf() {
        let mut buf = vec![0.5f32; 100];
        buf[10] = f32::INFINITY;
        let (_, corrupted) = rms_level_checked(&buf);
        assert!(corrupted);
    }

    // ── rms_to_permyriad (G-AUDIO-02 boundary converter) ─────────────────────

    #[test]
    fn rms_to_permyriad_bounds() {
        assert_eq!(rms_to_permyriad(0.0), 0); // silence
        assert_eq!(rms_to_permyriad(-1.0), 0); // guard: negative
        assert_eq!(rms_to_permyriad(1.0), 10_000); // 0 dBFS = full scale
        assert_eq!(rms_to_permyriad(2.0), 10_000); // clamps above 0 dB
    }

    // [BOARD: G-AUDIO-02]
    /// The dB-domain twin, at the two floors that ship: the metering floor and
    /// the star field's authored −48. Both ends are exact, and the floor is the
    /// only thing that differs between them.
    #[test]
    fn db_to_permyriad_spans_its_floor() {
        assert_eq!(db_to_permyriad(0.0, RMS_FLOOR_DB), 10_000); // 0 dBFS = full scale
        assert_eq!(db_to_permyriad(-60.0, RMS_FLOOR_DB), 0); // metering floor
        assert_eq!(db_to_permyriad(-30.0, RMS_FLOOR_DB), 5_000); // half the span
        assert_eq!(db_to_permyriad(-48.0, -48), 0); // star-field floor
        assert_eq!(db_to_permyriad(-24.0, -48), 5_000);
        assert_eq!(db_to_permyriad(-99.0, -48), 0, "below floor clamps, never wraps");
        assert_eq!(db_to_permyriad(12.0, -48), 10_000, "above 0 dBFS clamps");
        assert_eq!(db_to_permyriad(f64::NAN, -48), 0);
    }

    #[test]
    fn rms_to_permyriad_rejects_nonfinite() {
        assert_eq!(rms_to_permyriad(f32::NAN), 0);
        assert_eq!(rms_to_permyriad(f32::INFINITY), 0);
    }

    #[test]
    fn rms_to_permyriad_monotonic_and_midpoint() {
        // −30 dB (≈0.03162) maps to the mid of the −60..0 dB window.
        let mid = rms_to_permyriad(0.031_62);
        assert!((4900..=5100).contains(&mid), "midpoint drifted: {mid}");
        // strictly increasing across the audible range
        assert!(rms_to_permyriad(0.01) < rms_to_permyriad(0.1));
        assert!(rms_to_permyriad(0.1) < rms_to_permyriad(0.5));
    }

    // ── LUFS tests ──────────────────────────────────────────────────────────

    #[test]
    fn lufs_silence_is_minus_70() {
        let mut meter = LufsMeter::new(48000.0);
        let silence = vec![0.0f32; 48000]; // 1 second mono
        meter.process_mono(&silence);
        assert!(meter.integrated_lufs <= -69.0);
    }

    #[test]
    fn lufs_sine_is_reasonable() {
        let mut meter = LufsMeter::new(48000.0);
        // 1 kHz sine at -20 dBFS for 1 second
        let amp = 10.0_f32.powf(-20.0 / 20.0);
        let samples: Vec<f32> = (0..48000).map(|i| {
            amp * (i as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 48000.0).sin()
        }).collect();
        meter.process_mono(&samples);
        // LUFS should be approximately -20 (K-weighting at 1kHz is ~0 dB)
        assert!(meter.integrated_lufs > -25.0 && meter.integrated_lufs < -15.0,
            "Expected LUFS near -20, got {}", meter.integrated_lufs);
    }

    #[test]
    fn lufs_k_weight_boosts_high_freq() {
        let mut meter_low = LufsMeter::new(48000.0);
        let mut meter_high = LufsMeter::new(48000.0);
        let amp = 0.1f32;
        let low: Vec<f32> = (0..48000).map(|i| {
            amp * (i as f32 * 2.0 * std::f32::consts::PI * 100.0 / 48000.0).sin()
        }).collect();
        let high: Vec<f32> = (0..48000).map(|i| {
            amp * (i as f32 * 2.0 * std::f32::consts::PI * 5000.0 / 48000.0).sin()
        }).collect();
        meter_low.process_mono(&low);
        meter_high.process_mono(&high);
        // K-weighting boosts HF, so 5kHz should read louder than 100Hz at same amplitude
        assert!(meter_high.integrated_lufs > meter_low.integrated_lufs,
            "5kHz ({}) should read louder than 100Hz ({})",
            meter_high.integrated_lufs, meter_low.integrated_lufs);
    }

    #[test]
    fn lufs_normalized_range() {
        let mut meter = LufsMeter::new(48000.0);
        assert!(meter.normalized() >= 0.0 && meter.normalized() <= 1.0);
        let loud: Vec<f32> = (0..48000).map(|i| {
            0.9 * (i as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 48000.0).sin()
        }).collect();
        meter.process_mono(&loud);
        assert!(meter.normalized() > 0.5);
    }

    // ── Parametric EQ tests ─────────────────────────────────────────────────

    #[test]
    fn eq_flat_is_passthrough() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(1000.0, 0.0, 1.0); // 0 dB gain = passthrough
        let input: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.1).sin()).collect();
        let mut output = input.clone();
        eq.process_block(&mut output);
        // After settling, output should match input closely
        let tail_diff: f32 = input[512..].iter().zip(&output[512..])
            .map(|(a, b)| (a - b).abs()).sum::<f32>() / 512.0;
        assert!(tail_diff < 0.01, "Flat EQ should be passthrough, avg diff {}", tail_diff);
    }

    #[test]
    fn eq_boost_increases_energy() {
        let mut eq = ParametricEq::new(48000.0);
        eq.add_band(1000.0, 12.0, 1.0); // +12 dB at 1kHz
        let input: Vec<f32> = (0..4800).map(|i| {
            0.1 * (i as f32 * 2.0 * std::f32::consts::PI * 1000.0 / 48000.0).sin()
        }).collect();
        let mut output = input.clone();
        eq.process_block(&mut output);
        let rms_in = rms_level(&input[2400..]);
        let rms_out = rms_level(&output[2400..]);
        assert!(rms_out > rms_in * 2.0, "12dB boost should at least double RMS");
    }

    // ── Sidechain compressor tests ──────────────────────────────────────────

    #[test]
    fn compressor_no_reduction_below_threshold() {
        let mut comp = SidechainCompressor::new(48000.0);
        comp.threshold_db = -10.0;
        let mut signal = vec![0.1f32; 1024]; // ~-20 dBFS
        let sidechain = vec![0.01f32; 1024]; // very quiet sidechain
        let gr = comp.process_block(&mut signal, &sidechain);
        assert!(gr > -1.0, "Should have minimal GR, got {} dB", gr);
    }

    #[test]
    fn compressor_reduces_above_threshold() {
        let mut comp = SidechainCompressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;
        let mut signal = vec![0.5f32; 4800]; // -6 dBFS
        let sidechain = vec![0.5f32; 4800]; // loud sidechain
        let gr = comp.process_block(&mut signal, &sidechain);
        assert!(gr < -3.0, "Should have significant GR, got {} dB", gr);
        // Signal should be quieter after compression
        let rms_out = rms_level(&signal[2400..]);
        assert!(rms_out < 0.5, "Compressed signal should be quieter, got {}", rms_out);
    }

    // ── Distance attenuation tests ──────────────────────────────────────────

    #[test]
    fn distance_at_ref_is_unity() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        assert_eq!(atten.gain(0.5), 1.0);
        assert_eq!(atten.gain(1.0), 1.0);
    }

    #[test]
    fn distance_at_max_is_zero() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        assert_eq!(atten.gain(100.0), 0.0);
        assert_eq!(atten.gain(200.0), 0.0);
    }

    #[test]
    fn distance_monotonically_decreases() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        let mut prev = 1.0f32;
        for d in (2..100).map(|i| i as f32) {
            let g = atten.gain(d);
            assert!(g <= prev, "Gain should decrease: {} at d={}", g, d);
            prev = g;
        }
    }

    // ── Occlusion filter tests ──────────────────────────────────────────────

    #[test]
    fn occlusion_zero_is_passthrough() {
        let mut filt = OcclusionFilter::new(48000.0);
        filt.occlusion = 0.0;
        let input: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.5).sin()).collect();
        let mut output = input.clone();
        filt.process_block(&mut output);
        assert_eq!(input, output);
    }

    #[test]
    fn occlusion_full_attenuates_high_freq() {
        let mut filt = OcclusionFilter::new(48000.0);
        filt.occlusion = 1.0;
        // 10kHz sine
        let input: Vec<f32> = (0..4800).map(|i| {
            0.5 * (i as f32 * 2.0 * std::f32::consts::PI * 10000.0 / 48000.0).sin()
        }).collect();
        let mut output = input.clone();
        filt.process_block(&mut output);
        let rms_in = rms_level(&input[2400..]);
        let rms_out = rms_level(&output[2400..]);
        assert!(rms_out < rms_in * 0.3, "Full occlusion should heavily attenuate 10kHz");
    }

    // ── Z-plane spatial mapping tests (§D.6) ────────────────────────────────

    #[test]
    fn z_plane_flat_sprite_is_clear_and_full_gain() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        let (gain, occlusion) = z_plane_spatial(0, &atten);
        assert_eq!(gain, 1.0, "z=0 (flat 2D plane) is at the listener");
        assert_eq!(occlusion, 0.0);
    }

    #[test]
    fn z_plane_authored_cree_depth_attenuates_and_occludes() {
        // ᐍ=4200 — the green Lab arming value (forge_vision_lab.rs).
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        let (gain, occlusion) = z_plane_spatial(4200, &atten);
        assert!(gain < 0.5, "4.2px deep must lose over half its energy: {gain}");
        assert!(gain > 0.0, "still inside max_distance — never silent");
        assert!((gain + occlusion - 1.0).abs() < 1e-6, "occlusion is the complement");
    }

    #[test]
    fn z_plane_deeper_is_softer_and_more_occluded() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        let (g_near, o_near) = z_plane_spatial(2_000, &atten);
        let (g_far, o_far) = z_plane_spatial(20_000, &atten);
        assert!(g_far < g_near);
        assert!(o_far > o_near);
    }

    #[test]
    fn z_plane_negative_depth_clamps_to_listener() {
        let atten = DistanceAttenuation::new(1.0, 100.0, 1.0);
        assert_eq!(z_plane_spatial(-500, &atten), (1.0, 0.0));
    }
}
