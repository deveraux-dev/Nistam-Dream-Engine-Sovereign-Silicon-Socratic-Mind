//! Lambert diffuse and the Reinhard display transform. Ported from v2
//! `forge-lighting/src/lib.rs:172-219` and `sky_render.rs:16-22`.

use crate::rig::LightRig;

/// Lambert diffuse intensity at a surface point, `0.0..=1.0`.
/// Returns `0.0` when the light coincides with the surface — an undefined
/// direction is refused rather than normalized into a wrong answer.
pub fn compute_illumination(
    light_pos: [f32; 3],
    surface_pos: [f32; 3],
    surface_normal: [f32; 3],
) -> f32 {
    let l = [
        light_pos[0] - surface_pos[0],
        light_pos[1] - surface_pos[1],
        light_pos[2] - surface_pos[2],
    ];
    let len_sq = l[0] * l[0] + l[1] * l[1] + l[2] * l[2];
    if len_sq < 1e-10 {
        return 0.0;
    }
    let inv = 1.0 / len_sq.sqrt();
    let dot = surface_normal[0] * l[0] * inv
        + surface_normal[1] * l[1] * inv
        + surface_normal[2] * l[2] * inv;
    dot.clamp(0.0, 1.0)
}

/// Lambert against a rig's sun, scaled by sun energy and capped at `1.0`.
///
/// DIVERGES FROM THE DONOR, deliberately. `LightRig::sun_direction` points
/// TOWARD the sun: `rig_from_tod` gives it a positive Y at noon, and
/// `sky_radiance` reads a positive Y as daytime. v2's own
/// `compute_illumination_from_rig` negated it, so under v2's own noon rig every
/// surface came back unlit. The negation is dropped here and the convention is
/// stated once: toward-the-sun, everywhere.
pub fn compute_illumination_from_rig(rig: &LightRig, surface_normal: [f32; 3]) -> f32 {
    let l = rig.sun_direction;
    let len_sq = l[0] * l[0] + l[1] * l[1] + l[2] * l[2];
    if len_sq < 1e-10 {
        return 0.0;
    }
    let inv = 1.0 / len_sq.sqrt();
    let dot = surface_normal[0] * l[0] * inv
        + surface_normal[1] * l[1] * inv
        + surface_normal[2] * l[2] * inv;
    (dot.max(0.0) * rig.sun_energy).min(1.0)
}

/// Reinhard tone-map plus 2.2 gamma: one linear-HDR radiance to sRGB8.
#[inline]
pub fn tonemap(c: [f32; 3]) -> [u8; 3] {
    let f = |v: f32| {
        // NaN is not a brightness; an infinity is a very bright one. Clamping
        // rather than zeroing keeps a blown highlight white instead of black.
        let v = if v.is_nan() { 0.0 } else { v.clamp(0.0, 1e30) };
        let mapped = (v / (1.0 + v)).clamp(0.0, 1.0);
        (mapped.powf(1.0 / 2.2) * 255.0).round() as u8
    };
    [f(c[0]), f(c[1]), f(c[2])]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::rig_from_tod;

    #[test]
    fn facing_the_light_is_full_and_facing_away_is_none() {
        let n = [0.0, 1.0, 0.0];
        assert!((compute_illumination([0.0, 10.0, 0.0], [0.0; 3], n) - 1.0).abs() < 0.01);
        assert_eq!(compute_illumination([0.0, -10.0, 0.0], [0.0; 3], n), 0.0);
    }

    #[test]
    fn grazing_light_is_zero_and_forty_five_degrees_is_cos_45() {
        let n = [0.0, 1.0, 0.0];
        assert!(compute_illumination([10.0, 0.0, 0.0], [0.0; 3], n) < 0.01);
        let got = compute_illumination([0.0, 10.0, 10.0], [0.0; 3], n);
        assert!((got - 1.0 / 2.0f32.sqrt()).abs() < 0.01, "got {got}");
    }

    #[test]
    fn a_light_inside_the_surface_is_refused() {
        assert_eq!(compute_illumination([5.0; 3], [5.0; 3], [0.0, 1.0, 0.0]), 0.0);
    }

    #[test]
    fn the_rig_path_agrees_with_the_positional_one() {
        // sun_direction points TOWARD the sun, so a light placed along it must
        // give the same answer as the rig path. This is the assertion that
        // caught v2's sign bug (see compute_illumination_from_rig's note).
        let rig = rig_from_tod(0.5, 0, 0.0);
        let n = [0.0, 1.0, 0.0];
        let d = rig.sun_direction;
        let from_rig = compute_illumination_from_rig(&rig, n);
        let positional =
            compute_illumination([d[0] * 100.0, d[1] * 100.0, d[2] * 100.0], [0.0; 3], n);
        assert!(
            (from_rig - (positional * rig.sun_energy).min(1.0)).abs() < 0.02,
            "rig {from_rig} vs positional {positional}"
        );
    }

    #[test]
    fn a_noon_sun_actually_lights_an_upward_face() {
        // The regression the donor shipped: with the negation in place this is
        // exactly 0.0 at noon.
        let rig = rig_from_tod(0.5, 6, 0.0);
        assert!(rig.sun_direction[1] > 0.0, "noon sun_direction points up, toward the sun");
        let lit = compute_illumination_from_rig(&rig, [0.0, 1.0, 0.0]);
        assert!(lit > 0.5, "a noon up-face must be lit, got {lit}");
        let unlit = compute_illumination_from_rig(&rig, [0.0, -1.0, 0.0]);
        assert_eq!(unlit, 0.0, "and a downward face must not be");
    }

    #[test]
    fn midnight_lights_nothing() {
        let rig = rig_from_tod(0.0, 6, 0.0);
        assert_eq!(compute_illumination_from_rig(&rig, [0.0, 1.0, 0.0]), 0.0);
    }

    #[test]
    fn tonemap_is_monotonic_and_bounded() {
        let mut last = 0u8;
        for step in 0..64 {
            let v = step as f32 * 0.5;
            let got = tonemap([v, v, v])[0];
            assert!(got >= last, "tone-map must not fall as radiance rises");
            last = got;
        }
        assert_eq!(tonemap([0.0; 3]), [0, 0, 0]);
        assert_eq!(tonemap([1e9; 3]), [255, 255, 255]);
    }

    #[test]
    fn tonemap_refuses_negatives_and_nan_rather_than_wrapping() {
        assert_eq!(tonemap([-5.0, -1.0, -0.001]), [0, 0, 0]);
        assert_eq!(tonemap([f32::NAN, f32::INFINITY, f32::NEG_INFINITY]), [0, 255, 0]);
    }
}
