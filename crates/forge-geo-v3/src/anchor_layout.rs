//! Morphometric anchor layout — the bridge that was missing.
//!
//! Places the 20 Mobometric bone anchors on a sprite **silhouette** by
//! COMPOSITION GEOMETRY, not a bounding box and not an AI guess:
//!
//! - **Central column + stepped bands**: the spine chain holds x=5000 dead
//!   centre; limb roots step outward from it (clavicle tails 3800/6200, arm
//!   tails 2800/7200, pelvis tails 4200/5800); head toward the top, feet at the
//!   floor. CORRECTED 2026-08-24 — this bullet used to read "Rule-of-Thirds …
//!   shoulders/hips on the thirds lines (3333 / 6667)". The template contains
//!   NEITHER value, nor 6666: the numbers are hand-tuned proportions, not
//!   `pp_math::formation::thirds_points` output. Read that as a warning, not an
//!   invitation — moving them onto true thirds would shift every bone of every
//!   already-rigged figure. The composition INTENT is thirds-like; the values
//!   are authored.
//! - **Quincunx**: Root (centre) + the four limb roots (clavicles, pelves) on the
//!   quadrant points — the 5-point body cross.
//! - **Watertight holds the bones in**: each anchor is clamped to *inside* the
//!   foreground silhouette (walk toward the centroid until contained), so no bone
//!   escapes the manifold.
//! - **Catmull-Rom drives the chains** downstream (`crate::bone_spline`) — the
//!   anchors here are its control points.
//!
//! Output = `SpatialAnchor`s ready for `rigging_pipeline::resolve_anchors`.
//! Integer permyriad template, deterministic. (Encounter geometry — Yod /
//! Finger-of-God / Superior Dexter — is a SEPARATE axis and lives in lore_sieve.)

use crate::rigging_pipeline::{BoneEndpoint, BoneId, SpatialAnchor};

/// Normalized composition template: `(bone, endpoint, x‰, y‰)` in permyriad
/// (x right, y down), scaled at layout time to the SILHOUETTE's bounding box —
/// not the image frame. The spine holds the centre column (5000); limb roots
/// step outward in authored bands. These are hand-tuned proportions: no entry
/// equals 3333, 6666 or 6667, so this table is not thirds-line output and must
/// not be "corrected" toward it.
const TEMPLATE: &[(BoneId, BoneEndpoint, i32, i32)] = &[
    // ── Spine chain — centre column ──────────────────────────────────────────
    (BoneId::Root,  BoneEndpoint::Head, 5000, 5500), (BoneId::Root,  BoneEndpoint::Tail, 5000, 5000),
    (BoneId::Spine, BoneEndpoint::Head, 5000, 5000), (BoneId::Spine, BoneEndpoint::Tail, 5000, 3800),
    (BoneId::Neck,  BoneEndpoint::Head, 5000, 3800), (BoneId::Neck,  BoneEndpoint::Tail, 5000, 3000),
    (BoneId::Head,  BoneEndpoint::Head, 5000, 3000), (BoneId::Head,  BoneEndpoint::Tail, 5000, 1500),
    // ── Left arm — left third, dropping to the lower-left quincunx point ──────
    (BoneId::LeftClavicle, BoneEndpoint::Head, 5000, 3800), (BoneId::LeftClavicle, BoneEndpoint::Tail, 3800, 3800),
    (BoneId::LeftUpperArm, BoneEndpoint::Head, 3800, 3800), (BoneId::LeftUpperArm, BoneEndpoint::Tail, 2800, 5000),
    (BoneId::LeftLowerArm, BoneEndpoint::Head, 2800, 5000), (BoneId::LeftLowerArm, BoneEndpoint::Tail, 2000, 6000),
    (BoneId::LeftHand,     BoneEndpoint::Head, 2000, 6000), (BoneId::LeftHand,     BoneEndpoint::Tail, 1700, 6500),
    // ── Right arm — mirror ───────────────────────────────────────────────────
    (BoneId::RightClavicle, BoneEndpoint::Head, 5000, 3800), (BoneId::RightClavicle, BoneEndpoint::Tail, 6200, 3800),
    (BoneId::RightUpperArm, BoneEndpoint::Head, 6200, 3800), (BoneId::RightUpperArm, BoneEndpoint::Tail, 7200, 5000),
    (BoneId::RightLowerArm, BoneEndpoint::Head, 7200, 5000), (BoneId::RightLowerArm, BoneEndpoint::Tail, 8000, 6000),
    (BoneId::RightHand,     BoneEndpoint::Head, 8000, 6000), (BoneId::RightHand,     BoneEndpoint::Tail, 8300, 6500),
    // ── Left leg — lower third ───────────────────────────────────────────────
    (BoneId::LeftPelvis, BoneEndpoint::Head, 5000, 5500), (BoneId::LeftPelvis, BoneEndpoint::Tail, 4200, 6000),
    (BoneId::LeftThigh,  BoneEndpoint::Head, 4200, 6000), (BoneId::LeftThigh,  BoneEndpoint::Tail, 4200, 7800),
    (BoneId::LeftCalf,   BoneEndpoint::Head, 4200, 7800), (BoneId::LeftCalf,   BoneEndpoint::Tail, 4200, 9000),
    (BoneId::LeftFoot,   BoneEndpoint::Head, 4200, 9000), (BoneId::LeftFoot,   BoneEndpoint::Tail, 4200, 9700),
    // ── Right leg — mirror ───────────────────────────────────────────────────
    (BoneId::RightPelvis, BoneEndpoint::Head, 5000, 5500), (BoneId::RightPelvis, BoneEndpoint::Tail, 5800, 6000),
    (BoneId::RightThigh,  BoneEndpoint::Head, 5800, 6000), (BoneId::RightThigh,  BoneEndpoint::Tail, 5800, 7800),
    (BoneId::RightCalf,   BoneEndpoint::Head, 5800, 7800), (BoneId::RightCalf,   BoneEndpoint::Tail, 5800, 9000),
    (BoneId::RightFoot,   BoneEndpoint::Head, 5800, 9000), (BoneId::RightFoot,   BoneEndpoint::Tail, 5800, 9700),
];

/// Foreground bounding box + centroid from a silhouette mask (`true` = figure).
/// Returns `(min_x, min_y, max_x, max_y, centroid_x, centroid_y)`, or `None` if
/// the mask is empty.
fn silhouette_extent(mask: &[bool], w: usize, h: usize) -> Option<(u32, u32, u32, u32, u32, u32)> {
    let (mut minx, mut miny, mut maxx, mut maxy) = (w, h, 0usize, 0usize);
    let (mut sx, mut sy, mut cnt) = (0u64, 0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            if mask[y * w + x] {
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
                sx += x as u64;
                sy += y as u64;
                cnt += 1;
            }
        }
    }
    if cnt == 0 {
        return None;
    }
    Some((minx as u32, miny as u32, maxx as u32, maxy as u32, (sx / cnt) as u32, (sy / cnt) as u32))
}

/// Watertight containment: if `(px,py)` is outside the silhouette, walk toward
/// the centroid until inside; fall back to the centroid. Keeps every bone in.
fn clamp_into(mask: &[bool], w: usize, h: usize, px: u32, py: u32, cx: u32, cy: u32) -> (u32, u32) {
    let inside = |x: u32, y: u32| (x as usize) < w && (y as usize) < h && mask[y as usize * w + x as usize];
    if inside(px, py) {
        return (px, py);
    }
    const STEPS: i64 = 64;
    for s in 1..=STEPS {
        let x = (px as i64 + (cx as i64 - px as i64) * s / STEPS) as u32;
        let y = (py as i64 + (cy as i64 - py as i64) * s / STEPS) as u32;
        if inside(x, y) {
            return (x, y);
        }
    }
    (cx, cy)
}

/// Per-arm-bone march fraction: `(is_left, t‰)` from the shoulder centre (t=0)
/// out to the detected hand tip (t=10000). `None` for non-arm bones — they keep
/// the composition template.
fn arm_frac(bone: BoneId, ep: BoneEndpoint) -> Option<(bool, i64)> {
    use BoneEndpoint::{Head, Tail};
    use BoneId::{
        LeftClavicle, LeftHand, LeftLowerArm, LeftUpperArm, RightClavicle, RightHand, RightLowerArm,
        RightUpperArm,
    };
    let v = match (bone, ep) {
        (LeftClavicle, Head) => (true, 0),
        (LeftClavicle, Tail) => (true, 1800),
        (LeftUpperArm, Head) => (true, 1800),
        (LeftUpperArm, Tail) => (true, 5000),
        (LeftLowerArm, Head) => (true, 5000),
        (LeftLowerArm, Tail) => (true, 8000),
        (LeftHand, Head) => (true, 8000),
        (LeftHand, Tail) => (true, 10000),
        (RightClavicle, Head) => (false, 0),
        (RightClavicle, Tail) => (false, 1800),
        (RightUpperArm, Head) => (false, 1800),
        (RightUpperArm, Tail) => (false, 5000),
        (RightLowerArm, Head) => (false, 5000),
        (RightLowerArm, Tail) => (false, 8000),
        (RightHand, Head) => (false, 8000),
        (RightHand, Tail) => (false, 10000),
        _ => return None,
    };
    Some(v)
}

/// Detect an extended-arm span (T-/A-pose): the widest silhouette row in the
/// upper body, when it reaches ≥1.3× the waist width. Returns
/// `(left_tip_x, right_tip_x, shoulder_y)`, else `None` (arms at sides → the
/// composition template handles them). This is the morphometric step — the arm
/// chain is driven by the figure's actual shape, not a fixed pose assumption.
fn detect_arm_span(mask: &[bool], w: usize, h: usize, minx: u32, miny: u32, maxx: u32, maxy: u32) -> Option<(u32, u32, u32)> {
    let ey = (maxy - miny).max(1);
    let row_span = |y: u32| -> Option<(u32, u32)> {
        if y as usize >= h {
            return None;
        }
        let row = y as usize * w;
        let mut lo: Option<u32> = None;
        let mut hi: Option<u32> = None;
        for x in minx..=maxx {
            if mask[row + x as usize] {
                if lo.is_none() {
                    lo = Some(x);
                }
                hi = Some(x);
            }
        }
        Some((lo?, hi?))
    };
    // Widest row in the upper-body band [18%, 48%] of extent = the arm line.
    let y0 = miny + 1800 * ey / 10_000;
    let y1 = miny + 4800 * ey / 10_000;
    let mut best: Option<(u32, u32, u32, u32)> = None; // (span, lo, hi, y)
    for y in y0..=y1 {
        if let Some((lo, hi)) = row_span(y) {
            let span = hi - lo;
            if best.map_or(true, |b| span > b.0) {
                best = Some((span, lo, hi, y));
            }
        }
    }
    let (span, lo, hi, y) = best?;
    // Waist reference (55% of extent) — the torso width without arms.
    let waist_y = (miny + 5500 * ey / 10_000).min(h as u32 - 1);
    let waist = row_span(waist_y).map_or(0, |(a, b)| b - a);
    if span > 0 && span * 10 >= waist * 13 {
        Some((lo, hi, y))
    } else {
        None
    }
}

/// Lay out the 20-bone Mobometric anchors on a silhouette by Rule-of-Thirds +
/// Quincunx composition, every anchor clamped inside (watertight). Empty mask →
/// empty result. Deterministic. Feed the result to `resolve_anchors`.
pub fn layout_anchors(mask: &[bool], width: u32, height: u32) -> Vec<SpatialAnchor> {
    let w = width as usize;
    let h = height as usize;
    let Some((minx, miny, maxx, maxy, cx, cy)) = silhouette_extent(mask, w, h) else {
        return Vec::new();
    };
    let ex = (maxx - minx).max(1);
    let ey = (maxy - miny).max(1);

    // Morphometric arm pass: if the figure holds its arms out (T-/A-pose), the
    // arm chain is laid along the detected horizontal span instead of the
    // arms-down composition template.
    let arm_span = detect_arm_span(mask, w, h, minx, miny, maxx, maxy);

    let mut anchors = Vec::with_capacity(TEMPLATE.len());
    for &(bone_id, endpoint, nx, ny) in TEMPLATE {
        let (px, py) = match (arm_span, arm_frac(bone_id, endpoint)) {
            // Arms out: march from the shoulder centre to the detected hand tip.
            (Some((lo, hi, sy)), Some((left, t))) => {
                let x = if left {
                    cx as i64 - (cx as i64 - lo as i64) * t / 10_000
                } else {
                    cx as i64 + (hi as i64 - cx as i64) * t / 10_000
                };
                (x.clamp(0, w as i64 - 1) as u32, sy)
            }
            // Otherwise: project the composition point into the silhouette extent.
            _ => (minx + (nx as u32 * ex) / 10_000, miny + (ny as u32 * ey) / 10_000),
        };
        // Watertight: pull every anchor inside the figure.
        let (px, py) = clamp_into(mask, w, h, px, py, cx, cy);
        anchors.push(SpatialAnchor { bone_id, endpoint, pixel_x: px, pixel_y: py });
    }
    anchors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Solid rectangle silhouette filling `[x0,x1) × [y0,y1)` in a `w×h` mask.
    fn rect_mask(w: usize, h: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<bool> {
        let mut m = vec![false; w * h];
        for y in y0..y1 {
            for x in x0..x1 {
                m[y * w + x] = true;
            }
        }
        m
    }

    #[test]
    fn template_covers_all_20_bones_head_and_tail() {
        assert_eq!(TEMPLATE.len(), 40, "20 bones × head+tail");
        let mut set: BTreeSet<(u8, u8)> = BTreeSet::new();
        for &(b, e, ..) in TEMPLATE {
            set.insert((b as u8, e as u8));
        }
        assert_eq!(set.len(), 40, "every (bone, endpoint) appears exactly once");
    }

    #[test]
    fn layout_on_full_silhouette_lands_inside() {
        let (w, h) = (100usize, 200usize);
        let mask = rect_mask(w, h, 0, 0, w, h); // whole frame is figure
        let anchors = layout_anchors(&mask, w as u32, h as u32);
        assert_eq!(anchors.len(), 40);
        for a in &anchors {
            let inside = mask[a.pixel_y as usize * w + a.pixel_x as usize];
            assert!(inside, "anchor {:?}/{:?} at ({},{}) must be inside", a.bone_id, a.endpoint, a.pixel_x, a.pixel_y);
        }
    }

    #[test]
    fn watertight_clamps_an_outside_point_in() {
        // Mask with a hole exactly where a point sits; centroid is solid.
        let (w, h) = (20usize, 20usize);
        let mut mask = rect_mask(w, h, 0, 0, w, h);
        mask[5 * w + 3] = false; // punch a bg hole at (3,5)
        let (cx, cy) = (10u32, 10u32); // centroid region is solid
        let (px, py) = clamp_into(&mask, w, h, 3, 5, cx, cy);
        assert!(mask[py as usize * w + px as usize], "clamp must land inside the figure");
        assert!(!(px == 3 && py == 5), "must move off the hole");
    }

    #[test]
    fn composition_head_sits_above_feet() {
        let (w, h) = (80usize, 160usize);
        let mask = rect_mask(w, h, 0, 0, w, h);
        let anchors = layout_anchors(&mask, w as u32, h as u32);
        let head_tip = anchors.iter().find(|a| a.bone_id == BoneId::Head && a.endpoint == BoneEndpoint::Tail).unwrap();
        let foot = anchors.iter().find(|a| a.bone_id == BoneId::LeftFoot && a.endpoint == BoneEndpoint::Tail).unwrap();
        assert!(head_tip.pixel_y < foot.pixel_y, "head must be above the feet (rule of thirds)");
    }

    #[test]
    fn empty_mask_yields_no_anchors() {
        assert!(layout_anchors(&vec![false; 16], 4, 4).is_empty());
    }

    /// Pin the `formation` primitives' values and orientation.
    ///
    /// CORRECTED 2026-08-24. This test's first draft claimed the module header
    /// "hardcodes 6667" and called it one permyriad of drift from the primitive.
    /// Both claims were wrong: 6667 appeared only in PROSE, never in code, and
    /// the TEMPLATE contains none of 3333 / 6666 / 6667 — its X values are
    /// 5000 / 3800 / 2800 / 2000 / 1700 / 6200 / 7200 / 8000 / 8300 / 4200 /
    /// 5800, hand-tuned proportions rather than thirds-line output. The header
    /// now says so; this test no longer asserts a relationship that never held.
    ///
    /// What it DOES pin: the primitives' own values, and `superior_dexter`'s
    /// orientation — fixed here before anything starts relying on it, because
    /// "dexter" is the bearer's right and therefore the VIEWER'S LEFT, which is
    /// exactly the sort of thing that gets silently mirrored.
    #[test]
    fn the_thirds_constants_match_the_formation_primitive() {
        use pp_math::formation::{quincunx_points, superior_dexter, thirds_points};
        use pp_math::fixed_point::MilliUnit;

        let (lo, hi) = (MilliUnit(0), MilliUnit(10_000));
        let pts = thirds_points(lo, hi, lo, hi);
        let xs: Vec<i64> = vec![pts[0].0 .0, pts[1].0 .0];
        assert_eq!(xs[0], 3_333, "lower thirds line over a 0..10000 span");
        assert_eq!(xs[1], 6_666, "upper thirds line: (2*10000)/3 floors to 6666, not 6667");

        // And the TEMPLATE genuinely does not use them — the header's corrected
        // claim, pinned so a later edit cannot quietly move anchors onto thirds.
        for &(_, _, x, y) in TEMPLATE {
            for v in [x, y] {
                assert!(
                    v != 3_333 && v != 6_666 && v != 6_667,
                    "TEMPLATE holds authored proportions, not thirds lines; found {v}"
                );
            }
        }

        // Quincunx centre is the spine column this file pins at 5000.
        let q = quincunx_points(lo, hi, lo, hi);
        assert_eq!(q[4].0 .0, 5_000, "centre column");
        assert_eq!(q[4].1 .0, 5_000);

        // Superior dexter = heraldic bearer's right = VIEWER'S LEFT, top half.
        // Unused anywhere in the tree today; named here so the orientation is
        // pinned before anything starts relying on it.
        let (dx0, dx1, dy0, dy1) = superior_dexter(lo, hi, lo, hi);
        assert_eq!((dx0.0, dx1.0), (0, 5_000), "dexter is the LEFT half on screen");
        assert_eq!((dy0.0, dy1.0), (0, 5_000), "superior is the TOP half");
    }

    #[test]
    fn deterministic() {
        let (w, h) = (60usize, 120usize);
        let mask = rect_mask(w, h, 5, 5, 55, 115);
        assert_eq!(layout_anchors(&mask, w as u32, h as u32), layout_anchors(&mask, w as u32, h as u32));
    }

    /// T-pose silhouette: centre torso column + a wide horizontal arm bar at
    /// shoulder height + split legs. The arms must be TRACKED out to the
    /// horizontal extremes, not draped to the hips (the arms-down template).
    fn tpose_mask(w: usize, h: usize) -> Vec<bool> {
        let mut m = vec![false; w * h];
        let mut set = |x0: usize, y0: usize, x1: usize, y1: usize| {
            for y in y0..y1 {
                for x in x0..x1 {
                    m[y * w + x] = true;
                }
            }
        };
        set(w * 42 / 100, h * 7 / 100, w * 58 / 100, h * 20 / 100);   // head
        set(w * 44 / 100, h * 18 / 100, w * 56 / 100, h * 56 / 100);  // torso column
        set(w * 6 / 100, h * 24 / 100, w * 94 / 100, h * 32 / 100);   // horizontal arms (shoulder height)
        set(w * 40 / 100, h * 55 / 100, w * 47 / 100, h * 100 / 100); // left leg
        set(w * 53 / 100, h * 55 / 100, w * 60 / 100, h * 100 / 100); // right leg
        m
    }

    #[test]
    fn tpose_arms_track_horizontal_span() {
        let (w, h) = (200usize, 400usize);
        let mask = tpose_mask(w, h);
        let anchors = layout_anchors(&mask, w as u32, h as u32);
        let lhand = anchors.iter().find(|a| a.bone_id == BoneId::LeftHand && a.endpoint == BoneEndpoint::Tail).unwrap();
        let rhand = anchors.iter().find(|a| a.bone_id == BoneId::RightHand && a.endpoint == BoneEndpoint::Tail).unwrap();
        // hands reach the horizontal arm tips (tracked), not the hips
        assert!(lhand.pixel_x < w as u32 * 20 / 100, "left hand should reach the left arm tip, got x={}", lhand.pixel_x);
        assert!(rhand.pixel_x > w as u32 * 80 / 100, "right hand should reach the right arm tip, got x={}", rhand.pixel_x);
        // and sit at shoulder height, not low at the hips
        assert!(lhand.pixel_y < h as u32 * 45 / 100, "left hand should be at shoulder height, got y={}", lhand.pixel_y);
        assert!(rhand.pixel_y < h as u32 * 45 / 100, "right hand should be at shoulder height, got y={}", rhand.pixel_y);
    }
}
