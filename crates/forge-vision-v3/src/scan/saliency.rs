//! From F:\NewRepo\crates\forge-vision\src\scan\saliency.rs (lines 1-284)
//! Spectral Residual saliency (Hou & Zhang, CVPR 2007).

use rustfft::{num_complex::Complex, FftPlanner};

const SR_GRID: usize = 64;

/// Saliency map and the peak statistics the input gate consumes.
#[derive(Debug, Clone)]
pub struct SaliencyMap {
    pub width: u32,
    pub height: u32,
    /// Row-major, `0..=255`. Higher = more salient.
    pub map: Vec<u8>,
    /// Fraction of pixels above the high-saliency floor, in permyriad.
    pub salient_fraction_pmy: u32,
    /// Peak saliency, `0..=255`.
    pub peak: u8,
}

/// Compute a spectral-residual saliency map for `gray`.
pub fn spectral_residual(gray: &[u8], width: u32, height: u32) -> SaliencyMap {
    let w = width as usize;
    let h = height as usize;
    assert_eq!(gray.len(), w * h, "saliency: gray length mismatch");

    if w == 0 || h == 0 {
        return SaliencyMap {
            width,
            height,
            map: Vec::new(),
            salient_fraction_pmy: 0,
            peak: 0,
        };
    }

    let mut work = vec![Complex::new(0.0f64, 0.0); SR_GRID * SR_GRID];
    for y in 0..SR_GRID {
        let sy = (y * h / SR_GRID).min(h - 1);
        for x in 0..SR_GRID {
            let sx = (x * w / SR_GRID).min(w - 1);
            work[y * SR_GRID + x] =
                Complex::new(gray[sy * w + sx] as f64, 0.0);
        }
    }

    let v0 = work[0].re;
    if work.iter().all(|c| (c.re - v0).abs() < 1e-12) {
        return SaliencyMap {
            width,
            height,
            map: vec![0u8; w * h],
            salient_fraction_pmy: 0,
            peak: 0,
        };
    }

    let mut planner = FftPlanner::<f64>::new();
    let fft_fwd = planner.plan_fft_forward(SR_GRID);
    let fft_inv = planner.plan_fft_inverse(SR_GRID);

    fft2d(&mut work, &*fft_fwd);

    let mut log_mag = vec![0.0f64; SR_GRID * SR_GRID];
    for i in 0..log_mag.len() {
        let m = work[i].norm();
        log_mag[i] = (m + 1e-12).ln();
    }
    let smoothed = box_blur_3x3(&log_mag, SR_GRID, SR_GRID);
    for i in 0..work.len() {
        if i == 0 {
            work[i] = Complex::new(0.0, 0.0);
            continue;
        }
        let residual = log_mag[i] - smoothed[i];
        let new_mag = residual.exp();
        let phase = work[i].arg();
        work[i] = Complex::new(new_mag * phase.cos(), new_mag * phase.sin());
    }

    fft2d(&mut work, &*fft_inv);
    let scale = 1.0 / (SR_GRID * SR_GRID) as f64;
    let mut saliency = vec![0.0f64; SR_GRID * SR_GRID];
    for i in 0..work.len() {
        let v = work[i] * scale;
        saliency[i] = v.re * v.re + v.im * v.im;
    }

    let saliency = box_blur_3x3(&saliency, SR_GRID, SR_GRID);

    let max = saliency.iter().cloned().fold(0.0f64, f64::max).max(1e-12);
    let small: Vec<u8> = saliency
        .iter()
        .map(|&v| ((v / max) * 255.0).clamp(0.0, 255.0) as u8)
        .collect();

    let mut out = vec![0u8; w * h];
    for y in 0..h {
        let sy = (y * SR_GRID / h.max(1)).min(SR_GRID - 1);
        for x in 0..w {
            let sx = (x * SR_GRID / w.max(1)).min(SR_GRID - 1);
            out[y * w + x] = small[sy * SR_GRID + sx];
        }
    }

    let floor: u8 = 96;
    let salient_pixels = out.iter().filter(|&&v| v >= floor).count() as u64;
    let salient_fraction_pmy =
        ((salient_pixels * 10_000) / out.len().max(1) as u64) as u32;
    let peak = out.iter().copied().max().unwrap_or(0);

    SaliencyMap {
        width,
        height,
        map: out,
        salient_fraction_pmy,
        peak,
    }
}

fn fft2d(buf: &mut [Complex<f64>], plan: &dyn rustfft::Fft<f64>) {
    let n = SR_GRID;
    let scratch_len = plan.get_inplace_scratch_len();
    let mut scratch = vec![Complex::new(0.0, 0.0); scratch_len];

    for row in 0..n {
        let start = row * n;
        plan.process_with_scratch(&mut buf[start..start + n], &mut scratch);
    }

    let mut col = vec![Complex::new(0.0, 0.0); n];
    for c in 0..n {
        for r in 0..n {
            col[r] = buf[r * n + c];
        }
        plan.process_with_scratch(&mut col, &mut scratch);
        for r in 0..n {
            buf[r * n + c] = col[r];
        }
    }
}

fn box_blur_3x3(src: &[f64], w: usize, h: usize) -> Vec<f64> {
    let mut out = vec![0.0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0f64;
            let mut count = 0.0f64;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    sum += src[(ny as usize) * w + nx as usize];
                    count += 1.0;
                }
            }
            out[y * w + x] = sum / count;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_spot(w: u32, h: u32, cx: u32, cy: u32, radius: u32) -> Vec<u8> {
        let mut gray = vec![32u8; (w * h) as usize];
        let r2 = (radius * radius) as i32;
        for y in 0..h {
            for x in 0..w {
                let dx = x as i32 - cx as i32;
                let dy = y as i32 - cy as i32;
                if dx * dx + dy * dy <= r2 {
                    gray[(y * w + x) as usize] = 240;
                }
            }
        }
        gray
    }

    #[test]
    fn output_shape_matches_input() {
        let gray = single_spot(96, 64, 48, 32, 8);
        let s = spectral_residual(&gray, 96, 64);
        assert_eq!(s.width, 96);
        assert_eq!(s.height, 64);
        assert_eq!(s.map.len(), 96 * 64);
    }

    #[test]
    fn uniform_grey_is_finite() {
        let gray = vec![128u8; 96 * 96];
        let s = spectral_residual(&gray, 96, 96);
        assert!(s.map.iter().all(|&v| v == 0));
        assert_eq!(s.peak, 0);
        assert_eq!(s.salient_fraction_pmy, 0);
    }

    #[test]
    fn single_spot_peak_is_near_spot() {
        let (w, h) = (96u32, 96u32);
        let gray = single_spot(w, h, 24, 24, 6);
        let s = spectral_residual(&gray, w, h);
        let (mi, _) = s
            .map
            .iter()
            .enumerate()
            .max_by_key(|&(_, v)| *v)
            .unwrap();
        let px = (mi as u32) % w;
        let py = (mi as u32) / w;
        assert!(px < w / 2, "peak x={} should be in left half", px);
        assert!(py < h / 2, "peak y={} should be in top half", py);
        assert!(s.peak >= 32, "expected at least mid saliency, got {}", s.peak);
    }

    #[test]
    fn determinism() {
        let gray = single_spot(80, 80, 40, 40, 10);
        let a = spectral_residual(&gray, 80, 80);
        let b = spectral_residual(&gray, 80, 80);
        assert_eq!(a.map, b.map);
        assert_eq!(a.salient_fraction_pmy, b.salient_fraction_pmy);
    }

    #[test]
    fn salient_fraction_responds_to_content() {
        let blank = vec![128u8; 96 * 96];
        let with_spot = single_spot(96, 96, 48, 48, 12);
        let s0 = spectral_residual(&blank, 96, 96);
        let s1 = spectral_residual(&with_spot, 96, 96);
        assert_eq!(s0.salient_fraction_pmy, 0);
        assert!(
            s1.salient_fraction_pmy > 0,
            "expected non-zero salient fraction, got {}",
            s1.salient_fraction_pmy
        );
    }

    #[test]
    fn tiny_input_does_not_panic() {
        let gray = single_spot(8, 8, 4, 4, 2);
        let s = spectral_residual(&gray, 8, 8);
        assert_eq!(s.map.len(), 64);
    }
}
