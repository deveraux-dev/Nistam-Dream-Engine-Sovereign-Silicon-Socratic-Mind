//! # emit.rs — surface tree → `WidgetSpec { .. }` literal Rust source
//!
//! The AOT codegen arm `forge-vix/build.rs` calls (ADR-0031 runtime_parse =
//! forbidden). Emits the exact literal shape the old inlined emitter wrote;
//! variant paths come from the tables, so a new kind/policy is one row edit.
//! Sizing defaults and the slot_list StackV default mirror `parse.rs::to_spec`.

use crate::surface::SurfaceTree;
use crate::tables::{LayoutPolicy, SlotKind};
use std::fmt::Write as _;

/// Emit one `WidgetSpec { .. }` literal (no trailing newline; caller closes the fn).
pub fn emit_spec(node: &SurfaceTree, indent: usize, out: &mut String) {
    let s = &node.slot;
    let pad = "    ".repeat(indent);
    let pad1 = "    ".repeat(indent + 1);

    // Sizing: mirrors to_spec() in forge-vix/src/parse.rs.
    let sizing = match s.size_main {
        Some(n) => format!("Sizing::FixedMain({n}_i64)"),
        None => match s.kind {
            SlotKind::Region | SlotKind::JournalText => "Sizing::Fill".into(),
            SlotKind::SlotList | SlotKind::Drawer => "Sizing::Hug".into(),
            _ => "Sizing::Intrinsic".into(),
        },
    };

    // slot_list default layout = StackV (mirrors to_spec).
    let layout_expr = match (s.kind, s.layout) {
        (SlotKind::SlotList, None) => {
            format!("Some({})", LayoutPolicy::StackV.variant_path())
        }
        (_, Some(l)) => format!("Some({})", l.variant_path()),
        (_, None) => "None".into(),
    };

    let widget_name_expr = match &s.widget_name {
        Some(n) => format!("Some(WidgetName({n:?}.into()))"),
        None => "None".into(),
    };

    let material_expr = match &s.material {
        Some(m) => format!("Some({m:?}.into())"),
        None => "None".into(),
    };

    let grid_cols_expr = match s.grid_cols {
        Some(c) => format!("Some({c}_u16)"),
        None => "None".into(),
    };

    let slot_list_max_expr = match s.slot_list_max {
        Some(m) => format!("Some({m}_u16)"),
        None => "None".into(),
    };

    writeln!(out, "{pad}WidgetSpec {{").unwrap();
    writeln!(out, "{pad1}stable_key: {:?}.into(),", s.name).unwrap();
    writeln!(out, "{pad1}kind: {},", s.kind.variant_path()).unwrap();
    writeln!(out, "{pad1}widget_name: {widget_name_expr},").unwrap();
    writeln!(out, "{pad1}layout: {layout_expr},").unwrap();
    writeln!(out, "{pad1}sizing: {sizing},").unwrap();
    writeln!(out, "{pad1}grid_cols: {grid_cols_expr},").unwrap();
    writeln!(out, "{pad1}slot_list_max: {slot_list_max_expr},").unwrap();

    if node.children.is_empty() {
        writeln!(out, "{pad1}children: vec![],").unwrap();
    } else {
        writeln!(out, "{pad1}children: vec![").unwrap();
        for child in &node.children {
            emit_spec(child, indent + 2, out);
            writeln!(out, ",").unwrap();
        }
        writeln!(out, "{pad1}],").unwrap();
    }

    writeln!(out, "{pad1}hover_reveal: {},", s.hover_reveal).unwrap();
    writeln!(out, "{pad1}long_press_drawer: {},", s.long_press_drawer).unwrap();
    writeln!(out, "{pad1}collapsible: {},", s.collapsible).unwrap();
    writeln!(out, "{pad1}audio_reactive: {},", s.audio_reactive).unwrap();
    // AOT TWIN, STILL DROPPING (08-02): the runtime parser carries the brush
    // figure (parse.rs::to_spec -> WidgetSpec::brush); the surface parser does
    // not read shape/radius/phase/stride yet. Emitting None is the honest state:
    // an AOT-baked panel paints no figure until surface.rs learns those attrs.
    writeln!(out, "{pad1}brush: None,").unwrap();
    writeln!(out, "{pad1}material: {material_expr},").unwrap();
    let opt_i64 = |v: Option<i64>| match v {
        Some(n) => format!("Some({n}_i64)"),
        None => "None".to_string(),
    };
    let text_expr = match &s.text {
        Some(t) => format!("Some({:?}.to_string())", t),
        None => "None".into(),
    };
    writeln!(out, "{pad1}text: {text_expr},").unwrap();
    writeln!(out, "{pad1}gap: {},", opt_i64(s.gap)).unwrap();
    writeln!(out, "{pad1}padding: {},", opt_i64(s.padding)).unwrap();
    let chrome_color_expr = match &s.chrome_color {
        Some(c) => format!("Some({:?}.to_string())", c),
        None => "None".into(),
    };
    writeln!(out, "{pad1}chrome_color: {chrome_color_expr},").unwrap();
    writeln!(out, "{pad1}border_radius: {},", opt_i64(s.border_radius)).unwrap();
    let alpha_expr = match s.alpha_pmy {
        Some(a) => format!("Some({a})"),
        None => "None".to_string(),
    };
    writeln!(out, "{pad1}alpha_pmy: {alpha_expr},").unwrap();
    let opt_owned = |v: &Option<String>| match v {
        Some(s) => format!("Some({s:?}.to_string())"),
        None => "None".to_string(),
    };
    writeln!(out, "{pad1}font: {},", opt_owned(&s.font)).unwrap();
    writeln!(out, "{pad1}semantic: {},", opt_owned(&s.semantic)).unwrap();
    let opt_bare_i64 = |v: Option<i64>| match v {
        Some(n) => format!("Some({n})"),
        None => "None".to_string(),
    };
    writeln!(out, "{pad1}min_size: {},", opt_bare_i64(s.min_size)).unwrap();
    writeln!(out, "{pad1}max_size: {},", opt_bare_i64(s.max_size)).unwrap();
    writeln!(out, "{pad1}margin: {},", opt_bare_i64(s.margin)).unwrap();
    let opt_variant = |v: Option<&'static str>| match v {
        Some(p) => format!("Some({p})"),
        None => "None".to_string(),
    };
    writeln!(out, "{pad1}justify: {},", opt_variant(s.justify.map(|j| j.variant_path()))).unwrap();
    writeln!(out, "{pad1}align: {},", opt_variant(s.align.map(|a| a.variant_path()))).unwrap();
    // Every field WidgetSpec grows after this line costs the emitter nothing: the
    // literal closes on the zero value instead of naming it (massloop law 4).
    // 08-04 receipt: adding min_size/max_size without this produced 1614 E0063s.
    writeln!(out, "{pad1}..WidgetSpec::default()").unwrap();
    write!(out, "{pad}}}").unwrap();
}

/// The whole builder-fn body `build.rs` writes for one panel.
pub fn emit_panel_body(tree: &SurfaceTree) -> String {
    let mut out = String::new();
    emit_spec(tree, 1, &mut out);
    out.push('\n');
    out
}
