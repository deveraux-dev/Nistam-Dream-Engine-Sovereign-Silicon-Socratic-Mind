//! forge-geo's rigging wing — ported verbatim from
//! `F:\NewRepo\crates\forge-geo\src\{mesh,watertight,volumetric_distance,
//! rigging_pipeline,heat_map_binder,glb_bone_data,bone_spline,bone_timeline,
//! anchor_layout,anchor_draft,mobometric}.rs` (2026-08-13).
//!
//! **Scope.** Per `BLUEPRINT-SUBSTRATE-CENSUS-2026-08-11.md:131-139`: "rigging
//! wing is INTEGER (20-bone Mobometric pipeline MilliUnit/Permyriad end-to-end;
//! bone_spline integer Catmull-Rom w/ i128; f32 only at the one GLB export
//! file; anchor_layout/draft = auto-layout + MANDATORY human-review gate before
//! bake). heat_map_binder: exact-sum weights (=10000 per vertex, remainder to
//! top bone)." That whole closure — the 11 modules above — has zero dependency
//! on forge-geo's quarantined 3D mesh core (`primitives`/`builder`/`sdf`/`csg`,
//! "f32 soup — render-edge only, never machine path... CSG stub: do not port
//! as CSG", same census, lines 136-137, 174-175). `mesh.rs` here is only
//! `ForgeMesh` — a plain `{positions, normals, uvs, indices}` data container
//! and its accessor methods — not that generator machinery, so porting it does
//! not violate the quarantine.
//!
//! **Dependency now real.** `pp_math::{MilliUnit, Permyriad}` is rewritten to
//! `forge_core_v3::fixed_point::{MilliUnit, Permyriad}` throughout — identical
//! layout (`MilliUnit(pub i64)`, `Permyriad(pub i32)`, same derives), landed in
//! `forge-core-v3` per the pp-math Wave 1 port. This crate was blocked on that
//! landing (L06); it is not blocked anymore.
//!
//! **Pipeline order** (`rigging_pipeline::run_rigging_pipeline`): validate
//! watertight → resolve 2D pixel anchors to 3D `MilliUnit` positions on the
//! mesh surface → generate the 20-bone armature → build a voxelized volumetric
//! workspace → bind per-vertex weights via geodesic inverse-distance weighting
//! → convert to glTF 2.0 skinning data (the one f32 boundary, `glb_bone_data`).
//! `anchor_layout`/`anchor_draft` sit upstream of all of it: auto-layout
//! proposes anchors by composition geometry (Rule-of-Thirds/Quincunx), but
//! `AnchorDraft` holds them mutable until a human `commit`s — nothing bakes
//! into a rig unreviewed.

pub mod anchor_draft;
pub mod auto_rig;
pub mod anchor_layout;
pub mod bone_spline;
pub mod bone_timeline;
pub mod culling;
pub mod glb_bone_data;
pub mod heat_map_binder;
pub mod laban;
pub mod mesh;
pub mod mobometric;
pub mod picking;
pub mod reverse_poisson;
pub mod rigging_pipeline;
pub mod spatial_index;
pub mod surfaceledger_spline;
pub mod synthetic_weld;
pub mod volumetric_distance;
pub mod watertight;

// Re-export public types from ported modules
pub use culling::{AABB, CullUniforms, Frustum};
pub use picking::{pick_part, ray_first_hit, PickResult, Ray};
pub use spatial_index::{FlatSceneIndex, SceneBvh, SceneSpatialIndex};
