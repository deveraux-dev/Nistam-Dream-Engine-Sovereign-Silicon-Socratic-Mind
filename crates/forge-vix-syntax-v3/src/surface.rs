//! # surface.rs — the ONE `.kit.vixi` slot-line surface parser (AOT semantics)
//!
//! Lifted verbatim from the parser `forge-vix/build.rs` inlined (a hand-copy of
//! parse.rs that drifted twice: hex_grid 07-12, quoted-text 07-29). Base tree
//! only — `variant` blocks are skipped, exactly as the AOT lane always has.
//! forge-vix::parse remains the SEMANTIC parser (BakedAttrs, gates, variants);
//! this module owns the layout-affecting attribute surface both lanes share.

use crate::tables::{Align, Justify, LayoutPolicy, SlotKind};
use std::collections::HashMap;

/// One parsed `slot` line — layout-affecting attributes, enum-typed.
pub struct SurfaceSlot {
    /// Dotted path from the root (e.g. `root.control_rail.import_box`).
    pub name: String,
    /// The `kind=` closed vocabulary — what this slot renders as.
    pub kind: SlotKind,
    /// The `layout=` policy, for `Region`-kind slots.
    pub layout: Option<LayoutPolicy>,
    /// `name=`/`of=` — the widget-inventory entry a `Widget`-kind slot binds to.
    pub widget_name: Option<String>,
    /// `cols=` — fixed column count for `Grid` layout.
    pub grid_cols: Option<u16>,
    /// `max=` — bound on a `SlotList`'s dynamic child count.
    pub slot_list_max: Option<u16>,
    /// `size=mu(N)` — fixed extent on the region's main axis.
    pub size_main: Option<i64>,
    /// `gap=mu(N)` — spacing between children on the main axis.
    pub gap: Option<i64>,
    /// `padding=mu(N)` — inset applied on all sides of the region.
    pub padding: Option<i64>,
    /// MAIN-axis clamp — mirrors `layout::clamp_main`.
    pub min_size: Option<i64>,
    /// MAIN-axis clamp — mirrors `layout::clamp_main`.
    pub max_size: Option<i64>,
    /// `margin=mu(N)` — outer gap consuming the parent's flow.
    pub margin: Option<i64>,
    /// `justify=` — main-axis child distribution.
    pub justify: Option<Justify>,
    /// `align=` — cross-axis child sizing/placement.
    pub align: Option<Align>,
    /// `color=palette.<name>` — chrome palette slot name (never a hex literal).
    pub chrome_color: Option<String>,
    /// `border_radius=mu(N)` — chrome corner rounding.
    pub border_radius: Option<i64>,
    /// `alpha=permyriad(N)` — opacity, clamped to `0..=10_000`.
    pub alpha_pmy: Option<u16>,
    /// `font=` — named font-face reference.
    pub font: Option<String>,
    /// `semantic=` — accessibility/automation role hint.
    pub semantic: Option<String>,
    /// `hover_reveal=true` — chrome/content only paints on hover.
    pub hover_reveal: bool,
    /// `long_press_drawer=true` — this slot opens as a long-press drawer.
    pub long_press_drawer: bool,
    /// `collapsible=true` — this region can collapse to zero cross-extent.
    pub collapsible: bool,
    /// `audio_reactive=true` — this slot's paint responds to the audio bus.
    pub audio_reactive: bool,
    /// `material=` — named paint material (Brush-kind slots).
    pub material: Option<String>,
    /// Authored `text="…"` — the kit's own words, baked into the AOT panel.
    pub text: Option<String>,
}

/// The dotted-name slot tree.
pub struct SurfaceTree {
    /// This node's own parsed slot attributes.
    pub slot: SurfaceSlot,
    /// Direct children, in source order.
    pub children: Vec<SurfaceTree>,
}

/// `mu(N)` (or bare N) → i64 MilliUnit.
pub fn parse_mu(v: &str) -> Option<i64> {
    let inner = v.strip_prefix("mu(").and_then(|s| s.strip_suffix(')')).unwrap_or(v);
    inner.trim().parse::<i64>().ok().map(|px| px * 1_000)
}

/// Split a `slot` line on whitespace, except inside a double-quoted value.
/// 2026-07-29: `split_whitespace` tore `text="AST Viewer"` down to `AST` in the
/// AOT panel, so every shipped multi-word label was truncated while the runtime
/// parser read it whole. `unlower_roundtrip.rs::runtime_parse_agrees_with_aot`
/// holds the two lanes together; since the leaf-crate fold there is only ONE.
pub fn split_attr_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                cur.push(ch);
            }
            c if c.is_whitespace() && !in_quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Parse one `slot <dotted.name> key=value…` line into a [`SurfaceSlot`].
pub fn parse_slot_line(line: &str, line_no: usize) -> Result<SurfaceSlot, String> {
    let mut it = split_attr_tokens(line).into_iter();
    let _kw = it.next(); // "slot"
    let name = it
        .next()
        .ok_or_else(|| format!("line {line_no}: slot missing name"))?
        .to_string();

    let depth = name.bytes().filter(|&b| b == b'.').count();
    if depth > 64 {
        return Err(format!("line {line_no}: slot '{name}' nesting depth > 64"));
    }

    let mut kind: Option<SlotKind> = None;
    let mut layout: Option<LayoutPolicy> = None;
    let mut widget_name: Option<String> = None;
    let mut grid_cols: Option<u16> = None;
    let mut slot_list_max: Option<u16> = None;
    let mut size_main: Option<i64> = None;
    let mut gap: Option<i64> = None;
    let mut padding: Option<i64> = None;
    let mut min_size: Option<i64> = None;
    let mut max_size: Option<i64> = None;
    let mut margin: Option<i64> = None;
    let mut justify: Option<Justify> = None;
    let mut align: Option<Align> = None;
    let mut chrome_color: Option<String> = None;
    let mut border_radius: Option<i64> = None;
    let mut alpha_pmy: Option<u16> = None;
    let mut font: Option<String> = None;
    let mut semantic: Option<String> = None;
    let mut hover_reveal = false;
    let mut long_press_drawer = false;
    let mut collapsible = false;
    let mut audio_reactive = false;
    let mut material: Option<String> = None;
    let mut text: Option<String> = None;

    for tok in it {
        let (k, v) = match tok.split_once('=') {
            Some(kv) => kv,
            None => continue, // non-key=value token: skip (visible_if, role=, etc.)
        };
        match k {
            "kind" => {
                kind = Some(
                    SlotKind::from_name(v)
                        .ok_or_else(|| format!("line {line_no}: unknown kind '{v}'"))?,
                );
            }
            "layout" => {
                layout = Some(
                    LayoutPolicy::from_name(v)
                        .ok_or_else(|| format!("line {line_no}: unknown layout '{v}'"))?,
                );
            }
            "name" | "of" => {
                let n = v.strip_prefix("widget.").unwrap_or(v);
                widget_name = Some(n.to_string());
            }
            // The kit's own words, baked in (quotes stripped here).
            "text" => {
                text = Some(v.trim_matches('"').to_string());
            }
            "cols" => {
                grid_cols =
                    Some(v.parse().map_err(|_| format!("line {line_no}: bad cols '{v}'"))?);
            }
            "max" => {
                slot_list_max =
                    Some(v.parse().map_err(|_| format!("line {line_no}: bad max '{v}'"))?);
            }
            "size" => {
                size_main =
                    Some(parse_mu(v).ok_or_else(|| format!("line {line_no}: bad size '{v}'"))?);
            }
            "gap" => {
                gap = Some(parse_mu(v).ok_or_else(|| format!("line {line_no}: bad gap '{v}'"))?);
            }
            "padding" => {
                padding = Some(
                    parse_mu(v).ok_or_else(|| format!("line {line_no}: bad padding '{v}'"))?,
                );
            }
            "min_size" => {
                min_size = Some(
                    parse_mu(v).ok_or_else(|| format!("line {line_no}: bad min_size '{v}'"))?,
                );
            }
            "max_size" => {
                max_size = Some(
                    parse_mu(v).ok_or_else(|| format!("line {line_no}: bad max_size '{v}'"))?,
                );
            }
            "margin" => {
                margin = Some(
                    parse_mu(v).ok_or_else(|| format!("line {line_no}: bad margin '{v}'"))?,
                );
            }
            "justify" => {
                justify = Some(Justify::from_name(v).ok_or_else(|| {
                    format!(
                        "line {line_no}: bad justify '{v}' — expected {}",
                        Justify::NAMES.join("|")
                    )
                })?);
            }
            "align" => {
                align = Some(Align::from_name(v).ok_or_else(|| {
                    format!(
                        "line {line_no}: bad align '{v}' — expected {}",
                        Align::NAMES.join("|")
                    )
                })?);
            }
            // Authored chrome (2026-07-29) — the palette slot NAME travels,
            // never a hex literal; the profile sheet still owns colour.
            "color" => {
                chrome_color = Some(v.strip_prefix("palette.").unwrap_or(v).to_string());
            }
            "border_radius" => {
                border_radius = Some(
                    parse_mu(v)
                        .ok_or_else(|| format!("line {line_no}: bad border_radius '{v}'"))?,
                );
            }
            // Integer lattice: `gate float_in_ir = forbidden`. Clamp, never wrap.
            "alpha" => {
                let raw =
                    v.strip_prefix("permyriad(").and_then(|r| r.strip_suffix(')')).unwrap_or(v);
                alpha_pmy = Some(raw.trim().parse::<u32>().map(|n| n.min(10_000) as u16).map_err(
                    |_| format!("line {line_no}: bad alpha '{v}' — expected permyriad(0..10000)"),
                )?);
            }
            "font" => font = Some(v.to_string()),
            "semantic" => semantic = Some(v.to_string()),
            "hover_reveal" => hover_reveal = v == "true",
            "long_press_drawer" => long_press_drawer = v == "true",
            "collapsible" => collapsible = v == "true",
            "audio_reactive" => audio_reactive = v == "true",
            "material" => material = Some(v.to_string()),
            // Authoring-only attrs (ramp=, bind=, curve=, role=, etc.) bake into
            // BakedSlot in forge-vix::parse; they do not affect WidgetSpec layout.
            _ => {}
        }
    }

    let kind =
        kind.ok_or_else(|| format!("line {line_no}: slot '{name}' missing kind="))?;
    Ok(SurfaceSlot {
        name,
        kind,
        layout,
        widget_name,
        grid_cols,
        slot_list_max,
        size_main,
        gap,
        padding,
        min_size,
        max_size,
        margin,
        justify,
        align,
        hover_reveal,
        long_press_drawer,
        collapsible,
        audio_reactive,
        material,
        text,
        chrome_color,
        border_radius,
        alpha_pmy,
        font,
        semantic,
    })
}

/// Parse a whole `.kit.vixi` source to its base slot tree (variants skipped —
/// only the base tree bakes into the AOT builder fn; metadata/gate lines are
/// ignored for layout).
pub fn parse_kit_surface(src: &str) -> Result<SurfaceTree, String> {
    let mut slots: Vec<SurfaceSlot> = Vec::new();
    let mut in_variant = false;
    let mut has_automaton = false;

    for (i, raw) in src.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let first = line.split_whitespace().next().unwrap_or("");
        match first {
            "variant" => in_variant = true,
            "slot" if !in_variant => slots.push(parse_slot_line(line, line_no)?),
            "slot" => {} // variant slot — skip
            // Automaton dialect opener (lowered by forge-vix-v3 parse.rs since
            // 2026-08-24); this surface lane stays layout-only, but a slot-less
            // automaton kit is a legal document, not a missing-root defect.
            "state" => has_automaton = true,
            _ => {} // metadata / gate / automaton rows
        }
    }

    if slots.is_empty() {
        if has_automaton {
            // Mirror the semantic lane's relax: synthesize the root through the
            // REAL slot-line parser, never a second construction path.
            slots.push(parse_slot_line("slot root kind=region layout=stack_v", 0)?);
        } else {
            return Err("no base slots declared".to_string());
        }
    }
    build_tree(slots)
}

fn build_tree(slots: Vec<SurfaceSlot>) -> Result<SurfaceTree, String> {
    let index: HashMap<String, usize> =
        slots.iter().enumerate().map(|(i, s)| (s.name.clone(), i)).collect();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); slots.len()];
    let mut root_idx: Option<usize> = None;

    for (i, s) in slots.iter().enumerate() {
        match s.name.rfind('.').map(|p| &s.name[..p]) {
            None => {
                if let Some(prev) = root_idx {
                    return Err(format!(
                        "multiple root slots ('{}' and '{}')",
                        slots[prev].name, s.name
                    ));
                }
                root_idx = Some(i);
            }
            Some(parent) => {
                let pi = *index.get(parent).ok_or_else(|| {
                    format!("orphan slot '{}': parent '{parent}' not declared", s.name)
                })?;
                children[pi].push(i);
            }
        }
    }

    let root_idx = root_idx.ok_or("no root slot")?;
    Ok(to_tree(root_idx, &mut slots.into_iter().map(Some).collect::<Vec<_>>(), &children))
}

fn to_tree(i: usize, slots: &mut Vec<Option<SurfaceSlot>>, children: &[Vec<usize>]) -> SurfaceTree {
    let kids = children[i].iter().map(|&c| to_tree(c, slots, children)).collect();
    SurfaceTree {
        slot: slots[i].take().expect("slot consumed once"),
        children: kids,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_kit_surface;

    /// The real authored automaton kit is slot-less and LEGAL (it lowers in
    /// forge-vix-v3 since 2026-08-24) — this surface lane must synthesize a
    /// root and gate Allow, not deny "no base slots declared".
    #[test]
    fn slotless_automaton_kit_passes_the_surface_gate() {
        const STARMAP: &str =
            include_str!("../../forge-envelope/surfaceledger/astrological_starmap.kit.vixi");
        let tree = parse_kit_surface(STARMAP).expect("automaton kit must parse at the surface");
        assert_eq!(tree.slot.name, "root", "synthesized root must come from the real slot parser");
        match crate::gate::gate_surface_tree(&tree) {
            crate::GateDecision::Allow => {}
            crate::GateDecision::Deny { reason } => panic!("automaton kit must gate Allow, got: {reason}"),
        }
    }

    #[test]
    fn slotless_doc_without_automaton_still_refuses() {
        match parse_kit_surface("#vixi:kit v1\nsurface: empty\n") {
            Ok(_) => panic!("a doc with neither slots nor states must refuse"),
            Err(err) => assert!(err.contains("no base slots"), "{err}"),
        }
    }
}
