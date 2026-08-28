//! Pixel-art tween interpolation between two RGBA frame buffers.
//!
//! Uses nearest-neighbor (hard-cut at midpoint) to preserve pixel-art crispness.
//! `t` is a `Permyriad` (0..=10000): 0 = full `a`, 10000 = full `b`.
//!
//! FUTURE: add 4x4 Bayer ordered-dither blend in the 0.4..0.6 midzone
//!         (`t.0` in 4000..6000) for smooth pixel-art crossfades.

use forge_core_v3::fixed_point::Permyriad;

/// Blend two RGBA frame buffers using nearest-neighbor pixel selection.
///
/// - `t = 0`     → returns a copy of `a`
/// - `t = 10000` → returns a copy of `b`
/// - `t < 5000`  → picks `a`; `t >= 5000` → picks `b`
///
/// Returns `Err` if either input length mismatches `width * height * 4`.
pub fn tween_between(
    a: &[u8],
    b: &[u8],
    width: u32,
    height: u32,
    t: Permyriad,
) -> Result<Vec<u8>, &'static str> {
    let expected = (width as usize) * (height as usize) * 4;
    if a.len() != expected || b.len() != expected {
        return Err("input length mismatch");
    }
    if t.0 < 5000 {
        Ok(a.to_vec())
    } else {
        Ok(b.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a solid-color 2x2 RGBA buffer for testing.
    fn solid(r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
        vec![r, g, b, a, r, g, b, a, r, g, b, a, r, g, b, a]
    }

    /// L07: determinism test — same inputs must produce same output.
    #[test]
    fn tween_between_is_deterministic() {
        let a = solid(255, 0, 0, 255);
        let b = solid(0, 0, 255, 255);
        let t = Permyriad(7500);
        let r1 = tween_between(&a, &b, 2, 2, t).unwrap();
        let r2 = tween_between(&a, &b, 2, 2, t).unwrap();
        assert_eq!(r1, r2, "tween_between must be deterministic");
    }

    #[test]
    fn tween_at_zero_returns_a() {
        let a = solid(255, 0, 0, 255);
        let b = solid(0, 0, 255, 255);
        let out = tween_between(&a, &b, 2, 2, Permyriad(0)).unwrap();
        assert_eq!(out, a, "t=0 must return a exactly");
    }

    #[test]
    fn tween_at_max_returns_b() {
        let a = solid(255, 0, 0, 255);
        let b = solid(0, 0, 255, 255);
        let out = tween_between(&a, &b, 2, 2, Permyriad(10000)).unwrap();
        assert_eq!(out, b, "t=10000 must return b exactly");
    }

    /// L18: sabotage test — verify midpoint threshold by checking the boundary.
    #[test]
    fn tween_threshold_at_5000() {
        let a = solid(255, 0, 0, 255);
        let b = solid(0, 0, 255, 255);

        // t=4999 should return a
        let out_before = tween_between(&a, &b, 2, 2, Permyriad(4999)).unwrap();
        assert_eq!(out_before, a, "t=4999 (below threshold) must return a");

        // t=5000 should return b
        let out_at = tween_between(&a, &b, 2, 2, Permyriad(5000)).unwrap();
        assert_eq!(out_at, b, "t=5000 (at threshold) must return b");
    }

    #[test]
    fn tween_with_mismatched_lengths_errors() {
        let a = vec![255u8; 16]; // 2x2 = 16 bytes
        let b = vec![0u8; 32];   // 4x2 = 32 bytes (wrong)
        let result = tween_between(&a, &b, 2, 2, Permyriad(0));
        assert!(result.is_err(), "mismatched buffer length must error");
    }

    /// L07: bijection test — output length must always match input length.
    #[test]
    fn tween_preserves_buffer_length() {
        for t_val in [0, 4999, 5000, 10000] {
            let a = solid(255, 0, 0, 255);
            let b = solid(0, 0, 255, 255);
            let out = tween_between(&a, &b, 2, 2, Permyriad(t_val)).unwrap();
            assert_eq!(out.len(), 16, "output must always be width*height*4 bytes");
        }
    }
}
