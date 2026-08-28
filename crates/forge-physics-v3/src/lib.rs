//! Ported verbatim from `F:\NewRepo\crates\forge-physics\src\hermite.rs`
//! (2026-08-13) — the deterministic Hermite/Catmull-Rom spline kernel.
//! 2026-08-17: added a scoped port of `types.rs` (see that module's doc
//! comment for exactly what is and isn't ported). The rest of v2's
//! `forge-physics` remains untouched and not claimed by this crate.

pub mod hermite;
/// Scoped physics-effect and material types — see module doc for exact scope.
pub mod types;
/// 2D support functions (Support2D/ConvexPolygon2D) for Minkowski-difference collision.
pub mod support;
/// 2D GJK collision test (gjk_intersects_2d) — integer MilliUnit, no float.
pub mod gjk2d;
/// 3D GJK collision test (ConvexHull/gjk_intersects) — bake-time asset QC, float legal.
pub mod gjk3d;
