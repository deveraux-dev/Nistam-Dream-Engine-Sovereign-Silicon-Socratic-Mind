// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! DSP effects: Bitcrusher, StereoWidener, Phaser, DopplerShift, HRTF,
//! ConvolutionReverb, and the realtime algorithmic ZoneReverb (Schroeder/Freeverb).
//!
//! All delay-line heap allocs happen at construction only — tagged @forge:allow_alloc.
//! The hot `process` / `process_block` paths are zero-heap pointer arithmetic.

// ── Bitcrusher ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Bitcrusher {
    pub bits: u8,
    pub downsample: u32,
    hold_counter: u32,
    hold_value: f32, // @forge:allow_float
}

impl Bitcrusher {
    pub fn new(bits: u8, downsample: u32) -> Self {
        Self { bits: bits.clamp(1, 16), downsample: downsample.max(1), hold_counter: 0, hold_value: 0.0 }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        self.hold_counter += 1;
        if self.hold_counter >= self.downsample {
            self.hold_counter = 0;
            let levels = (1u32 << self.bits) as f32;
            self.hold_value = (sample * levels).round() / levels;
        }
        self.hold_value
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() { *s = self.process(*s); }
    }

    pub fn reset(&mut self) { self.hold_counter = 0; self.hold_value = 0.0; }
}

// ── Stereo Widener (M/S) ──────────────────────────────────────────────────────

/// Width 0.0 = mono, 1.0 = normal stereo, 2.0 = extra wide.
#[derive(Clone, Debug)]
pub struct StereoWidener {
    pub width: f32, // @forge:allow_float
}

impl StereoWidener {
    pub fn new(width: f32) -> Self { Self { width: width.clamp(0.0, 2.0) } }

    pub fn process(&self, left: f32, right: f32) -> (f32, f32) {
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;
        let new_side = side * self.width;
        (mid + new_side, mid - new_side)
    }

    pub fn process_interleaved(&self, samples: &mut [f32]) {
        let frames = samples.len() / 2;
        for i in 0..frames {
            let (l, r) = self.process(samples[i * 2], samples[i * 2 + 1]);
            samples[i * 2] = l;
            samples[i * 2 + 1] = r;
        }
    }
}

// ── Phaser (4-stage allpass) ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Phaser {
    pub rate_hz: f32,    // @forge:allow_float
    pub depth: f32,      // @forge:allow_float
    pub feedback: f32,   // @forge:allow_float
    phase: f32,          // @forge:allow_float
    allpass_state: [f32; 4],  // @forge:allow_float
    sample_rate: f32,    // @forge:allow_float
}

impl Phaser {
    pub fn new(sample_rate: f32, rate_hz: f32, depth: f32, feedback: f32) -> Self {
        Self {
            rate_hz,
            depth: depth.clamp(0.0, 1.0),
            feedback: feedback.clamp(0.0, 0.95),
            phase: 0.0,
            allpass_state: [0.0; 4],
            sample_rate,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        let lfo = (self.phase * core::f32::consts::TAU).sin() * self.depth;

        let base_coeff = 0.5 + 0.4 * lfo;
        let coeffs = [base_coeff, base_coeff * 0.9, base_coeff * 0.8, base_coeff * 0.7];

        let mut x = sample + self.allpass_state[3] * self.feedback;
        for i in 0..4 {
            let c = coeffs[i].clamp(-0.99, 0.99);
            let y = c * x + self.allpass_state[i];
            self.allpass_state[i] = x - c * y;
            x = y;
        }
        (sample + x) * 0.5
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() { *s = self.process(*s); }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.allpass_state = [0.0; 4];
    }
}

// ── Doppler shift ─────────────────────────────────────────────────────────────

/// Doppler pitch ratio minimum (0.5x, half speed) — prevents extreme downward shift.
pub const DOPPLER_RATIO_MIN_PMY: u32 = 5_000;  // 0.5 × 10_000

/// Doppler pitch ratio maximum (2.0x, double speed) — prevents extreme upward shift.
pub const DOPPLER_RATIO_MAX_PMY: u32 = 20_000; // 2.0 × 10_000

#[derive(Clone, Debug)]
pub struct DopplerShift {
    pub speed_of_sound: f32, // @forge:allow_float
    buffer: Vec<f32>,        // @forge:allow_float  @forge:allow_alloc — init-time delay ring
    write_pos: usize,
    read_pos: f64,           // @forge:allow_float
}

impl DopplerShift {
    pub fn new(sample_rate: f32, max_delay_ms: f32) -> Self {
        let buf_size = (sample_rate * max_delay_ms * 0.001) as usize + 2;
        Self { speed_of_sound: 343.0, buffer: vec![0.0; buf_size], write_pos: 0, read_pos: 0.0 }
    }

    pub fn pitch_ratio(&self, radial_velocity: f32) -> f32 {
        let denom = self.speed_of_sound - radial_velocity;
        if denom.abs() < 1.0 { return 1.0; }
        let ratio = self.speed_of_sound / denom;
        let min = DOPPLER_RATIO_MIN_PMY as f32 / 10_000.0;
        let max = DOPPLER_RATIO_MAX_PMY as f32 / 10_000.0;
        ratio.clamp(min, max)
    }

    pub fn process(&mut self, sample: f32, radial_velocity: f32) -> f32 {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        let rate = self.pitch_ratio(radial_velocity) as f64;
        self.read_pos += rate;
        while self.read_pos >= self.buffer.len() as f64 { self.read_pos -= self.buffer.len() as f64; }

        let idx = self.read_pos as usize % self.buffer.len();
        let frac = self.read_pos.fract() as f32;
        let next = (idx + 1) % self.buffer.len();
        self.buffer[idx] * (1.0 - frac) + self.buffer[next] * frac
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.read_pos = 0.0;
    }
}

#[cfg(test)]
mod doppler_tests {
    use super::*;

    #[test]
    fn doppler_ratio_clamps_at_bounds() {
        let ds = DopplerShift::new(48_000.0, 100.0);

        // Test minimum clamp: high negative velocity → ratio > 2.0 would clamp to max
        let high_approach = ds.pitch_ratio(-600.0);
        let max = DOPPLER_RATIO_MAX_PMY as f32 / 10_000.0;
        assert!(high_approach <= max, "approaching velocity should clamp at max {max}");

        // Test maximum clamp: high positive velocity → ratio < 0.5 would clamp to min
        let high_recede = ds.pitch_ratio(600.0);
        let min = DOPPLER_RATIO_MIN_PMY as f32 / 10_000.0;
        assert!(high_recede >= min, "receding velocity should clamp at min {min}");

        // Identity: zero velocity → ratio ≈ 1.0
        let stationary = ds.pitch_ratio(0.0);
        assert!((stationary - 1.0).abs() < 0.01, "zero velocity should yield ratio near 1.0");
    }
}

// ── Reverb zone presets ───────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ReverbZonePreset {
    pub name: &'static str,
    pub predelay_ms: f32,  // @forge:allow_float
    pub decay_s: f32,      // @forge:allow_float
    pub damping: f32,      // @forge:allow_float
    pub wet: f32,          // @forge:allow_float
    pub room_size: f32,    // @forge:allow_float
}

pub const REVERB_PRESETS: &[ReverbZonePreset] = &[
    ReverbZonePreset { name: "small_room",     predelay_ms: 5.0,  decay_s: 0.4, damping: 0.5, wet: 0.20, room_size: 0.30 },
    ReverbZonePreset { name: "medium_hall",    predelay_ms: 15.0, decay_s: 1.2, damping: 0.4, wet: 0.35, room_size: 0.60 },
    ReverbZonePreset { name: "large_hall",     predelay_ms: 30.0, decay_s: 2.5, damping: 0.3, wet: 0.45, room_size: 0.85 },
    ReverbZonePreset { name: "cathedral",      predelay_ms: 50.0, decay_s: 4.0, damping: 0.2, wet: 0.50, room_size: 1.00 },
    ReverbZonePreset { name: "cave",           predelay_ms: 20.0, decay_s: 3.0, damping: 0.7, wet: 0.55, room_size: 0.70 },
    ReverbZonePreset { name: "outdoor",        predelay_ms: 2.0,  decay_s: 0.2, damping: 0.8, wet: 0.10, room_size: 0.10 },
    ReverbZonePreset { name: "underwater",     predelay_ms: 10.0, decay_s: 1.5, damping: 0.9, wet: 0.60, room_size: 0.50 },
    ReverbZonePreset { name: "metal_corridor", predelay_ms: 8.0,  decay_s: 0.8, damping: 0.1, wet: 0.40, room_size: 0.40 },
];

pub fn reverb_preset(name: &str) -> Option<&'static ReverbZonePreset> {
    REVERB_PRESETS.iter().find(|p| p.name == name)
}

// ── HRTF (binaural ITD+ILD) ───────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct HrtfProcessor {
    delay_buf_l: Vec<f32>,  // @forge:allow_float  @forge:allow_alloc — init-time delay
    delay_buf_r: Vec<f32>,  // @forge:allow_float  @forge:allow_alloc — init-time delay
    write_pos: usize,
    sample_rate: f32,       // @forge:allow_float
}

impl HrtfProcessor {
    pub fn new(sample_rate: f32) -> Self {
        let max_itd = (sample_rate * 0.0007) as usize + 1;
        Self {
            delay_buf_l: vec![0.0; max_itd + 1],
            delay_buf_r: vec![0.0; max_itd + 1],
            write_pos: 0,
            sample_rate,
        }
    }

    /// Process mono → stereo (L, R) given azimuth in radians (0=front, π/2=right).
    pub fn process(&mut self, sample: f32, azimuth: f32) -> (f32, f32) {
        let buf_len = self.delay_buf_l.len();
        let max_itd = self.sample_rate * 0.0007;
        let itd = azimuth.sin() * max_itd;
        let ild_db = azimuth.sin() * 6.0;
        let gain_r = 10.0f32.powf(ild_db / 20.0);
        let gain_l = 10.0f32.powf(-ild_db / 20.0);

        self.delay_buf_l[self.write_pos] = sample;
        self.delay_buf_r[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % buf_len;

        let delay_l = if itd > 0.0 { itd } else { 0.0 };
        let delay_r = if itd < 0.0 { -itd } else { 0.0 };

        let read_l = (self.write_pos as f32 - delay_l - 1.0 + buf_len as f32) % buf_len as f32;
        let read_r = (self.write_pos as f32 - delay_r - 1.0 + buf_len as f32) % buf_len as f32;

        let l = lerp_buf(&self.delay_buf_l, read_l) * gain_l;
        let r = lerp_buf(&self.delay_buf_r, read_r) * gain_r;
        (l, r)
    }

    pub fn reset(&mut self) {
        self.delay_buf_l.fill(0.0);
        self.delay_buf_r.fill(0.0);
        self.write_pos = 0;
    }
}

fn lerp_buf(buf: &[f32], pos: f32) -> f32 {
    let idx = pos as usize % buf.len();
    let frac = pos.fract();
    let next = (idx + 1) % buf.len();
    buf[idx] * (1.0 - frac) + buf[next] * frac
}

// ── Convolution Reverb ────────────────────────────────────────────────────────

/// Partitioned time-domain convolution reverb (short IRs < 2048 samples).
pub struct ConvolutionReverb {
    ir: Vec<f32>,         // @forge:allow_float  @forge:allow_alloc — init-time IR copy
    input_buf: Vec<f32>,  // @forge:allow_float  @forge:allow_alloc — init-time ring
    write_pos: usize,
    pub wet: f32,         // @forge:allow_float
}

impl ConvolutionReverb {
    pub fn new(impulse_response: &[f32], max_len: usize, wet: f32) -> Self {
        let len = impulse_response.len().min(max_len);
        let mut ir = vec![0.0; len];
        ir.copy_from_slice(&impulse_response[..len]);
        Self { input_buf: vec![0.0; len], write_pos: 0, ir, wet: wet.clamp(0.0, 1.0) }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        self.input_buf[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.ir.len();
        let mut acc = 0.0f32;
        let len = self.ir.len();
        let mut read = self.write_pos;
        for i in 0..len {
            if read == 0 { read = len; }
            read -= 1;
            acc += self.input_buf[read] * self.ir[i];
        }
        sample * (1.0 - self.wet) + acc * self.wet
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() { *s = self.process(*s); }
    }

    pub fn ir_len(&self) -> usize { self.ir.len() }

    pub fn reset(&mut self) { self.input_buf.fill(0.0); self.write_pos = 0; }
}

// ── ZoneReverb (Schroeder/Freeverb algorithmic hall) ──────────────────────────

const COMB_TUNINGS: [usize; 4] = [1116, 1188, 1277, 1356];
const ALLPASS_TUNINGS: [usize; 2] = [556, 441];
const ALLPASS_FEEDBACK: f32 = 0.5;

#[derive(Clone, Debug)]
struct Comb {
    buf: Vec<f32>,       // @forge:allow_float  @forge:allow_alloc — init-time delay
    pos: usize,
    feedback: f32,       // @forge:allow_float
    damp1: f32,          // @forge:allow_float
    damp2: f32,          // @forge:allow_float
    filterstore: f32,    // @forge:allow_float
}

impl Comb {
    fn new(size: usize) -> Self {
        Self { buf: vec![0.0; size.max(1)], pos: 0, feedback: 0.5, damp1: 0.5, damp2: 0.5, filterstore: 0.0 }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let out = self.buf[self.pos];
        self.filterstore = out * self.damp2 + self.filterstore * self.damp1;
        self.buf[self.pos] = input + self.filterstore * self.feedback;
        self.pos += 1;
        if self.pos >= self.buf.len() { self.pos = 0; }
        out
    }

    fn reset(&mut self) { self.buf.fill(0.0); self.filterstore = 0.0; self.pos = 0; }
}

#[derive(Clone, Debug)]
struct Allpass {
    buf: Vec<f32>, // @forge:allow_float  @forge:allow_alloc — init-time delay
    pos: usize,
    feedback: f32, // @forge:allow_float
}

impl Allpass {
    fn new(size: usize) -> Self {
        Self { buf: vec![0.0; size.max(1)], pos: 0, feedback: ALLPASS_FEEDBACK }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buf[self.pos];
        let out = -input + bufout;
        self.buf[self.pos] = input + bufout * self.feedback;
        self.pos += 1;
        if self.pos >= self.buf.len() { self.pos = 0; }
        out
    }

    fn reset(&mut self) { self.buf.fill(0.0); self.pos = 0; }
}

/// Realtime algorithmic reverb driven by a [`ReverbZonePreset`]. Mono in / mono out.
/// Delay-line sizes fixed at construction; `process`/`process_block` are zero-heap.
#[derive(Clone, Debug)]
pub struct ZoneReverb {
    predelay: Vec<f32>, // @forge:allow_float  @forge:allow_alloc — init-time ring
    predelay_pos: usize,
    predelay_len: usize,
    combs: [Comb; 4],
    allpasses: [Allpass; 2],
    wet: f32,         // @forge:allow_float
    sample_rate: f32, // @forge:allow_float
}

impl ZoneReverb {
    pub fn new(sample_rate: f32, preset: &ReverbZonePreset) -> Self {
        let scale = sample_rate / 44_100.0;
        let room = 0.5 + preset.room_size;
        let size = |t: usize| ((t as f32 * scale * room) as usize).max(1);
        let combs = [
            Comb::new(size(COMB_TUNINGS[0])),
            Comb::new(size(COMB_TUNINGS[1])),
            Comb::new(size(COMB_TUNINGS[2])),
            Comb::new(size(COMB_TUNINGS[3])),
        ];
        let allpasses = [
            Allpass::new(size(ALLPASS_TUNINGS[0])),
            Allpass::new(size(ALLPASS_TUNINGS[1])),
        ];
        let max_predelay = (sample_rate * 0.2) as usize + 1;
        let mut r = Self {
            predelay: vec![0.0; max_predelay],
            predelay_pos: 0,
            predelay_len: 0,
            combs,
            allpasses,
            wet: 0.0,
            sample_rate,
        };
        r.set_preset(preset);
        r
    }

    /// Retune tail in-place (delay-line sizes stay fixed — realtime-safe).
    pub fn set_preset(&mut self, preset: &ReverbZonePreset) {
        self.wet = preset.wet.clamp(0.0, 1.0);
        let pd = (preset.predelay_ms * 0.001 * self.sample_rate) as usize;
        self.predelay_len = pd.min(self.predelay.len().saturating_sub(1));
        let damp = preset.damping.clamp(0.0, 1.0);
        for c in self.combs.iter_mut() {
            let delay_s = c.buf.len() as f32 / self.sample_rate;
            let fb = 10f32.powf(-3.0 * delay_s / preset.decay_s.max(0.05));
            c.feedback = fb.clamp(0.0, 0.98);
            c.damp1 = damp;
            c.damp2 = 1.0 - damp;
        }
    }

    pub fn set_wet(&mut self, wet: f32) { self.wet = wet.clamp(0.0, 1.0); }
    pub fn wet(&self) -> f32 { self.wet }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let delayed = if self.predelay_len == 0 {
            input
        } else {
            let len = self.predelay.len();
            let read = (self.predelay_pos + len - self.predelay_len) % len;
            let d = self.predelay[read];
            self.predelay[self.predelay_pos] = input;
            self.predelay_pos += 1;
            if self.predelay_pos >= len { self.predelay_pos = 0; }
            d
        };

        let mut acc = 0.0;
        for c in self.combs.iter_mut() { acc += c.process(delayed); }
        acc /= self.combs.len() as f32;
        for a in self.allpasses.iter_mut() { acc = a.process(acc); }

        input * (1.0 - self.wet) + acc * self.wet
    }

    pub fn process_block(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() { *s = self.process(*s); }
    }

    pub fn reset(&mut self) {
        self.predelay.fill(0.0);
        self.predelay_pos = 0;
        for c in self.combs.iter_mut() { c.reset(); }
        for a in self.allpasses.iter_mut() { a.reset(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    #[test]
    fn bitcrusher_16bit_is_near_passthrough() {
        let mut bc = Bitcrusher::new(16, 1);
        assert!((bc.process(0.12345f32) - 0.12345).abs() < 0.001);
    }

    #[test]
    fn bitcrusher_downsample_holds() {
        let mut bc = Bitcrusher::new(16, 4);
        let v1 = bc.process(0.5);
        let v2 = bc.process(0.9);
        let v3 = bc.process(0.1);
        assert_eq!(v2, v1);
        assert_eq!(v3, v1);
    }

    #[test]
    fn widener_mono_collapses() {
        let w = StereoWidener::new(0.0);
        let (l, r) = w.process(1.0, -1.0);
        assert!((l - r).abs() < 0.001);
    }

    #[test]
    fn widener_unity_is_passthrough() {
        let w = StereoWidener::new(1.0);
        let (l, r) = w.process(0.8, 0.2);
        assert!((l - 0.8).abs() < 0.001);
        assert!((r - 0.2).abs() < 0.001);
    }

    #[test]
    fn doppler_stationary_is_unity() {
        let d = DopplerShift::new(48000.0, 50.0);
        assert!((d.pitch_ratio(0.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn doppler_approaching_raises_pitch() {
        let d = DopplerShift::new(48000.0, 50.0);
        assert!(d.pitch_ratio(50.0) > 1.0);
    }

    #[test]
    fn reverb_presets_all_valid() {
        for p in REVERB_PRESETS {
            assert!(p.decay_s > 0.0);
            assert!(p.wet >= 0.0 && p.wet <= 1.0);
        }
    }

    #[test]
    fn reverb_preset_lookup() {
        assert!(reverb_preset("cave").is_some());
        assert!(reverb_preset("nonexistent").is_none());
    }

    #[test]
    fn impulse_produces_a_decaying_tail() {
        let mut rev = ZoneReverb::new(48_000.0, reverb_preset("large_hall").unwrap());
        let mut buf = vec![0.0f32; 48_000];
        buf[0] = 1.0;
        rev.process_block(&mut buf);
        let tail = &buf[24_000..];
        assert!(rms(tail) > 1e-5, "hall must ring into the tail");
    }

    #[test]
    fn cathedral_tail_outlasts_small_room() {
        let mut cath = ZoneReverb::new(48_000.0, reverb_preset("cathedral").unwrap());
        let mut room = ZoneReverb::new(48_000.0, reverb_preset("small_room").unwrap());
        let (mut a, mut b) = (vec![0.0f32; 48_000], vec![0.0f32; 48_000]);
        a[0] = 1.0; b[0] = 1.0;
        cath.process_block(&mut a);
        room.process_block(&mut b);
        let late = 36_000..;
        assert!(rms(&a[late.clone()]) > rms(&b[late]));
    }

    #[test]
    fn fully_dry_is_passthrough() {
        let mut rev = ZoneReverb::new(48_000.0, reverb_preset("medium_hall").unwrap());
        rev.set_wet(0.0);
        let dry = [0.3f32, -0.7, 0.1, 0.9, -0.2];
        let mut buf = dry;
        rev.process_block(&mut buf);
        for (i, (&got, &want)) in buf.iter().zip(dry.iter()).enumerate() {
            assert!((got - want).abs() < 1e-6, "sample {i}: {got} != {want}");
        }
    }

    #[test]
    fn reset_clears_the_tail() {
        let mut rev = ZoneReverb::new(48_000.0, reverb_preset("cathedral").unwrap());
        let mut excite = vec![0.0f32; 1000];
        excite[0] = 1.0;
        rev.process_block(&mut excite);
        rev.reset();
        let mut after = vec![0.0f32; 4800];
        rev.process_block(&mut after);
        assert!(rms(&after) < 1e-7, "reset must clear the tail");
    }

    #[test]
    fn phaser_produces_output() {
        let mut p = Phaser::new(48000.0, 1.0, 0.7, 0.5);
        let sum: f32 = (0..480).map(|i| p.process((i as f32 * 0.1).sin()).abs()).sum();
        assert!(sum > 0.0);
    }

    #[test]
    fn hrtf_center_is_symmetric() {
        let mut h = HrtfProcessor::new(48000.0);
        let (l, r) = h.process(1.0, 0.0);
        assert!((l - r).abs() < 0.01);
    }

    #[test]
    fn convolution_impulse_response_passthrough() {
        let mut conv = ConvolutionReverb::new(&[1.0], 2048, 1.0);
        let out = conv.process(0.75);
        assert!((out - 0.75).abs() < 0.01);
    }
}
