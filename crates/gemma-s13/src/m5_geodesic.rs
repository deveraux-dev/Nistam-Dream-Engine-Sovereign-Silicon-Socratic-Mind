// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! 5D Discretized Geodesic Metric Tensor Coordinate Kernel.
//!
//! Implements branchless O(1) 5D spatial lookups via ternary lattice coordinates.
//! Maps workspace objects into 5-element discrete tuples (x1, x2, x3, x4, x5),
//! with each axis holding ternary values {-1, 0, +1} for 3^5 = 243 total states.

use crate::s13::S13Error;

/// 5D Manifold Ternary Lattice Coordinate.
/// Each of the 5 axes is a balanced ternary value in {-1, 0, +1}.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M5Coordinate {
    /// 5-element ternary coordinate vector: (x1, x2, x3, x4, x5)
    pub axes: [i8; 5],
}

impl M5Coordinate {
    /// The absolute physical drift-free equilibrium origin (all axes zero).
    pub const ORIGIN: Self = Self {
        axes: [0, 0, 0, 0, 0],
    };

    /// Total number of states in a 5D ternary lattice (3^5).
    pub const TOTAL_STATES: u16 = 243;

    /// Create a new 5D coordinate, validating each axis is in {-1, 0, +1}.
    pub const fn new(axes: [i8; 5]) -> Result<Self, S13Error> {
        let mut i = 0;
        while i < 5 {
            let a = axes[i];
            if a < -1 || a > 1 {
                return Err(S13Error::InvalidTritValue(a));
            }
            i += 1;
        }
        Ok(Self { axes })
    }

    /// Check if this coordinate is precisely at the equilibrium origin.
    #[inline(always)]
    pub const fn is_origin(&self) -> bool {
        self.axes[0] == 0
            && self.axes[1] == 0
            && self.axes[2] == 0
            && self.axes[3] == 0
            && self.axes[4] == 0
    }

    /// Compute the scalar integer index in 0..243 (branchless O(1) array indexing).
    /// Maps balanced ternary (-1, 0, +1) to radix-3 (0, 1, 2).
    #[inline(always)]
    pub const fn to_scalar_index(&self) -> u8 {
        let d0 = match self.axes[0] {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => 1u16,
        };
        let d1 = match self.axes[1] {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => 1u16,
        };
        let d2 = match self.axes[2] {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => 1u16,
        };
        let d3 = match self.axes[3] {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => 1u16,
        };
        let d4 = match self.axes[4] {
            -1 => 0u16,
            0 => 1u16,
            1 => 2u16,
            _ => 1u16,
        };
        ((((d0 * 3 + d1) * 3 + d2) * 3 + d3) * 3 + d4) as u8
    }

    /// Reconstruct a 5D coordinate from a scalar index (0..243).
    pub const fn from_scalar_index(mut idx: u8) -> Result<Self, S13Error> {
        if idx >= Self::TOTAL_STATES as u8 {
            return Err(S13Error::IndexOutOfBounds);
        }
        let mut axes = [0i8; 5];
        let mut i = 4;
        loop {
            let digit = (idx % 3) as u8;
            idx /= 3;
            axes[i] = match digit {
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
        Ok(Self { axes })
    }

    /// Chebyshev distance (max absolute difference) in the 5D lattice.
    #[inline(always)]
    pub const fn chebyshev_distance(&self, other: &Self) -> u8 {
        let mut max_diff = 0i8;
        let mut i = 0;
        while i < 5 {
            let diff = self.axes[i] - other.axes[i];
            let abs_diff = if diff < 0 { -diff } else { diff };
            if abs_diff > max_diff {
                max_diff = abs_diff;
            }
            i += 1;
        }
        max_diff as u8
    }

    /// Manhattan distance (sum of absolute differences) in the 5D lattice.
    /// Zero-allocation SIMD-ready kernel: stack-resident 5-byte integer subtraction.
    #[inline(always)]
    pub fn manhattan_distance(&self, other: &Self) -> u8 {
        self.axes
            .iter()
            .zip(other.axes.iter())
            .map(|(&x, &y)| (x - y).unsigned_abs())
            .sum()
    }

    /// Vector addition with saturation at [-1, 1].
    #[inline]
    pub const fn add_saturating(&self, rhs: &Self) -> Self {
        let mut out = [0i8; 5];
        let mut i = 0;
        while i < 5 {
            let sum = self.axes[i] + rhs.axes[i];
            out[i] = if sum > 1 {
                1
            } else if sum < -1 {
                -1
            } else {
                sum
            };
            i += 1;
        }
        Self { axes: out }
    }
}

/// Geodesic metric tensor lookup table for the 5D ternary manifold.
/// Stores precomputed shortest-path distances from a fixed reference point.
pub struct M5GeodesicLookup {
    /// Distance table indexed by scalar M5 coordinate (0..243).
    distances: [u8; 243],
}

impl M5GeodesicLookup {
    /// Create a new geodesic lookup table initialized to zero.
    pub const fn new() -> Self {
        Self {
            distances: [0u8; 243],
        }
    }

    /// Build geodesic distances from a reference coordinate using breadth-first traversal.
    pub fn build_from_origin(&mut self) {
        let origin = M5Coordinate::ORIGIN;
        self.build_from_coordinate(&origin);
    }

    /// Build geodesic distances from an arbitrary coordinate.
    /// Uses Manhattan distance (taxicab metric) on the 5D ternary lattice.
    pub fn build_from_coordinate(&mut self, reference: &M5Coordinate) {
        self.distances = [0u8; 243];
        for idx in 0..243u8 {
            if let Ok(coord) = M5Coordinate::from_scalar_index(idx) {
                self.distances[idx as usize] = reference.manhattan_distance(&coord);
            }
        }
    }

    /// Query the geodesic distance to a coordinate (O(1) lookup).
    #[inline(always)]
    pub fn query(&self, coord: &M5Coordinate) -> u8 {
        self.distances[coord.to_scalar_index() as usize]
    }

    /// Count reachable coordinates within a maximum distance.
    pub fn count_neighbors_within(&self, max_distance: u8) -> usize {
        self.distances.iter()
            .filter(|&&dist| dist <= max_distance && dist != u8::MAX)
            .count()
    }
}

impl Default for M5GeodesicLookup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m5_origin_equilibrium() {
        let origin = M5Coordinate::ORIGIN;
        assert!(origin.is_origin());
        assert_eq!(origin.to_scalar_index(), 121u8);
    }

    #[test]
    fn test_m5_all_243_coordinates_roundtrip() {
        for idx in 0..243u8 {
            let coord = M5Coordinate::from_scalar_index(idx).expect("Valid M5 index");
            let repacked = coord.to_scalar_index();
            assert_eq!(idx, repacked, "Roundtrip failed for index {}", idx);
        }
    }

    #[test]
    fn test_m5_coordinate_bounds() {
        assert_eq!(
            M5Coordinate::from_scalar_index(243),
            Err(S13Error::IndexOutOfBounds)
        );
        assert_eq!(
            M5Coordinate::new([2, 0, 0, 0, 0]),
            Err(S13Error::InvalidTritValue(2))
        );
    }

    #[test]
    fn test_m5_add_saturating() {
        let a = M5Coordinate::new([1, 0, -1, 1, 0]).unwrap();
        let b = M5Coordinate::new([1, 0, 1, -1, 0]).unwrap();
        let sum = a.add_saturating(&b);
        assert_eq!(sum.axes, [1, 0, 0, 0, 0]);

        let c = M5Coordinate::new([1, 1, 1, 1, 1]).unwrap();
        let d = M5Coordinate::new([1, 1, 1, 1, 1]).unwrap();
        let sum2 = c.add_saturating(&d);
        assert_eq!(sum2.axes, [1, 1, 1, 1, 1]);
    }

    #[test]
    fn test_m5_chebyshev_distance() {
        let origin = M5Coordinate::ORIGIN;
        let a = M5Coordinate::new([1, 0, 0, 0, 0]).unwrap();
        let b = M5Coordinate::new([1, 1, 1, 0, 0]).unwrap();
        let c = M5Coordinate::new([-1, -1, -1, -1, -1]).unwrap();

        assert_eq!(origin.chebyshev_distance(&a), 1);
        assert_eq!(origin.chebyshev_distance(&b), 1);
        assert_eq!(origin.chebyshev_distance(&c), 1);
    }

    #[test]
    fn test_m5_manhattan_distance() {
        let origin = M5Coordinate::ORIGIN;
        let a = M5Coordinate::new([1, 0, 0, 0, 0]).unwrap();
        let b = M5Coordinate::new([1, 1, 1, 0, 0]).unwrap();
        let c = M5Coordinate::new([-1, -1, -1, -1, -1]).unwrap();

        assert_eq!(origin.manhattan_distance(&a), 1);
        assert_eq!(origin.manhattan_distance(&b), 3);
        assert_eq!(origin.manhattan_distance(&c), 5);
    }

    #[test]
    fn test_m5_geodesic_lookup_from_origin() {
        let mut lookup = M5GeodesicLookup::new();
        lookup.build_from_origin();

        let origin = M5Coordinate::ORIGIN;
        assert_eq!(lookup.query(&origin), 0);

        let neighbor = M5Coordinate::new([1, 0, 0, 0, 0]).unwrap();
        assert_eq!(lookup.query(&neighbor), 1);

        let far = M5Coordinate::new([-1, -1, -1, -1, -1]).unwrap();
        assert_eq!(lookup.query(&far), 5);
    }

    #[test]
    fn test_m5_neighbors_within() {
        let mut lookup = M5GeodesicLookup::new();
        lookup.build_from_origin();

        let neighbors_1 = lookup.count_neighbors_within(1);
        assert_eq!(neighbors_1, 11);

        let neighbors_2 = lookup.count_neighbors_within(2);
        assert!(neighbors_2 > neighbors_1);
    }
}
