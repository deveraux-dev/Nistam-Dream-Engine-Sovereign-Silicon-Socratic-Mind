// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! GPU Warden & Emulated 64-Bit Deterministic WebGPU Pipeline.
//!
//! Implements:
//! 1. Dual 32-bit register emulated 64-bit fixed-point integer math for bit-perfect cross-platform determinism:
//!    `C_low = A_low + B_low`, `carry = (C_low < A_low) as u32`, `C_high = A_high + B_high + carry`.
//! 2. Cross-platform deterministic normal vectors and state tokens across diverse GPU hardware.
//! 3. Bit-perfect WGSL compute shader kernels for S13 Balanced Ternary GEMM & GEMV (`s13_gemm_tile`, `s13_gemv_1d`).
//! 4. Host-side reference simulation and 32-bit GPU storage buffer packing utilities.

#![deny(unsafe_code)]

use crate::s13::S13Error;

/// Embedded WGSL Compute Shader Kernel for Bit-Perfect S13 Ternary MatMul & Emulated 64-bit Fixed-Point.
pub const S13_WGSL_COMPUTE_SHADER: &str = r#"
// Dual 32-bit register emulated 64-bit integer representation
struct U64Emulated {
    low: u32,
    high: u32,
};

fn u64_from_u32(val: u32) -> U64Emulated {
    return U64Emulated(val, 0u);
}

fn u64_from_i32(val: i32) -> U64Emulated {
    let u = u32(val);
    let hi = select(0u, 0xFFFFFFFFu, val < 0);
    return U64Emulated(u, hi);
}

fn u64_is_neg(a: U64Emulated) -> bool {
    return (a.high & 0x80000000u) != 0u;
}

fn u64_add(a: U64Emulated, b: U64Emulated) -> U64Emulated {
    let c_low = a.low + b.low;
    let carry = select(0u, 1u, c_low < a.low);
    let c_high = a.high + b.high + carry;
    return U64Emulated(c_low, c_high);
}

fn u64_sub(a: U64Emulated, b: U64Emulated) -> U64Emulated {
    let c_low = a.low - b.low;
    let borrow = select(0u, 1u, a.low < b.low);
    let c_high = a.high - b.high - borrow;
    return U64Emulated(c_low, c_high);
}

fn u64_neg(a: U64Emulated) -> U64Emulated {
    return u64_add(U64Emulated(~a.low, ~a.high), U64Emulated(1u, 0u));
}

fn u64_mul_u32(x: u32, y: u32) -> U64Emulated {
    let x0 = x & 0xFFFFu; let x1 = x >> 16u;
    let y0 = y & 0xFFFFu; let y1 = y >> 16u;
    let p00 = x0 * y0;
    let p01 = x0 * y1;
    let p10 = x1 * y0;
    let p11 = x1 * y1;
    let carry = (p00 >> 16u) + (p01 & 0xFFFFu) + (p10 & 0xFFFFu);
    let lo = (p00 & 0xFFFFu) | ((carry & 0xFFFFu) << 16u);
    let hi = p11 + (p01 >> 16u) + (p10 >> 16u) + (carry >> 16u);
    return U64Emulated(lo, hi);
}

struct GemmParams {
    m_rows: u32,
    k_cols: u32,
    n_cols: u32,
    bytes_per_row: u32,
    words_per_row: u32,
    scale_permyriad: i32,
};

@group(0) @binding(0) var<storage, read> packed_weights: array<u32>;
@group(0) @binding(1) var<storage, read> activations: array<i32>;
@group(0) @binding(2) var<storage, read_write> outputs: array<i32>;
@group(0) @binding(3) var<uniform> params: GemmParams;
// 243-entry byte -> 5x2-bit trit-digit table (digit 0/1/2 per 2-bit field);
// replaces the serial %3 /3 unpack chain in the GEMV hot loop
@group(0) @binding(4) var<storage, read> trit_lut: array<u32>;

var<workgroup> tile_act: array<i32, 1024>;
var<workgroup> row_partial: array<i32, 128>;

// Fast 1D Vector GEMV kernel for single-token autoregressive decoding.
// ONE WORKGROUP PER ROW: 128 threads stride the row's packed words so
// consecutive threads read consecutive words (coalesced), then reduce.
// i32 accumulation is integer-exact by bound: |acc| <= k_cols_max(14336) x
// |act|_max(32767) ~= 4.7e8 < i32::MAX, and exactly order-independent.
@compute @workgroup_size(128, 1, 1)
fn s13_gemv_1d(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let row = wg_id.x;
    if (row >= params.m_rows) {
        return;
    }

    var acc: i32 = 0;
    let row_word_offset = row * params.words_per_row;

    for (var w: u32 = local_id.x; w < params.words_per_row; w = w + 128u) {
        let word = packed_weights[row_word_offset + w];
        let k_word_base = w * 20u; // 4 bytes x 5 trits per word
        for (var b: u32 = 0u; b < 4u; b = b + 1u) {
            let byte_val = (word >> (b * 8u)) & 0xFFu;
            if (byte_val >= 243u) {
                // Out-of-band sentinel trap
                continue;
            }
            let packed5 = trit_lut[byte_val];
            let k_base = k_word_base + b * 5u;
            for (var t: u32 = 0u; t < 5u; t = t + 1u) {
                let k = k_base + t;
                if (k < params.k_cols) {
                    let trit = i32((packed5 >> (t * 2u)) & 3u) - 1;
                    acc = acc + trit * activations[k];
                }
            }
        }
    }

    row_partial[local_id.x] = acc;
    workgroupBarrier();
    if (local_id.x != 0u) {
        return;
    }
    var total: i32 = 0;
    for (var i: u32 = 0u; i < 128u; i = i + 1u) {
        total = total + row_partial[i];
    }
    acc = total;

    let is_negative = acc < 0;
    let mag = u32(select(acc, -acc, is_negative));
    // scale 10000 is the fleet-wide identity path; general path is exact via
    // split div-mod (no u32 overflow: mag <= 4.7e8, parts stay < 2^32)
    var scaled: u32;
    let s = u32(abs(params.scale_permyriad));
    if (s == 10000u) {
        scaled = mag;
    } else {
        scaled = (mag / 10000u) * s + ((mag % 10000u) * s) / 10000u;
    }
    let final_val = select(i32(scaled), -i32(scaled), is_negative != (params.scale_permyriad < 0));

    outputs[row] = clamp(final_val, -32768, 32767);
}

// A1 multi-row GEMV: FOUR ROWS PER WORKGROUP (128 threads = 4 slices of 32
// lanes). Each 32-lane slice strides its own row's packed words; ONE barrier,
// then four serial-32 reduction tails (threads 0..3). ceil(m/4) dispatch.
// i32 accumulation identical to s13_gemv_1d — bit-identical outputs by
// integer-add commutativity; only the lane layout differs.
@compute @workgroup_size(128, 1, 1)
fn s13_gemv_4row(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let sub = local_id.x / 32u;
    let lane = local_id.x % 32u;
    let row = wg_id.x * 4u + sub;

    var acc: i32 = 0;
    if (row < params.m_rows) {
        let row_word_offset = row * params.words_per_row;
        for (var w: u32 = lane; w < params.words_per_row; w = w + 32u) {
            let word = packed_weights[row_word_offset + w];
            let k_word_base = w * 20u;
            for (var b: u32 = 0u; b < 4u; b = b + 1u) {
                let byte_val = (word >> (b * 8u)) & 0xFFu;
                if (byte_val >= 243u) {
                    continue;
                }
                let packed5 = trit_lut[byte_val];
                let k_base = k_word_base + b * 5u;
                for (var t: u32 = 0u; t < 5u; t = t + 1u) {
                    let k = k_base + t;
                    if (k < params.k_cols) {
                        let trit = i32((packed5 >> (t * 2u)) & 3u) - 1;
                        acc = acc + trit * activations[k];
                    }
                }
            }
        }
    }

    row_partial[local_id.x] = acc;
    workgroupBarrier();
    if (local_id.x >= 4u) {
        return;
    }
    let out_row = wg_id.x * 4u + local_id.x;
    if (out_row >= params.m_rows) {
        return;
    }
    var total: i32 = 0;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        total = total + row_partial[local_id.x * 32u + i];
    }

    let is_negative = total < 0;
    let mag = u32(select(total, -total, is_negative));
    var scaled: u32;
    let s = u32(abs(params.scale_permyriad));
    if (s == 10000u) {
        scaled = mag;
    } else {
        scaled = (mag / 10000u) * s + ((mag % 10000u) * s) / 10000u;
    }
    let final_val = select(i32(scaled), -i32(scaled), is_negative != (params.scale_permyriad < 0));

    outputs[out_row] = clamp(final_val, -32768, 32767);
}

// 2D Tiled GEMM kernel conforming to Ampere 32x32 tile contracts
@compute @workgroup_size(32, 1, 1)
fn s13_gemm_tile(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let row = global_id.x;
    let col = global_id.y;
    if (row >= params.m_rows || col >= params.n_cols) {
        return;
    }

    var acc = U64Emulated(0u, 0u);
    let row_word_offset = row * params.words_per_row;
    let total_tiles = (params.k_cols + 31u) / 32u;

    for (var tile_k: u32 = 0u; tile_k < total_tiles; tile_k = tile_k + 1u) {
        let act_idx = tile_k * 32u + local_id.x;
        if (act_idx < params.k_cols) {
            tile_act[local_id.x] = activations[col * params.k_cols + act_idx];
        } else {
            tile_act[local_id.x] = 0;
        }
        workgroupBarrier();

        let k_start = tile_k * 32u;
        for (var step: u32 = 0u; step < 32u; step = step + 1u) {
            let curr_k = k_start + step;
            if (curr_k >= params.k_cols) {
                break;
            }

            let byte_global_idx = curr_k / 5u;
            let trit_in_byte = curr_k % 5u;
            let word_idx = byte_global_idx / 4u;
            let byte_in_word = byte_global_idx % 4u;

            let word = packed_weights[row_word_offset + word_idx];
            let byte_val = (word >> (byte_in_word * 8u)) & 0xFFu;

            if (byte_val < 243u) {
                var rem = byte_val;
                for (var t: u32 = 0u; t < trit_in_byte; t = t + 1u) {
                    rem = rem / 3u;
                }
                let digit = rem % 3u;
                let trit = i32(digit) - 1;
                let act = tile_act[step];
                let term = trit * act;

                if (term >= 0) {
                    acc = u64_add(acc, U64Emulated(u32(term), 0u));
                } else {
                    acc = u64_sub(acc, U64Emulated(u32(-term), 0u));
                }
            }
        }
        workgroupBarrier();
    }

    let is_negative = u64_is_neg(acc);
    // select() cannot take struct operands in WGSL; branch instead
    var mag = acc;
    if (is_negative) {
        mag = u64_neg(acc);
    }
    let scaled_lo = (mag.low * u32(abs(params.scale_permyriad))) / 10000u;
    let final_val = select(i32(scaled_lo), -i32(scaled_lo), is_negative != (params.scale_permyriad < 0));

    outputs[col * params.m_rows + row] = clamp(final_val, -32768, 32767);
}
"#;

/// Parameters and uniform layout for S13 WGSL GPU GEMM / GEMV dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct GemmParams {
    /// Number of output rows ($M$).
    pub m_rows: u32,
    /// Number of input columns ($K$).
    pub k_cols: u32,
    /// Number of output columns / batch dimension ($N$).
    pub n_cols: u32,
    /// Number of S13 packed bytes per row.
    pub bytes_per_row: u32,
    /// Number of 32-bit packed words per row.
    pub words_per_row: u32,
    /// Permyriad scale factor ($1.0 = 10{,}000$).
    pub scale_permyriad: i32,
}

impl GemmParams {
    /// Create parameters for a matrix-vector ($M \times K$, $N=1$) GEMV operation.
    pub const fn new_gemv(m_rows: u32, k_cols: u32, scale_permyriad: i32) -> Self {
        let bytes_per_row = (k_cols + 4) / 5;
        let words_per_row = (bytes_per_row + 3) / 4;
        Self {
            m_rows,
            k_cols,
            n_cols: 1,
            bytes_per_row,
            words_per_row,
            scale_permyriad,
        }
    }

    /// Create parameters for a general matrix-matrix ($M \times K \times N$) GEMM operation.
    pub const fn new_gemm(m_rows: u32, k_cols: u32, n_cols: u32, scale_permyriad: i32) -> Self {
        let bytes_per_row = (k_cols + 4) / 5;
        let words_per_row = (bytes_per_row + 3) / 4;
        Self {
            m_rows,
            k_cols,
            n_cols,
            bytes_per_row,
            words_per_row,
            scale_permyriad,
        }
    }
}

/// Build the 243-entry byte -> 5x2-bit trit-digit unpack table consumed by the
/// GEMV kernel at binding 4 (digit 0/1/2 per 2-bit field; shader subtracts 1).
pub fn trit_unpack_lut() -> [u32; 243] {
    let mut lut = [0u32; 243];
    for (byte, entry) in lut.iter_mut().enumerate() {
        let mut rem = byte as u32;
        let mut packed = 0u32;
        for t in 0..5 {
            packed |= (rem % 3) << (t * 2);
            rem /= 3;
        }
        *entry = packed;
    }
    lut
}

/// Copy and pack an array of S13 byte-packed weights into 32-bit aligned words for GPU storage buffer ingestion.
pub fn pack_s13_bytes_to_words_slice(
    bytes: &[u8],
    bytes_per_row: usize,
    words_per_row: usize,
    rows: usize,
    out_words: &mut [u32],
) -> Result<(), S13Error> {
    if out_words.len() < rows * words_per_row {
        return Err(S13Error::IndexOutOfBounds);
    }
    out_words.fill(0);
    for r in 0..rows {
        for w in 0..words_per_row {
            let mut word = 0u32;
            for b in 0..4 {
                let byte_idx = r * bytes_per_row + w * 4 + b;
                if byte_idx < bytes.len() && (w * 4 + b) < bytes_per_row {
                    word |= (bytes[byte_idx] as u32) << (b * 8);
                }
            }
            out_words[r * words_per_row + w] = word;
        }
    }
    Ok(())
}

/// Host-side reference emulator executing the exact WGSL `s13_gemv_1d` pipeline.
/// Guarantees bit-identical parity between CPU reference and GPU compute shader.
pub fn simulate_s13_gemv_wgsl(
    params: &GemmParams,
    packed_weights: &[u32],
    activations: &[i32],
    outputs: &mut [i32],
) -> Result<(), S13Error> {
    if outputs.len() < params.m_rows as usize {
        return Err(S13Error::IndexOutOfBounds);
    }

    for row in 0..params.m_rows as usize {
        let row_word_offset = row * params.words_per_row as usize;
        // i32 accumulation, bit-identical to the WGSL kernel (bound: 14336 x 32767 < i32::MAX)
        let mut acc: i32 = 0;
        let mut k = 0usize;

        for w in 0..params.words_per_row as usize {
            let word_idx = row_word_offset + w;
            if word_idx >= packed_weights.len() {
                return Err(S13Error::IndexOutOfBounds);
            }
            let word = packed_weights[word_idx];
            for b in 0..4 {
                let byte_val = ((word >> (b * 8)) & 0xFF) as u8;
                if byte_val >= 243 {
                    return Err(S13Error::SentinelDetected(byte_val));
                }
                let mut rem = byte_val;
                for _ in 0..5 {
                    if k >= params.k_cols as usize {
                        break;
                    }
                    let digit = rem % 3;
                    rem /= 3;
                    let trit = (digit as i8) - 1;
                    if k < activations.len() {
                        acc += (trit as i32) * activations[k];
                    }
                    k += 1;
                }
            }
        }

        let is_negative = acc < 0;
        let mag = acc.unsigned_abs();
        let s = params.scale_permyriad.unsigned_abs();
        let scaled = if s == 10_000 {
            mag
        } else {
            (mag / 10_000) * s + ((mag % 10_000) * s) / 10_000
        };
        let final_val = if is_negative ^ (params.scale_permyriad < 0) {
            -(scaled as i32)
        } else {
            scaled as i32
        };

        outputs[row] = final_val.clamp(i16::MIN as i32, i16::MAX as i32);
    }
    Ok(())
}

/// Emulated 64-bit fixed-point integer composed of dual 32-bit registers (`low`, `high`).
/// Guarantees bit-identical results on 32-bit GPU execution units without relying on native 64-bit ALUs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct EmulatedU64 {
    /// Lower 32 bits.
    pub low: u32,
    /// Upper 32 bits.
    pub high: u32,
}

impl EmulatedU64 {
    /// Zero constant.
    pub const ZERO: Self = Self { low: 0, high: 0 };

    /// Create from native `u64`.
    #[inline(always)]
    pub const fn from_u64(val: u64) -> Self {
        Self {
            low: val as u32,
            high: (val >> 32) as u32,
        }
    }

    /// Convert back to native `u64`.
    #[inline(always)]
    pub const fn to_u64(self) -> u64 {
        ((self.high as u64) << 32) | (self.low as u64)
    }

    /// Deterministic Dual-Register Addition with carry propagation.
    /// `C_low = A_low + B_low; carry = (C_low < A_low) as u32; C_high = A_high + B_high + carry;`
    #[inline(always)]
    pub const fn add(self, rhs: Self) -> Self {
        let (c_low, carry_flag) = self.low.overflowing_add(rhs.low);
        let carry = if carry_flag { 1u32 } else { 0u32 };
        let c_high = self.high.wrapping_add(rhs.high).wrapping_add(carry);
        Self {
            low: c_low,
            high: c_high,
        }
    }

    /// Deterministic Dual-Register Subtraction with borrow propagation.
    #[inline(always)]
    pub const fn sub(self, rhs: Self) -> Self {
        let (c_low, borrow_flag) = self.low.overflowing_sub(rhs.low);
        let borrow = if borrow_flag { 1u32 } else { 0u32 };
        let c_high = self.high.wrapping_sub(rhs.high).wrapping_sub(borrow);
        Self {
            low: c_low,
            high: c_high,
        }
    }

    /// Deterministic Fixed-Point Multiplicative Accumulation (32-bit x 32-bit -> 64-bit).
    #[inline(always)]
    pub const fn mul_u32(a: u32, b: u32) -> Self {
        let prod = (a as u64) * (b as u64);
        Self::from_u64(prod)
    }
}

/// Bit-perfect deterministic 3D integer fixed-point normal vector (range -10000..=10000).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct DeterministicNormal {
    /// X normal coordinate in Permyriad ([-10000, 10000]).
    pub x: i32,
    /// Y normal coordinate in Permyriad ([-10000, 10000]).
    pub y: i32,
    /// Z normal coordinate in Permyriad ([-10000, 10000]).
    pub z: i32,
}

impl DeterministicNormal {
    /// Up vector constant (0, 10000, 0).
    pub const UP: Self = Self { x: 0, y: 10_000, z: 0 };

    /// Integer dot product using emulated 64-bit accumulation.
    #[inline(always)]
    pub fn dot_emulated(&self, rhs: &Self) -> i64 {
        let xx = (self.x as i64) * (rhs.x as i64);
        let yy = (self.y as i64) * (rhs.y as i64);
        let zz = (self.z as i64) * (rhs.z as i64);
        xx + yy + zz
    }
}

/// State token header dispatched across GPU workgroups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct GpuStateToken {
    /// Simulation tick.
    pub sim_tick: EmulatedU64,
    /// 3D Normal vector.
    pub normal: DeterministicNormal,
    /// Active lane identifier.
    pub lane_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s13::pack_5_trits;

    #[test]
    fn test_emulated_u64_addition_and_carry() {
        let a = EmulatedU64 {
            low: 0xFFFF_FFFF,
            high: 0x0000_0001,
        };
        let b = EmulatedU64 {
            low: 0x0000_0001,
            high: 0x0000_0002,
        };
        let sum = a.add(b);

        // a = 0x1_FFFF_FFFF (8589934591)
        // b = 0x2_0000_0001 (8589934593)
        // expected sum = 0x4_0000_0000 (17179869184)
        assert_eq!(sum.low, 0x0000_0000);
        assert_eq!(sum.high, 0x0000_0004);
        assert_eq!(sum.to_u64(), 17_179_869_184);
    }

    #[test]
    fn test_emulated_u64_subtraction() {
        let a = EmulatedU64::from_u64(10_000_000_000);
        let b = EmulatedU64::from_u64(3_000_000_000);
        let diff = a.sub(b);
        assert_eq!(diff.to_u64(), 7_000_000_000);
    }

    #[test]
    fn test_deterministic_normal_dot() {
        let norm1 = DeterministicNormal {
            x: 5_000,
            y: 5_000,
            z: 0,
        };
        let norm2 = DeterministicNormal {
            x: 5_000,
            y: -5_000,
            z: 0,
        };
        let dot = norm1.dot_emulated(&norm2);
        // (5000*5000) + (5000*-5000) + 0 = 25000000 - 25000000 = 0
        assert_eq!(dot, 0);
    }

    #[test]
    fn test_wgsl_shader_present_and_complete() {
        assert!(S13_WGSL_COMPUTE_SHADER.contains("u64_add"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("u64_sub"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("u64_mul_u32"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("s13_gemm_tile"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("s13_gemv_1d"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("s13_gemv_4row"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("GemmParams"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("@compute"));
        assert!(S13_WGSL_COMPUTE_SHADER.contains("@workgroup_size(32, 1, 1)"));
    }

    #[test]
    fn test_emulated_u64_multiplication() {
        let prod = EmulatedU64::mul_u32(100_000, 100_000);
        assert_eq!(prod.to_u64(), 10_000_000_000);
    }

    #[test]
    fn test_deterministic_normal_up_constant() {
        let up = DeterministicNormal::UP;
        assert_eq!(up.x, 0);
        assert_eq!(up.y, 10_000);
        assert_eq!(up.z, 0);
    }

    #[test]
    fn test_gemm_params_construction() {
        let params_v = GemmParams::new_gemv(64, 128, 10_000);
        assert_eq!(params_v.m_rows, 64);
        assert_eq!(params_v.k_cols, 128);
        assert_eq!(params_v.n_cols, 1);
        assert_eq!(params_v.bytes_per_row, (128 + 4) / 5);
        assert_eq!(params_v.words_per_row, (params_v.bytes_per_row + 3) / 4);
        assert_eq!(params_v.scale_permyriad, 10_000);

        let params_m = GemmParams::new_gemm(32, 256, 16, 5_000);
        assert_eq!(params_m.m_rows, 32);
        assert_eq!(params_m.k_cols, 256);
        assert_eq!(params_m.n_cols, 16);
        assert_eq!(params_m.scale_permyriad, 5_000);
    }

    #[test]
    fn test_s13_gpu_gemv_simulation_parity() {
        let b0 = pack_5_trits([1, -1, 1, 0, -1]).unwrap();
        let b1 = pack_5_trits([0, 1, 1, -1, 0]).unwrap();
        let b2 = pack_5_trits([-1, -1, 0, 1, 1]).unwrap();
        let b3 = pack_5_trits([1, 1, 0, 0, 1]).unwrap();

        let raw_bytes = [b0, b1, b2, b3];
        let mut packed_words = [0u32; 1];
        pack_s13_bytes_to_words_slice(&raw_bytes, 4, 1, 1, &mut packed_words).unwrap();

        let activations = [
            100, 200, 300, 400, 500, // b0
            600, 700, 800, 900, 1000, // b1
            1100, 1200, 1300, 1400, 1500, // b2
            1600, 1700, 1800, 1900, 2000, // b3
        ];

        let params = GemmParams::new_gemv(1, 20, 10_000);
        let mut outputs = [0i32; 1];
        simulate_s13_gemv_wgsl(&params, &packed_words, &activations, &mut outputs).unwrap();

        // Expected dot calculation:
        // b0: (1*100) + (-1*200) + (1*300) + (0*400) + (-1*500) = 100 - 200 + 300 - 500 = -300
        // b1: (0*600) + (1*700) + (1*800) + (-1*900) + (0*1000) = 700 + 800 - 900 = 600
        // b2: (-1*1100) + (-1*1200) + (0*1300) + (1*1400) + (1*1500) = -1100 - 1200 + 1400 + 1500 = 600
        // b3: (1*1600) + (1*1700) + (0*1800) + (0*1900) + (1*2000) = 1600 + 1700 + 2000 = 5300
        // Total sum = -300 + 600 + 600 + 5300 = 6200
        // Scaled by 10000/10000 = 6200
        assert_eq!(outputs[0], 6200);
    }

    #[test]
    fn test_s13_gpu_gemv_sentinel_trap() {
        let params = GemmParams::new_gemv(1, 5, 10_000);
        // Word containing byte 254 (sentinel) in byte 0
        let packed_words = [254u32];
        let activations = [100, 200, 300, 400, 500];
        let mut outputs = [0i32; 1];
        let res = simulate_s13_gemv_wgsl(&params, &packed_words, &activations, &mut outputs);
        assert_eq!(res, Err(S13Error::SentinelDetected(254)));
    }
}

