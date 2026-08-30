//! From F:\NewRepo\crates\forge-vision\src\inlined_compress.rs (lines 1-271)
//! CPU-only tile hash compression using FNV-1a.

use crate::inlined_types::{CompressedFrame, VisionFrame, TileDescriptor};

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Hash a single tile's pixel data using FNV-1a.
pub fn hash_tile(frame: &VisionFrame, col: u32, row: u32, tile_size: u32) -> u64 {
    let mut hash: u64 = FNV_OFFSET;
    let x0 = (col * tile_size) as usize;
    let y0 = (row * tile_size) as usize;
    let w = frame.width as usize;
    let h = frame.height as usize;

    for dy in 0..tile_size as usize {
        let y = y0 + dy;
        if y >= h { break; }
        for dx in 0..tile_size as usize {
            let x = x0 + dx;
            if x >= w { break; }
            let px = (y * w + x) * 4;
            for c in 0..4 {
                hash ^= frame.data[px + c] as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
    hash
}

/// Compress a frame into a `CompressedFrame` using CPU-side FNV-1a tile hashing.
pub fn cpu_compress_frame(
    frame: &VisionFrame,
    prev_hashes: &mut Option<Vec<u64>>,
    tile_size: u32,
) -> CompressedFrame {
    let cols = (frame.width + tile_size - 1) / tile_size;
    let rows = (frame.height + tile_size - 1) / tile_size;
    let total = (cols * rows) as usize;

    let mut current_hashes = vec![0u64; total];
    for r in 0..rows {
        for c in 0..cols {
            let idx = (r * cols + c) as usize;
            current_hashes[idx] = hash_tile(frame, c, r, tile_size);
        }
    }

    let changed_indices: Vec<u32> = match prev_hashes {
        None => (0..total as u32).collect(),
        Some(prev) => (0..total)
            .filter(|&i| i >= prev.len() || current_hashes[i] != prev[i])
            .map(|i| i as u32)
            .collect(),
    };

    let gray = to_grayscale(frame);
    let tiles: Vec<TileDescriptor> = changed_indices
        .iter()
        .map(|&idx| {
            let col = (idx % cols) as usize;
            let row = (idx / cols) as usize;
            extract_tile_features(frame, &gray, col, row, tile_size as usize)
        })
        .collect();

    *prev_hashes = Some(current_hashes);

    CompressedFrame {
        width: frame.width,
        height: frame.height,
        tile_size,
        total_tiles: total as u32,
        changed_tiles: tiles.len() as u32,
        tiles,
    }
}

/// Convert a BGRA frame to single-channel grayscale.
pub fn to_grayscale(frame: &VisionFrame) -> Vec<u8> {
    let pixel_count = (frame.width as usize) * (frame.height as usize);
    let mut gray = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let px = i * 4;
        let b = frame.data[px] as f32;
        let g = frame.data[px + 1] as f32;
        let r = frame.data[px + 2] as f32;
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        gray.push(lum.round() as u8);
    }
    gray
}

/// Extract per-tile features: mean color (RGBA), edge density, likely_text.
pub fn extract_tile_features(
    frame: &VisionFrame,
    gray: &[u8],
    col: usize,
    row: usize,
    tile_size: usize,
) -> TileDescriptor {
    let x0 = col * tile_size;
    let y0 = row * tile_size;
    let w = frame.width as usize;
    let h = frame.height as usize;
    let tw = tile_size.min(w.saturating_sub(x0));
    let th = tile_size.min(h.saturating_sub(y0));
    let pixel_count = tw * th;

    if pixel_count == 0 {
        return TileDescriptor {
            col: col as u16,
            row: row as u16,
            mean_color: [0, 0, 0, 0],
            edge_density: 0.0,
            likely_text: false,
        };
    }

    let mut sum_r: u64 = 0;
    let mut sum_g: u64 = 0;
    let mut sum_b: u64 = 0;
    let mut sum_a: u64 = 0;

    for dy in 0..th {
        let y = y0 + dy;
        for dx in 0..tw {
            let x = x0 + dx;
            let px = (y * w + x) * 4;
            sum_b += frame.data[px] as u64;
            sum_g += frame.data[px + 1] as u64;
            sum_r += frame.data[px + 2] as u64;
            sum_a += frame.data[px + 3] as u64;
        }
    }

    let n = pixel_count as u64;
    let mean_color = [
        (sum_r / n) as u8,
        (sum_g / n) as u8,
        (sum_b / n) as u8,
        (sum_a / n) as u8,
    ];

    let edge_density = compute_edge_density(gray, x0, y0, tw, th, w, h);
    let likely_text = edge_density > 0.05 && edge_density < 0.4;

    TileDescriptor { col: col as u16, row: row as u16, mean_color, edge_density, likely_text }
}

/// Compute edge density for a tile region using Sobel approximation.
pub fn compute_edge_density(
    gray: &[u8],
    x0: usize,
    y0: usize,
    tw: usize,
    th: usize,
    frame_w: usize,
    frame_h: usize,
) -> f32 {
    if tw < 2 || th < 2 { return 0.0; }

    let mut gradient_sum: f64 = 0.0;
    let mut count: u32 = 0;

    for dy in 0..th {
        let y = y0 + dy;
        if y == 0 || y >= frame_h - 1 { continue; }
        for dx in 0..tw {
            let x = x0 + dx;
            if x == 0 || x >= frame_w - 1 { continue; }

            let tl = gray[(y - 1) * frame_w + (x - 1)] as f64;
            let tc = gray[(y - 1) * frame_w + x] as f64;
            let tr = gray[(y - 1) * frame_w + (x + 1)] as f64;
            let ml = gray[y * frame_w + (x - 1)] as f64;
            let mr = gray[y * frame_w + (x + 1)] as f64;
            let bl = gray[(y + 1) * frame_w + (x - 1)] as f64;
            let bc = gray[(y + 1) * frame_w + x] as f64;
            let br = gray[(y + 1) * frame_w + (x + 1)] as f64;

            let gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
            let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
            gradient_sum += (gx * gx + gy * gy).sqrt() / 1442.5;
            count += 1;
        }
    }

    if count == 0 { return 0.0; }
    ((gradient_sum / count as f64) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_tile_deterministic() {
        let frame = VisionFrame::new(4, 4, vec![42u8; 4 * 4 * 4]);
        assert_eq!(hash_tile(&frame, 0, 0, 4), hash_tile(&frame, 0, 0, 4));
    }

    #[test]
    fn hash_tile_different_data() {
        let fa = VisionFrame::new(4, 4, vec![0u8; 64]);
        let fb = VisionFrame::new(4, 4, vec![255u8; 64]);
        assert_ne!(hash_tile(&fa, 0, 0, 4), hash_tile(&fb, 0, 0, 4));
    }

    #[test]
    fn compress_first_frame_all_changed() {
        let frame = VisionFrame::new(32, 32, vec![128u8; 32 * 32 * 4]);
        let mut prev = None;
        let cf = cpu_compress_frame(&frame, &mut prev, 16);
        assert_eq!(cf.total_tiles, 4);
        assert_eq!(cf.changed_tiles, 4);
    }

    #[test]
    fn compress_static_screen_zero_changes() {
        let frame = VisionFrame::new(32, 32, vec![128u8; 32 * 32 * 4]);
        let mut prev = None;
        let _ = cpu_compress_frame(&frame, &mut prev, 16);
        let cf2 = cpu_compress_frame(&frame, &mut prev, 16);
        assert_eq!(cf2.changed_tiles, 0);
    }

    #[test]
    fn to_grayscale_single_pixel() {
        let frame = VisionFrame::new(1, 1, vec![0, 0, 255, 255]);
        let gray = to_grayscale(&frame);
        assert_eq!(gray[0], 76);
    }
}
