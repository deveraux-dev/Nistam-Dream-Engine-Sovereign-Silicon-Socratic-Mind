//! Custom VixiScript grammar SoT inside the language server to avoid legacy forge-vix dependency.

pub type Token = (&'static str, &'static str);

pub const SLOT_KINDS: &[Token] = &[
    ("chrome", "non-interactive surround — borders, plates, frames"),
    ("text", "glyph run; usually bound to a type.ramp + a fg colour token"),
    ("image", "bitmap / atlas region"),
    ("widget", "interactive primitive; requires name=<widget> from the inventory"),
    ("region", "container for child slots; requires layout=<policy>"),
    ("brush", "audio-reactive paint surface (references a Vixi audio-dialect brush)"),
    ("slot_list", "ordered, bounded list of homogeneous children; requires of=widget.<n> max=<N>"),
    ("sigil_corner", "corner sigil / badge — error dot, notification, status pip (Smithy ext)"),
    ("journal_text", "journal / long-form text surface (Smithy ext)"),
    ("drawer", "collapsible drawer container (Smithy ext)"),
];

pub const LAYOUT_POLICIES: &[Token] = &[
    ("stack_v", "vertical stack"),
    ("stack_h", "horizontal stack"),
    ("grid", "R x C grid; pair with cols=<N>"),
    ("overlay", "z-stacked, free position — every child fills the parent rect"),
    ("flow", "wrap to width"),
    ("hex_grid", "hexagonal tessellation; pair with hex_size=mu(N) for cell radius"),
    ("split_view", "two-pane split (alias → stack_h)"),
    ("quad_view", "four-pane 2x2 view (alias → grid)"),
    ("timeline_tracks", "vertical stack of animation tracks (alias → stack_v)"),
    ("deck_mixer_deck", "side-by-side DJ decks (alias → stack_h)"),
    ("dockspace", "dockable shell container (alias → overlay; native docking via .dock descriptor)"),
];

pub const WIDGET_NAMES: &[&str] = &[
    // ── Core interactive widgets ─────────────────────────────────────────────
    "button", "slider", "range_slider", "toggle", "checkbox", "text_field", "dropdown",
    "scrollview", "tab", "modal", "tooltip",
    // ── Display / indicator widgets ──────────────────────────────────────────
    "progress_bar", "bar", "progress", "spacer", "gauge", "gate_indicator", "glow_dot",
    // ── Navigation / collection widgets ─────────────────────────────────────
    "list", "tree", "tree_node", "dial", "icon", "icon_button",
    // ── Game / studio domain widgets ─────────────────────────────────────────
    "dialogue_choice", "quest_entry", "quest_row", "hotbar", "healthbar",
    "minimap", "portrait", "menu_item", "command_row", "log_row",
    "ability_slot", "resource_chip", "document_thumb", "lb_row",
];

pub const SLOT_ATTRS: &[Token] = &[
    ("kind", "REQUIRED — the slot kind (see SLOT_KINDS)"),
    ("layout", "region layout policy (see LAYOUT_POLICIES)"),
    ("name", "widget inventory name (kind=widget)"),
    ("of", "slot_list element type, e.g. of=widget.button"),
    ("cols", "grid column count (kind=region layout=grid)"),
    ("hex_size", "hex cell radius in MilliUnit (kind=region layout=hex_grid)"),
    ("max", "slot_list max length (REQUIRED on slot_list — size-stability)"),
    ("size", "fixed MAIN-axis extent, e.g. size=mu(64) — rail width in stack_h / bar height in stack_v; a sibling Fill takes the rest (→ Sizing::FixedMain)"),
    // ── Flexbox-aligned layout overrides (spec §4 LayoutProperties) ──────────
    ("justify", "main-axis child distribution: start | center | end | space_between | space_around"),
    ("align", "cross-axis child alignment: start | center | end | stretch"),
    ("padding", "inset from region edges applied before child layout, e.g. padding=mu(8)"),
    ("margin", "outer gap around this slot (consumes space from parent flow), e.g. margin=mu(4)"),
    ("gap", "inter-child spacing override (default: density_base token), e.g. gap=mu(12)"),
    ("border_radius", "corner rounding radius in MilliUnit, e.g. border_radius=mu(4)"),
    ("min_size", "minimum MAIN-axis extent; prevents shrinking below this, e.g. min_size=mu(32)"),
    ("max_size", "maximum MAIN-axis extent; caps growth, e.g. max_size=mu(256)"),
    ("bind", "token bound to the slot surface (v1: resolved by TokenCtx)"),
    ("color", "chrome colour token (v1: resolved by TokenCtx)"),
    ("curve", "chrome curvature token"),
    ("thick", "chrome thickness token"),
    ("ramp", "type ramp index, e.g. ramp=type.ramp[1]"),
    ("bus_in", "audio bus feeding a brush slot"),
    ("shape", "brush figure: ring | disc | star | arc | line (kind=brush)"),
    ("radius", "figure radius in PERMYRIAD of the parent's short side, e.g. radius=pmy(5774) — permyriad because a plate circle is a RATIO of its plate, never a pixel count; see forge_tile_crawler::architecture::ratio"),
    ("phase", "rotation source: 0 = fixed (the mater does not turn) | tick = advances on SimTick (the rete does). NEVER a wall clock — G-PLAT-01"),
    ("stride", "sibling angular distribution: weyl = pp_math::formation::thirds_stride_bucket (low-discrepancy, spreads instead of piling at bucket 0) | even = equal arcs"),
    ("source", "engine snapshot field feeding this slot (M3 host bind: parse.rs bakes attrs.source; hosts fill by serde field name — telemetry/eight_angles/constellation)"),
    ("role", "semantic role hint (accepted in v1)"),
    ("priority", "layout/z priority hint (accepted in v1)"),
    ("material", "CE material group (see MATERIAL_GROUPS) -> baked MaterialAtom"),
    ("mass", "Permyriad mass override -> baked physics"),
    ("friction", "Permyriad friction override -> baked physics"),
    ("vibe_scale", "VibeMatrix channel -> scale-radius bind"),
    ("vibe_glow", "VibeMatrix channel -> emissive-glow bind"),
    ("vibe_opacity", "VibeMatrix channel -> opacity bind"),
    ("vibe_offsety", "VibeMatrix channel -> offset-Y bind"),
    ("screen_edge", "ambilight edge sample (see SCREEN_EDGES)"),
    ("blend", "ambilight blend mode (see BLEND_MODES)"),
    ("motion", "spring kinematics preset (see MOTION_PRESETS)"),
    ("attractor", "attractor=pointer — spring follows the cursor"),
    ("on_click", "semantic edict trigger, e.g. on_click=edict:<id>"),
    ("on_key", "key-chord edict trigger, e.g. on_key=ctrl_m:edict:<id> — chord baked AOT to u32 (low16=VK, bit16/17/18=Ctrl/Shift/Alt)"),
    ("hover_reveal", "true|false — reveal-on-hover affordance"),
    ("long_press_drawer", "true|false — long-press opens a drawer"),
    ("collapsible", "true|false — slot can collapse"),
    ("audio_reactive", "true|false — slot reacts to the audio bus"),
    ("optional", "true|false — slot lowers even when its data is absent"),
    ("unit", "measurement unit a scale/ruler counts in, e.g. unit=ticks"),
    ("alpha", "slot opacity on the permyriad lattice, alpha=permyriad(0..10000)"),
    ("font", "type family token for a text slot, e.g. font=mono"),
    ("semantic", "meaning-carrying colour ramp for a meter, e.g. semantic=green_yellow_red"),
    ("primary", "true|false — primary focus region; several may be co-equal"),
    ("fixed_position", "true|false — slot holds its place when siblings reflow"),
    ("searchable", "true|false — region owns a searchable inventory"),
    ("text", "authored literal words for a text/widget slot, e.g. text=\"< PREV\""),
    ("glaze_opacity", "Permyriad glaze overlay density modulator (0..10000)"),
    ("glaze_intensity", "Permyriad baseline glaze intensity bound to combo_heat -> visual.opacity"),
    ("spcc_gate", "RenderGate5D SPCC synchronization status binding"),
    ("cell_5d", "TritCell5D ternary lattice address coordinate"),
];

/// Look up a slot-kind doc by name. `None` if not a known kind.
pub fn kind_doc(name: &str) -> Option<&'static str> {
    SLOT_KINDS.iter().find(|(n, _)| *n == name).map(|(_, d)| *d)
}

/// Look up a layout-policy doc by name. `None` if not a known policy.
pub fn layout_doc(name: &str) -> Option<&'static str> {
    LAYOUT_POLICIES.iter().find(|(n, _)| *n == name).map(|(_, d)| *d)
}

/// True if `src` is a header-less `.vixel` source — no `#vixi:` header and the first
/// meaningful token opens a `.vixel` block.
pub fn is_headerless_vixel(src: &str) -> bool {
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.starts_with('#') {
            return false; // `#vixi:` header present → not header-less
        }
        let first = t
            .split(|c: char| c.is_whitespace() || c == '{')
            .next()
            .unwrap_or("");
        return matches!(
            first,
            "material"
                | "ui"
                | "theme"
                | "rule"
                | "spatial_5d"
                | "spcc_lane"
                | "glaze"
                | "render_gate_5d"
        );
    }
    false
}

/// The closed completion vocabulary for a `#vixi:<dialect>` source.
pub fn dialect_vocab(dialect: &str) -> Vec<Token> {
    match dialect {
        "vixel" => vec![
            ("material", "block opener — defines physical rendering characteristics"),
            ("ui", "block opener — defines custom interface overlay"),
            ("theme", "block opener — custom color scheme rules"),
            ("rule", "block opener — interaction logic"),
            // ── 5D Spatial & SPCC Awareness (RenderGate5D) ───────────────────
            ("spatial_5d", "5D spatial coordinate manifold block (X, Y, Z, T, S)"),
            ("render_gate_5d", "RenderGate5D telemetry lane structure embedding SPCC & 5D lattice"),
            ("trit_cell_5d", "1-byte balanced ternary lattice cell address ([-1, 0, +1] per axis)"),
            ("validity_mask", "1-byte bitmask of active valid axes across the 5D spatial manifold"),
            ("cell_ordinal", "2-byte intra-cell coordinate identifier"),
            ("landauer_margin_pmy", "Landauer thermal/erasure budget margin in permyriad"),
            ("erased_drive_pmy", "SPCC entropy erased drive energy in permyriad"),
            ("mass_in_pmy", "SPCC ingress mass coupling in permyriad"),
            ("mass_out_pmy", "SPCC egress mass coupling in permyriad"),
            ("interference_gain_pmy", "SPCC phase interference gain in permyriad"),
            ("ghostmoon_intersects", "5D hyperbox collision / intersection boolean state"),
            ("spcc_sync_status", "Soliton-Phase Context Collapse synchronization telemetry state byte"),
            ("spcc_lane", "SPCC context collapse channel definition"),
            // ── 8x8 Dual-Array Glaze & Dither ─────────────────────────────────
            ("glaze", "dual-array 8x8 glaze overlay block with spatial opacity variance"),
            ("on_glaze", "composite dither evaluator combining spatial glaze modulation and Bayer threshold"),
            ("bayer8", "8x8 Bayer ordered dithering threshold matrix (values 0..63)"),
            ("glaze_opacity_lut", "8x8 lookup table representing hand-applied glaze density modulation"),
            ("glaze_intensity_pmy", "glaze overlay baseline intensity in permyriad (0..10000)"),
            ("dither_threshold", "ordered dither threshold level"),
        ],
        "shaderbind" => vec![
            ("uniforms", "VibeUniforms binding definition"),
            ("sampler", "texture sampler binding definition"),
        ],
        _ => Vec::new(),
    }
}

/// LSP hover for a non-kit dialect.
pub fn dialect_hover(dialect: &str, word: &str) -> Option<&'static str> {
    dialect_vocab(dialect)
        .into_iter()
        .find(|(n, _)| *n == word)
        .map(|(_, d)| d)
}
