//! From F:\NewRepo\crates\forge-vision\src\inlined_types.rs (lines 1-190)
//! Core types for frame capture and compression.

use serde::{Deserialize, Serialize};

/// A raw vision frame buffer (BGRA, 8 bits per channel).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionFrame {
    pub width: u32,
    pub height: u32,
    /// Row-major BGRA pixels. Length = width * height * 4.
    pub data: Vec<u8>,
}

impl VisionFrame {
    /// Create a new frame with the given dimensions and pixel data.
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        debug_assert_eq!(data.len(), (width as usize) * (height as usize) * 4);
        Self { width, height, data }
    }

    /// Create a zeroed frame (black).
    pub fn zeroed(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; (width as usize) * (height as usize) * 4],
        }
    }

    /// Number of tiles in a grid of tile_size x tile_size.
    pub fn tile_count(&self, tile_size: u32) -> (u32, u32) {
        let cols = (self.width + tile_size - 1) / tile_size;
        let rows = (self.height + tile_size - 1) / tile_size;
        (cols, rows)
    }
}

/// A rectangular region within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A tile hash for change detection.
/// Each tile is tile_size x tile_size pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileHash {
    pub col: u16,
    pub row: u16,
    pub hash: u64,
}

/// Result of comparing two frames via tile hashing.
#[derive(Debug, Clone)]
pub struct DeltaResult {
    pub tile_size: u32,
    pub cols: u32,
    pub rows: u32,
    /// Indices of tiles that changed between frames.
    pub changed_indices: Vec<u32>,
    /// Hashes of the current frame's tiles.
    pub current_hashes: Vec<u64>,
}

impl DeltaResult {
    /// Fraction of tiles that changed.
    pub fn change_ratio(&self) -> f32 {
        let total = self.cols * self.rows;
        if total == 0 { return 0.0; }
        self.changed_indices.len() as f32 / total as f32
    }

    /// Changed tile coordinates.
    pub fn changed_tiles(&self) -> Vec<(u16, u16)> {
        self.changed_indices.iter().map(|&idx| {
            let col = (idx % self.cols) as u16;
            let row = (idx / self.cols) as u16;
            (col, row)
        }).collect()
    }
}

/// Compact representation of a frame delta for token output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressedFrame {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub total_tiles: u32,
    pub changed_tiles: u32,
    /// Changed tile descriptors: (col, row, feature_bytes).
    pub tiles: Vec<TileDescriptor>,
}

/// A single changed tile with optional extracted features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileDescriptor {
    pub col: u16,
    pub row: u16,
    /// Mean color (BGRA).
    pub mean_color: [u8; 4],
    /// Edge density (0.0 = flat, 1.0 = all edges). From Sobel if available.
    pub edge_density: f32,
    /// Whether this tile likely contains text (high horizontal contrast).
    pub likely_text: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_frame_zeroed() {
        let f = VisionFrame::zeroed(4, 4);
        assert_eq!(f.width, 4);
        assert_eq!(f.height, 4);
        assert_eq!(f.data.len(), 64);
        assert!(f.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn tile_count() {
        let f = VisionFrame::zeroed(64, 64);
        let (cols, rows) = f.tile_count(16);
        assert_eq!(cols, 4);
        assert_eq!(rows, 4);
    }

    #[test]
    fn delta_result_change_ratio() {
        let dr = DeltaResult {
            tile_size: 16,
            cols: 4,
            rows: 4,
            changed_indices: vec![0, 1, 2],
            current_hashes: vec![],
        };
        let ratio = dr.change_ratio();
        assert!((ratio - 3.0 / 16.0).abs() < 0.001);
    }

    #[test]
    fn compressed_frame_serde() {
        let cf = CompressedFrame {
            width: 32,
            height: 32,
            tile_size: 16,
            total_tiles: 4,
            changed_tiles: 1,
            tiles: vec![TileDescriptor {
                col: 0,
                row: 0,
                mean_color: [128, 128, 128, 255],
                edge_density: 0.5,
                likely_text: false,
            }],
        };
        let json = serde_json::to_string(&cf).unwrap();
        let decoded: CompressedFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(cf, decoded);
    }
}
