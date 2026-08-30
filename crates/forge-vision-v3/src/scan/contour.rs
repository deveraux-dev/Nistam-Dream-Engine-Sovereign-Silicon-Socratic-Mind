//! Contour extraction from binary edge images.

use super::edges::BinaryImage;

/// Extract contour points from a binary image using boundary tracing.
pub fn extract_contours(edges: &BinaryImage) -> Vec<Vec<(f32, f32)>> {
    let mut visited = vec![false; edges.data.len()];
    let mut contours = Vec::new();
    let w = edges.width as i32;
    let h = edges.height as i32;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if edges.data[idx] && !visited[idx] {
                let contour = trace_contour(edges, &mut visited, x, y);
                if contour.len() >= 3 {
                    let normalized: Vec<(f32, f32)> = contour.iter()
                        .map(|&(px, py)| (px as f32 / edges.width as f32, py as f32 / edges.height as f32))
                        .collect();
                    contours.push(simplify_dp(&normalized, 0.005));
                }
            }
        }
    }
    contours
}

fn trace_contour(edges: &BinaryImage, visited: &mut [bool], sx: i32, sy: i32) -> Vec<(i32, i32)> {
    let w = edges.width as i32;
    let h = edges.height as i32;
    let dirs: [(i32, i32); 8] = [(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1),(0,-1),(1,-1)];
    let mut points = vec![(sx, sy)];
    visited[(sy * w + sx) as usize] = true;
    let (mut cx, mut cy) = (sx, sy);

    for _ in 0..10000 {
        let mut found = false;
        for &(dx, dy) in &dirs {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
            let idx = (ny * w + nx) as usize;
            if edges.data[idx] && !visited[idx] {
                visited[idx] = true;
                points.push((nx, ny));
                cx = nx;
                cy = ny;
                found = true;
                break;
            }
        }
        if !found { break; }
    }
    points
}

/// Douglas-Peucker polyline simplification.
fn simplify_dp(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    if points.len() <= 2 { return points.to_vec(); }
    let (start, end) = (points[0], *points.last().unwrap());
    let mut max_dist = 0.0_f32;
    let mut max_idx = 0;
    for (i, &p) in points.iter().enumerate().skip(1) {
        let d = point_line_dist(p, start, end);
        if d > max_dist { max_dist = d; max_idx = i; }
    }
    if max_dist > epsilon {
        let mut left = simplify_dp(&points[..=max_idx], epsilon);
        let right = simplify_dp(&points[max_idx..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![start, end]
    }
}

fn point_line_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 { return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt(); }
    let num = ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs();
    num / len_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::edges::detect_edges;

    #[test]
    fn circle_contour() {
        let (w, h) = (64u32, 64u32);
        let mut gray = vec![0u8; (w * h) as usize];
        for y in 0..h { for x in 0..w {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            if dx * dx + dy * dy < 20.0 * 20.0 { gray[(y * w + x) as usize] = 255; }
        }}
        let edges = detect_edges(&gray, w, h, 100.0);
        let contours = extract_contours(&edges);
        assert!(!contours.is_empty(), "should find at least one contour");
    }
}
