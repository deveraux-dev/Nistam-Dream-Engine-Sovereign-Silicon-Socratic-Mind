//! Hexgrid — axial hex coordinates with integer distance and neighbours. For
//! hex-tile maps; no float.

use serde::{Deserialize, Serialize};

/// An axial hex coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hex {
    /// The q-coordinate in axial hex coordinates.
    pub q: i32,
    /// The r-coordinate in axial hex coordinates.
    pub r: i32,
}

impl Hex {
    /// Constructs a new hex at the given axial coordinates.
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// The implicit cube s-coordinate (q + r + s = 0).
    pub fn s(&self) -> i32 {
        -self.q - self.r
    }

    /// Hex distance (axial): the number of steps between the two hexes.
    pub fn distance(&self, other: &Hex) -> i32 {
        ((self.q - other.q).abs() + (self.r - other.r).abs() + (self.s() - other.s()).abs()) / 2
    }

    /// The six neighbours, clockwise from east.
    pub fn neighbors(&self) -> [Hex; 6] {
        const DIRS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
        DIRS.map(|(dq, dr)| Hex::new(self.q + dq, self.r + dr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_symmetric_and_zero_on_self() {
        let a = Hex::new(0, 0);
        let b = Hex::new(2, -1);
        assert_eq!(a.distance(&a), 0);
        assert_eq!(a.distance(&b), b.distance(&a));
        assert_eq!(a.distance(&b), 2);
    }

    #[test]
    fn neighbors_are_distance_one() {
        let h = Hex::new(3, -2);
        for n in h.neighbors() {
            assert_eq!(h.distance(&n), 1);
        }
    }
}
