//! From F:\NewRepo\crates\forge-vision\src\poll5d\contact.rs (lines 1-134)
//! Frame to 5D contacts: changed tiles become positioned points.

use crate::poll5d::spatial::P5;
use crate::visual_debug::{compute_tile_hashes, detect_changes_cpu, TileHash};

/// Holds previous frame tile hashes for diffing successive frames.
pub struct ContactExtractor {
    tile: usize,
    prev: Vec<TileHash>,
    tiles_x: usize,
    tiles_y: usize,
    last_changed: usize,
}

impl ContactExtractor {
    /// Create a new contact extractor with tile size.
    pub fn new(tile_size: usize) -> Self {
        Self {
            tile: tile_size.max(1),
            prev: Vec::new(),
            tiles_x: 0,
            tiles_y: 0,
            last_changed: 0,
        }
    }

    /// Number of changed tiles from the most recent extraction.
    pub fn last_changed(&self) -> usize {
        self.last_changed
    }

    /// Grid dimensions (tiles_x, tiles_y).
    pub fn grid(&self) -> (usize, usize) {
        (self.tiles_x, self.tiles_y)
    }

    /// Diff this frame against the last and return changed tiles as positioned 5D contacts.
    pub fn extract(
        &mut self,
        rgba: &[u8],
        w: u32,
        h: u32,
        tick: u64,
        z: i32,
        s: u32,
    ) -> Vec<(P5, u32)> {
        let (wu, hu) = (w as usize, h as usize);
        let curr = compute_tile_hashes(rgba, wu, hu, self.tile);
        self.tiles_x = wu.div_ceil(self.tile);
        self.tiles_y = hu.div_ceil(self.tile);

        if self.prev.len() != curr.len() || self.prev.is_empty() {
            self.prev = curr;
            self.last_changed = 0;
            return Vec::new();
        }

        let changed = detect_changes_cpu(&self.prev, &curr);
        self.last_changed = changed.len();
        let tiles_x = self.tiles_x.max(1);
        let contacts = changed
            .into_iter()
            .map(|i| {
                let tx = (i % tiles_x) as i32;
                let ty = (i / tiles_x) as i32;
                (P5::new(tx, ty, z, tick, s), 1u32)
            })
            .collect();
        self.prev = curr;
        contacts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, c: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity(w * h * 4);
        for _ in 0..(w * h) {
            v.extend_from_slice(&c);
        }
        v
    }

    #[test]
    fn first_frame_primes_no_contacts() {
        let mut ex = ContactExtractor::new(8);
        let f = solid(32, 32, [10, 20, 30, 255]);
        let c = ex.extract(&f, 32, 32, 1, 0, 0);
        assert!(c.is_empty(), "priming frame emits no contacts");
        assert_eq!(ex.last_changed(), 0);
    }

    #[test]
    fn static_scene_yields_nothing() {
        let mut ex = ContactExtractor::new(8);
        let f = solid(32, 32, [10, 20, 30, 255]);
        ex.extract(&f, 32, 32, 1, 0, 0);
        let c = ex.extract(&f, 32, 32, 2, 0, 0);
        assert!(c.is_empty(), "no motion → no contacts");
    }

    #[test]
    fn changed_tile_becomes_positioned_contact() {
        let mut ex = ContactExtractor::new(8);
        let f0 = solid(32, 32, [0, 0, 0, 255]);
        ex.extract(&f0, 32, 32, 1, 0, 0);

        let mut f1 = f0.clone();
        for y in 24..32usize {
            for x in 24..32usize {
                let i = (y * 32 + x) * 4;
                f1[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let c = ex.extract(&f1, 32, 32, 2, 7, 3);
        assert!(!c.is_empty(), "a changed region produces contacts");
        let (p, w) = c[0];
        assert_eq!(p.t, 2);
        assert_eq!(p.z, 7);
        assert_eq!(p.s, 3);
        assert_eq!(w, 1);
        assert!(p.x >= 2 && p.y >= 2, "contact lands in the bottom-right quadrant");
    }
}
