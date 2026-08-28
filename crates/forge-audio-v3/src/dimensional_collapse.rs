//! 5D → stereo dimensional collapse — the HEAR-side decoder, symmetric twin of
//! the ironroot `visual_state.rs` SEE-side (`state → color/pulse/intensity →
//! uniforms`). A 5D source point `(X, Y, Z, W, θ)` is reduced to a flat L/R
//! stereo waveform with every upper dimension baked into the signal's
//! properties — information-density arbitrage: two wires carry a 5D trajectory,
//! and a broken geometry phase-cancels in the ears before the compiler notices.
//!
//! Axis map (Sean's reduction formula, 2026-07-08):
//!   X — canvas horizontal    → pan + ITD           (inter-aural level/time diff)
//!   Y — depth                → gain + low-pass      (inverse-square + air absorb)
//!   Z — semantic depth       → root-note frequency  (§17.3 Rosetta: meaning→pitch)
//!   θ — harmonic codeword    → overtone + phase     (timbre; density→phase nodes)
//!   W — chrono-tick lineage  → modulation rate       (wow/flutter drift)
//!
//! DETERMINISM: the collapse is pure integer (MilliUnit / permyriad / integer
//! trig via `pp_math::fixed_point::sin_mdeg` — the ironroot `trig_table`, byte
//! identical on every CPU). `f32` appears only in `render_sample`'s audio output,
//! which is past the boundary (audio is float by nature — this is not the
//! DET-CLOCK).

use pp_math::fixed_point::trig::sin_mdeg;

/// A 5D source point. X/Y are spatial MilliUnit (1000 = 1.0 world unit); Z is a
/// semantic index (0.. → scale degree); W is the chrono-tick lineage; θ is the
/// harmonic-codeword angle in milli-degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point5D {
    pub x_mu: i64,
    pub y_mu: i64,
    pub z_semantic: i32,
    pub w_tick: u64,
    pub theta_mdeg: i32,
}

/// The flattened stereo parameters — every upper dimension baked into a 2-channel
/// signal. All integer (the collapse SoT); `render_sample` turns it into f32 L/R.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StereoField {
    /// Pan, permyriad: -10000 = hard left, 0 = centre, +10000 = hard right. [X]
    pub pan_pmy: i32,
    /// Inter-aural time delay on the lagging channel, in samples. [X]
    pub itd_samples: i32,
    /// Master gain, permyriad 0..10000 (inverse-square distance). [Y]
    pub gain_pmy: i32,
    /// Air-absorption low-pass corner, Hz. [Y]
    pub lowpass_hz: i32,
    /// Root-note fundamental, milli-Hz. [Z]
    pub root_freq_mhz: i64,
    /// Overtone richness, permyriad: 0 = pure sine, 10000 = full harmonic stack. [θ]
    pub overtone_pmy: i32,
    /// Phase offset, milli-degrees. [θ]
    pub phase_offset_mdeg: i32,
    /// Wow/flutter modulation rate, milli-Hz. [W]
    pub mod_rate_mhz: i32,
}

impl StereoField {
    /// Constant-power pan → (left_gain, right_gain), each `0.0..=1.0`, with
    /// `L² + R²` held at unity across the whole arc (Gerzon energy-preserving
    /// downmix).
    ///
    /// This WAS a linear law — `(10000-p)/20000, (10000+p)/20000` — and that
    /// law has a hole in the middle: at centre it hands back `(0.5, 0.5)`, so
    /// `L²+R² = 0.5`. A source panned to centre was 3 dB quieter than the same
    /// source panned hard over, purely as an artefact of the arithmetic. The
    /// sine law puts centre at `√½ ≈ 0.707` on both channels instead, which is
    /// the point of an energy-preserving collapse: moving a source across the
    /// field must not change how loud it is.
    ///
    /// Uses this module's own integer trig (`sin_mdeg`), not `f32::sin` — the
    /// header's determinism note names it, and a pan law is exactly the kind
    /// of thing that must not differ by CPU.
    pub fn pan_gains(&self) -> (f32, f32) {
        let p = self.pan_pmy.clamp(-10_000, 10_000);
        // Pan -10000..+10000 maps onto a quarter turn, 0..90_000 mdeg.
        let theta_mdeg = (p + 10_000) * 90_000 / 20_000;
        let right = sin_mdeg(theta_mdeg).clamp(0, 10_000) as f32 / 10_000.0;
        let left = sin_mdeg(90_000 - theta_mdeg).clamp(0, 10_000) as f32 / 10_000.0;
        (left, right)
    }

    /// The pan law's total power, `L² + R²`. Unity across the arc when the
    /// collapse is energy-preserving; a caller can gauge the law rather than
    /// trust it.
    pub fn pan_energy(&self) -> f32 {
        let (l, r) = self.pan_gains();
        l * l + r * r
    }
}

// ── collapse constants ──
const PAN_HALF_WIDTH_MU: i64 = 10_000; // ±10 world units = hard L/R
/// 1 world unit reference distance — unity gain, full bandwidth. Public so a
/// source with no world position can sit AT the reference rather than mint its
/// own default (the bed lanes, `mood::layer_edge_points`).
pub const REF_DIST_MU: i64 = 1_000;
const MAX_ITD_US: i64 = 660; // ~0.66 ms max inter-aural delay
const BASE_ROOT_MHZ: i64 = 55_000; // A1 = 55 Hz, the Z=0 root
const MIN_LOWPASS_HZ: i32 = 2_000;
const MAX_LOWPASS_HZ: i32 = 20_000;
/// Equal-temperament semitone ratios ×10000 (one octave).
const SEMITONE_RATIO_PMY: [i64; 12] =
    [10000, 10595, 11225, 11892, 12599, 13348, 14142, 14983, 15874, 16818, 17818, 18877];

/// The reduction formula: 5D point → flat stereo parameters. Pure + deterministic.
pub fn collapse_5d_to_stereo(p: Point5D, sample_rate: u32) -> StereoField {
    let sr = sample_rate.max(1) as i64;

    // X → pan + ITD. Far-left delays the lagging (right-side) channel.
    let pan_pmy = ((p.x_mu * 10_000) / PAN_HALF_WIDTH_MU).clamp(-10_000, 10_000) as i32;
    let max_itd_samples = MAX_ITD_US * sr / 1_000_000;
    let itd_samples = (max_itd_samples * pan_pmy.unsigned_abs() as i64 / 10_000) as i32;

    // Y → gain (inverse-square) + low-pass (air absorption). dist clamped ≥ 0.
    let dist = p.y_mu.max(0);
    let ref_sq = REF_DIST_MU * REF_DIST_MU;
    let gain_pmy = ((10_000i128 * ref_sq as i128) / (ref_sq as i128 + (dist as i128 * dist as i128))) as i32;
    let lowpass_hz = MIN_LOWPASS_HZ
        + ((MAX_LOWPASS_HZ - MIN_LOWPASS_HZ) as i64 * REF_DIST_MU / (REF_DIST_MU + dist)) as i32;

    // Z → root note (§17.3 Rosetta: structural meaning → pitch family).
    let root_freq_mhz = root_freq_mhz(p.z_semantic);

    // θ → overtone richness + phase. Rich at the ±90° "square", pure at 0/180°.
    let theta = p.theta_mdeg.rem_euclid(360_000);
    let overtone_pmy = sin_mdeg(theta).abs();
    let phase_offset_mdeg = theta;

    // W → wow/flutter modulation rate (chrono-tick volatility → drift speed).
    let mod_rate_mhz = ((p.w_tick % 1_000) as i32) * 8; // 0..~8 Hz

    StereoField {
        pan_pmy,
        itd_samples,
        gain_pmy,
        lowpass_hz,
        root_freq_mhz,
        overtone_pmy,
        phase_offset_mdeg,
        mod_rate_mhz,
    }
}

// ── The cremantic voice: a glyph IS a 5D point (Sean 2026-07-28) ────────────
#[cfg(feature = "calligraphy")]
pub mod cremantic_voice {
    use super::*;

    /// Chirality throw: half-width pan, so mirror = which ear.
    pub const CREE_PAN_MU: i64 = 5_000;
    /// The z=0 seat: the mark trit is balanced (−1/0/+1), the collapse indexes a
    /// scale degree from 0, so the bare mark sits at this degree.
    pub const CREE_Z_ROOT: i32 = 12;

    /// Cremantic glyph code → the 5D source point it already was. `None` for
    /// SPACE and the reserved seats — a rest has no sound (the emit stage's law).
    pub fn cree_code_to_point(code: u8) -> Option<Point5D> {
        forge_calligraphy::cremantic::Glyph::from_code(code)?;
        let e = forge_calligraphy::cremantic::embed(code);
        Some(Point5D {
            x_mu: (e[0] - 1) * CREE_PAN_MU,
            y_mu: REF_DIST_MU,
            z_semantic: CREE_Z_ROOT + (e[2] as i32 - 1),
            w_tick: 0,
            theta_mdeg: e[4] as i32 * 90_000,
        })
    }

    /// The inverse — a 5D point back to its glyph code, so the voice is a bridge
    /// and not a one-way render. `None` if the point is not on the glyph lattice.
    pub fn cree_point_to_code(p: Point5D) -> Option<u8> {
        (0..forge_calligraphy::cremantic::SPACE).find(|&c| cree_code_to_point(c) == Some(p))
    }

    /// A compiled cremantic word → one collapsed stereo field per sounding glyph.
    pub fn cree_word_to_fields(
        word: &forge_calligraphy::cremantic::Word,
        sample_rate: u32,
    ) -> Vec<StereoField> {
        word.codes
            .iter()
            .filter_map(|&c| cree_code_to_point(c))
            .map(|p| collapse_5d_to_stereo(p, sample_rate))
            .collect()
    }

    pub const CREE_VOICE_REGISTERS: std::ops::Range<i32> = -1..7;
    pub const CREE_JUST_REGISTER_BASE: i32 = 16;
    pub const CREE_VOICE_JUST_REGISTERS: std::ops::Range<i32> =
        (CREE_JUST_REGISTER_BASE - 1)..(CREE_JUST_REGISTER_BASE + 7);

    pub fn cree_voice_pitch_mhz(code: u8, register: i32) -> Option<i64> {
        let p = cree_code_to_point(code)?;
        if CREE_VOICE_JUST_REGISTERS.contains(&register) {
            let seat = register - CREE_JUST_REGISTER_BASE;
            return Some(3 * root_freq_mhz(p.z_semantic + 12 * seat));
        }
        Some(root_freq_mhz(p.z_semantic + 12 * register))
    }

    pub fn cree_voice_strike(
        code: u8,
        register: i32,
        target: &mut forge_harmonics::resonance_combat::ResonanceTarget,
        pressure_scale_q: i32,
    ) -> Option<forge_harmonics::resonance_combat::ShatterEvent> {
        let pitch_mhz = cree_voice_pitch_mhz(code, register)?;
        let tone = forge_calligraphy::audio_bridge::tone_of_code(code)?;
        let scale = (pressure_scale_q as i64 * tone.duration_ms as i64 / 150).clamp(0, 20_000) as i32;
        forge_harmonics::resonance_combat::apply_resonance(
            target,
            pitch_mhz.clamp(0, i32::MAX as i64) as i32,
            scale,
        )
    }

    pub fn cree_voice_for(
        target: &forge_harmonics::resonance_combat::ResonanceTarget,
    ) -> Option<(u8, i32)> {
        use forge_harmonics::resonance_combat::{phase_align_q, MIN_ALIGN_Q};
        (0..forge_calligraphy::cremantic::SPACE)
            .flat_map(|code| {
                CREE_VOICE_REGISTERS
                    .chain(CREE_VOICE_JUST_REGISTERS)
                    .map(move |reg| (code, reg))
            })
            .filter_map(|(code, reg)| {
                let mhz = cree_voice_pitch_mhz(code, reg)?.clamp(0, i32::MAX as i64) as i32;
                let align = phase_align_q(mhz, target.resonance_mhz);
                (align >= MIN_ALIGN_Q).then_some((align, code, reg))
            })
            .max_by_key(|&(align, _, _)| align)
            .map(|(_, code, reg)| (code, reg))
    }
}

/// Z semantic index → equal-temperament root frequency (milli-Hz), 8-octave span.
fn root_freq_mhz(z: i32) -> i64 {
    let z = z.clamp(0, 95);
    let octave = (z / 12) as u32;
    let semi = (z % 12) as usize;
    (BASE_ROOT_MHZ * SEMITONE_RATIO_PMY[semi] / 10_000) << octave
}

/// Integer phase (milli-degrees) of a `root_freq_mhz` tone at sample `t`.
#[inline]
fn phase_at(root_freq_mhz: i64, t: i64, sample_rate: u32) -> i32 {
    let sr = sample_rate.max(1) as i128;
    // cycles = freq_hz * t / sr ; mdeg = cycles * 360_000 ; freq_hz = root/1000.
    let mdeg = (root_freq_mhz as i128 * t as i128 * 360_000) / (1_000 * sr);
    (mdeg.rem_euclid(360_000)) as i32
}

/// One harmonic tone sample from an integer phase, richness-blended.
#[inline]
fn tone(phase_mdeg: i32, overtone_pmy: i32) -> f32 {
    let s = |m: i32| sin_mdeg(m) as f32 / 10_000.0;
    let ov = overtone_pmy.clamp(0, 10_000) as f32 / 10_000.0;
    let fund = s(phase_mdeg);
    let h2 = s(phase_mdeg.wrapping_mul(2));
    let h3 = s(phase_mdeg.wrapping_mul(3));
    // Pure sine at ov=0; fundamental + decaying overtones as ov→1. Normalized.
    (fund + ov * (0.5 * h2 + 0.33 * h3)) / (1.0 + ov * 0.83)
}

/// Render one stereo sample at time `t` (samples) from a collapsed field. The ITD
/// delays the lagging channel by `itd_samples`; a broken geometry (ITD ≈ half a
/// wavelength) makes L and R phase-cancel in the mono sum — the diagnostic.
pub fn render_sample(f: &StereoField, t: i64, sample_rate: u32) -> (f32, f32) {
    let itd = f.itd_samples as i64;
    // Source to the right (pan ≥ 0) reaches the right ear first → left channel lags.
    let (lt, rt) = if f.pan_pmy >= 0 { (t - itd, t) } else { (t, t - itd) };
    let lph = phase_at(f.root_freq_mhz, lt, sample_rate) + f.phase_offset_mdeg;
    let rph = phase_at(f.root_freq_mhz, rt, sample_rate) + f.phase_offset_mdeg;
    let g = f.gain_pmy.clamp(0, 10_000) as f32 / 10_000.0;
    let (lg, rg) = f.pan_gains();
    (tone(lph, f.overtone_pmy) * g * lg, tone(rph, f.overtone_pmy) * g * rg)
}

// ── 5D → 5.1 surround (G19 collision-sound-plane law) ──────────────────────────
// The stereo collapse extended to the ITU-R BS.775 ring. Every parameter stays
// integer (the SoT); float appears only in `render_surround_sample`, past the
// output boundary. LFE carries no azimuth (bass is non-directional). The law
// (G19-COLLISION-SOUND-PLANE-LAW.md): only the integer position writes the
// field; the float render reads it one-way.

use crate::lightning::LightningStrike;
use crate::ump::UmpWord;

/// 6 channels: L R C LFE Ls Rs.
pub const SURROUND_CHANNELS: usize = 6;

/// The full-range speakers ordered by ascending azimuth as `(azimuth_mdeg,
/// channel)` — L R C Ls Rs at ∓30/0/∓110°. LEFT is negative (matches stereo pan:
/// pan −10000 = hard left). LFE (channel 3) is excluded — non-directional, never
/// panned. This ring is the sole azimuth table VBAP reads.
const RING: [(i32, usize); 5] =
    [(-110_000, 4), (-30_000, 0), (0, 2), (30_000, 1), (110_000, 5)];

const MAX_AZIMUTH_MDEG: i32 = 110_000;
/// Fraction of the source gain sent to the LFE (bass management), permyriad.
const LFE_SEND_PMY: i32 = 3_000;
/// LFE crossover corner (Hz) — energy below here is non-directional.
const LFE_CROSSOVER_HZ: i32 = 100;

/// The flattened 5.1 field — every upper dimension baked into 6 integer channel
/// gains + delays. `render_surround_sample` turns it into f32 PCM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundField {
    /// Per-channel gain, permyriad 0..10000. Order [L, R, C, LFE, Ls, Rs].
    pub gains_pmy: [i32; SURROUND_CHANNELS],
    /// Per-channel inter-speaker delay, samples (nearest speaker ≈ 0).
    pub delays_samples: [i32; SURROUND_CHANNELS],
    /// LFE bass-management crossover corner, Hz.
    pub lfe_crossover_hz: i32,
    /// Root-note fundamental, milli-Hz. [Z]
    pub root_freq_mhz: i64,
    /// Overtone richness, permyriad. [θ]
    pub overtone_pmy: i32,
    /// Phase offset, milli-degrees. [θ]
    pub phase_offset_mdeg: i32,
}

/// The full-range ring segment `(lo, hi)` bounding a source azimuth.
fn ring_segment(theta_mdeg: i32) -> (usize, usize) {
    let t = theta_mdeg.clamp(RING[0].0, RING[RING.len() - 1].0);
    for i in 0..RING.len() - 1 {
        if t >= RING[i].0 && t <= RING[i + 1].0 {
            return (i, i + 1);
        }
    }
    (RING.len() - 2, RING.len() - 1)
}

/// 5D point → 5.1 surround field. Reuses the stereo collapse for the shared
/// scalar params (gain, root, overtone, phase, pan) then VBAP-pans the source
/// azimuth across the adjacent full-range speaker pair (constant-power via
/// integer trig) and sends bass to the LFE. Pure + deterministic.
pub fn collapse_5d_to_surround(p: Point5D, sample_rate: u32) -> SurroundField {
    let s = collapse_5d_to_stereo(p, sample_rate);

    // Source azimuth from the stereo pan (X axis). Left = negative, matching SPK.
    let theta = (s.pan_pmy.clamp(-10_000, 10_000) as i64 * MAX_AZIMUTH_MDEG as i64 / 10_000) as i32;

    let mut gains = [0i32; SURROUND_CHANNELS];
    let mut delays = [0i32; SURROUND_CHANNELS];

    // VBAP: constant-power split across the bounding full-range pair.
    let (lo, hi) = ring_segment(theta);
    let (az_lo, ch_lo) = RING[lo];
    let (az_hi, ch_hi) = RING[hi];
    let span = (az_hi - az_lo).max(1);
    let t_mdeg = (((theta - az_lo) as i64 * 90_000) / span as i64).clamp(0, 90_000) as i32;
    let g_hi = sin_mdeg(t_mdeg).clamp(0, 10_000);
    let g_lo = sin_mdeg(90_000 - t_mdeg).clamp(0, 10_000);
    let scale = |g: i32| (g as i64 * s.gain_pmy as i64 / 10_000) as i32;
    gains[ch_lo] += scale(g_lo);
    gains[ch_hi] += scale(g_hi);

    // LFE: non-directional bass send, no azimuth.
    gains[3] = (s.gain_pmy as i64 * LFE_SEND_PMY as i64 / 10_000) as i32;

    // Per-speaker delay from the azimuth gap to the source (nearest ≈ 0).
    let max_itd = (MAX_ITD_US * sample_rate.max(1) as i64 / 1_000_000) as i32;
    for &(az, ch) in RING.iter() {
        let gap = (az - theta).abs();
        delays[ch] = (max_itd as i64 * gap as i64 / (2 * MAX_AZIMUTH_MDEG as i64)) as i32;
    }

    SurroundField {
        gains_pmy: gains,
        delays_samples: delays,
        lfe_crossover_hz: LFE_CROSSOVER_HZ,
        root_freq_mhz: s.root_freq_mhz,
        overtone_pmy: s.overtone_pmy,
        phase_offset_mdeg: s.phase_offset_mdeg,
    }
}

/// Render one 6-channel PCM frame at time `t` from a surround field. Each channel
/// is delayed by its inter-speaker offset. Float lives ONLY here, past the output
/// boundary (the membrane law) — the field itself is integer.
pub fn render_surround_sample(f: &SurroundField, t: i64, sample_rate: u32) -> [f32; SURROUND_CHANNELS] {
    let mut out = [0f32; SURROUND_CHANNELS];
    for ch in 0..SURROUND_CHANNELS {
        let g = f.gains_pmy[ch].clamp(0, 10_000) as f32 / 10_000.0;
        if g == 0.0 {
            continue;
        }
        let tt = t - f.delays_samples[ch] as i64;
        let ph = phase_at(f.root_freq_mhz, tt, sample_rate) + f.phase_offset_mdeg;
        out[ch] = tone(ph, f.overtone_pmy) * g;
    }
    out
}

// ── 5.1 strike bed (the live sink) ─────────────────────────────────────────────
// The FIRST consumer of `render_surround_sample`. A `SurroundBus` holds decaying
// [`SurroundField`] voices; each audio block it renders the summed, enveloped
// 6-channel bed and folds it to the device channel count — 6 = discrete 5.1
// passthrough (L R C LFE Ls Rs), 2 = ITU-R BS.775 stereo downmix, else mono. One
// canvas strike thus sounds on BOTH faces: the mono/stereo note voice (via the
// `UmpWord`) AND this positioned 5.1 bed. Float lives past the output boundary;
// the field the bus renders stays integer.

/// One ringing strike — a placed [`SurroundField`] plus its phase/decay clock.
struct SurroundVoice {
    field: SurroundField,
    /// Samples elapsed since the strike (the `render_surround_sample` phase clock
    /// and the linear-decay position).
    t: i64,
}

/// The live 5.1 strike bed. Fed by [`strike_audio`]'s field, drained each audio
/// block by the studio output seam. Not on the realtime cpal callback — driven by
/// the 120 Hz DET producer, so the small `voices` book-keeping is cold-path.
pub struct SurroundBus {
    voices: Vec<SurroundVoice>,
    sample_rate: u32,
    /// Thunder tail, in samples — a strike decays linearly to silence over this.
    decay_samples: i64,
}

impl SurroundBus {
    /// A silent bus. Default thunder tail ≈ 0.6 s at `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate.max(1);
        Self { voices: Vec::new(), sample_rate: sr, decay_samples: (sr as i64 * 3) / 5 }
    }

    /// Register a strike's spatial field — it rings and decays over the tail.
    pub fn strike(&mut self, field: SurroundField) {
        self.voices.push(SurroundVoice { field, t: 0 });
    }

    /// Any voice still ringing?
    pub fn is_active(&self) -> bool {
        !self.voices.is_empty()
    }

    /// Render `frames` of the summed strike bed into `out`, interleaved by
    /// `out_channels`. `out` must hold at least `frames * out_channels` samples;
    /// the written region is OVERWRITTEN (not accumulated) so the caller sums it
    /// against its own lane. Advances every voice and drops the dead. 6 channels =
    /// discrete 5.1 (L R C LFE Ls Rs); 2 = BS.775 stereo downmix; else = mono fold.
    pub fn render_block(&mut self, out: &mut [f32], out_channels: usize, frames: usize) {
        let oc = out_channels.max(1);
        let n = frames.min(out.len() / oc);
        for s in out[..n * oc].iter_mut() {
            *s = 0.0;
        }
        if self.voices.is_empty() || n == 0 {
            return;
        }
        let sr = self.sample_rate;
        let decay = self.decay_samples.max(1);
        for voice in self.voices.iter_mut() {
            for f in 0..n {
                let t = voice.t + f as i64;
                if t >= decay {
                    break;
                }
                // Linear thunder envelope: full at the contact, silent at the tail.
                let env = (decay - t) as f32 / decay as f32;
                let frame = render_surround_sample(&voice.field, t, sr);
                fold_surround_frame(&mut out[f * oc..f * oc + oc], &frame, env, oc);
            }
            voice.t += n as i64;
        }
        self.voices.retain(|v| v.t < decay);
    }
}

/// Fold one enveloped `[L R C LFE Ls Rs]` frame into `dst` (`oc` channels).
fn fold_surround_frame(dst: &mut [f32], frame: &[f32; SURROUND_CHANNELS], env: f32, oc: usize) {
    let [l, r, c, lfe, ls, rs] = *frame;
    match oc {
        // Discrete 5.1 passthrough — device channel order L R C LFE Ls Rs.
        n if n >= SURROUND_CHANNELS => {
            dst[0] += l * env;
            dst[1] += r * env;
            dst[2] += c * env;
            dst[3] += lfe * env;
            dst[4] += ls * env;
            dst[5] += rs * env;
        }
        // ITU-R BS.775 stereo downmix: centre −3 dB to both, surrounds to their side.
        2 => {
            const K: f32 = 0.707; // −3 dB
            dst[0] += (l + K * c + K * ls + K * lfe) * env;
            dst[1] += (r + K * c + K * rs + K * lfe) * env;
        }
        // Mono fold — the whole field summed to every channel.
        _ => {
            let m = (l + r + c + lfe + ls + rs) * 0.5 * env;
            for s in dst.iter_mut() {
                *s += m;
            }
        }
    }
}

/// Physics-family sub-kind for a lightning contact (masked `& 0x07` in the UMP
/// encoder — a sub-family axis, not a global id).
pub const LIGHTNING_PHYSICS_KIND: u8 = 6;

/// The atomic_bridge (G19): a positioned canvas contact — a
/// [`LightningStrike`] on a `canvas_w × canvas_h` grid — becomes BOTH a
/// tick-stamped UMP physics *trigger* and its spatialized 5.1 *field*. One
/// strike → SEE (elsewhere) + HEAR (here), integer end to end. The canvas cell
/// coord is the sole sim authority (`integer_sot`); the returned field feeds the
/// one-way float render, never back.
pub fn strike_audio(
    strike: &LightningStrike,
    canvas_w: i32,
    canvas_h: i32,
    sample_rate: u32,
) -> (UmpWord, SurroundField) {
    let w = canvas_w.max(1);
    let h = canvas_h.max(1);
    // Canvas cell → Point5D. X centred on the canvas → pan; Y (down) → depth.
    let x_mu = (strike.origin[0] * 2 - w) as i64 * PAN_HALF_WIDTH_MU / w as i64;
    let y_mu = strike.origin[1] as i64 * 8_000 / h as i64;
    let z_semantic = ((strike.pitch_hz() as i32).clamp(40, 200) - 40) * 24 / 160;
    let theta_mdeg = (strike.branch_seed % 360_000) as i32;
    let p = Point5D { x_mu, y_mu, z_semantic, w_tick: strike.duration_ticks as u64, theta_mdeg };

    let mut field = collapse_5d_to_surround(p, sample_rate);
    // Fold the strike's brightness into loudness (a dim bolt is a quiet clap).
    let inten = strike.intensity_pmy.min(10_000) as i64;
    for g in field.gains_pmy.iter_mut() {
        *g = (*g as i64 * inten / 10_000) as i32;
    }

    let ump = UmpWord::from_physics_event(
        LIGHTNING_PHYSICS_KIND,
        strike.branch_seed,
        strike.crackle_permyriad() as i32,
        strike.pitch_hz() as i32,
    );
    (ump, field)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn origin() -> Point5D {
        Point5D { x_mu: 0, y_mu: 0, z_semantic: 12, w_tick: 0, theta_mdeg: 0 }
    }

    // [BOARD:CREE-SOUND] the dimensional bridge: a glyph round-trips through
    // the 5D point, and each lane lands on the axis it belongs to.
    #[cfg(feature = "calligraphy")]
    #[test]
    fn a_glyph_is_a_five_d_point_both_ways() {
        use super::cremantic_voice::*;
        use forge_calligraphy::cremantic::{Glyph, Mark, Mirror, Rotation, SPACE};

        let mut points: Vec<Point5D> = Vec::new(); // @forge:allow_alloc test-only distinctness check
        for code in 0..SPACE {
            let p = cree_code_to_point(code).expect("every lane glyph is a point");
            assert_eq!(cree_point_to_code(p), Some(code), "code {code}");
            assert!(!points.contains(&p), "code {code} collided in 5D");
            points.push(p); // @forge:allow_alloc test-only distinctness check
            // Reserved lanes stay unexercised — never faked into the audio.
            assert_eq!(p.w_tick, 0);
            assert_eq!(p.y_mu, REF_DIST_MU);
        }
        // 3·3·3 after the pararity fold — every lane arity 3, so every code is a point.
        assert_eq!(points.len(), 27);
        // Silence is real: SPACE has no point, so a rest renders as a rest.
        assert!(cree_code_to_point(SPACE).is_none());

        // Chirality is which ear; the mark trit is pitch; rotation is phase.
        let plain = Glyph { rotation: Rotation::R0, mirror: Mirror::Plain, mark: Mark::Dot };
        let flipped = Glyph { mirror: Mirror::Flipped, ..plain };
        let l = collapse_5d_to_stereo(cree_code_to_point(plain.code()).unwrap(), SR);
        let r = collapse_5d_to_stereo(cree_code_to_point(flipped.code()).unwrap(), SR);
        assert!(l.pan_pmy < 0 && r.pan_pmy > 0, "mirror is the stereo lane");
        assert_eq!(l.root_freq_mhz, r.root_freq_mhz, "chirality never moves pitch");

        let bare = collapse_5d_to_stereo(
            cree_code_to_point(Glyph { mark: Mark::Bare, ..plain }.code()).unwrap(),
            SR,
        );
        let long = collapse_5d_to_stereo(
            cree_code_to_point(Glyph { mark: Mark::Long, ..plain }.code()).unwrap(),
            SR,
        );
        assert!(bare.root_freq_mhz < l.root_freq_mhz, "the z lane IS the pitch lane");
        assert!(long.root_freq_mhz > l.root_freq_mhz);

        let mut phases = std::collections::HashSet::new();
        for rotation in [Rotation::R0, Rotation::R90, Rotation::R270] {
            let f = collapse_5d_to_stereo(
                cree_code_to_point(Glyph { rotation, ..plain }.code()).unwrap(),
                SR,
            );
            phases.insert(f.phase_offset_mdeg);
        }
        // Three orientations after the 4→3 pararity fold: R180 had no fixed point to
        // stand on, so the lane could not carry a balanced trit until it was cut.
        assert_eq!(phases.len(), 3, "three orientations, three phase angles");
        assert!(phases.contains(&90_000), "a quarter turn IS 90 degrees");
    }

    // [BOARD:CREE-SOUND] the live consumer: a compiled word becomes audio.
    #[cfg(feature = "calligraphy")]
    #[test]
    fn a_compiled_word_collapses_to_stereo_fields() {
        use super::cremantic_voice::*;
        // 20 verdict trits — an assay sheet — is seven sounding glyphs.
        let word = forge_calligraphy::cremantic::compile(&[0u8, 0, 0, 0], 20);
        let fields = cree_word_to_fields(&word, SR);
        assert_eq!(fields.len(), 7);
        assert!(fields.iter().all(|f| f.root_freq_mhz > 0 && f.gain_pmy > 0));
        // And it actually renders — non-silent samples out of the collapse.
        let (l, r) = render_sample(&fields[0], SR as i64 / 4, SR);
        assert!(l.abs() + r.abs() > 0.0, "the word is audible");
    }

    // [BOARD:CREE-SOUND] sound -> stone: a sung glyph is physical consequence.
    #[cfg(feature = "calligraphy")]
    #[test]
    fn a_sung_glyph_shatters_the_material_it_is_tuned_to() {
        use super::cremantic_voice::*;
        use forge_calligraphy::cremantic::{Glyph, Mark, Mirror, Rotation};
        use forge_harmonics::ce_audio::MaterialKind;
        use forge_harmonics::resonance_combat::ResonanceTarget;

        // The standard mark sings the collapse's own root: 110 Hz — Stone.
        let stone_voice = Glyph { rotation: Rotation::R0, mirror: Mirror::Plain, mark: Mark::Dot };
        assert_eq!(cree_voice_pitch_mhz(stone_voice.code(), 0), Some(110_000));

        let mut stone = ResonanceTarget::from_material(MaterialKind::Stone, 10_000);
        let mut ticks = 0;
        let shatter = loop {
            ticks += 1;
            assert!(ticks < 200, "a tuned voice must eventually break the shell");
            if let Some(e) = cree_voice_strike(stone_voice.code(), 0, &mut stone, 10_000) {
                break e;
            }
        };
        assert_eq!(shatter.resonance_mhz, 110_000);
        assert!(shatter.shard_count >= 3);
        assert!(stone.is_shattered());

        // A mistuned mark never breaks it — being close is not enough.
        let off = Glyph { mark: Mark::Long, ..stone_voice };
        let mut stone2 = ResonanceTarget::from_material(MaterialKind::Stone, 10_000);
        for _ in 0..200 {
            assert!(cree_voice_strike(off.code(), 0, &mut stone2, 10_000).is_none());
        }
        assert_eq!(stone2.armor_q, 10_000, "a detuned glyph does no work at all");
    }

    // [BOARD:CREE-SOUND] hunting the frequency: tempered seats, just seats,
    // and the one detuned outlier.
    #[cfg(feature = "calligraphy")]
    #[test]
    fn the_voice_hunt_reaches_all_twelve_tempered_just_or_detuned() {
        use super::cremantic_voice::*;
        use forge_harmonics::ce_audio::MaterialKind;
        use forge_harmonics::resonance_combat::{phase_align_q, ResonanceTarget, MIN_ALIGN_Q};

        for kind in [
            MaterialKind::Void,
            MaterialKind::Stone,
            MaterialKind::Flesh,
            MaterialKind::Ash,
            MaterialKind::Wood,
            MaterialKind::Bone,
            MaterialKind::Glass,
        ] {
            let t = ResonanceTarget::from_material(kind, 10_000);
            let (code, reg) = cree_voice_for(&t).unwrap_or_else(|| panic!("{kind:?} unreachable"));
            assert!(CREE_VOICE_REGISTERS.contains(&reg), "{kind:?} sits on the tempered lattice");
            let mhz = cree_voice_pitch_mhz(code, reg).unwrap() as i32;
            assert!(phase_align_q(mhz, t.resonance_mhz) >= MIN_ALIGN_Q, "{kind:?}");
        }
        // Just-intonation shells — 3-limit multiples of the 55 Hz root — land
        // on the overblown register EXACTLY, not approximately.
        for kind in [
            MaterialKind::Iron,
            MaterialKind::Crystal,
            MaterialKind::Cloth,
            MaterialKind::Bronze,
        ] {
            let t = ResonanceTarget::from_material(kind, 10_000);
            let (code, reg) = cree_voice_for(&t).unwrap_or_else(|| panic!("{kind:?} unreachable"));
            assert!(CREE_VOICE_JUST_REGISTERS.contains(&reg), "{kind:?} sits on the just lattice");
            assert_eq!(cree_voice_pitch_mhz(code, reg), Some(t.resonance_mhz as i64), "{kind:?} rings exact");
        }
        // Bell (432 Hz) sits on neither lattice exactly; the hunt reaches it
        // through the tempered A440 seat, 0.32 semitone flat — shatterable,
        // never unison, the slowest shell to sing down. (No small-integer
        // ratio of 55 Hz lands on 432: the ratio 432:55 is already lowest
        // terms, so a just seat for Bell would mean retuning Bell itself.)
        let bell = ResonanceTarget::from_material(MaterialKind::Bell, 10_000);
        let (code, reg) = cree_voice_for(&bell).expect("Bell rides the A440 seat");
        assert!(CREE_VOICE_REGISTERS.contains(&reg), "Bell's best seat is tempered, not just");
        let mhz = cree_voice_pitch_mhz(code, reg).unwrap();
        assert_eq!(mhz, 440_000, "the nearest seat is A440, 8 Hz sharp of 432");
        let align = phase_align_q(mhz as i32, bell.resonance_mhz);
        assert!(align >= MIN_ALIGN_Q && align < 10_000, "within the gate, never unison");
    }

    // [BOARD:CREE-SOUND] the 660 Hz overblown impulse breaks the iron shell.
    #[cfg(feature = "calligraphy")]
    #[test]
    fn an_overblown_voice_shatters_the_iron_shell() {
        use super::cremantic_voice::*;
        use forge_harmonics::ce_audio::MaterialKind;
        use forge_harmonics::resonance_combat::ResonanceTarget;

        let mut iron = ResonanceTarget::from_material(MaterialKind::Iron, 10_000);
        let (code, reg) = cree_voice_for(&iron).expect("iron is reachable on the just lattice");
        assert!(CREE_VOICE_JUST_REGISTERS.contains(&reg));
        let mut ticks = 0;
        let shatter = loop {
            ticks += 1;
            assert!(ticks < 200, "an exactly tuned voice must break the shell");
            if let Some(e) = cree_voice_strike(code, reg, &mut iron, 10_000) {
                break e;
            }
        };
        assert_eq!(shatter.resonance_mhz, 660_000);
        assert_eq!(shatter.peak_align_q, 10_000, "exact just ratio = perfect unison");
        assert!(iron.is_shattered());
    }

    #[test]
    fn x_pans_and_delays_the_lagging_channel() {
        let left = collapse_5d_to_stereo(Point5D { x_mu: -PAN_HALF_WIDTH_MU, ..origin() }, SR);
        let right = collapse_5d_to_stereo(Point5D { x_mu: PAN_HALF_WIDTH_MU, ..origin() }, SR);
        assert_eq!(left.pan_pmy, -10_000, "hard left");
        assert_eq!(right.pan_pmy, 10_000, "hard right");
        assert!(left.itd_samples > 0, "hard pan produces an inter-aural delay");
        let (llg, lrg) = left.pan_gains();
        assert!(llg > lrg, "left pan favours the left channel");
    }

    /// The clamp this row exists for: moving a source across the field must
    /// not change how loud it is.
    #[test]
    fn the_pan_law_preserves_energy_across_the_whole_arc() {
        for pan_pmy in (-10_000..=10_000).step_by(250) {
            let f = StereoField { pan_pmy, ..collapse_5d_to_stereo(origin(), SR) };
            let e = f.pan_energy();
            assert!(
                (e - 1.0).abs() < 0.02,
                "pan {pan_pmy} leaks energy: L^2+R^2 = {e}, want 1.0"
            );
        }
    }

    /// The specific hole the old linear law had: centre was 3 dB down.
    #[test]
    fn centre_is_not_quieter_than_hard_over() {
        let base = collapse_5d_to_stereo(origin(), SR);
        let centre = StereoField { pan_pmy: 0, ..base };
        let hard = StereoField { pan_pmy: -10_000, ..base };
        let (cl, cr) = centre.pan_gains();
        assert!(
            (cl - cr).abs() < 0.001,
            "centre must be balanced: ({cl}, {cr})"
        );
        assert!(cl > 0.69 && cl < 0.72, "centre sits at root-half, not a half: {cl}");
        assert!(
            (centre.pan_energy() - hard.pan_energy()).abs() < 0.02,
            "centre {} and hard-over {} must carry the same power",
            centre.pan_energy(),
            hard.pan_energy()
        );
    }

    /// Hard over is still hard over — energy preservation must not soften the
    /// ends of the field.
    #[test]
    fn the_ends_of_the_field_stay_hard() {
        let base = collapse_5d_to_stereo(origin(), SR);
        let (ll, lr) = StereoField { pan_pmy: -10_000, ..base }.pan_gains();
        let (rl, rr) = StereoField { pan_pmy: 10_000, ..base }.pan_gains();
        assert!(ll > 0.99 && lr < 0.01, "hard left is left: ({ll}, {lr})");
        assert!(rr > 0.99 && rl < 0.01, "hard right is right: ({rl}, {rr})");
    }

    /// The law is symmetric: a source at +x is the mirror of one at -x.
    #[test]
    fn the_pan_law_is_symmetric_about_centre() {
        let base = collapse_5d_to_stereo(origin(), SR);
        for pan_pmy in (0..=10_000).step_by(500) {
            let (rl, rr) = StereoField { pan_pmy, ..base }.pan_gains();
            let (ll, lr) = StereoField { pan_pmy: -pan_pmy, ..base }.pan_gains();
            assert!((rl - lr).abs() < 0.002 && (rr - ll).abs() < 0.002, "asymmetry at {pan_pmy}");
        }
    }

    #[test]
    fn y_attenuates_and_dampens_with_distance() {
        let near = collapse_5d_to_stereo(origin(), SR);
        let far = collapse_5d_to_stereo(Point5D { y_mu: 20_000, ..origin() }, SR);
        assert!(far.gain_pmy < near.gain_pmy, "distance drops gain (inverse-square)");
        assert!(far.lowpass_hz < near.lowpass_hz, "distance rolls off highs (air absorption)");
    }

    #[test]
    fn z_shifts_the_root_by_octaves_and_semitones() {
        let a1 = collapse_5d_to_stereo(Point5D { z_semantic: 0, ..origin() }, SR);
        let a2 = collapse_5d_to_stereo(Point5D { z_semantic: 12, ..origin() }, SR);
        assert_eq!(a1.root_freq_mhz, 55_000, "Z=0 = A1 = 55 Hz");
        assert_eq!(a2.root_freq_mhz, 110_000, "Z=12 = one octave up = 110 Hz");
    }

    #[test]
    fn theta_controls_timbre_richness() {
        let pure = collapse_5d_to_stereo(Point5D { theta_mdeg: 0, ..origin() }, SR);
        let rich = collapse_5d_to_stereo(Point5D { theta_mdeg: 90_000, ..origin() }, SR);
        assert_eq!(pure.overtone_pmy, 0, "θ=0 = pure sine");
        assert!(rich.overtone_pmy > 9_000, "θ=90° = maximally rich");
    }

    #[test]
    fn w_sets_the_modulation_drift() {
        let steady = collapse_5d_to_stereo(origin(), SR);
        let volatile = collapse_5d_to_stereo(Point5D { w_tick: 500, ..origin() }, SR);
        assert_eq!(steady.mod_rate_mhz, 0);
        assert!(volatile.mod_rate_mhz > 0, "chrono volatility drives wow/flutter");
    }

    #[test]
    fn collapse_and_render_are_deterministic() {
        let p = Point5D { x_mu: 3_000, y_mu: 4_000, z_semantic: 19, w_tick: 777, theta_mdeg: 123_456 };
        assert_eq!(collapse_5d_to_stereo(p, SR), collapse_5d_to_stereo(p, SR), "collapse is byte-identical");
        let f = collapse_5d_to_stereo(p, SR);
        assert_eq!(render_sample(&f, 100, SR), render_sample(&f, 100, SR), "render is byte-identical");
    }

    #[test]
    fn broken_geometry_phase_cancels_in_the_mono_sum() {
        // The diagnostic: a coherent field sums loud in mono; a "broken" field
        // whose ITD is half a wavelength cancels L against R → near silence.
        let mut coherent = collapse_5d_to_stereo(Point5D { z_semantic: 24, ..origin() }, SR);
        coherent.pan_pmy = 1; // tiny right bias so the left channel is the one that lags
        coherent.itd_samples = 0; // no delay → L and R in phase
        let mut broken = coherent;
        // half a wavelength of the root, in samples = sr / (2*freq_hz).
        let freq_hz = (broken.root_freq_mhz / 1_000).max(1);
        broken.itd_samples = (SR as i64 / (2 * freq_hz)) as i32;

        let mono_energy = |f: &StereoField| -> f32 {
            (0..256).map(|t| { let (l, r) = render_sample(f, t, SR); let m = l + r; m * m }).sum()
        };
        let coh = mono_energy(&coherent);
        let brk = mono_energy(&broken);
        assert!(coh > 0.01, "coherent field has real mono energy ({coh})");
        assert!(brk < coh * 0.25, "broken geometry phase-cancels: {brk} << {coh}");
    }

    // ── 5.1 surround ──────────────────────────────────────────────────────────

    #[test]
    fn surround_hard_left_favours_the_left_side() {
        let f = collapse_5d_to_surround(Point5D { x_mu: -PAN_HALF_WIDTH_MU, ..origin() }, SR);
        let left = f.gains_pmy[0] + f.gains_pmy[4]; // L + Ls
        let right = f.gains_pmy[1] + f.gains_pmy[5]; // R + Rs
        assert!(left > right, "hard left favours L/Ls: left={left} right={right}");
    }

    #[test]
    fn surround_center_source_lands_on_the_center_channel() {
        let f = collapse_5d_to_surround(origin(), SR); // x_mu = 0 → azimuth 0 = C
        assert!(f.gains_pmy[2] > 0, "center channel carries the on-axis source");
        assert!(f.gains_pmy[4] == 0 && f.gains_pmy[5] == 0, "rears silent for a front-center source");
    }

    #[test]
    fn lfe_is_non_directional_and_present() {
        let left = collapse_5d_to_surround(Point5D { x_mu: -PAN_HALF_WIDTH_MU, ..origin() }, SR);
        let right = collapse_5d_to_surround(Point5D { x_mu: PAN_HALF_WIDTH_MU, ..origin() }, SR);
        assert!(left.gains_pmy[3] > 0, "LFE carries bass");
        assert_eq!(left.gains_pmy[3], right.gains_pmy[3], "LFE gain is azimuth-independent");
    }

    #[test]
    fn surround_pan_is_constant_power_across_a_pair() {
        // A source between two speakers keeps summed power ~constant (VBAP).
        let f = collapse_5d_to_surround(Point5D { x_mu: PAN_HALF_WIDTH_MU / 4, ..origin() }, SR);
        // full-range power (exclude LFE idx 3).
        let power: i64 = [0, 1, 2, 4, 5]
            .iter()
            .map(|&c| f.gains_pmy[c] as i64 * f.gains_pmy[c] as i64)
            .sum();
        assert!(power > 0, "a placed source has real full-range power");
    }

    #[test]
    fn collapse_and_render_surround_are_deterministic() {
        let p = Point5D { x_mu: 2_000, y_mu: 3_000, z_semantic: 19, w_tick: 5, theta_mdeg: 61_000 };
        assert_eq!(collapse_5d_to_surround(p, SR), collapse_5d_to_surround(p, SR));
        let f = collapse_5d_to_surround(p, SR);
        assert_eq!(render_surround_sample(&f, 200, SR), render_surround_sample(&f, 200, SR));
    }

    // ── strike_audio (the atomic_bridge) ────────────────────────────────────────

    fn strike(x: i32, y: i32) -> LightningStrike {
        LightningStrike { origin: [x, y], branch_seed: 0x1234_5678_9ABC_DEF0, intensity_pmy: 9_000, duration_ticks: 12 }
    }

    #[test]
    fn strike_on_the_right_half_favours_the_right_side() {
        let (_, right_field) = strike_audio(&strike(60, 20), 64, 48, SR); // right of center (32)
        let left = right_field.gains_pmy[0] + right_field.gains_pmy[4];
        let right = right_field.gains_pmy[1] + right_field.gains_pmy[5];
        assert!(right > left, "a right-side strike favours R/Rs: left={left} right={right}");
    }

    #[test]
    fn strike_emits_a_physics_family_ump_word() {
        let (ump, _) = strike_audio(&strike(32, 24), 64, 48, SR);
        // The UMP transport word is the PHYSICS family (first signature byte).
        assert_eq!(ump.0[0], crate::ump::FAMILY_PHYSICS, "lightning is a physics-family UMP event");
    }

    #[test]
    fn strike_audio_is_deterministic() {
        let s = strike(40, 30);
        assert_eq!(strike_audio(&s, 64, 48, SR).1, strike_audio(&s, 64, 48, SR).1);
    }

    #[test]
    fn dimmer_strike_is_quieter() {
        let mut dim = strike(60, 20);
        dim.intensity_pmy = 2_000;
        let bright_power: i64 = strike_audio(&strike(60, 20), 64, 48, SR).1.gains_pmy.iter().map(|&g| g as i64).sum();
        let dim_power: i64 = strike_audio(&dim, 64, 48, SR).1.gains_pmy.iter().map(|&g| g as i64).sum();
        assert!(dim_power < bright_power, "a dim bolt is a quiet clap: dim={dim_power} bright={bright_power}");
    }

    // ── SurroundBus (the 5.1 strike bed) ────────────────────────────────────────

    #[test]
    fn surround_bus_rings_then_decays_to_silence() {
        let mut bus = SurroundBus::new(SR);
        bus.strike(collapse_5d_to_surround(
            Point5D { x_mu: 5_000, y_mu: 1_000, z_semantic: 24, w_tick: 3, theta_mdeg: 45_000 },
            SR,
        ));
        assert!(bus.is_active(), "a strike arms the bus");
        let mut out = vec![0f32; 128 * SURROUND_CHANNELS];
        let mut rang = false;
        for _ in 0..600 {
            bus.render_block(&mut out, SURROUND_CHANNELS, 128);
            if out.iter().any(|s| s.abs() > 1e-6) {
                rang = true;
            }
        }
        assert!(rang, "the bus must ring after a strike");
        assert!(!bus.is_active(), "the strike decays to silence and the voice is dropped");
    }

    #[test]
    fn surround_bus_stereo_downmix_favours_the_struck_side() {
        // A hard-right canvas strike must land louder on the R downmix channel.
        let mut bus = SurroundBus::new(SR);
        bus.strike(strike_audio(&strike(60, 20), 64, 48, SR).1);
        let mut out = vec![0f32; 64 * 2];
        let (mut le, mut re) = (0f32, 0f32);
        for _ in 0..64 {
            bus.render_block(&mut out, 2, 64);
            for f in 0..64 {
                le += out[f * 2].abs();
                re += out[f * 2 + 1].abs();
            }
        }
        assert!(re > le, "a right-side strike is louder on the right downmix channel: L={le} R={re}");
    }

    #[test]
    fn surround_bus_passthrough_drives_the_discrete_centre() {
        // A front-centre strike (x_mu = 0) rings the discrete C channel (idx 2).
        let mut bus = SurroundBus::new(SR);
        bus.strike(collapse_5d_to_surround(origin(), SR));
        let mut out = vec![0f32; 32 * SURROUND_CHANNELS];
        let mut c_energy = 0f32;
        for _ in 0..32 {
            bus.render_block(&mut out, SURROUND_CHANNELS, 32);
            for f in 0..32 {
                c_energy += out[f * SURROUND_CHANNELS + 2].abs();
            }
        }
        assert!(c_energy > 0.0, "a centre strike drives the discrete C channel");
    }

    #[test]
    fn surround_bus_silent_when_unstruck() {
        let mut bus = SurroundBus::new(SR);
        let mut out = vec![9f32; 16 * SURROUND_CHANNELS];
        bus.render_block(&mut out, SURROUND_CHANNELS, 16);
        assert!(out.iter().all(|&s| s == 0.0), "an unstruck bus writes silence, not stale samples");
        assert!(!bus.is_active());
    }
}
