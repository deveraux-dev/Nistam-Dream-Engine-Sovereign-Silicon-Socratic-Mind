//! MSDF (multi-channel signed distance field) atlas generator — Chlumsky technique.
//!
//! Three colour channels each carry an independent signed distance field so corners
//! survive reconstruction (median of 3) where a plain single-channel SDF rounds them off.
//!
//! **Note**: This module uses floating-point arithmetic at the font rasterization boundary,
//! which is unavoidable for accurate signed distance calculations on bezier curves.
//! All output is quantized to `u8` for GPU texture binding.

/// A single edge in a closed outline: a straight line or a quadratic bezier.
#[derive(Debug, Clone, Copy)]
pub enum Segment {
    /// A straight line from point `a` to point `b`.
    Line {
        /// Start point.
        a: [f32; 2],
        /// End point.
        b: [f32; 2],
    },
    /// A quadratic bezier with start `a`, control point `ctrl`, and end `b`.
    Quad {
        /// Start point.
        a: [f32; 2],
        /// Control point.
        ctrl: [f32; 2],
        /// End point.
        b: [f32; 2],
    },
}

/// Signed distance plus the orthogonality tiebreaker MSDF needs for equidistant edges.
///
/// MSDF uses orthogonality to pick the "most perpendicular" edge when multiple
/// edges are equidistant, ensuring sharp corners survive reconstruction.
#[derive(Debug, Clone, Copy)]
pub struct SignedDist {
    /// Signed distance to the nearest edge (negative=outside, positive=inside).
    pub dist: f32,
    /// Orthogonality: how perpendicular the edge is to the point (0..=1).
    pub orthogonality: f32,
}

/// Channel mask a segment writes into; true = that RGB channel carries this edge.
///
/// MSDF works by assigning each edge to 1–3 channels. Corners switch channels
/// to maximize corner preservation; smooth curves stay on one channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeColor {
    /// Red channel.
    pub r: bool,
    /// Green channel.
    pub g: bool,
    /// Blue channel.
    pub b: bool,
}

/// A closed outline: ordered segments plus the per-segment channel assignment.
#[derive(Debug, Clone, Default)]
pub struct Contour {
    /// Segments forming the closed outline (order matters).
    pub segments: Vec<Segment>,
    /// Per-segment channel mask (parallel to `segments`).
    pub colors: Vec<EdgeColor>,
}

impl Segment {
    /// Tangent vector at the start of this segment.
    fn tangent_at_start(&self) -> [f32; 2] {
        match self {
            Segment::Line { a, b } => sub(*b, *a),
            Segment::Quad { a, ctrl, .. } => sub(*ctrl, *a),
        }
    }

    /// Tangent vector at the end of this segment.
    fn tangent_at_end(&self) -> [f32; 2] {
        match self {
            Segment::Line { a, b } => sub(*b, *a),
            Segment::Quad { ctrl, b, .. } => sub(*b, *ctrl),
        }
    }

    /// Signed distance from point `p` to this segment, plus the orthogonality tiebreaker.
    pub fn signed_distance(&self, p: [f32; 2]) -> SignedDist {
        match self {
            Segment::Line { a, b } => line_signed_distance(*a, *b, p),
            Segment::Quad { a, ctrl, b } => quad_signed_distance(*a, *ctrl, *b, p),
        }
    }
}

/// Subtract two 2D points: `a - b`.
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// Dot product: `a · b`.
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// Euclidean length of a 2D vector.
fn vec_len(a: [f32; 2]) -> f32 {
    dot(a, a).sqrt()
}

/// Cross product in 2D (returns scalar): `a × b = a.x·b.y - a.y·b.x`.
fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

/// Normalize a 2D vector to unit length. Returns zero vector if length < 1e-9.
fn normalize(a: [f32; 2]) -> [f32; 2] {
    let l = vec_len(a);
    if l < 1e-9 {
        [0.0, 0.0]
    } else {
        [a[0] / l, a[1] / l]
    }
}

/// Signed distance from point `p` to a line segment from `a` to `b`.
fn line_signed_distance(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> SignedDist {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let len_sq = dot(ab, ab).max(1e-12);
    let t = (dot(ap, ab) / len_sq).clamp(0.0, 1.0);
    let closest = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    let to_p = sub(p, closest);
    let dist = vec_len(to_p);
    let sign = if cross(ab, ap) < 0.0 { -1.0 } else { 1.0 };
    let dir = normalize(ab);
    let to_point = normalize(to_p);
    let orthogonality = cross(dir, to_point).abs();
    SignedDist { dist: dist * sign, orthogonality }
}

/// Coarse signed distance to a quadratic bezier via subdivision.
///
/// Adequate at atlas bake resolution. Subdivides the curve into 24 line segments
/// and returns the closest distance found.
fn quad_signed_distance(a: [f32; 2], ctrl: [f32; 2], b: [f32; 2], p: [f32; 2]) -> SignedDist {
    const STEPS: usize = 24;
    let mut best = SignedDist { dist: f32::MAX, orthogonality: 0.0 };
    let mut prev = a;
    for i in 1..=STEPS {
        let t = i as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let pt = [
            mt * mt * a[0] + 2.0 * mt * t * ctrl[0] + t * t * b[0],
            mt * mt * a[1] + 2.0 * mt * t * ctrl[1] + t * t * b[1],
        ];
        let seg = line_signed_distance(prev, pt, p);
        if seg.dist.abs() < best.dist.abs() {
            best = seg;
        }
        prev = pt;
    }
    best
}

/// Assign RGB channel masks to each segment.
///
/// The mask switches only at CORNERS (tangent discontinuity beyond `angle_threshold_rad`).
/// Smooth joins keep the previous mask — this is what makes the field multi-channel
/// only where it needs to be.
///
/// # Arguments
///
/// - `contour`: Mutable reference to the contour to be colored.
/// - `angle_threshold_rad`: Angle (in radians) above which a join is considered a corner.
pub fn edge_colors(contour: &mut Contour, angle_threshold_rad: f32) {
    let n = contour.segments.len();
    if n == 0 {
        return;
    }
    let palette = [
        EdgeColor { r: true, g: false, b: true },
        EdgeColor { r: true, g: true, b: false },
        EdgeColor { r: false, g: true, b: true },
    ];
    let mut colors = vec![palette[0]; n];
    let mut idx = 0usize;
    for i in 0..n {
        let prev = &contour.segments[(i + n - 1) % n];
        let cur = &contour.segments[i];
        let incoming = normalize(prev.tangent_at_end());
        let outgoing = normalize(cur.tangent_at_start());
        let cos_angle = dot(incoming, outgoing).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();
        if i > 0 && angle > angle_threshold_rad {
            idx = (idx + 1) % palette.len();
        }
        colors[i] = palette[idx];
    }
    contour.colors = colors;
}

/// Bake a `w×h` texel MSDF atlas from a contour.
///
/// # Arguments
///
/// - `contour`: The contour to rasterize.
/// - `w`, `h`: Texture dimensions in texels.
/// - `px_range`: Distance-to-byte scale factor (larger = more detail at edges).
///
/// # Returns
///
/// A vector of `[u8; 3]` RGB texels. Per texel, takes the minimum `|distance|` segment
/// carrying each channel, mapped through `px_range` and quantized to `[0, 255]`.
pub fn generate(contour: &Contour, w: usize, h: usize, px_range: f32) -> Vec<[u8; 3]> {
    let mut out = vec![[0u8; 3]; w * h];
    for y in 0..h {
        for x in 0..w {
            let p = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
            let mut best_r = f32::MAX;
            let mut best_g = f32::MAX;
            let mut best_b = f32::MAX;
            for (seg, color) in contour.segments.iter().zip(contour.colors.iter()) {
                let sd = seg.signed_distance(p);
                if color.r && sd.dist.abs() < best_r.abs() {
                    best_r = sd.dist;
                }
                if color.g && sd.dist.abs() < best_g.abs() {
                    best_g = sd.dist;
                }
                if color.b && sd.dist.abs() < best_b.abs() {
                    best_b = sd.dist;
                }
            }
            let enc = |d: f32| -> u8 {
                // No segment ever claimed this channel (sentinel still MAX, or a
                // non-finite distance slipped through) — encode as neutral (0.0),
                // not as "infinitely far inside".
                let v = if d.is_finite() && d != f32::MAX { d } else { 0.0 };
                (((v / px_range) * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8
            };
            out[y * w + x] = [enc(best_r), enc(best_g), enc(best_b)];
        }
    }
    out
}

/// The reconstruction the shader does at sample time: median of the three channels.
///
/// This is the key operation that recovers the high-quality signed distance
/// from the three-channel encoding.
pub fn median(rgb: [u8; 3]) -> u8 {
    let mut v = rgb;
    v.sort_unstable();
    v[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: Create a unit square contour at position (ox, oy) with side length s.
    fn unit_square(ox: f32, oy: f32, s: f32) -> Contour {
        let p00 = [ox, oy];
        let p10 = [ox + s, oy];
        let p11 = [ox + s, oy + s];
        let p01 = [ox, oy + s];
        Contour {
            segments: vec![
                Segment::Line { a: p00, b: p10 },
                Segment::Line { a: p10, b: p11 },
                Segment::Line { a: p11, b: p01 },
                Segment::Line { a: p01, b: p00 },
            ],
            colors: vec![],
        }
    }

    #[test]
    fn field_inside_vs_outside() {
        let mut c = unit_square(0.25, 0.25, 0.5);
        edge_colors(&mut c, 0.1);
        let field = generate(&c, 32, 32, 0.1);
        let centre = median(field[16 * 32 + 16]);
        let corner = median(field[0]);
        assert!(centre > 127, "centre should read inside: {}", centre);
        assert!(corner < 127, "far corner should read outside: {}", corner);
    }

    // L07: Determinism test — generate the same contour twice and compare.
    #[test]
    fn msdf_generate_deterministic() {
        let mut c1 = unit_square(0.0, 0.0, 1.0);
        edge_colors(&mut c1, 0.1);
        let field1 = generate(&c1, 16, 16, 0.1);

        let mut c2 = unit_square(0.0, 0.0, 1.0);
        edge_colors(&mut c2, 0.1);
        let field2 = generate(&c2, 16, 16, 0.1);

        assert_eq!(field1.len(), field2.len());
        for i in 0..field1.len() {
            assert_eq!(field1[i], field2[i], "generation must be deterministic at texel {i}");
        }
    }

    // L18: Sabotage test — verify that an empty contour yields neutral (128) median.
    #[test]
    fn msdf_empty_contour_is_neutral() {
        let c = Contour { segments: vec![], colors: vec![] };
        let field = generate(&c, 4, 4, 1.0);
        for texel in field {
            let m = median(texel);
            assert!(
                m >= 100 && m <= 156,
                "empty contour should yield neutral median near 128, got {m}"
            );
        }
    }

    #[test]
    fn edge_colors_square_vs_circle() {
        let mut square = unit_square(0.0, 0.0, 1.0);
        edge_colors(&mut square, 0.1);
        let distinct: std::collections::HashSet<EdgeColor> = square.colors.iter().copied().collect();
        assert!(distinct.len() > 1, "square corners must split channels");

        let mut circle = Contour {
            segments: vec![
                Segment::Quad { a: [1.0, 0.0], ctrl: [1.0, 1.0], b: [0.0, 1.0] },
                Segment::Quad { a: [0.0, 1.0], ctrl: [-1.0, 1.0], b: [-1.0, 0.0] },
                Segment::Quad { a: [-1.0, 0.0], ctrl: [-1.0, -1.0], b: [0.0, -1.0] },
                Segment::Quad { a: [0.0, -1.0], ctrl: [1.0, -1.0], b: [1.0, 0.0] },
            ],
            colors: vec![],
        };
        edge_colors(&mut circle, 0.1);
        let distinct: std::collections::HashSet<EdgeColor> = circle.colors.iter().copied().collect();
        assert_eq!(distinct.len(), 1, "smooth circle keeps one channel mask");
    }

    #[test]
    fn median_picks_middle_value() {
        assert_eq!(median([10, 200, 50]), 50);
        assert_eq!(median([255, 0, 128]), 128);
    }

    #[test]
    fn generate_returns_w_times_h_texels() {
        let mut c = unit_square(0.1, 0.1, 0.8);
        edge_colors(&mut c, 0.1);
        let field = generate(&c, 5, 7, 0.1);
        assert_eq!(field.len(), 5 * 7);
    }

    #[test]
    fn on_outline_reconstructs_near_128() {
        let mut c = unit_square(0.25, 0.25, 0.5);
        c.colors = vec![EdgeColor { r: true, g: true, b: true }; 4];
        let field = generate(&c, 1, 2, 1.0);
        let v = median(field[0]);
        assert!((v as i32 - 128).abs() <= 6, "boundary point should reconstruct near 128: {}", v);
    }
}
