//! forge-canvas-v3 — sovereign UI foundation: integer layout, spring physics,
//! text shaping, draw commands, software rasterizer.
//!
//! WARM QUARRY CANVAS RASTER lane. Drained from F:\NewRepo\crates\forge-canvas
//! (v2), layered by real internal dependency order (recon'd via import grep,
//! not guessed) so every module compiles against modules that already exist.
//! `draw` and `ui::widgets` are a genuine circular pair in the donor — welded
//! together as one lane, never split (spine-pipeline LANE=COMPILE-UNIT law).
//!
//! Bottom-of-graph UI primitive: depends only on forge-core-v3 (Permyriad/
//! MilliUnit fixed-point types) and fontdue (ARCH000-nodded 2026-08-12).

// ── Layer 0 (leaf: zero internal deps) ──────────────────────────────────────
pub mod geom;
pub mod resolution;
pub mod text;
pub mod theme;
pub mod spring;
pub mod tween;
pub mod gpos_kern;
pub mod msdf;
pub mod juice;
pub mod pad_glyphs;
pub mod cree_bitmap;
pub mod cree_syllabics;
pub mod cree_glyphs;
pub mod ritual_glyph;
pub mod accent_safe;
pub mod workflow_deck;
pub mod zen_canvas;
pub mod glyph_stencil;

// ── Layer 1 (deps: layer 0 only) ────────────────────────────────────────────
pub mod caption;
pub mod supermaxatom_camera;
pub mod camera5d;
pub mod compositor;
pub mod tokens;
pub mod structural_box;
pub mod input;
pub mod layout;
pub mod dock;

// ── Layer 2 (draw ↔ widgets: genuine circular pair, welded together) ────────
pub mod draw;
pub mod widgets;
pub mod material_params;
pub mod sphere_brush;
pub mod stencil;
pub mod pen_cst_bridge;
pub mod path_shape;
pub mod raycast;
pub mod patex;

// ── Layer 3 (canvas widgets and retained UI over layer 2) ─────────────────────
pub mod pixel_canvas;
pub mod ui_manifest;
pub mod level_zero;
pub mod playtest_bridge;
pub mod penteract;

// ── Layer 3 (UI engine: depends on draw + widgets + spring) ────────────────
pub mod ui_engine;

// ── Layer 4 (specialized renderers: depend on draw + text) ─────────────────
pub mod bitmap_font;
pub mod pixel_baseline;
pub mod preview;
pub mod rasterizer;

// ── Layer 5 (organ-level composers and theme exporters) ──────────────────────
pub mod organs;
