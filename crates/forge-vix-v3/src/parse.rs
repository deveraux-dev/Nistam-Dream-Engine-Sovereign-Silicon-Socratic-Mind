//! # parse.rs — `.kit.vixi` UI-dialect parser → `WidgetSpec` tree (task 3b)
//!
//! Grammar: `docs/forge-design/template_grammar.md`. Hand-rolled, never panics
//! (returns `ParseError` with a line number).
//!
//! **v1 scope:** `#vixi:kit vN` header, `key: value` metadata, `slot` lines with
//! **dotted-name nesting** (`root.panel.apply` is a child of `root.panel`),
//! `gate` lines, `# comments`.
//! **Deferred (explicit errors):** `variant` blocks; reserved
//! `condition`/`transition`/`import`; flat-sibling / `dockspace` surfaces —
//! those route to the egui-bridge per `ffi-ui-assimilator`, not this parser.

use std::collections::HashMap;

use super::baked::{self, BakedAttrs, BakedSlot, VibeBind, VibeTarget};
use super::ir::{LayoutPolicy, SlotKind, WidgetName};
use super::layout::{Align, Justify, Sizing, WidgetSpec};

/// A `.kit.vixi` parse failure — never a panic, always a line + message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based source line the error occurred on.
    pub line: usize,
    /// The full diagnostic text.
    pub message: String,
}

impl ParseError {
    pub(crate) fn at(line: usize, msg: impl Into<String>) -> Self {
        Self { line, message: msg.into() }
    }
}

/// Surface-level z-capability, declared in the kit header (`z: front` /
/// `z: keep`) — the first cremantic: the surface STATES its projection
/// capability; consumers read the declaration instead of hand-threading a
/// mode param (maps to `canvas_projection_pass` `projection_mode` 0/1).
/// Slot-level z-VALUE is the separate `ᐍ=N` attribute (ADR-0006 D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZMode {
    /// 2D front plane — vertex z collapsed (`projection_mode = 0`).
    Front,
    /// Depth kept — 3D perspective (`projection_mode = 1`).
    Keep,
}

impl ZMode {
    /// The SOLE lowering of a declared z-capability to the `forge-gpu`
    /// `canvas_projection_pass` uniform. Hosts must not re-derive the 0/1.
    pub const fn projection_mode(self) -> u32 {
        match self {
            ZMode::Front => 0,
            ZMode::Keep => 1,
        }
    }
}

/// Cremantic plane-claim — the quinary ladder (pinned 2026-07-09). Slot-level
/// bare-token backtick sigil: bare = Unclaimed, digit 0..=4 = P0..P4. Variant
/// names are PLACEHOLDERS pending Sean's nêhiyawêwin research — indices are
/// canon; the rename is one mechanical pass. Closed set, typos LOUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneClaim {
    /// Bare sigil — explicitly unclaimed: lowers like absent, visible to census.
    Unclaimed,
    /// Priority 0 — the quinary ladder's floor.
    P0,
    /// Priority 1.
    P1,
    /// Priority 2.
    P2,
    /// Priority 3.
    P3,
    /// Priority 4 — the quinary ladder's ceiling.
    P4,
}

impl PlaneClaim {
    /// Sigil body (text after the backtick) → claim. Closed set, typos LOUD.
    fn from_sigil_body(body: &str, line_no: usize) -> Result<Self, ParseError> {
        Ok(match body {
            "" => PlaneClaim::Unclaimed,
            "0" => PlaneClaim::P0,
            "1" => PlaneClaim::P1,
            "2" => PlaneClaim::P2,
            "3" => PlaneClaim::P3,
            "4" => PlaneClaim::P4,
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("plane sigil `{other} outside the closed set (bare `, `0..`4)"),
                ))
            }
        })
    }
}

/// A parsed `.kit.vixi` document. `root` is the base tree (or base + active
/// variant via [`parse_kit_variant`]); `variants` lists declared variant names.
#[derive(Debug, Clone, PartialEq)]
pub struct KitDoc {
    /// The `#vixi:kit vN` header version.
    pub dialect_version: u32,
    /// The `surface:` header value — this kit's declared identity.
    pub surface: Option<String>,
    /// Declared visual profile (`profile:` header key) — the authored
    /// `themes/<name>.profile.sheet.vixi` this surface wears. `None` = wear the
    /// live/comfy default. Accepted-by-ignore until 2026-07-28; now consumed by
    /// `loader::load_kit_live` (unported — no `loader` module exists in this
    /// crate yet) via `live::ctx_for`.
    pub profile: Option<String>,
    /// Declared z-capability (`z:` header key). None = undeclared (consumer default).
    pub z_mode: Option<ZMode>,
    /// The base slot tree.
    pub root: WidgetSpec,
    /// `gate <name> = <value>` authoring constraints.
    pub gates: Vec<(String, String)>,
    /// `rule <name> = <value>` gate-family authoring constraints (semantic-UI rules,
    /// e.g. `colour_alone = forbidden_for_meaning`). Golden-corpus intake 2026-07-23.
    pub rules: Vec<(String, String)>,
    /// `state_mutation <name> = <value>` gate-family authoring constraints
    /// (e.g. `authoritative = forbidden`). Golden-corpus intake 2026-07-23.
    pub state_mutations: Vec<(String, String)>,
    /// Declared `variant` block names.
    pub variants: Vec<String>,
    /// Prebaked artist attributes, keyed by slot `stable_key` (only slots that
    /// carried a keyword appear). Resolved AOT; consumers join by key.
    pub baked: Vec<BakedSlot>,
    /// Slot-level cremantic plane-claims `(slot name, claim)` — quinary-ladder
    /// census rows; only slots that spoke a sigil appear. Names pending (Sean).
    pub plane_claims: Vec<(String, PlaneClaim)>,
    /// Signal bindings + effort axes + `state {}` blocks, when the kit authors
    /// an automaton (astrological_starmap dialect, lowered 2026-08-24).
    pub automaton: Option<crate::automaton::KitAutomaton>,
}

struct ParsedSlot {
    name: String,
    kind: SlotKind,
    layout: Option<LayoutPolicy>,
    widget_name: Option<WidgetName>,
    grid_cols: Option<u16>,
    slot_list_max: Option<u16>,
    /// `size=mu(N)` — fixed MAIN-axis extent (i64 MilliUnit). None = kind-derived.
    size_main: Option<i64>,
    /// `gap=mu(N)` / `padding=mu(N)` — authored region spacing. None = density base.
    gap: Option<i64>,
    padding: Option<i64>,
    /// `min_size=mu(N)` / `max_size=mu(N)` — MAIN-axis floor and ceiling, applied
    /// AFTER the sizing policy resolves an extent. None = unclamped.
    min_size: Option<i64>,
    max_size: Option<i64>,
    /// `margin=mu(N)` — outer gap consuming the parent's flow. None = 0.
    margin: Option<i64>,
    /// `justify=` / `align=` — this region's child distribution. None = Start/Stretch.
    justify: Option<Justify>,
    align: Option<Align>,
    /// `text="…"` — the authored words. Was accepted-by-ignore until 2026-07-28,
    /// so every host re-declared its own labels in Rust and the kit's copy was
    /// decorative. Now carried through to [`WidgetSpec::text`]: the kit owns the
    /// words (root#vixi-t1), the host only places them.
    text: Option<String>,
    line: usize,
    hover_reveal: bool,
    long_press_drawer: bool,
    collapsible: bool,
    audio_reactive: bool,
    material: Option<String>,
    /// `color=palette.<slot>` — the slot's own chrome colour. Was
    /// accepted-by-ignore until 2026-07-29, which is why every card on the
    /// launcher drew the same `TokenId::BgDust` no matter what it authored:
    /// the kit stated an intent the runtime never read (root#rank
    /// DECLARED != EXERCISED). Carried to [`WidgetSpec::chrome_color`].
    chrome_color: Option<String>,
    /// `border_radius=mu(N)` — corner rounding. `DrawCmd::Rect` has carried a
    /// `radius` since forge-canvas draw.rs:590; nothing ever fed it from the
    /// kit, so every authored surface drew hard boxes.
    border_radius: Option<i64>,
    /// BRUSH FIGURE (Sean 2026-08-02, "update the grammar"). A brush could carry a bus and
    /// a colour but had no way to say WHAT IT DRAWS, so every figure lived in a Rust
    /// strangler. Declaring these in `grammar::SLOT_ATTRS` alone would only silence
    /// `check_kit` while nothing read them — the `justify=` shape (declared at
    /// grammar.rs:101, reaching no parser) that root#rank calls DECLARED != EXERCISED.
    /// They are baked HERE so the declaration and the behaviour land together.
    shape: Option<BrushShape>,
    /// `radius=pmy(N)` — permyriad of the parent's short side. A plate circle is a RATIO
    /// of its plate, never a pixel count, so this is not `mu()`.
    radius_pmy: Option<i64>,
    /// `phase=0 | tick`. `Tick` advances on `SimTick` — NEVER a wall clock (G-PLAT-01).
    phase: Option<BrushPhase>,
    /// `stride=weyl | even` — how sibling brushes distribute around the parent.
    stride: Option<BrushStride>,
    plane_claim: Option<PlaneClaim>,
    baked: BakedAttrs,
}

/// What a brush draws. The astrolabe's vocabulary, because that is the figure that forced
/// the attribute to exist: rings for the mater, stars for the rete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushShape {
    /// An annulus — the mater's rim.
    Ring,
    /// A filled circle.
    Disc,
    /// A pointed radial figure — the rete's stars.
    Star,
    /// A partial ring segment.
    Arc,
    /// A straight stroke.
    Line,
}

/// Whether a brush turns. The mater is fixed; the rete rotates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushPhase {
    /// `phase=0` — fixed. The plate does not turn.
    Fixed,
    /// `phase=tick` — advances on `SimTick`, integer, inside the DET plane.
    Tick,
}

/// How sibling brushes spread around their parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushStride {
    /// `stride=weyl` — `pp_math::formation::thirds_stride_bucket`, low-discrepancy: mass
    /// spreads rule-of-thirds-wide instead of piling at bucket 0.
    Weyl,
    /// `stride=even` — equal arcs.
    Even,
}

pub(crate) fn brush_shape(s: &str) -> Option<BrushShape> {
    Some(match s {
        "ring" => BrushShape::Ring,
        "disc" => BrushShape::Disc,
        "star" => BrushShape::Star,
        "arc" => BrushShape::Arc,
        "line" => BrushShape::Line,
        _ => return None,
    })
}

pub(crate) fn slot_kind(s: &str) -> Option<SlotKind> {
    // One table row per kind — forge_vix_syntax::tables is the SoT (2026-08-05).
    SlotKind::from_name(s)
}

pub(crate) fn layout_policy(s: &str) -> Option<LayoutPolicy> {
    // One table row per policy; the golden-corpus semantic aliases (2026-07-23,
    // split_view→StackH …) live in the same table (`@aliases`). SoT:
    // forge_vix_syntax::tables (2026-08-05); dockspace's native docking is
    // still the `.dock` descriptor (dock.rs), not here.
    LayoutPolicy::from_name(s)
}

pub(crate) const RESERVED: &[&str] = &["condition", "transition", "import"];

/// Max dotted-nesting depth for a slot name. `build_tree`/`to_spec` recurse once
/// per tree level, so an unbounded-depth name is a **stack-overflow DoS** (an
/// abort, not a catchable panic) — caught by `forge-fuzz`'s
/// `bounded_deep_nesting_is_rejected_not_crashed`. Cap it and return a structured
/// error instead. 64 is ~10x the deepest real authored kit, far below any
/// overflow threshold.
pub(crate) const MAX_SLOT_DEPTH: usize = 64;

/// Parse a `.kit.vixi` source string into a [`KitDoc`] (base tree only).
pub fn parse_kit(src: &str) -> Result<KitDoc, ParseError> {
    build_doc(src, None)
}

/// Parse with `active_variant` merged into the base tree (`Variant Blocks`,
/// template_grammar §). Variant slots override base slots of the same dotted
/// name and append new ones. Variant selection is a build/scene-time decision.
pub fn parse_kit_variant(src: &str, active_variant: &str) -> Result<KitDoc, ParseError> {
    build_doc(src, Some(active_variant))
}

fn build_doc(src: &str, active_variant: Option<&str>) -> Result<KitDoc, ParseError> {
    let mut dialect_version: Option<u32> = None;
    let mut surface: Option<String> = None;
    let mut profile: Option<String> = None;
    let mut z_mode: Option<ZMode> = None;
    let mut gates: Vec<(String, String)> = Vec::new();
    let mut rules: Vec<(String, String)> = Vec::new();
    let mut state_mutations: Vec<(String, String)> = Vec::new();
    let mut base: Vec<ParsedSlot> = Vec::new();
    let mut variants: Vec<(String, Vec<ParsedSlot>)> = Vec::new();
    let mut current: Option<usize> = None; // None = base; Some(i) = variants[i]
    let mut automaton = crate::automaton::AutomatonBuilder::new();

    for (i, raw) in src.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        // Header `#vixi:<dialect> vN` is checked before the comment rule.
        if let Some(rest) = line.strip_prefix("#vixi:") {
            let mut it = rest.split_whitespace();
            let dialect = it.next().unwrap_or("");
            if dialect != "kit" {
                return Err(ParseError::at(line_no, format!("expected '#vixi:kit', got dialect '{dialect}'")));
            }
            let v = it.next().unwrap_or("");
            let n = v
                .strip_prefix('v')
                .and_then(|d| d.parse::<u32>().ok())
                .ok_or_else(|| ParseError::at(line_no, format!("bad version token '{v}' (want vN)")))?;
            dialect_version = Some(n);
            continue;
        }
        if line.starts_with('#') {
            continue; // comment
        }

        // Inside an open `state {}` block every line is an automaton row (pole /
        // drive / `}`). Trailing `# …` comments are legal on automaton lines.
        if automaton.in_state() {
            let bare = line.split('#').next().unwrap_or("").trim();
            if !bare.is_empty() {
                automaton.state_line(line_no, bare)?;
            }
            continue;
        }

        let first = line.split_whitespace().next().unwrap_or("");
        match first {
            "slot" => {
                let slot = parse_slot(line, line_no)?;
                match current {
                    None => base.push(slot),
                    Some(idx) => variants[idx].1.push(slot),
                }
            }
            "variant" => {
                let name = line["variant".len()..].trim();
                if name.is_empty() {
                    return Err(ParseError::at(line_no, "`variant` block missing a name"));
                }
                variants.push((name.to_string(), Vec::new()));
                current = Some(variants.len() - 1);
            }
            "gate" => {
                let after = line["gate".len()..].trim();
                let (name, value) = after
                    .split_once('=')
                    .ok_or_else(|| ParseError::at(line_no, "gate line missing '='"))?;
                gates.push((name.trim().to_string(), value.trim().to_string()));
            }
            // `rule <name> = <value>` / `state_mutation <name> = <value>` — gate-family
            // authoring constraints (golden-corpus intake 2026-07-23). Same `name = value`
            // shape as `gate`; recorded (not invented behaviour), consumed downstream later.
            "rule" => {
                let after = line["rule".len()..].trim();
                let (name, value) = after
                    .split_once('=')
                    .ok_or_else(|| ParseError::at(line_no, "rule line missing '='"))?;
                rules.push((name.trim().to_string(), value.trim().to_string()));
            }
            "state_mutation" => {
                let after = line["state_mutation".len()..].trim();
                let (name, value) = after
                    .split_once('=')
                    .ok_or_else(|| ParseError::at(line_no, "state_mutation line missing '='"))?;
                state_mutations.push((name.trim().to_string(), value.trim().to_string()));
            }
            "state" => {
                let bare = line.split('#').next().unwrap_or("").trim();
                automaton.open_state(line_no, bare["state".len()..].trim())?;
            }
            _ if RESERVED.contains(&first) => {
                return Err(ParseError::at(line_no, format!("`{first}` is reserved/planned, not in v1")))
            }
            _ if first.ends_with(':') => {
                let key = first.trim_end_matches(':');
                let value = line[first.len()..].trim();
                if key == "surface" {
                    surface = Some(value.to_string());
                } else if key == "profile" {
                    // Recorded, not validated: an unknown profile name resolves to
                    // the live/comfy default downstream (`live::ctx_for`) rather
                    // than failing a panel that otherwise lowers clean.
                    profile = Some(value.to_string());
                } else if key == "z" {
                    // Cremantic z-capability — a bad value is LOUD (signal law),
                    // never accepted-by-ignore: a typo'd capability is a lie.
                    z_mode = Some(match value {
                        "front" => ZMode::Front,
                        "keep" => ZMode::Keep,
                        other => {
                            return Err(ParseError::at(
                                line_no,
                                format!("z: wants front|keep, got '{other}'"),
                            ))
                        }
                    });
                }
                // classification / audio_reactive: accepted-by-ignore in v1.
            }
            // Automaton top-level rows: `<dotted> = signal(<token>)` binding and
            // `<dotted> = <a> | <b>` axis (astrological_starmap dialect).
            _ if first.contains('.') && line.contains('=') => {
                let bare = line.split('#').next().unwrap_or("").trim();
                let (lhs, rhs) = bare
                    .split_once('=')
                    .map(|(l, r)| (l.trim(), r.trim()))
                    .ok_or_else(|| ParseError::at(line_no, format!("unrecognized line starting '{first}'")))?;
                if rhs.starts_with("signal(") {
                    automaton.bind(line_no, lhs, rhs)?;
                } else if rhs.contains('|') {
                    automaton.axis(line_no, lhs, rhs)?;
                } else {
                    return Err(ParseError::at(
                        line_no,
                        format!("automaton row wants `= signal(<token>)` or `= <a> | <b>`, got '{rhs}'"),
                    ));
                }
            }
            _ => return Err(ParseError::at(line_no, format!("unrecognized line starting '{first}'"))),
        }
    }

    let dialect_version =
        dialect_version.ok_or_else(|| ParseError::at(0, "missing `#vixi:kit vN` header"))?;
    let automaton = automaton.finish()?;

    // Resolve the active variant (if any) before consuming `variants` for names.
    let active_idx = match active_variant {
        Some(a) => Some(
            variants
                .iter()
                .position(|(n, _)| n == a)
                .ok_or_else(|| ParseError::at(0, format!("unknown variant '{a}'")))?,
        ),
        None => None,
    };
    let variant_names: Vec<String> = variants.iter().map(|(n, _)| n.clone()).collect();

    // Merge: base, then the active variant's slots override (by dotted name) /
    // append. Moves slots out of the variant group (no Clone on ParsedSlot).
    let mut merged = base;
    if let Some(idx) = active_idx {
        let group_slots = std::mem::take(&mut variants[idx].1);
        for vs in group_slots {
            match merged.iter().position(|s| s.name == vs.name) {
                Some(pos) => merged[pos] = vs,
                None => merged.push(vs),
            }
        }
    }

    if merged.is_empty() {
        // A pure-automaton kit (astrological_starmap dialect) carries no slots;
        // synthesize the root through the REAL slot parser so `root` stays total
        // and no second construction path exists.
        if automaton.as_ref().is_some_and(|a| !a.states.is_empty()) {
            merged.push(parse_slot("slot root kind=region layout=stack_v", 0)?);
        } else {
            return Err(ParseError::at(0, "no slots declared"));
        }
    }
    // Collect prebaked artist attributes (keyed by stable_key) before build_tree
    // consumes the slots. Only slots that carried a keyword get an entry.
    let baked: Vec<BakedSlot> = merged
        .iter()
        .filter(|s| s.baked.has_any())
        .map(|s| BakedSlot { stable_key: s.name.clone(), attrs: s.baked.clone() })
        .collect();
    // Cremantic plane-claim census rows — collected before build_tree consumes
    // the slots (same pattern as `baked`).
    let plane_claims: Vec<(String, PlaneClaim)> =
        merged.iter().filter_map(|s| s.plane_claim.map(|p| (s.name.clone(), p))).collect();
    let root = build_tree(merged)?;
    Ok(KitDoc { dialect_version, surface, profile, z_mode, root, gates, rules, state_mutations, variants: variant_names, baked, plane_claims, automaton })
}

/// Split a `slot` line on whitespace, except inside a double-quoted value.
///
/// `split_whitespace` tore `text="UI Defs:"` into `text="UI` and `Defs:"`, so an
/// authored label with a space read as a malformed attr. Cold-path parse, so the
/// allocation is free of the zero-alloc law (crate `compute-boundary-rules`).
fn split_attr_tokens(line: &str) -> Vec<String> {
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

fn parse_slot(line: &str, line_no: usize) -> Result<ParsedSlot, ParseError> {
    let toks = split_attr_tokens(line);
    let mut it = toks.iter().map(|s| s.as_str());
    // (see `split_attr_tokens` — a quoted attr value may contain spaces)
    let _slot_kw = it.next(); // "slot"
    let name = it
        .next()
        .ok_or_else(|| ParseError::at(line_no, "slot missing name"))?
        .to_string();

    // Depth guard: cap dotted nesting before it reaches the recursive tree
    // builder (an over-deep name stack-overflows `to_spec` — an uncatchable
    // abort, not a panic). Reject with a structured error instead.
    let depth = name.bytes().filter(|&b| b == b'.').count();
    if depth > MAX_SLOT_DEPTH {
        return Err(ParseError::at(
            line_no,
            format!("slot name nesting depth {depth} exceeds MAX_SLOT_DEPTH ({MAX_SLOT_DEPTH})"),
        ));
    }

    let mut kind: Option<SlotKind> = None;
    let mut layout: Option<LayoutPolicy> = None;
    let mut widget_name: Option<WidgetName> = None;
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
    let mut text: Option<String> = None;
    let mut hover_reveal = false;
    let mut long_press_drawer = false;
    let mut collapsible = false;
    let mut audio_reactive = false;
    let mut material: Option<String> = None;
    let mut chrome_color: Option<String> = None;
    let mut border_radius: Option<i64> = None;
    let mut shape: Option<BrushShape> = None;
    let mut radius_pmy: Option<i64> = None;
    let mut phase: Option<BrushPhase> = None;
    let mut stride: Option<BrushStride> = None;
    let mut plane_claim: Option<PlaneClaim> = None;
    let mut baked = BakedAttrs::default();

    for tok in it {
        // `visible_if <expr>` (condition, template_grammar §) is parsed-but-not-
        // lowered: reject explicitly rather than dropping it silently
        // (spec forbid.silent_parser_drops).
        if tok == "visible_if" {
            return Err(ParseError::at(line_no, "`visible_if` (condition) is reserved/planned — deferred, not in v1"));
        }
        // Cremantic plane-claim — bare backtick sigil token, NOT key=value
        // (quinary ladder 2026-07-09). One claim per slot; duplicates LOUD.
        if let Some(body) = tok.strip_prefix('`') {
            if plane_claim.is_some() {
                return Err(ParseError::at(line_no, format!("duplicate plane sigil on slot '{name}'")));
            }
            plane_claim = Some(PlaneClaim::from_sigil_body(body, line_no)?);
            continue;
        }
        let (k, v) = tok
            .split_once('=')
            .ok_or_else(|| ParseError::at(line_no, format!("slot attr '{tok}' is not key=value")))?;
        match k {
            "kind" => {
                kind = Some(slot_kind(v).ok_or_else(|| ParseError::at(line_no, format!("unknown kind '{v}'")))?)
            }
            "layout" => {
                layout = Some(layout_policy(v).ok_or_else(|| ParseError::at(line_no, format!("unknown layout '{v}'")))?)
            }
            // `name=button` (widget) and `of=widget.button` (slot_list element type)
            // both name the inventory widget; strip the optional `widget.` prefix.
            "name" | "of" => {
                let n = v.strip_prefix("widget.").unwrap_or(v);
                widget_name = Some(WidgetName(n.to_string()));
            }
            // grid column count (forge-vix extension to the slot attr grammar).
            "cols" => {
                grid_cols =
                    Some(v.parse::<u16>().map_err(|_| ParseError::at(line_no, format!("bad cols '{v}'")))?)
            }
            "max" => {
                slot_list_max =
                    Some(v.parse::<u16>().map_err(|_| ParseError::at(line_no, format!("bad max '{v}'")))?)
            }
            // `size=mu(N)` — fixed MAIN-axis extent (N px → MilliUnit): rail width in
            // stack_h / bar height in stack_v. A sibling Fill takes the rest; the
            // cross axis fills. Canon extension 2026-06-10 (was the unauthorable gap
            // that forced full-bleed overlays). See layout::Sizing::FixedMain.
            "size" => {
                size_main = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad size '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            // `gap=mu(N)` / `padding=mu(N)` — authored region spacing (was
            // accepted-by-ignore; the solver used density everywhere, inflating
            // every authored-tight surface by 12px per seam).
            "gap" => {
                gap = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad gap '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            "padding" => {
                padding = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad padding '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            // `min_size=mu(N)` / `max_size=mu(N)` — MAIN-axis clamp. Declared in
            // SLOT_ATTRS since v1 (grammar.rs:107-108) and reaching no parser, so a
            // kit could state a floor and the solver would shrink straight through it
            // (root#rank DECLARED != EXERCISED). Typed refusal, never accepted-by-ignore.
            "min_size" => {
                min_size = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad min_size '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            "max_size" => {
                max_size = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad max_size '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            // `margin=mu(N)` — outer gap, consuming the PARENT's flow (padding insets
            // this slot's own children; margin holds space around the slot itself).
            "margin" => {
                margin = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad margin '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            // `justify=` / `align=` — MAIN-axis distribution and CROSS-axis placement
            // of this region's children (grammar.rs:101-102, declared since v1 and
            // reaching no parser: every authored surface packed start/stretch no
            // matter what it said). Typed refusal, never accepted-by-ignore.
            "justify" => {
                justify = Some(Justify::from_name(v).ok_or_else(|| ParseError::at(
                    line_no,
                    format!("bad justify '{v}' — expected {}", Justify::NAMES.join("|")),
                ))?)
            }
            "align" => {
                align = Some(Align::from_name(v).ok_or_else(|| ParseError::at(
                    line_no,
                    format!("bad align '{v}' — expected {}", Align::NAMES.join("|")),
                ))?)
            }
            // `text="…"` — authored words. Quotes are stripped here so the host
            // never has to; `split_attr_tokens` already kept a spaced label whole.
            "text" => {
                text = Some(v.trim_matches('"').to_string());
            }
            // Smithy substrate Patch 6 (2026-05-26): affordance boolean flags.
            // Valid values: "true" | "false". Any other value is ignored (forward-compat).
            "hover_reveal"      => { hover_reveal      = v == "true"; }
            "long_press_drawer" => { long_press_drawer = v == "true"; }
            "collapsible"       => { collapsible       = v == "true"; }
            "audio_reactive"    => { audio_reactive    = v == "true"; }
            // `material=bronze` → carried to WidgetSpec/WidgetNode for the forge-canvas
            // PanelMaterial (GPU uber-shader axis), AND baked to a physical MaterialAtom
            // (albedo/reflectiveness/mohs/mass…) when the name is a CE material group.
            "material"          => { material = Some(v.to_string()); baked.material = baked::resolve_material(v); }
            // ── Prebaked artist keywords (UI-as-physical-voxel) ──────────────────
            // 1. Material physics overrides.
            "mass"              => { baked.mass_pmy = v.parse::<u16>().ok(); }
            "friction"          => { baked.friction_pmy = v.parse::<u16>().ok(); }
            // 2. Synesthetic binders (audio-reactive → VibeMatrix channel).
            "vibe_scale"        => { if let Some(c) = baked::parse_channel(v) { baked.vibe.push(VibeBind { channel: c, target: VibeTarget::ScaleRadius }); } }
            "vibe_glow"         => { if let Some(c) = baked::parse_channel(v) { baked.vibe.push(VibeBind { channel: c, target: VibeTarget::EmissiveGlow }); } }
            "vibe_opacity"      => { if let Some(c) = baked::parse_channel(v) { baked.vibe.push(VibeBind { channel: c, target: VibeTarget::Opacity }); } }
            "vibe_offsety"      => { if let Some(c) = baked::parse_channel(v) { baked.vibe.push(VibeBind { channel: c, target: VibeTarget::OffsetY }); } }
            // 3. Chromatic reflection (ambilight).
            "screen_edge"       => { baked.screen_edge = baked::parse_edge(v); }
            "blend"             => { baked.blend = baked::parse_blend(v); }
            // 4. Deterministic kinematics (spring).
            "motion"            => { baked.motion = baked::resolve_motion(v); }
            "attractor"         => { baked.motion_attractor_pointer = v == "pointer"; }
            // 5. Semantic edict triggers (`on_click=edict:<id>` · `on_key=<chord>:edict:<id>`).
            "on_click"          => { baked.on_click_edict = v.strip_prefix("edict:").map(|s| s.to_string()); }
            "on_key"            => {
                baked.on_key_edict = v.split_once(':').and_then(|(chord, rest)| {
                    Some((baked::parse_chord(chord)?, rest.strip_prefix("edict:")?.to_string()))
                });
            }
            // 6. Stroke z-depth — ᐍ (West-Cree WE U+140D); author-time commitment, ADR-0006 D8.
            "\u{140D}"          => { baked.stroke_z_mu = v.parse::<i32>().ok(); }
            // 7. Live host-data bind (STUDIO-TRANSFER M3): `source=<snapshot field>`.
            "source"            => { baked.source = Some(v.to_string()); }
            // Authored binding word, kept verbatim and uninterpreted — see
            // `BakedAttrs::bind`. Was accepted-by-ignore in BOTH v2 and v3 until
            // 2026-08-24, so every `bind=` in the panel corpus was discarded.
            "bind"              => { baked.bind = Some(v.to_string()); }
            // 8. Semantic role — retained for audit::audit_layout (2026-07-20).
            "role"              => { baked.role = Some(v.to_string()); }
            // 8b. Golden-corpus vocabulary registered 2026-08-01 (the 16 exemplars
            // authored these; the grammar warned unknown-attr and dropped them).
            // `unit`/`font`/`semantic` are tokens the host resolves; `alpha` is
            // permyriad (float_in_ir forbidden); the three flags follow Patch 6.
            "unit"              => { baked.unit = Some(v.to_string()); }
            "font"              => { baked.font = Some(v.to_string()); }
            "semantic"          => { baked.semantic = Some(v.to_string()); }
            "alpha"             => { baked.alpha_pmy = baked::BakedAttrs::parse_alpha_pmy(v); }
            "primary"           => { baked.primary        = v == "true"; }
            "fixed_position"    => { baked.fixed_position = v == "true"; }
            "searchable"        => { baked.searchable     = v == "true"; }
            // 9. Safe-area class (SMPTE ST 2046 / BBC 90-80 margins) — HUD anchors resolve inside this rect.
            "safe_area"         => { baked.safe_area = baked::parse_safe_area(v); }
            // 10. HUD taxonomy (Fagerholt/Lorentzon) — where the element lives in the fiction.
            "hud_class"         => { baked.hud_class = baked::parse_hud_class(v); }
            // 11. Pad focus-walk landing slot.
            "focus"             => { baked.focusable = v == "true"; }
            // 12. Authored chrome (2026-07-29). `color=` names a palette slot and
            // `border_radius=` its rounding; both were accepted-by-ignore, so the
            // runtime styled by convention and the kit was decorative. The token
            // NAME is carried, never a literal colour — the profile still owns the
            // hex (forge-gui#colour-resolution), the kit only says WHICH slot.
            "color"             => { chrome_color = Some(v.strip_prefix("palette.").unwrap_or(v).to_string()); }
            // 12b. `ramp=type.ramp[N]` — authored type-ramp stop (was
            // accepted-by-ignore at the `_` arm below; `diagnostics::ramp_stop_for`
            // is the ONE reader, so no second copy of the pick rule lives here).
            "ramp" => {
                let idx = v
                    .strip_prefix("type.ramp[")
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or_else(|| ParseError::at(line_no, format!("bad ramp '{v}' — expected type.ramp[0..=4]")))?;
                let idx: u8 = idx.parse().map_err(|_| {
                    ParseError::at(line_no, format!("bad ramp '{v}' — expected type.ramp[0..=4]"))
                })?;
                if idx > 4 {
                    return Err(ParseError::at(line_no, format!("bad ramp '{v}' — expected type.ramp[0..=4]")));
                }
                baked.ramp = Some(forge_canvas_v3::text::FontSize::from_index(idx));
            }
            "border_radius"     => {
                border_radius = Some(parse_mu(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad border_radius '{v}' — expected mu(N) or N (px)"))
                })?)
            }
            // ── Brush figure — typed refusal, never accepted-by-ignore ──────────
            "shape" => {
                shape = Some(brush_shape(v).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad shape '{v}' — expected ring|disc|star|arc|line"))
                })?)
            }
            "radius" => {
                // PERMYRIAD, not MilliUnit: a plate circle is a ratio of its plate.
                let raw = v.strip_prefix("pmy(").and_then(|s| s.strip_suffix(')')).unwrap_or(v);
                radius_pmy = Some(raw.parse::<i64>().ok().filter(|n| (0..=10_000).contains(n)).ok_or_else(|| {
                    ParseError::at(line_no, format!("bad radius '{v}' — expected pmy(0..=10000)"))
                })?)
            }
            "phase" => {
                phase = Some(match v {
                    "0" | "fixed" => BrushPhase::Fixed,
                    "tick" => BrushPhase::Tick,
                    _ => return Err(ParseError::at(line_no, format!("bad phase '{v}' — expected 0|tick (never a wall clock)"))),
                })
            }
            "stride" => {
                stride = Some(match v {
                    "weyl" => BrushStride::Weyl,
                    "even" => BrushStride::Even,
                    _ => return Err(ParseError::at(line_no, format!("bad stride '{v}' — expected weyl|even"))),
                })
            }
            // bind/curve/thick/bus_in/priority/... :
            // accepted-by-ignore in v1 (styling resolved via TokenCtx + later passes).
            // `min_size`/`max_size`/`justify`/`align` left this arm 2026-08-04 — they are
            // typechecked above and consumed by `layout::{clamp_main,distribute,cross_offset}`.
            // `ramp` left this arm 2026-08-07 (DEBT-RAMP-ATTR-ACCEPTED-BY-IGNORE) — typed
            // above and consumed by `LoweredUi::ramp_binds`.
            _ => {}
        }
    }

    let kind = kind.ok_or_else(|| ParseError::at(line_no, format!("slot '{name}' missing kind=")))?;
    Ok(ParsedSlot {
        name, kind, layout, widget_name, grid_cols, slot_list_max, size_main, gap, padding,
        min_size, max_size, margin, justify, align, text,
        line: line_no,
        hover_reveal, long_press_drawer, collapsible, audio_reactive, material,
        chrome_color, border_radius, shape, radius_pmy, phase, stride, plane_claim, baked,
    })
}

fn parent_name(name: &str) -> Option<&str> {
    name.rfind('.').map(|i| &name[..i])
}

/// Parse a `size=` value into i64 MilliUnit. Accepts `mu(N)` or a bare `N`; both
/// are N pixels (1px = 1000 MilliUnit, the IR unit). `None` on a malformed value.
fn parse_mu(v: &str) -> Option<i64> {
    let inner = v.strip_prefix("mu(").and_then(|s| s.strip_suffix(')')).unwrap_or(v);
    inner.trim().parse::<i64>().ok().map(|px| px * 1000)
}

/// Claim slot `i` as the single tree root; a second claim is the multi-root error
/// (a genuine flat-sibling/dockspace surface with two DISTINCT namespaces still routes
/// to the egui-bridge, not the v1 parser).
fn claim_root(root: &mut Option<usize>, i: usize, slots: &[ParsedSlot]) -> Result<(), ParseError> {
    if let Some(prev) = *root {
        return Err(ParseError::at(
            slots[i].line,
            format!(
                "multiple root slots ('{}' and '{}') — flat-sibling/dockspace surfaces route to the egui-bridge, not the v1 parser",
                slots[prev].name, slots[i].name
            ),
        ));
    }
    *root = Some(i);
    Ok(())
}

fn build_tree(slots: Vec<ParsedSlot>) -> Result<WidgetSpec, ParseError> {
    let index: HashMap<String, usize> =
        slots.iter().enumerate().map(|(i, s)| (s.name.clone(), i)).collect();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); slots.len()];
    let mut root: Option<usize> = None;

    for (i, s) in slots.iter().enumerate() {
        match parent_name(&s.name) {
            None => claim_root(&mut root, i, &slots)?,
            Some(p) => {
                // Flat-namespace convention (golden-corpus intake 2026-07-23): a surface
                // may skip the explicit `slot X` and declare `X.root` (the container, with
                // its layout) plus flat siblings `X.<name>` meant to live inside it. So an
                // undeclared parent `X` resolves to `X.root`, and `X.root` itself is THE
                // surface root — the `.root` suffix is the design signal, not a guess.
                if !index.contains_key(p) {
                    let ns_root = format!("{p}.root");
                    if s.name == ns_root {
                        claim_root(&mut root, i, &slots)?;
                        continue;
                    }
                    if let Some(&ri) = index.get(&ns_root) {
                        children[ri].push(i);
                        continue;
                    }
                }
                let pi = *index.get(p).ok_or_else(|| {
                    ParseError::at(s.line, format!("orphan slot '{}': parent '{}' not declared", s.name, p))
                })?;
                children[pi].push(i);
            }
        }
    }

    let root = root.ok_or_else(|| ParseError::at(0, "no root slot (every slot had a dotted parent)"))?;
    Ok(to_spec(root, &slots, &children))
}

fn to_spec(i: usize, slots: &[ParsedSlot], children: &[Vec<usize>]) -> WidgetSpec {
    let s = &slots[i];
    // Token-driven sizing defaults:
    //   Region / JournalText → Fill the parent.
    //   SlotList / Drawer    → Hug content (collapses to zero when empty/closed).
    //   Leaves (Widget, Chrome, Text, Image, Brush, SigilCorner) → Intrinsic.
    // Explicit `size=mu(N)` wins over the kind-derived default — a fixed MAIN-axis
    // extent (FixedMain), the authorable rail/bar thickness (canon extension).
    let sizing = match s.size_main {
        Some(n) => Sizing::FixedMain(n),
        None => match s.kind {
            SlotKind::Region | SlotKind::JournalText => Sizing::Fill,
            SlotKind::SlotList | SlotKind::Drawer    => Sizing::Hug,
            _ => Sizing::Intrinsic,
        },
    };
    // A slot_list with no declared layout stacks its (homogeneous) children.
    let layout = match (s.kind, s.layout) {
        (SlotKind::SlotList, None) => Some(LayoutPolicy::StackV),
        (_, l) => l,
    };
    let kids = children[i].iter().map(|&c| to_spec(c, slots, children)).collect();
    WidgetSpec {
        stable_key: s.name.clone(),
        kind: s.kind,
        widget_name: s.widget_name.clone(),
        layout,
        sizing,
        grid_cols: s.grid_cols,
        slot_list_max: s.slot_list_max,
        children: kids,
        hover_reveal: s.hover_reveal,
        long_press_drawer: s.long_press_drawer,
        collapsible: s.collapsible,
        audio_reactive: s.audio_reactive,
        material: s.material.clone(),
        // The FIGURE reaches the spec instead of dying here. `to_spec` copied every other
        // authored attribute and dropped these four, which is why a brush panel lays out
        // correctly and paints nothing (accepted-by-ignore, visible in the 08-02 captures:
        // raycast_brush_panel renders as empty bars). A shape alone is enough to draw with;
        // the rest carry their own documented defaults.
        brush: s.shape.map(|shape| crate::layout::BrushSlot {
            shape,
            radius_pmy: s.radius_pmy.unwrap_or(5_774).clamp(0, 10_000) as u16,
            phase: s.phase.unwrap_or(BrushPhase::Fixed),
            stride: s.stride.unwrap_or(BrushStride::Even),
        }),
        text: s.text.clone(),
        gap: s.gap,
        padding: s.padding,
        min_size: s.min_size,
        max_size: s.max_size,
        margin: s.margin,
        justify: s.justify,
        align: s.align,
        chrome_color: s.chrome_color.clone(),
        border_radius: s.border_radius,
        alpha_pmy: s.baked.alpha_pmy,
        font: s.baked.font.clone(),
        semantic: s.baked.semantic.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrRect, WidgetId};
    use crate::layout::{lower, TokenCtx};

    /// Quinary plane ladder (seed wave 2026-07-09): full closed set parses,
    /// absence is silent, everything outside the set fails LOUD.
    #[test]
    fn plane_sigil_full_ladder_parses() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v `4\n\
slot root.a kind=text `0\n\
slot root.b kind=text `1\n\
slot root.c kind=text `2\n\
slot root.d kind=text `3\n\
slot root.e kind=text `\n";
        let doc = parse_kit(src).expect("ladder parses");
        assert_eq!(
            doc.plane_claims,
            vec![
                ("root".to_string(), PlaneClaim::P4),
                ("root.a".to_string(), PlaneClaim::P0),
                ("root.b".to_string(), PlaneClaim::P1),
                ("root.c".to_string(), PlaneClaim::P2),
                ("root.d".to_string(), PlaneClaim::P3),
                ("root.e".to_string(), PlaneClaim::Unclaimed),
            ]
        );
    }

    #[test]
    fn plane_sigil_absent_is_silent() {
        assert!(parse_kit(SLICE).expect("parse").plane_claims.is_empty());
    }

    #[test]
    fn plane_sigil_outside_closed_set_is_loud() {
        for bad in ["`5", "`x", "`00", "`01", "`canvas"] {
            let src = format!("#vixi:kit v1\nslot root kind=region layout=stack_v {bad}\n");
            assert!(parse_kit(&src).is_err(), "{bad} must fail LOUD, never default");
        }
    }

    #[test]
    fn plane_sigil_duplicate_is_loud() {
        let src = "#vixi:kit v1\nslot root kind=region layout=stack_v `2 `3\n";
        assert!(parse_kit(src).is_err(), "two sigils on one slot must fail LOUD");
    }

    const SLICE: &str = "#vixi:kit v1\n\
surface: studio\n\
# the vertical slice\n\
slot root kind=region layout=stack_v\n\
slot root.panel kind=region layout=stack_v\n\
slot root.panel.apply kind=widget name=button\n\
slot root.panel.name kind=widget name=text_field\n\
gate hit_target_min = mu(44)\n";

    // [BOARD: AOT-PARITY-104] AOT PARITY across the WHOLE registry. `baked` is resolved
    // ahead of time and consumers join it by `stable_key` (parse.rs KitDoc.baked doc), so
    // the bake is only trustworthy while three things hold for EVERY authored panel:
    // every baked key names a slot the runtime tree actually has (no join against a ghost),
    // no key appears twice (a join would silently pick one), and re-parsing the same source
    // yields the identical bake (an AOT artifact that drifts per run is not an artifact).
    // A ghost key is the failure that hurts: the consumer joins, misses, and falls back to
    // a default that looks like an authoring choice.
    // REMOVED (gap named, not faked): v1's `every_registered_panel_bakes_in_parity_
    // with_its_runtime_tree` needs `crate::loader::STUDIO_PANELS` — the panel
    // registry from v1's `loader.rs` (~350k bytes with grammar.rs/semantic.rs,
    // explicitly out of scope for this crate per its own lib.rs doc comment from
    // the first forge-vix-v3 slice). This test iterates every registered studio
    // panel; without a panel registry there is nothing for it to iterate. The
    // AOT-parity PROPERTY it checks (baked keys all resolve, no duplicates,
    // determinism across re-parses) is still exercised per-source by the other
    // tests in this module — this test specifically checked it across the whole
    // registry, which doesn't exist here yet.

    // [BOARD: RAMP-ATTR-ACCEPTED-BY-IGNORE] `ramp=type.ramp[N]` bakes to a real
    // FontSize instead of the `_ => {}` catch-all dropping it on the floor.
    #[test]
    fn ramp_attr_bakes_instead_of_accepted_by_ignore() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.body kind=text ramp=type.ramp[4]\n";
        let doc = parse_kit(src).expect("parse");
        let baked = doc
            .baked
            .iter()
            .find(|b| b.stable_key == "root.body")
            .expect("root.body carries a baked row (ramp= makes it non-empty)");
        assert_eq!(baked.attrs.ramp, Some(forge_canvas_v3::text::FontSize::Display));
    }

    #[test]
    fn ramp_attr_out_of_range_fails_loud() {
        let src = "#vixi:kit v1\nslot root kind=region layout=stack_v\nslot root.body kind=text ramp=type.ramp[9]\n";
        assert!(parse_kit(src).is_err(), "an out-of-range ramp index must fail LOUD, never clamp silently");
    }

    /// Cremantic z-capability: declared -> carried, absent -> None, typo -> LOUD.
    #[test]
    fn z_capability_declares_or_fails_loud() {
        let keep = "#vixi:kit v1\nz: keep\nslot root kind=region layout=stack_v\n";
        assert_eq!(parse_kit(keep).expect("keep parses").z_mode, Some(ZMode::Keep));

        let front = "#vixi:kit v1\nz: front\nslot root kind=region layout=stack_v\n";
        assert_eq!(parse_kit(front).expect("front parses").z_mode, Some(ZMode::Front));

        // Undeclared = None (consumer default) — SLICE has no z: line.
        assert_eq!(parse_kit(SLICE).expect("parse").z_mode, None);

        // A typo'd capability is a parse ERROR, never accepted-by-ignore.
        let bad = "#vixi:kit v1\nz: sideways\nslot root kind=region layout=stack_v\n";
        assert!(parse_kit(bad).is_err(), "bad z value must fail LOUD");
    }

    #[test]
    fn parses_slice_tree() {
        let doc = parse_kit(SLICE).expect("parse");
        assert_eq!(doc.dialect_version, 1);
        assert_eq!(doc.surface.as_deref(), Some("studio"));
        assert_eq!(doc.gates, vec![("hit_target_min".to_string(), "mu(44)".to_string())]);

        let root = &doc.root;
        assert_eq!(root.stable_key, "root");
        assert_eq!(root.kind, SlotKind::Region);
        assert_eq!(root.layout, Some(LayoutPolicy::StackV));
        assert_eq!(root.children.len(), 1);

        let panel = &root.children[0];
        assert_eq!(panel.stable_key, "root.panel");
        assert_eq!(panel.kind, SlotKind::Region);
        assert_eq!(panel.children.len(), 2);
        assert_eq!(panel.children[0].kind, SlotKind::Widget);
        assert_eq!(panel.children[0].widget_name, Some(WidgetName("button".into())));
        assert_eq!(panel.children[1].widget_name, Some(WidgetName("text_field".into())));
    }

    #[test]
    fn parsed_tree_lowers_and_is_hittable() {
        let doc = parse_kit(SLICE).expect("parse");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 960_000, 540_000), &TokenCtx::comfy(), 1);
        assert_eq!(ui.focus.len(), 2); // button + text_field focusable
        assert_eq!(ui.text_inputs.len(), 1); // text_field
        assert!(ui.versions_synced());

        // The parsed button is hittable at its own computed center (end-to-end).
        let btn: WidgetId = ui
            .widgets
            .iter()
            .find(|w| w.widget_name == Some(WidgetName("button".into())))
            .unwrap()
            .id;
        let r = ui.layout.iter().find(|b| b.widget_id == btn).unwrap().rect;
        let (cx, cy) = ((r.min_x + r.max_x) / 2, (r.min_y + r.max_y) / 2);
        assert_eq!(ui.hit_test(cx, cy), Some(btn));
    }

    #[test]
    fn rejects_reserved_and_condition() {
        // condition/transition/import remain explicitly deferred (fail validation,
        // never silently dropped — spec forbid.silent_parser_drops).
        assert!(parse_kit("#vixi:kit v1\nimport base\n").is_err());
        assert!(parse_kit("#vixi:kit v1\ntransition fade\n").is_err());
        assert!(parse_kit("#vixi:kit v1\ncondition x\n").is_err());
        assert!(
            parse_kit("#vixi:kit v1\nslot q kind=widget name=button visible_if affection>0\n").is_err()
        );
    }

    /// The authored starmap dialect (signal bindings + effort axes + `state`
    /// blocks) LOWERS as of 2026-08-24 — this flips the old pinned refusal.
    /// The real file is the fixture, so drift between the authored automaton
    /// and the grammar fails here, not on glass.
    #[test]
    fn automaton_kit_lowers_the_authored_starmap_file() {
        const STARMAP: &str =
            include_str!("../../forge-envelope/surfaceledger/astrological_starmap.kit.vixi");
        let doc = parse_kit(STARMAP).expect("the authored automaton kit must lower");
        let auto = doc.automaton.expect("automaton must be present");
        assert_eq!(auto.bindings.len(), 4);
        assert_eq!(auto.binding("schaeffer.mass"), Some("audio.sub_bass"));
        assert_eq!(auto.binding("schaeffer.dynamic"), Some("audio.rms"));
        assert_eq!(auto.binding("schaeffer.grain"), Some("audio.spectrum_high"));
        assert_eq!(auto.binding("schaeffer.allure"), Some("audio.formant_energy"));
        assert_eq!(auto.axes.len(), 4);
        assert_eq!(auto.states.len(), 4);
        for (name, space, weight, time, flow, glow_src, glow, shake_src, shake) in [
            ("listening", "direct", "light", "sudden", "bound", Some("schaeffer.mass"), 8_000, Some("schaeffer.grain"), 2_000),
            ("previewing", "indirect", "light", "sustained", "free", Some("schaeffer.mass"), 5_000, Some("schaeffer.allure"), 4_000),
            ("executing", "direct", "strong", "sudden", "free", Some("schaeffer.dynamic"), 9_500, Some("schaeffer.grain"), 8_500),
            ("sleep", "indirect", "light", "sustained", "bound", None, 1_000, None, 0),
        ] {
            let st = auto.state(name).unwrap_or_else(|| panic!("state '{name}' must lower"));
            assert_eq!(st.pole("laban.space"), Some(space), "{name} space");
            assert_eq!(st.pole("laban.weight"), Some(weight), "{name} weight");
            assert_eq!(st.pole("laban.time"), Some(time), "{name} time");
            assert_eq!(st.pole("laban.flow"), Some(flow), "{name} flow");
            let g = st.drive("vibe_glow").unwrap_or_else(|| panic!("{name} glow drive"));
            assert_eq!((g.source.as_deref(), g.gain_pmy), (glow_src, glow), "{name} glow");
            let s = st.drive("vibe_shake").unwrap_or_else(|| panic!("{name} shake drive"));
            assert_eq!((s.source.as_deref(), s.gain_pmy), (shake_src, shake), "{name} shake");
        }
        // Gate footer still rides the same doc.
        assert!(doc.gates.iter().any(|(n, v)| n == "runtime_parse" && v == "forbidden"));
    }

    /// Malformed automaton rows refuse LOUD with their line number.
    #[test]
    fn automaton_rows_refuse_loud() {
        let head = "#vixi:kit v1\nslot root kind=region layout=stack_v\n";
        // Undeclared axis / pole outside the axis / undeclared drive source / bad gain.
        for bad in [
            "state s {\nlaban.space <- direct\n}\n",
            "laban.space = direct | indirect\nstate s {\nlaban.space <- sideways\n}\n",
            "state s {\nvibe_glow <- schaeffer.mass * 100p\n}\n",
            "schaeffer.mass = signal(audio.rms)\nstate s {\nvibe_glow <- schaeffer.mass * 100q\n}\n",
            "state s {\nvibe_glow <- 5p\n", // unclosed block
            "state s {\n}\nstate s {\n}\n", // duplicate state
            "schaeffer.mass = nonsense\n",  // bad top-level automaton row
        ] {
            assert!(parse_kit(&format!("{head}{bad}")).is_err(), "must refuse: {bad}");
        }
        // A slot-less doc with no automaton still refuses.
        assert!(parse_kit("#vixi:kit v1\nsurface: empty\n").is_err());
    }

    #[test]
    fn parses_grid_and_slot_list() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=grid cols=2\n\
slot root.a kind=widget name=button\n\
slot root.b kind=widget name=button\n\
slot root.list kind=slot_list of=widget.command_row max=6\n";
        let doc = parse_kit(src).expect("parse");
        assert_eq!(doc.root.layout, Some(LayoutPolicy::Grid));
        assert_eq!(doc.root.grid_cols, Some(2));
        let list = doc.root.children.iter().find(|c| c.stable_key == "root.list").unwrap();
        assert_eq!(list.kind, SlotKind::SlotList);
        assert_eq!(list.slot_list_max, Some(6));
        // `widget.` prefix stripped off the `of=` element type.
        assert_eq!(list.widget_name, Some(WidgetName("command_row".into())));
        assert_eq!(list.layout, Some(LayoutPolicy::StackV)); // default for slot_list
    }

    #[test]
    fn variant_overrides_base() {
        let src = "#vixi:kit v1\n\
surface: dialogue\n\
slot root kind=region layout=stack_v\n\
slot root.choices kind=widget name=button\n\
variant rpg\n\
slot root.choices kind=slot_list of=widget.button max=4\n";

        // Base: root.choices is a plain widget; rpg variant declared.
        let base = parse_kit(src).expect("parse base");
        assert_eq!(base.variants, vec!["rpg".to_string()]);
        let bc = base.root.children.iter().find(|c| c.stable_key == "root.choices").unwrap();
        assert_eq!(bc.kind, SlotKind::Widget);

        // Active rpg variant overrides root.choices with a slot_list.
        let rpg = parse_kit_variant(src, "rpg").expect("parse rpg");
        let rc = rpg.root.children.iter().find(|c| c.stable_key == "root.choices").unwrap();
        assert_eq!(rc.kind, SlotKind::SlotList);
        assert_eq!(rc.slot_list_max, Some(4));

        // Unknown variant is an error, not a silent fallback.
        assert!(parse_kit_variant(src, "nope").is_err());
    }

    #[test]
    fn rejects_orphan_slot() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.panel.apply kind=widget name=button\n";
        let err = parse_kit(src).unwrap_err();
        assert!(err.message.contains("orphan"), "got: {}", err.message);
    }

    #[test]
    fn requires_header() {
        assert!(parse_kit("slot root kind=region layout=stack_v\n").is_err());
    }

    // The studio.kit.vixi I authored in forge-studio-blueprint uses mystery
    // affordance attributes that the v1 parser accepts-by-ignore via the catch-all
    // arm in slot attribute parsing. These tests lock that behavior in.

    #[test]
    fn patch6_new_slot_kinds_parse() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.badge kind=sigil_corner\n\
slot root.log kind=journal_text\n\
slot root.tray kind=drawer\n";
        let doc = parse_kit(src).expect("patch 6 kinds must parse");
        let badge = doc.root.children.iter().find(|c| c.stable_key == "root.badge").unwrap();
        let log   = doc.root.children.iter().find(|c| c.stable_key == "root.log").unwrap();
        let tray  = doc.root.children.iter().find(|c| c.stable_key == "root.tray").unwrap();
        assert_eq!(badge.kind, SlotKind::SigilCorner);
        assert_eq!(log.kind,   SlotKind::JournalText);
        assert_eq!(tray.kind,  SlotKind::Drawer);
        // Sizing defaults: SigilCorner=Intrinsic, JournalText=Fill, Drawer=Hug.
        assert_eq!(badge.sizing, Sizing::Intrinsic);
        assert_eq!(log.sizing,   Sizing::Fill);
        assert_eq!(tray.sizing,  Sizing::Hug);
    }

    /// The golden corpus authored these seven before the grammar knew them, so the
    /// lint warned unknown-attr and the lowerer dropped every one. Registration is
    /// only half — this asserts each lands in `BakedAttrs`, on the exact lines the
    /// exemplars author (audio_vis:7-12, debug_overlay:6-9, animation_timeline:8,
    /// forgewright_cad:8). A grammar entry with no landing field is an allowlist.
    #[test]
    fn golden_corpus_vocabulary_lands_in_baked_attrs() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.deck kind=region role=deck primary=true\n\
slot root.mixer kind=region role=center_mixer fixed_position=true\n\
slot root.catalog kind=region role=mesh_catalog searchable=true\n\
slot root.overlay kind=region role=overlay alpha=permyriad(8500)\n\
slot root.fps kind=text role=data font=mono\n\
slot root.meter kind=widget role=meter semantic=green_yellow_red\n\
slot root.ruler kind=widget role=tick_ruler unit=ticks\n";
        let doc = parse_kit(src).expect("golden vocabulary must parse");
        let baked = |key: &str| {
            doc.baked
                .iter()
                .find(|b| b.stable_key == key)
                .unwrap_or_else(|| panic!("{key} carried no BakedAttrs — the attr was dropped"))
                .attrs
                .clone()
        };
        assert!(baked("root.deck").primary, "primary=true must land");
        assert!(baked("root.mixer").fixed_position, "fixed_position=true must land");
        assert!(baked("root.catalog").searchable, "searchable=true must land");
        assert_eq!(baked("root.overlay").alpha_pmy, Some(8500), "alpha must land on the permyriad lattice, not as a float");
        assert_eq!(baked("root.fps").font.as_deref(), Some("mono"));
        assert_eq!(baked("root.meter").semantic.as_deref(), Some("green_yellow_red"));
        assert_eq!(baked("root.ruler").unit.as_deref(), Some("ticks"));
    }

    /// RED guard: an unauthored slot must NOT acquire these by default, or the
    /// GREEN above would pass on a struct that sets everything.
    #[test]
    fn unauthored_slots_carry_none_of_the_new_vocabulary() {
        let doc = parse_kit("#vixi:kit v1\nslot root kind=region layout=stack_v\n").expect("parses");
        let a = crate::baked::BakedAttrs::default();
        assert!(!a.primary && !a.fixed_position && !a.searchable);
        assert!(a.unit.is_none() && a.font.is_none() && a.semantic.is_none() && a.alpha_pmy.is_none());
        assert!(!a.has_any(), "a default BakedAttrs must not look authored");
        assert!(doc.baked.is_empty(), "a bare slot must bake nothing");
    }

    /// `alpha` is permyriad, and the lattice has a ceiling: past-range clamps to
    /// opaque rather than wrapping a u16 into near-transparent.
    #[test]
    fn alpha_past_the_permyriad_ceiling_clamps_opaque() {
        use crate::baked::BakedAttrs as B;
        assert_eq!(B::parse_alpha_pmy("permyriad(8500)"), Some(8500));
        assert_eq!(B::parse_alpha_pmy("8500"), Some(8500));
        assert_eq!(B::parse_alpha_pmy("permyriad(12000)"), Some(10_000), "clamp, never wrap");
        assert_eq!(B::parse_alpha_pmy("permyriad(0)"), Some(0));
        assert_eq!(B::parse_alpha_pmy("half"), None);
    }

    #[test]
    fn patch6_affordance_flags_propagate() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.hov kind=region layout=overlay hover_reveal=true\n\
slot root.lp  kind=drawer long_press_drawer=true\n\
slot root.col kind=region layout=stack_v collapsible=true\n\
slot root.aud kind=brush audio_reactive=true\n";
        let doc = parse_kit(src).expect("affordance flags must parse");
        let hov = doc.root.children.iter().find(|c| c.stable_key == "root.hov").unwrap();
        let lp  = doc.root.children.iter().find(|c| c.stable_key == "root.lp").unwrap();
        let col = doc.root.children.iter().find(|c| c.stable_key == "root.col").unwrap();
        let aud = doc.root.children.iter().find(|c| c.stable_key == "root.aud").unwrap();
        assert!(hov.hover_reveal,      "hover_reveal=true must propagate");
        assert!(lp.long_press_drawer,  "long_press_drawer=true must propagate");
        assert!(col.collapsible,       "collapsible=true must propagate");
        assert!(aud.audio_reactive,    "audio_reactive=true must propagate");
        // Flags default false on unset slots.
        assert!(!hov.long_press_drawer);
        assert!(!col.hover_reveal);
    }

    #[test]
    fn patch6_flags_default_false() {
        let src = "#vixi:kit v1\nslot root kind=region layout=stack_v\n";
        let doc = parse_kit(src).expect("parse");
        assert!(!doc.root.hover_reveal);
        assert!(!doc.root.long_press_drawer);
        assert!(!doc.root.collapsible);
        assert!(!doc.root.audio_reactive);
    }

    #[test]
    fn smithy_mystery_attrs_accepted_by_ignore() {
        let src = "#vixi:kit v1\n\
surface: studio\n\
profile: forge_smithy\n\
slot root kind=region layout=stack_v\n\
slot root.bar kind=region layout=stack_h role=container\n\
slot root.bar.btn kind=widget name=button\n\
slot root.props kind=region layout=stack_v role=context_properties collapsible=true\n\
slot root.assets kind=region layout=grid role=asset_grid searchable=true\n\
slot root.debug kind=region layout=overlay role=telemetry_overlay optional=true hover_reveal=true\n\
slot root.cmd kind=region layout=stack_v role=command_palette optional=true long_press_drawer=true\n\
gate hit_target_min = mu(44)\n";
        let doc = parse_kit(src).expect("parser must accept unknown attrs via catch-all");
        assert_eq!(doc.surface.as_deref(), Some("studio"));
        // Spot-check that the slots landed despite unknown attrs.
        let root = &doc.root;
        assert!(root.children.iter().any(|c| c.stable_key == "root.debug"));
        assert!(root.children.iter().any(|c| c.stable_key == "root.cmd"));
    }

    // RETIRED 2026-06-09: `smithy_studio_kit_parses_end_to_end` reached across crate
    // boundaries via `include_str!` into `forge-studio-blueprint/` — a path the
    // consolidation move relocated, breaking the whole test target. The fixture was a
    // stale older-UI kit (used `name=sigil_corner`/`searchable=` — syntax the current
    // grammar treats differently) that only "passed" via parser leniency, and it never
    // exercised the Patch-6 slot-kinds. Those kinds are covered directly above
    // (`kind=sigil_corner/journal_text/drawer` at the smithy-kinds test) + locked by the
    // grammar arm-count tripwire; current-UI lowering is anchored by
    // `loader::tests::all_studio_panels_lower` (14 live panels). Zero coverage lost.

    #[test]
    fn kit_roundtrip_names_and_material_lower_to_nodes() {
        // Sweep 2/3 E2E: a .kit.vixi names canonical inventory widgets and authors a
        // material on one. After parse + lower, the WidgetNames survive (the inventory
        // round-trip the render dispatch keys on) and the material attr reaches the
        // lowered node (→ panel_material_from_name → render_widget → SetMaterial).
        use crate::layout::{lower, TokenCtx};
        use crate::ir::WidgetName;
        let src = "\
#vixi:kit v1
surface: test
slot root kind=region layout=stack_v
slot root.go kind=widget name=button material=bronze
slot root.vol kind=widget name=slider
slot root.cap kind=widget name=label
";
        let doc = parse_kit(src).expect("kit parses");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 400_000, 300_000), &TokenCtx::comfy(), 1);

        let names: Vec<&str> = ui
            .widgets
            .iter()
            .filter_map(|w| w.widget_name.as_ref().map(|n| n.0.as_str()))
            .collect();
        for want in ["button", "slider", "label"] {
            assert!(names.contains(&want), "inventory name '{want}' survives lowering (round-trip)");
        }

        let button_node = ui
            .widgets
            .iter()
            .find(|w| w.widget_name == Some(WidgetName("button".into())))
            .expect("button node present");
        assert_eq!(
            button_node.material.as_deref(),
            Some("bronze"),
            "material=bronze attr lowers onto the WidgetNode (→ PanelMaterial::Bronze at render)"
        );
    }

    #[test]
    fn bakes_all_five_artist_keyword_families() {
        // One slate authoring all 5 keyword families; every value resolves AOT.
        let src = "#vixi:kit v1\nsurface: phys\n\
slot root kind=region layout=stack_v\n\
slot root.slate kind=chrome material=stone mass=8500 friction=8000 vibe_scale=1 vibe_glow=0 screen_edge=left blend=overlay motion=snap_fluid attractor=pointer on_click=edict:surge_hostile_crimson on_key=ctrl_m:edict:surge_hostile_crimson\n";
        let doc = parse_kit(src).expect("parses");
        let b = doc.baked.iter().find(|b| b.stable_key == "root.slate").expect("slate is baked");
        let a = &b.attrs;
        // 1. material / mass / friction → physical atom + overrides
        let atom = a.material.expect("material atom baked");
        assert_eq!(atom.material, forge_correspondence_v3::correspondence::Material::Stone);
        assert_eq!(atom.mohs_x10, 65, "Stone bakes its real Mohs");
        assert_eq!(a.mass_pmy, Some(8500));
        assert_eq!(a.friction_pmy, Some(8000));
        assert_eq!(a.effective_mass_pmy(), Some(8500), "explicit mass override wins");
        // 2. synesthetic binders → VibeMatrix channels
        assert!(a.vibe.iter().any(|v| v.channel == 1 && v.target == VibeTarget::ScaleRadius));
        assert!(a.vibe.iter().any(|v| v.channel == 0 && v.target == VibeTarget::EmissiveGlow));
        // 3. chromatic reflection (ambilight)
        assert_eq!(a.screen_edge, Some(baked::ScreenEdge::Left));
        assert_eq!(a.blend, Some(baked::BlendMode::Overlay));
        // 4. deterministic spring kinematics
        assert!(a.motion.is_some(), "motion preset baked");
        assert!(a.motion_attractor_pointer, "attractor=pointer baked");
        // 5. semantic edict triggers — click + key chord (ctrl_m → CTRL bit16 | VK 'M' 0x4D)
        assert_eq!(a.on_click_edict.as_deref(), Some("surge_hostile_crimson"));
        assert_eq!(
            a.on_key_edict,
            Some(((1 << 16) | 0x4D, "surge_hostile_crimson".to_string()))
        );
    }

    #[test]
    fn slot_without_keywords_has_no_baked_entry() {
        let doc = parse_kit("#vixi:kit v1\nslot root kind=region layout=stack_v\n").unwrap();
        assert!(doc.baked.is_empty(), "no keywords → no baked entries (contiguous, no holes)");
    }
}
