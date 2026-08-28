// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! S13 Balanced Ternary Core Engine.
//!
//! Implements 1.58-bit balanced ternary weight packing (-1, 0, +1).
//! Exactly 5 trits are packed into a single 8-bit byte ($3^5 = 243$ states: 0..=242).
//! The upper 13 states (243..=255) are strictly reserved for out-of-band lunar control sentinels.
//! Implements the 13-lane coordinate vector of arity-3 ($3^{13} = 1,594,323$ states) with the
//! all-zero state acting as the absolute physical drift-free equilibrium origin.

#![deny(unsafe_code)]

/// Number of trits packed in a single byte.
pub const TRITS_PER_BYTE: usize = 5;

/// Number of valid ternary states representable in a 5-trit packed byte ($3^5$).
pub const TERNARY_STATES_COUNT: u8 = 243;

/// Out-of-band lunar sentinel boundary threshold (bytes >= 243 are sentinels).
pub const SENTINEL_THRESHOLD: u8 = 243;

/// Total number of states in a 13-lane arity-3 coordinate space ($3^{13}$).
pub const COORDINATE_13_TOTAL_STATES: u32 = 1_594_323;

/// S13 Core Error definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S13Error {
    /// Trit value out of range (must be -1, 0, or +1).
    InvalidTritValue(i8),
    /// Byte is in the upper sentinel range (>= 243).
    SentinelDetected(u8),
    /// State index out of bounds.
    IndexOutOfBounds,
}

/// Balanced ternary trit representation (-1, 0, +1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum Trit {
    /// Negative unit (-1)
    Minus = -1,
    /// Equilibrium zero (0)
    Zero = 0,
    /// Positive unit (+1)
    Plus = 1,
}

impl Trit {
    /// Convert an `i8` integer (-1, 0, +1) into a `Trit`.
    #[inline(always)]
    pub const fn from_i8(val: i8) -> Result<Self, S13Error> {
        match val {
            -1 => Ok(Self::Minus),
            0 => Ok(Self::Zero),
            1 => Ok(Self::Plus),
            _ => Err(S13Error::InvalidTritValue(val)),
        }
    }

    /// Map `Trit` into unsigned radix-3 digit (0, 1, 2) where:
    /// -1 -> 0, 0 -> 1, +1 -> 2.
    #[inline(always)]
    pub const fn to_radix3(self) -> u8 {
        match self {
            Self::Minus => 0,
            Self::Zero => 1,
            Self::Plus => 2,
        }
    }

    /// Convert unsigned radix-3 digit (0, 1, 2) back to `Trit`.
    #[inline(always)]
    pub const fn from_radix3(digit: u8) -> Result<Self, S13Error> {
        match digit {
            0 => Ok(Self::Minus),
            1 => Ok(Self::Zero),
            2 => Ok(Self::Plus),
            _ => Err(S13Error::InvalidTritValue(digit as i8)),
        }
    }

    /// Value as raw `i8`.
    #[inline(always)]
    pub const fn as_i8(self) -> i8 {
        self as i8
    }
}

/// Pack exactly 5 balanced trits into a single 8-bit byte ($3^5 = 243$ states).
/// Returns `Ok(byte)` in range `0..=242`.
#[inline]
pub const fn pack_5_trits(trits: [i8; 5]) -> Result<u8, S13Error> {
    let mut acc: u16 = 0;
    let mut i = 0;
    while i < 5 {
        let t = trits[i];
        let digit = match t {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => return Err(S13Error::InvalidTritValue(t)),
        };
        acc = acc * 3 + digit;
        i += 1;
    }
    if acc < SENTINEL_THRESHOLD as u16 {
        Ok(acc as u8)
    } else {
        Err(S13Error::SentinelDetected(acc as u8))
    }
}

/// Unpack a single byte (range `0..=242`) into exactly 5 balanced trits.
/// Fails with `S13Error::SentinelDetected` if `byte >= 243`.
#[inline]
pub const fn unpack_5_trits(byte: u8) -> Result<[i8; 5], S13Error> {
    if byte >= SENTINEL_THRESHOLD {
        return Err(S13Error::SentinelDetected(byte));
    }
    let mut rem = byte;
    let mut out = [0i8; 5];
    let mut i = 4;
    loop {
        let digit = rem % 3;
        rem /= 3;
        out[i] = match digit {
            0 => -1,
            1 => 0,
            2 => 1,
            _ => 0,
        };
        if i == 0 {
            break;
        }
        i -= 1;
    }
    Ok(out)
}

/// 13-lane coordinate vector of arity-3 ($3^{13} = 1,594,323$ states).
/// The all-zero state `[0; 13]` represents absolute physical drift-free equilibrium.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinate13 {
    /// 13 independent balanced ternary lanes (-1, 0, +1).
    pub lanes: [i8; 13],
}

impl Coordinate13 {
    /// The absolute physical drift-free equilibrium origin (all lanes zero).
    pub const ORIGIN: Self = Self { lanes: [0; 13] };

    /// Create a new 13-lane coordinate vector, validating each lane is in `{-1, 0, 1}`.
    pub const fn new(lanes: [i8; 13]) -> Result<Self, S13Error> {
        let mut i = 0;
        while i < 13 {
            let l = lanes[i];
            if l < -1 || l > 1 {
                return Err(S13Error::InvalidTritValue(l));
            }
            i += 1;
        }
        Ok(Self { lanes })
    }

    /// Check if this coordinate vector is precisely at the equilibrium origin.
    #[inline(always)]
    pub const fn is_origin(&self) -> bool {
        let mut i = 0;
        while i < 13 {
            if self.lanes[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Compute the scalar integer index in $0..1,594,323$.
    pub const fn to_scalar_index(&self) -> u32 {
        let mut idx: u32 = 0;
        let mut i = 0;
        while i < 13 {
            let digit = match self.lanes[i] {
                -1 => 0u32,
                0 => 1u32,
                1 => 2u32,
                _ => 1u32,
            };
            idx = idx * 3 + digit;
            i += 1;
        }
        idx
    }

    /// Reconstruct a 13-lane coordinate vector from a scalar index in $0..1,594,323$.
    pub const fn from_scalar_index(mut idx: u32) -> Result<Self, S13Error> {
        if idx >= COORDINATE_13_TOTAL_STATES {
            return Err(S13Error::IndexOutOfBounds);
        }
        let mut lanes = [0i8; 13];
        let mut i = 12;
        loop {
            let digit = (idx % 3) as u8;
            idx /= 3;
            lanes[i] = match digit {
                0 => -1,
                1 => 0,
                2 => 1,
                _ => 0,
            };
            if i == 0 {
                break;
            }
            i -= 1;
        }
        Ok(Self { lanes })
    }

    /// Vector addition with saturation at `[-1, 1]`.
    pub const fn add_saturating(&self, rhs: &Self) -> Self {
        let mut out = [0i8; 13];
        let mut i = 0;
        while i < 13 {
            let sum = self.lanes[i] + rhs.lanes[i];
            out[i] = if sum > 1 {
                1
            } else if sum < -1 {
                -1
            } else {
                sum
            };
            i += 1;
        }
        Self { lanes: out }
    }

    /// Inner dot product against 13-element integer vector.
    #[inline(always)]
    pub fn dot_i32(&self, weights: &[i32; 13]) -> i32 {
        let mut acc = 0i32;
        for (i, &w) in weights.iter().enumerate() {
            acc += (self.lanes[i] as i32) * w;
        }
        acc
    }
}

/// Precomputed 243-entry lookup table for 5-trit unpacking ($3^5 = 243$ states).
/// Enables instantaneous 0-overhead unpacking without runtime division or modulo loops.
pub static UNPACK_LUT_243: [[i8; 5]; 243] = generate_unpack_lut();

/// Precomputed 243-entry lookup table for 8-lane aligned i16 vector loading.
/// 5 active trits followed by 3 zero padding lanes for direct SIMD register ingestion.
pub static UNPACK_LUT_I16: [[i16; 8]; 243] = generate_unpack_lut_i16();

const fn generate_unpack_lut() -> [[i8; 5]; 243] {
    let mut table = [[0i8; 5]; 243];
    let mut b: usize = 0;
    while b < 243 {
        let mut rem = b as u8;
        let mut out = [0i8; 5];
        let mut i = 4;
        loop {
            let digit = rem % 3;
            rem /= 3;
            out[i] = match digit {
                0 => -1,
                1 => 0,
                2 => 1,
                _ => 0,
            };
            if i == 0 {
                break;
            }
            i -= 1;
        }
        table[b] = out;
        b += 1;
    }
    table
}

const fn generate_unpack_lut_i16() -> [[i16; 8]; 243] {
    let mut table = [[0i16; 8]; 243];
    let mut b: usize = 0;
    while b < 243 {
        let mut rem = b as u8;
        let mut out = [0i16; 8];
        let mut i = 4;
        loop {
            let digit = rem % 3;
            rem /= 3;
            out[i] = match digit {
                0 => -1,
                1 => 0,
                2 => 1,
                _ => 0,
            };
            if i == 0 {
                break;
            }
            i -= 1;
        }
        table[b] = out;
        b += 1;
    }
    table
}

/// Instantaneous $O(1)$ lookup unpacking a single byte (range `0..=242`) into 5 balanced trits.
#[inline(always)]
pub fn unpack_5_trits_fast(byte: u8) -> Result<[i8; 5], S13Error> {
    if byte >= SENTINEL_THRESHOLD {
        Err(S13Error::SentinelDetected(byte))
    } else {
        Ok(UNPACK_LUT_243[byte as usize])
    }
}

/// AVX2 16-byte PSHUFB lookup tables for parallel 2-trit unpacking ($3^2 = 9 \le 16$).
/// Lane value maps radix-3 pair index `(t0*3 + t1)` to individual trit components.
#[cfg(target_arch = "x86_64")]
pub mod avx2_unpacker {
    use super::*;
    use core::arch::x86_64::*;

    /// Low trit PSHUFB LUT: maps pair index 0..8 to t1 in `{-1, 0, 1}`.
    pub const PSHUFB_LUT_LOW: [i8; 32] = [
        -1, 0, 1, -1, 0, 1, -1, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        -1, 0, 1, -1, 0, 1, -1, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    ];

    /// High trit PSHUFB LUT: maps pair index 0..8 to t0 in `{-1, 0, 1}`.
    pub const PSHUFB_LUT_HIGH: [i8; 32] = [
        -1, -1, -1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
        -1, -1, -1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    ];

    /// Unpack 32 bytes of 2-trit radix-3 indices in parallel into two 256-bit registers using `_mm256_shuffle_epi8`.
    #[allow(unsafe_code)]
    #[target_feature(enable = "avx2")]
    pub unsafe fn unpack_2trits_pshufb_avx2(indices: __m256i) -> (__m256i, __m256i) {
        let lut_low = _mm256_loadu_si256(PSHUFB_LUT_LOW.as_ptr() as *const __m256i);
        let lut_high = _mm256_loadu_si256(PSHUFB_LUT_HIGH.as_ptr() as *const __m256i);

        let trits_low = _mm256_shuffle_epi8(lut_low, indices);
        let trits_high = _mm256_shuffle_epi8(lut_high, indices);

        (trits_high, trits_low)
    }

    /// AVX2-accelerated ternary vector matmul with PSHUFB vector-unpacked weights and VPMADDWD.
    #[allow(unsafe_code)]
    #[target_feature(enable = "avx2")]
    pub unsafe fn matmul_vector_avx2(
        packed_weights: &[u8],
        activations: &[i16],
        scale_permyriad: i32,
    ) -> Result<i32, S13Error> {
        let num_trits = packed_weights.len() * TRITS_PER_BYTE;
        if activations.len() < num_trits {
            return Err(S13Error::IndexOutOfBounds);
        }

        let mut acc_vec = _mm256_setzero_si256();
        let mut byte_idx = 0;
        let total_bytes = packed_weights.len();

        let lut_low = _mm256_loadu_si256(PSHUFB_LUT_LOW.as_ptr() as *const __m256i);
        let lut_high = _mm256_loadu_si256(PSHUFB_LUT_HIGH.as_ptr() as *const __m256i);

        // Process 2 packed bytes at a time (10 trits mapped into two 8-lane chunks = 16 lanes)
        while byte_idx + 1 < total_bytes {
            let b0 = packed_weights[byte_idx];
            let b1 = packed_weights[byte_idx + 1];

            if b0 >= SENTINEL_THRESHOLD {
                return Err(S13Error::SentinelDetected(b0));
            }
            if b1 >= SENTINEL_THRESHOLD {
                return Err(S13Error::SentinelDetected(b1));
            }

            let act_base = byte_idx * TRITS_PER_BYTE;

            // Decompose radix-3 pair indices for b0 and b1
            let p0_0 = b0 / 27;
            let p0_1 = (b0 % 27) / 3;
            let t0_4 = (b0 % 3) as i8 - 1;

            let p1_0 = b1 / 27;
            let p1_1 = (b1 % 27) / 3;
            let t1_4 = (b1 % 3) as i8 - 1;

            // Pack pair indices into a 256-bit register
            let mut idx_bytes = [0u8; 32];
            idx_bytes[0] = p0_0;
            idx_bytes[1] = p0_1;
            idx_bytes[16] = p1_0;
            idx_bytes[17] = p1_1;

            let idx_vec = _mm256_loadu_si256(idx_bytes.as_ptr() as *const __m256i);
            let trits_low = _mm256_shuffle_epi8(lut_low, idx_vec);
            let trits_high = _mm256_shuffle_epi8(lut_high, idx_vec);

            let mut low_out = [0i8; 32];
            let mut high_out = [0i8; 32];
            _mm256_storeu_si256(low_out.as_mut_ptr() as *mut __m256i, trits_low);
            _mm256_storeu_si256(high_out.as_mut_ptr() as *mut __m256i, trits_high);

            // Vector weights matching 5-trit + 3-zero packing layout
            let w0 = [
                high_out[0] as i16,
                low_out[0] as i16,
                high_out[1] as i16,
                low_out[1] as i16,
                t0_4 as i16,
                0, 0, 0,
            ];
            let w1 = [
                high_out[16] as i16,
                low_out[16] as i16,
                high_out[17] as i16,
                low_out[17] as i16,
                t1_4 as i16,
                0, 0, 0,
            ];

            let vec_w0 = _mm_loadu_si128(w0.as_ptr() as *const __m128i);
            let vec_w1 = _mm_loadu_si128(w1.as_ptr() as *const __m128i);
            let weights_256 = _mm256_set_m128i(vec_w1, vec_w0);

            // Construct activation vectors matching the 5-trit + 3-zero packing layout
            let a0 = [
                activations[act_base],
                activations[act_base + 1],
                activations[act_base + 2],
                activations[act_base + 3],
                activations[act_base + 4],
                0, 0, 0,
            ];
            let a1 = [
                activations[act_base + 5],
                activations[act_base + 6],
                activations[act_base + 7],
                activations[act_base + 8],
                activations[act_base + 9],
                0, 0, 0,
            ];
            let act0 = _mm_loadu_si128(a0.as_ptr() as *const __m128i);
            let act1 = _mm_loadu_si128(a1.as_ptr() as *const __m128i);
            let acts_256 = _mm256_set_m128i(act1, act0);

            // VPMADDWD: multiplies 16-bit pairs and sums into 32-bit ints
            let prod = _mm256_madd_epi16(weights_256, acts_256);
            acc_vec = _mm256_add_epi32(acc_vec, prod);

            byte_idx += 2;
        }

        // Horizontal reduction of acc_vec
        let mut temp = [0i32; 8];
        _mm256_storeu_si256(temp.as_mut_ptr() as *mut __m256i, acc_vec);
        let mut accum: i64 = temp.iter().map(|&x| x as i64).sum();

        // Handle remaining tail byte if odd length
        if byte_idx < total_bytes {
            let b = packed_weights[byte_idx];
            if b >= SENTINEL_THRESHOLD {
                return Err(S13Error::SentinelDetected(b));
            }
            let trits = UNPACK_LUT_243[b as usize];
            let act_base = byte_idx * TRITS_PER_BYTE;
            for j in 0..5 {
                accum += (trits[j] as i64) * (activations[act_base + j] as i64);
            }
        }

        let scaled = (accum * (scale_permyriad as i64)) / 10_000;
        Ok(scaled as i32)
    }
}

/// Compute 1.58-bit ternary dot product between packed ternary byte array and i16 activations.
/// Uses precomputed 243-entry LUT and dispatches to AVX2 when available. Zero heap allocations.
#[inline]
pub fn ternary_matmul_vector(
    packed_weights: &[u8],
    activations: &[i16],
    scale_permyriad: i32,
) -> Result<i32, S13Error> {
    let num_trits = packed_weights.len() * TRITS_PER_BYTE;
    if activations.len() < num_trits {
        return Err(S13Error::IndexOutOfBounds);
    }

    #[cfg(target_arch = "x86_64")]
    {
        #[allow(unsafe_code)]
        unsafe {
            return avx2_unpacker::matmul_vector_avx2(packed_weights, activations, scale_permyriad);
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        ternary_matmul_vector_scalar(packed_weights, activations, scale_permyriad)
    }
}

/// Scalar fallback for 1.58-bit ternary dot product using fast precomputed 243-entry LUT.
#[inline]
pub fn ternary_matmul_vector_scalar(
    packed_weights: &[u8],
    activations: &[i16],
    scale_permyriad: i32,
) -> Result<i32, S13Error> {
    let num_trits = packed_weights.len() * TRITS_PER_BYTE;
    if activations.len() < num_trits {
        return Err(S13Error::IndexOutOfBounds);
    }

    let mut accum: i64 = 0;
    for (byte_idx, &b) in packed_weights.iter().enumerate() {
        let trits = unpack_5_trits_fast(b)?;
        let act_base = byte_idx * TRITS_PER_BYTE;
        for j in 0..5 {
            let t = trits[j] as i64;
            let a = activations[act_base + j] as i64;
            accum += t * a;
        }
    }

    let scaled = (accum * (scale_permyriad as i64)) / 10_000;
    Ok(scaled as i32)
}

/// Merkle-Morin 64-byte Header Magic Identifier (`b"S13M"`).
pub const S13_MERKLE_MAGIC: [u8; 4] = *b"S13M";

/// Fixed Merkle leaf chunk size in bytes (64 bytes = 320 trits per cache-line leaf).
pub const MERKLE_LEAF_BYTES: usize = 64;

/// Merkle-Morin 64-byte Aligned Binary Header for Cryptographically Verified S13 Weight Matrices.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MerkleMorinHeader {
    /// Magic bytes `b"S13M"`.
    pub magic: [u8; 4],
    /// Binary format version (0x0001).
    pub version: u16,
    /// Header flags.
    pub flags: u16,
    /// Number of matrix rows.
    pub rows: u32,
    /// Number of matrix columns.
    pub cols: u32,
    /// SHA-256 Merkle root hash for all leaf weight blocks.
    pub merkle_root: [u8; 32],
    /// Leaf size in bytes (typically 64 bytes).
    pub leaf_size_bytes: u16,
    /// Fixed-point layer scaling permyriad (e.g. 10_000 = 1.0x).
    pub scale_permyriad: i32,
    /// Out-of-band sentinel threshold boundary (243).
    pub sentinel_boundary: u8,
    /// Reserved zero-padded bytes for 64-byte cache alignment.
    pub _reserved: [u8; 11],
}

impl MerkleMorinHeader {
    /// Create a new Merkle-Morin header.
    pub const fn new(rows: u32, cols: u32, merkle_root: [u8; 32], scale_permyriad: i32) -> Self {
        Self {
            magic: S13_MERKLE_MAGIC,
            version: 1,
            flags: 0,
            rows,
            cols,
            merkle_root,
            leaf_size_bytes: MERKLE_LEAF_BYTES as u16,
            scale_permyriad,
            sentinel_boundary: SENTINEL_THRESHOLD,
            _reserved: [0u8; 11],
        }
    }

    /// Safe serialization of header into a 64-byte array.
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.rows.to_le_bytes());
        out[12..16].copy_from_slice(&self.cols.to_le_bytes());
        out[16..48].copy_from_slice(&self.merkle_root);
        out[48..50].copy_from_slice(&self.leaf_size_bytes.to_le_bytes());
        out[50..54].copy_from_slice(&self.scale_permyriad.to_le_bytes());
        out[54] = self.sentinel_boundary;
        out
    }

    /// Safe deserialization from byte slice.
    pub fn from_bytes(raw_bytes: &[u8]) -> Result<Self, S13Error> {
        if raw_bytes.len() < 64 {
            return Err(S13Error::IndexOutOfBounds);
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&raw_bytes[0..4]);
        if magic != S13_MERKLE_MAGIC {
            return Err(S13Error::InvalidTritValue(-128));
        }
        let version = u16::from_le_bytes([raw_bytes[4], raw_bytes[5]]);
        let flags = u16::from_le_bytes([raw_bytes[6], raw_bytes[7]]);
        let rows = u32::from_le_bytes([raw_bytes[8], raw_bytes[9], raw_bytes[10], raw_bytes[11]]);
        let cols = u32::from_le_bytes([raw_bytes[12], raw_bytes[13], raw_bytes[14], raw_bytes[15]]);
        let mut merkle_root = [0u8; 32];
        merkle_root.copy_from_slice(&raw_bytes[16..48]);
        let leaf_size_bytes = u16::from_le_bytes([raw_bytes[48], raw_bytes[49]]);
        let scale_permyriad = i32::from_le_bytes([raw_bytes[50], raw_bytes[51], raw_bytes[52], raw_bytes[53]]);
        let sentinel_boundary = raw_bytes[54];

        Ok(Self {
            magic,
            version,
            flags,
            rows,
            cols,
            merkle_root,
            leaf_size_bytes,
            scale_permyriad,
            sentinel_boundary,
            _reserved: [0u8; 11],
        })
    }
}

/// Cryptographically Verified Zero-Allocation Merkle-Morin S13 Matrix Container.
#[derive(Debug, Clone)]
pub struct MerkleMorinMatrix<'a> {
    /// Verified 64-byte header.
    pub header: MerkleMorinHeader,
    /// Packed 1.58-bit ternary weights (5 trits per byte).
    pub packed_weights: &'a [u8],
}

impl<'a> MerkleMorinMatrix<'a> {
    /// Zero-copy initialization verifying header magic and dimensions.
    pub fn from_slice(raw_bytes: &'a [u8]) -> Result<Self, S13Error> {
        let header = MerkleMorinHeader::from_bytes(raw_bytes)?;
        let total_weights = (header.rows as usize * header.cols as usize) / TRITS_PER_BYTE;
        let weight_offset = 64;

        if raw_bytes.len() < weight_offset + total_weights {
            return Err(S13Error::IndexOutOfBounds);
        }

        let packed_weights = &raw_bytes[weight_offset..weight_offset + total_weights];
        Ok(Self {
            header,
            packed_weights,
        })
    }

    /// Number of rows.
    #[inline(always)]
    pub fn rows(&self) -> usize {
        self.header.rows as usize
    }

    /// Number of columns.
    #[inline(always)]
    pub fn cols(&self) -> usize {
        self.header.cols as usize
    }

    /// Compute matrix-vector multiplication for a given row against an activation slice.
    #[inline]
    pub fn dot_row(&self, row_idx: usize, activations: &[i16]) -> Result<i32, S13Error> {
        if row_idx >= self.rows() {
            return Err(S13Error::IndexOutOfBounds);
        }
        let row_bytes = self.cols() / TRITS_PER_BYTE;
        let start = row_idx * row_bytes;
        let row_slice = &self.packed_weights[start..start + row_bytes];
        ternary_matmul_vector(row_slice, activations, self.header.scale_permyriad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_243_trit_packings_roundtrip() {
        for b in 0..243u8 {
            let trits = unpack_5_trits(b).expect("Valid ternary byte unpacking");
            let repacked = pack_5_trits(trits).expect("Valid ternary repacking");
            assert_eq!(b, repacked);
        }
    }

    #[test]
    fn test_sentinel_boundary_rejection() {
        for b in 243..=255u8 {
            assert_eq!(unpack_5_trits(b), Err(S13Error::SentinelDetected(b)));
        }
    }

    #[test]
    fn test_coordinate_13_origin_and_equilibrium() {
        let origin = Coordinate13::ORIGIN;
        assert!(origin.is_origin());

        // Origin in balanced ternary corresponds to mid-index (digit '1' for all 13 lanes)
        let origin_idx = origin.to_scalar_index();
        let reconstructed = Coordinate13::from_scalar_index(origin_idx).unwrap();
        assert_eq!(origin, reconstructed);
        assert!(reconstructed.is_origin());
    }

    #[test]
    fn test_coordinate_13_bounds() {
        assert_eq!(
            Coordinate13::from_scalar_index(COORDINATE_13_TOTAL_STATES),
            Err(S13Error::IndexOutOfBounds)
        );
    }

    #[test]
    fn test_ternary_matmul_vector() {
        // 1 byte = 5 trits: [-1, 0, 1, -1, 1]
        let packed = pack_5_trits([-1, 0, 1, -1, 1]).unwrap();
        let activations: [i16; 5] = [100, 200, 300, 400, 500];
        // expected: (-1*100) + (0*200) + (1*300) + (-1*400) + (1*500) = -100 + 0 + 300 - 400 + 500 = 300
        // scale = 10_000 (1.0x) => result = 300
        let res = ternary_matmul_vector(&[packed], &activations, 10_000).unwrap();
        assert_eq!(res, 300);
    }

    #[test]
    fn test_trit_conversions() {
        assert_eq!(Trit::from_i8(-1).unwrap(), Trit::Minus);
        assert_eq!(Trit::from_i8(0).unwrap(), Trit::Zero);
        assert_eq!(Trit::from_i8(1).unwrap(), Trit::Plus);
        assert_eq!(Trit::from_i8(2), Err(S13Error::InvalidTritValue(2)));

        assert_eq!(Trit::Minus.to_radix3(), 0);
        assert_eq!(Trit::Zero.to_radix3(), 1);
        assert_eq!(Trit::Plus.to_radix3(), 2);

        assert_eq!(Trit::from_radix3(0).unwrap(), Trit::Minus);
        assert_eq!(Trit::from_radix3(1).unwrap(), Trit::Zero);
        assert_eq!(Trit::from_radix3(2).unwrap(), Trit::Plus);
        assert_eq!(Trit::Minus.as_i8(), -1);
    }

    #[test]
    fn test_coordinate_13_add_saturating() {
        let a = Coordinate13::new([1; 13]).unwrap();
        let b = Coordinate13::new([1; 13]).unwrap();
        let sum = a.add_saturating(&b);
        assert_eq!(sum.lanes, [1; 13]); // saturates at 1

        let c = Coordinate13::new([-1; 13]).unwrap();
        let sum2 = c.add_saturating(&c);
        assert_eq!(sum2.lanes, [-1; 13]); // saturates at -1
    }

    #[test]
    fn test_coordinate_13_dot_i32() {
        let v = Coordinate13::new([1, -1, 0, 1, -1, 0, 1, -1, 0, 1, -1, 0, 1]).unwrap();
        let weights = [10; 13];
        // 5 * 10 - 4 * 10 = 10
        assert_eq!(v.dot_i32(&weights), 10);
    }

    #[test]
    fn test_unpack_5_trits_fast_matches_slow_unpack() {
        for b in 0..243u8 {
            let slow = unpack_5_trits(b).unwrap();
            let fast = unpack_5_trits_fast(b).unwrap();
            assert_eq!(slow, fast);
        }
        for b in 243..=255u8 {
            assert_eq!(unpack_5_trits_fast(b), Err(S13Error::SentinelDetected(b)));
        }
    }

    #[test]
    fn test_multi_byte_matmul_parity() {
        // Test 5 bytes (25 trits)
        let mut packed = [0u8; 5];
        for i in 0..5 {
            packed[i] = ((i * 47) % 243) as u8;
        }

        let mut activations = [0i16; 25];
        for i in 0..25 {
            activations[i] = ((i as i16) * 73) - 500;
        }

        let scalar_res = ternary_matmul_vector_scalar(&packed, &activations, 10_000).unwrap();
        let main_res = ternary_matmul_vector(&packed, &activations, 10_000).unwrap();
        assert_eq!(scalar_res, main_res);

        #[cfg(target_arch = "x86_64")]
        {
            #[allow(unsafe_code)]
            let avx2_res = unsafe {
                avx2_unpacker::matmul_vector_avx2(&packed, &activations, 10_000).unwrap()
            };
            assert_eq!(scalar_res, avx2_res);
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_pshufb_2trit_unpacking() {
        use core::arch::x86_64::*;
        #[allow(unsafe_code)]
        unsafe {
            // Fill 32 bytes with pair indices 0..8 (representing (t0, t1))
            let mut indices = [0u8; 32];
            for i in 0..32 {
                indices[i] = (i % 9) as u8;
            }
            let idx_vec = _mm256_loadu_si256(indices.as_ptr() as *const __m256i);
            let (high_trits, low_trits) = avx2_unpacker::unpack_2trits_pshufb_avx2(idx_vec);

            let mut high_out = [0i8; 32];
            let mut low_out = [0i8; 32];
            _mm256_storeu_si256(high_out.as_mut_ptr() as *mut __m256i, high_trits);
            _mm256_storeu_si256(low_out.as_mut_ptr() as *mut __m256i, low_trits);

            for i in 0..32 {
                let pair_idx = indices[i];
                let expected_t0 = match pair_idx / 3 {
                    0 => -1,
                    1 => 0,
                    2 => 1,
                    _ => unreachable!(),
                };
                let expected_t1 = match pair_idx % 3 {
                    0 => -1,
                    1 => 0,
                    2 => 1,
                    _ => unreachable!(),
                };
                assert_eq!(high_out[i], expected_t0, "high trit mismatch at index {i}");
                assert_eq!(low_out[i], expected_t1, "low trit mismatch at index {i}");
            }
        }
    }

    #[test]
    fn test_s13_tensor_view_s13m_and_s133() {
        // 1. S13M format (16-byte header, 10 weights -> 2 trit bytes)
        let mut s13m_buf = Vec::new();
        s13m_buf.extend_from_slice(&S13M_MAGIC);
        s13m_buf.extend_from_slice(&2u32.to_le_bytes()); // out_f = 2
        s13m_buf.extend_from_slice(&5u32.to_le_bytes()); // in_f = 5
        s13m_buf.extend_from_slice(&0.5f32.to_le_bytes()); // scale = 0.5
        let b0 = pack_5_trits([1, 0, -1, 1, 0]).unwrap();
        let b1 = pack_5_trits([-1, -1, 1, 1, 0]).unwrap();
        s13m_buf.push(b0);
        s13m_buf.push(b1);

        let view_m = S13TensorView::parse(&s13m_buf).expect("S13M parse");
        assert_eq!(view_m.out_features, 2);
        assert_eq!(view_m.in_features, 5);
        assert_eq!(view_m.scale, 0.5);
        assert!(view_m.group_scales.is_none());
        assert_eq!(view_m.get_trit(0, 0).unwrap(), 1);
        assert_eq!(view_m.get_trit(0, 2).unwrap(), -1);
        assert_eq!(view_m.get_trit(1, 0).unwrap(), -1);

        // 2. S133 format (20-byte header, 64 weights -> 2 scale bytes (1 i16 scale), 13 trit bytes)
        let mut s133_buf = Vec::new();
        s133_buf.extend_from_slice(&S133_MAGIC);
        s133_buf.extend_from_slice(&1u32.to_le_bytes()); // out_f = 1
        s133_buf.extend_from_slice(&64u32.to_le_bytes()); // in_f = 64
        s133_buf.extend_from_slice(&0.25f32.to_le_bytes()); // global scale = 0.25
        s133_buf.extend_from_slice(&64u32.to_le_bytes()); // group_size = 64
        // 2 scale bytes = 1 i16 scale in permyriad: 5000 pmy (= 0.5000)
        s133_buf.extend_from_slice(&5000i16.to_le_bytes());
        // 64 trits -> ceil(64/5) = 13 bytes
        for _ in 0..13 {
            s133_buf.push(b0); // pack_5_trits([1, 0, -1, 1, 0])
        }

        let view_3 = S13TensorView::parse(&s133_buf).expect("S133 parse");
        assert_eq!(view_3.out_features, 1);
        assert_eq!(view_3.in_features, 64);
        assert_eq!(view_3.scale, 0.25);
        assert!(view_3.group_scales.is_some());
        assert_eq!(view_3.get_group_scale_pmy(0).unwrap(), 5000);
        assert_eq!(view_3.get_trit(0, 0).unwrap(), 1);
        // dequantized weight = trit (1) * scale_pmy (0.5) * global_scale (0.25) = 0.125
        let w0 = view_3.get_weight_f32(0, 0).unwrap();
        assert!((w0 - 0.125).abs() < 1e-6);
    }

    /// A 1x64 tensor cannot see either group bug: one row and one group make
    /// `total_weights / 32`, `linear_idx / group_size`, and the real
    /// `[i16; out * ceil(in / group)]` layout all agree by coincidence.
    ///
    /// 2 rows x 96 cols at group 64 separates all three. `ceil(96/64) == 2`, so
    /// the writer lays down `2 * 2 == 4` scales (8 bytes) where the old
    /// `total_weights / 32` claimed 6; and row 1 group 0 is scale index
    /// `1 * 2 + 0 == 2` where the old `linear_idx / 64` claimed
    /// `(1*96 + 0) / 64 == 1`. Both old forms parse without error and read the
    /// wrong cell — which is exactly why this shape has to be pinned.
    #[test]
    fn a_ragged_group_row_pins_the_scale_grid_layout() {
        // every digit +1 -> 2 + 2*3 + 2*9 + 2*27 + 2*81 = 242
        const ALL_PLUS_ONE: u8 = 242;
        let (out_f, in_f, group) = (2usize, 96usize, 64usize);
        let n_groups = in_f.div_ceil(group);
        assert_eq!(n_groups, 2, "the ragged tail must round up to a whole group");

        let mut buf = Vec::new();
        buf.extend_from_slice(&S133_MAGIC);
        buf.extend_from_slice(&(out_f as u32).to_le_bytes());
        buf.extend_from_slice(&(in_f as u32).to_le_bytes());
        buf.extend_from_slice(&1.0f32.to_le_bytes()); // base = identity
        buf.extend_from_slice(&(group as u32).to_le_bytes());
        // [i16; out * n_groups], row-major: r0g0 r0g1 r1g0 r1g1
        for pmy in [1000i16, 2000, 3000, 4000] {
            buf.extend_from_slice(&pmy.to_le_bytes());
        }
        let trit_bytes = (out_f * in_f).div_ceil(5);
        buf.extend(std::iter::repeat(ALL_PLUS_ONE).take(trit_bytes));

        let v = S13TensorView::parse(&buf).expect("ragged S133 must parse");
        assert_eq!(v.group_size, group as u32);
        assert_eq!(
            v.group_scales.expect("S133 carries scales").len(),
            out_f * n_groups * 2,
            "scale segment is out * ceil(in/group) i16s, not total_weights/32"
        );

        // Every trit is +1 and base is 1.0, so the weight IS the group scale.
        assert_eq!(v.get_trit(1, 0).unwrap(), 1);
        for (row, col, want_pmy) in [(0, 0, 1000.0), (0, 64, 2000.0), (1, 0, 3000.0), (1, 64, 4000.0)] {
            let w = v.get_weight_f32(row, col).unwrap();
            let want = want_pmy / 10_000.0;
            assert!(
                (w - want).abs() < 1e-6,
                "({row},{col}) read {w}, want {want} — scale grid is indexed row * n_groups + col / group"
            );
        }
    }

    /// The header's `group_size` is load-bearing, not decorative: a zero would
    /// make `ceil(in / group)` divide by zero, so it is refused at the door.
    #[test]
    fn a_zero_group_size_is_refused_not_divided_by() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&S133_MAGIC);
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&64u32.to_le_bytes());
        buf.extend_from_slice(&1.0f32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // group_size = 0
        buf.extend(std::iter::repeat(0u8).take(64));
        assert!(S13TensorView::parse(&buf).is_err());
    }

    #[test]
    fn test_merkle_morin_matrix_operations() {
        let header = MerkleMorinHeader::new(2, 5, [0xAA; 32], 10_000);
        let header_bytes = header.to_bytes();

        // 2 rows x 5 cols = 10 trits = 2 packed bytes
        let b0 = pack_5_trits([1, 0, -1, 1, 0]).unwrap();
        let b1 = pack_5_trits([-1, -1, 1, 1, 0]).unwrap();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&header_bytes);
        buffer.push(b0);
        buffer.push(b1);

        let matrix = MerkleMorinMatrix::from_slice(&buffer).expect("Valid Merkle-Morin matrix");
        assert_eq!(matrix.rows(), 2);
        assert_eq!(matrix.cols(), 5);

        let activations = [100i16, 200, 300, 400, 500];
        // row 0: [1, 0, -1, 1, 0] . [100, 200, 300, 400, 500] = 100 - 300 + 400 = 200
        let dot0 = matrix.dot_row(0, &activations).unwrap();
        assert_eq!(dot0, 200);

        // row 1: [-1, -1, 1, 1, 0] . [100, 200, 300, 400, 500] = -100 - 200 + 300 + 400 = 400
        let dot1 = matrix.dot_row(1, &activations).unwrap();
        assert_eq!(dot1, 400);

        // Out of bounds row
        assert_eq!(matrix.dot_row(2, &activations), Err(S13Error::IndexOutOfBounds));
    }
}

/// Magic bytes for S13M (unscaled, 16-byte header).
pub const S13M_MAGIC: [u8; 4] = *b"S13M";
/// Magic bytes for S133 (per-(row, input-group) i16 permyriad scales, 20-byte header).
pub const S133_MAGIC: [u8; 4] = *b"S133";

/// Unified zero-copy view of an S13 packed tensor (`S13M` or `S133`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct S13TensorView<'a> {
    /// Number of output features (rows).
    pub out_features: usize,
    /// Number of input features (columns).
    pub in_features: usize,
    /// Global per-tensor float scale.
    pub scale: f32,
    /// Input columns covered by one scale, read from the header (64 on every
    /// seat on disk; 0 for `S13M`, which carries no groups).
    pub group_size: u32,
    /// Optional per-group scale segment for `S133`: the raw bytes of
    /// `[i16 LE; out_features * ceil(in_features / group_size)]` permyriad
    /// scales, row-major over the group grid. Decode via
    /// [`Self::get_group_scale_pmy`] — never read these bytes directly.
    pub group_scales: Option<&'a [u8]>,
    /// Base-243 packed trit bytes (`(out*in+4)/5` bytes).
    pub packed_trits: &'a [u8],
}

impl<'a> S13TensorView<'a> {
    /// Parse and validate an S13 tensor from bytes (`S13M` or `S133`).
    pub fn parse(bytes: &'a [u8]) -> Result<Self, &'static str> {
        if bytes.len() < 16 {
            return Err("S13 buffer too short for header");
        }

        let magic = &bytes[0..4];
        if magic == S13M_MAGIC {
            let out_features = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
            let in_features = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
            let scale = f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
            let total_weights = out_features * in_features;
            let expected_trits = total_weights.div_ceil(5);

            if bytes.len() < 16 + expected_trits {
                return Err("S13M buffer truncated: missing trit payload");
            }

            Ok(Self {
                out_features,
                in_features,
                scale,
                group_size: 0,
                group_scales: None,
                packed_trits: &bytes[16..16 + expected_trits],
            })
        } else if magic == S133_MAGIC {
            if bytes.len() < 20 {
                return Err("S133 buffer too short for 20-byte header");
            }
            let out_features = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
            let in_features = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
            let scale = f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
            let group_size = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

            if group_size == 0 {
                return Err("S133 header declares group_size 0");
            }
            let total_weights = out_features * in_features;
            // Derived from the header, never hardcoded: the writer lays down
            // [i16 LE; out * ceil(in / group)]. `total_weights / 32` gives the
            // same count only while `group == 64` divides `in_features`.
            let n_groups = in_features.div_ceil(group_size as usize);
            let scale_bytes = out_features * n_groups * 2;
            let expected_trits = total_weights.div_ceil(5);

            let trits_offset = 20 + scale_bytes;
            if bytes.len() < trits_offset + expected_trits {
                return Err("S133 buffer truncated: missing scale or trit payload");
            }

            Ok(Self {
                out_features,
                in_features,
                scale,
                group_size,
                group_scales: Some(&bytes[20..trits_offset]),
                packed_trits: &bytes[trits_offset..trits_offset + expected_trits],
            })
        } else {
            Err("Unrecognized S13 magic: expected S13M or S133")
        }
    }

    /// Retrieve the trit value at `(row, col)`.
    #[inline(always)]
    pub fn get_trit(&self, row: usize, col: usize) -> Result<i8, S13Error> {
        if row >= self.out_features || col >= self.in_features {
            return Err(S13Error::IndexOutOfBounds);
        }
        let linear_idx = row * self.in_features + col;
        let byte_idx = linear_idx / 5;
        let trit_idx = linear_idx % 5;

        let packed_byte = self.packed_trits[byte_idx];
        let trits = unpack_5_trits(packed_byte)?;
        Ok(trits[trit_idx])
    }

    /// Retrieve the `i16` permyriad group scale for a given group index.
    ///
    /// Value-space invariant: Valid scale values must strictly land in `1..=10_000`.
    #[inline(always)]
    pub fn get_group_scale_pmy(&self, group_idx: usize) -> Result<i16, S13Error> {
        if let Some(scales) = self.group_scales {
            let offset = group_idx * 2;
            if offset + 2 > scales.len() {
                return Err(S13Error::IndexOutOfBounds);
            }
            let scale_pmy = i16::from_le_bytes([scales[offset], scales[offset + 1]]);
            if !(1..=10_000).contains(&scale_pmy) {
                // Value-space corruption detected
                return Err(S13Error::InvalidTritValue(-128));
            }
            Ok(scale_pmy)
        } else {
            Ok(10_000) // S13M unscaled identity path (10,000 pmy = 1.0)
        }
    }

    /// Retrieve the dequantized float32 weight at `(row, col)`.
    #[inline(always)]
    pub fn get_weight_f32(&self, row: usize, col: usize) -> Result<f32, S13Error> {
        let trit = self.get_trit(row, col)? as f32;
        if trit == 0.0 {
            return Ok(0.0);
        }
        let group_size = if self.group_size == 0 { 64 } else { self.group_size as usize };

        // Group index address trap. `group_in_row` is the TritCell5D interior
        // address: 243 states (atom.rs), and the widest tensor on disk is the
        // 9B ffn_down at in=14336 -> 224 groups, so every legal index is
        // interior and anything at or past 243 is a sentinel by construction.
        let group_in_row = col / group_size;
        if group_in_row >= 243 {
            return Err(S13Error::SentinelDetected(group_in_row as u8));
        }

        // Row-major over the group grid, matching the writer's
        // [i16 LE; out * n_groups] layout. `linear_idx / group_size` coincides
        // with this only while `group_size` divides `in_features`.
        let n_groups = self.in_features.div_ceil(group_size);
        let group_idx = row * n_groups + group_in_row;

        let scale_pmy = self.get_group_scale_pmy(group_idx)? as f32 / 10_000.0;
        Ok(trit * scale_pmy * self.scale)
    }
}

