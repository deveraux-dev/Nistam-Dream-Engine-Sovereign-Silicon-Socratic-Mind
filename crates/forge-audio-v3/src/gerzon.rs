//! Gerzon localisation vectors (Gerzon 1975/1992) — where a set of gains
//! points, and how confidently. Pure math over `(gain, direction)` pairs; the
//! band→azimuth mapping below it is authored, and says so.

/// A localisation vector and its magnitude.
///
/// `magnitude` is the directional confidence: `1.0` when every contributing
/// gain agrees on one direction, `0.0` when they cancel or there is no energy
/// at all. It is NOT a loudness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GerzonVector {
    /// The vector itself, in the caller's own coordinate frame.
    pub vector: [f32; 3],
    /// `|vector|`, in `0.0..=1.0` for normalised inputs.
    pub magnitude: f32,
}

impl GerzonVector {
    /// The no-energy answer: pointing nowhere, with no confidence.
    pub const ZERO: Self = Self { vector: [0.0; 3], magnitude: 0.0 };
}

fn normalise(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

fn finish(acc: [f32; 3], weight_sum: f32) -> GerzonVector {
    if weight_sum <= f32::EPSILON {
        return GerzonVector::ZERO;
    }
    let vector = [acc[0] / weight_sum, acc[1] / weight_sum, acc[2] / weight_sum];
    let magnitude =
        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    GerzonVector { vector, magnitude }
}

/// Gerzon ENERGY vector `rE = Σ(gᵢ² uᵢ) / Σ(gᵢ²)` — the high-frequency
/// localisation cue, where the ear localises by energy rather than phase.
///
/// Gains are squared, so a source pulling twice as loud pulls four times as
/// hard. Zero-length directions are skipped, not normalised into a NaN.
pub fn energy_vector(sources: &[(f32, [f32; 3])]) -> GerzonVector {
    let mut acc = [0.0f32; 3];
    let mut weight_sum = 0.0f32;
    for (gain, dir) in sources {
        let Some(u) = normalise(*dir) else { continue };
        let w = gain * gain;
        acc[0] += w * u[0];
        acc[1] += w * u[1];
        acc[2] += w * u[2];
        weight_sum += w;
    }
    finish(acc, weight_sum)
}

/// Gerzon VELOCITY vector `rV = Σ(gᵢ uᵢ) / Σ(gᵢ)` — the low-frequency cue,
/// where the ear localises by pressure gradient.
///
/// Linear in gain, and therefore signed: an out-of-phase source pulls the
/// vector back rather than adding to it, which is exactly the physical
/// behaviour `rE` cannot express.
pub fn velocity_vector(sources: &[(f32, [f32; 3])]) -> GerzonVector {
    let mut acc = [0.0f32; 3];
    let mut weight_sum = 0.0f32;
    for (gain, dir) in sources {
        let Some(u) = normalise(*dir) else { continue };
        acc[0] += gain * u[0];
        acc[1] += gain * u[1];
        acc[2] += gain * u[2];
        weight_sum += gain;
    }
    finish(acc, weight_sum)
}

/// `[AUTHORED]` azimuths for the three bands `bus::uniforms::spectrum_bands`
/// actually returns, in radians on the horizontal plane, `0` = straight ahead.
///
/// This mapping is a CHOICE, not physics — a spectrum band is not a direction.
/// It is authored on one perceptual ground: low frequencies are the least
/// localisable, so the low band sits dead centre and contributes magnitude
/// without pulling the vector off-axis, while mid and high open symmetrically
/// across the frontal arc. Change these and the visual lanes swing; they are
/// the one tunable in this file and they are named so a reader can find them.
pub const BAND_AZIMUTH_RAD: [f32; 3] = [0.0, -std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4];

fn azimuth_to_unit(rad: f32) -> [f32; 3] {
    [rad.sin(), 0.0, rad.cos()]
}

/// The three-band adapter: the real `(low, mid, high)` shape from
/// `bus::uniforms::spectrum_bands`, localised through [`energy_vector`].
///
/// Scoped against the live 3-tuple, NOT the 7-element array the original
/// aspire row imagined — that array does not exist and never did.
pub fn three_band_energy_vector(low: f32, mid: f32, high: f32) -> GerzonVector {
    let sources = [
        (low.max(0.0), azimuth_to_unit(BAND_AZIMUTH_RAD[0])),
        (mid.max(0.0), azimuth_to_unit(BAND_AZIMUTH_RAD[1])),
        (high.max(0.0), azimuth_to_unit(BAND_AZIMUTH_RAD[2])),
    ];
    energy_vector(&sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn no_sources_point_nowhere() {
        assert_eq!(energy_vector(&[]), GerzonVector::ZERO);
        assert_eq!(velocity_vector(&[]), GerzonVector::ZERO);
        assert_eq!(three_band_energy_vector(0.0, 0.0, 0.0), GerzonVector::ZERO);
    }

    /// One source is total agreement: the vector points at it with confidence 1.
    #[test]
    fn a_lone_source_localises_perfectly() {
        let r = energy_vector(&[(0.7, [0.0, 0.0, 1.0])]);
        assert!(close(r.magnitude, 1.0), "one source must be fully confident: {r:?}");
        assert!(close(r.vector[2], 1.0));
    }

    /// Gain scale cancels out — rE is a direction, never a loudness.
    #[test]
    fn the_energy_vector_is_gain_invariant() {
        let quiet = energy_vector(&[(0.1, [1.0, 0.0, 0.0]), (0.05, [0.0, 0.0, 1.0])]);
        let loud = energy_vector(&[(1.0, [1.0, 0.0, 0.0]), (0.5, [0.0, 0.0, 1.0])]);
        assert!(close(quiet.magnitude, loud.magnitude));
        assert!(close(quiet.vector[0], loud.vector[0]));
    }

    /// Two equal, opposed sources cancel: maximum ambiguity, zero confidence.
    #[test]
    fn opposed_sources_cancel_to_no_confidence() {
        let r = energy_vector(&[(0.5, [0.0, 0.0, 1.0]), (0.5, [0.0, 0.0, -1.0])]);
        assert!(r.magnitude < EPS, "opposed equal gains must not claim a direction: {r:?}");
    }

    /// The two vectors are NOT the same measurement: gain-squared weighting
    /// pulls rE harder toward the louder source than rV.
    #[test]
    fn energy_and_velocity_disagree_when_gains_differ() {
        let sources = [(1.0, [0.0, 0.0, 1.0]), (0.5, [1.0, 0.0, 0.0])];
        let e = energy_vector(&sources);
        let v = velocity_vector(&sources);
        assert!(
            e.vector[2] > v.vector[2],
            "rE must lean further toward the louder source than rV: rE={e:?} rV={v:?}"
        );
    }

    #[test]
    fn a_zero_length_direction_is_skipped_not_nan() {
        let r = energy_vector(&[(1.0, [0.0, 0.0, 0.0]), (1.0, [0.0, 0.0, 1.0])]);
        assert!(r.vector.iter().all(|c| c.is_finite()), "no NaN from a zero direction");
        assert!(close(r.magnitude, 1.0), "the degenerate source must not dilute the real one");
    }

    /// The band adapter is scoped to the REAL 3-tuple shape.
    #[test]
    fn the_band_adapter_takes_the_three_bands_that_actually_exist() {
        let bass_only = three_band_energy_vector(1.0, 0.0, 0.0);
        assert!(close(bass_only.vector[0], 0.0), "the low band sits dead centre: {bass_only:?}");
        assert!(close(bass_only.magnitude, 1.0));

        let treble_only = three_band_energy_vector(0.0, 0.0, 1.0);
        assert!(treble_only.vector[0] > 0.0, "the high band opens to one side");

        let mid_only = three_band_energy_vector(0.0, 1.0, 0.0);
        assert!(mid_only.vector[0] < 0.0, "the mid band opens to the other");
    }

    /// Mid and high are symmetric, so an equal pair localises straight ahead —
    /// the authored mapping has no built-in left/right bias.
    #[test]
    fn equal_mid_and_high_localise_straight_ahead() {
        let r = three_band_energy_vector(0.0, 1.0, 1.0);
        assert!(close(r.vector[0], 0.0), "symmetric bands must not lean: {r:?}");
        assert!(r.vector[2] > 0.0, "and must still point forward");
    }

    /// Negative band energy is not a direction reversal — it is bad input,
    /// clamped rather than silently flipping the vector.
    #[test]
    fn negative_band_energy_is_clamped_not_reflected() {
        let r = three_band_energy_vector(0.0, -1.0, 1.0);
        assert!(r.vector[0] > 0.0, "a negative mid must not pull the vector left: {r:?}");
    }
}
