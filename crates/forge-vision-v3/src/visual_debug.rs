//! From F:\NewRepo\crates\forge-vision\src\visual_debug.rs (lines 1-717)
//! Visual-debug primitives — perceptual hashing + tile-delta frame compression.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

const PRODUCT_TARGETS: &[&str] = &[
    "Pixel Pastures",
    "SLAPP",
    "Godot",
    "Deveraux",
    "Dead Drop",
    "13Forge",
];
const MAX_CAPTURES: usize = 30;
const MAX_BYTES: usize = 50 * 1024 * 1024;

/// Window information for capture result tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_debug: bool,
}

/// Result of a single window capture operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub window: WindowInfo,
    pub thumbnail_path: PathBuf,
    pub detail_path: PathBuf,
    pub perceptual_hash: u64,
    pub size_bytes: usize,
}

const FALSE_GREEN_LUMA_FLOOR: u64 = 5;

/// Validates that a frame looks like real capture data, not an all-black failure.
pub fn validate_not_false_green(frame: &[u8], width: u32, height: u32) -> bool {
    let pixel_count = width as u64 * height as u64;
    if pixel_count == 0 || frame.len() < (pixel_count as usize) * 4 {
        return false;
    }
    let total_luma: u64 = frame
        .chunks_exact(4)
        .map(|px| (px[0] as u64 * 77 + px[1] as u64 * 150 + px[2] as u64 * 29) >> 8)
        .sum();
    (total_luma / pixel_count) > FALSE_GREEN_LUMA_FLOOR
}

/// LRU cache for capture results with size and count limits.
pub struct CaptureCache {
    entries: VecDeque<CaptureResult>,
    total_bytes: usize,
}

impl Default for CaptureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureCache {
    /// Create a new empty capture cache.
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            total_bytes: 0,
        }
    }

    /// Add a capture result, evicting oldest entries if limits exceeded.
    pub fn push(&mut self, capture: CaptureResult) {
        self.total_bytes += capture.size_bytes;
        self.entries.push_back(capture);
        while self.entries.len() > MAX_CAPTURES || self.total_bytes > MAX_BYTES {
            if let Some(old) = self.entries.pop_front() {
                self.total_bytes -= old.size_bytes;
            }
        }
    }

    /// Number of cached captures.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total bytes used by all cached captures.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Get the most recent capture, if any.
    pub fn latest(&self) -> Option<&CaptureResult> {
        self.entries.back()
    }
}

/// Check if window title belongs to a tracked product.
pub fn is_product_window(title: &str) -> bool {
    PRODUCT_TARGETS.iter().any(|t| title.contains(t))
}

/// Check if window title indicates a debug build.
pub fn is_debug_window(title: &str) -> bool {
    title.contains("(DEBUG)")
}

/// Perceptual hash (8x8 area-mean grid, mean-thresholded to 64-bit).
pub fn perceptual_hash(pixels: &[u8], width: usize, height: usize) -> u64 {
    if pixels.is_empty() || width == 0 || height == 0 {
        return 0;
    }
    let mut grid = [0u32; 64];
    for gy in 0..8 {
        let y0 = gy * height / 8;
        let y1 = ((gy + 1) * height / 8).clamp(y0 + 1, height.max(y0 + 1));
        for gx in 0..8 {
            let x0 = gx * width / 8;
            let x1 = ((gx + 1) * width / 8).clamp(x0 + 1, width.max(x0 + 1));
            let mut sum: u64 = 0;
            let mut n: u64 = 0;
            for y in y0..y1.min(height) {
                for x in x0..x1.min(width) {
                    let idx = (y * width + x) * 4;
                    if idx + 2 < pixels.len() {
                        sum += (pixels[idx] as u64 * 299
                            + pixels[idx + 1] as u64 * 587
                            + pixels[idx + 2] as u64 * 114)
                            / 1000;
                        n += 1;
                    }
                }
            }
            grid[gy * 8 + gx] = if n > 0 { (sum / n) as u32 } else { 0 };
        }
    }
    let mean = (grid.iter().map(|&v| v as u64).sum::<u64>() / 64) as u32;
    let mut hash = 0u64;
    for (i, &v) in grid.iter().enumerate() {
        if v > mean {
            hash |= 1 << i;
        }
    }
    hash
}

/// Hamming distance between two 64-bit hashes.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// A 64-bit hash of a single tile's pixel content.
pub type TileHash = u64;

/// Descriptor for a single changed tile (from visual_debug pipeline).
pub struct VisualTileDescriptor {
    /// Tile column index (0-based).
    pub tile_x: u32,
    /// Tile row index (0-based).
    pub tile_y: u32,
    /// Mean RGB color of the tile.
    pub mean_color: [u8; 3],
    /// Edge density (0.0 = smooth, 1.0 = all edges).
    pub edge_density: f32,
    /// True if likely contains text (high edge density).
    pub has_text: bool,
}

/// Result of compressing a frame against the previous frame (from visual_debug pipeline).
pub struct VisualCompressedFrame {
    /// Descriptors for tiles that changed vs the previous frame.
    pub changed_tiles: Vec<VisualTileDescriptor>,
    /// Total number of tiles in the frame (changed + unchanged).
    pub total_tiles: usize,
    /// Source frame width in pixels.
    pub frame_width: u32,
    /// Source frame height in pixels.
    pub frame_height: u32,
    /// Tile edge length in pixels.
    pub tile_size: u32,
}

/// Compute a 64-bit hash for a single tile region of an RGBA frame.
pub fn tile_hash_cpu(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_x: usize,
    tile_y: usize,
    tile_size: usize,
) -> TileHash {
    if pixels.is_empty() || frame_width == 0 || frame_height == 0 {
        return 0;
    }

    let x_start = tile_x * tile_size;
    let y_start = tile_y * tile_size;
    let x_end = (x_start + tile_size).min(frame_width);
    let y_end = (y_start + tile_size).min(frame_height);

    let mut hash: u64 = 0;
    let mut sample_count: u64 = 0;

    let step = 4usize;
    let mut y = y_start;
    while y < y_end {
        let mut x = x_start;
        while x < x_end {
            let idx = (y * frame_width + x) * 4;
            if idx + 2 < pixels.len() {
                let lum = (pixels[idx] as f32 * 0.299
                    + pixels[idx + 1] as f32 * 0.587
                    + pixels[idx + 2] as f32 * 0.114) as u64;
                let pos = (y * frame_width + x) as u64;
                hash ^= lum.wrapping_mul(pos.wrapping_add(1).wrapping_mul(0x9e3779b97f4a7c15));
                sample_count += 1;
            }
            x += step;
        }
        y += step;
    }

    hash ^ sample_count
}

/// Position-free tile content hash; identical pixels hash identically regardless of position.
pub fn tile_content_hash_cpu(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_x: usize,
    tile_y: usize,
    tile_size: usize,
) -> TileHash {
    if pixels.is_empty() || frame_width == 0 || frame_height == 0 {
        return 0;
    }
    let x_start = tile_x * tile_size;
    let y_start = tile_y * tile_size;
    let x_end = (x_start + tile_size).min(frame_width);
    let y_end = (y_start + tile_size).min(frame_height);

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut sample_count: u64 = 0;
    let step = 4usize;
    let mut y = y_start;
    while y < y_end {
        let mut x = x_start;
        while x < x_end {
            let idx = (y * frame_width + x) * 4;
            if idx + 2 < pixels.len() {
                let lum = (pixels[idx] as f32 * 0.299
                    + pixels[idx + 1] as f32 * 0.587
                    + pixels[idx + 2] as f32 * 0.114) as u64;
                hash = (hash ^ lum).wrapping_mul(0x0000_0100_0000_01b3);
                sample_count += 1;
            }
            x += step;
        }
        y += step;
    }
    hash ^ sample_count
}

/// Content-hash every tile in a frame, row-major.
pub fn compute_tile_content_hashes(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_size: usize,
) -> Vec<TileHash> {
    if frame_width == 0 || frame_height == 0 || tile_size == 0 {
        return Vec::new();
    }
    let tiles_x = frame_width.div_ceil(tile_size);
    let tiles_y = frame_height.div_ceil(tile_size);
    let mut hashes = Vec::with_capacity(tiles_x * tiles_y);
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            hashes.push(tile_content_hash_cpu(pixels, frame_width, frame_height, tx, ty, tile_size));
        }
    }
    hashes
}

/// Compute tile hashes for all tiles in a frame, row-major.
pub fn compute_tile_hashes(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_size: usize,
) -> Vec<TileHash> {
    if frame_width == 0 || frame_height == 0 || tile_size == 0 {
        return Vec::new();
    }

    let tiles_x = frame_width.div_ceil(tile_size);
    let tiles_y = frame_height.div_ceil(tile_size);
    let mut hashes = Vec::with_capacity(tiles_x * tiles_y);

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            hashes.push(tile_hash_cpu(pixels, frame_width, frame_height, tx, ty, tile_size));
        }
    }
    hashes
}

/// Detect which tile indices changed between two hash arrays.
pub fn detect_changes_cpu(prev: &[TileHash], curr: &[TileHash]) -> Vec<usize> {
    prev.iter()
        .zip(curr.iter())
        .enumerate()
        .filter_map(|(i, (p, c))| if p != c { Some(i) } else { None })
        .collect()
}

fn tile_descriptor_cpu(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_x: usize,
    tile_y: usize,
    tile_size: usize,
) -> VisualTileDescriptor {
    let x_start = tile_x * tile_size;
    let y_start = tile_y * tile_size;
    let x_end = (x_start + tile_size).min(frame_width);
    let y_end = (y_start + tile_size).min(frame_height);

    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut pixel_count = 0u64;
    let mut edge_sum = 0.0f32;
    let mut edge_count = 0u32;

    for y in y_start..y_end {
        for x in x_start..x_end {
            let idx = (y * frame_width + x) * 4;
            if idx + 2 >= pixels.len() {
                continue;
            }

            r_sum += pixels[idx] as u64;
            g_sum += pixels[idx + 1] as u64;
            b_sum += pixels[idx + 2] as u64;
            pixel_count += 1;

            if x + 1 < x_end && y + 1 < y_end {
                let right_idx = (y * frame_width + x + 1) * 4;
                let down_idx = ((y + 1) * frame_width + x) * 4;
                if right_idx + 2 < pixels.len() && down_idx + 2 < pixels.len() {
                    let lum_c = pixels[idx] as f32 * 0.299
                        + pixels[idx + 1] as f32 * 0.587
                        + pixels[idx + 2] as f32 * 0.114;
                    let lum_r = pixels[right_idx] as f32 * 0.299
                        + pixels[right_idx + 1] as f32 * 0.587
                        + pixels[right_idx + 2] as f32 * 0.114;
                    let lum_d = pixels[down_idx] as f32 * 0.299
                        + pixels[down_idx + 1] as f32 * 0.587
                        + pixels[down_idx + 2] as f32 * 0.114;
                    let gx = (lum_r - lum_c).abs();
                    let gy = (lum_d - lum_c).abs();
                    edge_sum += (gx * gx + gy * gy).sqrt() / 255.0;
                    edge_count += 1;
                }
            }
        }
    }

    let mean_color = [
        r_sum.checked_div(pixel_count).unwrap_or(0) as u8,
        g_sum.checked_div(pixel_count).unwrap_or(0) as u8,
        b_sum.checked_div(pixel_count).unwrap_or(0) as u8,
    ];

    let edge_density = if edge_count > 0 {
        edge_sum / edge_count as f32
    } else {
        0.0
    };

    VisualTileDescriptor {
        tile_x: tile_x as u32,
        tile_y: tile_y as u32,
        mean_color,
        edge_density,
        has_text: edge_density > 0.4,
    }
}

/// Compress a frame against the previous frame using tile-delta encoding.
pub fn compress_frame_cpu(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    prev_hashes: &mut Option<Vec<TileHash>>,
) -> VisualCompressedFrame {
    const TILE_SIZE: usize = 16;

    if frame_width == 0 || frame_height == 0 {
        return VisualCompressedFrame {
            changed_tiles: Vec::new(),
            total_tiles: 0,
            frame_width: frame_width as u32,
            frame_height: frame_height as u32,
            tile_size: TILE_SIZE as u32,
        };
    }

    let tiles_x = frame_width.div_ceil(TILE_SIZE);
    let tiles_y = frame_height.div_ceil(TILE_SIZE);
    let total_tiles = tiles_x * tiles_y;

    let curr_hashes = compute_tile_hashes(pixels, frame_width, frame_height, TILE_SIZE);

    let changed_indices = match prev_hashes {
        Some(prev) => detect_changes_cpu(prev, &curr_hashes),
        None => (0..total_tiles).collect(),
    };

    let changed_tiles: Vec<VisualTileDescriptor> = changed_indices
        .iter()
        .map(|&idx| {
            let tx = idx % tiles_x;
            let ty = idx / tiles_x;
            tile_descriptor_cpu(pixels, frame_width, frame_height, tx, ty, TILE_SIZE)
        })
        .collect();

    *prev_hashes = Some(curr_hashes);

    VisualCompressedFrame {
        changed_tiles,
        total_tiles,
        frame_width: frame_width as u32,
        frame_height: frame_height as u32,
        tile_size: TILE_SIZE as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn false_green_guard_flags_an_all_black_capture() {
        let black = vec![0u8; 64 * 64 * 4];
        assert!(!validate_not_false_green(&black, 64, 64), "all-black capture must be flagged");
    }

    #[test]
    fn false_green_guard_passes_a_real_frame() {
        let mut grey = vec![0u8; 64 * 64 * 4];
        for px in grey.chunks_exact_mut(4) {
            px[0] = 128; px[1] = 128; px[2] = 128; px[3] = 255;
        }
        assert!(validate_not_false_green(&grey, 64, 64), "a real rendered frame must pass");
    }

    #[test]
    fn false_green_guard_fails_closed_on_degenerate_input() {
        assert!(!validate_not_false_green(&[], 0, 0), "zero-sized frame is not trusted");
        assert!(!validate_not_false_green(&[1, 2, 3], 64, 64), "truncated buffer is not trusted");
    }

    #[test]
    fn product_window_detection() {
        assert!(is_product_window("SLAPP - Main Window"));
        assert!(is_product_window("Godot Engine"));
        assert!(!is_product_window("Firefox"));
    }

    #[test]
    fn debug_window_detection() {
        assert!(is_debug_window("SLAPP (DEBUG) Console"));
        assert!(!is_debug_window("SLAPP Main"));
    }

    #[test]
    fn lru_eviction_count() {
        let mut cache = CaptureCache::new();
        for i in 0..40 {
            cache.push(CaptureResult {
                window: WindowInfo {
                    title: format!("w{}", i),
                    hwnd: i as u64,
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                    is_debug: false,
                },
                thumbnail_path: PathBuf::new(),
                detail_path: PathBuf::new(),
                perceptual_hash: 0,
                size_bytes: 100,
            });
        }
        assert!(cache.len() <= MAX_CAPTURES);
    }

    #[test]
    fn lru_eviction_bytes() {
        let mut cache = CaptureCache::new();
        for i in 0..5 {
            cache.push(CaptureResult {
                window: WindowInfo {
                    title: "w".into(),
                    hwnd: i,
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                    is_debug: false,
                },
                thumbnail_path: PathBuf::new(),
                detail_path: PathBuf::new(),
                perceptual_hash: 0,
                size_bytes: 20 * 1024 * 1024,
            });
        }
        assert!(cache.total_bytes() <= MAX_BYTES);
    }

    #[test]
    fn perceptual_hash_deterministic() {
        let pixels = vec![128u8; 400];
        let a = perceptual_hash(&pixels, 10, 10);
        let b = perceptual_hash(&pixels, 10, 10);
        assert_eq!(a, b);
    }

    #[test]
    fn bright_structured_window_does_not_saturate() {
        let (w, h) = (64usize, 64usize);
        let dark_band = |top: usize| -> Vec<u8> {
            let mut px = vec![255u8; w * h * 4];
            for y in top..top + 4 {
                for x in 8..56 {
                    let i = (y * w + x) * 4;
                    px[i] = 20;
                    px[i + 1] = 20;
                    px[i + 2] = 20;
                }
            }
            px
        };
        let ha = perceptual_hash(&dark_band(20), w, h);
        assert_ne!(ha, u64::MAX, "bright frame must not saturate to all-ones");
        assert_ne!(ha, 0, "structured bright frame must not collapse to zero");
        let hb = perceptual_hash(&dark_band(40), w, h);
        assert_ne!(ha, hb, "two different bright frames must hash differently");
        assert_eq!(perceptual_hash(&vec![255u8; w * h * 4], w, h), 0);
    }

    #[test]
    fn hamming_identical() {
        assert_eq!(hamming_distance(0xFF, 0xFF), 0);
    }

    #[test]
    fn hamming_different() {
        assert!(hamming_distance(0x00, 0xFF) > 0);
    }

    #[test]
    fn tile_delta_idempotence() {
        let w = 64usize;
        let h = 64usize;
        let pixels: Vec<u8> = (0..w * h * 4).map(|i| (i % 256) as u8).collect();

        let mut prev_hashes: Option<Vec<TileHash>> = None;

        let first = compress_frame_cpu(&pixels, w, h, &mut prev_hashes);
        assert!(!first.changed_tiles.is_empty(), "first frame should have changed tiles");

        let second = compress_frame_cpu(&pixels, w, h, &mut prev_hashes);
        assert_eq!(
            second.changed_tiles.len(),
            0,
            "identical frame should produce 0 changed tiles (idempotence)"
        );
    }

    #[test]
    fn tile_hash_determinism() {
        let w = 32usize;
        let h = 32usize;
        let pixels: Vec<u8> = (0..w * h * 4).map(|i| ((i * 7 + 13) % 256) as u8).collect();

        let hash_a = tile_hash_cpu(&pixels, w, h, 0, 0, 16);
        let hash_b = tile_hash_cpu(&pixels, w, h, 0, 0, 16);
        assert_eq!(hash_a, hash_b, "tile_hash_cpu must be deterministic");

        let hashes_a = compute_tile_hashes(&pixels, w, h, 16);
        let hashes_b = compute_tile_hashes(&pixels, w, h, 16);
        assert_eq!(hashes_a, hashes_b, "compute_tile_hashes must be deterministic");
    }

    #[test]
    fn tile_delta_detects_changes() {
        let w = 64usize;
        let h = 64usize;
        let frame_a: Vec<u8> = vec![128u8; w * h * 4];
        let mut frame_b = frame_a.clone();
        for y in 0..16 {
            for x in 0..16 {
                let idx = (y * w + x) * 4;
                frame_b[idx] = 255;
                frame_b[idx + 1] = 0;
                frame_b[idx + 2] = 0;
            }
        }

        let mut prev: Option<Vec<TileHash>> = None;
        let _ = compress_frame_cpu(&frame_a, w, h, &mut prev);
        let result = compress_frame_cpu(&frame_b, w, h, &mut prev);

        assert!(!result.changed_tiles.is_empty(), "changed tile should be detected");
        assert!(
            result.changed_tiles.iter().any(|t| t.tile_x == 0 && t.tile_y == 0),
            "tile (0,0) should be detected as changed"
        );
    }

    #[test]
    fn perceptual_hash_idempotence() {
        let pixels: Vec<u8> = (0..64 * 64 * 4).map(|i| (i % 256) as u8).collect();
        let h1 = perceptual_hash(&pixels, 64, 64);
        let h2 = perceptual_hash(&pixels, 64, 64);
        assert_eq!(h1, h2, "perceptual_hash must be idempotent");
    }

    #[test]
    fn compressed_frame_total_tiles_correct() {
        let w = 64usize;
        let h = 64usize;
        let pixels = vec![0u8; w * h * 4];
        let mut prev: Option<Vec<TileHash>> = None;
        let result = compress_frame_cpu(&pixels, w, h, &mut prev);
        assert_eq!(result.total_tiles, 16);
        assert_eq!(result.tile_size, 16);
    }
}
