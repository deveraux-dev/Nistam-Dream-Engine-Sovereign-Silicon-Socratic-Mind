//! Dither — a 4x4 Bayer ordered-dither matrix (integer). For the parchment/
//! halftone wash on the canvas page.

/// The classic 4x4 Bayer matrix (values 0..15).
pub const BAYER4: [[u8; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

/// The dither threshold at `(x, y)` in permyriad (0..~10000).
pub fn threshold_pmy(x: u32, y: u32) -> u32 {
    BAYER4[(y % 4) as usize][(x % 4) as usize] as u32 * 10_000 / 16
}

/// Should the pixel at `(x, y)` be on, given a `value_pmy` intensity?
pub fn on(value_pmy: u32, x: u32, y: u32) -> bool {
    value_pmy > threshold_pmy(x, y)
}

/// The classic 8x8 Bayer matrix (values 0..63) for high-fidelity spatial ordered dithering.
pub const BAYER8: [[u8; 8]; 8] = [
    [ 0, 48, 12, 60,  3, 51, 15, 63],
    [32, 16, 44, 28, 35, 19, 47, 31],
    [ 8, 56,  4, 52, 11, 59,  7, 55],
    [40, 24, 36, 20, 43, 27, 39, 23],
    [ 2, 50, 14, 62,  1, 49, 13, 61],
    [34, 18, 46, 30, 33, 17, 45, 29],
    [10, 58,  6, 54,  9, 57,  5, 53],
    [42, 26, 38, 22, 41, 25, 37, 21],
];

/// Dual 8x8 Glaze Opacity Lookup Table (values 0..10_000, permyriad).
/// Provides spatial opacity variance behaving like a hand-applied glaze overlay.
pub const GLAZE_OPACITY_LUT: [[u16; 8]; 8] = [
    [8000, 7200, 8500, 6800, 9000, 7500, 8200, 7000],
    [6500, 9500, 6000, 8200, 7000, 8800, 6300, 8000],
    [7800, 6900, 9200, 7100, 8300, 6600, 9000, 7400],
    [8900, 7600, 6400, 8500, 6100, 8100, 6800, 9300],
    [7100, 8300, 8800, 6200, 7900, 9100, 7300, 6600],
    [9200, 6000, 7500, 8000, 6700, 8400, 9500, 7800],
    [6300, 8700, 7000, 9300, 7400, 6900, 8100, 6200],
    [8000, 7200, 8600, 6500, 9000, 7600, 6300, 8800],
];

/// The 8x8 dither threshold at `(x, y)` in permyriad (0..~10000).
pub fn threshold8_pmy(x: u32, y: u32) -> u32 {
    BAYER8[(y % 8) as usize][(x % 8) as usize] as u32 * 10_000 / 64
}

/// Should the pixel at `(x, y)` be glazed (rendered with overlay), given a baseline
/// `glaze_intensity_pmy` and spatial variance modulator?
pub fn on_glaze(glaze_intensity_pmy: u32, x: u32, y: u32) -> bool {
    let opacity_mod = GLAZE_OPACITY_LUT[(y % 8) as usize][(x % 8) as usize] as u32;
    // Dual array logic: combine spatial opacity modulator and dither threshold
    let adjusted_intensity = (glaze_intensity_pmy * opacity_mod) / 10_000;
    adjusted_intensity > threshold8_pmy(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_span_the_matrix() {
        assert_eq!(threshold_pmy(0, 0), 0); // smallest cell
        assert_eq!(threshold_pmy(0, 3), 15 * 10_000 / 16); // largest cell
    }

    #[test]
    fn full_intensity_all_on_zero_all_off() {
        for y in 0..4 {
            for x in 0..4 {
                assert!(on(10_000, x, y));
                assert!(!on(0, x, y));
            }
        }
    }

    #[test]
    fn mid_intensity_dithers() {
        // Half intensity turns roughly half the 16 cells on.
        let count = (0..4).flat_map(|y| (0..4).map(move |x| (x, y))).filter(|(x, y)| on(5_000, *x, *y)).count();
        assert!((6..=10).contains(&count));
    }

    #[test]
    fn bayer8_thresholds_span_the_matrix() {
        assert_eq!(threshold8_pmy(0, 0), 0);
        assert_eq!(threshold8_pmy(7, 0), 63 * 10_000 / 64);
    }

    #[test]
    fn bayer8_dual_array_glaze_coverage() {
        // Full baseline glaze intensity with full spatial opacity LUT modulation
        let active_glaze_pixels = (0..8)
            .flat_map(|y| (0..8).map(move |x| (x, y)))
            .filter(|(x, y)| on_glaze(10_000, *x, *y))
            .count();
        // Spatially modulated, not all 64 will be on due to GLAZE_OPACITY_LUT
        assert!(active_glaze_pixels < 64);
        assert!(active_glaze_pixels > 0);
    }
}
