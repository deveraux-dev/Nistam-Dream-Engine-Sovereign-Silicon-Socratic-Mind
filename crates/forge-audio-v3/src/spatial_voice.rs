//! spatial_voice — a single moving sound source rendered to the two ears.
//!
//! [`SpatialVoice`] is the runtime that turns a listener-relative emitter POSITION into a
//! binaural stereo pair, fed entirely by forge-audio's own geometric HRTF math
//! ([`crate::spatial_hrtf`]) — no measured/vendor HRTF table, no forge-ml dependency (the
//! 5D-codebook side stays firewalled; the app layer feeds coefficients in). Everything on
//! the per-sample path is zero-heap: a fixed ring delay line for the Interaural Time
//! Difference and two `Copy` [`Biquad`]s for the Brown-Duda head shadow. Float DSP is
//! legitimate here — the zero-alloc law is about the heap, and [`SpatialVoice::process`]
//! touches none.
//!
//! Geometry rides the workspace-canonical [`glam::DVec3`] (the same vector algebra
//! forge-geo and the rest of the engine use) — we do NOT hand-roll a vector type.

use glam::DVec3;

use crate::spatial_hrtf::{
    head::{self, DEFAULT_HEAD_RADIUS_M, SPEED_OF_SOUND_MPS},
    Biquad, CrosstalkCanceller,
};

/// Identity (pass-through) biquad `[1,0,0,1,0,0]` — the safe fallback when a generated
/// coefficient set is somehow degenerate. Built from the same public constructor the rest
/// of the DSP uses, so it can never diverge from the real filter's numerics.
fn identity_biquad() -> Biquad {
    Biquad::from_coeffs(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]).expect("identity coeffs are valid")
}

// ── Geometry ────────────────────────────────────────────────────────────────────────

/// Listener-relative angles derived from an emitter position. The listener sits at the
/// origin facing `+Y`, `+X` right, `+Z` up (right-handed). Azimuth is compass degrees
/// (`0` front, `90` right, `180` back, `270` left) — the SAME convention
/// `forge_ml::acoustic_index::compile_intent` uses, so a live voice and the 5D codebook
/// share one θ lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// Compass azimuth degrees, `[0, 360)`.
    pub azimuth_deg: f64,
    /// Elevation degrees, `[-90, 90]` (`0` ear-level, `+90` overhead).
    pub elevation_deg: f64,
    /// Interaural lateral angle (radians, `[-π/2, π/2]`, `+` = right) — the coordinate that
    /// drives ITD. Correctly folds elevation in: an overhead source has lateral `0`.
    pub lateral_rad: f64,
    /// Source distance in metres (`>= 0`).
    pub distance_m: f64,
    /// Angle (radians `[0, π]`) between the source and the RIGHT ear axis (`+X`).
    pub incidence_right_rad: f64,
    /// Angle (radians `[0, π]`) between the source and the LEFT ear axis (`-X`).
    pub incidence_left_rad: f64,
}

impl Geometry {
    /// Derive from a listener-relative emitter position (metres). A source AT the head
    /// (degenerate zero vector) resolves to straight-front so the DSP never NaNs.
    pub fn from_position(p: DVec3) -> Self {
        let r = p.length();
        let u = if r < 1e-9 { DVec3::new(0.0, 1.0, 0.0) } else { p / r };

        let mut az = p.x.atan2(p.y).to_degrees(); // 0 at +Y (front), +90 at +X (right)
        if az < 0.0 {
            az += 360.0;
        }
        let horiz = (p.x * p.x + p.y * p.y).sqrt();
        let el = p.z.atan2(horiz).to_degrees();

        let sx = u.x.clamp(-1.0, 1.0);
        let lateral = sx.asin();
        let inc_r = sx.acos(); // angle to +X (right ear)
        let inc_l = core::f64::consts::PI - inc_r; // angle to -X (left ear)

        Self {
            azimuth_deg: az,
            elevation_deg: el,
            lateral_rad: lateral,
            distance_m: r,
            incidence_right_rad: inc_r,
            incidence_left_rad: inc_l,
        }
    }

    /// Build from azimuth+elevation (degrees) at unit distance — for callers driving the
    /// voice by angles rather than Cartesian points.
    pub fn from_angles(azimuth_deg: f64, elevation_deg: f64) -> Self {
        let az = azimuth_deg.to_radians();
        let el = elevation_deg.to_radians();
        let ce = el.cos();
        // x=sin(az)cos(el) [right], y=cos(az)cos(el) [front], z=sin(el) — inverts the compass.
        Self::from_position(DVec3::new(az.sin() * ce, az.cos() * ce, el.sin()))
    }

    /// `true` when the source is on the listener's right (right ear is NEAR). At dead-center
    /// (lateral 0) reports `true` with ITD 0, so the choice is immaterial.
    #[inline]
    pub fn source_on_right(&self) -> bool {
        self.lateral_rad >= 0.0
    }
}

// ── Zero-alloc mono delay line (the ITD engine) ───────────────────────────────────────

/// A fixed-capacity mono delay line with independently-tapped, fractionally-interpolated
/// reads — the mechanism of Interaural Time Difference. One shared input history; each ear
/// taps it at its own delay. Zero heap: a fixed `[f64; RING]` ring, no `Vec`, no `Box`.
#[derive(Clone)]
pub struct DelayLine {
    buf: [f64; Self::RING],
    write: usize,
}

impl Default for DelayLine {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayLine {
    /// Ring length in samples. `> 0.7 ms` at any sane rate (≈34 @48k, ≈68 @96k) with a wide
    /// margin, so a whole-head ITD always fits.
    pub const RING: usize = 512;

    pub fn new() -> Self {
        Self { buf: [0.0; Self::RING], write: 0 }
    }

    /// Push one input sample (advances the write head). Zero-alloc.
    #[inline]
    pub fn push(&mut self, x: f64) {
        self.buf[self.write] = x;
        self.write = (self.write + 1) % Self::RING;
    }

    /// Read the signal `delay` samples in the past, linearly interpolating between the two
    /// bracketing samples so a fractional (sub-sample) ITD renders smoothly. `delay == 0`
    /// returns the most-recent sample. Clamped into the ring. Zero-alloc.
    #[inline]
    pub fn tap(&self, delay: f64) -> f64 {
        let d = delay.clamp(0.0, (Self::RING - 2) as f64);
        let i = d.floor() as usize;
        let frac = d - i as f64;
        // Most-recent sample sits at write-1; older samples count backward from there.
        let a = self.buf[(self.write + Self::RING - 1 - i) % Self::RING];
        let b = self.buf[(self.write + Self::RING - 2 - i) % Self::RING];
        a + (b - a) * frac
    }

    /// Zero the history (e.g. between unrelated render passes). Zero-alloc.
    pub fn clear(&mut self) {
        self.buf = [0.0; Self::RING];
        self.write = 0;
    }
}

// ── The voice ─────────────────────────────────────────────────────────────────────────

/// How the two channels reach the ears — headphones (independent ears) or a speaker pair
/// (needs crosstalk cancellation so the left channel doesn't leak to the right ear).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Binaural: the ITD + head-shadow pair is the whole render.
    Headphone,
    /// Loudspeakers / closely-coupled nodes: add a recursive crosstalk canceller.
    Speaker,
}

/// A single spatialised sound source. Set its position at control-rate ([`set_position`] /
/// [`set_angles`]); pull stereo frames at audio-rate ([`process`], zero-alloc). The DSP is
/// generated from geometry every position update — Woodworth ITD onto the [`DelayLine`],
/// Brown-Duda head shadow onto the per-ear [`Biquad`]s — so there is no coefficient table.
///
/// [`set_position`]: SpatialVoice::set_position
/// [`set_angles`]: SpatialVoice::set_angles
/// [`process`]: SpatialVoice::process
pub struct SpatialVoice {
    fs: f64,
    head_radius_m: f64,
    min_distance_m: f64,
    geom: Geometry,
    // control-rate derived state, read by the hot path:
    delay_l: f64,
    delay_r: f64,
    gain: f64,
    shadow_l: Biquad,
    shadow_r: Biquad,
    // hot-path state:
    line: DelayLine,
    xtc: Option<CrosstalkCanceller>,
}

impl SpatialVoice {
    /// New voice at sample rate `fs` in the given playback mode, source initialised dead
    /// front at 1 m. `Speaker` mode attaches a crosstalk canceller sized to the maximum
    /// whole-head ITD.
    pub fn new(fs: f64, mode: PlaybackMode) -> Self {
        let head_radius_m = DEFAULT_HEAD_RADIUS_M;
        let xtc = match mode {
            PlaybackMode::Headphone => None,
            PlaybackMode::Speaker => {
                // Size the canceller to the full-side ITD and the contralateral head shadow.
                let d = head::itd_samples(
                    core::f64::consts::FRAC_PI_2,
                    head_radius_m,
                    SPEED_OF_SOUND_MPS,
                    fs,
                )
                .max(1);
                let shadow = head::head_shadow(
                    fs,
                    core::f64::consts::PI,
                    head_radius_m,
                    SPEED_OF_SOUND_MPS,
                );
                Some(CrosstalkCanceller::new(d, 0.7, &shadow))
            }
        };
        let mut v = Self {
            fs,
            head_radius_m,
            min_distance_m: 0.15, // clamp: never boost louder than ~7× (source on the ear)
            geom: Geometry::from_position(DVec3::new(0.0, 1.0, 0.0)),
            delay_l: 0.0,
            delay_r: 0.0,
            gain: 1.0,
            shadow_l: identity_biquad(),
            shadow_r: identity_biquad(),
            line: DelayLine::new(),
            xtc,
        };
        v.recompute();
        v
    }

    /// Move the source to a listener-relative Cartesian position (metres). Control-rate:
    /// regenerates the ITD delays + head-shadow filters. Not for the audio callback.
    pub fn set_position(&mut self, position: DVec3) {
        self.geom = Geometry::from_position(position);
        self.recompute();
    }

    /// Move the source by azimuth+elevation (degrees) at unit distance. Control-rate.
    pub fn set_angles(&mut self, azimuth_deg: f64, elevation_deg: f64) {
        self.geom = Geometry::from_angles(azimuth_deg, elevation_deg);
        self.recompute();
    }

    /// Regenerate every control-rate coefficient from the current geometry.
    fn recompute(&mut self) {
        let c = SPEED_OF_SOUND_MPS;
        // Woodworth ITD (seconds) → fractional samples, applied to the FAR ear only.
        let itd = head::woodworth_itd(self.geom.lateral_rad, self.head_radius_m, c).abs() * self.fs;
        if self.geom.source_on_right() {
            self.delay_r = 0.0; // right ear near
            self.delay_l = itd; // left ear far
        } else {
            self.delay_l = 0.0;
            self.delay_r = itd;
        }
        // Brown-Duda head shadow per ear (contralateral ear gets the high-freq cut).
        let sl = head::head_shadow(self.fs, self.geom.incidence_left_rad, self.head_radius_m, c);
        let sr = head::head_shadow(self.fs, self.geom.incidence_right_rad, self.head_radius_m, c);
        self.shadow_l = Biquad::from_coeffs(&sl).unwrap_or_else(identity_biquad);
        self.shadow_r = Biquad::from_coeffs(&sr).unwrap_or_else(identity_biquad);
        // 1/r distance attenuation, clamped so a near source can't explode the level.
        self.gain = (1.0 / self.geom.distance_m.max(self.min_distance_m)).min(1.0 / self.min_distance_m);
    }

    /// Render ONE mono input sample to a binaural `(left, right)` frame. **Zero heap** — the
    /// entire path is fixed-array ring reads + two `Copy` biquads (+ the fixed-ring XTC in
    /// speaker mode). Safe to call from the realtime audio callback.
    #[inline]
    pub fn process(&mut self, x: f64) -> (f64, f64) {
        self.line.push(x);
        let dl = self.line.tap(self.delay_l);
        let dr = self.line.tap(self.delay_r);
        let mut l = self.shadow_l.process(dl) * self.gain;
        let mut r = self.shadow_r.process(dr) * self.gain;
        if let Some(xtc) = self.xtc.as_mut() {
            let (a, b) = xtc.process(l, r);
            l = a;
            r = b;
        }
        (l, r)
    }

    /// Current geometry snapshot (control-rate state).
    #[inline]
    pub fn geometry(&self) -> Geometry {
        self.geom
    }

    /// Signed interaural time difference in samples: `delay_left - delay_right`. Positive =
    /// the left ear is delayed = the source is on the RIGHT. Sweeps monotonically as a source
    /// crosses left→right, which is exactly the ITD ramp the spatial proof asserts.
    #[inline]
    pub fn signed_itd_samples(&self) -> f64 {
        self.delay_l - self.delay_r
    }

    /// The far (delayed) ear's whole-sample delay magnitude — the classic non-signed ITD.
    #[inline]
    pub fn itd_samples(&self) -> f64 {
        self.delay_l.max(self.delay_r)
    }

    /// The broadband distance gain currently applied to both ears.
    #[inline]
    pub fn gain(&self) -> f64 {
        self.gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    // ── Geometry ──────────────────────────────────────────────────────────────

    #[test]
    fn front_source_has_no_itd_and_is_symmetric() {
        let g = Geometry::from_position(DVec3::new(0.0, 2.0, 0.0));
        assert!(g.azimuth_deg.abs() < 1e-6);
        assert!(g.lateral_rad.abs() < EPS);
        assert!((g.incidence_left_rad - g.incidence_right_rad).abs() < EPS);
    }

    #[test]
    fn compass_matches_the_codebook_lexicon() {
        // Diagonals MUST equal forge_ml::acoustic_index::compile_intent's angles.
        assert!((Geometry::from_position(DVec3::new(1.0, 1.0, 0.0)).azimuth_deg - 45.0).abs() < 1e-6);
        assert!((Geometry::from_position(DVec3::new(-1.0, 1.0, 0.0)).azimuth_deg - 315.0).abs() < 1e-6);
        assert!((Geometry::from_position(DVec3::new(1.0, -1.0, 0.0)).azimuth_deg - 135.0).abs() < 1e-6);
        assert!((Geometry::from_position(DVec3::new(-1.0, -1.0, 0.0)).azimuth_deg - 225.0).abs() < 1e-6);
    }

    #[test]
    fn overhead_source_has_zero_lateral() {
        let g = Geometry::from_position(DVec3::new(0.0, 0.0, 5.0));
        assert!((g.elevation_deg - 90.0).abs() < 1e-6);
        assert!(g.lateral_rad.abs() < EPS, "overhead → no ITD");
    }

    #[test]
    fn degenerate_head_center_is_safe_front() {
        let g = Geometry::from_position(DVec3::ZERO);
        assert!(g.distance_m < EPS);
        assert!(g.lateral_rad.abs() < EPS);
        assert!(g.azimuth_deg.abs() < 1e-6);
    }

    #[test]
    fn incidence_pair_sums_to_pi() {
        for (az, el) in [(0.0, 0.0), (37.0, 12.0), (123.0, -40.0), (270.0, 5.0)] {
            let g = Geometry::from_angles(az, el);
            assert!((g.incidence_left_rad + g.incidence_right_rad - core::f64::consts::PI).abs() < 1e-9);
        }
    }

    // ── Delay line ────────────────────────────────────────────────────────────

    #[test]
    fn delay_line_taps_integer_and_fractional() {
        let mut d = DelayLine::new();
        // Push a ramp 1,2,3,4,5; most-recent (delay 0) is 5, delay 1 is 4, ...
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] {
            d.push(x);
        }
        assert!((d.tap(0.0) - 5.0).abs() < EPS);
        assert!((d.tap(1.0) - 4.0).abs() < EPS);
        // Halfway between sample 5 (delay 0) and 4 (delay 1) = 4.5.
        assert!((d.tap(0.5) - 4.5).abs() < EPS);
        // A single impulse emerges exactly N samples later.
        let mut imp = DelayLine::new();
        imp.push(1.0);
        for _ in 0..9 {
            imp.push(0.0);
        }
        assert!((imp.tap(9.0) - 1.0).abs() < EPS, "impulse delayed by exactly 9");
    }

    // ── Voice: the ITD ramp + contralateral shadow (the DONE-BAR shape) ─────────

    #[test]
    fn signed_itd_ramps_monotonically_left_to_right() {
        // Fly the source along a straight line front-left→back-right, held off the head by a
        // small +Z so it never crosses the origin singularity. The SIGNED ITD must increase
        // monotonically as the source crosses from the left hemisphere to the right.
        let mut v = SpatialVoice::new(48_000.0, PlaybackMode::Headphone);
        let mut last = f64::NEG_INFINITY;
        for k in 0..=20 {
            let t = k as f64 / 20.0;
            let x = -2.0 + 4.0 * t; //  -2 → +2 (left → right)
            let y = 1.5 - 3.0 * t; // +1.5 → -1.5 (front → back)
            v.set_position(DVec3::new(x, y, 0.5));
            let s = v.signed_itd_samples();
            assert!(s >= last - 1e-9, "signed ITD must not decrease: {s} < {last}");
            last = s;
        }
        // End points: left start delays the LEFT-lead... i.e. source-left → right ear far →
        // signed = delay_l - delay_r < 0; source-right → > 0. Ramp crosses zero.
        assert!(last > 0.0, "source ends on the right → positive signed ITD");
    }

    #[test]
    fn contralateral_ear_loses_high_frequencies() {
        // Source hard-right: the LEFT (far/contralateral) ear must carry less high-frequency
        // energy than the RIGHT (near/ipsilateral) ear for the same bright input.
        let mut v = SpatialVoice::new(48_000.0, PlaybackMode::Headphone);
        v.set_position(DVec3::new(3.0, 0.0, 0.0)); // hard right
        let (mut l_hf, mut r_hf) = (0.0f64, 0.0f64);
        for n in 0..4096 {
            let nyq = if n % 2 == 0 { 1.0 } else { -1.0 }; // brightest possible (Nyquist)
            let (l, r) = v.process(nyq);
            if n > 256 {
                l_hf += l.abs();
                r_hf += r.abs();
            }
        }
        assert!(
            r_hf > l_hf * 1.2,
            "near ear must keep more HF than the shadowed far ear: near {r_hf} vs far {l_hf}",
        );
    }

    #[test]
    fn far_ear_high_freq_drops_as_source_swings_into_shadow() {
        // As the source swings from front (az 0) to hard-left (az 270), the RIGHT ear moves
        // into shadow, so its high-frequency energy must fall.
        let fs = 48_000.0;
        let hf_energy_right = |az: f64| {
            let mut v = SpatialVoice::new(fs, PlaybackMode::Headphone);
            v.set_angles(az, 0.0);
            let mut e = 0.0f64;
            for n in 0..4096 {
                let nyq = if n % 2 == 0 { 1.0 } else { -1.0 };
                let (_l, r) = v.process(nyq);
                if n > 256 {
                    e += r.abs();
                }
            }
            e
        };
        let front = hf_energy_right(0.0);
        let left = hf_energy_right(270.0); // source left → right ear shadowed
        assert!(front > left * 1.2, "right-ear HF must drop as source swings left: {front} → {left}");
    }

    #[test]
    fn process_is_bounded_and_zero_alloc_by_construction() {
        // By construction the hot path allocates nothing: DelayLine is a fixed [f64; RING],
        // the shadow filters are Copy Biquads, the optional XTC is fixed rings. This test
        // hammers the path and asserts it stays finite/bounded — a functional companion to
        // that structural guarantee (no Vec/Box/String/HashMap appears in `process`).
        let mut v = SpatialVoice::new(48_000.0, PlaybackMode::Speaker);
        v.set_angles(35.0, 10.0);
        let mut peak = 0.0f64;
        for n in 0..48_000 {
            let x = (n as f64 * 0.02).sin();
            let (l, r) = v.process(x);
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert!(peak.is_finite() && peak < 100.0, "voice must stay bounded, got {peak}");
    }

    #[test]
    fn speaker_mode_attaches_a_crosstalk_canceller() {
        let head = SpatialVoice::new(48_000.0, PlaybackMode::Headphone);
        let spk = SpatialVoice::new(48_000.0, PlaybackMode::Speaker);
        assert!(head.xtc.is_none());
        assert!(spk.xtc.is_some());
    }
}
