// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! mic_fx — Broadcast Booth realtime mic-strip primitives.
//!
//! The streaming per-sample filters the booth runs on the mic bridge window
//! (logic/record lane — NOT the RT callback, so the zero-alloc callback rule does
//! not bind; every buffer here is sized at `::new` and reused regardless).
//!
//! Chain (the "late-night DJ" voice strip, DSPP-shaped):
//!   NoiseGate (downward expander) -> HighPass (rumble/plosive clear)
//!     -> DjFilter (bipolar resonant sweep — the DJ knob) -> Compressor (steady it)
//!
//! Spectral background suppression (the "NVIDIA RTX" step) is upstream in
//! `crate::noise_suppress::NoiseSuppressor` — it runs per-STFT-frame before this
//! per-sample strip. `heal_voice`/`BrickwallLimiter` finish the RECORD path.

use std::f32::consts::PI;

/// One RBJ-cookbook biquad section, Transposed Direct Form II (one state pair,
/// numerically kind at audio rates). Coefficients are pre-normalised by `a0`.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// A unity pass-through (b0=1, everything else 0).
    pub fn bypass() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }

    /// 2nd-order low-pass at `f0` Hz, quality `q` (0.707 = Butterworth).
    pub fn lowpass(sr: f32, f0: f32, q: f32) -> Self {
        let (w0, cw, alpha) = Self::prewarp(sr, f0, q);
        let b0 = (1.0 - cw) * 0.5;
        let b1 = 1.0 - cw;
        let b2 = (1.0 - cw) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cw;
        let a2 = 1.0 - alpha;
        let _ = w0;
        Self::normalised(b0, b1, b2, a0, a1, a2)
    }

    /// 2nd-order high-pass at `f0` Hz, quality `q`.
    pub fn highpass(sr: f32, f0: f32, q: f32) -> Self {
        let (_w0, cw, alpha) = Self::prewarp(sr, f0, q);
        let b0 = (1.0 + cw) * 0.5;
        let b1 = -(1.0 + cw);
        let b2 = (1.0 + cw) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cw;
        let a2 = 1.0 - alpha;
        Self::normalised(b0, b1, b2, a0, a1, a2)
    }

    /// Peaking EQ bell: `gain_db` boost/cut at `f0`, bandwidth from `q`.
    pub fn peaking(sr: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        let (_w0, cw, alpha) = Self::prewarp(sr, f0, q);
        let a = 10.0f32.powf(gain_db / 40.0);
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cw;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cw;
        let a2 = 1.0 - alpha / a;
        Self::normalised(b0, b1, b2, a0, a1, a2)
    }

    /// Low-shelf: `gain_db` applied below `f0` — the "warmth" tilt.
    pub fn low_shelf(sr: f32, f0: f32, q: f32, gain_db: f32) -> Self {
        let (_w0, cw, alpha) = Self::prewarp(sr, f0, q);
        let a = 10.0f32.powf(gain_db / 40.0);
        let sa = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) - (a - 1.0) * cw + sa);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cw);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cw - sa);
        let a0 = (a + 1.0) + (a - 1.0) * cw + sa;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cw);
        let a2 = (a + 1.0) + (a - 1.0) * cw - sa;
        Self::normalised(b0, b1, b2, a0, a1, a2)
    }

    #[inline]
    fn prewarp(sr: f32, f0: f32, q: f32) -> (f32, f32, f32) {
        let f = f0.clamp(5.0, sr * 0.49);
        let qq = q.max(0.05);
        let w0 = 2.0 * PI * f / sr;
        let cw = w0.cos();
        let alpha = w0.sin() / (2.0 * qq);
        (w0, cw, alpha)
    }

    #[inline]
    fn normalised(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv = 1.0 / a0;
        Self {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Retune this section in place, preserving the filter state (click-free sweep).
    pub fn set(&mut self, coeffs: Biquad) {
        self.b0 = coeffs.b0;
        self.b1 = coeffs.b1;
        self.b2 = coeffs.b2;
        self.a1 = coeffs.a1;
        self.a2 = coeffs.a2;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn process_block(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// The DJ filter: one bipolar knob (`-1.0 ..= 1.0`).
///
/// * `0.0` — bypass (filter open).
/// * `> 0.0` — **high-pass** sweeping 20 Hz → ~3 kHz as the knob turns right
///   (kills the low end — the classic "filter out the bass" build).
/// * `< 0.0` — **low-pass** sweeping ~18 kHz → 200 Hz as it turns left
///   (muffles to a late-night murmur).
///
/// Resonance lifts Q at the corner for the singing sweep DJs ride.
#[derive(Clone, Copy, Debug)]
pub struct DjFilter {
    sr: f32,
    biquad: Biquad,
    knob: f32,
    resonance: f32,
    active: bool,
}

impl DjFilter {
    pub fn new(sr: f32) -> Self {
        Self { sr, biquad: Biquad::bypass(), knob: 0.0, resonance: 0.707, active: false }
    }

    /// Set the bipolar knob and recompute coefficients (state preserved).
    pub fn set_knob(&mut self, knob: f32) {
        self.knob = knob.clamp(-1.0, 1.0);
        self.retune();
    }

    /// Resonance: 0.0 (flat, Q≈0.707) → 1.0 (singing, Q≈6).
    pub fn set_resonance(&mut self, r: f32) {
        self.resonance = 0.707 + r.clamp(0.0, 1.0) * 5.3;
        self.retune();
    }

    fn retune(&mut self) {
        let k = self.knob;
        if k.abs() < 0.02 {
            self.active = false;
            self.biquad.set(Biquad::bypass());
            return;
        }
        self.active = true;
        let nyq = self.sr * 0.49;
        if k > 0.0 {
            // high-pass: 20 Hz -> min(3 kHz, nyq), exponential in the knob
            let top = 3000.0f32.min(nyq);
            let f = 20.0 * (top / 20.0).powf(k);
            self.biquad.set(Biquad::highpass(self.sr, f, self.resonance));
        } else {
            // low-pass: 18 kHz (clamped to nyq) -> 200 Hz
            let t = -k;
            let top = 18_000.0f32.min(nyq);
            let f = top * (200.0 / top).powf(t);
            self.biquad.set(Biquad::lowpass(self.sr, f, self.resonance));
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        if self.active {
            self.biquad.process(x)
        } else {
            x
        }
    }

    pub fn process_block(&mut self, buf: &mut [f32]) {
        if self.active {
            self.biquad.process_block(buf);
        }
    }

    pub fn knob(&self) -> f32 {
        self.knob
    }

    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

/// Downward expander / noise gate — the per-sample companion to the spectral
/// suppressor. Below `threshold`, the signal is pulled down toward the gate
/// floor by `ratio`; above it, unity. Envelope-followed so it breathes, not clicks.
#[derive(Clone, Copy, Debug)]
pub struct NoiseGate {
    threshold: f32, // linear
    ratio: f32,     // >= 1.0 (downward expansion slope)
    floor: f32,     // linear minimum gain
    atk: f32,       // envelope attack coeff
    rel: f32,       // envelope release coeff
    env: f32,
    gain: f32,
}

impl NoiseGate {
    pub fn new(sr: f32, threshold_db: f32, ratio: f32, atk_ms: f32, rel_ms: f32) -> Self {
        Self {
            threshold: db_to_lin(threshold_db),
            ratio: ratio.max(1.0),
            floor: db_to_lin(-60.0),
            atk: time_coeff(atk_ms, sr),
            rel: time_coeff(rel_ms, sr),
            env: 0.0,
            gain: 1.0,
        }
    }

    pub fn set_threshold_db(&mut self, db: f32) {
        self.threshold = db_to_lin(db);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let coeff = if a > self.env { self.atk } else { self.rel };
        self.env = coeff * self.env + (1.0 - coeff) * a;
        // Target gain: unity above threshold, expand downward below it.
        let target = if self.env >= self.threshold {
            1.0
        } else if self.env < 1e-7 {
            self.floor
        } else {
            // downward expansion in the log domain
            let under_db = lin_to_db(self.env / self.threshold); // negative
            let g_db = under_db * (self.ratio - 1.0);
            db_to_lin(g_db).max(self.floor)
        };
        // Smooth the gain with the same envelope times (avoid zipper noise).
        let gc = if target < self.gain { self.atk } else { self.rel };
        self.gain = gc * self.gain + (1.0 - gc) * target;
        x * self.gain
    }

    pub fn process_block(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
        self.gain = 1.0;
    }
}

/// Streaming feed-forward peak compressor — the persistent-state twin of
/// `healing::compress` (which is a one-shot over a whole buffer). Steadies the
/// live delivery on the monitor/broadcast path.
#[derive(Clone, Copy, Debug)]
pub struct Compressor {
    thr_db: f32,
    ratio: f32,
    makeup: f32,
    atk: f32,
    rel: f32,
    env: f32,
}

impl Compressor {
    pub fn new(sr: f32, thr_db: f32, ratio: f32, atk_ms: f32, rel_ms: f32, makeup_db: f32) -> Self {
        Self {
            thr_db,
            ratio: ratio.max(1.0),
            makeup: db_to_lin(makeup_db),
            atk: time_coeff(atk_ms, sr),
            rel: time_coeff(rel_ms, sr),
            env: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let coeff = if a > self.env { self.atk } else { self.rel };
        self.env = coeff * self.env + (1.0 - coeff) * a;
        let env_db = lin_to_db(self.env);
        let gain_db = if env_db > self.thr_db {
            let over = env_db - self.thr_db;
            over / self.ratio - over // negative -> gain reduction
        } else {
            0.0
        };
        x * db_to_lin(gain_db) * self.makeup
    }

    pub fn process_block(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
    }
}

/// Tunable parameters for the whole mic strip.
#[derive(Clone, Copy, Debug)]
pub struct MicStripParams {
    pub gate_threshold_db: f32,
    pub gate_ratio: f32,
    pub hpf_hz: f32,
    pub dj_knob: f32,
    pub dj_resonance: f32,
    pub warmth_db: f32, // low-shelf tilt for the late-night body
    pub comp_threshold_db: f32,
    pub comp_ratio: f32,
    pub makeup_db: f32,
}

impl Default for MicStripParams {
    fn default() -> Self {
        Self {
            gate_threshold_db: -50.0,
            gate_ratio: 2.5,
            hpf_hz: 90.0,
            dj_knob: 0.0,
            dj_resonance: 0.2,
            warmth_db: 1.5,
            comp_threshold_db: -18.0,
            comp_ratio: 3.0,
            makeup_db: 4.0,
        }
    }
}

/// The composed live mic strip: gate -> HPF -> DJ filter -> warmth -> compressor.
/// Mono, streaming, state preserved across blocks.
pub struct MicStrip {
    sr: f32,
    gate: NoiseGate,
    hpf: Biquad,
    dj: DjFilter,
    warmth: Biquad,
    comp: Compressor,
    params: MicStripParams,
}

impl MicStrip {
    pub fn new(sr: f32, params: MicStripParams) -> Self {
        let mut dj = DjFilter::new(sr);
        dj.set_resonance(params.dj_resonance);
        dj.set_knob(params.dj_knob);
        Self {
            sr,
            gate: NoiseGate::new(sr, params.gate_threshold_db, params.gate_ratio, 2.0, 80.0),
            hpf: Biquad::highpass(sr, params.hpf_hz, 0.707),
            dj,
            warmth: Biquad::low_shelf(sr, 180.0, 0.707, params.warmth_db),
            comp: Compressor::new(
                sr,
                params.comp_threshold_db,
                params.comp_ratio,
                8.0,
                120.0,
                params.makeup_db,
            ),
            params,
        }
    }

    pub fn params(&self) -> &MicStripParams {
        &self.params
    }

    /// Retune the whole strip. Filter state is preserved so a live tweak does not click.
    pub fn set_params(&mut self, p: MicStripParams) {
        self.gate.set_threshold_db(p.gate_threshold_db);
        self.gate.ratio = p.gate_ratio.max(1.0);
        self.hpf.set(Biquad::highpass(self.sr, p.hpf_hz, 0.707));
        self.dj.set_resonance(p.dj_resonance);
        self.dj.set_knob(p.dj_knob);
        self.warmth.set(Biquad::low_shelf(self.sr, 180.0, 0.707, p.warmth_db));
        self.comp = Compressor::new(
            self.sr,
            p.comp_threshold_db,
            p.comp_ratio,
            8.0,
            120.0,
            p.makeup_db,
        );
        // carry the compressor envelope? new() zeroes it; acceptable on a param commit.
        self.params = p;
    }

    /// Just move the DJ knob (the common live gesture) — cheap, state-preserving.
    pub fn set_dj_knob(&mut self, knob: f32) {
        self.params.dj_knob = knob;
        self.dj.set_knob(knob);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let s = self.gate.process(x);
        let s = self.hpf.process(s);
        let s = self.dj.process(s);
        let s = self.warmth.process(s);
        self.comp.process(s)
    }

    pub fn process_block(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }

    pub fn reset(&mut self) {
        self.gate.reset();
        self.hpf.reset();
        self.dj.reset();
        self.warmth.reset();
        self.comp.reset();
    }
}

#[inline]
fn db_to_lin(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}
#[inline]
fn lin_to_db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}
#[inline]
fn time_coeff(ms: f32, sr: f32) -> f32 {
    (-1.0f32 / ((ms.max(0.01) / 1000.0) * sr)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|&s| s * s).sum::<f32>() / b.len().max(1) as f32).sqrt()
    }

    #[test]
    fn highpass_removes_dc() {
        let sr = 48_000.0;
        let mut hp = Biquad::highpass(sr, 90.0, 0.707);
        let mut buf: Vec<f32> = (0..sr as usize)
            .map(|i| 0.5 + 0.3 * (2.0 * PI * 500.0 * i as f32 / sr).sin())
            .collect();
        hp.process_block(&mut buf);
        // ignore the settling transient
        let tail = &buf[1000..];
        let mean: f32 = tail.iter().sum::<f32>() / tail.len() as f32;
        assert!(mean.abs() < 0.01, "DC survived HPF: mean {mean}");
        assert!(rms(tail) > 0.1, "500 Hz tone should pass the HPF");
    }

    #[test]
    fn dj_lowpass_kills_highs() {
        let sr = 48_000.0;
        let hi: Vec<f32> = (0..8192)
            .map(|i| (2.0 * PI * 12_000.0 * i as f32 / sr).sin())
            .collect();
        let mut dj = DjFilter::new(sr);
        dj.set_resonance(0.0);
        dj.set_knob(-1.0); // full low-pass, muffled
        let mut out = hi.clone();
        dj.process_block(&mut out);
        assert!(
            rms(&out[512..]) < 0.3 * rms(&hi[512..]),
            "DJ LPF at -1 should crush a 12 kHz tone"
        );
    }

    #[test]
    fn dj_highpass_kills_lows() {
        let sr = 48_000.0;
        let lo: Vec<f32> = (0..8192)
            .map(|i| (2.0 * PI * 80.0 * i as f32 / sr).sin())
            .collect();
        let mut dj = DjFilter::new(sr);
        dj.set_knob(1.0); // full high-pass, bass removed
        let mut out = lo.clone();
        dj.process_block(&mut out);
        assert!(
            rms(&out[512..]) < 0.3 * rms(&lo[512..]),
            "DJ HPF at +1 should remove an 80 Hz bass tone"
        );
    }

    #[test]
    fn dj_bypass_is_transparent() {
        let sr = 48_000.0;
        let sig: Vec<f32> = (0..4096)
            .map(|i| 0.4 * (2.0 * PI * 1000.0 * i as f32 / sr).sin())
            .collect();
        let mut dj = DjFilter::new(sr);
        dj.set_knob(0.0);
        let mut out = sig.clone();
        dj.process_block(&mut out);
        assert_eq!(out, sig, "knob at 0 must be a true bypass");
    }

    #[test]
    fn gate_silences_the_noise_floor() {
        let sr = 48_000.0;
        // quiet broadband-ish noise, ~-55 dBFS
        let noise: Vec<f32> = (0..sr as usize)
            .map(|i| 0.0018 * ((i as f32 * 0.9173).sin() + (i as f32 * 3.331).sin()))
            .collect();
        let before = rms(&noise);
        // Threshold sits well above the ~-52 dBFS floor so the expander bites.
        let mut gate = NoiseGate::new(sr, -40.0, 3.0, 1.0, 60.0);
        let mut out = noise.clone();
        gate.process_block(&mut out);
        let after = rms(&out[2000..]);
        assert!(after < 0.6 * before, "gate should pull the floor down: {before} -> {after}");
    }

    #[test]
    fn compressor_reduces_peaks() {
        let sr = 48_000.0;
        let hot: Vec<f32> = (0..sr as usize)
            .map(|i| 0.95 * (2.0 * PI * 220.0 * i as f32 / sr).sin())
            .collect();
        let mut comp = Compressor::new(sr, -24.0, 6.0, 3.0, 80.0, 0.0);
        let mut out = hot.clone();
        comp.process_block(&mut out);
        let peak_in = hot.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        let peak_out = out[4000..].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(peak_out < peak_in, "compressor should reduce peaks: {peak_in} -> {peak_out}");
    }

    #[test]
    fn strip_passes_voice_band_signal() {
        let sr = 48_000.0;
        let voice: Vec<f32> = (0..sr as usize)
            .map(|i| 0.3 * (2.0 * PI * 300.0 * i as f32 / sr).sin())
            .collect();
        let mut strip = MicStrip::new(sr, MicStripParams::default());
        let mut out = voice.clone();
        strip.process_block(&mut out);
        assert!(rms(&out[4000..]) > 0.05, "a 300 Hz voice tone must survive the strip");
        assert!(out.iter().all(|s| s.is_finite()), "strip produced non-finite samples");
    }
}
