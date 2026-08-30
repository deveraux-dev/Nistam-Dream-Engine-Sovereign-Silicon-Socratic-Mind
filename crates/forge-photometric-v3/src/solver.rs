//! Photometric relief solver: coverage → normals + depth. Ported from
//! F:\NewRepo\crates\forge-photometric and forge-vision scan modules.

use crate::{encode_octahedral, NormalAlbedo8, OCT_SCALE, PMY_MAX};
use glam::Vec3;
use rustfft::{FftPlanner, num_complex::Complex};

/// Solved glyph relief: octahedral normals and depth from coverage bitmap.
///
/// Output of `solve_relief`: per-texel surface normals (octahedral-encoded as
/// `NormalAlbedo8`) and integrated depth (0..=10000 permyriad units).
/// Each output vector has length `width * height`.
pub struct GlyphRelief {
    /// Image width in texels.
    pub width: u16,
    /// Image height in texels.
    pub height: u16,
    /// Integrated depth per texel, permyriad scale 0..=10000. Length = width*height.
    pub depth_pmy: Vec<u16>,
    /// Octahedral-encoded surface normals per texel. Length = width*height.
    pub normals: Vec<NormalAlbedo8>,
}

/// Morphometric depth prior: bounding box + expected depth.
#[derive(Debug, Clone)]
struct Morphometric {
    bbox: [f32; 4],
    depth_meters: f32,
    roundness: f32,
}

/// Sobel gradient → surface normals. Coverage 0..255 mapped to 0..1 intensity.
/// Returns f32 normals; internal only.
fn normals_from_gradient_internal(gray: &[u8], width: u16, height: u16) -> Vec<Vec3> {
    normals_from_gradient_scaled_internal(gray, width, height, 4.0)
}

/// Gradient with configurable strength. `strength=4.0` is the default.
fn normals_from_gradient_scaled_internal(
    gray: &[u8],
    width: u16,
    height: u16,
    strength: f32,
) -> Vec<Vec3> {
    let w = width as i32;
    let h = height as i32;
    let get = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x >= w || y >= h {
            return 0.0;
        }
        gray[(y * w + x) as usize] as f32 / 255.0
    };

    let mut normals = Vec::with_capacity((width as usize) * (height as usize));

    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x >= w - 1 || y >= h - 1 {
                normals.push(Vec3::Z);
                continue;
            }

            let gx = -get(x - 1, y - 1) - 2.0 * get(x - 1, y) - get(x - 1, y + 1)
                + get(x + 1, y - 1)
                + 2.0 * get(x + 1, y)
                + get(x + 1, y + 1);
            let gy = -get(x - 1, y - 1) - 2.0 * get(x, y - 1) - get(x + 1, y - 1)
                + get(x - 1, y + 1)
                + 2.0 * get(x, y + 1)
                + get(x + 1, y + 1);

            let n = Vec3::new(-gx * strength, -gy * strength, 1.0).normalize();
            normals.push(n);
        }
    }

    normals
}

/// 2D FFT in-place, row-then-column decomposition.
fn fft_2d(data: &mut [Complex<f32>], width: usize, height: usize, inverse: bool) {
    let mut planner = FftPlanner::new();

    let row_fft = if inverse {
        planner.plan_fft_inverse(width)
    } else {
        planner.plan_fft_forward(width)
    };
    let mut scratch = vec![Complex::new(0.0f32, 0.0); row_fft.get_inplace_scratch_len()];
    for y in 0..height {
        let start = y * width;
        row_fft.process_with_scratch(&mut data[start..start + width], &mut scratch);
    }

    let col_fft = if inverse {
        planner.plan_fft_inverse(height)
    } else {
        planner.plan_fft_forward(height)
    };
    scratch.resize(col_fft.get_inplace_scratch_len(), Complex::new(0.0, 0.0));
    let mut col = vec![Complex::new(0.0f32, 0.0); height];
    for x in 0..width {
        for y in 0..height {
            col[y] = data[y * width + x];
        }
        col_fft.process_with_scratch(&mut col, &mut scratch);
        for y in 0..height {
            data[y * width + x] = col[y];
        }
    }

    if inverse {
        let scale = 1.0 / (width * height) as f32;
        for c in data.iter_mut() {
            *c *= scale;
        }
    }
}

/// Poisson surface integration (Frankot-Chellappa 1988).
/// Reconstructs depth z(x,y) from surface normals via FFT.
/// Returns depth normalized to 0..1.
fn poisson_integrate(normals: &[Vec3], width: usize, height: usize) -> Vec<f32> {
    let n = width * height;
    assert_eq!(normals.len(), n);

    let mut p_data: Vec<Complex<f32>> = Vec::with_capacity(n);
    let mut q_data: Vec<Complex<f32>> = Vec::with_capacity(n);
    for normal in normals {
        if normal.z.abs() > 0.01 {
            p_data.push(Complex::new(-normal.x / normal.z, 0.0));
            q_data.push(Complex::new(-normal.y / normal.z, 0.0));
        } else {
            p_data.push(Complex::new(0.0, 0.0));
            q_data.push(Complex::new(0.0, 0.0));
        }
    }

    fft_2d(&mut p_data, width, height, false);
    fft_2d(&mut q_data, width, height, false);

    let mut z_data = vec![Complex::new(0.0f32, 0.0); n];
    let two_pi = 2.0 * std::f32::consts::PI;

    for v in 0..height {
        for u in 0..width {
            let fu = if u <= width / 2 {
                u as f32
            } else {
                u as f32 - width as f32
            };
            let fv = if v <= height / 2 {
                v as f32
            } else {
                v as f32 - height as f32
            };
            let wx = two_pi * fu / width as f32;
            let wy = two_pi * fv / height as f32;

            let denom = wx * wx + wy * wy;
            if denom < 1e-10 {
                continue;
            }

            let idx = v * width + u;
            let sum = p_data[idx] * wx + q_data[idx] * wy;
            z_data[idx] = Complex::new(sum.im, -sum.re) / denom;
        }
    }

    fft_2d(&mut z_data, width, height, true);

    let mut depth: Vec<f32> = z_data.iter().map(|c| c.re).collect();
    let min_d = depth.iter().cloned().fold(f32::MAX, f32::min);
    let max_d = depth.iter().cloned().fold(f32::MIN, f32::max);
    let range = (max_d - min_d).max(1e-6);
    for d in &mut depth {
        *d = (*d - min_d) / range;
    }

    depth
}

/// Morphometric depth prior from letter analysis.
fn letter_morphometrics(alpha: &[u8], width: u16, height: u16, _font_size: f32) -> Vec<Morphometric> {
    let (w, h) = (width as usize, height as usize);
    let mut priors = Vec::new();

    let solid = |x: usize, y: usize| -> bool { alpha[y * w + x] > 127 };

    let stem_threshold = (h as f32 * 0.6) as usize;
    let mut x = 0;
    while x < w {
        let col_fill: usize = (0..h).filter(|&y| solid(x, y)).count();
        if col_fill >= stem_threshold {
            let stem_start = x;
            while x < w && (0..h).filter(|&y| solid(x, y)).count() >= stem_threshold {
                x += 1;
            }
            let stem_end = x;
            let sw = stem_end - stem_start;
            if sw >= 2 {
                priors.push(Morphometric {
                    bbox: [
                        stem_start as f32 / w as f32,
                        0.0,
                        stem_end as f32 / w as f32,
                        1.0,
                    ],
                    depth_meters: 0.8 / 1000.0,
                    roundness: 0.6,
                });
            }
        } else {
            x += 1;
        }
    }

    let mut y = 0;
    while y < h {
        let row_fill: usize = (0..w).filter(|&x| solid(x, y)).count();
        if row_fill > w / 2 {
            let bar_start = y;
            while y < h && (0..w).filter(|&x| solid(x, y)).count() > w / 2 {
                y += 1;
            }
            let bar_end = y;
            let bh = bar_end - bar_start;
            if bh < h / 5 && bh >= 1 {
                priors.push(Morphometric {
                    bbox: [0.0, bar_start as f32 / h as f32, 1.0, bar_end as f32 / h as f32],
                    depth_meters: 0.5 / 1000.0,
                    roundness: 0.3,
                });
            }
        } else {
            y += 1;
        }
    }

    let mut visited = vec![false; w * h];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for x in 0..w {
        if !solid(x, 0) {
            stack.push((x, 0));
        }
        if !solid(x, h - 1) {
            stack.push((x, h - 1));
        }
    }
    for y in 0..h {
        if !solid(0, y) {
            stack.push((0, y));
        }
        if !solid(w - 1, y) {
            stack.push((w - 1, y));
        }
    }
    while let Some((px, py)) = stack.pop() {
        if px >= w || py >= h {
            continue;
        }
        let idx = py * w + px;
        if visited[idx] || solid(px, py) {
            continue;
        }
        visited[idx] = true;
        if px > 0 {
            stack.push((px - 1, py));
        }
        if px < w - 1 {
            stack.push((px + 1, py));
        }
        if py > 0 {
            stack.push((px, py - 1));
        }
        if py < h - 1 {
            stack.push((px, py + 1));
        }
    }

    let mut counter_min_x = w;
    let mut counter_max_x = 0usize;
    let mut counter_min_y = h;
    let mut counter_max_y = 0usize;
    let mut has_counter = false;
    for cy in 0..h {
        for cx in 0..w {
            let idx = cy * w + cx;
            if !visited[idx] && !solid(cx, cy) {
                has_counter = true;
                counter_min_x = counter_min_x.min(cx);
                counter_max_x = counter_max_x.max(cx);
                counter_min_y = counter_min_y.min(cy);
                counter_max_y = counter_max_y.max(cy);
            }
        }
    }
    if has_counter && counter_max_x > counter_min_x && counter_max_y > counter_min_y {
        priors.push(Morphometric {
            bbox: [
                counter_min_x as f32 / w as f32,
                counter_min_y as f32 / h as f32,
                (counter_max_x + 1) as f32 / w as f32,
                (counter_max_y + 1) as f32 / h as f32,
            ],
            depth_meters: 0.1 / 1000.0,
            roundness: 0.0,
        });
    }

    if priors.is_empty() {
        priors.push(Morphometric {
            bbox: [0.0, 0.0, 1.0, 1.0],
            depth_meters: 0.6 / 1000.0,
            roundness: 0.4,
        });
    }

    priors
}

/// Generate depth from morphometric depth priors.
fn depth_from_morphometrics(
    morphometrics: &[Morphometric],
    width: u16,
    height: u16,
) -> Vec<f32> {
    let w = width as f32;
    let h = height as f32;
    let mut depth = vec![0.0f32; (width as usize) * (height as usize)];

    for m in morphometrics {
        let x_min = (m.bbox[0] * w) as i32;
        let y_min = (m.bbox[1] * h) as i32;
        let x_max = (m.bbox[2] * w) as i32;
        let y_max = (m.bbox[3] * h) as i32;
        let cx = (x_min + x_max) as f32 / 2.0;
        let cy = (y_min + y_max) as f32 / 2.0;
        let rx = (x_max - x_min) as f32 / 2.0;
        let ry = (y_max - y_min) as f32 / 2.0;

        for y in y_min.max(0)..(y_max.min(height as i32)) {
            for x in x_min.max(0)..(x_max.min(width as i32)) {
                let dx = (x as f32 - cx) / rx.max(1.0);
                let dy = (y as f32 - cy) / ry.max(1.0);
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 1.0 {
                    let profile = if m.roundness > 0.01 {
                        (1.0 - dist * dist).sqrt() * m.roundness
                            + (1.0 - dist) * (1.0 - m.roundness)
                    } else {
                        1.0 - dist
                    };

                    let d = profile * m.depth_meters;
                    let idx = (y as u32 * width as u32 + x as u32) as usize;
                    depth[idx] = depth[idx].max(d);
                }
            }
        }
    }

    depth
}

/// Weighted merge of coarse and fine depth maps.
fn merge_depth(coarse: &[f32], fine: &[f32], coarse_weight: f32, fine_weight: f32) -> Vec<f32> {
    assert_eq!(coarse.len(), fine.len());

    let total = coarse_weight + fine_weight;
    coarse
        .iter()
        .zip(fine.iter())
        .map(|(&c, &f)| (c * coarse_weight + f * fine_weight) / total)
        .collect()
}

/// Solve glyph relief from coverage bitmap.
///
/// Input: coverage bitmap (0..255, where 255=fully opaque). Width and height in texels.
/// Output: GlyphRelief with normals (octahedral encoded) and depth (0..10000 permyriad).
///
/// Pipeline:
/// 1. Sobel gradient on coverage → rough surface normals.
/// 2. Poisson surface integration → fine depth map.
/// 3. Morphometric analysis → coarse depth prior.
/// 4. Merge coarse+fine → final depth.
/// 5. Encode normals as octahedral + quantize depth to u16 permyriad.
pub fn solve_relief(coverage: &[u8], width: u16, height: u16) -> GlyphRelief {
    let w = width as usize;
    let h = height as usize;
    assert_eq!(coverage.len(), w * h, "coverage len mismatch");

    let normals_f32 = normals_from_gradient_internal(coverage, width, height);
    let fine_depth = poisson_integrate(&normals_f32, w, h);

    let morphometrics = letter_morphometrics(coverage, width, height, 16.0);
    let coarse_depth = depth_from_morphometrics(&morphometrics, width, height);

    let max_coarse = coarse_depth.iter().cloned().fold(0.0f32, f32::max);
    let coarse_norm: Vec<f32> = if max_coarse > 1e-6 {
        coarse_depth.iter().map(|&d| d / max_coarse).collect()
    } else {
        coarse_depth.clone()
    };

    let merged = merge_depth(&coarse_norm, &fine_depth, 0.6, 0.4);

    let mut normals = Vec::with_capacity(w * h);
    let mut depth_pmy = Vec::with_capacity(w * h);

    for (i, &depth_f32) in merged.iter().enumerate() {
        let depth_clamped = depth_f32.clamp(0.0, 1.0);
        let depth_u16 = (depth_clamped * PMY_MAX as f32) as u16;
        depth_pmy.push(depth_u16);

        let normal_scale = OCT_SCALE as f32;
        let nx = (normals_f32[i].x * normal_scale) as i32;
        let ny = (normals_f32[i].y * normal_scale) as i32;
        let nz = (normals_f32[i].z * normal_scale) as i32;

        let (oct_u, oct_v) = encode_octahedral(nx, ny, nz);
        let normal_texel = NormalAlbedo8 {
            oct_u,
            oct_v,
            albedo_pmy: PMY_MAX,
            roughness_pmy: 0,
        };
        normals.push(normal_texel);
    }

    GlyphRelief {
        width,
        height,
        depth_pmy,
        normals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_coverage_yields_flat_normals() {
        let coverage = vec![128u8; 16 * 16];
        let relief = solve_relief(&coverage, 16, 16);

        assert_eq!(relief.width, 16);
        assert_eq!(relief.height, 16);
        assert_eq!(relief.normals.len(), 256);
        assert_eq!(relief.depth_pmy.len(), 256);

        for normal in &relief.normals {
            assert_eq!(normal.oct_u, NormalAlbedo8::FLAT.oct_u);
            assert_eq!(normal.oct_v, NormalAlbedo8::FLAT.oct_v);
        }

        let avg_depth: f32 = relief.depth_pmy.iter().map(|&d| d as f32).sum::<f32>() / relief.depth_pmy.len() as f32;
        assert!(avg_depth < PMY_MAX as f32 / 2.0, "flat coverage should have moderate depth");
    }

    #[test]
    fn bright_blob_has_peaked_depth() {
        let mut coverage = vec![0u8; 32 * 32];
        let center_x = 16usize;
        let center_y = 16usize;
        for y in 0..32 {
            for x in 0..32 {
                let dx = (x as i32 - center_x as i32) as f32;
                let dy = (y as i32 - center_y as i32) as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 8.0 {
                    coverage[y * 32 + x] = (255.0 * (1.0 - dist / 8.0)) as u8;
                }
            }
        }

        let relief = solve_relief(&coverage, 32, 32);
        let center_idx = center_y * 32 + center_x;
        let center_depth = relief.depth_pmy[center_idx];

        let corner_depth = relief.depth_pmy[0];
        assert!(
            center_depth > corner_depth,
            "center depth {} should exceed corner depth {}",
            center_depth,
            corner_depth
        );
    }

    #[test]
    fn depth_bounded_by_pmy_max() {
        let coverage = vec![255u8; 64 * 64];
        let relief = solve_relief(&coverage, 64, 64);

        for &depth in &relief.depth_pmy {
            assert!(depth <= PMY_MAX, "depth {} exceeds PMY_MAX {}", depth, PMY_MAX);
        }
    }

    #[test]
    fn same_input_yields_identical_output() {
        let coverage = vec![128u8; 24 * 24];

        let relief1 = solve_relief(&coverage, 24, 24);
        let relief2 = solve_relief(&coverage, 24, 24);

        assert_eq!(relief1.depth_pmy, relief2.depth_pmy);
        for (n1, n2) in relief1.normals.iter().zip(relief2.normals.iter()) {
            assert_eq!(n1.oct_u, n2.oct_u);
            assert_eq!(n1.oct_v, n2.oct_v);
        }
    }
}
