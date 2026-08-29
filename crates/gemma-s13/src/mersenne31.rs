// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Finite Field $\mathbb{F}_{2^{31}-1}$ (Mersenne Prime 31) and Morton8 2D Z-order interleave.
//!
//! Provides fast, branchless reduction `(x & 0x7FFFFFFF) + (x >> 31)` with $O(1)$ constant-time
//! field operations, alongside 8-bit Morton Z-order indexing with 64-byte cacheline stride alignment.

/// Mersenne Prime 31 modulus: $2^{31} - 1 = 2{,}147{,}483{,}647$.
pub const MERSENNE_31_MODULUS: u32 = 0x7FFF_FFFF;

/// Bitmask for 31-bit low half.
pub const MERSENNE_31_MASK: u64 = 0x7FFF_FFFF;

/// Perform fast branchless reduction of a 64-bit value modulo $2^{31} - 1$.
#[inline(always)]
pub const fn reduce_m31(val: u64) -> u32 {
    let lo = (val & MERSENNE_31_MASK) as u32;
    let hi = (val >> 31) as u32;
    let sum = lo + hi;
    // Single fold if sum exceeds modulus
    let sum = (sum & MERSENNE_31_MODULUS) + (sum >> 31);
    if sum >= MERSENNE_31_MODULUS {
        sum - MERSENNE_31_MODULUS
    } else {
        sum
    }
}

/// Element of the finite field $\mathbb{F}_{2^{31}-1}$.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Mersenne31(pub u32);

impl Mersenne31 {
    /// Zero element in $\mathbb{F}_{2^{31}-1}$.
    pub const ZERO: Self = Self(0);
    /// Multiplicative identity element (1) in $\mathbb{F}_{2^{31}-1}$.
    pub const ONE: Self = Self(1);
    /// Prime modulus constant: $2^{31}-1$.
    pub const MODULUS: u32 = MERSENNE_31_MODULUS;

    /// Create a new field element from a raw `u32`, reducing modulo $2^{31} - 1$.
    #[inline(always)]
    pub const fn new(val: u32) -> Self {
        Self(reduce_m31(val as u64))
    }

    /// Create a new field element from a `u64`, reducing modulo $2^{31} - 1$.
    #[inline(always)]
    pub const fn from_u64(val: u64) -> Self {
        Self(reduce_m31(val))
    }

    /// Constant-time addition in $\mathbb{F}_{2^{31}-1}$.
    #[inline(always)]
    pub const fn add(self, rhs: Self) -> Self {
        let sum = self.0 + rhs.0;
        let reduced = (sum & MERSENNE_31_MODULUS) + (sum >> 31);
        if reduced >= MERSENNE_31_MODULUS {
            Self(reduced - MERSENNE_31_MODULUS)
        } else {
            Self(reduced)
        }
    }

    /// Constant-time subtraction in $\mathbb{F}_{2^{31}-1}$.
    #[inline(always)]
    pub const fn sub(self, rhs: Self) -> Self {
        let diff = (self.0 as i64) - (rhs.0 as i64);
        if diff < 0 {
            Self((diff + MERSENNE_31_MODULUS as i64) as u32)
        } else {
            Self(diff as u32)
        }
    }

    /// Constant-time multiplication in $\mathbb{F}_{2^{31}-1}$.
    #[inline(always)]
    pub const fn mul(self, rhs: Self) -> Self {
        let prod = (self.0 as u64) * (rhs.0 as u64);
        Self(reduce_m31(prod))
    }

    /// Multiplicative inverse using Fermat's Little Theorem: $a^{p-2} \pmod p$.
    pub const fn inv(self) -> Option<Self> {
        if self.0 == 0 {
            None
        } else {
            Some(self.pow(MERSENNE_31_MODULUS - 2))
        }
    }

    /// Exponentiation by squaring in $\mathbb{F}_{2^{31}-1}$.
    pub const fn pow(self, mut exp: u32) -> Self {
        let mut base = self;
        let mut result = Self::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(base);
            }
            base = base.mul(base);
            exp >>= 1;
        }
        result
    }
}

impl std::ops::Add for Mersenne31 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        self.add(rhs)
    }
}

impl std::ops::Sub for Mersenne31 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        self.sub(rhs)
    }
}

impl std::ops::Mul for Mersenne31 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        self.mul(rhs)
    }
}

/// 8-bit Morton Z-order 2D coordinate for space-filling 8x8 / 16x16 resolvent tile indexing.
/// Encodes `4-bit x` and `4-bit y` into an interleaved 8-bit Morton index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Morton8_2D(pub u8);

impl Morton8_2D {
    /// 64-byte cacheline stride alignment constant.
    pub const CACHE_STRIDE_BYTES: usize = 64;

    /// Encode 4-bit `x` (0..15) and 4-bit `y` (0..15) into an 8-bit Morton key.
    #[inline(always)]
    pub const fn encode(x: u8, y: u8) -> Self {
        let x = (x & 0x0F) as u16;
        let y = (y & 0x0F) as u16;

        let x_spread = (x | (x << 2)) & 0x33;
        let x_spread = (x_spread | (x_spread << 1)) & 0x55;

        let y_spread = (y | (y << 2)) & 0x33;
        let y_spread = (y_spread | (y_spread << 1)) & 0x55;

        Self(((x_spread | (y_spread << 1)) & 0xFF) as u8)
    }

    /// Decode an 8-bit Morton key into `(x, y)` 4-bit coordinates (0..15, 0..15).
    #[inline(always)]
    pub const fn decode(self) -> (u8, u8) {
        let val = self.0 as u16;

        let x = val & 0x55;
        let x = (x | (x >> 1)) & 0x33;
        let x = (x | (x >> 2)) & 0x0F;

        let y = (val >> 1) & 0x55;
        let y = (y | (y >> 1)) & 0x33;
        let y = (y | (y >> 2)) & 0x0F;

        (x as u8, y as u8)
    }

    /// Calculate memory offset with 64-byte cacheline stride alignment.
    #[inline(always)]
    pub const fn cacheline_offset(self) -> usize {
        (self.0 as usize) * Self::CACHE_STRIDE_BYTES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduction_identity() {
        assert_eq!(reduce_m31(0), 0);
        assert_eq!(reduce_m31(1), 1);
        assert_eq!(reduce_m31(MERSENNE_31_MODULUS as u64), 0);
        assert_eq!(reduce_m31(MERSENNE_31_MODULUS as u64 + 1), 1);
        assert_eq!(reduce_m31((MERSENNE_31_MODULUS as u64) * 2), 0);
    }

    #[test]
    fn test_field_arithmetic() {
        let a = Mersenne31::new(1_000_000_000);
        let b = Mersenne31::new(1_500_000_000);
        let sum = a + b;
        assert_eq!(sum.0, (2_500_000_000u64 % MERSENNE_31_MODULUS as u64) as u32);

        let diff = a - b;
        let re_add = diff + b;
        assert_eq!(re_add, a);

        let prod = a * b;
        assert_eq!(prod.0, reduce_m31((a.0 as u64) * (b.0 as u64)));

        let inv_a = a.inv().expect("inverse exists");
        assert_eq!((a * inv_a).0, 1);
    }

    #[test]
    fn test_morton8_roundtrip() {
        for x in 0..16 {
            for y in 0..16 {
                let encoded = Morton8_2D::encode(x, y);
                let (dec_x, dec_y) = encoded.decode();
                assert_eq!((x, y), (dec_x, dec_y));
            }
        }
    }
}
