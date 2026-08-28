//! `StructuralBox` — the unified 3D integer AABB for the 2D canvas AND the 3D
//! playtest. There is **no overlay and no second layer** — it's a single integer
//! field. F5 swaps the *projection* (orthographic 2D ⟷ perspective 3D), never the
//! data. The 2D canvas is this box viewed flat (`project_to_plane`); the 3D
//! playtest is the same box viewed in perspective (the GPU camera's job).
//! "A 2D sprite is just z=0."
//!
//! **Integer-only authority.** Coordinates are raw `i64` MilliUnit (1000 = 1px),
//! the same convention as [`UiRect`] — NOT `glam::IVec3` / f32 (determinism + reuse
//! of the existing 2D rect). `StructuralBox` is the 3D *superset* of `UiRect`;
//! `project_to_plane` collapses it back to a `UiRect` for the flat view.
//! inclusive-min / EXCLUSIVE-max on all axes.

use crate::geom::UiRect;
use forge_core_v3::fixed_point::MilliUnit;

/// Which orthographic plane a 3D box collapses onto for the 2D (flat) view.
/// Perspective 3D is the GPU's job; these are the deterministic flat collapses
/// the sim/UI reason about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjectionPlane {
    /// x→screen-x, y→screen-y (drop Z). The flat 2D canvas / front elevation.
    Front,
    /// x→screen-x, z→screen-y (drop Y). The floor plan — the 3D playtest ground.
    Top,
    /// z→screen-x, y→screen-y (drop X). The side elevation.
    Side,
    /// Integer shear of [`ProjectionPlane::Top`]'s floor plan by elevation
    /// (`screen_x = x - y/2`, `screen_y = z - y/2`) — the 2.5D oblique
    /// world-map view: taller things push up-left on the floor plan, same
    /// MilliUnit integer-only convention as the other three planes. Shears
    /// the box's `min_y` corner only — exact for a thin/point-like box
    /// (most placed objects); a tall box's true sheared silhouette is a
    /// parallelogram, not a rect, and is NOT modeled here (named limit, not
    /// silently wrong: `w`/`h` stay the Top footprint).
    Iso,
}

/// 3D integer AABB in min/max form, `i64` MilliUnit (1000 = 1px). The unified box:
/// the canvas field and the playtest field are the same `StructuralBox`, drawn 2D
/// or 3D by swapping projection. inclusive-min / EXCLUSIVE-max.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct StructuralBox {
    /// Minimum x-coordinate (MilliUnit, inclusive).
    pub min_x: i64,
    /// Minimum y-coordinate (MilliUnit, inclusive).
    pub min_y: i64,
    /// Minimum z-coordinate (MilliUnit, inclusive).
    pub min_z: i64,
    /// Maximum x-coordinate (MilliUnit, exclusive).
    pub max_x: i64,
    /// Maximum y-coordinate (MilliUnit, exclusive).
    pub max_y: i64,
    /// Maximum z-coordinate (MilliUnit, exclusive).
    pub max_z: i64,
}

impl StructuralBox {
    /// Construct from explicit min/max corners.
    pub const fn new(min_x: i64, min_y: i64, min_z: i64, max_x: i64, max_y: i64, max_z: i64) -> Self {
        Self { min_x, min_y, min_z, max_x, max_y, max_z }
    }

    /// Construct from an origin corner + extents (`w`idth/`h`eight/`d`epth).
    pub const fn from_xyzwhd(x: i64, y: i64, z: i64, w: i64, h: i64, d: i64) -> Self {
        Self::new(x, y, z, x + w, y + h, z + d)
    }

    /// Size the unified box to a 2D **canvas extent** — the canvas→box bridge.
    /// `width`/`height` are the canvas's pixel dims (1px = 1 vixel); `depth` is the
    /// 3D extrude: `1` = a flat "z=0 sprite" board. Origin-anchored. The flat view
    /// round-trips — `from_extent(w, h, d).project_to_plane(Front)` is exactly the
    /// canvas's `w×h` rect — so the F5 ortho⟷persp swap shares ONE box.
    pub const fn from_extent(width: i64, height: i64, depth: i64) -> Self {
        Self::from_xyzwhd(0, 0, 0, width, height, depth)
    }

    /// Lift a flat 2D [`UiRect`] into a 3D slab spanning `[z_min, z_max)`.
    /// `from_rect_z(r, 0, 1)` is the flat board ("a 2D sprite is just z=0").
    pub const fn from_rect_z(r: UiRect, z_min: i64, z_max: i64) -> Self {
        Self::new(r.x.0, r.y.0, z_min, r.x.0 + r.w.0, r.y.0 + r.h.0, z_max)
    }

    /// Canonical voxel-chunk edge: **32**. Matches the engine's 32×32×32 voxel chunk.
    pub const CHUNK_EDGE: i64 = 32;

    /// Canvas aspect — **16:9** widescreen.
    pub const ASPECT_W: i64 = 16;
    /// Canvas aspect — **16:9** widescreen.
    pub const ASPECT_H: i64 = 9;

    /// One voxel chunk as a cube (32³) — the 3D substrate UNIT.
    pub const fn chunk() -> Self {
        Self::from_xyzwhd(0, 0, 0, Self::CHUNK_EDGE, Self::CHUNK_EDGE, Self::CHUNK_EDGE)
    }

    /// A 16:9 widescreen box: `width = height·16/9`, plus `depth` for the 3D extrude.
    pub const fn widescreen(height: i64, depth: i64) -> Self {
        Self::from_xyzwhd(0, 0, 0, height * Self::ASPECT_W / Self::ASPECT_H, height, depth)
    }

    /// Default widescreen background box: **512×288×32** — exactly 16:9 AND
    /// chunk-aligned (16 chunks wide × 9 tall × 1 deep).
    pub const fn background() -> Self {
        Self::widescreen(288, Self::CHUNK_EDGE)
    }

    /// Width along the x-axis (max_x - min_x).
    pub const fn width(&self) -> i64 { self.max_x - self.min_x }

    /// Height along the y-axis (max_y - min_y).
    pub const fn height(&self) -> i64 { self.max_y - self.min_y }

    /// Depth along the z-axis (max_z - min_z).
    pub const fn depth(&self) -> i64 { self.max_z - self.min_z }

    /// Integer center (truncating). Deterministic — no float.
    pub const fn center(&self) -> (i64, i64, i64) {
        (
            (self.min_x + self.max_x) / 2,
            (self.min_y + self.max_y) / 2,
            (self.min_z + self.max_z) / 2,
        )
    }

    /// Point-in-box test with inclusive-min / EXCLUSIVE-max semantics.
    pub const fn contains_point(&self, x: i64, y: i64, z: i64) -> bool {
        x >= self.min_x && x < self.max_x
            && y >= self.min_y && y < self.max_y
            && z >= self.min_z && z < self.max_z
    }

    /// AABB overlap test (exclusive-max: face-touching boxes do NOT intersect).
    pub const fn intersects(&self, o: &StructuralBox) -> bool {
        self.min_x < o.max_x && self.max_x > o.min_x
            && self.min_y < o.max_y && self.max_y > o.min_y
            && self.min_z < o.max_z && self.max_z > o.min_z
    }

    /// Smallest box covering both (const-fn manual min/max — `Ord::min` is not const-stable).
    pub const fn union(&self, o: &StructuralBox) -> StructuralBox {
        StructuralBox {
            min_x: if self.min_x < o.min_x { self.min_x } else { o.min_x },
            min_y: if self.min_y < o.min_y { self.min_y } else { o.min_y },
            min_z: if self.min_z < o.min_z { self.min_z } else { o.min_z },
            max_x: if self.max_x > o.max_x { self.max_x } else { o.max_x },
            max_y: if self.max_y > o.max_y { self.max_y } else { o.max_y },
            max_z: if self.max_z > o.max_z { self.max_z } else { o.max_z },
        }
    }

    /// Collapse to a flat [`UiRect`] along `plane` — the 2D view of the box. This
    /// is the orthographic half of the F5 swap; the renderer chooses ortho (flat →
    /// this rect) or perspective (3D) over the SAME box.
    ///
    /// Uses struct literals so the method stays `const fn` (UiRect::new is not const).
    pub const fn project_to_plane(&self, plane: ProjectionPlane) -> UiRect {
        match plane {
            ProjectionPlane::Front => UiRect {
                x: MilliUnit(self.min_x),
                y: MilliUnit(self.min_y),
                w: MilliUnit(self.width()),
                h: MilliUnit(self.height()),
            },
            ProjectionPlane::Top => UiRect {
                x: MilliUnit(self.min_x),
                y: MilliUnit(self.min_z),
                w: MilliUnit(self.width()),
                h: MilliUnit(self.depth()),
            },
            ProjectionPlane::Side => UiRect {
                x: MilliUnit(self.min_z),
                y: MilliUnit(self.min_y),
                w: MilliUnit(self.depth()),
                h: MilliUnit(self.height()),
            },
            ProjectionPlane::Iso => UiRect {
                x: MilliUnit(self.min_x - self.min_y / 2),
                y: MilliUnit(self.min_z - self.min_y / 2),
                w: MilliUnit(self.width()),
                h: MilliUnit(self.depth()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> StructuralBox {
        StructuralBox::from_xyzwhd(0, 0, 0, 100, 50, 200)
    }

    fn rect(x: i64, y: i64, w: i64, h: i64) -> UiRect {
        UiRect { x: MilliUnit(x), y: MilliUnit(y), w: MilliUnit(w), h: MilliUnit(h) }
    }

    #[test]
    fn extents_are_max_minus_min() {
        let b = sample();
        assert_eq!((b.width(), b.height(), b.depth()), (100, 50, 200));
        assert_eq!(b.center(), (50, 25, 100));
        assert_eq!(StructuralBox::default(), StructuralBox::new(0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn project_front_drops_z() {
        let r = sample().project_to_plane(ProjectionPlane::Front);
        assert_eq!(r, rect(0, 0, 100, 50), "front = the x/y face");
    }

    #[test]
    fn project_top_is_the_floor_plan() {
        let r = sample().project_to_plane(ProjectionPlane::Top);
        assert_eq!(r, rect(0, 0, 100, 200));
    }

    #[test]
    fn project_side_is_the_z_y_elevation() {
        let r = sample().project_to_plane(ProjectionPlane::Side);
        assert_eq!(r, rect(0, 0, 200, 50));
    }

    #[test]
    fn a_2d_sprite_is_just_z_zero() {
        let flat = rect(12_000, 8_000, 64_000, 64_000);
        let box3 = StructuralBox::from_rect_z(flat, 0, 1);
        assert_eq!(box3.project_to_plane(ProjectionPlane::Front), flat);
        assert_eq!(box3.depth(), 1);
    }

    #[test]
    fn intersects_overlap_but_not_face_touch() {
        let a = StructuralBox::from_xyzwhd(0, 0, 0, 10, 10, 10);
        let overlap = StructuralBox::from_xyzwhd(5, 5, 5, 10, 10, 10);
        let touch = StructuralBox::from_xyzwhd(10, 0, 0, 10, 10, 10);
        let apart = StructuralBox::from_xyzwhd(100, 0, 0, 10, 10, 10);
        assert!(a.intersects(&overlap));
        assert!(!a.intersects(&touch), "exclusive-max: face-touch is NOT overlap");
        assert!(!a.intersects(&apart));
    }

    #[test]
    fn contains_point_inclusive_min_exclusive_max() {
        let b = StructuralBox::from_xyzwhd(0, 0, 0, 10, 10, 10);
        assert!(b.contains_point(0, 0, 0), "min corner is inside");
        assert!(b.contains_point(9, 9, 9));
        assert!(!b.contains_point(10, 5, 5), "max corner is outside (exclusive)");
    }

    #[test]
    fn union_covers_both() {
        let a = StructuralBox::from_xyzwhd(0, 0, 0, 10, 10, 10);
        let b = StructuralBox::from_xyzwhd(20, -5, 5, 10, 10, 10);
        let u = a.union(&b);
        assert_eq!(u, StructuralBox::new(0, -5, 0, 30, 10, 15));
        assert!(u.contains_point(0, 0, 0) && u.contains_point(25, -1, 12));
    }

    #[test]
    fn chunk_is_the_32_cubed_voxel_unit() {
        let c = StructuralBox::chunk();
        assert_eq!((c.width(), c.height(), c.depth()), (32, 32, 32));
        assert_eq!(c.width() * c.height() * c.depth(), 32_768);
    }

    #[test]
    fn background_is_chunk_aligned_widescreen() {
        let b = StructuralBox::background();
        assert_eq!((b.width(), b.height(), b.depth()), (512, 288, 32));
        assert_eq!(b.width() * 9, b.height() * 16);
        let e = StructuralBox::CHUNK_EDGE;
        assert!(b.width() % e == 0 && b.height() % e == 0 && b.depth() % e == 0);
        let face = b.project_to_plane(ProjectionPlane::Front);
        assert_eq!((face.w.0, face.h.0), (512, 288));
    }

    #[test]
    fn widescreen_is_16_9_at_any_height() {
        let w = StructuralBox::widescreen(90, 1);
        assert_eq!((w.width(), w.height()), (160, 90));
        assert_eq!(w.width() * 9, w.height() * 16);
    }

    #[test]
    fn from_extent_round_trips_canvas_dims_on_front() {
        let b = StructuralBox::from_extent(320, 180, 1);
        assert_eq!((b.width(), b.height(), b.depth()), (320, 180, 1));
        assert_eq!(b.project_to_plane(ProjectionPlane::Front), rect(0, 0, 320, 180));
    }

    #[test]
    fn from_extent_depth_is_an_independent_axis() {
        let b = StructuralBox::from_extent(320, 180, 4);
        let front = b.project_to_plane(ProjectionPlane::Front);
        let top = b.project_to_plane(ProjectionPlane::Top);
        assert_eq!(front, rect(0, 0, 320, 180));
        assert_eq!(top, rect(0, 0, 320, 4));
        assert_ne!(front, top);
    }

    #[test]
    fn from_extent_is_the_canvas_constructor_behind_background() {
        assert_eq!(
            StructuralBox::from_extent(512, 288, StructuralBox::CHUNK_EDGE),
            StructuralBox::background(),
        );
    }

    #[test]
    fn const_evaluable() {
        const B: StructuralBox = StructuralBox::from_xyzwhd(0, 0, 0, 4, 4, 4);
        const R: UiRect = B.project_to_plane(ProjectionPlane::Front);
        const HIT: bool = B.contains_point(1, 1, 1);
        assert_eq!(R, UiRect { x: MilliUnit(0), y: MilliUnit(0), w: MilliUnit(4), h: MilliUnit(4) });
        assert!(HIT);
    }

    #[test]
    fn iso_shears_top_by_half_elevation() {
        let ground = StructuralBox::from_xyzwhd(100, 0, 100, 10, 5, 10);
        let raised = StructuralBox::from_xyzwhd(100, 40, 100, 10, 5, 10);
        let g = ground.project_to_plane(ProjectionPlane::Iso);
        let r = raised.project_to_plane(ProjectionPlane::Iso);
        assert_eq!(g, rect(100, 100, 10, 10), "min_y=0 shears by nothing");
        assert_eq!(r, rect(80, 80, 10, 10), "min_y=40 shears x and z by -20");
        assert_eq!((r.w.0, r.h.0), (g.w.0, g.h.0), "footprint size is unaffected by elevation");
    }

    // ── L18-style sabotage: projection plane collision ──────────────────────────
    // If we were to accidentally swap Front/Top projections, the dimensions would
    // flip. We verify this cannot happen by checking the invariant.
    #[test]
    fn projection_plane_sabotage_test() {
        let b = StructuralBox::from_xyzwhd(0, 0, 0, 100, 50, 200);
        let front = b.project_to_plane(ProjectionPlane::Front);
        let top = b.project_to_plane(ProjectionPlane::Top);
        // Front uses (x, y, w, h) from box -> (width, height)
        assert_eq!(front.w.0, 100);
        assert_eq!(front.h.0, 50);
        // Top uses (x, z, w, d) from box -> (width, depth)
        assert_eq!(top.w.0, 100);
        assert_eq!(top.h.0, 200);
        // If we accidentally used top's logic for front, this would fail:
        assert_ne!(front, top);
    }
}
