//! Tilemap — an integer tile grid (level composition, the tilemap ForgeAtom
//! family). Tiles are u16 ids; bounded and resolution-independent.

use serde::{Deserialize, Serialize};

/// A `w x h` grid of tile ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tilemap {
    /// Width of the tilemap in tiles.
    pub w: u32,
    /// Height of the tilemap in tiles.
    pub h: u32,
    /// Tile IDs stored in row-major order: index = `y * w + x`.
    pub tiles: Vec<u16>,
}

impl Tilemap {
    /// Create a new tilemap with dimensions `w` x `h`, initialized with zeros.
    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h, tiles: vec![0; (w * h) as usize] }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }

    /// Retrieve the tile ID at coordinates `(x, y)`, or `None` if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> Option<u16> {
        self.index(x, y).map(|i| self.tiles[i])
    }

    /// Set the tile ID at coordinates `(x, y)` to `id`; silently ignores out-of-bounds writes.
    pub fn set(&mut self, x: u32, y: u32, id: u16) {
        if let Some(i) = self.index(x, y) {
            self.tiles[i] = id;
        }
    }

    /// Set all tiles to the given ID.
    pub fn fill(&mut self, id: u16) {
        for t in &mut self.tiles {
            *t = id;
        }
    }

    /// How many cells hold `id`.
    pub fn count(&self, id: u16) -> usize {
        self.tiles.iter().filter(|t| **t == id).count()
    }

    /// Total number of tiles in the grid.
    pub fn area(&self) -> usize {
        self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut m = Tilemap::new(4, 3);
        assert_eq!(m.area(), 12);
        m.set(2, 1, 7);
        assert_eq!(m.get(2, 1), Some(7));
        assert_eq!(m.get(9, 9), None);
    }

    #[test]
    fn fill_and_count() {
        let mut m = Tilemap::new(3, 3);
        m.fill(5);
        assert_eq!(m.count(5), 9);
        m.set(0, 0, 2);
        assert_eq!(m.count(5), 8);
        assert_eq!(m.count(2), 1);
    }
}
