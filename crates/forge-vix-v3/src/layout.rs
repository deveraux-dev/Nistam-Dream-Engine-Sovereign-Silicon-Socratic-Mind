//! # layout.rs — AOT UI Lowering: WidgetSpec → LoweredUi (token-driven)
//!
//! Task 3a, realigned to canon. The engine consumes a declarative `WidgetSpec`
//! tree (what `.kit.vixi` parses to — `parse` mod, 3b) plus a `TokenCtx` of
//! resolved sizing tokens, and computes the integer `LayoutIR` + derives the
//! other planes (spec `core_invariant`).
//!
//! **Sizing is token-driven, not explicit `w/h`** (`token_taxonomy.md`):
//! - region spacing/padding ← `density` base spacing
//! - widget intrinsic height ← `type.ramp[body] + 2·density` (PROVISIONAL v1;
//!   real widget intrinsic sizes land with the token store — see
//!   `tokens_inventory.md`, 7 categories currently unimplemented)
//! - `Fill` takes the parent content rect; `Fixed` is an escape hatch.
//!
//! Integer-only (i64 MilliUnit). `Hug` (size-to-content) needs a measure pass
//! and is deferred.

use super::ir::*;

/// Resolved sizing tokens (the values a `.vibe.vixi` cascade would produce).
/// Passed in so `layout` does not depend on the (incomplete) token store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenCtx {
    /// `density` base spacing (MilliUnit). comfy = 12000 (12px).
    pub density_base: i64,
    /// `type.ramp[0..4]` text sizes (MilliUnit): caption/body/subhead/head/display.
    pub ramp: [i64; 5],
    /// `chrome.thickness` (MilliUnit).
    pub chrome_thickness: i64,
    /// `motion.snap` dials — the resolved spring tuning a panel seeds its
    /// `IntegerSpring`s from. Projected from `DesignTokens.motion`.
    pub motion: crate::tokens::MotionSnap,
    /// The face each `ramp` stop speaks in. Sizes alone were never a type ramp —
    /// one face at five sizes is one voice. `comfy` = the studio ladder; a
    /// fixed-advance surface resolves `MONO_RAMP_FACES` via `live::ctx_for`.
    pub faces: [forge_canvas_v3::text::TypeFace; 5],
}

impl TokenCtx {
    /// The `comfy` default density + a plain ramp. Used by tests and as a floor.
    pub fn comfy() -> Self {
        Self {
            density_base: 12_000,
            ramp: [12_000, 16_000, 20_000, 28_000, 40_000],
            chrome_thickness: 1_000,
            motion: crate::tokens::MotionSnap::comfy(),
            faces: forge_canvas_v3::text::RAMP_FACES,
        }
    }
    /// Provisional widget intrinsic height: body text + symmetric vertical pad.
    fn intrinsic_height(&self) -> i64 {
        self.ramp[1] + 2 * self.density_base
    }
}

/// Sizing intent for a slot. Token-driven by default (`Intrinsic`); `Fill`
/// takes the parent content rect; `Hug` sizes to content (two-pass measure);
/// `Fixed` is an explicit escape hatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sizing {
    /// Token-driven default — resolves from the type ramp / density.
    Intrinsic,
    /// Takes the parent's remaining content rect.
    Fill,
    /// Size-to-content. Hugs both axes; in a stack the cross-axis still fills.
    Hug,
    /// Explicit fixed extent on both axes.
    Fixed {
        /// Fixed width, MilliUnit.
        w: i64,
        /// Fixed height, MilliUnit.
        h: i64,
    },
    /// Fixed extent (i64 MilliUnit) along the parent's MAIN axis — width in a
    /// `stack_h`, height in a `stack_v` — while the cross axis fills. The slot
    /// graph authors this with `size=mu(N)`; it lets a rail/bar take a fixed
    /// thickness so a sibling `Fill` takes the rest. (`Fixed{w,h}` pins both
    /// axes and is unauthorable; this is the rail/bar case.)
    FixedMain(i64),
}

/// The brush FIGURE, carried from the parser to the emitters instead of dropped.
///
/// A brush could already say what colour it was and which audio bus fed it, and had no way
/// to say WHAT IT DRAWS — so every figure lived in a hand-Rust strangler while the kit
/// stated an intent no runtime read. Integer only: `radius_pmy` is PERMYRIAD of the parent's
/// short side, because a plate circle is a RATIO of its plate and never a pixel count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrushSlot {
    /// What geometric figure this brush draws.
    pub shape: crate::parse::BrushShape,
    /// 0..=10_000, range-checked at parse (`parse.rs:645`).
    pub radius_pmy: u16,
    /// `Fixed` = the mater does not turn; `Tick` advances on SimTick, NEVER a wall clock.
    pub phase: crate::parse::BrushPhase,
    /// How siblings spread: `Weyl` low-discrepancy, or `Even` equal arcs.
    pub stride: crate::parse::BrushStride,
}

/// The declarative authored tree (pre-layout). `.kit.vixi` lowers to this.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetSpec {
    /// This slot's stable identity key (the dotted `.kit.vixi` name).
    pub stable_key: String,
    /// What this slot renders as.
    pub kind: SlotKind,
    /// Inventory name for `kind == Widget` (or the element type of a `SlotList`).
    pub widget_name: Option<WidgetName>,
    /// Layout policy for `kind == Region`.
    pub layout: Option<LayoutPolicy>,
    /// How this slot's extent is determined.
    pub sizing: Sizing,
    /// Column count for `LayoutPolicy::Grid` (ignored by other policies).
    pub grid_cols: Option<u16>,
    /// Bounded length for `kind == SlotList` (hot-path size stability).
    pub slot_list_max: Option<u16>,
    /// Direct children, in source order.
    pub children: Vec<WidgetSpec>,
    // ── Smithy substrate Patch 6 affordance flags ────────────────────────────
    /// `hover_reveal=true` — chrome/content only paints on hover.
    pub hover_reveal: bool,
    /// `long_press_drawer=true` — this slot opens as a long-press drawer.
    pub long_press_drawer: bool,
    /// `collapsible=true` — this region can collapse to zero cross-extent.
    pub collapsible: bool,
    /// `audio_reactive=true` — this slot's paint responds to the audio bus.
    pub audio_reactive: bool,
    /// What a `kind=brush` slot DRAWS. `parse.rs` has typechecked `shape`/`radius`/`phase`/
    /// `stride` since 08-02 and `to_spec` dropped all four on the floor — the kit stated a
    /// figure and no emitter could ever read it (the accepted-by-ignore trap the grammar's
    /// own comment predicted). `None` for every non-brush kind.
    pub brush: Option<BrushSlot>,
    /// Optional GPU material authored via `.kit.vixi` (`material=bronze`), mapped
    /// to a `PanelMaterial` by the forge-canvas render dispatch (uber-shader axis).
    /// `None` = inherit the surrounding panel's material.
    pub material: Option<String>,
    /// Authored `text="…"` — the slot's WORDS, owned by the kit (root#vixi-t1).
    /// `None` = the host supplies a live value (a count, a name, a reading).
    /// Static labels belong here, never re-declared in a host's Rust const table.
    pub text: Option<String>,
    /// Authored `gap=mu(N)` between this region's children (None = density base).
    pub gap: Option<i64>,
    /// Authored `padding=mu(N)` inside this region (None = density base).
    pub padding: Option<i64>,
    /// Authored `min_size=mu(N)` / `max_size=mu(N)` — MAIN-axis floor and ceiling
    /// applied by `clamp_main` (private) after the sizing policy resolves an extent.
    /// `None` = unclamped on that side.
    pub min_size: Option<i64>,
    /// MAIN-axis clamp — see `clamp_main` (private).
    pub max_size: Option<i64>,
    /// Authored `margin=mu(N)` — outer gap around this slot, consuming space from
    /// the PARENT's flow (unlike `padding`, which insets this slot's own children).
    /// `None` = 0.
    ///
    /// The semantics are drained from `forge_viewport_host::layout` (`LayoutNode::margin`,
    /// inset applied before sizing resolves) rather than re-invented — that tree is a
    /// second-truth layout engine with zero callers (re-exported at its `lib.rs:47`,
    /// and `forge-studio/src/main.rs:1327` reaches only `forge_viewport_host::run`),
    /// which is why the behaviour lands HERE, in the engine the studio actually runs.
    pub margin: Option<i64>,
    /// Authored `justify=` / `align=` — MAIN-axis distribution and CROSS-axis
    /// placement of THIS region's children. `None` = `Start` / `Stretch`, the
    /// behaviour every surface got while the pair was accepted-by-ignore.
    pub justify: Option<Justify>,
    /// Authored `align=` — CROSS-axis sizing/placement. `None` = `Stretch`.
    pub align: Option<Align>,
    /// Authored `color=palette.<slot>` — the palette slot NAME, never a hex
    /// literal: the profile sheet still owns the colour (forge-gui#colour-resolution),
    /// the kit only names which slot it wants. `None` = the runtime's kind default.
    pub chrome_color: Option<String>,
    /// Authored `border_radius=mu(N)` corner rounding. `None` = the runtime default.
    pub border_radius: Option<i64>,
    /// Authored `alpha=permyriad(0..10000)`, carried to [`WidgetNode::style_atom`].
    pub alpha_pmy: Option<u16>,
    /// Authored `font=<family token>`, carried to [`WidgetNode::style_atom`].
    pub font: Option<String>,
    /// Authored `semantic=<ramp>`, carried to [`WidgetNode::style_atom`].
    pub semantic: Option<String>,
}

impl Default for WidgetSpec {
    /// The zero spec: an unnamed intrinsic chrome leaf carrying nothing authored.
    ///
    /// Every constructor below used to spell this out — fifteen `None`/`false` lines each,
    /// four times over, plus the `build.rs` emitter. Adding ONE field (`brush`, 08-02) cost
    /// six patches found one compiler error at a time. With this, the next field costs one
    /// line here and nothing anywhere else (`forge_book::oracle1_governor`
    /// `MASSLOOP_EXECUTION_LAW` "ZERO_STRUCT_IS_DEFAULT" — this constructor IS its receipt).
    fn default() -> Self {
        Self {
            stable_key: String::new(),
            kind: SlotKind::Chrome,
            widget_name: None,
            layout: None,
            sizing: Sizing::Intrinsic,
            grid_cols: None,
            slot_list_max: None,
            children: vec![],
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            brush: None,
            material: None,
            text: None,
            gap: None,
            padding: None,
            min_size: None,
            max_size: None,
            margin: None,
            justify: None,
            align: None,
            chrome_color: None,
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
        }
    }
}

impl WidgetSpec {
    /// Convenience: a `kind=widget name=<name>` leaf with intrinsic sizing.
    pub fn widget(stable_key: &str, name: &str) -> Self {
        Self {
            stable_key: stable_key.into(),
            kind: SlotKind::Widget,
            widget_name: Some(WidgetName(name.into())),
            ..Self::default()
        }
    }
    /// Convenience: a `kind=region` container with a layout policy.
    pub fn region(stable_key: &str, policy: LayoutPolicy, sizing: Sizing, children: Vec<WidgetSpec>) -> Self {
        Self {
            stable_key: stable_key.into(),
            kind: SlotKind::Region,
            layout: Some(policy),
            sizing,
            children,
            ..Self::default()
        }
    }
    /// Convenience: a `LayoutPolicy::Grid` region with `cols` columns.
    pub fn grid(stable_key: &str, cols: u16, sizing: Sizing, children: Vec<WidgetSpec>) -> Self {
        Self {
            stable_key: stable_key.into(),
            kind: SlotKind::Region,
            layout: Some(LayoutPolicy::Grid),
            sizing,
            grid_cols: Some(cols.max(1)),
            children,
            ..Self::default()
        }
    }
    /// Convenience: a bounded `kind=slot_list of=<name> max=<n>` (children stack).
    pub fn slot_list(stable_key: &str, of_name: &str, max: u16, children: Vec<WidgetSpec>) -> Self {
        Self {
            stable_key: stable_key.into(),
            kind: SlotKind::SlotList,
            widget_name: Some(WidgetName(of_name.into())),
            layout: Some(LayoutPolicy::StackV),
            sizing: Sizing::Hug,
            slot_list_max: Some(max),
            children,
            ..Self::default()
        }
    }
}

/// `justify=` — how a region distributes its children along the MAIN axis when
/// they do not fill it. Declared in `grammar::SLOT_ATTRS` since v1 (grammar.rs:101)
/// and reaching no parser until 2026-08-04, which is why every authored surface
/// packed to the start no matter what it said.
///
/// No effect when a `Fill` sibling is present — Fill has already eaten the slack,
/// which is the same rule CSS applies to a `flex-grow` child.
///
/// Defined in `forge_vix_syntax_v3::tables` (2026-08-05) so the string↔variant
/// mapping is one table row shared by parse/AOT/unlower/LSP.
pub use forge_vix_syntax_v3::Justify;

/// `align=` — how a region sizes and places its children on the CROSS axis.
/// `Stretch` (the default, and the only behaviour before 2026-08-04) fills the
/// cross extent; the other three hug the measured extent and place it.
/// Defined in `forge_vix_syntax_v3::tables` (one-table-row law, 2026-08-05).
pub use forge_vix_syntax_v3::Align;

/// MAIN-axis distribution: the leading offset before the first child and the extra
/// space inserted between every adjacent pair, given `free` slack over `n` children.
///
/// Integer division, remainder dropped — the lattice is MilliUnit and a sub-unit
/// residue is beneath the smallest thing the solver can place (root#substrate).
fn distribute(justify: Justify, free: i64, n: i64) -> (i64, i64) {
    if free <= 0 || n <= 0 {
        return (0, 0);
    }
    match justify {
        Justify::Start => (0, 0),
        Justify::Center => (free / 2, 0),
        Justify::End => (free, 0),
        // One child has no between-pair, so space_between degenerates to start —
        // CSS resolves it the same way, and it keeps a single row from drifting.
        Justify::SpaceBetween if n > 1 => (0, free / (n - 1)),
        Justify::SpaceBetween => (0, 0),
        // Half a share leads and trails, a full share between: the classic
        // space-around split, done in integers.
        Justify::SpaceAround => (free / (2 * n), free / n),
    }
}

/// CROSS-axis placement: the offset that puts an `extent`-wide child inside a
/// `content`-wide track under `align`.
fn cross_offset(align: Align, content: i64, extent: i64) -> i64 {
    let slack = (content - extent).max(0);
    match align {
        Align::Start | Align::Stretch => 0,
        Align::Center => slack / 2,
        Align::End => slack,
    }
}

/// Clamp a resolved MAIN-axis extent to the slot's authored `min_size`/`max_size`.
///
/// The floor wins a contradictory pair (`min > max`), same as CSS: a slot that says it
/// needs 96mu to be legible does not become illegible because a ceiling was authored
/// under it. Absent on either side = unclamped on that side.
fn clamp_main(spec: &WidgetSpec, extent: i64) -> i64 {
    let capped = match spec.max_size {
        Some(max) => extent.min(max),
        None => extent,
    };
    match spec.min_size {
        Some(min) => capped.max(min),
        None => capped,
    }
}

/// Per-lowering mutable context: the accumulating `LoweredUi`, the id counter,
/// the resolved tokens, and the layout version. Bundled so `place` stays small.
struct LowerCtx<'a> {
    ui: LoweredUi,
    next_id: u32,
    tokens: &'a TokenCtx,
    version: u32,
}

/// A parent-resolved placement request for one node.
struct Slot {
    x: i64,
    y: i64,
    avail_w: i64,
    avail_h: i64,
    /// When `Some`, the parent's layout policy already resolved this box.
    forced: Option<(i64, i64)>,
    parent: Option<WidgetId>,
}

/// Fan out a bounded, EMPTY `slot_list`: a `kind=slot_list of=<name> max=n` that authored
/// no children gets `n` synthetic `of=<name>` widget cells, so the list renders its full
/// bounded inventory (the picker grid's 64 swatches, an empty rail's rows) instead of
/// Hug-collapsing to nothing. A slot_list that DID author children is left exactly as
/// written — the caller supplied the real rows, so a partially-filled list never gets
/// padded (that would shift every existing panel + its readback proof). Recurses the whole
/// tree; deterministic + cold-path (once per lower, never per frame).
fn fan_out_slot_lists(spec: &WidgetSpec) -> WidgetSpec {
    let mut out = spec.clone();
    out.children = spec.children.iter().map(fan_out_slot_lists).collect();
    if out.kind == SlotKind::SlotList && out.children.is_empty() {
        if let Some(max) = out.slot_list_max {
            let of = out.widget_name.as_ref().map(|w| w.0.clone()).unwrap_or_else(|| "button".into());
            out.children = (0..max as usize)
                .map(|i| WidgetSpec::widget(&format!("{}.cell{i}", out.stable_key), &of))
                .collect();
        }
    }
    out
}

/// Lower a declarative tree + resolved tokens into a fully-derived `LoweredUi`.
pub fn lower(root: &WidgetSpec, viewport: IrRect, ctx: &TokenCtx, version: u32) -> LoweredUi {
    let mut lc = LowerCtx {
        ui: LoweredUi { layout_version: version, ..Default::default() },
        next_id: 0,
        tokens: ctx,
        version,
    };
    let avail_w = viewport.max_x - viewport.min_x;
    let avail_h = viewport.max_y - viewport.min_y;
    // Bounded empty slot_lists fan out to their full inventory before layout.
    let expanded = fan_out_slot_lists(root);
    // forced=None → the root sizes from its own `sizing` against the viewport.
    place(
        &mut lc,
        &expanded,
        Slot { x: viewport.min_x, y: viewport.min_y, avail_w, avail_h, forced: None, parent: None },
    );
    link_focus(&mut lc.ui);
    lc.ui
}

/// Provisional leaf intrinsic width (token-derived ≈ 8 spacing units). Real
/// text-fit needs glyph metrics — a forge-canvas concern; forge-vix sits below
/// text shaping, so it sizes leaves to a token default.
fn leaf_intrinsic_w(ctx: &TokenCtx) -> i64 {
    ctx.density_base * 8
}

/// Two-pass `Hug` measure: a spec's intrinsic content size. `Fill` has no
/// intrinsic extent (returns `(0, 0)` — it grows to its container instead).
fn measure(spec: &WidgetSpec, ctx: &TokenCtx) -> (i64, i64) {
    match spec.sizing {
        Sizing::Fixed { w, h } => return (w, h),
        // FixedMain's extent is parent-axis-relative (resolved in the stack
        // arm); like Fill it contributes no intrinsic content size to a Hug.
        Sizing::Fill | Sizing::FixedMain(_) => return (0, 0),
        Sizing::Intrinsic | Sizing::Hug => {}
    }
    match spec.layout {
        // Leaf: token-derived intrinsic box.
        None => (leaf_intrinsic_w(ctx), ctx.intrinsic_height()),
        // Region: combine child measures per policy, then add padding.
        Some(policy) => {
            let pad = spec.padding.unwrap_or(ctx.density_base);
            let gap = spec.gap.unwrap_or(ctx.density_base);
            let n = spec.children.len() as i64;
            let (mut sum_w, mut sum_h, mut max_w, mut max_h) = (0i64, 0i64, 0i64, 0i64);
            for c in &spec.children {
                let (mut cw, mut ch) = measure(c, ctx);
                // A FixedMain child's extent rides THIS region's main axis — the
                // old (0,0) measure made any Hug/Intrinsic region holding
                // `size=mu(N)` children under-measure and its children overflow
                // over the next sibling (the overlapping paint-rail bug).
                if let Sizing::FixedMain(k) = c.sizing {
                    match policy {
                        LayoutPolicy::StackV => ch = k,
                        LayoutPolicy::StackH | LayoutPolicy::Flow => {
                            cw = k;
                            ch = ctx.intrinsic_height();
                        }
                        _ => {}
                    }
                }
                sum_w += cw;
                sum_h += ch;
                max_w = max_w.max(cw);
                max_h = max_h.max(ch);
            }
            let gaps = if n > 0 { gap * (n - 1) } else { 0 };
            let (content_w, content_h) = match policy {
                LayoutPolicy::StackV => (max_w, sum_h + gaps),
                LayoutPolicy::StackH => (sum_w + gaps, max_h),
                LayoutPolicy::Overlay => (max_w, max_h),
                // Flow measured unwrapped (a measure needs no wrap width).
                LayoutPolicy::Flow => (sum_w + gaps, max_h),
                LayoutPolicy::Grid => {
                    let cols = spec.grid_cols.unwrap_or(1).max(1) as i64;
                    let rows = (n + cols - 1) / cols;
                    let gw = if cols > 1 { gap * (cols - 1) } else { 0 };
                    let gh = if rows > 1 { gap * (rows - 1) } else { 0 };
                    (cols * max_w + gw, rows * max_h + gh)
                }
                LayoutPolicy::HexGrid => {
                    let hex_r = spec.grid_cols.unwrap_or(44) as i64 * 1000;
                    let hex_w = hex_r * 2;
                    let hex_h = hex_r * 1732 / 1000;
                    let row_h = hex_h * 3 / 4;
                    let cols = if hex_w > 0 { (max_w / hex_w).max(1) } else { 1 };
                    let rows = (n + cols - 1) / cols;
                    (cols * hex_w + hex_w / 2, rows * row_h + hex_h / 4)
                }
            };
            (content_w + 2 * pad, content_h + 2 * pad)
        }
    }
}

fn place(lc: &mut LowerCtx, spec: &WidgetSpec, slot: Slot) -> (WidgetId, i64, i64) {
    let id = WidgetId(lc.next_id);
    lc.next_id += 1;
    let ctx = lc.tokens;
    let version = lc.version;

    let (w, h) = match slot.forced {
        Some(box_) => box_,
        None => match spec.sizing {
            Sizing::Fixed { w, h } => (w, h),
            // FixedMain with no parent policy (unforced / root) has no main axis to
            // resolve against — fill the available box like Fill.
            Sizing::Fill | Sizing::FixedMain(_) => (slot.avail_w, slot.avail_h),
            Sizing::Hug => measure(spec, ctx),
            // Intrinsic: fill cross-axis width, token-derived height.
            Sizing::Intrinsic => (slot.avail_w, ctx.intrinsic_height()),
        },
    };
    let rect = IrRect::from_xywh(slot.x, slot.y, w, h);
    let z = id.0 as i32;

    let node = WidgetNode {
        id,
        stable_key: StableKey(spec.stable_key.clone()),
        kind: spec.kind,
        widget_name: spec.widget_name.clone(),
        layout: spec.layout,
        slot_list_max: spec.slot_list_max,
        parent: slot.parent,
        hover_reveal: spec.hover_reveal,
        long_press_drawer: spec.long_press_drawer,
        collapsible: spec.collapsible,
        audio_reactive: spec.audio_reactive,
        brush: spec.brush,
        material: spec.material.clone(),
        chrome_color: spec.chrome_color.clone(),
        border_radius: spec.border_radius,
        alpha_pmy: spec.alpha_pmy,
        font: spec.font.clone(),
        semantic: spec.semantic.clone(),
    };
    let (accepts_pointer, accepts_keyboard, is_text) =
        (node.accepts_pointer(), node.accepts_keyboard(), node.is_text_input());
    lc.ui.widgets.push(node);

    lc.ui.layout.push(LayoutBox {
        widget_id: id,
        stable_key: StableKey(spec.stable_key.clone()),
        rect,
        z,
        clip_id: None,
        scroll_id: None,
        baseline: None,
        layout_version: version,
    });

    if accepts_pointer {
        let mut accepts = HitAccept::Pointer.bit();
        if accepts_keyboard {
            accepts |= HitAccept::Focus.bit();
        }
        if is_text {
            accepts |= HitAccept::Text.bit();
        }
        lc.ui.hits.push(HitRegion {
            widget_id: id,
            rect,
            z,
            accepts,
            disabled_policy: DisabledPolicy::Block,
            layout_version: version,
        });
    }

    if accepts_keyboard {
        lc.ui.focus.push(FocusNode {
            widget_id: id,
            stable_key: StableKey(spec.stable_key.clone()),
            tab_index: None,
            accepts_keyboard: true,
            focus_group: None,
            next: None,
            prev: None,
            restore_priority: RestorePriority::ExactKey,
            layout_version: version,
        });
    }

    if is_text {
        lc.ui.text_inputs.push(TextInputDecl {
            widget_id: id,
            stable_key: StableKey(spec.stable_key.clone()),
            text_buffer_id: id.0,
            rect,
            max_chars: 256,
            caret_policy: CaretPolicy::SingleLine,
            selection_policy: SelectionPolicy::Range,
            ime_policy: ImePolicy::Enabled,
            layout_version: version,
        });
    }

    lc.ui.draws.push(DrawCmd {
        cmd_id: id.0,
        widget_id: id,
        bounds: rect,
        clip_id: None,
        token_status: TokenStatus::Resolved,
        token_id: None,
        layout_version: version,
        render_version: version,
    });

    // Lay out children per policy. Padding + gap come from density. The parent
    // resolves each child's box (cross-axis fill, main-axis from child sizing)
    // and passes it down as `forced`, so child `Sizing` semantics are honored
    // without a child needing to know its parent's policy.
    if let Some(policy) = spec.layout {
        let pad = spec.padding.unwrap_or(ctx.density_base);
        let gap = spec.gap.unwrap_or(ctx.density_base);
        let cx0 = slot.x + pad;
        let cy0 = slot.y + pad;
        let content_w = (w - 2 * pad).max(0);
        let content_h = (h - 2 * pad).max(0);

        match policy {
            LayoutPolicy::StackV => {
                // Pre-pass: reserve the main-axis (height) taken by every
                // fixed-extent sibling (Fixed / FixedMain / Hug-or-Intrinsic
                // measured), then split the remainder evenly among the Fill
                // children. Order-independent — a Fill before a fixed rail/bar no
                // longer steals its space (the canvas-fold full-bleed bug).
                let n = spec.children.len() as i64;
                let total_gap = gap * (n - 1).max(0);
                let mut fixed_h = 0;
                let mut fill_n = 0;
                for child in &spec.children {
                    match child.sizing {
                        Sizing::Fixed { h, .. } => fixed_h += clamp_main(child, h),
                        Sizing::FixedMain(k) => fixed_h += clamp_main(child, k),
                        Sizing::Fill => fill_n += 1,
                        Sizing::Intrinsic | Sizing::Hug => {
                            fixed_h += clamp_main(child, measure(child, ctx).1)
                        }
                    }
                }
                // `margin=` consumes the PARENT's flow on both sides of the child, so
                // it is reserved before Fill splits the remainder — otherwise a margined
                // sibling pushes the last child past the content edge.
                let total_margin: i64 =
                    spec.children.iter().map(|c| 2 * c.margin.unwrap_or(0)).sum();
                let fixed_h = fixed_h + total_margin;
                let fill_h = if fill_n > 0 { (content_h - fixed_h - total_gap).max(0) / fill_n } else { 0 };
                // Slack left over after every child took its extent. A Fill sibling
                // has already absorbed it, so `justify` only moves a track of
                // fixed/hug children — the same rule flex-grow imposes in CSS.
                let free = if fill_n > 0 { 0 } else { (content_h - fixed_h - total_gap).max(0) };
                let (lead, extra) = distribute(spec.justify.unwrap_or_default(), free, n);
                let align = spec.align.unwrap_or_default();
                let mut cursor = lead;
                for child in &spec.children {
                    let (mw, mh) = measure(child, ctx);
                    let m = child.margin.unwrap_or(0);
                    let track_w = (content_w - 2 * m).max(0);
                    let cw = match child.sizing {
                        Sizing::Fixed { w, .. } => w,
                        Sizing::Hug => mw,
                        // Cross-axis under a non-Stretch align: only INTRINSIC hugs its
                        // measured extent. Fill means fill, and `measure` returns (0,0)
                        // for Fill/FixedMain by design (their extent is parent-relative),
                        // so hugging those authors a 0-extent unclickable box — the exact
                        // dead-button defect loader.rs:574 guards.
                        Sizing::Intrinsic if align != Align::Stretch => mw.min(track_w),
                        _ => track_w, // Fill / Intrinsic-stretched / FixedMain
                    };
                    let ch = clamp_main(child, match child.sizing {
                        Sizing::Fixed { h, .. } => h,
                        Sizing::FixedMain(k) => k,
                        Sizing::Fill => fill_h,
                        Sizing::Intrinsic | Sizing::Hug => mh,
                    });
                    cursor += m;
                    place(lc, child, Slot {
                        x: cx0 + m + cross_offset(align, track_w, cw),
                        y: cy0 + cursor,
                        avail_w: track_w, avail_h: (content_h - cursor).max(0),
                        forced: Some((cw, ch)), parent: Some(id),
                    });
                    cursor += ch + m + gap + extra;
                }
            }
            LayoutPolicy::StackH => {
                // Pre-pass (see StackV): reserve the main-axis (width) of every
                // fixed-extent sibling, then split the remainder evenly among Fill
                // children — so a Fill artboard before a fixed right-rail no longer
                // eats the rail's width.
                let n = spec.children.len() as i64;
                let total_gap = gap * (n - 1).max(0);
                let mut fixed_w = 0;
                let mut fill_n = 0;
                for child in &spec.children {
                    match child.sizing {
                        Sizing::Fixed { w, .. } => fixed_w += clamp_main(child, w),
                        Sizing::FixedMain(k) => fixed_w += clamp_main(child, k),
                        Sizing::Fill => fill_n += 1,
                        Sizing::Intrinsic | Sizing::Hug => {
                            fixed_w += clamp_main(child, measure(child, ctx).0)
                        }
                    }
                }
                // See StackV: margin is reserved out of the parent's flow first.
                let total_margin: i64 =
                    spec.children.iter().map(|c| 2 * c.margin.unwrap_or(0)).sum();
                let fixed_w = fixed_w + total_margin;
                let fill_w = if fill_n > 0 { (content_w - fixed_w - total_gap).max(0) / fill_n } else { 0 };
                // See StackV: Fill absorbs the slack, so justify moves only a track
                // of fixed/hug children.
                let free = if fill_n > 0 { 0 } else { (content_w - fixed_w - total_gap).max(0) };
                let (lead, extra) = distribute(spec.justify.unwrap_or_default(), free, n);
                let align = spec.align.unwrap_or_default();
                let mut cursor = lead;
                for child in &spec.children {
                    let (mw, mh) = measure(child, ctx);
                    let m = child.margin.unwrap_or(0);
                    let track_h = (content_h - 2 * m).max(0);
                    let cw = clamp_main(child, match child.sizing {
                        Sizing::Fixed { w, .. } => w,
                        Sizing::FixedMain(k) => k,
                        Sizing::Fill => fill_w,
                        Sizing::Intrinsic | Sizing::Hug => mw,
                    });
                    let ch = match child.sizing {
                        Sizing::Fixed { h, .. } => h,
                        Sizing::Hug => mh,
                        // See StackV: only Intrinsic hugs the cross axis.
                        Sizing::Intrinsic if align != Align::Stretch => mh.min(track_h),
                        _ => track_h, // Fill / Intrinsic-stretched / FixedMain
                    };
                    cursor += m;
                    place(lc, child, Slot {
                        x: cx0 + cursor,
                        y: cy0 + m + cross_offset(align, track_h, ch),
                        avail_w: (content_w - cursor).max(0), avail_h: track_h,
                        forced: Some((cw, ch)), parent: Some(id),
                    });
                    cursor += cw + m + gap + extra;
                }
            }
            LayoutPolicy::Overlay => {
                // All children share the content origin; z-order follows id.
                // `margin=` insets a child from that origin on all four sides —
                // the same parent-flow reservation StackV/StackH make above. It
                // was accepted-by-ignore here until 2026-08-26, so an authored
                // overlay plane sat flush on the content edge no matter what it
                // declared (`shell/panels/launcher.kit.vixi`'s rete, found there).
                for child in &spec.children {
                    let (mw, mh) = measure(child, ctx);
                    let m = child.margin.unwrap_or(0);
                    let track_w = (content_w - 2 * m).max(0);
                    let track_h = (content_h - 2 * m).max(0);
                    let cw = match child.sizing {
                        Sizing::Fixed { w, .. } => w,
                        Sizing::Hug => mw,
                        _ => track_w,
                    };
                    let ch = match child.sizing {
                        Sizing::Fixed { h, .. } => h,
                        Sizing::Hug => mh,
                        _ => track_h,
                    };
                    place(lc, child, Slot {
                        x: cx0 + m, y: cy0 + m, avail_w: track_w, avail_h: track_h,
                        forced: Some((cw, ch)), parent: Some(id),
                    });
                }
            }
            LayoutPolicy::Grid => {
                let cols = spec.grid_cols.unwrap_or(1).max(1) as i64;
                let n = spec.children.len() as i64;
                let rows = if n > 0 { (n + cols - 1) / cols } else { 0 };
                let cell_w = (content_w - gap * (cols - 1).max(0)).max(0) / cols;
                let cell_h = if rows > 0 {
                    (content_h - gap * (rows - 1).max(0)).max(0) / rows
                } else {
                    content_h
                };
                for (i, child) in spec.children.iter().enumerate() {
                    let col = i as i64 % cols;
                    let row = i as i64 / cols;
                    let cxp = cx0 + col * (cell_w + gap);
                    let cyp = cy0 + row * (cell_h + gap);
                    place(lc, child, Slot {
                        x: cxp, y: cyp, avail_w: cell_w, avail_h: cell_h,
                        forced: Some((cell_w, cell_h)), parent: Some(id),
                    });
                }
            }
            LayoutPolicy::Flow => {
                let (mut cursor_x, mut cursor_y, mut line_h) = (0i64, 0i64, 0i64);
                for child in &spec.children {
                    let (mw0, mh0) = measure(child, ctx);
                    // Resolve the child's box honoring its Sizing — a `measure()` pass
                    // returns (0,0) for FixedMain/Fill (their extent is parent-axis
                    // relative), so a leaf authored `size=mu(N)` in a FLOW region would
                    // otherwise collapse to a 0x0, UNCLICKABLE box: the dead "Surprise
                    // me" icon_button (root.face.vibe.random), 2026-07-12. FLOW's main
                    // axis is horizontal, so FixedMain pins the width; the cross-axis
                    // (height) takes the token intrinsic. stack_v/stack_h already do this.
                    let (cw, ch) = match child.sizing {
                        Sizing::Fixed { w, h } => (w, h),
                        Sizing::FixedMain(k) => (k, ctx.intrinsic_height()),
                        Sizing::Fill => (content_w, ctx.intrinsic_height()),
                        Sizing::Intrinsic | Sizing::Hug => (mw0, mh0),
                    };
                    let mw = cw.min(content_w);
                    if cursor_x > 0 && cursor_x + mw > content_w {
                        cursor_x = 0;
                        cursor_y += line_h + gap;
                        line_h = 0;
                    }
                    place(lc, child, Slot {
                        x: cx0 + cursor_x, y: cy0 + cursor_y, avail_w: mw, avail_h: ch,
                        forced: Some((mw, ch)), parent: Some(id),
                    });
                    cursor_x += mw + gap;
                    line_h = line_h.max(ch);
                }
            }
            LayoutPolicy::HexGrid => {
                // Hexagonal tessellation: flat-top hex placement.
                // hex_size (from grid_cols field, reused) = cell radius in mu.
                // Even rows are offset by half a cell width for tessellation.
                let hex_r = spec.grid_cols.unwrap_or(44) as i64 * 1000; // default 44mu
                let hex_w = hex_r * 2;                    // full cell width
                let hex_h = hex_r * 1732 / 1000;          // √3 * r ≈ 1.732r (integer approx)
                let row_h = hex_h * 3 / 4;                // vertical stride (3/4 of hex height for overlap)
                let cols = (content_w / hex_w).max(1);
                for (i, child) in spec.children.iter().enumerate() {
                    let col = i as i64 % cols;
                    let row = i as i64 / cols;
                    let x_offset = if row % 2 == 1 { hex_w / 2 } else { 0 };
                    let cxp = cx0 + col * hex_w + x_offset;
                    let cyp = cy0 + row * row_h;
                    place(lc, child, Slot {
                        x: cxp, y: cyp, avail_w: hex_w, avail_h: hex_h,
                        forced: Some((hex_w, hex_h)), parent: Some(id),
                    });
                }
            }
        }
    }

    (id, w, h)
}

fn link_focus(ui: &mut LoweredUi) {
    let ids: Vec<WidgetId> = ui.focus.iter().map(|f| f.widget_id).collect();
    let n = ids.len();
    for (i, node) in ui.focus.iter_mut().enumerate() {
        node.tab_index = Some(i as i32);
        node.prev = if i > 0 { Some(ids[i - 1]) } else { None };
        node.next = if i + 1 < n { Some(ids[i + 1]) } else { None };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec `minimal_vertical_slice`, canonical kinds: region root → region
    /// panel → [widget button, widget text_field].
    fn slice() -> WidgetSpec {
        WidgetSpec::region(
            "root",
            LayoutPolicy::StackV,
            Sizing::Fill,
            vec![WidgetSpec::region(
                "panel",
                LayoutPolicy::StackV,
                Sizing::Fixed { w: 500_000, h: 300_000 },
                vec![
                    WidgetSpec::widget("apply", "button"),
                    WidgetSpec::widget("name", "text_field"),
                ],
            )],
        )
    }

    fn box_of(ui: &LoweredUi, id: u32) -> IrRect {
        ui.layout.iter().find(|b| b.widget_id == WidgetId(id)).unwrap().rect
    }

    #[test]
    fn layout_golden_token_driven() {
        let ui = lower(&slice(), IrRect::from_xywh(0, 0, 960_000, 540_000), &TokenCtx::comfy(), 1);
        // density=12000 pad/gap; intrinsic h = ramp[1](16000) + 2*12000 = 40000.
        assert_eq!(box_of(&ui, 0), IrRect::from_xywh(0, 0, 960_000, 540_000)); // root Fill
        assert_eq!(box_of(&ui, 1), IrRect::from_xywh(12_000, 12_000, 500_000, 300_000)); // panel @ root pad
        assert_eq!(box_of(&ui, 2), IrRect::from_xywh(24_000, 24_000, 476_000, 40_000)); // button: panel pad, fill-w, token-h
        assert_eq!(box_of(&ui, 3), IrRect::from_xywh(24_000, 76_000, 476_000, 40_000)); // text: + h(40000) + gap(12000)
        assert!(ui.versions_synced());
    }

    #[test]
    fn hit_button_and_text_centers() {
        let ui = lower(&slice(), IrRect::from_xywh(0, 0, 960_000, 540_000), &TokenCtx::comfy(), 1);
        assert_eq!(ui.hit_test(24_000 + 238_000, 24_000 + 20_000), Some(WidgetId(2))); // button center
        assert_eq!(ui.hit_test(24_000 + 238_000, 76_000 + 20_000), Some(WidgetId(3))); // text center
        assert_eq!(ui.hit_test(900_000, 500_000), None); // empty area (regions pass-through)
    }

    #[test]
    fn focus_tab_order_button_then_text() {
        let ui = lower(&slice(), IrRect::from_xywh(0, 0, 960_000, 540_000), &TokenCtx::comfy(), 1);
        let order: Vec<WidgetId> = ui.focus.iter().map(|f| f.widget_id).collect();
        assert_eq!(order, vec![WidgetId(2), WidgetId(3)]);
        assert_eq!(ui.focus[0].next, Some(WidgetId(3)));
        assert_eq!(ui.text_inputs.len(), 1);
        assert_eq!(ui.text_inputs[0].widget_id, WidgetId(3));
    }

    // ----- Phase 5: layout policies, Hug, slot_list -----

    #[test]
    fn grid_two_cols_places_cells() {
        // 200000×200000 grid, 2 cols, pad/gap=12000 → cell 82000², gap 12000.
        let root = WidgetSpec::grid(
            "g",
            2,
            Sizing::Fixed { w: 200_000, h: 200_000 },
            vec![
                WidgetSpec::widget("a", "button"),
                WidgetSpec::widget("b", "button"),
                WidgetSpec::widget("c", "button"),
                WidgetSpec::widget("d", "button"),
            ],
        );
        let ui = lower(&root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
        assert_eq!(box_of(&ui, 1), IrRect::from_xywh(12_000, 12_000, 82_000, 82_000)); // r0c0
        assert_eq!(box_of(&ui, 2), IrRect::from_xywh(106_000, 12_000, 82_000, 82_000)); // r0c1
        assert_eq!(box_of(&ui, 3), IrRect::from_xywh(12_000, 106_000, 82_000, 82_000)); // r1c0
        assert_eq!(box_of(&ui, 4), IrRect::from_xywh(106_000, 106_000, 82_000, 82_000)); // r1c1
        assert!(ui.versions_synced());
    }

    #[test]
    fn overlay_children_share_origin() {
        let root = WidgetSpec::region(
            "o",
            LayoutPolicy::Overlay,
            Sizing::Fixed { w: 100_000, h: 100_000 },
            vec![WidgetSpec::widget("a", "button"), WidgetSpec::widget("b", "button")],
        );
        let ui = lower(&root, IrRect::from_xywh(0, 0, 100_000, 100_000), &TokenCtx::comfy(), 1);
        let a = box_of(&ui, 1);
        let b = box_of(&ui, 2);
        assert_eq!(a, IrRect::from_xywh(12_000, 12_000, 76_000, 76_000));
        assert_eq!(a, b); // overlay stacks at the same origin
    }

    #[test]
    fn flow_wraps_to_next_line() {
        // content_w=76000; each leaf clamps to 76000 wide → one per line.
        let root = WidgetSpec::region(
            "f",
            LayoutPolicy::Flow,
            Sizing::Fixed { w: 100_000, h: 200_000 },
            vec![
                WidgetSpec::widget("a", "button"),
                WidgetSpec::widget("b", "button"),
            ],
        );
        let ui = lower(&root, IrRect::from_xywh(0, 0, 100_000, 200_000), &TokenCtx::comfy(), 1);
        let a = box_of(&ui, 1);
        let b = box_of(&ui, 2);
        assert_eq!(a, IrRect::from_xywh(12_000, 12_000, 76_000, 40_000));
        // b wrapped onto the next line: y advanced by line_h(40000)+gap(12000).
        assert_eq!(b, IrRect::from_xywh(12_000, 64_000, 76_000, 40_000));
    }

    // `min_size`/`max_size` were declared in `grammar::SLOT_ATTRS` since v1 and
    // reached no parser and no solver, so an authored floor shrank through and an
    // authored ceiling grew past — DECLARED != EXERCISED on the exact pair that
    // exists to stop it. Byte-level: two `Fill` siblings in a 200_000mu-tall
    // StackV (pad 12_000, gap 12_000 → content 176_000, fill 82_000 each) — the
    // first floors at 120_000, the second caps at 40_000.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE] (duplicate #[test] from v1 fixed on port)
    #[test]
    fn min_and_max_size_clamp_the_main_axis() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.floored kind=region layout=stack_v min_size=mu(120)\n\
slot root.capped kind=region layout=stack_v max_size=mu(40)\n";
        let doc = crate::parse::parse_kit(src).expect("min_size/max_size parse");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
        let floored = box_of(&ui, 1);
        let capped = box_of(&ui, 2);
        assert_eq!(floored.max_y - floored.min_y, 120_000, "min_size floors the Fill share");
        assert_eq!(capped.max_y - capped.min_y, 40_000, "max_size caps the Fill share");
    }

    /// `justify=` moved nothing before 2026-08-04: the attr parsed nowhere and the
    /// solver always packed to the start. Byte-level, StackV 200_000mu tall, pad and
    /// gap 12_000 (comfy): two 40_000mu bars use 80_000 + 12_000 gap = 92_000 of the
    /// 176_000 content, leaving 84_000 slack. center → first bar starts at
    /// 12_000 + 84_000/2 = 54_000. end → 12_000 + 84_000 = 96_000.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE]
    #[test]
    fn justify_moves_the_track_along_the_main_axis() {
        let kit = |j: &str| {
            format!(
                "#vixi:kit v1\n\
slot root kind=region layout=stack_v justify={j}\n\
slot root.a kind=chrome size=mu(40)\n\
slot root.b kind=chrome size=mu(40)\n"
            )
        };
        let first_y = |j: &str| {
            let doc = crate::parse::parse_kit(&kit(j)).expect("justify parses");
            let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
            box_of(&ui, 1).min_y
        };
        assert_eq!(first_y("start"), 12_000, "start packs against the padding edge");
        assert_eq!(first_y("center"), 54_000, "center leads with half the slack");
        assert_eq!(first_y("end"), 96_000, "end leads with all of it");
    }

    /// `space_between` puts the whole slack BETWEEN the pair — the first bar does
    /// not move, the second lands at the far edge.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE]
    #[test]
    fn space_between_pushes_the_pair_apart() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v justify=space_between\n\
slot root.a kind=chrome size=mu(40)\n\
slot root.b kind=chrome size=mu(40)\n";
        let doc = crate::parse::parse_kit(src).expect("space_between parses");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
        assert_eq!(box_of(&ui, 1).min_y, 12_000);
        // 12_000 pad + 40_000 bar + 12_000 gap + 84_000 slack = 148_000, and the
        // second bar's 40_000 ends exactly on the 188_000 content edge.
        assert_eq!(box_of(&ui, 2).min_y, 148_000);
        assert_eq!(box_of(&ui, 2).max_y, 188_000);
    }

    /// `margin=` holds space around a slot inside the PARENT's flow — declared in
    /// grammar.rs:104 since v1, reaching no parser, so an authored outer gap did
    /// nothing at all. Byte-level, StackV 200_000mu tall, pad and gap 12_000: a
    /// 40_000mu bar with margin=mu(20) starts 20_000 below the padding edge
    /// (12_000 + 20_000 = 32_000) and its Fill sibling loses the 40_000 the two
    /// margins take, so the margin comes out of the PARENT and not out of the bar.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE]
    #[test]
    fn margin_consumes_the_parents_flow_not_the_slots_own_box() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.bar kind=chrome size=mu(40) margin=mu(20)\n\
slot root.rest kind=region layout=stack_v\n";
        let doc = crate::parse::parse_kit(src).expect("margin parses");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
        let bar = box_of(&ui, 1);
        assert_eq!(bar.min_y, 32_000, "the top margin offsets the bar");
        assert_eq!(bar.max_y - bar.min_y, 40_000, "the bar keeps its authored extent");
        // Cross axis: 176_000 content − 2×20_000 margin.
        assert_eq!(bar.max_x - bar.min_x, 136_000, "margin insets the cross axis too");
        // Fill sibling: 176_000 − 40_000 bar − 40_000 margins − 12_000 gap = 84_000.
        let rest = box_of(&ui, 2);
        assert_eq!(rest.max_y - rest.min_y, 84_000, "Fill splits what the margin left");
    }

    /// The same law under `overlay`, where every child owns the whole content
    /// box: `margin=` insets that box on all four sides. Byte-level, 200_000mu
    /// square, root pad 12_000: a plane with margin=mu(20) starts at
    /// 12_000 + 20_000 and its extent loses both margins (176_000 − 40_000).
    #[test]
    fn margin_insets_an_overlay_plane_on_all_four_sides() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=overlay\n\
slot root.ground kind=region layout=stack_v\n\
slot root.plane kind=region layout=stack_v margin=mu(20)\n";
        let doc = crate::parse::parse_kit(src).expect("margin parses under overlay");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 200_000, 200_000), &TokenCtx::comfy(), 1);
        let ground = box_of(&ui, 1);
        assert_eq!(ground.min_x, 12_000, "an unmargined plane stays on the padding edge");
        assert_eq!(ground.max_x - ground.min_x, 176_000);
        let plane = box_of(&ui, 2);
        assert_eq!(plane.min_x, 32_000, "the margin offsets the plane's origin");
        assert_eq!(plane.min_y, 32_000, "on both axes — overlay has no main axis");
        assert_eq!(plane.max_x - plane.min_x, 136_000, "and both margins come out of its extent");
        assert_eq!(plane.max_y - plane.min_y, 136_000);
    }

    /// `align=` sizes and places on the CROSS axis: `stretch` (the old and only
    /// behaviour) fills the track, `center` hugs the measured extent and centres it.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE]
    #[test]
    fn align_places_children_on_the_cross_axis() {
        let stretched = WidgetSpec::region(
            "r",
            LayoutPolicy::StackV,
            Sizing::Fill,
            vec![WidgetSpec { stable_key: "a".into(), sizing: Sizing::Fixed { w: 50_000, h: 30_000 }, ..WidgetSpec::default() }],
        );
        let mut centred = stretched.clone();
        centred.align = Some(Align::Center);
        let vp = IrRect::from_xywh(0, 0, 200_000, 200_000);
        let a = lower(&stretched, vp, &TokenCtx::comfy(), 1);
        let b = lower(&centred, vp, &TokenCtx::comfy(), 1);
        // A Fixed child pins its own width either way; align moves WHERE it sits:
        // content is 176_000 wide, so a 50_000 child centres at 12_000 + 63_000.
        assert_eq!(box_of(&a, 1).min_x, 12_000, "stretch leaves it on the padding edge");
        assert_eq!(box_of(&b, 1).min_x, 75_000, "center offsets by half the slack");
    }

    /// A contradictory pair resolves to the FLOOR: legibility is not negotiable
    /// downward by a ceiling authored under it.
    // [BOARD: DEBT-GOLDEN-VIXI-GRAMMAR-COVERAGE]
    #[test]
    fn a_floor_above_its_ceiling_wins() {
        let spec = WidgetSpec {
            min_size: Some(90_000),
            max_size: Some(30_000),
            ..WidgetSpec::default()
        };
        assert_eq!(clamp_main(&spec, 50_000), 90_000);
        assert_eq!(clamp_main(&WidgetSpec::default(), 50_000), 50_000);
    }

    #[test]
    fn hug_sizes_to_content() {
        // StackV Hug root: w = max child w + 2pad; h = sum child h + gap + 2pad.
        let root = WidgetSpec::region(
            "h",
            LayoutPolicy::StackV,
            Sizing::Hug,
            vec![
                WidgetSpec {
                    stable_key: "x".into(),
                    sizing: Sizing::Fixed { w: 50_000, h: 30_000 },
                    ..WidgetSpec::default()
                },
                WidgetSpec {
                    stable_key: "y".into(),
                    sizing: Sizing::Fixed { w: 50_000, h: 30_000 },
                    ..WidgetSpec::default()
                },
            ],
        );
        let ui = lower(&root, IrRect::from_xywh(0, 0, 500_000, 500_000), &TokenCtx::comfy(), 1);
        // w = 50000 + 24000 = 74000 ; h = (30000+30000+12000) + 24000 = 96000
        assert_eq!(box_of(&ui, 0), IrRect::from_xywh(0, 0, 74_000, 96_000));
        assert_eq!(box_of(&ui, 1), IrRect::from_xywh(12_000, 12_000, 50_000, 30_000));
        assert_eq!(box_of(&ui, 2), IrRect::from_xywh(12_000, 54_000, 50_000, 30_000));
    }

    #[test]
    fn slot_list_carries_bound_and_stacks() {
        let root = WidgetSpec::slot_list(
            "list",
            "command_row",
            8,
            vec![
                WidgetSpec::widget("list.0", "command_row"),
                WidgetSpec::widget("list.1", "command_row"),
            ],
        );
        let ui = lower(&root, IrRect::from_xywh(0, 0, 300_000, 300_000), &TokenCtx::comfy(), 1);
        assert_eq!(ui.widgets[0].kind, SlotKind::SlotList);
        assert_eq!(ui.widgets[0].slot_list_max, Some(8)); // bound carried to IR
        // two command_row children stacked + focusable
        assert_eq!(ui.focus.len(), 2);
    }
}

