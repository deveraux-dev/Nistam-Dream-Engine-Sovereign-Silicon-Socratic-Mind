//! capability_index — the VixiScript capability map (ADR-0004 #9 + ADR-0010).
//!
//! Every vixi keyword / `.vixel` AST node / `.atom` type → its capability
//! {name, goal, status, oracle} + AST/LSP wiring, in ONE table the door's
//! `capability_query` can serve (name/goal/status/oracle schema).
//!
//! ## Anti-rot (mirrors `capability_floor` DNA, ADR-019)
//! The authored [`CAPABILITIES`] table cannot fall behind source: the live
//! element set is RE-DERIVED from the grammar SoT (`forge-vix/grammar.rs`
//! `VIXEL_*` tables) and the atom structs at TEST time via `include_str!`, and
//! `tests::capability_index_covers_every_vixi_surface` (donor-only, excluded
//! from this v3 port — see "v3 port note" below) FAILS the moment a
//! keyword / node has no row. [`render_index`] is a pure, deterministic
//! projection — the analog of `capability_floor::render_floor`.
//!
//! ## `oracle` here (ADR-0010)
//! The `oracle` column is the **realising `crate::fn`** — pinned by ADR-0010,
//! distinct from the technothesia CST *ground-truth* oracle and the forge-daemon
//! *cloud* `Tier::Oracle`. (The field name is kept for door-schema parity.)
//!
//! ## Firewall
//! forge-ast gains NO new cargo edge: cross-crate sources are read as TEXT via
//! `include_str!` (a compile-time file read — the same move
//! `forge-vix/grammar.rs` uses for the `.vixel` SoT), never a dependency.
//!
//! ## Known gap (honest `NOT_PRESENT`)
//! `.vixel` has no syntax-highlight class: `forge-gui/vixel_highlight.rs`
//! classifies `.kit.vixi` tokens (`SLOT_KINDS`/`WIDGET_NAMES`/…), not the
//! `VIXEL_*` vocab — so every `highlight` column is `NOT_PRESENT` (a real
//! follow-on, not a blank).
//!
//! ## v3 port note (2026-08-19)
//! The donor's `#[cfg(test)] mod tests` (anti-rot proofs) is EXCLUDED here.
//! It `include_str!`s ~20 v2 sibling crates (forge-vix, forge-ml, nde_core,
//! forge-dag, forge-semantic, forge-meaning-budget, forge-game-systems,
//! ironroot-signal, forge-hal, moe-gpu-dsp, forge-gpu, forge-shaders,
//! forge-core (v2), forge-export, forge-kv-math, forge-vision, …) via
//! relative paths that don't exist in this port's scope — none of those
//! crates are ported to v3. Porting the test module verbatim would fail
//! `cargo test` immediately, not from a mechanical issue but a real
//! multi-crate dependency gap (L15/L25: name the blocker, don't paper over
//! it). The `oracle` column's crate-path strings below are inert `&'static
//! str` data — harmless to keep, they compile fine without those crates
//! existing. Re-porting the anti-rot tests is a real follow-up once (if)
//! those sibling crates land in v3.

/// Which surface an element belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `.vixel` body / theme-layer keyword (LSP completion vocab).
    VixiKeyword,
    /// A `.vixel` top-level block — opens an AST node (material/ui/theme/rule/spawn_/set_).
    VixelNode,
    /// A runtime atom type / automata rule (no LSP vocab).
    AtomType,
}

impl Kind {
    /// The `kind` column string used by `render_index`/`render_spine`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::VixiKeyword => "vixi_keyword",
            Kind::VixelNode => "vixel_node",
            Kind::AtomType => "atom_type",
        }
    }
}

/// One row of the capability index. Authored data — all `&'static str`.
#[derive(Debug, Clone, Copy)]
pub struct CapabilityRow {
    /// The VixiScript keyword / AST node / atom type name.
    pub element: &'static str,
    /// Which surface `element` belongs to.
    pub kind: Kind,
    /// `[judgment]` what it lets the author DO.
    pub capability: &'static str,
    /// `[judgment]` a ROADMAP plan id, or `NOT_PRESENT`.
    pub goal: &'static str,
    /// `[judgment]` `live` | `partial` | `planned`, or `NOT_PRESENT`.
    pub status: &'static str,
    /// [mechanical, ADR-0010] the realising `crate::fn`, or `NOT_PRESENT`.
    pub oracle: &'static str,
    /// `[mechanical]` the `grammar_bridge` parse arm, or `—` for atom types.
    pub ast_arm: &'static str,
    /// `[mechanical]` the `forge-vix/grammar.rs` mirror table, or `NOT_PRESENT`.
    pub lsp_mirror: &'static str,
    /// `[mechanical]` the `vixel_highlight::VixelMaterial`, or `NOT_PRESENT` (gap).
    pub highlight: &'static str,
    /// `[mechanical]` 0–4 liveness rung — Ghost·Design·Proven·Verified·Live. Auto-derived
    /// from `status` (first pass, `rung_from_status`); Sean re-rungs explicit rows.
    pub rung: u8,
}

#[allow(clippy::too_many_arguments)]
const fn r(
    element: &'static str,
    kind: Kind,
    capability: &'static str,
    goal: &'static str,
    status: &'static str,
    oracle: &'static str,
    ast_arm: &'static str,
    lsp_mirror: &'static str,
    highlight: &'static str,
) -> CapabilityRow {
    CapabilityRow { element, kind, capability, goal, status, oracle, ast_arm, lsp_mirror, highlight, rung: rung_from_status(status) }
}

/// 0–4 liveness rung derived from `status` (first pass; Sean re-rungs explicit rows).
/// `4` Live · `2` Proven (partial) · `1` Design (planned) · `0` Ghost (NOT_PRESENT/unknown).
const fn rung_from_status(status: &str) -> u8 {
    match status.as_bytes() {
        b"live" => 4,
        b"partial" => 2,
        b"planned" => 1,
        _ => 0,
    }
}

// Shorthands (cut row noise; not exported).
const VC: &str = "foundation.vixel-canvas";
const SB: &str = "poc.sound-brush";
const RND: &str = "foundation.render";
const DET: &str = "foundation.determinism-proof";
const VV: &str = "foundation.visual-verify";
const MOE: &str = "foundation.moe-routing";
const MOM: &str = "foundation.mom-audio";
const TDAG: &str = "foundation.task-dag";
const SEM: &str = "foundation.semantic";
const INF: &str = "foundation.inference";
const AUTH: &str = "foundation.authoring";
const SIG: &str = "foundation.signal-route";
const SAFE: &str = "foundation.safety-moe";
const TRAIN: &str = "foundation.ml-train";
const ML: &str = "foundation.ml";
const PM: &str = "forge_ast::vixel::grammar_bridge::parse_material";
const PU: &str = "forge_ast::vixel::grammar_bridge::parse_ui";
const PT: &str = "forge_ast::vixel::grammar_bridge::parse_theme";
const PS: &str = "forge_ast::vixel::grammar_bridge::parse_source";
const PA: &str = "forge_ast::vixel::grammar_bridge::parse_atom";
const NP: &str = "NOT_PRESENT";
const KW: &str = "VIXEL_KEYWORDS";
const KEYS: &str = "VIXEL_KEYS";
const LAYERS: &str = "VIXEL_THEME_LAYERS";
// VixelMaterial highlight shorthands (forge_gui::vixel_highlight::VixelMaterial).
const VM_CTL: &str = "VixelMaterial::Control";
const VM_IMM: &str = "VixelMaterial::Immutable";
const VM_STR: &str = "VixelMaterial::Structural";
const VM_KIN: &str = "VixelMaterial::Kinetic";

/// The capability index — one row per live VixiScript surface element.
/// Coverage + purity are locked by the tests below (anti-rot).
pub const CAPABILITIES: &[CapabilityRow] = &[
    // ── .vixel top-level blocks (VIXEL_KEYWORDS → AST nodes) ──
    r("material", Kind::VixelNode, "author a material atom (mass/hardness/albedo…)", VC, "live", PM, "parse_material", KW, VM_CTL),
    r("ui", Kind::VixelNode, "author a UI element (layout/material/sound/particle)", VC, "live", PU, "parse_ui", KW, VM_CTL),
    r("theme", Kind::VixelNode, "author a theme / design-token sheet", VC, "live", PT, "parse_theme", KW, VM_CTL),
    r("rule", Kind::VixelNode, "author a cellular-automaton rule", VC, "live", "forge_shaders::vixel_automata", PS, KW, VM_CTL),
    r("spawn_", Kind::VixelNode, "author a spatial spawn rule (socket graph)", VC, "live", "forge_ast::vixel::SpatialDef", PS, KW, VM_CTL),
    r("set_", Kind::VixelNode, "author an environment directive (temp/wind/gravity)", VC, "live", "forge_ast::vixel::EnvironmentDef", PS, KW, VM_CTL),
    r("atom", Kind::VixelNode, "author a VixelAtom (coord/material_id/resonance/color) — VixiScript's lowering target, terminal-audible", VC, "live", PA, "parse_atom", KW, VM_CTL),
    r("acrylic", Kind::VixelNode, "author a voxel-acrylic paint dab (color/material/essence/phase)", VC, "live", "forge_ast::vixel::grammar_bridge::parse_acrylic", "parse_acrylic", KW, VM_CTL),
    r("pressure", Kind::VixelNode, "author a pen-feel pressure curve (linear/soft/hard)", VC, "live", "forge_ast::vixel::grammar_bridge::parse_pressure", "parse_pressure", KW, VM_CTL),
    r("layers", Kind::VixelNode, "author a paint layer-stack depth (count)", VC, "live", "forge_ast::vixel::grammar_bridge::parse_layers", "parse_layers", KW, VM_CTL),
    r("viewport", Kind::VixelNode, "author a camera/viewport (w/h/zoom)", VC, "live", "forge_ast::vixel::grammar_bridge::parse_viewport", "parse_viewport", KW, VM_CTL),
    r("brush", Kind::VixelNode, "author a procedural brush tip (w/h/falloff)", VC, "live", "forge_ast::vixel::grammar_bridge::parse_brush", "parse_brush", KW, VM_CTL),

    // ── material body keys (parse_material) ──
    r("color", Kind::VixiKeyword, "colour literal color(r,g,b,a)", VC, "live", PM, "parse_material|parse_ui", KEYS, VM_IMM),
    r("mass", Kind::VixiKeyword, "material mass (Permyriad)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("hardness", Kind::VixiKeyword, "material hardness (Permyriad)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("flammability", Kind::VixiKeyword, "material flammability (Permyriad)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("roughness", Kind::VixiKeyword, "material roughness (Permyriad)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("metallic", Kind::VixiKeyword, "material metallic (Permyriad)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("albedo", Kind::VixiKeyword, "material albedo (u32 RGBA)", VC, "live", PM, "parse_material", KEYS, VM_IMM),
    r("destruction", Kind::VixiKeyword, "destruction mode (shatter/splinter/melt)", VC, "live", PM, "parse_material", KEYS, VM_IMM),

    // ── ui body keys (parse_ui) ──
    r("x", Kind::VixiKeyword, "ui x (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("y", Kind::VixiKeyword, "ui y (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("w", Kind::VixiKeyword, "ui width (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("h", Kind::VixiKeyword, "ui height (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("radius", Kind::VixiKeyword, "ui corner radius", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("vibe", Kind::VixiKeyword, "ui vibe mask (seehear channels)", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("color_selected", Kind::VixiKeyword, "ui selected-state colour", VC, "live", PU, "parse_ui", KEYS, VM_IMM),
    r("depth", Kind::VixiKeyword, "ui z-depth (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("font", Kind::VixiKeyword, "ui font id", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("font_size", Kind::VixiKeyword, "ui font size", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("text", Kind::VixiKeyword, "ui text content", VC, "live", PU, "parse_ui", KEYS, VM_IMM),
    r("text_color", Kind::VixiKeyword, "ui text colour", VC, "live", PU, "parse_ui", KEYS, VM_IMM),
    r("parent", Kind::VixiKeyword, "ui parent slot name", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("spacing", Kind::VixiKeyword, "ui child spacing (MilliUnit)", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("repeat", Kind::VixiKeyword, "ui repeat direction", VC, "live", PU, "parse_ui", KEYS, VM_STR),
    r("voxel_text", Kind::VixiKeyword, "ui voxel-text content (3D font)", VC, "partial", PU, "parse_ui", KEYS, VM_IMM),
    r("voxel_material", Kind::VixiKeyword, "ui voxel-text material", VC, "partial", PU, "parse_ui", KEYS, VM_IMM),
    r("spring_in", Kind::VixiKeyword, "ui spring-in motion", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("spring_hover", Kind::VixiKeyword, "ui spring-hover motion", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("particle", Kind::VixiKeyword, "ui particle emitter", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("particle_selected", Kind::VixiKeyword, "ui selected-state particles", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("fill", Kind::VixiKeyword, "ui fill (progress/meter)", VC, "live", PU, "parse_ui", KEYS, VM_KIN),
    r("sound_show", Kind::VixiKeyword, "sound on show", SB, "partial", PU, "parse_ui", KEYS, VM_KIN),
    r("sound_dismiss", Kind::VixiKeyword, "sound on dismiss", SB, "partial", PU, "parse_ui", KEYS, VM_KIN),
    r("sound_hover", Kind::VixiKeyword, "sound on hover", SB, "partial", PU, "parse_ui", KEYS, VM_KIN),
    r("sound_select", Kind::VixiKeyword, "sound on select", SB, "partial", PU, "parse_ui", KEYS, VM_KIN),

    // ── theme body keys (parse_theme) ──
    r("token", Kind::VixiKeyword, "a named token binding", VC, "live", PT, "parse_theme", KEYS, VM_CTL),
    r("layer", Kind::VixiKeyword, "theme cascade layer selector", VC, "live", PT, "parse_theme", KEYS, VM_CTL),

    // ── theme layers (VIXEL_THEME_LAYERS) ──
    r("base", Kind::VixiKeyword, "engine base theme layer", VC, "live", PT, "parse_theme", LAYERS, VM_IMM),
    r("profile", Kind::VixiKeyword, "visual-profile theme layer", VC, "live", PT, "parse_theme", LAYERS, VM_IMM),
    r("celestial", Kind::VixiKeyword, "runtime celestial theme layer", VC, "live", PT, "parse_theme", LAYERS, VM_IMM),
    r("override", Kind::VixiKeyword, "explicit override theme layer", VC, "live", PT, "parse_theme", LAYERS, VM_IMM),

    // ── atom types (runtime). VixelAtom is JOINED: authorable via the `atom {}` block
    //    (parse_atom → VIXEL_ATOM_KEYS), so its CST node sings in the TKNO terminal. The
    //    rest stay runtime-only (ast_arm=—, lsp_mirror=NOT_PRESENT) until wired the same way. ──
    r("VixelAtom", Kind::AtomType, "8-byte unified engine atom (IPC/SoA)", VC, "live", "forge_daemon_types::atom::VixelAtom", "parse_atom", "VIXEL_ATOM_KEYS", NP),
    r("AtomicCanvasChunk", Kind::AtomType, "64-byte L1 chunk of 8 atoms", VC, "live", "forge_daemon_types::atom::AtomicCanvasChunk", "—", NP, NP),
    r("VixelAtom", Kind::AtomType, "28-byte GPU atom (pos+Z/material/opacity/flags)", VC, "live", "forge_gpu::vixel_pass::VixelAtom", "—", NP, NP),
    r("VixelViewport", Kind::AtomType, "GPU camera/viewport uniform (32B std140)", RND, "live", "forge_gpu::vixel_pass::VixelViewport", "parse_viewport", "VIXEL_KEYWORDS", NP),
    r("VixelDiff", Kind::AtomType, "18-byte mutation diff (little-nistam IPC)", VC, "live", "forge_daemon_types::atom::VixelDiff", "—", NP, NP),
    r("VixelDiff", Kind::AtomType, "18-byte chunk diff (rollback ring)", VC, "live", "forge_core::diff_pool::VixelDiff", "—", NP, NP),
    r("rule_ignite", Kind::AtomType, "fire-spread automata rule", VC, "live", "forge_shaders::vixel_automata::rule_ignite", "—", NP, NP),
    r("rule_gravity", Kind::AtomType, "gravity-settle automata rule", VC, "live", "forge_shaders::vixel_automata::rule_gravity", "—", NP, NP),
    r("rule_fluid_flow", Kind::AtomType, "fluid-flow automata rule", VC, "live", "forge_shaders::vixel_automata::rule_fluid_flow", "—", NP, NP),

    // ── forge-export: canvas PNG / GIF / spritesheet egress (foundation.vixel-canvas) ──
    r("export_png",          Kind::AtomType, "encode canvas RGBA8 frame as PNG bytes",                          VC, "live", "forge_export::export_png",                    "—", NP, NP),
    r("export_spritesheet",  Kind::AtomType, "pack RGBA8 frames into a sprite-sheet PNG",                       VC, "live", "forge_export::export_spritesheet",             "—", NP, NP),
    r("import_png",          Kind::AtomType, "decode PNG bytes → canvas RGBA8 + dimensions",                    VC, "live", "forge_export::import_png",                    "—", NP, NP),
    r("export_gif",          Kind::AtomType, "encode RGBA8 frames as animated GIF (NeuQuant quantised)",        VC, "live", "forge_export::gif_writer::export_gif",         "—", NP, NP),
    r("export_gif_indexed",  Kind::AtomType, "encode ColourID-indexed frames as GIF (sovereign, lossless 1:1)", VC, "live", "forge_export::gif_writer::export_gif_indexed", "—", NP, NP),

    // ── forge-core: multi-layer canvas paint stack (foundation.vixel-canvas, D13) ──
    r("LayerStack",   Kind::AtomType, "paint on multiple stacked layers (each a full VibeBuffer); coverage painter's composite preserves the colour_id", VC, "live", "forge_core::layer_stack::LayerStack", "parse_layers", "VIXEL_KEYWORDS", NP),
    r("flatten_into", Kind::AtomType, "composite the visible layers into a zero-alloc RGBA8 plane (CanvasWindow artboard / FrameComposer LayerPlane)", VC, "live", "forge_core::layer_stack::LayerStack::flatten_into", "—", NP, NP),

    // ── forge-core: authored voxel-brush falloff masks (foundation.vixel-canvas, ADR-0016) ──
    r("BrushMask",     Kind::AtomType, "w×h permyriad falloff grid (0..=10000); authored .exr shape source, float decode firewalled to forge-export",                                        VC, "live", "forge_core::brush_mask::BrushMask", "parse_brush", "VIXEL_KEYWORDS", NP),
    r("MaskStamp",     Kind::AtomType, "a BrushMask scaled to paint radius; implements BrushStamp so every heightbrush sculpt op (raise/lower/smooth/flatten/slope/carve) is generic over it", VC, "live", "forge_core::brush_mask::MaskStamp", "—", NP, NP),
    r("voxel_brushes", Kind::AtomType, "baked FBM1 little-nistam library of 35 authored falloff stamps (voxel_brushes.fbpack); host-agnostic pack, mounted by forge-gui (forge-tui: NOT_PRESENT, corrected 2026-07-06 Orphan-Wire Law drain — was claimed, zero refs on disk)", VC, "live", "forge_core::brush_mask::BrushPack", "—", NP, NP),

    // ── forge-core: pen-pressure → brush strength seam (foundation.vixel-canvas, ADR-0016 addendum) ──
    r("PressureCurve", Kind::AtomType, "pen-pressure feel curve (Linear/Soft/Hard); integer gamma maps QuantizedTabletSample.pressure permyriad → applied strength (no powf, endpoints pinned, monotonic)", VC, "live", "forge_core::pressure::PressureCurve",    "parse_pressure", "VIXEL_KEYWORDS", NP),
    r("with_pressure", Kind::AtomType, "scale a Stamp/MaskStamp strength_q16 by curved pen pressure — the Wacom↔voxel-brush seam (pressure-sensitive sculpt; no forge-input crate edge)",                     VC, "live", "forge_core::pressure::pressure_strength", "—", NP, NP),

    // ── forge-core: the VOXEL-ACRYLIC colour/material brush (foundation.vixel-canvas, ADR-0016 colour-brush follow-up) ──
    r("AcrylicLoad",   Kind::AtomType, "the paint LOAD a voxel-acrylic dab deposits: full VixelAtom (colour_id + material_id + essence_id + phase) + buildable opaque flow — bridges the authored falloff tip into the canvas atoms",        VC, "live", "forge_core::acrylic::AcrylicLoad",   "parse_acrylic", "VIXEL_KEYWORDS", NP),
    r("stamp_acrylic", Kind::AtomType, "deposit a BrushMask falloff into a VibeBuffer as buildable colour/material/essence (acrylic: overlap thickens, saturating to 255); authored shape paints its silhouette, not a hard disc",          VC, "live", "forge_core::acrylic::stamp_acrylic", "—", NP, NP),

    // ── REVIVED 2026-07-11: forge-kv-math returned to canon from the airgap (crates/forge-kv-math,
    //    lib+registry compile-green, 10/10 unit). inv #7/#156 CPU==GPU integer parity are LIVE again.
    //    ast_arm/lsp_mirror stay — / NOT_PRESENT: these are determinism kernels, not .vixel-authored. ──
    r("prismatic_hash_u32",       Kind::AtomType, "integer u32 hash proven bit-identical CPU==GPU (inv.#7, codepoint U+E101)",                                            DET, "live", "forge_kv_math::registry::SemanticPrimitive::PrismaticHashU32",       "—", NP, NP),
    r("permyriad_mul_div_i64",    Kind::AtomType, "i64 Permyriad mul/div proven bit-identical CPU==GPU on native AND vec2<u32> emulated (any GPU) (inv.#156, U+E102)",     DET, "live", "forge_kv_math::registry::SemanticPrimitive::PermyriadMulDivI64",     "—", NP, NP),
    r("stat_codepoint_permyriad", Kind::AtomType, "FNV-1a semantic codepoints proven safe operands through GPU Permyriad arithmetic, bit-identical CPU==GPU (U+E103)",     DET, "live", "forge_kv_math::registry::SemanticPrimitive::StatCodepointPermyriad", "—", NP, NP),

    // ── forge-vision: forgewright QAQC "always-on lens" (foundation.visual-verify, ADR-0008 Proof Law) ──
    r("WindowDriver",       Kind::AtomType, "drive a live product window — focus + inject input (Win32 SendInput) + xcap capture — for deterministic visual QAQC", VV, "live", "forge_vision::window_driver::WindowDriver",     "—", NP, NP),
    r("compress_frame_cpu", Kind::AtomType, "tile-delta frame compression: only the tiles that changed vs the previous capture (the lens' change-detector)",     VV, "live", "forge_vision::visual_debug::compress_frame_cpu", "—", NP, NP),
    r("perceptual_hash",    Kind::AtomType, "64-bit BT.601 average-hash of a captured frame (the regression baseline key)",                                        VV, "live", "forge_vision::visual_debug::perceptual_hash",    "—", NP, NP),

    // ── FOLDED COMPUTE STACK (13engine → NewRepo fold 2026-07-11): on-demand MoE / MoM / task-DAG engine
    //    capabilities — the answer to "on-demand incremental computation engine". Non-.vixel (ast_arm=—,
    //    lsp_mirror/highlight=NOT_PRESENT); oracle = realising crate::fn (ADR-0010). Each source is bundle-cited
    //    in atom_rows_exist_in_source below so a rename/delete of the folded crate fails loud. ──
    r("MoeRouter",        Kind::AtomType, "MoE expert router <TOTAL_CELLS,T>: on-demand expert select", MOE,  "live", "forge_hal::expert_pool::MoeRouter",    "—", NP, NP),
    r("MoeRouterSoA",     Kind::AtomType, "MoE expert router SoA (from_aos): cache-hot batch route",    MOE,  "live", "forge_hal::expert_pool::MoeRouterSoA", "—", NP, NP),
    r("Musician",         Kind::AtomType, "MoM musician expert trait (7 families)",                     MOM,  "live", "nde_core::musician::Musician",         "—", NP, NP),
    r("MomBus",           Kind::AtomType, "MoM accumulate/mix bus (PDC-aligned)",                       MOM,  "live", "nde_core::bus::MomBus",                "—", NP, NP),
    r("topological_sort", Kind::AtomType, "task DAG Kahn topo-sort + budget schedule",                  TDAG, "live", "forge_dag::topological_sort",   "—", NP, NP),
    r("MomRouter",        Kind::AtomType, "MoM 7x7 cell router (seeded, PDC)",                          MOM,  "live", "nde_core::mom_router::MomRouter",             "—", NP, NP),
    r("Conductor",        Kind::AtomType, "MoM conductor: register/dispatch active musicians",          MOM,  "live", "nde_core::conductor::Conductor",             "—", NP, NP),
    r("GpuDsp",           Kind::AtomType, "MoE-routed GPU DSP pipeline (cuFFT/STFT/soft-mask)",          MOE,  "live", "moe_gpu_dsp::pipeline::GpuDsp",              "—", NP, NP),
    r("ExecutionDag",     Kind::AtomType, "sealed-immutable exec DAG (budget/strike/complete)",         TDAG, "live", "forge_dag::ExecutionDag",                    "—", NP, NP),
    r("EffectDispatcher", Kind::AtomType, "semantic effect dispatch layer",                            SEM,  "live", "forge_semantic::dispatch::EffectDispatcher", "—", NP, NP),
    r("AuthorityLedger",  Kind::AtomType, "semantic seed/authority ledger",                            SEM,  "live", "forge_semantic::ledger::AuthorityLedger",    "—", NP, NP),
    r("run_meaning_budget_audit", Kind::AtomType, "meaning-budget audit -> report (semantic cost)",    SEM,  "live", "forge_meaning_budget::meaning_budget::run_meaning_budget_audit", "—", NP, NP),
    r("InferEngine",      Kind::AtomType, "sovereign neural inference engine (entry inference_api::infer)", INF, "live", "forge_ml::infer::InferEngine",           "—", NP, NP),
    r("LedgerEvent",      Kind::AtomType, "deterministic authoring: locked ledger commit event",        AUTH, "live", "ironroot_creation_engine::ledger::LedgerEvent", "—", NP, NP),
    r("SignalProxy",      Kind::AtomType, "quantized signal routing (live -> deterministic gate)",      SIG,  "live", "ironroot_signal::proxy::SignalProxy",        "—", NP, NP),
    // ── forge-ml deep surface (sovereign ML crate — inference/MoE/safety/training, 2026-07-08 fold) ──
    r("CudaInferEngine",   Kind::AtomType, "CUDA GPU inference engine (feature-gated)",                 INF,  "live", "forge_ml::cuda_infer::CudaInferEngine",        "—", NP, NP),
    r("HierarchicalMoe",   Kind::AtomType, "7-700-7 reasoning MoE (MetaRouter->SubRouter, 49 experts)", MOE,  "live", "forge_ml::hierarchical_moe::HierarchicalMoe",  "—", NP, NP),
    r("MetaRouter",        Kind::AtomType, "distilled BQ 1-of-7 domain router (<1us, ~364B)",           MOE,  "live", "forge_ml::metarouter::MetaRouter",             "—", NP, NP),
    r("BqRouter",          Kind::AtomType, "binary-quantized hamming router (64B XOR+POPCNT)",          MOE,  "live", "forge_ml::bq_router::BqRouter",                "—", NP, NP),
    r("ExpertCache",       Kind::AtomType, "hot/warm/cold expert weight tiering",                       MOE,  "live", "forge_ml::expert_cache::ExpertCache",          "—", NP, NP),
    r("SpatialPrior",      Kind::AtomType, "64B executable spatial prior (registry identity)",          INF,  "live", "forge_ml::spatial_prior::SpatialPrior",        "—", NP, NP),
    r("SafetyRouter",      Kind::AtomType, "14-expert safety ensemble router",                          SAFE, "live", "forge_ml::safety_router::SafetyRouter",        "—", NP, NP),
    r("BigByteClassifier", Kind::AtomType, "byte-level SAFE/DANGER expert classifier",                  SAFE, "live", "forge_ml::byte_classifier::BigByteClassifier", "—", NP, NP),
    r("Guillotine",        Kind::AtomType, "inference kill-switch (response interception)",             SAFE, "live", "forge_ml::guillotine::Guillotine",             "—", NP, NP),
    r("GateLadder",        Kind::AtomType, "gate-ladder test/explain/create/fix sequence",             SAFE, "live", "forge_ml::gate_ladder::GateLadder",            "—", NP, NP),
    r("ShadowSeer",        Kind::AtomType, "sovereign self-training flywheel observer",                 TRAIN,"live", "forge_ml::shadowseer::ShadowSeer",             "—", NP, NP),
    r("DistillPipeline",   Kind::AtomType, "dual-school teacher->student distillation",                 TRAIN,"live", "forge_ml::distill_pipeline::DistillPipeline",  "—", NP, NP),
    // ── remaining fold surface: forge-semantic / forge-hal / forge-dag / moe-gpu-dsp / nde_core / ironroot-* deep ──
    r("SeedInventory",     Kind::AtomType, "semantic seed inventory (authority grants)",                SEM,  "live", "forge_semantic::ledger::SeedInventory",        "—", NP, NP),
    r("ExpertPool",        Kind::AtomType, "HAL expert weight pool (MoE backing store)",                MOE,  "live", "forge_hal::expert_pool::ExpertPool",           "—", NP, NP),
    r("TaskNode",          Kind::AtomType, "DAG task node (id/deps/file/readiness)",                    TDAG, "live", "forge_dag::TaskNode",                          "—", NP, NP),
    r("DspFrame",          Kind::AtomType, "MoE-DSP output frame (bridge result)",                      MOE,  "live", "moe_gpu_dsp::dsp_bridge::DspFrame",            "—", NP, NP),
    r("UmpSender",         Kind::AtomType, "MoM UMP ring sender (conductor pair)",                      MOM,  "live", "nde_core::mom_rt::UmpSender",                  "—", NP, NP),
    r("WorldMetronome",    Kind::AtomType, "quantized world metronome (tick clock)",                    SIG,  "live", "ironroot_signal::metronome::WorldMetronome",   "—", NP, NP),
    r("WorldParameterBus", Kind::AtomType, "world ambient parameter bus",                               SIG,  "live", "ironroot_signal::parameter_bus::WorldParameterBus", "—", NP, NP),
    r("CreationStamp",     Kind::AtomType, "ledgered creation stamp (replayable fx)",                   SIG,  "live", "ironroot_signal::stamp::CreationStamp",        "—", NP, NP),
    r("ArtifactStatus",    Kind::AtomType, "Draft->Preview->Locked artifact lifecycle",                 AUTH, "live", "ironroot_creation_engine::artifact::ArtifactStatus", "—", NP, NP),
    r("RelationKind",      Kind::AtomType, "toybox artifact-graph relation kinds",                      AUTH, "live", "ironroot_creation_engine::graph::RelationKind","—", NP, NP),
    r("ChoiceArchetype",   Kind::AtomType, "CYOA scene-sieve choice archetype",                         AUTH, "live", "ironroot_creation_engine::cyoa::ChoiceArchetype", "—", NP, NP),
    r("AcousticKind", Kind::AtomType, "enum forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::AcousticKind", "-", NP, NP),
    r("AcousticCell", Kind::AtomType, "struct forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::AcousticCell", "-", NP, NP),
    r("AcousticCodebook", Kind::AtomType, "struct forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::AcousticCodebook", "-", NP, NP),
    r("BlendedSection", Kind::AtomType, "struct forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::BlendedSection", "-", NP, NP),
    r("FilterIntent", Kind::AtomType, "enum forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::FilterIntent", "-", NP, NP),
    r("IntentVector", Kind::AtomType, "struct forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::IntentVector", "-", NP, NP),
    r("IntentResolution", Kind::AtomType, "enum forge_ml::acoustic_index", ML, "live", "forge_ml::acoustic_index::IntentResolution", "-", NP, NP),
    r("TensorStats", Kind::AtomType, "struct forge_ml::analyze", ML, "live", "forge_ml::analyze::TensorStats", "-", NP, NP),
    r("WeightAnalysis", Kind::AtomType, "struct forge_ml::analyze", ML, "live", "forge_ml::analyze::WeightAnalysis", "-", NP, NP),
    r("LoraMapping", Kind::AtomType, "struct forge_ml::backward", ML, "live", "forge_ml::backward::LoraMapping", "-", NP, NP),
    r("BqCentroid", Kind::AtomType, "struct forge_ml::bq_router", ML, "live", "forge_ml::bq_router::BqCentroid", "-", NP, NP),
    r("TrainingPairSlim", Kind::AtomType, "struct forge_ml::bq_router", ML, "live", "forge_ml::bq_router::TrainingPairSlim", "-", NP, NP),
    r("FlywheelTrainStats", Kind::AtomType, "struct forge_ml::bq_router", ML, "live", "forge_ml::bq_router::FlywheelTrainStats", "-", NP, NP),
    r("ByteSequenceClassifier", Kind::AtomType, "struct forge_ml::byte_classifier", ML, "live", "forge_ml::byte_classifier::ByteSequenceClassifier", "-", NP, NP),
    r("ForwardCache", Kind::AtomType, "struct forge_ml::byte_classifier", ML, "live", "forge_ml::byte_classifier::ForwardCache", "-", NP, NP),
    r("Xorshift64", Kind::AtomType, "struct forge_ml::byte_classifier", ML, "live", "forge_ml::byte_classifier::Xorshift64", "-", NP, NP),
    r("BigForwardCache", Kind::AtomType, "struct forge_ml::byte_classifier", ML, "live", "forge_ml::byte_classifier::BigForwardCache", "-", NP, NP),
    r("ByteTokenizer", Kind::AtomType, "struct forge_ml::byte_corpus", ML, "live", "forge_ml::byte_corpus::ByteTokenizer", "-", NP, NP),
    r("CanonicalJsonError", Kind::AtomType, "enum forge_ml::canonical_json", ML, "live", "forge_ml::canonical_json::CanonicalJsonError", "-", NP, NP),
    r("CanonicalValue", Kind::AtomType, "enum forge_ml::canonical_json", ML, "live", "forge_ml::canonical_json::CanonicalValue", "-", NP, NP),
    r("ContextChunk", Kind::AtomType, "struct forge_ml::context_assembler", ML, "live", "forge_ml::context_assembler::ContextChunk", "-", NP, NP),
    r("ContextIndex", Kind::AtomType, "struct forge_ml::context_assembler", ML, "live", "forge_ml::context_assembler::ContextIndex", "-", NP, NP),
    r("ContextHit", Kind::AtomType, "struct forge_ml::context_assembler", ML, "live", "forge_ml::context_assembler::ContextHit", "-", NP, NP),
    r("TurnAnchor", Kind::AtomType, "struct forge_ml::conversation", ML, "live", "forge_ml::conversation::TurnAnchor", "-", NP, NP),
    r("ConversationState", Kind::AtomType, "struct forge_ml::conversation", ML, "live", "forge_ml::conversation::ConversationState", "-", NP, NP),
    r("ChunkFormat", Kind::AtomType, "enum forge_ml::corpus", ML, "live", "forge_ml::corpus::ChunkFormat", "-", NP, NP),
    r("Chunk", Kind::AtomType, "struct forge_ml::corpus", ML, "live", "forge_ml::corpus::Chunk", "-", NP, NP),
    r("SearchResult", Kind::AtomType, "struct forge_ml::corpus", ML, "live", "forge_ml::corpus::SearchResult", "-", NP, NP),
    r("Bm25Index", Kind::AtomType, "struct forge_ml::corpus", ML, "live", "forge_ml::corpus::Bm25Index", "-", NP, NP),
    r("TrainingPair", Kind::AtomType, "struct forge_ml::corpus", ML, "live", "forge_ml::corpus::TrainingPair", "-", NP, NP),
    r("DcgsFormatter", Kind::AtomType, "struct forge_ml::dcgs_formatter", ML, "live", "forge_ml::dcgs_formatter::DcgsFormatter", "-", NP, NP),
    r("Dcgs", Kind::AtomType, "struct forge_ml::dcgs", ML, "live", "forge_ml::dcgs::Dcgs", "-", NP, NP),
    r("DedupTable", Kind::AtomType, "struct forge_ml::dedup", ML, "live", "forge_ml::dedup::DedupTable", "-", NP, NP),
    r("QualityBaseline", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::QualityBaseline", "-", NP, NP),
    r("RegressionAlert", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::RegressionAlert", "-", NP, NP),
    r("CheckpointRotation", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::CheckpointRotation", "-", NP, NP),
    r("MergePolicy", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::MergePolicy", "-", NP, NP),
    r("DistributionShift", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::DistributionShift", "-", NP, NP),
    r("PipelineActions", Kind::AtomType, "struct forge_ml::distill_pipeline", ML, "live", "forge_ml::distill_pipeline::PipelineActions", "-", NP, NP),
    r("DistillStats", Kind::AtomType, "struct forge_ml::distill_thread", ML, "live", "forge_ml::distill_thread::DistillStats", "-", NP, NP),
    r("DistillConfig", Kind::AtomType, "struct forge_ml::distill_thread", ML, "live", "forge_ml::distill_thread::DistillConfig", "-", NP, NP),
    r("ExpertNode", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::ExpertNode", "-", NP, NP),
    r("LearningEvent", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::LearningEvent", "-", NP, NP),
    r("LearningKind", Kind::AtomType, "enum forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::LearningKind", "-", NP, NP),
    r("GhostHint", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::GhostHint", "-", NP, NP),
    r("EngineSignal", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::EngineSignal", "-", NP, NP),
    r("SwapPulse", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::SwapPulse", "-", NP, NP),
    r("SignalBus", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::SignalBus", "-", NP, NP),
    r("SignalReader", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::SignalReader", "-", NP, NP),
    r("SignalWriter", Kind::AtomType, "struct forge_ml::engine_signal", ML, "live", "forge_ml::engine_signal::SignalWriter", "-", NP, NP),
    r("EphemeralEnvelope", Kind::AtomType, "struct forge_ml::envelope", ML, "live", "forge_ml::envelope::EphemeralEnvelope", "-", NP, NP),
    r("Tier", Kind::AtomType, "enum forge_ml::esl", ML, "live", "forge_ml::esl::Tier", "-", NP, NP),
    r("Domain", Kind::AtomType, "enum forge_ml::esl", ML, "live", "forge_ml::esl::Domain", "-", NP, NP),
    r("Intent", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::Intent", "-", NP, NP),
    r("Grammar", Kind::AtomType, "enum forge_ml::esl", ML, "live", "forge_ml::esl::Grammar", "-", NP, NP),
    r("ProjectFrontmatter", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::ProjectFrontmatter", "-", NP, NP),
    r("ProjectContext", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::ProjectContext", "-", NP, NP),
    r("AnchorPacket", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::AnchorPacket", "-", NP, NP),
    r("McpServer", Kind::AtomType, "enum forge_ml::esl", ML, "live", "forge_ml::esl::McpServer", "-", NP, NP),
    r("McpToolFilter", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::McpToolFilter", "-", NP, NP),
    r("DialogueSlice", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::DialogueSlice", "-", NP, NP),
    r("NarrationSlice", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::NarrationSlice", "-", NP, NP),
    r("GameContext", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::GameContext", "-", NP, NP),
    r("ContentTrigger", Kind::AtomType, "enum forge_ml::esl", ML, "live", "forge_ml::esl::ContentTrigger", "-", NP, NP),
    r("GameAnchorPacket", Kind::AtomType, "struct forge_ml::esl", ML, "live", "forge_ml::esl::GameAnchorPacket", "-", NP, NP),
    r("ExpertId", Kind::AtomType, "type forge_ml::expert_cache", ML, "live", "forge_ml::expert_cache::ExpertId", "-", NP, NP),
    r("Tier", Kind::AtomType, "enum forge_ml::expert_cache", ML, "live", "forge_ml::expert_cache::Tier", "-", NP, NP),
    r("CacheStats", Kind::AtomType, "struct forge_ml::expert_cache", ML, "live", "forge_ml::expert_cache::CacheStats", "-", NP, NP),
    r("ExplorationConfig", Kind::AtomType, "struct forge_ml::exploration", ML, "live", "forge_ml::exploration::ExplorationConfig", "-", NP, NP),
    r("SubRouterState", Kind::AtomType, "enum forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::SubRouterState", "-", NP, NP),
    r("DistillationReadiness", Kind::AtomType, "enum forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::DistillationReadiness", "-", NP, NP),
    r("SubExpertComboStats", Kind::AtomType, "struct forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::SubExpertComboStats", "-", NP, NP),
    r("SpecialistSummary", Kind::AtomType, "struct forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::SpecialistSummary", "-", NP, NP),
    r("FlywheelReport", Kind::AtomType, "struct forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::FlywheelReport", "-", NP, NP),
    r("FlywheelAnalyzer", Kind::AtomType, "struct forge_ml::flywheel_analyzer", ML, "live", "forge_ml::flywheel_analyzer::FlywheelAnalyzer", "-", NP, NP),
    r("Flywheel", Kind::AtomType, "struct forge_ml::flywheel", ML, "live", "forge_ml::flywheel::Flywheel", "-", NP, NP),
    r("FlywheelConfig", Kind::AtomType, "struct forge_ml::flywheel", ML, "live", "forge_ml::flywheel::FlywheelConfig", "-", NP, NP),
    r("ForgetGate", Kind::AtomType, "struct forge_ml::forget_gate", ML, "live", "forge_ml::forget_gate::ForgetGate", "-", NP, NP),
    r("ModelConfig", Kind::AtomType, "struct forge_ml::format", ML, "live", "forge_ml::format::ModelConfig", "-", NP, NP),
    r("NdeModel", Kind::AtomType, "struct forge_ml::format", ML, "live", "forge_ml::format::NdeModel", "-", NP, NP),
    r("Gate", Kind::AtomType, "enum forge_ml::gate_ladder", ML, "live", "forge_ml::gate_ladder::Gate", "-", NP, NP),
    r("GateVerdict", Kind::AtomType, "enum forge_ml::gate_ladder", ML, "live", "forge_ml::gate_ladder::GateVerdict", "-", NP, NP),
    r("GateResult", Kind::AtomType, "struct forge_ml::gate_ladder", ML, "live", "forge_ml::gate_ladder::GateResult", "-", NP, NP),
    r("GateFailure", Kind::AtomType, "struct forge_ml::gate_ladder", ML, "live", "forge_ml::gate_ladder::GateFailure", "-", NP, NP),
    r("GateResultLegacy", Kind::AtomType, "struct forge_ml::gate_ladder", ML, "live", "forge_ml::gate_ladder::GateResultLegacy", "-", NP, NP),
    r("GbnfConstraint", Kind::AtomType, "struct forge_ml::gbnf_sampler", ML, "live", "forge_ml::gbnf_sampler::GbnfConstraint", "-", NP, NP),
    r("GpuTrainContext", Kind::AtomType, "struct forge_ml::gpu_train", ML, "live", "forge_ml::gpu_train::GpuTrainContext", "-", NP, NP),
    r("SubExpertSlice", Kind::AtomType, "struct forge_ml::hierarchical_moe", ML, "live", "forge_ml::hierarchical_moe::SubExpertSlice", "-", NP, NP),
    r("SubRouter", Kind::AtomType, "struct forge_ml::hierarchical_moe", ML, "live", "forge_ml::hierarchical_moe::SubRouter", "-", NP, NP),
    r("DomainSpecialist", Kind::AtomType, "struct forge_ml::hierarchical_moe", ML, "live", "forge_ml::hierarchical_moe::DomainSpecialist", "-", NP, NP),
    r("GenerateConfig", Kind::AtomType, "struct forge_ml::infer", ML, "live", "forge_ml::infer::GenerateConfig", "-", NP, NP),
    r("InferenceError", Kind::AtomType, "enum forge_ml::inference_api", ML, "live", "forge_ml::inference_api::InferenceError", "-", NP, NP),
    r("InferenceResult", Kind::AtomType, "struct forge_ml::inference_api", ML, "live", "forge_ml::inference_api::InferenceResult", "-", NP, NP),
    r("SynthesisRequest", Kind::AtomType, "struct forge_ml::inference_api", ML, "live", "forge_ml::inference_api::SynthesisRequest", "-", NP, NP),
    r("JoinGate", Kind::AtomType, "struct forge_ml::join", ML, "live", "forge_ml::join::JoinGate", "-", NP, NP),
    r("SplicedBins", Kind::AtomType, "struct forge_ml::join", ML, "live", "forge_ml::join::SplicedBins", "-", NP, NP),
    r("LiveTapOverride", Kind::AtomType, "struct forge_ml::live_tap_override", ML, "live", "forge_ml::live_tap_override::LiveTapOverride", "-", NP, NP),
    r("QueryRecord", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::QueryRecord", "-", NP, NP),
    r("ResponseSlot", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::ResponseSlot", "-", NP, NP),
    r("EscalationReason", Kind::AtomType, "enum forge_ml::live_tap", ML, "live", "forge_ml::live_tap::EscalationReason", "-", NP, NP),
    r("DistillSource", Kind::AtomType, "enum forge_ml::live_tap", ML, "live", "forge_ml::live_tap::DistillSource", "-", NP, NP),
    r("PayloadKind", Kind::AtomType, "enum forge_ml::live_tap", ML, "live", "forge_ml::live_tap::PayloadKind", "-", NP, NP),
    r("PayloadRef", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::PayloadRef", "-", NP, NP),
    r("StreamMeta", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::StreamMeta", "-", NP, NP),
    r("TrainingPair", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::TrainingPair", "-", NP, NP),
    r("TapStats", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::TapStats", "-", NP, NP),
    r("MarginTracker", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::MarginTracker", "-", NP, NP),
    r("SessionState", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::SessionState", "-", NP, NP),
    r("QueryRingSender", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::QueryRingSender", "-", NP, NP),
    r("QueryRingReceiver", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::QueryRingReceiver", "-", NP, NP),
    r("TapInterceptor", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::TapInterceptor", "-", NP, NP),
    r("OutcomeScorer", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::OutcomeScorer", "-", NP, NP),
    r("FlywheelLogger", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::FlywheelLogger", "-", NP, NP),
    r("ShadowInferThread", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::ShadowInferThread", "-", NP, NP),
    r("ModelLoader", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::ModelLoader", "-", NP, NP),
    r("SessionDistillConfig", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::SessionDistillConfig", "-", NP, NP),
    r("SessionDistillReport", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::SessionDistillReport", "-", NP, NP),
    r("SessionDistiller", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::SessionDistiller", "-", NP, NP),
    r("LiveTapSession", Kind::AtomType, "struct forge_ml::live_tap", ML, "live", "forge_ml::live_tap::LiveTapSession", "-", NP, NP),
    r("LoraAdapter", Kind::AtomType, "struct forge_ml::lora", ML, "live", "forge_ml::lora::LoraAdapter", "-", NP, NP),
    r("LoraGradAcc", Kind::AtomType, "struct forge_ml::lora", ML, "live", "forge_ml::lora::LoraGradAcc", "-", NP, NP),
    r("LoraBundle", Kind::AtomType, "struct forge_ml::lora", ML, "live", "forge_ml::lora::LoraBundle", "-", NP, NP),
    r("SourceFormat", Kind::AtomType, "enum forge_ml::master_decode", ML, "live", "forge_ml::master_decode::SourceFormat", "-", NP, NP),
    r("DecodeRoute", Kind::AtomType, "enum forge_ml::master_decode", ML, "live", "forge_ml::master_decode::DecodeRoute", "-", NP, NP),
    r("NoteEvent", Kind::AtomType, "struct forge_ml::master_decode", ML, "live", "forge_ml::master_decode::NoteEvent", "-", NP, NP),
    r("MlpMemory", Kind::AtomType, "struct forge_ml::memory", ML, "live", "forge_ml::memory::MlpMemory", "-", NP, NP),
    r("AdamState", Kind::AtomType, "struct forge_ml::moe_train", ML, "live", "forge_ml::moe_train::AdamState", "-", NP, NP),
    r("GradAccumulator", Kind::AtomType, "struct forge_ml::moe_train", ML, "live", "forge_ml::moe_train::GradAccumulator", "-", NP, NP),
    r("MoeParams", Kind::AtomType, "struct forge_ml::moe_train", ML, "live", "forge_ml::moe_train::MoeParams", "-", NP, NP),
    r("ForwardCache", Kind::AtomType, "struct forge_ml::moe_train", ML, "live", "forge_ml::moe_train::ForwardCache", "-", NP, NP),
    r("TrainConfig", Kind::AtomType, "struct forge_ml::moe_train", ML, "live", "forge_ml::moe_train::TrainConfig", "-", NP, NP),
    r("CodeEntry", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::CodeEntry", "-", NP, NP),
    r("Ray5D", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::Ray5D", "-", NP, NP),
    r("CreeCell", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::CreeCell", "-", NP, NP),
    r("RiverIdf", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::RiverIdf", "-", NP, NP),
    r("SovereignCoder", Kind::AtomType, "trait forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::SovereignCoder", "-", NP, NP),
    r("GhostMoonImpulse", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::GhostMoonImpulse", "-", NP, NP),
    r("GhostMoonBridge", Kind::AtomType, "struct forge_ml::nearest_neighbor", ML, "live", "forge_ml::nearest_neighbor::GhostMoonBridge", "-", NP, NP),
    r("SpeculativePrefetcher", Kind::AtomType, "struct forge_ml::prefetcher", ML, "live", "forge_ml::prefetcher::SpeculativePrefetcher", "-", NP, NP),
    r("PrngDraftModel", Kind::AtomType, "struct forge_ml::prng_draft", ML, "live", "forge_ml::prng_draft::PrngDraftModel", "-", NP, NP),
    r("QuantLevel", Kind::AtomType, "enum forge_ml::quantize", ML, "live", "forge_ml::quantize::QuantLevel", "-", NP, NP),
    r("QuantInfo", Kind::AtomType, "struct forge_ml::quantize", ML, "live", "forge_ml::quantize::QuantInfo", "-", NP, NP),
    r("QuantTensor", Kind::AtomType, "struct forge_ml::quantize", ML, "live", "forge_ml::quantize::QuantTensor", "-", NP, NP),
    r("QuantizedModel", Kind::AtomType, "struct forge_ml::quantize", ML, "live", "forge_ml::quantize::QuantizedModel", "-", NP, NP),
    r("Codeword", Kind::AtomType, "struct forge_ml::resonance_codec", ML, "live", "forge_ml::resonance_codec::Codeword", "-", NP, NP),
    r("ResonanceCodec", Kind::AtomType, "struct forge_ml::resonance_codec", ML, "live", "forge_ml::resonance_codec::ResonanceCodec", "-", NP, NP),
    r("QuadraticRouter", Kind::AtomType, "struct forge_ml::router", ML, "live", "forge_ml::router::QuadraticRouter", "-", NP, NP),
    r("EnsembleMeta", Kind::AtomType, "struct forge_ml::safety_moe", ML, "live", "forge_ml::safety_moe::EnsembleMeta", "-", NP, NP),
    r("SafetyMoe", Kind::AtomType, "struct forge_ml::safety_moe", ML, "live", "forge_ml::safety_moe::SafetyMoe", "-", NP, NP),
    r("DialogueState", Kind::AtomType, "struct forge_ml::seed_compressor", ML, "live", "forge_ml::seed_compressor::DialogueState", "-", NP, NP),
    r("SeedError", Kind::AtomType, "enum forge_ml::seed_compressor", ML, "live", "forge_ml::seed_compressor::SeedError", "-", NP, NP),
    r("SeedCompressor", Kind::AtomType, "struct forge_ml::seed_compressor", ML, "live", "forge_ml::seed_compressor::SeedCompressor", "-", NP, NP),
    r("ExpertHealth", Kind::AtomType, "struct forge_ml::shadowseer", ML, "live", "forge_ml::shadowseer::ExpertHealth", "-", NP, NP),
    r("SeerSnapshot", Kind::AtomType, "struct forge_ml::shadowseer", ML, "live", "forge_ml::shadowseer::SeerSnapshot", "-", NP, NP),
    r("SpatialPriorReject", Kind::AtomType, "enum forge_ml::spatial_prior", ML, "live", "forge_ml::spatial_prior::SpatialPriorReject", "-", NP, NP),
    r("SpatialPriorRecord", Kind::AtomType, "struct forge_ml::spatial_prior", ML, "live", "forge_ml::spatial_prior::SpatialPriorRecord", "-", NP, NP),
    r("CaptureError", Kind::AtomType, "enum forge_ml::spatial_prior", ML, "live", "forge_ml::spatial_prior::CaptureError", "-", NP, NP),
    r("SpatialCapture", Kind::AtomType, "struct forge_ml::spatial_prior", ML, "live", "forge_ml::spatial_prior::SpatialCapture", "-", NP, NP),
    r("KvEntry", Kind::AtomType, "struct forge_ml::speculative", ML, "live", "forge_ml::speculative::KvEntry", "-", NP, NP),
    r("KvCache", Kind::AtomType, "struct forge_ml::speculative", ML, "live", "forge_ml::speculative::KvCache", "-", NP, NP),
    r("SpeculativeDecoder", Kind::AtomType, "struct forge_ml::speculative", ML, "live", "forge_ml::speculative::SpeculativeDecoder", "-", NP, NP),
    r("StackLayer", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::StackLayer", "-", NP, NP),
    r("StackForwardCache", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::StackForwardCache", "-", NP, NP),
    r("StackGrads", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::StackGrads", "-", NP, NP),
    r("ByteScanner", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::ByteScanner", "-", NP, NP),
    r("WordBridge", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::WordBridge", "-", NP, NP),
    r("LanguageInterface", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::LanguageInterface", "-", NP, NP),
    r("MultiModalEmbed", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::MultiModalEmbed", "-", NP, NP),
    r("QualityGate", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::QualityGate", "-", NP, NP),
    r("StackedFlywheel", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::StackedFlywheel", "-", NP, NP),
    r("FlywheelOutput", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::FlywheelOutput", "-", NP, NP),
    r("LiveDistiller", Kind::AtomType, "struct forge_ml::stacked_flywheel", ML, "live", "forge_ml::stacked_flywheel::LiveDistiller", "-", NP, NP),
    r("StrikeRecord", Kind::AtomType, "struct forge_ml::strike_tracker", ML, "live", "forge_ml::strike_tracker::StrikeRecord", "-", NP, NP),
    r("Strike", Kind::AtomType, "struct forge_ml::strike_tracker", ML, "live", "forge_ml::strike_tracker::Strike", "-", NP, NP),
    r("StrikeReason", Kind::AtomType, "enum forge_ml::strike_tracker", ML, "live", "forge_ml::strike_tracker::StrikeReason", "-", NP, NP),
    r("StrikeVerdict", Kind::AtomType, "enum forge_ml::strike_tracker", ML, "live", "forge_ml::strike_tracker::StrikeVerdict", "-", NP, NP),
    r("StrikeTracker", Kind::AtomType, "struct forge_ml::strike_tracker", ML, "live", "forge_ml::strike_tracker::StrikeTracker", "-", NP, NP),
    r("TrainConfig", Kind::AtomType, "struct forge_ml::train", ML, "live", "forge_ml::train::TrainConfig", "-", NP, NP),
    r("TrainingExample", Kind::AtomType, "struct forge_ml::train", ML, "live", "forge_ml::train::TrainingExample", "-", NP, NP),
    r("EpochMetrics", Kind::AtomType, "struct forge_ml::train", ML, "live", "forge_ml::train::EpochMetrics", "-", NP, NP),
    r("Tensor", Kind::AtomType, "struct forge_ml::weights", ML, "live", "forge_ml::weights::Tensor", "-", NP, NP),
    r("TensorMap", Kind::AtomType, "type forge_ml::weights", ML, "live", "forge_ml::weights::TensorMap", "-", NP, NP),
    r("ModelWeights", Kind::AtomType, "struct forge_ml::weights", ML, "live", "forge_ml::weights::ModelWeights", "-", NP, NP),
    r("PdcDelayLine", Kind::AtomType, "struct nde_core::bus", MOM, "live", "nde_core::bus::PdcDelayLine", "-", NP, NP),
    r("Gravebell", Kind::AtomType, "struct nde_core::gravebell", MOM, "live", "nde_core::gravebell::Gravebell", "-", NP, NP),
    r("MomCell", Kind::AtomType, "type nde_core::mom_router", MOM, "live", "nde_core::mom_router::MomCell", "-", NP, NP),
    r("MomBusHook", Kind::AtomType, "struct nde_core::mom_rt", MOM, "live", "nde_core::mom_rt::MomBusHook", "-", NP, NP),
    r("RdBaseParams", Kind::AtomType, "struct nde_core::mom_viz", MOM, "live", "nde_core::mom_viz::RdBaseParams", "-", NP, NP),
    r("RdCellEvent", Kind::AtomType, "struct nde_core::mom_viz", MOM, "live", "nde_core::mom_viz::RdCellEvent", "-", NP, NP),
    r("MomVizFrame", Kind::AtomType, "struct nde_core::mom_viz", MOM, "live", "nde_core::mom_viz::MomVizFrame", "-", NP, NP),
    r("DspmcpReadback", Kind::AtomType, "struct nde_core::mom_viz", MOM, "live", "nde_core::mom_viz::DspmcpReadback", "-", NP, NP),
    r("Biquad", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::Biquad", "-", NP, NP),
    r("DelayLine", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::DelayLine", "-", NP, NP),
    r("DampedComb", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::DampedComb", "-", NP, NP),
    r("Allpass", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::Allpass", "-", NP, NP),
    r("EnvelopeFollower", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::EnvelopeFollower", "-", NP, NP),
    r("Lfo", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::Lfo", "-", NP, NP),
    r("WeightSmoother", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::WeightSmoother", "-", NP, NP),
    r("Freeverb", Kind::AtomType, "struct nde_core::nde_dsp", MOM, "live", "nde_core::nde_dsp::Freeverb", "-", NP, NP),
    r("UmpWord", Kind::AtomType, "struct nde_core::ump", MOM, "live", "nde_core::ump::UmpWord", "-", NP, NP),
    r("RoutingTag", Kind::AtomType, "struct nde_core::ump", MOM, "live", "nde_core::ump::RoutingTag", "-", NP, NP),
    r("RoutedUmp", Kind::AtomType, "struct nde_core::ump", MOM, "live", "nde_core::ump::RoutedUmp", "-", NP, NP),
    r("ViewMode", Kind::AtomType, "enum forge_semantic::bus", SEM, "live", "forge_semantic::bus::ViewMode", "-", NP, NP),
    r("BusEvent", Kind::AtomType, "enum forge_semantic::bus", SEM, "live", "forge_semantic::bus::BusEvent", "-", NP, NP),
    r("BindingEntry", Kind::AtomType, "struct forge_semantic::dispatch", SEM, "live", "forge_semantic::dispatch::BindingEntry", "-", NP, NP),
    r("GlyphBinding", Kind::AtomType, "struct forge_semantic::glyph", SEM, "live", "forge_semantic::glyph::GlyphBinding", "-", NP, NP),
    r("StoredTicket", Kind::AtomType, "struct forge_semantic::ledger", SEM, "live", "forge_semantic::ledger::StoredTicket", "-", NP, NP),
    r("ForgottenReceipt", Kind::AtomType, "struct forge_semantic::ledger", SEM, "live", "forge_semantic::ledger::ForgottenReceipt", "-", NP, NP),
    r("CommitRefusal", Kind::AtomType, "enum forge_semantic::ledger", SEM, "live", "forge_semantic::ledger::CommitRefusal", "-", NP, NP),
    r("IngressRefusal", Kind::AtomType, "enum forge_semantic::ledger", SEM, "live", "forge_semantic::ledger::IngressRefusal", "-", NP, NP),
    r("ExecLane", Kind::AtomType, "enum forge_semantic::quad_lane", SEM, "live", "forge_semantic::quad_lane::ExecLane", "-", NP, NP),
    r("LaneFanout", Kind::AtomType, "struct forge_semantic::quad_lane", SEM, "live", "forge_semantic::quad_lane::LaneFanout", "-", NP, NP),
    r("Conductor", Kind::AtomType, "struct forge_semantic::quad_lane", SEM, "live", "forge_semantic::quad_lane::Conductor", "-", NP, NP),
    r("RoomInferer", Kind::AtomType, "trait forge_semantic::room_inference", SEM, "live", "forge_semantic::room_inference::RoomInferer", "-", NP, NP),
    r("DeterministicInferer", Kind::AtomType, "struct forge_semantic::room_inference", SEM, "live", "forge_semantic::room_inference::DeterministicInferer", "-", NP, NP),
    r("SemanticInferer", Kind::AtomType, "struct forge_semantic::room_inference", SEM, "live", "forge_semantic::room_inference::SemanticInferer", "-", NP, NP),
    r("ModelInferer", Kind::AtomType, "struct forge_semantic::room_inference", SEM, "live", "forge_semantic::room_inference::ModelInferer", "-", NP, NP),
    r("DiagnoserId", Kind::AtomType, "struct forge_semantic::types", SEM, "live", "forge_semantic::types::DiagnoserId", "-", NP, NP),
    r("DispatchReport", Kind::AtomType, "struct forge_semantic::types", SEM, "live", "forge_semantic::types::DispatchReport", "-", NP, NP),
    r("DspBridge", Kind::AtomType, "struct moe_gpu_dsp::dsp_bridge", MOE, "live", "moe_gpu_dsp::dsp_bridge::DspBridge", "-", NP, NP),
    r("GpuDsp", Kind::AtomType, "struct moe_gpu_dsp", MOE, "live", "moe_gpu_dsp::GpuDsp", "-", NP, NP),
    r("DspConfig", Kind::AtomType, "struct moe_gpu_dsp", MOE, "live", "moe_gpu_dsp::DspConfig", "-", NP, NP),
    r("DspConfig", Kind::AtomType, "struct moe_gpu_dsp::pipeline", MOE, "live", "moe_gpu_dsp::pipeline::DspConfig", "-", NP, NP),
    r("DagError", Kind::AtomType, "enum forge_dag", TDAG, "live", "forge_dag::DagError", "-", NP, NP),
    r("ModelRoute", Kind::AtomType, "enum forge_dag", TDAG, "live", "forge_dag::ModelRoute", "-", NP, NP),
    r("Budget", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::Budget", "-", NP, NP),
    r("NodeStatus", Kind::AtomType, "enum forge_dag", TDAG, "live", "forge_dag::NodeStatus", "-", NP, NP),
    r("CapsulePayload", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::CapsulePayload", "-", NP, NP),
    r("ReconBudget", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::ReconBudget", "-", NP, NP),
    r("Readiness", Kind::AtomType, "enum forge_dag", TDAG, "live", "forge_dag::Readiness", "-", NP, NP),
    r("StateBrief", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::StateBrief", "-", NP, NP),
    r("Permit", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::Permit", "-", NP, NP),
    r("BriefRing", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::BriefRing", "-", NP, NP),
    r("TaskGraphError", Kind::AtomType, "type forge_dag", TDAG, "live", "forge_dag::TaskGraphError", "-", NP, NP),
    r("ModelRoute", Kind::AtomType, "enum forge_dag", TDAG, "live", "forge_dag::ModelRoute", "-", NP, NP),
    r("Budget", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::Budget", "-", NP, NP),
    r("TaskNode", Kind::AtomType, "struct forge_dag", TDAG, "live", "forge_dag::TaskNode", "-", NP, NP),
    r("ArtifactKind", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ArtifactKind", "-", NP, NP),
    r("Visibility", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::Visibility", "-", NP, NP),
    r("Rarity", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::Rarity", "-", NP, NP),
    r("Scope", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::Scope", "-", NP, NP),
    r("SourceKind", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::SourceKind", "-", NP, NP),
    r("Owner", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::Owner", "-", NP, NP),
    r("FactTag", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::FactTag", "-", NP, NP),
    r("ArtifactHeader", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ArtifactHeader", "-", NP, NP),
    r("ArtifactPayload", Kind::AtomType, "enum ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ArtifactPayload", "-", NP, NP),
    r("ProceduralArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ProceduralArtifact", "-", NP, NP),
    r("NpcArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::NpcArtifact", "-", NP, NP),
    r("ItemArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ItemArtifact", "-", NP, NP),
    r("SecretArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::SecretArtifact", "-", NP, NP),
    r("ZoneArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::ZoneArtifact", "-", NP, NP),
    r("FactionArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::FactionArtifact", "-", NP, NP),
    r("MotifArtifact", Kind::AtomType, "struct ironroot_creation_engine::artifact", AUTH, "live", "ironroot_creation_engine::artifact::MotifArtifact", "-", NP, NP),
    r("InstrumentId", Kind::AtomType, "enum ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::InstrumentId", "-", NP, NP),
    r("ChoiceAction", Kind::AtomType, "enum ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::ChoiceAction", "-", NP, NP),
    r("SceneChoice", Kind::AtomType, "struct ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::SceneChoice", "-", NP, NP),
    r("ChoiceScene", Kind::AtomType, "struct ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::ChoiceScene", "-", NP, NP),
    r("SceneScore", Kind::AtomType, "struct ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::SceneScore", "-", NP, NP),
    r("SceneRuntimeState", Kind::AtomType, "struct ironroot_creation_engine::cyoa", AUTH, "live", "ironroot_creation_engine::cyoa::SceneRuntimeState", "-", NP, NP),
    r("GenerationContext", Kind::AtomType, "struct ironroot_creation_engine::generation", AUTH, "live", "ironroot_creation_engine::generation::GenerationContext", "-", NP, NP),
    r("GenerationWeight", Kind::AtomType, "struct ironroot_creation_engine::generation", AUTH, "live", "ironroot_creation_engine::generation::GenerationWeight", "-", NP, NP),
    r("WeightModifier", Kind::AtomType, "struct ironroot_creation_engine::generation", AUTH, "live", "ironroot_creation_engine::generation::WeightModifier", "-", NP, NP),
    r("WordTable", Kind::AtomType, "struct ironroot_creation_engine::generation", AUTH, "live", "ironroot_creation_engine::generation::WordTable", "-", NP, NP),
    r("GraphPosition", Kind::AtomType, "struct ironroot_creation_engine::graph", AUTH, "live", "ironroot_creation_engine::graph::GraphPosition", "-", NP, NP),
    r("ArtifactBranchKind", Kind::AtomType, "enum ironroot_creation_engine::graph", AUTH, "live", "ironroot_creation_engine::graph::ArtifactBranchKind", "-", NP, NP),
    r("GraphNode", Kind::AtomType, "struct ironroot_creation_engine::graph", AUTH, "live", "ironroot_creation_engine::graph::GraphNode", "-", NP, NP),
    r("GraphEdge", Kind::AtomType, "struct ironroot_creation_engine::graph", AUTH, "live", "ironroot_creation_engine::graph::GraphEdge", "-", NP, NP),
    r("CreationGraph", Kind::AtomType, "struct ironroot_creation_engine::graph", AUTH, "live", "ironroot_creation_engine::graph::CreationGraph", "-", NP, NP),
    r("ArtifactId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::ArtifactId", "-", NP, NP),
    r("LoreFactId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::LoreFactId", "-", NP, NP),
    r("EdgeId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::EdgeId", "-", NP, NP),
    r("BranchId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::BranchId", "-", NP, NP),
    r("SceneId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::SceneId", "-", NP, NP),
    r("ChoiceId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::ChoiceId", "-", NP, NP),
    r("SecretId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::SecretId", "-", NP, NP),
    r("ItemSetId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::ItemSetId", "-", NP, NP),
    r("MotifId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::MotifId", "-", NP, NP),
    r("FactionId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::FactionId", "-", NP, NP),
    r("ZoneId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::ZoneId", "-", NP, NP),
    r("NpcId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::NpcId", "-", NP, NP),
    r("ItemId", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::ItemId", "-", NP, NP),
    r("Tick", Kind::AtomType, "struct ironroot_creation_engine::ids", AUTH, "live", "ironroot_creation_engine::ids::Tick", "-", NP, NP),
    r("SetBonusKind", Kind::AtomType, "enum ironroot_creation_engine::item_sets", AUTH, "live", "ironroot_creation_engine::item_sets::SetBonusKind", "-", NP, NP),
    r("SetBonus", Kind::AtomType, "struct ironroot_creation_engine::item_sets", AUTH, "live", "ironroot_creation_engine::item_sets::SetBonus", "-", NP, NP),
    r("ItemSet", Kind::AtomType, "struct ironroot_creation_engine::item_sets", AUTH, "live", "ironroot_creation_engine::item_sets::ItemSet", "-", NP, NP),
    r("FactionItem", Kind::AtomType, "struct ironroot_creation_engine::item_sets", AUTH, "live", "ironroot_creation_engine::item_sets::FactionItem", "-", NP, NP),
    r("LoreFactKind", Kind::AtomType, "enum ironroot_creation_engine::ledger", AUTH, "live", "ironroot_creation_engine::ledger::LoreFactKind", "-", NP, NP),
    r("LoreFact", Kind::AtomType, "struct ironroot_creation_engine::ledger", AUTH, "live", "ironroot_creation_engine::ledger::LoreFact", "-", NP, NP),
    r("LedgerEventKind", Kind::AtomType, "enum ironroot_creation_engine::ledger", AUTH, "live", "ironroot_creation_engine::ledger::LedgerEventKind", "-", NP, NP),
    r("Ledger", Kind::AtomType, "struct ironroot_creation_engine::ledger", AUTH, "live", "ironroot_creation_engine::ledger::Ledger", "-", NP, NP),
    r("ResetScope", Kind::AtomType, "enum ironroot_creation_engine::reset", AUTH, "live", "ironroot_creation_engine::reset::ResetScope", "-", NP, NP),
    r("WorldBranch", Kind::AtomType, "struct ironroot_creation_engine::reset", AUTH, "live", "ironroot_creation_engine::reset::WorldBranch", "-", NP, NP),
    r("RunProjection", Kind::AtomType, "struct ironroot_creation_engine::reset", AUTH, "live", "ironroot_creation_engine::reset::RunProjection", "-", NP, NP),
    r("CreationUiMode", Kind::AtomType, "enum ironroot_creation_engine::ui_contract", AUTH, "live", "ironroot_creation_engine::ui_contract::CreationUiMode", "-", NP, NP),
    r("UiAction", Kind::AtomType, "enum ironroot_creation_engine::ui_contract", AUTH, "live", "ironroot_creation_engine::ui_contract::UiAction", "-", NP, NP),
    r("ToyboxCard", Kind::AtomType, "struct ironroot_creation_engine::ui_contract", AUTH, "live", "ironroot_creation_engine::ui_contract::ToyboxCard", "-", NP, NP),
    r("ExplainWhyLine", Kind::AtomType, "struct ironroot_creation_engine::ui_contract", AUTH, "live", "ironroot_creation_engine::ui_contract::ExplainWhyLine", "-", NP, NP),
    r("WhatBreaksReport", Kind::AtomType, "struct ironroot_creation_engine::ui_contract", AUTH, "live", "ironroot_creation_engine::ui_contract::WhatBreaksReport", "-", NP, NP),
    r("LoreValidationError", Kind::AtomType, "enum ironroot_creation_engine::validation", AUTH, "live", "ironroot_creation_engine::validation::LoreValidationError", "-", NP, NP),
    r("ValidationSeverity", Kind::AtomType, "enum ironroot_creation_engine::validation", AUTH, "live", "ironroot_creation_engine::validation::ValidationSeverity", "-", NP, NP),
    r("ValidationIssue", Kind::AtomType, "struct ironroot_creation_engine::validation", AUTH, "live", "ironroot_creation_engine::validation::ValidationIssue", "-", NP, NP),
    r("ValidationReport", Kind::AtomType, "struct ironroot_creation_engine::validation", AUTH, "live", "ironroot_creation_engine::validation::ValidationReport", "-", NP, NP),
    r("CreationEvent", Kind::AtomType, "enum ironroot_signal::events", SIG, "live", "ironroot_signal::events::CreationEvent", "-", NP, NP),
    r("AmbientParameter", Kind::AtomType, "enum ironroot_signal::events", SIG, "live", "ironroot_signal::events::AmbientParameter", "-", NP, NP),
    r("StampInfluenceKind", Kind::AtomType, "enum ironroot_signal::events", SIG, "live", "ironroot_signal::events::StampInfluenceKind", "-", NP, NP),
    r("WorldFluxEvent", Kind::AtomType, "enum ironroot_signal::events", SIG, "live", "ironroot_signal::events::WorldFluxEvent", "-", NP, NP),
    r("Tick", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::Tick", "-", NP, NP),
    r("SignalSourceId", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::SignalSourceId", "-", NP, NP),
    r("AssetId", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::AssetId", "-", NP, NP),
    r("ToolId", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::ToolId", "-", NP, NP),
    r("ZoneId", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::ZoneId", "-", NP, NP),
    r("Vec3i", Kind::AtomType, "struct ironroot_signal::ids", SIG, "live", "ironroot_signal::ids::Vec3i", "-", NP, NP),
    r("SignalHealth", Kind::AtomType, "enum ironroot_signal::proxy", SIG, "live", "ironroot_signal::proxy::SignalHealth", "-", NP, NP),
    r("RawSignalFrame", Kind::AtomType, "struct ironroot_signal::proxy", SIG, "live", "ironroot_signal::proxy::RawSignalFrame", "-", NP, NP),
    r("FilteredSignalFrame", Kind::AtomType, "struct ironroot_signal::proxy", SIG, "live", "ironroot_signal::proxy::FilteredSignalFrame", "-", NP, NP),
    r("CreationStampHash", Kind::AtomType, "struct ironroot_signal::stamp", SIG, "live", "ironroot_signal::stamp::CreationStampHash", "-", NP, NP),
    r("StampGameplayEffect", Kind::AtomType, "enum ironroot_signal::stamp", SIG, "live", "ironroot_signal::stamp::StampGameplayEffect", "-", NP, NP),
    r("MeaningBudgetRequest", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::MeaningBudgetRequest", "-", NP, NP),
    r("MeaningBudgetReport", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::MeaningBudgetReport", "-", NP, NP),
    r("LogicCategory", Kind::AtomType, "enum forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::LogicCategory", "-", NP, NP),
    r("LoreCategory", Kind::AtomType, "enum forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::LoreCategory", "-", NP, NP),
    r("RequiredAssetKind", Kind::AtomType, "enum forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::RequiredAssetKind", "-", NP, NP),
    r("ExecutableLogicFinding", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::ExecutableLogicFinding", "-", NP, NP),
    r("LoreLogicFinding", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::LoreLogicFinding", "-", NP, NP),
    r("RequiredAssetFinding", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::RequiredAssetFinding", "-", NP, NP),
    r("HourBudgetEstimate", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::HourBudgetEstimate", "-", NP, NP),
    r("MeaningHealthMetrics", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::MeaningHealthMetrics", "-", NP, NP),
    r("GapSeverity", Kind::AtomType, "enum forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::GapSeverity", "-", NP, NP),
    r("ProductionGap", Kind::AtomType, "struct forge_meaning_budget::meaning_budget", SEM, "live", "forge_meaning_budget::meaning_budget::ProductionGap", "-", NP, NP),
];

/// Render the index as a deterministic, sorted, tab-separated artifact — one row
/// per element. Pure: identical input → identical bytes (the anti-staleness proof,
/// mirroring `capability_floor::render_floor`). This is the format the
/// (to-be-ported) `capability_query` parser reads.
pub fn render_index() -> String {
    let mut rows: Vec<&CapabilityRow> = CAPABILITIES.iter().collect();
    rows.sort_by(|a, b| {
        (a.kind.as_str(), a.element, a.oracle).cmp(&(b.kind.as_str(), b.element, b.oracle))
    });
    let mut s = String::new();
    for row in rows {
        s.push_str(row.element);
        s.push('\t');
        s.push_str(row.kind.as_str());
        s.push('\t');
        s.push_str(row.capability);
        s.push('\t');
        s.push_str(row.goal);
        s.push('\t');
        s.push_str(row.status);
        s.push('\t');
        s.push_str(row.oracle);
        s.push('\t');
        s.push_str(row.ast_arm);
        s.push('\t');
        s.push_str(row.lsp_mirror);
        s.push('\t');
        s.push_str(row.highlight);
        s.push('\n');
    }
    s
}

/// The TERSE MACHINE SPINE projection (Sean's format law: no prose, ≤60B/line-friendly).
/// One row per capability: `element ⇥ kind ⇥ domain ⇥ rung(0–4) ⇥ oracle`. Deterministic,
/// same sort as [`render_index`]. This is what feeds `river.idx` (belt = spine); the prose
/// `capability` sentence is intentionally DROPPED — symbol + oracle + domain + rung carry it.
pub fn render_spine() -> String {
    let mut rows: Vec<&CapabilityRow> = CAPABILITIES.iter().collect();
    rows.sort_by(|a, b| {
        (a.kind.as_str(), a.element, a.oracle).cmp(&(b.kind.as_str(), b.element, b.oracle))
    });
    let mut s = String::new();
    for row in rows {
        s.push_str(row.element);
        s.push('\t');
        s.push_str(row.kind.as_str());
        s.push('\t');
        s.push_str(row.goal); // domain (foundation.*, poc.*)
        s.push('\t');
        s.push((b'0' + row.rung) as char); // rung 0–4, single machine digit
        s.push('\t');
        s.push_str(row.oracle);
        s.push('\n');
    }
    s
}
