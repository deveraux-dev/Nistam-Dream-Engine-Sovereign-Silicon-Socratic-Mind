//! spatial_hrtf — LOAD-TIME loader for a spatial-audio coefficient profile
//! (crosstalk-cancellation biquads + HRTF FastEqualizer matrix).
//!
//! The coefficient tables are DATA, not code: a profile can be thousands of lines of
//! biquad/matrix floats. Embedding that as a `static` array bloats a compilation unit
//! and forces a recompile on every tweak — so it lives as a JSON asset under
//! `assets/audio/` and is read at runtime here. This is LOAD-TIME (never the realtime
//! callback), so the heap allocation of decode/parse is inside forge-audio's documented
//! load-time carve-out.
//!
//! The DEFAULT coefficients are MATH-GENERATED, not lifted from any vendor: the DSP
//! (biquad filtering, crosstalk cancellation, HRTF EQ) is public-domain textbook — the
//! RBJ Audio-EQ-Cookbook design equations — so [`design`] computes our own coefficients
//! from `(fs, f0, Q, gain)`. Only a vendor's *specific tuned numbers* are proprietary; we
//! don't copy those. The loader stays SOURCE-AGNOSTIC so a clean-room or licensed profile
//! can still drop in at `SPATIAL_PROFILE_PATH` with the same schema, but nothing here
//! depends on one. The float DSP stays a design/leaf concern (assimilator firewall), never
//! compiled as a static table into the integer core.

use serde::Deserialize;

/// Default asset path (workspace-relative) for the full spatial profile drop-in.
pub const SPATIAL_PROFILE_PATH: &str = "assets/audio/thx_spatial_coeffs.json";

/// One biquad section's six coefficients: `[b0, b1, b2, a0, a1, a2]`.
pub type BiquadCoeffs = Vec<f64>;

/// A cascaded biquad filter — the `coeffsPerBiQuad` list from the profile.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FilterBlock {
    #[serde(rename = "coeffsPerBiQuad", default)]
    pub biquads: Vec<BiquadCoeffs>,
}

/// A crosstalk-cancellation stage: attenuation + inter-ear delay + two biquad filters.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CrosstalkProfile {
    #[serde(rename = "attenuationdB", default)]
    pub attenuation_db: f64,
    #[serde(rename = "delaySeconds", default)]
    pub delay_seconds: f64,
    #[serde(default)]
    pub filter1: FilterBlock,
    #[serde(default)]
    pub filter2: FilterBlock,
    #[serde(default)]
    pub fs: u32,
}

/// The HRTF FastEqualizer — a dense coefficient matrix.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FastEqualizer {
    #[serde(rename = "FASTEQ_MATRIX", default)]
    pub matrix: Vec<Vec<f64>>,
}

/// The headphone (binaural) DSP path.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeadphonePath {
    #[serde(rename = "FastEqualizer", default)]
    pub fast_eq: FastEqualizer,
}

/// A full spatial-audio coefficient profile parsed from a vendor/dev JSON asset.
/// Unknown fields are ignored and missing sections default, so schema drift between
/// vendor exports never hard-fails the load.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SpatialProfile {
    #[serde(rename = "deviceName", default)]
    pub device_name: String,
    #[serde(rename = "crosstalkV9", default)]
    pub crosstalk_v9: CrosstalkProfile,
    #[serde(rename = "crosstalkV11", default)]
    pub crosstalk_v11: CrosstalkProfile,
    #[serde(rename = "HeadphonePath", default)]
    pub headphone: HeadphonePath,
}

/// Why a spatial-profile load failed. Loud, never silent.
#[derive(Debug)]
pub enum SpatialProfileError {
    /// Filesystem read failed (missing asset, permissions).
    Io(String),
    /// The bytes were not valid profile JSON.
    Parse(String),
}

impl std::fmt::Display for SpatialProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpatialProfileError::Io(s) => write!(f, "spatial profile I/O: {s}"),
            SpatialProfileError::Parse(s) => write!(f, "spatial profile parse: {s}"),
        }
    }
}
impl std::error::Error for SpatialProfileError {}

impl SpatialProfile {
    /// Total biquad sections across both crosstalk stages — a liveness gauge that a
    /// real profile (not the empty default) actually loaded.
    pub fn biquad_count(&self) -> usize {
        let c = |p: &CrosstalkProfile| p.filter1.biquads.len() + p.filter2.biquads.len();
        c(&self.crosstalk_v9) + c(&self.crosstalk_v11)
    }

    /// FastEqualizer matrix dimensions `(rows, cols)` — `(0, 0)` when absent.
    pub fn fasteq_dims(&self) -> (usize, usize) {
        let m = &self.headphone.fast_eq.matrix;
        (m.len(), m.first().map(|r| r.len()).unwrap_or(0))
    }
}

/// Load and parse a spatial-audio coefficient profile from `path`. LOAD-TIME only.
pub fn load_spatial_profile(path: &str) -> Result<SpatialProfile, SpatialProfileError> {
    let bytes = std::fs::read(path).map_err(|e| SpatialProfileError::Io(format!("{path}: {e}")))?;
    serde_json::from_slice(&bytes).map_err(|e| SpatialProfileError::Parse(format!("{path}: {e}")))
}

/// Native biquad coefficient design — the RBJ "Audio EQ Cookbook" equations (public
/// domain). Load-time float math; the resulting integer-normalized coefficients feed the
/// zero-alloc [`Biquad`] processor. We generate our own numbers here so no vendor's tuned
/// coefficient table is ever needed or copied.
pub mod design {
    use super::BiquadCoeffs;
    use core::f64::consts::PI;

    /// Normalize `[b0,b1,b2,a0,a1,a2]` by `a0` → `[b0,b1,b2,1,a1,a2]` (the profile layout).
    fn normalize(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> BiquadCoeffs {
        vec![b0 / a0, b1 / a0, b2 / a0, 1.0, a1 / a0, a2 / a0]
    }

    #[inline]
    fn omega(fs: f64, f0: f64) -> (f64, f64) {
        let w0 = 2.0 * PI * f0 / fs;
        (w0.cos(), w0.sin())
    }

    /// 2nd-order low-pass at cutoff `f0` (Hz) with quality `q`.
    pub fn lowpass(fs: f64, f0: f64, q: f64) -> BiquadCoeffs {
        let (cw, sw) = omega(fs, f0);
        let alpha = sw / (2.0 * q);
        normalize((1.0 - cw) / 2.0, 1.0 - cw, (1.0 - cw) / 2.0, 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
    }

    /// 2nd-order high-pass at cutoff `f0` (Hz) with quality `q`.
    pub fn highpass(fs: f64, f0: f64, q: f64) -> BiquadCoeffs {
        let (cw, sw) = omega(fs, f0);
        let alpha = sw / (2.0 * q);
        normalize((1.0 + cw) / 2.0, -(1.0 + cw), (1.0 + cw) / 2.0, 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
    }

    /// Peaking EQ: `+/-gain_db` around center `f0` (Hz) with quality `q`.
    pub fn peaking(fs: f64, f0: f64, q: f64, gain_db: f64) -> BiquadCoeffs {
        let (cw, sw) = omega(fs, f0);
        let alpha = sw / (2.0 * q);
        let a = 10f64.powf(gain_db / 40.0);
        normalize(
            1.0 + alpha * a, -2.0 * cw, 1.0 - alpha * a,
            1.0 + alpha / a, -2.0 * cw, 1.0 - alpha / a,
        )
    }
}

/// Geometric spherical-head model — Interaural Time Delay (Woodworth) and head-shadow
/// filtering (Brown-Duda). Everything is derived from physical geometry (head radius,
/// speed of sound, incidence angle), so no measured/proprietary HRTF map is needed.
pub mod head {
    use super::BiquadCoeffs;

    /// Speed of sound in air, m/s (≈20 °C).
    pub const SPEED_OF_SOUND_MPS: f64 = 343.0;
    /// Radius of a generic human head, metres (≈8.75 cm).
    pub const DEFAULT_HEAD_RADIUS_M: f64 = 0.0875;
    /// Brown-Duda minimum head-shadow gain factor.
    const ALPHA_MIN: f64 = 0.1;

    /// Woodworth ITD (seconds) for a source at `azimuth_rad` (0 = front, +ve toward the
    /// far ear). The extra path length to the shadowed ear is `(a/c)(θ + sin θ)`, valid to
    /// ±90°; beyond that it saturates. Positive result = the far ear is delayed by this.
    pub fn woodworth_itd(azimuth_rad: f64, radius_m: f64, c: f64) -> f64 {
        let t = azimuth_rad.clamp(-core::f64::consts::FRAC_PI_2, core::f64::consts::FRAC_PI_2);
        (radius_m / c) * (t + t.sin())
    }

    /// ITD in whole samples at `fs` — what a delay line needs.
    pub fn itd_samples(azimuth_rad: f64, radius_m: f64, c: f64, fs: f64) -> usize {
        (woodworth_itd(azimuth_rad, radius_m, c).abs() * fs).round() as usize
    }

    /// Brown-Duda head-shadow: a first-order filter (returned in the 6-coeff biquad layout,
    /// `b2=a2=0`). `incidence_rad` is the angle from the ear axis — 0 = ipsilateral (facing
    /// the source, highs boosted), π = contralateral (in shadow, highs attenuated to αmin).
    pub fn head_shadow(fs: f64, incidence_rad: f64, radius_m: f64, c: f64) -> BiquadCoeffs {
        let inc = incidence_rad.clamp(0.0, core::f64::consts::PI);
        // α(θ): 1+αmin/2 at ipsilateral → αmin at contralateral (150° maps to 180°).
        let alpha = (1.0 + ALPHA_MIN / 2.0) + (1.0 - ALPHA_MIN / 2.0) * (1.2 * inc).cos();
        let w0 = c / radius_m; // head-shadow corner (rad/s)
        let g = 2.0 * fs; // bilinear-transform gain
        // Analog H(s) = (w0 + α s)/(w0 + s), bilinear → first-order digital.
        let den = w0 + g;
        let b0 = (w0 + alpha * g) / den;
        let b1 = (w0 - alpha * g) / den;
        let a1 = (w0 - g) / den;
        vec![b0, b1, 0.0, 1.0, a1, 0.0]
    }
}

/// A zero-alloc biquad filter (Direct Form I). Coefficients come from [`design`] or a
/// loaded profile; state is two input + two output taps. Float DSP is legitimate on the
/// audio path — the zero-alloc law is about heap, and this holds none.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f64, b1: f64, b2: f64, a1: f64, a2: f64,
    x1: f64, x2: f64, y1: f64, y2: f64,
}

impl Biquad {
    /// Build from a 6-coefficient `[b0,b1,b2,a0,a1,a2]` section (any `a0`, re-normalized).
    /// Returns `None` if the slice isn't 6 long or `a0` is zero.
    pub fn from_coeffs(c: &[f64]) -> Option<Self> {
        if c.len() != 6 || c[3] == 0.0 {
            return None;
        }
        let a0 = c[3];
        Some(Self {
            b0: c[0] / a0, b1: c[1] / a0, b2: c[2] / a0,
            a1: c[4] / a0, a2: c[5] / a0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        })
    }

    /// Process one sample. Zero allocation, branch-free.
    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Recursive crosstalk canceller (XTC) for speaker / closely-coupled-node playback: keeps
/// the left ear from hearing the right channel at full power. Public-domain recursive
/// layout — take the opposite channel's PAST output (delayed by the ITD, ≥1 sample so the
/// loop is causal and stable), phase-invert it, apply the head-shadow attenuation, and mix
/// it back into this channel. Zero-alloc: fixed ring buffers, fixed biquad state.
pub struct CrosstalkCanceller {
    ring_l: [f64; Self::RING],
    ring_r: [f64; Self::RING],
    write: usize,
    delay: usize,
    atten: f64,
    shadow_l: Biquad,
    shadow_r: Biquad,
}

impl CrosstalkCanceller {
    const RING: usize = 256; // > any plausible ITD in samples (≈0.7 ms ≈ 34 @48k)

    /// Build a canceller: `delay_samples` = ITD (clamped ≥1, < ring), `atten` in `[0,1]`
    /// the cross-path attenuation, `shadow` the head-shadow biquad coefficients applied to
    /// the cancellation signal.
    pub fn new(delay_samples: usize, atten: f64, shadow: &[f64]) -> Self {
        let bq = Biquad::from_coeffs(shadow).unwrap_or_else(|| {
            Biquad::from_coeffs(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]).unwrap()
        });
        Self {
            ring_l: [0.0; Self::RING],
            ring_r: [0.0; Self::RING],
            write: 0,
            delay: delay_samples.clamp(1, Self::RING - 1),
            atten: atten.clamp(0.0, 1.0),
            shadow_l: bq,
            shadow_r: bq,
        }
    }

    #[inline]
    fn tap(ring: &[f64; Self::RING], write: usize, delay: usize) -> f64 {
        ring[(write + Self::RING - delay) % Self::RING]
    }

    /// Process one stereo frame → the crosstalk-cancelled `(left, right)`.
    #[inline]
    pub fn process(&mut self, l: f64, r: f64) -> (f64, f64) {
        // Cancellation signal = opposite channel's delayed past output, shadowed + attenuated.
        let c_l = self.atten * self.shadow_l.process(Self::tap(&self.ring_r, self.write, self.delay));
        let c_r = self.atten * self.shadow_r.process(Self::tap(&self.ring_l, self.write, self.delay));
        let yl = l - c_l; // phase-invert (−) and mix into the opposite ear's path
        let yr = r - c_r;
        self.ring_l[self.write] = yl;
        self.ring_r[self.write] = yr;
        self.write = (self.write + 1) % Self::RING;
        (yl, yr)
    }
}

impl SpatialProfile {
    /// A math-generated default spatial profile — a gentle HRTF-shaping cascade computed
    /// from the RBJ equations at 48 kHz. No vendor data: this is the honest zero-dependency
    /// default the sound axis ships with until a clean-room/licensed profile is loaded.
    pub fn designed_default() -> Self {
        let fs = 48_000.0;
        let filter1 = FilterBlock {
            biquads: vec![
                design::highpass(fs, 120.0, 0.707), // rumble cut
                design::peaking(fs, 3000.0, 1.0, 3.0), // presence lift (HRTF pinna notch region)
            ],
        };
        let filter2 = FilterBlock {
            biquads: vec![design::lowpass(fs, 16000.0, 0.707)], // air roll-off
        };
        Self {
            device_name: "designed-default-rbj".into(),
            crosstalk_v9: CrosstalkProfile {
                attenuation_db: -3.5,
                delay_seconds: 0.0002262,
                filter1,
                filter2,
                fs: 48_000,
            },
            crosstalk_v11: CrosstalkProfile::default(),
            headphone: HeadphonePath::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_path() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/audio/spatial_profile.example.json").into()
    }

    #[test]
    fn loads_example_profile_and_counts_coeffs() {
        let p = load_spatial_profile(&example_path()).expect("example profile must load");
        assert_eq!(p.device_name, "example-generic-hrtf-profile");
        assert_eq!(p.crosstalk_v9.fs, 48000);
        assert!((p.crosstalk_v9.attenuation_db - -3.5).abs() < 1e-9);
        // 2 biquads in filter1 + 1 in filter2 = 3 sections.
        assert_eq!(p.biquad_count(), 3);
        assert_eq!(p.fasteq_dims(), (2, 3));
        // First biquad's six coefficients survived the round-trip.
        assert_eq!(p.crosstalk_v9.filter1.biquads[0].len(), 6);
    }

    #[test]
    fn missing_asset_is_a_loud_io_error() {
        let err = load_spatial_profile("does/not/exist.json").unwrap_err();
        assert!(matches!(err, SpatialProfileError::Io(_)));
    }

    #[test]
    fn empty_default_profile_gauges_as_empty() {
        let p = SpatialProfile::default();
        assert_eq!(p.biquad_count(), 0);
        assert_eq!(p.fasteq_dims(), (0, 0));
    }

    #[test]
    fn rbj_lowpass_passes_dc_and_normalizes_a0() {
        // A low-pass has unity gain at DC: summing b's / summing a's == 1.
        let c = design::lowpass(48_000.0, 1000.0, 0.707);
        assert_eq!(c.len(), 6);
        assert!((c[3] - 1.0).abs() < 1e-12, "a0 must be normalized to 1");
        let dc = (c[0] + c[1] + c[2]) / (c[3] + c[4] + c[5]);
        assert!((dc - 1.0).abs() < 1e-9, "low-pass DC gain must be ~1, got {dc}");
    }

    #[test]
    fn rbj_highpass_blocks_dc() {
        // A high-pass has zero gain at DC: b0+b1+b2 ~ 0.
        let c = design::highpass(48_000.0, 1000.0, 0.707);
        assert!((c[0] + c[1] + c[2]).abs() < 1e-9, "high-pass must block DC");
    }

    #[test]
    fn biquad_identity_is_passthrough() {
        // [1,0,0,1,0,0] must return the input untouched.
        let mut bq = Biquad::from_coeffs(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]).unwrap();
        for x in [0.0, 1.0, -0.5, 0.25] {
            assert!((bq.process(x) - x).abs() < 1e-12);
        }
    }

    #[test]
    fn biquad_lowpass_attenuates_nyquist_more_than_dc() {
        // Drive a designed low-pass: a slow DC-ish signal survives; alternating
        // (Nyquist) is crushed. Proves the coefficients actually filter.
        let c = design::lowpass(48_000.0, 1000.0, 0.707);
        let mut lo = Biquad::from_coeffs(&c).unwrap();
        let mut hi = Biquad::from_coeffs(&c).unwrap();
        let (mut dc_energy, mut nyq_energy) = (0.0f64, 0.0f64);
        for n in 0..2048 {
            dc_energy += lo.process(1.0).abs();
            let nyq = if n % 2 == 0 { 1.0 } else { -1.0 };
            nyq_energy += hi.process(nyq).abs();
        }
        assert!(dc_energy > nyq_energy * 10.0, "low-pass must pass DC ≫ Nyquist");
    }

    #[test]
    fn woodworth_itd_is_zero_at_front_and_max_at_side() {
        use head::{woodworth_itd, DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS};
        let r = DEFAULT_HEAD_RADIUS_M;
        let c = SPEED_OF_SOUND_MPS;
        assert!(woodworth_itd(0.0, r, c).abs() < 1e-12, "front source has no ITD");
        let side = woodworth_itd(core::f64::consts::FRAC_PI_2, r, c);
        // Full side ≈ (a/c)(π/2 + 1) ≈ 0.655 ms — the textbook ~0.6–0.7 ms max.
        assert!((0.0004..0.0009).contains(&side), "side ITD ~0.65ms, got {side}");
        // Antisymmetric: +θ and −θ mirror.
        assert!((woodworth_itd(0.5, r, c) + woodworth_itd(-0.5, r, c)).abs() < 1e-12);
    }

    #[test]
    fn head_shadow_passes_dc_and_attenuates_contralateral_highs() {
        use head::{head_shadow, DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS};
        let (r, c, fs) = (DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS, 44_100.0);
        // Contralateral (in shadow): DC ~ unity, but Nyquist strongly attenuated.
        let contra = head_shadow(fs, core::f64::consts::PI, r, c);
        let dc = contra[0] + contra[1]; // first-order: b0 + b1 at z=1 over 1 + a1
        let dc_gain = dc / (1.0 + contra[4]);
        assert!((dc_gain - 1.0).abs() < 0.05, "head shadow ~unity at DC, got {dc_gain}");
        let mut hi = Biquad::from_coeffs(&contra).unwrap();
        let mut lo = Biquad::from_coeffs(&contra).unwrap();
        let (mut nyq_e, mut dc_e) = (0.0f64, 0.0f64);
        for n in 0..2048 {
            nyq_e += hi.process(if n % 2 == 0 { 1.0 } else { -1.0 }).abs();
            dc_e += lo.process(1.0).abs();
        }
        assert!(dc_e > nyq_e * 3.0, "contralateral shadow must cut highs: dc {dc_e} nyq {nyq_e}");
    }

    #[test]
    fn crosstalk_canceller_is_stable_and_injects_anti_signal() {
        use head::{head_shadow, itd_samples, DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS};
        let (r, c, fs) = (DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS, 48_000.0);
        let d = itd_samples(core::f64::consts::FRAC_PI_2, r, c, fs).max(1);
        let shadow = head_shadow(fs, core::f64::consts::PI, r, c);
        let mut xtc = CrosstalkCanceller::new(d, 0.7, &shadow);

        // Drive an impulse in LEFT only; assert the loop stays bounded (stable) and that a
        // cancellation signal appears in the RIGHT channel after the delay.
        let mut max_abs = 0.0f64;
        let mut right_energy = 0.0f64;
        for n in 0..4096 {
            let x = if n == 0 { 1.0 } else { 0.0 };
            let (yl, yr) = xtc.process(x, 0.0);
            max_abs = max_abs.max(yl.abs()).max(yr.abs());
            if n > 0 { right_energy += yr.abs(); }
        }
        assert!(max_abs.is_finite() && max_abs < 10.0, "XTC must stay bounded, got {max_abs}");
        assert!(right_energy > 1e-6, "XTC must inject a cancellation signal into the far channel");
    }

    #[test]
    fn designed_default_needs_no_vendor_file() {
        // The shipped default is math-generated — non-empty with ZERO external data.
        let p = SpatialProfile::designed_default();
        assert_eq!(p.biquad_count(), 3);
        assert_eq!(p.crosstalk_v9.fs, 48_000);
        // Every generated section is a usable biquad.
        for bq in p.crosstalk_v9.filter1.biquads.iter().chain(&p.crosstalk_v9.filter2.biquads) {
            assert!(Biquad::from_coeffs(bq).is_some(), "designed coeffs must build a biquad");
        }
    }

    /// If the full vendor blob has been dropped in, prove it parses too — otherwise
    /// skip (green), so this stays honest whether or not the proprietary asset is present.
    #[test]
    fn full_vendor_blob_parses_when_present() {
        let path: String =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../", "assets/audio/thx_spatial_coeffs.json").into();
        match load_spatial_profile(&path) {
            Ok(p) => {
                assert!(
                    p.biquad_count() > 0 || p.fasteq_dims().0 > 0,
                    "a present vendor blob must carry coefficients",
                );
            }
            Err(SpatialProfileError::Io(_)) => {
                eprintln!("[spatial_hrtf] vendor blob not present — skipping (drop it at {path})");
            }
            Err(e) => panic!("vendor blob present but failed to parse: {e}"),
        }
    }
}
