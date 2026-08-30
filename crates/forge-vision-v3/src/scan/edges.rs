//! Edge detection via Sobel operator. Ported verbatim from
//! `F:\NewRepo\crates\forge-vision\src\scan\edges.rs` (2026-08-13), minus
//! `detect_edges_from_file` — that helper reads image files through
//! `super::photometric::load_image_sniffed` (the `image` crate), a file-I/O
//! path this port doesn't need yet; `detect_edges` itself is pure pixel math
//! and carries no such dependency.

pub struct BinaryImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<bool>,
}

pub fn detect_edges(gray: &[u8], width: u32, height: u32, threshold: f32) -> BinaryImage {
    let w = width as i32;
    let h = height as i32;
    let get = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x >= w || y >= h { return 0.0; }
        gray[(y * w + x) as usize] as f32
    };

    let mut data = vec![false; (width * height) as usize];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let gx = -get(x-1,y-1) - 2.0*get(x-1,y) - get(x-1,y+1)
                    + get(x+1,y-1) + 2.0*get(x+1,y) + get(x+1,y+1);
            let gy = -get(x-1,y-1) - 2.0*get(x,y-1) - get(x+1,y-1)
                    + get(x-1,y+1) + 2.0*get(x,y+1) + get(x+1,y+1);
            let mag = (gx * gx + gy * gy).sqrt();
            data[(y * w + x) as usize] = mag > threshold;
        }
    }
    BinaryImage { width, height, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_edges() {
        let (w, h) = (64u32, 64u32);
        let mut gray = vec![0u8; (w * h) as usize];
        // Draw filled circle
        for y in 0..h { for x in 0..w {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            if dx * dx + dy * dy < 20.0 * 20.0 {
                gray[(y * w + x) as usize] = 255;
            }
        }}
        let edges = detect_edges(&gray, w, h, 100.0);
        let edge_count = edges.data.iter().filter(|&&v| v).count();
        assert!(edge_count > 20, "should detect circle edges, got {}", edge_count);
    }
}
