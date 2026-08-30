//! Vision — a tile-scan grid: an image as a grid of tile hashes, diffed for
//! delta detection (harvested from forge-vision / forge-gpu-ops tile-hash).

use serde::{Deserialize, Serialize};

/// A grid of per-tile hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGrid {
    /// Number of tile columns in this grid.
    pub cols: u32,
    /// Number of tile rows in this grid.
    pub rows: u32,
    /// Per-tile hash values in row-major order.
    pub hashes: Vec<u64>,
}

impl TileGrid {
    /// Create a new grid with all hashes zeroed.
    pub fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows, hashes: vec![0; (cols * rows) as usize] }
    }

    fn index(&self, col: u32, row: u32) -> Option<usize> {
        if col < self.cols && row < self.rows {
            Some((row * self.cols + col) as usize)
        } else {
            None
        }
    }

    /// Set the hash for a tile at the given grid position.
    pub fn set(&mut self, col: u32, row: u32, hash: u64) {
        if let Some(i) = self.index(col, row) {
            self.hashes[i] = hash;
        }
    }

    /// Retrieve the hash for a tile at the given grid position.
    pub fn get(&self, col: u32, row: u32) -> Option<u64> {
        self.index(col, row).map(|i| self.hashes[i])
    }

    /// Tiles whose hash differs from `other` (same-shape grids only).
    pub fn delta(&self, other: &TileGrid) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        if self.cols != other.cols || self.rows != other.rows {
            return out;
        }
        for row in 0..self.rows {
            for col in 0..self.cols {
                if self.get(col, row) != other.get(col, row) {
                    out.push((col, row));
                }
            }
        }
        out
    }

    /// Total number of tiles in this grid.
    pub fn tile_count(&self) -> usize {
        self.hashes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_finds_changed_tiles() {
        let mut a = TileGrid::new(3, 2);
        let mut b = TileGrid::new(3, 2);
        a.set(0, 0, 111);
        b.set(0, 0, 111);
        b.set(2, 1, 999); // changed
        let d = a.delta(&b);
        assert_eq!(d, vec![(2, 1)]);
    }

    #[test]
    fn identical_grids_have_no_delta() {
        let a = TileGrid::new(4, 4);
        let b = TileGrid::new(4, 4);
        assert!(a.delta(&b).is_empty());
        assert_eq!(a.tile_count(), 16);
    }

    #[test]
    fn mismatched_shapes_yield_empty() {
        let a = TileGrid::new(2, 2);
        let b = TileGrid::new(3, 3);
        assert!(a.delta(&b).is_empty());
    }
}
