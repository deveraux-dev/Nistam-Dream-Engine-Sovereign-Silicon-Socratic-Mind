//! Region-pack — shelf-pack sprite regions into an atlas of a fixed width, row
//! by row. Integer bin-packing; deterministic order.

use serde::{Deserialize, Serialize};

/// A placed region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed {
    /// Index of the region in the input list.
    pub id: usize,
    /// X coordinate in the atlas.
    pub x: u32,
    /// Y coordinate in the atlas.
    pub y: u32,
    /// Width of the region.
    pub w: u32,
    /// Height of the region.
    pub h: u32,
}

/// Shelf-pack `sizes` (w, h) into rows no wider than `atlas_w`.
pub fn pack(sizes: &[(u32, u32)], atlas_w: u32) -> Vec<Placed> {
    let mut out = Vec::with_capacity(sizes.len());
    let (mut x, mut y, mut shelf_h) = (0u32, 0u32, 0u32);
    for (id, &(w, h)) in sizes.iter().enumerate() {
        if x + w > atlas_w && x > 0 {
            x = 0;
            y += shelf_h;
            shelf_h = 0;
        }
        out.push(Placed { id, x, y, w, h });
        x += w;
        shelf_h = shelf_h.max(h);
    }
    out
}

/// The atlas height needed for a packing.
pub fn atlas_height(placed: &[Placed]) -> u32 {
    placed.iter().map(|p| p.y + p.h).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_into_rows() {
        let sizes = [(40, 20), (40, 30), (40, 10)]; // atlas_w 80 fits 2 per row
        let placed = pack(&sizes, 80);
        assert_eq!(placed[0], Placed { id: 0, x: 0, y: 0, w: 40, h: 20 });
        assert_eq!(placed[1], Placed { id: 1, x: 40, y: 0, w: 40, h: 30 });
        // third wraps to a new shelf at y = 30 (tallest of first row)
        assert_eq!(placed[2], Placed { id: 2, x: 0, y: 30, w: 40, h: 10 });
        assert_eq!(atlas_height(&placed), 40);
    }

    #[test]
    fn oversized_still_places() {
        let placed = pack(&[(200, 50)], 80); // wider than atlas; placed at origin
        assert_eq!(placed[0].x, 0);
        assert_eq!(atlas_height(&placed), 50);
    }
}
