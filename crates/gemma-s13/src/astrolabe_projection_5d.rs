// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! # 5D-to-2D Projection & Parallax Math Engine
//!
//! Autonomous 5D celestial projection module mapping high-dimensional
//! coordinates $P = (x, y, z, w, v)$ to 2D viewports with $SO(5)$ Givens
//! hyperplane rotations, Planck blackbody spectral chromaticity,
//! depth-of-field Airy disk blur, and relativistic stellar aberration.

use crate::star_codebook::{BakedStarCentroid, StarCodebookView};

/// 5D Astral Coordinate Node representing $(x, y, z, w, v)$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star5D {
    /// Direction cosine X (right ascension component).
    pub x: f32,
    /// Direction cosine Y (declination component).
    pub y: f32,
    /// Primary heliocentric depth (distance in parsecs / scale).
    pub z: f32,
    /// Hyper-depth (apparent magnitude / focal modulus).
    pub w: f32,
    /// Phase axis (resonant frequency / temperature spectral index).
    pub v: f32,
}

/// Projected 2D Celestial Point ready for viewport rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedStar {
    /// Screen X position in pixels.
    pub px: f32,
    /// Screen Y position in pixels.
    pub py: f32,
    /// Rendered star disk radius in pixels.
    pub radius: f32,
    /// Focal alpha opacity in range `[0.0, 1.0]`.
    pub alpha: f32,
    /// RGB Blackbody spectral color tint `[r, g, b]` in `[0.0, 1.0]`.
    pub rgb: [f32; 3],
}

/// Compute RGB chromaticity from normalized phase/temperature axis `v` in `[-1.0, 1.0]`.
///
/// Maps `v` to the Planck blackbody color gradient:
/// - `v < -0.33`: Cool Red / Amber (M-Dwarf class, e.g., Betelgeuse).
/// - `-0.33 <= v < 0.33`: Solar Golden / White (G/F class, e.g., Sun, Capella).
/// - `v >= 0.33`: Hot Electric Blue / Violet (B/O class, e.g., Sirius, Rigel).
#[inline]
pub fn spectral_temperature_rgb(v: f32) -> [f32; 3] {
    let t_norm = ((v + 1.0) * 0.5).clamp(0.0, 1.0);
    if t_norm < 0.33 {
        // Cool Red/Amber
        [1.0, 0.4 + 1.2 * t_norm, 0.1]
    } else if t_norm < 0.66 {
        // Solar White/Golden
        [1.0, 0.95, 0.4 + 1.5 * (t_norm - 0.33)]
    } else {
        // Hot Electric Blue/Violet
        let blue_shift = 0.7 - 0.6 * (t_norm - 0.66);
        [blue_shift.clamp(0.1, 1.0), 0.85, 1.0]
    }
}

impl Star5D {
    /// Construct a `Star5D` coordinate directly from a baked HYG star centroid.
    #[inline]
    pub fn from_baked_star(star: &BakedStarCentroid) -> Self {
        // Direction cosines derived from normalized RA [0, 1] and Dec [-1, 1]
        let ra_rad = star.ra_normalized() * core::f32::consts::PI * 2.0;
        let dec_rad = star.dec_normalized() * (core::f32::consts::PI * 0.5);

        let cos_dec = dec_rad.cos();
        let x = cos_dec * ra_rad.cos();
        let y = cos_dec * ra_rad.sin();
        let z = (star.distance_u16 as f32).max(1.0);
        let w = (star.mag_permyriad as f32) / 1000.0; // Scaled apparent magnitude
        let v = ((star.teff_idx as f32) / 127.5) - 1.0; // Normalized to [-1.0, 1.0]

        Star5D { x, y, z, w, v }
    }

    /// Calculate the 5D Euclidean norm squared ($x^2 + y^2 + z^2 + w^2 + v^2$).
    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w + self.v * self.v
    }

    /// Apply an $SO(5)$ Givens hyperplane rotation across the `(Z, W)` and `(W, V)` planes.
    ///
    /// Preserves the 5D Euclidean norm bit-for-bit up to floating-point precision.
    #[inline]
    pub fn rotate_5d(&self, theta_zw: f32, phi_wv: f32) -> Self {
        // (Z, W) plane rotation
        let (s_zw, c_zw) = theta_zw.sin_cos();
        let z_rot = self.z * c_zw - self.w * s_zw;
        let w_rot = self.z * s_zw + self.w * c_zw;

        // (W, V) plane rotation
        let (s_wv, c_wv) = phi_wv.sin_cos();
        let w_final = w_rot * c_wv - self.v * s_wv;
        let v_final = w_rot * s_wv + self.v * c_wv;

        Star5D {
            x: self.x,
            y: self.y,
            z: z_rot,
            w: w_final,
            v: v_final,
        }
    }

    /// Apply relativistic stellar aberration (Lorentz boost) for a probe traveling at speed `beta = v / c`.
    #[inline]
    pub fn apply_lorentz_boost(&self, beta: f32) -> Self {
        let beta_clamped = beta.clamp(-0.99, 0.99);
        let gamma = 1.0 / (1.0 - beta_clamped * beta_clamped).sqrt();

        // Longitudinal contraction and Doppler shift along phase axis
        let z_boosted = gamma * (self.z - beta_clamped * self.w);
        let w_boosted = gamma * (self.w - beta_clamped * self.z);
        let v_boosted = self.v * ((1.0 - beta_clamped) / (1.0 + beta_clamped)).sqrt();

        Star5D {
            x: self.x,
            y: self.y,
            z: z_boosted,
            w: w_boosted,
            v: v_boosted,
        }
    }

    /// Project this 5D star into 2D viewport space with parallax, depth-of-field, and chromaticity.
    #[inline]
    pub fn project(
        &self,
        cam_x: f32,
        cam_y: f32,
        theta_zw: f32,
        screen_w: f32,
        screen_h: f32,
    ) -> ProjectedStar {
        // 1. Hyperplane rotation in (Z, W)
        let (s, c) = theta_zw.sin_cos();
        let z_eff = (self.z * c - self.w * s).max(0.1);
        let w_eff = self.z * s + self.w * c;

        // 2. Parallax factor (inverse depth)
        let parallax = 1.0 / z_eff;

        // 3. Screen coordinates with camera offset
        let px = (self.x * 400.0 - cam_x * parallax) + screen_w * 0.5;
        let py = (self.y * 400.0 - cam_y * parallax) + screen_h * 0.5;

        // 4. Perceptual depth-of-field & opacity
        let radius = (3.5 * parallax * 10.0).clamp(0.5, 8.0);
        let alpha = (1.0 / (1.0 + w_eff.abs() * 0.2)).clamp(0.08, 1.0);
        let rgb = spectral_temperature_rgb(self.v);

        ProjectedStar {
            px,
            py,
            radius,
            alpha,
            rgb,
        }
    }
}

/// Zero-heap batch projector: projects stars from codebook into a pre-allocated output buffer.
///
/// Returns the number of stars successfully projected.
pub fn project_star_batch(
    codebook: &StarCodebookView,
    out_projected: &mut [ProjectedStar],
    cam_x: f32,
    cam_y: f32,
    theta_zw: f32,
    screen_w: f32,
    screen_h: f32,
) -> usize {
    let count = codebook.star_count().min(out_projected.len());
    for i in 0..count {
        if let Some(star) = codebook.get_star(i) {
            let star5d = Star5D::from_baked_star(&star);
            out_projected[i] = star5d.project(cam_x, cam_y, theta_zw, screen_w, screen_h);
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_so5_rotation_norm_conservation() {
        let star = Star5D {
            x: 0.5,
            y: 0.3,
            z: 10.0,
            w: 2.5,
            v: -0.4,
        };

        let initial_norm = star.norm_squared();
        let rotated = star.rotate_5d(0.785, 0.456);
        let rotated_norm = rotated.norm_squared();

        // Energy/Norm conserved within float epsilon
        assert!(
            (initial_norm - rotated_norm).abs() < 1e-4,
            "SO(5) rotation must conserve 5D Euclidean norm: {} vs {}",
            initial_norm,
            rotated_norm
        );
    }

    #[test]
    fn test_blackbody_spectral_color_bounds() {
        for v_step in -10..=10 {
            let v = v_step as f32 / 10.0;
            let rgb = spectral_temperature_rgb(v);
            for channel in rgb {
                assert!(
                    (0.0..=1.0).contains(&channel),
                    "RGB channel must be in [0.0, 1.0], found {}",
                    channel
                );
            }
        }
    }

    #[test]
    fn test_projection_bounds_and_parallax() {
        let near_star = Star5D {
            x: 0.1,
            y: 0.1,
            z: 2.0,
            w: 0.5,
            v: 0.8,
        };
        let far_star = Star5D {
            x: 0.1,
            y: 0.1,
            z: 200.0,
            w: 0.5,
            v: 0.8,
        };

        let p_near_stationary = near_star.project(0.0, 0.0, 0.0, 1920.0, 1080.0);
        let p_far_stationary = far_star.project(0.0, 0.0, 0.0, 1920.0, 1080.0);

        let p_near = near_star.project(10.0, 10.0, 0.0, 1920.0, 1080.0);
        let p_far = far_star.project(10.0, 10.0, 0.0, 1920.0, 1080.0);

        let disp_near = (p_near.px - p_near_stationary.px).abs();
        let disp_far = (p_far.px - p_far_stationary.px).abs();

        // Near star has larger radius and higher parallax displacement
        assert!(p_near.radius > p_far.radius);
        assert!(disp_near > disp_far, "Near star displacement {} must exceed far star displacement {}", disp_near, disp_far);
        assert!(p_near.alpha > 0.0 && p_near.alpha <= 1.0);
    }
}
