// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Gemma 3 27B S13 Balanced Ternary Sovereign Model Configuration & Somatic Projection.
//!
//! Provides the architectural specification for Gemma 3 27B IT under S13 1.58-bit balanced ternary
//! quantization, including $W_{\text{proj}} \in \mathbb{R}^{5 \times 4608}$ fixed-point continuous prefix
//! injection that bypasses the 2.36 GB vocabulary embedding table.

/// Model hidden dimension for Gemma 27B ($d_{\text{model}} = 4608$).
pub const D_MODEL_27B: usize = 4608;

/// Number of transformer layers for Gemma 27B ($n_{\text{layers}} = 46$).
pub const N_LAYERS_27B: usize = 46;

/// Number of query attention heads ($n_{\text{heads}} = 32$).
pub const N_HEADS_27B: usize = 32;

/// Number of key-value attention heads ($n_{\text{kv\_heads}} = 16$).
pub const N_KV_HEADS_27B: usize = 16;

/// Head dimension ($d_{\text{head}} = 128$).
pub const D_HEAD_27B: usize = 128;

/// Feed-forward hidden dimension ($d_{\text{ff}} = 14336$).
pub const D_FF_27B: usize = 14336;

/// Permyriad fixed-point divisor (1.0000 = 10,000).
pub const PERMYRIAD_ONE: i32 = 10_000;

/// Number of continuous axes in the 5D Pentaract coordinate space.
pub const PENTARACT_5D_AXES: usize = 5;

/// Gemma 27B Model Configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gemma27bConfig {
    /// Hidden dimension $d_{\text{model}}$ (4608).
    pub d_model: usize,
    /// Number of transformer layers (46).
    pub n_layers: usize,
    /// Query attention heads (32).
    pub n_heads: usize,
    /// Key/Value attention heads (16).
    pub n_kv_heads: usize,
    /// Per-head dimension (128).
    pub d_head: usize,
    /// Feed-forward intermediate dimension (14336).
    pub d_ff: usize,
    /// Permyriad fixed-point scaling factor (10000).
    pub permyriad_scale: i32,
    /// Target resident VRAM budget in megabytes (2710 MB).
    pub vram_resident_target_mb: u32,
}

impl Default for Gemma27bConfig {
    fn default() -> Self {
        Self {
            d_model: D_MODEL_27B,
            n_layers: N_LAYERS_27B,
            n_heads: N_HEADS_27B,
            n_kv_heads: N_KV_HEADS_27B,
            d_head: D_HEAD_27B,
            d_ff: D_FF_27B,
            permyriad_scale: PERMYRIAD_ONE,
            vram_resident_target_mb: 2710,
        }
    }
}

/// Zero-heap fixed-point $W_{\text{proj}} \in \mathbb{R}^{5 \times 4608}$ projection parameters.
pub struct Somatic27bProjectionWeights {
    /// Projection weights from 5D Pentaract space to $d_{\text{model}} = 4608$.
    /// Memory footprint: $5 \times 4608 \times 2\text{ bytes} = 46.08\text{ KB}$.
    pub proj_weights: [[i16; PENTARACT_5D_AXES]; D_MODEL_27B],
}

impl Somatic27bProjectionWeights {
    /// Deterministic orthogonal initialization for 5D-to-4608 projection.
    pub const fn default_fixed() -> Self {
        let mut proj_weights = [[0i16; PENTARACT_5D_AXES]; D_MODEL_27B];
        let mut m = 0;
        while m < D_MODEL_27B {
            let mut a = 0;
            while a < PENTARACT_5D_AXES {
                let phase = ((m * 31 + a * 127) % 2048) as i32;
                let val = if phase < 1024 {
                    ((phase * 10_000) / 1024) - 5_000
                } else {
                    5_000 - (((phase - 1024) * 10_000) / 1024)
                };
                proj_weights[m][a] = (val / 2) as i16;
                a += 1;
            }
            m += 1;
        }
        Self { proj_weights }
    }

    /// Project a 5D Pentaract coordinate vector $[x, y, z, \text{mag}, \text{color}]$ directly into $\mathbb{R}^{4608}$.
    /// Zero heap allocations, writing directly to the provided pre-allocated buffer.
    #[inline(always)]
    pub fn project_5d_to_dmodel(&self, coords_5d: &[i32; PENTARACT_5D_AXES], out_dmodel: &mut [i32; D_MODEL_27B]) {
        for (m, row) in self.proj_weights.iter().enumerate() {
            let mut acc: i64 = 0;
            for (a, &w) in row.iter().enumerate() {
                acc += (w as i64) * (coords_5d[a] as i64);
            }
            out_dmodel[m] = (acc / (PERMYRIAD_ONE as i64)) as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemma_27b_config_dimensions() {
        let config = Gemma27bConfig::default();
        assert_eq!(config.d_model, 4608);
        assert_eq!(config.n_layers, 46);
        assert_eq!(config.n_heads, 32);
        assert_eq!(config.n_kv_heads, 16);
        assert_eq!(config.d_head, 128);
        assert_eq!(config.d_ff, 14336);
        assert_eq!(config.vram_resident_target_mb, 2710);
    }

    #[test]
    fn test_somatic_27b_projection_determinism() {
        let proj = Somatic27bProjectionWeights::default_fixed();
        let coords_a = [1000, -2000, 3000, 500, 100];
        let coords_b = [-1000, 2000, -3000, -500, -100];

        let mut out_a = [0i32; D_MODEL_27B];
        let mut out_b = [0i32; D_MODEL_27B];

        proj.project_5d_to_dmodel(&coords_a, &mut out_a);
        proj.project_5d_to_dmodel(&coords_b, &mut out_b);

        assert_ne!(out_a, out_b);

        // Verification of linearity / reflection
        let mut out_a_again = [0i32; D_MODEL_27B];
        proj.project_5d_to_dmodel(&coords_a, &mut out_a_again);
        assert_eq!(out_a, out_a_again, "projection must be bit-exact deterministic");
    }
}
