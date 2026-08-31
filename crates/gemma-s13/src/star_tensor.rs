// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Tensor Star Decomposition: S13 balanced-ternary core, continuous peripheral leaves.
//! Offline tooling only — heap-allocating; firewalled from the no_std inference hotpaths.
//! Base-243 packing delegates to `crate::s13` (one home); no duplicate encode logic here.

use crate::s13::{pack_5_trits, unpack_5_trits, S13Error, TRITS_PER_BYTE};
use std::vec;
use std::vec::Vec;

/// Error surface for tensor-star construction and unpacking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarTensorError {
    /// Trit count does not equal the product of the declared shape.
    ShapeMismatch {
        /// Number of trits supplied.
        got: usize,
        /// Product of the declared shape.
        expected: usize,
    },
    /// Underlying S13 pack/unpack failure (invalid trit or sentinel byte).
    S13(S13Error),
}

impl From<S13Error> for StarTensorError {
    fn from(e: S13Error) -> Self {
        StarTensorError::S13(e)
    }
}

/// Tensor dimensions, core ranks, and group quantization size for a star network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarTensorConfig {
    /// Full tensor dimensions `I_1 ..= I_N`.
    pub dimensions: Vec<usize>,
    /// Core ranks `R_1 ..= R_N`, one per mode.
    pub core_ranks: Vec<usize>,
    /// Elements per quantization group (group-scale normalization unit).
    pub group_size: usize,
}

/// Discrete balanced-ternary core tensor packed 5 trits/byte via `crate::s13`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S13TensorCore {
    /// Base-243 packed bytes, each in `0..=242` (sentinel range excluded by construction).
    pub packed_data: Vec<u8>,
    /// Core tensor shape `R_1 ..= R_N`.
    pub shape: Vec<usize>,
}

impl S13TensorCore {
    /// Pack balanced trits (`-1 | 0 | +1`) into a core of the given shape.
    /// Trailing positions in the final byte are padded with `0`-trits.
    pub fn from_trits(trits: &[i8], shape: &[usize]) -> Result<Self, StarTensorError> {
        let expected: usize = shape.iter().product();
        if trits.len() != expected {
            return Err(StarTensorError::ShapeMismatch {
                got: trits.len(),
                expected,
            });
        }
        let mut packed_data = Vec::with_capacity(expected.div_ceil(TRITS_PER_BYTE));
        for chunk in trits.chunks(TRITS_PER_BYTE) {
            let mut group = [0i8; 5];
            group[..chunk.len()].copy_from_slice(chunk);
            packed_data.push(pack_5_trits(group)?);
        }
        Ok(S13TensorCore {
            packed_data,
            shape: shape.to_vec(),
        })
    }

    /// Number of live trit elements (`shape` product; excludes pad trits).
    pub fn element_count(&self) -> usize {
        self.shape.iter().product()
    }

    /// Random-access one trit by coordinate (row-major, last dim fastest) —
    /// unpacks only the single containing byte.
    pub fn get_trit(&self, coords: &[usize]) -> Result<i8, StarTensorError> {
        if coords.len() != self.shape.len()
            || coords.iter().zip(self.shape.iter()).any(|(&c, &d)| c >= d)
        {
            return Err(StarTensorError::ShapeMismatch {
                got: coords.len(),
                expected: self.shape.len(),
            });
        }
        let mut flat = 0usize;
        for (&c, &d) in coords.iter().zip(self.shape.iter()) {
            flat = flat * d + c;
        }
        let five = unpack_5_trits(self.packed_data[flat / TRITS_PER_BYTE])?;
        Ok(five[flat % TRITS_PER_BYTE])
    }

    /// Unpack to balanced trits, truncated to `element_count()`.
    pub fn unpack_trits(&self) -> Result<Vec<i8>, StarTensorError> {
        let n = self.element_count();
        let mut out = Vec::with_capacity(self.packed_data.len() * TRITS_PER_BYTE);
        for &byte in &self.packed_data {
            out.extend_from_slice(&unpack_5_trits(byte)?);
        }
        out.truncate(n);
        Ok(out)
    }
}

const JACOBI_MAX_SWEEPS: usize = 30;
const JACOBI_EPSILON: f32 = 1e-7;

/// Deterministic one-sided (Hestenes) Jacobi SVD of an `rows x cols` row-major matrix.
/// Returns `(U, S, V)`: `U` is `rows x cols` column-orthonormal, `S` descending, `V` is
/// `cols x cols` orthogonal (not transposed). Fixed sweep order + `total_cmp` sort keep
/// output bit-reproducible for identical input on the same target.
pub fn jacobi_svd(matrix: &[f32], rows: usize, cols: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut u = matrix.to_vec();
    let mut v = vec![0.0f32; cols * cols];
    for i in 0..cols {
        v[i * cols + i] = 1.0;
    }

    for _ in 0..JACOBI_MAX_SWEEPS {
        let mut converged = true;
        for i in 0..cols {
            for j in (i + 1)..cols {
                let mut dot_ii = 0.0f32;
                let mut dot_jj = 0.0f32;
                let mut dot_ij = 0.0f32;
                for r in 0..rows {
                    let vi = u[r * cols + i];
                    let vj = u[r * cols + j];
                    dot_ii += vi * vi;
                    dot_jj += vj * vj;
                    dot_ij += vi * vj;
                }
                if dot_ij.abs() <= JACOBI_EPSILON * (dot_ii * dot_jj).sqrt() {
                    continue;
                }
                converged = false;

                let tau = (dot_jj - dot_ii) / (2.0 * dot_ij);
                let t = if tau >= 0.0 {
                    1.0 / (tau + (1.0 + tau * tau).sqrt())
                } else {
                    -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = c * t;

                for r in 0..rows {
                    let u_i = u[r * cols + i];
                    let u_j = u[r * cols + j];
                    u[r * cols + i] = c * u_i - s * u_j;
                    u[r * cols + j] = s * u_i + c * u_j;
                }
                for r in 0..cols {
                    let v_i = v[r * cols + i];
                    let v_j = v[r * cols + j];
                    v[r * cols + i] = c * v_i - s * v_j;
                    v[r * cols + j] = s * v_i + c * v_j;
                }
            }
        }
        if converged {
            break;
        }
    }

    let mut s_vals = vec![0.0f32; cols];
    for i in 0..cols {
        let mut norm_sq = 0.0f32;
        for r in 0..rows {
            let val = u[r * cols + i];
            norm_sq += val * val;
        }
        s_vals[i] = norm_sq.sqrt();
        if s_vals[i] > JACOBI_EPSILON {
            let inv_s = 1.0 / s_vals[i];
            for r in 0..rows {
                u[r * cols + i] *= inv_s;
            }
        }
    }

    let mut indices: Vec<usize> = (0..cols).collect();
    indices.sort_by(|&a, &b| s_vals[b].total_cmp(&s_vals[a]));

    let mut u_sorted = vec![0.0f32; rows * cols];
    let mut s_sorted = vec![0.0f32; cols];
    let mut v_sorted = vec![0.0f32; cols * cols];
    for (new_i, &old_i) in indices.iter().enumerate() {
        s_sorted[new_i] = s_vals[old_i];
        for r in 0..rows {
            u_sorted[r * cols + new_i] = u[r * cols + old_i];
        }
        for r in 0..cols {
            v_sorted[r * cols + new_i] = v[r * cols + old_i];
        }
    }
    (u_sorted, s_sorted, v_sorted)
}

/// Mode-`mode` unfolding: fibers of that mode become rows of a
/// `shape[mode] x (product/shape[mode])` row-major matrix.
pub fn unfold(tensor: &[f32], shape: &[usize], mode: usize) -> Vec<f32> {
    let mode_dim = shape[mode];
    let total_elements: usize = shape.iter().product();
    let cols = total_elements / mode_dim;
    let mut unfolded = vec![0.0f32; mode_dim * cols];

    for (flat_idx, &val) in tensor.iter().enumerate() {
        let mut rem = flat_idx;
        let mut mode_idx = 0;
        let mut col_idx = 0;
        let mut col_stride = 1;
        for (d, &dim) in shape.iter().enumerate().rev() {
            let coord = rem % dim;
            rem /= dim;
            if d == mode {
                mode_idx = coord;
            } else {
                col_idx += coord * col_stride;
                col_stride *= dim;
            }
        }
        unfolded[mode_idx * cols + col_idx] = val;
    }
    unfolded
}

/// Inverse of `unfold` under the same coordinate convention.
pub fn fold(matrix: &[f32], shape: &[usize], mode: usize) -> Vec<f32> {
    let mode_dim = shape[mode];
    let total_elements: usize = shape.iter().product();
    let cols = total_elements / mode_dim;
    let mut folded = vec![0.0f32; total_elements];

    for row in 0..mode_dim {
        for col in 0..cols {
            let val = matrix[row * cols + col];
            let mut flat_idx = 0;
            let mut rem_col = col;
            let mut stride = 1;
            for (d, &dim) in shape.iter().enumerate().rev() {
                let coord = if d == mode {
                    row
                } else {
                    let c = rem_col % dim;
                    rem_col /= dim;
                    c
                };
                flat_idx += coord * stride;
                stride *= dim;
            }
            folded[flat_idx] = val;
        }
    }
    folded
}

/// Mode-`mode` product: contracts `matrix` (`m_rows x shape[mode]`, row-major) against the
/// tensor's mode fibers. Returns `(tensor', shape')` with `shape'[mode] = m_rows`.
pub fn mode_multiply(
    tensor: &[f32],
    shape: &[usize],
    mode: usize,
    matrix: &[f32],
    m_rows: usize,
) -> (Vec<f32>, Vec<usize>) {
    let mode_dim = shape[mode];
    let cols = tensor.len() / mode_dim;
    let unfolded = unfold(tensor, shape, mode);

    let mut product = vec![0.0f32; m_rows * cols];
    for r in 0..m_rows {
        for k in 0..mode_dim {
            let m_rk = matrix[r * mode_dim + k];
            if m_rk == 0.0 {
                continue;
            }
            for c in 0..cols {
                product[r * cols + c] += m_rk * unfolded[k * cols + c];
            }
        }
    }

    let mut new_shape = shape.to_vec();
    new_shape[mode] = m_rows;
    (fold(&product, &new_shape, mode), new_shape)
}

/// HOSVD initialization: per-mode truncated left singular bases as continuous leaves
/// (`I_n x R_n`, row-major), core = tensor contracted with every leaf transposed.
pub fn fit_hosvd(tensor: &[f32], config: &StarTensorConfig) -> (Vec<f32>, Vec<Vec<f32>>) {
    let n_modes = config.dimensions.len();
    let mut leaves: Vec<Vec<f32>> = Vec::with_capacity(n_modes);

    for mode in 0..n_modes {
        let mode_dim = config.dimensions[mode];
        let rank = config.core_ranks[mode];
        let cols = tensor.len() / mode_dim;
        let unfolded = unfold(tensor, &config.dimensions, mode);
        let (u, _s, _v) = jacobi_svd(&unfolded, mode_dim, cols);

        let mut leaf = vec![0.0f32; mode_dim * rank];
        for r in 0..mode_dim {
            leaf[r * rank..(r + 1) * rank].copy_from_slice(&u[r * cols..r * cols + rank]);
        }
        leaves.push(leaf);
    }

    let mut core = tensor.to_vec();
    let mut core_shape = config.dimensions.clone();
    for (mode, leaf) in leaves.iter().enumerate() {
        let mode_dim = config.dimensions[mode];
        let rank = config.core_ranks[mode];
        let mut leaf_t = vec![0.0f32; rank * mode_dim];
        for r in 0..mode_dim {
            for c in 0..rank {
                leaf_t[c * mode_dim + r] = leaf[r * rank + c];
            }
        }
        let (next, next_shape) = mode_multiply(&core, &core_shape, mode, &leaf_t, rank);
        core = next;
        core_shape = next_shape;
    }
    (core, leaves)
}

/// Complete star network: quantized ternary core, group scales, continuous leaves.
#[derive(Debug, Clone, PartialEq)]
pub struct StarTensorDecomposition {
    /// Discrete Base-243 packed core.
    pub core: S13TensorCore,
    /// One max-abs scale per quantization group of the core.
    pub group_scales: Vec<f32>,
    /// Peripheral factor matrices, `I_n x R_n` row-major, one per mode.
    pub leaves: Vec<Vec<f32>>,
    /// Star network configuration.
    pub config: StarTensorConfig,
}

impl StarTensorDecomposition {
    /// Group-scale normalize a continuous core, project to trits (`|x| > 0.5` of group
    /// max-abs), and pack via the Card 1 wrapper. Returns the core and per-group scales.
    pub fn quantize_core_s13(
        continuous_core: &[f32],
        shape: &[usize],
        group_size: usize,
    ) -> Result<(S13TensorCore, Vec<f32>), StarTensorError> {
        let mut trits = Vec::with_capacity(continuous_core.len());
        let mut scales = Vec::with_capacity(continuous_core.len().div_ceil(group_size));
        for chunk in continuous_core.chunks(group_size) {
            let max_abs = chunk.iter().fold(1e-7f32, |acc, &v| acc.max(v.abs()));
            scales.push(max_abs);
            for &v in chunk {
                let normalized = v / max_abs;
                trits.push(if normalized > 0.5 {
                    1i8
                } else if normalized < -0.5 {
                    -1i8
                } else {
                    0i8
                });
            }
        }
        let core = S13TensorCore::from_trits(&trits, shape)?;
        Ok((core, scales))
    }

    /// Dequantize the core back to f32 via per-group scales.
    pub fn dequantize_core(&self) -> Result<Vec<f32>, StarTensorError> {
        let trits = self.core.unpack_trits()?;
        let group = self.config.group_size;
        Ok(trits
            .iter()
            .enumerate()
            .map(|(i, &t)| (t as f32) * self.group_scales[i / group])
            .collect())
    }

    /// Full star contraction: dequantized core expanded through every leaf.
    pub fn contract(&self) -> Result<Vec<f32>, StarTensorError> {
        let mut y = self.dequantize_core()?;
        let mut shape = self.core.shape.clone();
        for (mode, leaf) in self.leaves.iter().enumerate() {
            let (next, next_shape) =
                mode_multiply(&y, &shape, mode, leaf, self.config.dimensions[mode]);
            y = next;
            shape = next_shape;
        }
        Ok(y)
    }

    /// One ALS pass per iteration: for each mode, contract the dequantized core with every
    /// other leaf, unfold at that mode, and update the leaf via `X_(n) * P^+` (closed form).
    pub fn refine_leaves_als(
        &mut self,
        original_tensor: &[f32],
        iterations: usize,
    ) -> Result<(), StarTensorError> {
        let core_f32 = self.dequantize_core()?;
        let n_modes = self.config.dimensions.len();
        for _ in 0..iterations {
            for n in 0..n_modes {
                let mut y = core_f32.clone();
                let mut shape = self.core.shape.clone();
                for d in 0..n_modes {
                    if d == n {
                        continue;
                    }
                    let (next, next_shape) =
                        mode_multiply(&y, &shape, d, &self.leaves[d], self.config.dimensions[d]);
                    y = next;
                    shape = next_shape;
                }
                let r_n = self.core.shape[n];
                let i_rest = y.len() / r_n;
                let projected = unfold(&y, &shape, n);
                let pinv_proj = pinv(&projected, r_n, i_rest);

                let i_n = self.config.dimensions[n];
                let x_n = unfold(original_tensor, &self.config.dimensions, n);
                self.leaves[n] = matmul(&x_n, &pinv_proj, i_n, i_rest, r_n);
            }
        }
        Ok(())
    }

    /// Ratio of dense f32 bytes to packed core + scales + leaf bytes.
    pub fn compression_ratio(&self) -> f32 {
        let original_bytes = self.config.dimensions.iter().product::<usize>() * 4;
        let compressed_bytes = self.core.packed_data.len()
            + self.group_scales.len() * 4
            + self.leaves.iter().map(|l| l.len() * 4).sum::<usize>();
        original_bytes as f32 / compressed_bytes as f32
    }
}

/// Frobenius norm of the elementwise difference.
pub fn frobenius_error(original: &[f32], reconstructed: &[f32]) -> f32 {
    original
        .iter()
        .zip(reconstructed.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

fn matmul(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    for row in 0..m {
        for step in 0..k {
            let a_rs = a[row * k + step];
            if a_rs == 0.0 {
                continue;
            }
            for col in 0..n {
                c[row * n + col] += a_rs * b[step * n + col];
            }
        }
    }
    c
}

fn pinv(matrix: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let transpose = rows < cols;
    let (work, w_rows, w_cols) = if transpose {
        let mut t = vec![0.0f32; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                t[c * rows + r] = matrix[r * cols + c];
            }
        }
        (t, cols, rows)
    } else {
        (matrix.to_vec(), rows, cols)
    };

    let (u, s, v) = jacobi_svd(&work, w_rows, w_cols);
    let mut s_inv = vec![0.0f32; w_cols];
    for i in 0..w_cols {
        if s[i] > JACOBI_EPSILON {
            s_inv[i] = 1.0 / s[i];
        }
    }

    let mut out = vec![0.0f32; rows * cols];
    if transpose {
        for r in 0..cols {
            for c in 0..rows {
                let mut sum = 0.0f32;
                for k in 0..w_cols {
                    sum += u[r * w_cols + k] * s_inv[k] * v[c * w_cols + k];
                }
                out[r * rows + c] = sum;
            }
        }
    } else {
        for r in 0..cols {
            for c in 0..rows {
                let mut sum = 0.0f32;
                for k in 0..cols {
                    sum += v[r * cols + k] * s_inv[k] * u[c * cols + k];
                }
                out[r * rows + c] = sum;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s13_tensor_core_packing() {
        // Golden bytes derived from s13::pack_5_trits MSB-first digit order:
        // [1,-1,0,1,-1] -> digits [2,0,1,2,0] -> 177; [0;5] (incl. pad) -> 121.
        let trits: [i8; 8] = [1, -1, 0, 1, -1, 0, 0, 0];
        let core = S13TensorCore::from_trits(&trits, &[2, 4]).unwrap();
        assert_eq!(core.packed_data, vec![177u8, 121u8]);
        assert_eq!(core.element_count(), 8);
        assert_eq!(core.unpack_trits().unwrap(), trits);
    }

    #[test]
    fn test_get_trit_coordinate_flattening() {
        let trits: Vec<i8> = (0..3 * 4 * 5).map(|i| ((i % 3) as i8) - 1).collect();
        let core = S13TensorCore::from_trits(&trits, &[3, 4, 5]).unwrap();
        // Row-major, last dim fastest: flat(c0,c1,c2) = c0*20 + c1*5 + c2.
        for c0 in 0..3 {
            for c1 in 0..4 {
                for c2 in 0..5 {
                    let flat = c0 * 20 + c1 * 5 + c2;
                    assert_eq!(core.get_trit(&[c0, c1, c2]).unwrap(), trits[flat]);
                }
            }
        }
        assert!(core.get_trit(&[3, 0, 0]).is_err());
        assert!(core.get_trit(&[0, 0]).is_err());
    }

    #[test]
    fn test_multidimensional_shapes_roundtrip() {
        let trits: Vec<i8> = (0..3 * 4 * 5).map(|i| ((i % 3) as i8) - 1).collect();
        let core = S13TensorCore::from_trits(&trits, &[3, 4, 5]).unwrap();
        assert_eq!(core.packed_data.len(), 12); // 60 trits / 5 per byte
        assert_eq!(core.unpack_trits().unwrap(), trits);
    }

    #[test]
    fn test_shape_mismatch_rejected() {
        let err = S13TensorCore::from_trits(&[0, 0, 0], &[2, 2]).unwrap_err();
        assert_eq!(
            err,
            StarTensorError::ShapeMismatch {
                got: 3,
                expected: 4
            }
        );
    }

    #[test]
    fn test_jacobi_svd_reconstruction() {
        let (rows, cols) = (3, 2);
        let matrix = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (u, s, v) = jacobi_svd(&matrix, rows, cols);
        assert_eq!((u.len(), s.len(), v.len()), (6, 2, 4));
        assert!(s[0] >= s[1]);
        for r in 0..rows {
            for c in 0..cols {
                let mut sum = 0.0f32;
                for k in 0..cols {
                    sum += u[r * cols + k] * s[k] * v[c * cols + k];
                }
                assert!((matrix[r * cols + c] - sum).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn test_jacobi_svd_deterministic() {
        let matrix: Vec<f32> = (0..20).map(|x| ((x * 7 + 3) % 11) as f32).collect();
        let (u1, s1, v1) = jacobi_svd(&matrix, 5, 4);
        let (u2, s2, v2) = jacobi_svd(&matrix, 5, 4);
        assert_eq!(u1, u2);
        assert_eq!(s1, s2);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_tensor_star_5d_projection_isomorphism() {
        // SO(5) Givens rotation matrix over the (Z,W) then (W,V) planes — same plane
        // convention as astrolabe_projection_5d::Star5D::rotate_5d.
        fn givens_5d(theta_zw: f32, phi_wv: f32) -> Vec<f32> {
            let mut rot_zw = vec![0.0f32; 25];
            let mut rot_wv = vec![0.0f32; 25];
            for i in 0..5 {
                rot_zw[i * 5 + i] = 1.0;
                rot_wv[i * 5 + i] = 1.0;
            }
            let (s1, c1) = theta_zw.sin_cos();
            rot_zw[2 * 5 + 2] = c1;
            rot_zw[2 * 5 + 3] = -s1;
            rot_zw[3 * 5 + 2] = s1;
            rot_zw[3 * 5 + 3] = c1;
            let (s2, c2) = phi_wv.sin_cos();
            rot_wv[3 * 5 + 3] = c2;
            rot_wv[3 * 5 + 4] = -s2;
            rot_wv[4 * 5 + 3] = s2;
            rot_wv[4 * 5 + 4] = c2;
            matmul(&rot_wv, &rot_zw, 5, 5, 5)
        }

        let config = StarTensorConfig {
            dimensions: vec![5, 4, 3],
            core_ranks: vec![3, 2, 2],
            group_size: 6,
        };
        let tensor: Vec<f32> = (0..60).map(|x| (((x * 7 + 1) % 19) as f32) * 0.05).collect();
        let (cont_core, leaves) = fit_hosvd(&tensor, &config);
        let (core, group_scales) =
            StarTensorDecomposition::quantize_core_s13(&cont_core, &config.core_ranks, config.group_size)
                .unwrap();
        let decomp = StarTensorDecomposition {
            core,
            group_scales,
            leaves,
            config: config.clone(),
        };

        let rot = givens_5d(0.37, -0.81);

        // LHS: rotate the mode-0 leaf, then contract.
        let mut rotated = decomp.clone();
        rotated.leaves[0] = matmul(&rot, &decomp.leaves[0], 5, 5, config.core_ranks[0]);
        let lhs = rotated.contract().unwrap();

        // RHS: contract, then rotate the reconstruction along mode 0.
        let (rhs, _) = mode_multiply(&decomp.contract().unwrap(), &config.dimensions, 0, &rot, 5);

        for (a, b) in lhs.iter().zip(rhs.iter()) {
            assert!((a - b).abs() <= 1e-5, "isomorphism violated: {a} vs {b}");
        }
    }

    #[test]
    fn test_quantize_core_group_scaling() {
        // Group 1 max-abs = 0.8: 0.8 -> +1, -0.6 -> -1 (|-0.75|>0.5), 0.1 -> 0, 0.3 -> 0.
        let core = [0.8f32, -0.6, 0.1, 0.3, 8.0, -2.0, 5.0, 0.0];
        let (packed, scales) =
            StarTensorDecomposition::quantize_core_s13(&core, &[2, 4], 4).unwrap();
        assert_eq!(scales, vec![0.8, 8.0]);
        assert_eq!(
            packed.unpack_trits().unwrap(),
            vec![1, -1, 0, 0, 1, 0, 1, 0]
        );
    }

    #[test]
    fn test_als_refinement_error_reduction() {
        let config = StarTensorConfig {
            dimensions: vec![3, 3, 3],
            core_ranks: vec![3, 3, 3],
            group_size: 9,
        };
        let tensor: Vec<f32> = (0..27).map(|x| (((x * 11 + 5) % 17) as f32) * 0.1).collect();

        let (cont_core, leaves) = fit_hosvd(&tensor, &config);
        let (core, group_scales) =
            StarTensorDecomposition::quantize_core_s13(&cont_core, &config.core_ranks, config.group_size)
                .unwrap();
        let mut decomp = StarTensorDecomposition {
            core,
            group_scales,
            leaves,
            config,
        };

        // Ternary quantization shock: reconstruction error is real and nonzero.
        let err_shock = frobenius_error(&tensor, &decomp.contract().unwrap());
        assert!(err_shock > 1e-3, "expected a quantization shock, got {err_shock}");

        decomp.refine_leaves_als(&tensor, 1).unwrap();
        let err_als1 = frobenius_error(&tensor, &decomp.contract().unwrap());
        assert!(
            err_als1 < err_shock,
            "ALS pass 1 must reduce error: {err_als1} vs {err_shock}"
        );

        decomp.refine_leaves_als(&tensor, 1).unwrap();
        let err_als2 = frobenius_error(&tensor, &decomp.contract().unwrap());
        assert!(
            err_als2 <= err_als1 + 1e-5,
            "ALS pass 2 must not regress: {err_als2} vs {err_als1}"
        );

        assert!(decomp.compression_ratio() > 0.0);
    }

    #[test]
    fn test_tensor_star_hosvd_roundtrip() {
        // Full-rank HOSVD must reconstruct the tensor: X == core x_1 U1 x_2 U2 x_3 U3.
        let config = StarTensorConfig {
            dimensions: vec![2, 3, 4],
            core_ranks: vec![2, 3, 4],
            group_size: 8,
        };
        let tensor: Vec<f32> = (0..24).map(|x| ((x * 5 + 2) % 13) as f32).collect();
        let (core, leaves) = fit_hosvd(&tensor, &config);
        assert_eq!(core.len(), 24);
        assert_eq!(leaves.len(), 3);

        let mut recon = core;
        let mut shape = config.core_ranks.clone();
        for (mode, leaf) in leaves.iter().enumerate() {
            let (next, next_shape) =
                mode_multiply(&recon, &shape, mode, leaf, config.dimensions[mode]);
            recon = next;
            shape = next_shape;
        }
        assert_eq!(shape, config.dimensions);
        for (a, b) in tensor.iter().zip(recon.iter()) {
            assert!((a - b).abs() < 1e-3, "roundtrip drift: {a} vs {b}");
        }
    }

    #[test]
    fn test_unfold_fold_roundtrip() {
        let shape = vec![2usize, 3, 4];
        let tensor: Vec<f32> = (0..24).map(|x| x as f32).collect();
        for mode in 0..3 {
            let unfolded = unfold(&tensor, &shape, mode);
            assert_eq!(unfolded.len(), 24);
            assert_eq!(fold(&unfolded, &shape, mode), tensor);
        }
    }

    #[test]
    fn test_invalid_trit_rejected() {
        let err = S13TensorCore::from_trits(&[2, 0, 0, 0, 0], &[5]).unwrap_err();
        assert!(matches!(err, StarTensorError::S13(_)));
    }
}
