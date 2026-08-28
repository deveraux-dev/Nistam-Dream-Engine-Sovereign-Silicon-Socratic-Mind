//! SH9 projection and evaluation — 9 real spherical-harmonic bands (l = 0..2)
//! sampled on a Fibonacci sphere. Ported from v2
//! `public-tools/forge-ibl-bake/src/sh.rs:16-62`; the `HdrImage` argument is
//! generalized to a radiance closure so the analytic sky projects directly.

use core::f32::consts::PI;

/// `Y00 = 0.5 sqrt(1/pi)`.
const K0: f32 = 0.282_094_8;
/// `Y1m = 0.5 sqrt(3/pi)`.
const K1: f32 = 0.488_602_5;
/// `Y2{-2,-1,1}`.
const K2A: f32 = 1.092_548_4;
/// `Y20`.
const K2B: f32 = 0.315_391_6;
/// `Y22`.
const K2C: f32 = 0.546_274_2;
/// `pi(3 - sqrt 5)` radians.
const GOLDEN_ANGLE: f32 = 2.399_963_2;

/// Coefficient count: bands `l = 0..2`.
pub const SH9_COEFFS: usize = 9;

/// Nine RGB spherical-harmonic coefficients.
pub type Sh9 = [[f32; 3]; SH9_COEFFS];

/// The `i`-th of `n` equal-solid-angle Fibonacci-sphere directions.
/// Deterministic — no RNG, so a bake is reproducible.
#[inline]
pub fn fibonacci_dir(i: u32, n: u32) -> [f32; 3] {
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
    let r = (1.0 - y * y).max(0.0).sqrt();
    let phi = i as f32 * GOLDEN_ANGLE;
    [r * phi.cos(), y, r * phi.sin()]
}

/// The nine real SH basis values at a unit direction.
#[inline]
pub fn sh9_basis(dir: [f32; 3]) -> [f32; SH9_COEFFS] {
    let [x, y, z] = dir;
    [
        K0,
        K1 * y,
        K1 * z,
        K1 * x,
        K2A * x * y,
        K2A * y * z,
        K2B * (3.0 * z * z - 1.0),
        K2A * x * z,
        K2C * (x * x - y * y),
    ]
}

/// Project a radiance field onto SH9. `radiance(dir)` is asked for linear HDR
/// RGB at each Fibonacci direction. `samples` of 256 or more is the donor's
/// stated accuracy target.
pub fn project_sh9<F>(samples: u32, radiance: F) -> Sh9
where
    F: Fn([f32; 3]) -> [f32; 3],
{
    let mut sh = [[0.0f32; 3]; SH9_COEFFS];
    if samples == 0 {
        return sh;
    }
    for i in 0..samples {
        let dir = fibonacci_dir(i, samples);
        let rad = radiance(dir);
        let basis = sh9_basis(dir);
        for (c, &b) in basis.iter().enumerate() {
            sh[c][0] += rad[0] * b;
            sh[c][1] += rad[1] * b;
            sh[c][2] += rad[2] * b;
        }
    }
    // Equal-solid-angle Monte-Carlo weight: the integral is (4 pi / N) times the sum.
    let w = 4.0 * PI / samples as f32;
    for coeff in &mut sh {
        coeff[0] *= w;
        coeff[1] *= w;
        coeff[2] *= w;
    }
    sh
}

/// Reconstruct irradiance from SH9 at a direction. Clamped at zero — a
/// truncated SH series can ring negative, and negative light is not a colour.
pub fn eval_sh9(sh: &Sh9, dir: [f32; 3]) -> [f32; 3] {
    let basis = sh9_basis(dir);
    let mut out = [0.0f32; 3];
    for (c, &b) in basis.iter().enumerate() {
        out[0] += sh[c][0] * b;
        out[1] += sh[c][1] * b;
        out[2] += sh[c][2] * b;
    }
    [out[0].max(0.0), out[1].max(0.0), out[2].max(0.0)]
}

/// SH9 evaluated once per direction on the 26-point trit lattice — the static
/// lookup that replaces per-fragment ambient maths. Index with
/// [`crate::TritDir::0`]; the origin slot is zero and never a direction.
pub fn bake_trit_ambient(sh: &Sh9) -> [[f32; 3]; crate::trit_dir::DIR_STATES as usize] {
    let mut table = [[0.0f32; 3]; crate::trit_dir::DIR_STATES as usize];
    for d in crate::trit_dir::all_directions() {
        table[d.0 as usize] = eval_sh9(sh, d.to_unit());
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: [f32; 3]) -> f32 {
        c[0] + c[1] + c[2]
    }

    #[test]
    fn fibonacci_directions_are_unit_length() {
        for i in 0..256 {
            let d = fibonacci_dir(i, 256);
            let m = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            assert!((m - 1.0).abs() < 1e-4, "sample {i} magnitude {m}");
        }
    }

    #[test]
    fn fibonacci_covers_both_poles_and_averages_to_the_centre() {
        let n = 512;
        let mut sum = [0.0f32; 3];
        let (mut lo, mut hi) = (1.0f32, -1.0f32);
        for i in 0..n {
            let d = fibonacci_dir(i, n);
            sum[0] += d[0];
            sum[1] += d[1];
            sum[2] += d[2];
            lo = lo.min(d[1]);
            hi = hi.max(d[1]);
        }
        assert!(lo < -0.99 && hi > 0.99, "poles uncovered: {lo}..{hi}");
        for c in sum {
            assert!((c / n as f32).abs() < 0.01, "sphere is not balanced: {c}");
        }
    }

    #[test]
    fn a_uniform_field_projects_to_its_own_average() {
        // The DC term of a constant field reconstructs that constant.
        let sh = project_sh9(1024, |_| [0.5, 0.25, 0.75]);
        let got = eval_sh9(&sh, [0.0, 1.0, 0.0]);
        for (g, w) in got.iter().zip([0.5f32, 0.25, 0.75]) {
            assert!((g - w).abs() < 0.05, "uniform field lost its value: {got:?}");
        }
    }

    #[test]
    fn a_directional_field_is_brightest_toward_its_own_lobe() {
        // Radiance concentrated at +Y must reconstruct brighter at +Y than -Y.
        let sh = project_sh9(1024, |d| {
            let up = d[1].max(0.0);
            [up, up, up]
        });
        let top = eval_sh9(&sh, [0.0, 1.0, 0.0]);
        let bottom = eval_sh9(&sh, [0.0, -1.0, 0.0]);
        assert!(lum(top) > lum(bottom), "top {top:?} bottom {bottom:?}");
    }

    #[test]
    fn projection_is_deterministic() {
        let f = |d: [f32; 3]| [d[1].abs(), 0.3, 0.7];
        assert_eq!(project_sh9(256, f), project_sh9(256, f), "same input, same coefficients");
    }

    #[test]
    fn zero_samples_yields_zero_rather_than_a_divide_by_zero() {
        assert_eq!(project_sh9(0, |_| [1.0, 1.0, 1.0]), [[0.0; 3]; SH9_COEFFS]);
    }

    #[test]
    fn evaluation_never_returns_negative_light() {
        // A hard one-sided field makes the truncated series ring; the clamp is
        // what keeps a negative colour off the sheet.
        let sh = project_sh9(512, |d| if d[1] > 0.7 { [8.0, 8.0, 8.0] } else { [0.0; 3] });
        for d in crate::trit_dir::all_directions() {
            let c = eval_sh9(&sh, d.to_unit());
            assert!(c.iter().all(|v| *v >= 0.0), "negative light at dir {}: {c:?}", d.0);
        }
    }

    #[test]
    fn the_trit_table_matches_direct_evaluation_and_leaves_the_origin_dark() {
        let sh = project_sh9(256, |d| [d[0].abs(), d[1].abs(), d[2].abs()]);
        let table = bake_trit_ambient(&sh);
        for d in crate::trit_dir::all_directions() {
            assert_eq!(table[d.0 as usize], eval_sh9(&sh, d.to_unit()), "dir {}", d.0);
        }
        assert_eq!(
            table[crate::trit_dir::DIR_ORIGIN as usize],
            [0.0; 3],
            "the origin is not a direction and must stay dark"
        );
    }
}
