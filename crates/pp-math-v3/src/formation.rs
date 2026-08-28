//! Formation geometry primitives (Sean 07-21, "geometry, needs to be primitive"): rule-of-thirds
//! power points (Smith 1797) · quincunx five-spot (Browne 1658) · superior-dexter quadrant
//! (Fox-Davies heraldry; twin: forge-cart-brain SieveGeometry) · Yod apex (60/150 aspect) · Weyl stride.

use crate::fixed_point::MilliUnit;

/// Angular wrap base in millidegrees — theta-lane convention (forge-audio/forge-ml parity).
pub const WRAP_MDEG: i64 = 360_000;

fn wrap(x: i64) -> i64 {
    x.rem_euclid(WRAP_MDEG)
}

/// Signed shortest angular difference b-a in (-WRAP/2, WRAP/2].
pub fn wrap_delta_mdeg(a: i64, b: i64) -> i64 {
    let d = wrap(b - a);
    if d > WRAP_MDEG / 2 { d - WRAP_MDEG } else { d }
}

/// Rule-of-thirds power points: the 4 intersections of the 1/3 grid over the span.
pub fn thirds_points(
    x0: MilliUnit, x1: MilliUnit, y0: MilliUnit, y1: MilliUnit,
) -> [(MilliUnit, MilliUnit); 4] {
    let (w, h) = (x1.0 - x0.0, y1.0 - y0.0);
    let xs = [x0.0 + w / 3, x0.0 + 2 * w / 3];
    let ys = [y0.0 + h / 3, y0.0 + 2 * h / 3];
    [
        (MilliUnit(xs[0]), MilliUnit(ys[0])),
        (MilliUnit(xs[1]), MilliUnit(ys[0])),
        (MilliUnit(xs[0]), MilliUnit(ys[1])),
        (MilliUnit(xs[1]), MilliUnit(ys[1])),
    ]
}

/// Quincunx five-spot: 4 corners + center (die-face 5; the 5D box's planar shadow).
pub fn quincunx_points(
    x0: MilliUnit, x1: MilliUnit, y0: MilliUnit, y1: MilliUnit,
) -> [(MilliUnit, MilliUnit); 5] {
    [
        (x0, y0),
        (x1, y0),
        (x0, y1),
        (x1, y1),
        (MilliUnit((x0.0 + x1.0) / 2), MilliUnit((y0.0 + y1.0) / 2)),
    ]
}

/// Superior-dexter quadrant. Heraldic dexter = bearer's right = VIEWER'S LEFT; superior =
/// chief/top. Screen y grows downward, so this is [x0, midx] x [y0, midy].
pub fn superior_dexter(
    x0: MilliUnit, x1: MilliUnit, y0: MilliUnit, y1: MilliUnit,
) -> (MilliUnit, MilliUnit, MilliUnit, MilliUnit) {
    (x0, MilliUnit((x0.0 + x1.0) / 2), y0, MilliUnit((y0.0 + y1.0) / 2))
}

/// Yod ("finger of God"): a,b sextile (60° ± tol), both quincunx (150°) to the apex —
/// apex = wrapped midpoint of a,b rotated 180°. None when a,b are not sextile.
pub fn yod_apex_mdeg(a: i64, b: i64, tol: i64) -> Option<i64> {
    let d = wrap_delta_mdeg(a, b);
    if (d.abs() - 60_000).abs() > tol {
        return None;
    }
    Some(wrap(a + d / 2 + 180_000))
}

/// Weyl/Kronecker low-discrepancy stride bucket: index i lands at (i * s) % n with s an odd
/// coprime near 2n/3 — mass spreads rule-of-thirds-wide instead of piling at bucket 0.
pub fn thirds_stride_bucket(i: usize, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut s = ((2 * n) / 3) | 1;
    while gcd(s, n) != 1 {
        s += 2;
    }
    i.wrapping_mul(s) % n
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mu(v: i64) -> MilliUnit {
        MilliUnit(v)
    }

    #[test]
    fn yod_apex_resolves_across_the_wrap_seam() {
        assert_eq!(yod_apex_mdeg(350_000, 50_000, 2_000), Some(200_000));
        assert_eq!(yod_apex_mdeg(0, 60_000, 2_000), Some(210_000));
    }

    #[test]
    fn yod_rejects_non_sextile_anchors() {
        assert_eq!(yod_apex_mdeg(0, 90_000, 2_000), None);
        assert_eq!(yod_apex_mdeg(0, 0, 2_000), None);
    }

    #[test]
    fn quincunx_is_four_corners_plus_true_center() {
        let p = quincunx_points(mu(0), mu(1_000), mu(0), mu(2_000));
        assert_eq!(p[4], (mu(500), mu(1_000)));
        let mut uniq: Vec<_> = p.to_vec();
        uniq.dedup();
        assert_eq!(uniq.len(), 5);
    }

    #[test]
    fn superior_dexter_is_viewer_top_left() {
        let (qx0, qx1, qy0, qy1) = superior_dexter(mu(0), mu(1_000), mu(0), mu(2_000));
        assert_eq!((qx0, qx1, qy0, qy1), (mu(0), mu(500), mu(0), mu(1_000)));
    }

    #[test]
    fn thirds_points_sit_inside_the_span() {
        for (x, y) in thirds_points(mu(0), mu(900), mu(0), mu(300)) {
            assert!(x.0 > 0 && x.0 < 900 && y.0 > 0 && y.0 < 300);
        }
    }

    #[test]
    fn thirds_stride_spreads_short_runs_wide() {
        let n = 512;
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..25 {
            let k = thirds_stride_bucket(i, n);
            assert!(k < n);
            seen.insert(k);
        }
        assert_eq!(seen.len(), 25, "25 indices must land in 25 distinct buckets");
        assert!(*seen.iter().max().unwrap() > n / 2, "spread must reach past midframe");
    }
}
