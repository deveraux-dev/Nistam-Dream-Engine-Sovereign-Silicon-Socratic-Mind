//! CPU parity tests: prove the PBR math is well-formed and energy-respecting on
//! the host, so that once the same code compiles to SPIR-V we already know the
//! math is correct without looking at a screen.
//!
//! Note: the R4 brief's `pbr_energy_conservation` test (asserting the raw BRDF
//! value stays ≤ 1.01) is physically wrong — a Cook-Torrance specular lobe spikes
//! without bound as roughness → 0, so proptest finds a low-roughness counter-
//! example immediately. It is replaced here by the *actual* energy invariants:
//! Fresnel reflectance stays in `[f0, 1]`, and the BRDF is finite and non-negative.

use forge_look_v3::pbr::*;
use glam::{vec3, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn ggx_distribution_bounded(
        n_dot_h in 0.0f32..=1.0,
        roughness in 0.001f32..=1.0,
    ) {
        let d = distribution_ggx(n_dot_h, roughness);
        prop_assert!(d >= 0.0, "GGX distribution must be non-negative, got {d}");
        prop_assert!(d.is_finite(), "GGX distribution must be finite, got {d}");
    }

    #[test]
    fn fresnel_at_zero_equals_f0(f0_r in 0.0f32..=1.0) {
        let f0 = Vec3::splat(f0_r);
        let result = fresnel_schlick(1.0, f0); // cos_theta = 1 → head-on
        prop_assert!((result.x - f0.x).abs() < 1e-5);
    }

    // Energy invariant #1: Fresnel never reflects less than f0 nor more than 100%.
    #[test]
    fn fresnel_bounded(
        cos_theta in 0.0f32..=1.0,
        f0_r in 0.0f32..=1.0,
    ) {
        let f = fresnel_schlick(cos_theta, Vec3::splat(f0_r));
        prop_assert!(f.x >= f0_r - 1e-5, "Fresnel below f0: {} < {f0_r}", f.x);
        prop_assert!(f.x <= 1.0 + 1e-5, "Fresnel above 1: {}", f.x);
    }

    // Energy invariant #2: the BRDF is well-formed (finite, non-negative) for all
    // valid inputs — the meaningful replacement for the broken ≤1.01 assertion.
    #[test]
    fn brdf_finite_nonneg(
        roughness in 0.01f32..=1.0,
        metallic in 0.0f32..=1.0,
        lx in -1.0f32..=1.0,
        ly in 0.0f32..=1.0,
        lz in -1.0f32..=1.0,
    ) {
        let n = vec3(0.0, 1.0, 0.0);
        let v = vec3(0.0, 1.0, 0.0);
        let l = vec3(lx, ly, lz).normalize_or_zero();
        let albedo = vec3(0.8, 0.7, 0.6);
        let c = cook_torrance_brdf(n, v, l, albedo, roughness, metallic);
        prop_assert!(
            c.x.is_finite() && c.y.is_finite() && c.z.is_finite(),
            "BRDF produced a non-finite component: {c:?}"
        );
        prop_assert!(
            c.x >= 0.0 && c.y >= 0.0 && c.z >= 0.0,
            "BRDF produced a negative component: {c:?}"
        );
    }

    // Sanity: a brighter / closer light never produces a darker result.
    #[test]
    fn brighter_light_is_not_darker(intensity in 0.1f32..=50.0) {
        let light_a = PointLight { position: vec3(0.0, 2.0, 0.0), color: Vec3::ONE, intensity };
        let light_b = PointLight { intensity: intensity * 2.0, ..light_a };
        let p = Vec3::ZERO;
        let n = vec3(0.0, 1.0, 0.0);
        let eye = vec3(0.0, 1.0, 1.0);
        let albedo = vec3(0.7, 0.7, 0.7);
        let a = evaluate_pbr_light(&light_a, p, n, albedo, 0.5, 0.0, eye);
        let b = evaluate_pbr_light(&light_b, p, n, albedo, 0.5, 0.0, eye);
        prop_assert!(b.x + 1e-4 >= a.x, "doubling intensity darkened the result: {} < {}", b.x, a.x);
    }
}
