//! Physically-based rendering math — Cook-Torrance BRDF.
//!
//! Every function is `#[inline]`, allocation-free, and written against `glam` +
//! `core` only, so the identical code runs on the CPU (these parity tests) and,
//! once the rust-gpu toolchain lands, inside the SPIR-V fragment entry point.
//! Floats live here at the GPU boundary by design; the integer-only rule applies
//! to sim/game logic, not to the BRDF.

use glam::Vec3;

const PI: f32 = core::f32::consts::PI;

/// Normalize, returning zero for a (near-)zero vector — same semantics as glam's
/// `normalize_or_zero`, but WITHOUT its internal `rcp.is_finite()` guard. That
/// `is_finite` call lowers to a comparison against the `f32::INFINITY` literal,
/// which rust-gpu emits as an `OpConstant` that naga's shader validator rejects
/// ("Float literal is infinite") — making the compiled module unusable in a wgpu
/// render pipeline. The finite epsilon guard here is naga-clean. `length_squared`
/// + `normalize` route through glam's spirv intrinsics (no raw `f32::sqrt`).
#[inline]
fn normalize_safe(v: Vec3) -> Vec3 {
    if v.length_squared() > 1e-12 {
        v.normalize()
    } else {
        Vec3::ZERO
    }
}

/// A point light, shareable CPU↔GPU. (Gets `#[repr(C)] + Pod` when it becomes a
/// real GPU storage-buffer entry in Phase B; plain `Copy` is enough for the math.)
#[derive(Copy, Clone, Debug)]
pub struct PointLight {
    /// Light position in world space.
    pub position: Vec3,
    /// Light color (RGB).
    pub color: Vec3,
    /// Light intensity (brightness multiplier).
    pub intensity: f32,
}

/// GGX / Trowbridge-Reitz normal distribution function.
#[inline]
pub fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    a2 / (PI * denom * denom)
}

/// Schlick-GGX geometry term for a single direction (direct-lighting `k`).
#[inline]
pub fn geometry_schlick_ggx(n_dot_x: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    n_dot_x / (n_dot_x * (1.0 - k) + k)
}

/// Smith geometry term: masking-shadowing across view and light.
#[inline]
pub fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness)
}

/// Fresnel-Schlick reflectance. Result is bounded in `[f0, 1]` for `cos_theta`
/// and `f0` components in `[0, 1]`.
#[inline]
pub fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    let m = (1.0 - cos_theta).clamp(0.0, 1.0);
    let m5 = m * m * m * m * m;
    f0 + (Vec3::ONE - f0) * m5
}

/// Cook-Torrance BRDF evaluated for one light direction `l`, returning the
/// outgoing radiance factor (already weighted by `n·l`). Multiply by the light's
/// incoming radiance to get the contribution.
#[inline]
pub fn cook_torrance_brdf(
    n: Vec3,
    v: Vec3,
    l: Vec3,
    albedo: Vec3,
    roughness: f32,
    metallic: f32,
) -> Vec3 {
    let h = normalize_safe(v + l);
    let n_dot_v = n.dot(v).max(0.0);
    let n_dot_l = n.dot(l).max(0.0);
    let n_dot_h = n.dot(h).max(0.0);
    let h_dot_v = h.dot(v).max(0.0);

    // F0: 0.04 for dielectrics, lerped toward albedo by metallic.
    let f0 = Vec3::splat(0.04).lerp(albedo, metallic);

    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(h_dot_v, f0);

    let numerator = f * (ndf * g);
    let denominator = 4.0 * n_dot_v * n_dot_l + 1e-4;
    let specular = numerator / denominator;

    // Energy split: diffuse keeps what specular reflection did not take, and
    // metals have no diffuse term.
    let kd = (Vec3::ONE - f) * (1.0 - metallic);
    let diffuse = kd * albedo / PI;

    (diffuse + specular) * n_dot_l
}

/// Full single-light contribution: derive the light direction + inverse-square
/// attenuation, then evaluate the BRDF.
#[inline]
pub fn evaluate_pbr_light(
    light: &PointLight,
    frag_pos: Vec3,
    normal: Vec3,
    albedo: Vec3,
    roughness: f32,
    metallic: f32,
    eye: Vec3,
) -> Vec3 {
    let to_light = light.position - frag_pos;
    let dist2 = to_light.length_squared().max(1e-6);
    // glam's normalize handles the sqrt on both the host (std) and the spirv
    // target (no_std); a raw `f32::sqrt` isn't available under rustc_codegen_spirv.
    let l = normalize_safe(to_light);
    let v = normalize_safe(eye - frag_pos);
    let radiance = light.color * (light.intensity / dist2);
    cook_torrance_brdf(normal, v, l, albedo, roughness, metallic) * radiance
}

/// Directional-light PBR + ambient term. Shared host/GPU — the fragment shader
/// and the CPU pixel-readback parity test call THIS exact function with the same
/// inputs. `light_dir` is the direction the light travels, so the surface→light
/// vector is `-light_dir`. Returns linear RGB (no tone-map / sRGB).
#[inline]
pub fn evaluate_pbr_directional(
    light_dir: Vec3,
    light_color: Vec3,
    ambient: f32,
    frag_pos: Vec3,
    normal: Vec3,
    albedo: Vec3,
    roughness: f32,
    metallic: f32,
    camera_pos: Vec3,
) -> Vec3 {
    let n = normalize_safe(normal);
    let l = normalize_safe(-light_dir);
    let v = normalize_safe(camera_pos - frag_pos);
    let brdf = cook_torrance_brdf(n, v, l, albedo, roughness, metallic);
    brdf * light_color + albedo * ambient
}
