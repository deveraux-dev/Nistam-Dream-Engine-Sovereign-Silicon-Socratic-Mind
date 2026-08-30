//! Automatic sprite rigging — opaque pixels → anatomical joint regions → posed skeleton.
//!
//! Ported 2026-08-24 from `F:\NewRepo\crates\forge-core\src\pixel\auto_rig.rs`
//! (21,964 B), itself a port of sovereign-canvas `autoRig.ts`/`skeleton.ts`/`pose.ts`.
//! Landed HERE rather than in a new pixel crate because this is the front end of
//! the rig lane that already lives in this crate: [`crate::rigging_pipeline`]'s
//! 20-bone `MobometricArmature`, [`crate::bone_spline`], [`crate::bone_timeline`].
//! The `&[u32]` input is just where the joints come from.
//!
//! ONE DEVIATION FROM THE DONOR, and it is a fix, not a port choice. The donor's
//! `sin_cos_mdeg` computed rotation through `f64` and said so in its own comment:
//! "For production: precompute 360 entries. For now: use core float internally".
//! A float sin makes a pose non-reproducible across machines. This uses
//! `pp_math::fixed_point::trig::{sin_mdeg, cos_mdeg}` — the table
//! `dimensional_collapse.rs:16` calls "byte identical on every CPU". Everything
//! here is now integer end to end.
//!
//! NOT ported: the 13-joint → 20-bone bridge. It lives in v2's
//! `forge-pixel/src/rig_bridge.rs` and targets a `ForgeSkeleton` type this tree
//! does not have; mapping it onto `MobometricArmature` is a design decision, not
//! a transcription, and it gets its own weld.

use pp_math::fixed_point::trig::{cos_mdeg, sin_mdeg};
use pp_math::fixed_point::MilliUnit;

/// Maximum joints in a skeleton.
pub const MAX_JOINTS: usize = 16;

/// Permyriad unity — 10000 = 1.0. Every coordinate and weight here rides it.
pub const PMY: i32 = 10_000;

/// A joint region bounding box, in normalised permyriad coordinates.
#[derive(Clone, Copy, Debug)]
pub struct JointRegion {
    /// Region name, NUL-padded.
    pub id: [u8; 16],
    /// Left edge, permyriad.
    pub x0: i32,
    /// Top edge, permyriad.
    pub y0: i32,
    /// Right edge, permyriad.
    pub x1: i32,
    /// Bottom edge, permyriad.
    pub y1: i32,
}

impl JointRegion {
    /// Name + bounds. Names longer than 16 bytes are truncated, never wrapped.
    pub const fn new(id: &[u8], x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let mut name = [0u8; 16];
        let len = if id.len() < 16 { id.len() } else { 16 };
        let mut i = 0;
        while i < len {
            name[i] = id[i];
            i += 1;
        }
        Self { id: name, x0, y0, x1, y1 }
    }

    /// The region's name, NUL-trimmed. Invalid UTF-8 reads empty, never panics.
    pub fn id_str(&self) -> &str {
        let end = self.id.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.id[..end]).unwrap_or("")
    }

    /// Is this normalised point inside the box? Bounds inclusive.
    #[inline]
    pub fn contains(&self, nx: i32, ny: i32) -> bool {
        nx >= self.x0 && nx <= self.x1 && ny >= self.y0 && ny <= self.y1
    }

    /// Box centre, permyriad.
    #[inline]
    pub fn center(&self) -> (i32, i32) {
        ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2)
    }
}

/// The 13 default humanoid regions, in permyriad. Authored for a T-pose or
/// A-pose sheet: arms occupy the outer thirds, legs split the lower half.
pub const DEFAULT_REGIONS: [JointRegion; 13] = [
    JointRegion::new(b"head", 3000, 0, 7000, 2800),
    JointRegion::new(b"chest", 3000, 2800, 7000, 5000),
    JointRegion::new(b"upper_arm_l", 0, 2800, 3000, 4500),
    JointRegion::new(b"upper_arm_r", 7000, 2800, 10000, 4500),
    JointRegion::new(b"lower_arm_l", 0, 4500, 2800, 6000),
    JointRegion::new(b"lower_arm_r", 7200, 4500, 10000, 6000),
    JointRegion::new(b"root", 3500, 5000, 6500, 6200),
    JointRegion::new(b"upper_leg_l", 2500, 6200, 4800, 7800),
    JointRegion::new(b"upper_leg_r", 5200, 6200, 7500, 7800),
    JointRegion::new(b"lower_leg_l", 2300, 7800, 4700, 9000),
    JointRegion::new(b"lower_leg_r", 5300, 7800, 7700, 9000),
    JointRegion::new(b"foot_l", 2000, 9000, 4800, 10000),
    JointRegion::new(b"foot_r", 5200, 9000, 8000, 10000),
];

/// Per-joint pixel counts from one rig pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct RigResult {
    /// Pixels assigned to each region, indexed as the region slice was ordered.
    pub counts: [u32; MAX_JOINTS],
    /// Total pixels assigned across all regions.
    pub total_assigned: u32,
}

/// Assign every opaque pixel to a joint region.
///
/// `pixels` is packed `0xAARRGGBB`; alpha below 10 is treated as empty. A pixel
/// outside every box falls back to the NEAREST region centre by squared distance
/// — no pixel of the subject is silently dropped, which is what makes the counts
/// a usable proportion check in [`validate_rig`].
pub fn auto_rig(pixels: &[u32], width: u32, height: u32, regions: &[JointRegion]) -> RigResult {
    let mut result = RigResult::default();
    let region_count = regions.len().min(MAX_JOINTS);
    if width == 0 || height == 0 || region_count == 0 {
        return result;
    }
    let need = (width as usize) * (height as usize);
    if pixels.len() < need {
        return result;
    }

    for y in 0..height {
        for x in 0..width {
            let px = pixels[(y * width + x) as usize];
            if (px >> 24) & 0xFF < 10 {
                continue;
            }
            let nx = (x as i32 * PMY) / width as i32;
            let ny = (y as i32 * PMY) / height as i32;

            let hit = (0..region_count).find(|&r| regions[r].contains(nx, ny));
            let idx = match hit {
                Some(r) => r,
                None => {
                    let mut best = 0usize;
                    let mut best_dist = i64::MAX;
                    for r in 0..region_count {
                        let (cx, cy) = regions[r].center();
                        let (dx, dy) = ((nx - cx) as i64, (ny - cy) as i64);
                        let dist = dx * dx + dy * dy;
                        if dist < best_dist {
                            best_dist = dist;
                            best = r;
                        }
                    }
                    best
                }
            };
            result.counts[idx] += 1;
            result.total_assigned += 1;
        }
    }
    result
}

/// Opaque texels in the SUPERIOR DEXTER quadrant, and the total, as `(quad, all)`.
///
/// Heraldic dexter is the BEARER's right, which on screen is the **viewer's
/// left**; superior is the chief, the top. So this is the upper-left quarter of
/// the sheet — and the bounds come from
/// [`pp_math::formation::superior_dexter`] rather than a local `w/2`, because
/// that primitive is where the orientation is defined and mirroring it by hand
/// is exactly how a left becomes a right.
///
/// [`validate_rig`] already notices when one arm outweighs the other; it cannot
/// say WHICH side carries the mass. This names it. On the reference armour
/// sheet the coral shoulder growth sits here, and it is why that figure's
/// `upper_arm_l` outscores `upper_arm_r` roughly two to one.
///
/// Counts by the same alpha rule as [`auto_rig`] (alpha < 10 is empty), so the
/// two are comparable: `quad` is always a subset of `all`.
pub fn dexter_share(pixels: &[u32], width: u32, height: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (0, 0);
    }
    let need = (width as usize) * (height as usize);
    if pixels.len() < need {
        return (0, 0);
    }
    // The primitive owns the orientation; we only read the bounds it returns.
    let (x0, x1, y0, y1) = pp_math::formation::superior_dexter(
        MilliUnit(0),
        MilliUnit(width as i64),
        MilliUnit(0),
        MilliUnit(height as i64),
    );
    let (qx0, qx1, qy0, qy1) = (x0.0, x1.0, y0.0, y1.0);

    let (mut quad, mut all) = (0u32, 0u32);
    for y in 0..height {
        for x in 0..width {
            if (pixels[(y * width + x) as usize] >> 24) & 0xFF < 10 {
                continue;
            }
            all += 1;
            let (px, py) = (x as i64, y as i64);
            if px >= qx0 && px < qx1 && py >= qy0 && py < qy1 {
                quad += 1;
            }
        }
    }
    (quad, all)
}

/// One complaint about a rigged sheet.
#[derive(Clone, Copy, Debug)]
pub struct RigWarning {
    /// Which region the complaint is about.
    pub joint_idx: usize,
    /// How bad it is.
    pub severity: WarnSeverity,
}

/// Severity of a [`RigWarning`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarnSeverity {
    /// Suspicious proportion — the rig will work, it may look wrong.
    Warn,
    /// A region caught nothing; the sheet probably is not the pose expected.
    Error,
}

/// Check rig proportions: empty joints, oversized head or root, left/right
/// asymmetry. Lookup is by `id_str()`, never raw index, so a caller may reorder
/// or substitute regions freely. Feet are allowed to be empty — a sheet cropped
/// at the ankles is a legitimate sprite, not a broken rig.
pub fn validate_rig(result: &RigResult, regions: &[JointRegion]) -> ([RigWarning; MAX_JOINTS], usize) {
    let mut warnings = [RigWarning { joint_idx: 0, severity: WarnSeverity::Warn }; MAX_JOINTS];
    let mut wc = 0usize;
    let total = result.total_assigned;
    if total == 0 {
        return (warnings, 0);
    }

    let region_count = regions.len().min(MAX_JOINTS);
    let pct = |idx: usize| -> i32 { (result.counts[idx] as i64 * 100 / total as i64) as i32 };
    let find = |name: &str| -> Option<usize> {
        regions.iter().take(region_count).position(|r| r.id_str() == name)
    };
    let mut push = |idx: usize, sev: WarnSeverity, wc: &mut usize| {
        if *wc < MAX_JOINTS {
            warnings[*wc] = RigWarning { joint_idx: idx, severity: sev };
            *wc += 1;
        }
    };

    for i in 0..region_count {
        let name = regions[i].id_str();
        if result.counts[i] == 0 && name != "foot_l" && name != "foot_r" {
            push(i, WarnSeverity::Error, &mut wc);
        }
    }
    if let Some(head) = find("head") {
        let p = pct(head);
        if p > 25 || p < 2 {
            push(head, WarnSeverity::Warn, &mut wc);
        }
    }
    for (l, r) in [("upper_arm_l", "upper_arm_r"), ("upper_leg_l", "upper_leg_r")] {
        if let (Some(li), Some(ri)) = (find(l), find(r)) {
            if (pct(li) - pct(ri)).abs() > 10 {
                push(li, WarnSeverity::Warn, &mut wc);
            }
        }
    }
    if let Some(root) = find("root") {
        if pct(root) > 25 {
            push(root, WarnSeverity::Warn, &mut wc);
        }
    }

    (warnings, wc)
}

/// A joint in the rig hierarchy.
///
/// Named `RigJoint`, not `Joint`: `forge-mud-v3::weapon_wireframes` already owns
/// that name for an unrelated wireframe type, and two live homes for one name is
/// an L05 defect (the one-home hook blocked the first write on exactly this).
#[derive(Clone, Copy, Debug)]
pub struct RigJoint {
    /// Joint name, NUL-padded.
    pub id: [u8; 16],
    /// Rest x, permyriad.
    pub x: i32,
    /// Rest y, permyriad.
    pub y: i32,
    /// Parent index, or 255 for the root.
    pub parent: u8,
}

impl RigJoint {
    /// Name + rest position + parent.
    pub const fn new(id: &[u8], x: i32, y: i32, parent: u8) -> Self {
        let mut name = [0u8; 16];
        let len = if id.len() < 16 { id.len() } else { 16 };
        let mut i = 0;
        while i < len {
            name[i] = id[i];
            i += 1;
        }
        Self { id: name, x, y, parent }
    }

    /// The joint's name, NUL-trimmed.
    pub fn id_str(&self) -> &str {
        let end = self.id.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.id[..end]).unwrap_or("")
    }
}

/// The 13-joint default humanoid, ordered parent-before-child so
/// [`apply_pose`] can walk it in one forward pass.
pub const DEFAULT_SKELETON: [RigJoint; 13] = [
    RigJoint::new(b"root", 5000, 5500, 255),
    RigJoint::new(b"chest", 5000, 3800, 0),
    RigJoint::new(b"head", 5000, 1800, 1),
    RigJoint::new(b"upper_arm_l", 2800, 3800, 1),
    RigJoint::new(b"upper_arm_r", 7200, 3800, 1),
    RigJoint::new(b"lower_arm_l", 1800, 5200, 3),
    RigJoint::new(b"lower_arm_r", 8200, 5200, 4),
    RigJoint::new(b"upper_leg_l", 4000, 6800, 0),
    RigJoint::new(b"upper_leg_r", 6000, 6800, 0),
    RigJoint::new(b"lower_leg_l", 3800, 8200, 7),
    RigJoint::new(b"lower_leg_r", 6200, 8200, 8),
    RigJoint::new(b"foot_l", 3600, 9300, 9),
    RigJoint::new(b"foot_r", 6400, 9300, 10),
];

/// Per-joint pose delta: rotation in millidegrees, translation in permyriad.
#[derive(Clone, Copy, Debug, Default)]
pub struct JointPose {
    /// Rotation, millidegrees (1000 = 1 degree).
    pub angle_mdeg: i32,
    /// X offset, permyriad.
    pub dx: i32,
    /// Y offset, permyriad.
    pub dy: i32,
}

/// A full pose — one per frame.
#[derive(Clone, Debug)]
pub struct Pose {
    /// Per-joint deltas, indexed as the skeleton is ordered.
    pub joints: [JointPose; MAX_JOINTS],
}

impl Default for Pose {
    fn default() -> Self {
        Self { joints: [JointPose::default(); MAX_JOINTS] }
    }
}

/// Permyriad lerp between two poses, `t` in `0..=10000`. One floor-division
/// rounding rule, `i64` interior so a large offset cannot overflow mid-blend.
pub fn lerp_pose(a: &Pose, b: &Pose, t: i32) -> Pose {
    let t = t.clamp(0, PMY) as i64;
    let inv = PMY as i64 - t;
    let mix = |x: i32, y: i32| -> i32 { ((x as i64 * inv + y as i64 * t) / PMY as i64) as i32 };
    let mut out = Pose::default();
    for i in 0..MAX_JOINTS {
        out.joints[i] = JointPose {
            angle_mdeg: mix(a.joints[i].angle_mdeg, b.joints[i].angle_mdeg),
            dx: mix(a.joints[i].dx, b.joints[i].dx),
            dy: mix(a.joints[i].dy, b.joints[i].dy),
        };
    }
    out
}

/// Apply a pose to a skeleton, writing solved positions into `out`.
///
/// Rotation is hierarchical: rotating a joint carries every descendant with it.
/// Trig is the deterministic permyriad table, so the same pose solves to the
/// same integers on any machine.
pub fn apply_pose(skeleton: &[RigJoint], pose: &Pose, out: &mut [(i32, i32); MAX_JOINTS]) {
    let count = skeleton.len().min(MAX_JOINTS);
    for i in 0..count {
        out[i] = (skeleton[i].x + pose.joints[i].dx, skeleton[i].y + pose.joints[i].dy);
    }
    for i in 0..count {
        let angle = pose.joints[i].angle_mdeg;
        if angle == 0 {
            continue;
        }
        let pivot = out[i];
        let (sin_k, cos_k) = (sin_mdeg(angle) as i64, cos_mdeg(angle) as i64);
        for j in (i + 1)..count {
            if is_descendant(skeleton, j, i, count) {
                let rx = (out[j].0 - pivot.0) as i64;
                let ry = (out[j].1 - pivot.1) as i64;
                out[j].0 = pivot.0 + ((rx * cos_k - ry * sin_k) / PMY as i64) as i32;
                out[j].1 = pivot.1 + ((rx * sin_k + ry * cos_k) / PMY as i64) as i32;
            }
        }
    }
}

/// Is `child` below `ancestor` in the hierarchy? Walks parents to the root and
/// bails on an out-of-range parent, so a malformed skeleton cannot spin forever.
fn is_descendant(skeleton: &[RigJoint], child: usize, ancestor: usize, count: usize) -> bool {
    let mut cur = child;
    for _ in 0..count {
        let p = skeleton[cur].parent;
        if p == 255 || p as usize >= count {
            return false;
        }
        if p as usize == ancestor {
            return true;
        }
        cur = p as usize;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(w: u32, h: u32) -> Vec<u32> {
        vec![0xFF00_0000u32; (w * h) as usize]
    }

    #[test]
    fn every_opaque_pixel_lands_somewhere() {
        let (w, h) = (40u32, 40u32);
        let r = auto_rig(&opaque(w, h), w, h, &DEFAULT_REGIONS);
        assert_eq!(r.total_assigned, w * h, "no pixel of the subject is dropped");
        assert_eq!(r.counts.iter().sum::<u32>(), w * h, "counts reconcile to the total");
    }

    #[test]
    fn transparent_pixels_are_skipped() {
        let (w, h) = (16u32, 16u32);
        let mut px = opaque(w, h);
        for p in px.iter_mut().take(128) {
            *p = 0x0000_0000; // alpha 0
        }
        let r = auto_rig(&px, w, h, &DEFAULT_REGIONS);
        assert_eq!(r.total_assigned, w * h - 128);
    }

    #[test]
    fn a_short_buffer_is_refused_rather_than_read_past() {
        let r = auto_rig(&[0xFF00_0000; 4], 100, 100, &DEFAULT_REGIONS);
        assert_eq!(r.total_assigned, 0, "a lying width must not index out of bounds");
        assert_eq!(auto_rig(&[], 0, 0, &DEFAULT_REGIONS).total_assigned, 0);
    }

    #[test]
    fn regions_are_found_by_name_not_index() {
        // Reversed order: validate_rig must still find head/root/limbs.
        let mut rev = DEFAULT_REGIONS;
        rev.reverse();
        let (w, h) = (40u32, 40u32);
        let r = auto_rig(&opaque(w, h), w, h, &rev);
        let (_, n) = validate_rig(&r, &rev);
        let r2 = auto_rig(&opaque(w, h), w, h, &DEFAULT_REGIONS);
        let (_, n2) = validate_rig(&r2, &DEFAULT_REGIONS);
        assert_eq!(n, n2, "reordering regions must not change the verdict");
    }

    #[test]
    fn an_empty_sheet_warns_about_nothing() {
        let r = RigResult::default();
        let (_, n) = validate_rig(&r, &DEFAULT_REGIONS);
        assert_eq!(n, 0, "no pixels means no proportions to judge");
    }

    #[test]
    fn feet_may_be_empty_but_a_missing_head_is_an_error() {
        // Only the head band is inked -> every other region is empty.
        let (w, h) = (40u32, 40u32);
        let mut px = vec![0u32; (w * h) as usize];
        for y in 0..(h / 4) {
            for x in (w * 3 / 10)..(w * 7 / 10) {
                px[(y * w + x) as usize] = 0xFF00_0000;
            }
        }
        let r = auto_rig(&px, w, h, &DEFAULT_REGIONS);
        let (warns, n) = validate_rig(&r, &DEFAULT_REGIONS);
        let errs: Vec<&str> = warns[..n]
            .iter()
            .filter(|w| w.severity == WarnSeverity::Error)
            .map(|w| DEFAULT_REGIONS[w.joint_idx].id_str())
            .collect();
        assert!(!errs.is_empty(), "empty limbs must be errors");
        assert!(!errs.contains(&"foot_l") && !errs.contains(&"foot_r"), "feet are allowed to be empty");
    }

    #[test]
    fn a_rest_pose_moves_nothing() {
        let mut out = [(0i32, 0i32); MAX_JOINTS];
        apply_pose(&DEFAULT_SKELETON, &Pose::default(), &mut out);
        for (i, j) in DEFAULT_SKELETON.iter().enumerate() {
            assert_eq!(out[i], (j.x, j.y), "{}", j.id_str());
        }
    }

    #[test]
    fn rotating_a_parent_carries_its_children() {
        let mut pose = Pose::default();
        pose.joints[1].angle_mdeg = 90_000; // chest, 90 degrees
        let mut out = [(0i32, 0i32); MAX_JOINTS];
        apply_pose(&DEFAULT_SKELETON, &pose, &mut out);
        assert_eq!(out[0], (DEFAULT_SKELETON[0].x, DEFAULT_SKELETON[0].y), "root is not a child of chest");
        assert_ne!(out[2], (DEFAULT_SKELETON[2].x, DEFAULT_SKELETON[2].y), "head follows the chest");
        assert_ne!(out[5], (DEFAULT_SKELETON[5].x, DEFAULT_SKELETON[5].y), "lower arm is a grandchild and follows too");
    }

    #[test]
    fn the_pose_solve_is_deterministic_and_float_free() {
        let mut pose = Pose::default();
        pose.joints[1].angle_mdeg = 37_123; // deliberately not a table-friendly angle
        let solve = || {
            let mut o = [(0i32, 0i32); MAX_JOINTS];
            apply_pose(&DEFAULT_SKELETON, &pose, &mut o);
            o
        };
        let first = solve();
        for _ in 0..64 {
            assert_eq!(solve(), first, "same pose, same integers, always");
        }
    }

    #[test]
    fn lerp_hits_both_ends_and_the_middle() {
        let a = Pose::default();
        let mut b = Pose::default();
        b.joints[0].angle_mdeg = 10_000;
        b.joints[0].dx = 400;
        assert_eq!(lerp_pose(&a, &b, 0).joints[0].angle_mdeg, 0);
        assert_eq!(lerp_pose(&a, &b, PMY).joints[0].angle_mdeg, 10_000);
        assert_eq!(lerp_pose(&a, &b, PMY / 2).joints[0].angle_mdeg, 5_000);
        assert_eq!(lerp_pose(&a, &b, PMY / 2).joints[0].dx, 200);
        // Out-of-range t clamps rather than extrapolating past the target pose.
        assert_eq!(lerp_pose(&a, &b, 99_999).joints[0].angle_mdeg, 10_000);
    }

    #[test]
    fn dexter_is_the_upper_left_quarter_on_screen() {
        // Ink ONLY the upper-left quarter of an 8x8 sheet.
        let (w, h) = (8u32, 8u32);
        let mut px = vec![0u32; (w * h) as usize];
        for y in 0..4 {
            for x in 0..4 {
                px[(y * w + x) as usize] = 0xFF00_0000;
            }
        }
        let (quad, all) = dexter_share(&px, w, h);
        assert_eq!((quad, all), (16, 16), "all of it is superior dexter");

        // Mirror it to the upper-RIGHT (sinister): none of it should count.
        let mut px = vec![0u32; (w * h) as usize];
        for y in 0..4 {
            for x in 4..8 {
                px[(y * w + x) as usize] = 0xFF00_0000;
            }
        }
        let (quad, all) = dexter_share(&px, w, h);
        assert_eq!((quad, all), (0, 16), "sinister is not dexter — the mirror must not pass");
    }

    #[test]
    fn dexter_is_a_quarter_of_an_evenly_inked_sheet() {
        let (w, h) = (40u32, 40u32);
        let (quad, all) = dexter_share(&opaque(w, h), w, h);
        assert_eq!(all, w * h);
        assert_eq!(quad, all / 4, "an even field splits four ways");
    }

    #[test]
    fn dexter_counts_the_same_texels_auto_rig_does() {
        // Same alpha rule, so quad is always a subset of the rig's total.
        let (w, h) = (16u32, 16u32);
        let mut px = opaque(w, h);
        for p in px.iter_mut().take(50) {
            *p = 0x0000_0000;
        }
        let (quad, all) = dexter_share(&px, w, h);
        let r = auto_rig(&px, w, h, &DEFAULT_REGIONS);
        assert_eq!(all, r.total_assigned, "both honour alpha < 10 as empty");
        assert!(quad <= all);
    }

    #[test]
    fn a_short_or_empty_buffer_yields_no_dexter_count() {
        assert_eq!(dexter_share(&[0xFF00_0000; 4], 100, 100), (0, 0));
        assert_eq!(dexter_share(&[], 0, 0), (0, 0));
    }

    #[test]
    fn a_cyclic_parent_chain_cannot_hang_the_solver() {
        let mut sk = [DEFAULT_SKELETON[0]; 3];
        sk[0] = RigJoint::new(b"a", 0, 0, 2);
        sk[1] = RigJoint::new(b"b", 0, 0, 0);
        sk[2] = RigJoint::new(b"c", 0, 0, 1);
        assert!(!is_descendant(&sk, 0, 99, 3), "a cycle terminates instead of spinning");
    }
}
