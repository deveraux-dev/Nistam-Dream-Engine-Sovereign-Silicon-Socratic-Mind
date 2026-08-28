//! # ir.rs — AOT UI Lowering Engine IR
//!
//! The lowered-UI planes a `.kit.vixi` source compiles to: widget tree,
//! solved layout boxes, hit map, focus graph, text-input declarations and
//! the paint plane (`DrawCmd`). Drained from v2
//! `F:\NewRepo\crates\forge-vix\src\ir.rs` (MIGRATION.md:179-181, §MV Track
//! B first slice) — the front-end that PRODUCES a [`LoweredUi`] from `.kit.vixi`
//! source text (v2 parse.rs / grammar.rs / semantic.rs / loader.rs) is a later
//! tranche; this module carries the IR shape and its emitter-facing methods only.
//!
//! **Integer-only.** Geometry is raw `i64` MilliUnit (1000 = 1px). Core
//! invariant: every plane derives from one `LayoutIR` and shares one
//! `layout_version`.

// ---------------------------------------------------------------------------
// Identity + canonical slot vocabulary
// ---------------------------------------------------------------------------

/// Runtime-stable, `Copy` widget identity. Re-exported from `forge-canvas-v3`
/// so the whole v3 UI stack shares one widget-identity type.
pub use forge_canvas_v3::input::WidgetId;

/// Hot-reload restoration key — stable across reloads when kind + capability
/// stay compatible. Build/reload-time only; never read on the hotpath.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StableKey(pub String);

impl StableKey {
    /// Borrow the key as a plain string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A widget-inventory reference (the `name=` of a `kind=widget` slot):
/// `button`, `text_field`, `slider`, `toggle`, `dropdown`, `tab`, `tree`,
/// `list`, … resolved against the widget inventory at load.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetName(pub String);

/// Canonical slot kinds (`template_grammar.md` §Slot Kinds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// Non-interactive surround (borders, plates, frames).
    Chrome,
    /// Glyph run.
    Text,
    /// Bitmap / atlas region.
    Image,
    /// Interactive primitive — [`WidgetNode::widget_name`] selects from inventory.
    Widget,
    /// Container for child slots; [`WidgetNode::layout`] carries the policy.
    Region,
    /// Audio-reactive paint surface (Vixi audio-dialect brush).
    Brush,
    /// Ordered, bounded dynamic list of homogeneous children.
    SlotList,
    /// Corner sigil / badge (error dot, notification, status pip).
    /// Non-interactive; intrinsic size; positioned via corner-anchor CSS.
    SigilCorner,
    /// Long-form journal/log text surface. Parchment material. Fills parent.
    JournalText,
    /// Collapsible side drawer. Triggered by `long_press_drawer` attribute.
    /// Hugs content when open; zero-height when collapsed.
    Drawer,
}

impl SlotKind {
    /// Every variant, canonical table order.
    pub const ALL: &'static [Self] = &[
        Self::Chrome,
        Self::Text,
        Self::Image,
        Self::Widget,
        Self::Region,
        Self::Brush,
        Self::SlotList,
        Self::SigilCorner,
        Self::JournalText,
        Self::Drawer,
    ];

    /// Authored name → variant.
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "chrome" => Self::Chrome,
            "text" => Self::Text,
            "image" => Self::Image,
            "widget" => Self::Widget,
            "region" => Self::Region,
            "brush" => Self::Brush,
            "slot_list" => Self::SlotList,
            "sigil_corner" => Self::SigilCorner,
            "journal_text" => Self::JournalText,
            "drawer" => Self::Drawer,
            _ => return None,
        })
    }

    /// Variant → canonical authoring name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Text => "text",
            Self::Image => "image",
            Self::Widget => "widget",
            Self::Region => "region",
            Self::Brush => "brush",
            Self::SlotList => "slot_list",
            Self::SigilCorner => "sigil_corner",
            Self::JournalText => "journal_text",
            Self::Drawer => "drawer",
        }
    }
}

/// Region layout policies (`template_grammar.md` §Layout Policies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutPolicy {
    /// Children stacked top-to-bottom.
    StackV,
    /// Children stacked left-to-right.
    StackH,
    /// Children in a uniform grid.
    Grid,
    /// Children painted on top of one another, same box.
    Overlay,
    /// Children wrap onto multiple lines along the main axis.
    Flow,
    /// Hexagonal tessellation: children snap to hex-prism coordinates.
    HexGrid,
}

impl LayoutPolicy {
    /// Every variant, canonical table order.
    pub const ALL: &'static [Self] =
        &[Self::StackV, Self::StackH, Self::Grid, Self::Overlay, Self::Flow, Self::HexGrid];

    /// Authored name → variant, including the golden-corpus semantic aliases
    /// (`split_view` → `StackH`, …).
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "stack_v" | "timeline_tracks" => Self::StackV,
            "stack_h" | "split_view" | "deck_mixer_deck" => Self::StackH,
            "grid" | "quad_view" => Self::Grid,
            "overlay" | "dockspace" => Self::Overlay,
            "flow" => Self::Flow,
            "hex_grid" => Self::HexGrid,
            _ => return None,
        })
    }

    /// Variant → canonical authoring name (aliases never serialize).
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::StackV => "stack_v",
            Self::StackH => "stack_h",
            Self::Grid => "grid",
            Self::Overlay => "overlay",
            Self::Flow => "flow",
            Self::HexGrid => "hex_grid",
        }
    }
}

/// (1) WidgetIR — normalized node with stable identity + canonical kind.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetNode {
    /// Runtime-stable widget identity.
    pub id: WidgetId,
    /// Hot-reload restoration key.
    pub stable_key: StableKey,
    /// Canonical slot kind.
    pub kind: SlotKind,
    /// `Some` when `kind == Widget` (the inventory name) or for `SlotList::of`.
    pub widget_name: Option<WidgetName>,
    /// `Some` when `kind == Region`.
    pub layout: Option<LayoutPolicy>,
    /// `Some` when `kind == SlotList` — bounded length (hot-path size-stability).
    pub slot_list_max: Option<u16>,
    /// The enclosing widget, or `None` at the root.
    pub parent: Option<WidgetId>,
    /// Slot is only visible when its parent region is hovered.
    pub hover_reveal: bool,
    /// A long-press on this slot opens a `kind=drawer` child or sibling.
    pub long_press_drawer: bool,
    /// Slot can be collapsed to zero height (user toggle or programmatic).
    pub collapsible: bool,
    /// Slot responds to the audio-reactive VibeMatrix bus.
    pub audio_reactive: bool,
    /// What a `kind=brush` slot DRAWS, carried through to the emitters.
    ///
    /// `parse.rs:748-773` typechecks `shape`/`radius`/`phase`/`stride` with typed
    /// refusals, and `layout.rs:132` lands them on the authored `WidgetSpec` —
    /// but `lower()` had no field to put them in, so the figure died one hop
    /// short of every renderer. The spec's own doc claimed this was already
    /// fixed; it was fixed on the PRE-layout tree only (2026-08-26).
    pub brush: Option<crate::layout::BrushSlot>,
    /// Optional GPU material authored via `.kit.vixi` (`material=bronze`).
    /// `None` = inherit the surrounding panel's material.
    pub material: Option<String>,
    /// Authored `color=palette.<slot>` — the palette slot NAME the runtime
    /// resolves against the installed sheet. `None` = the kind's default token.
    pub chrome_color: Option<String>,
    /// Authored `border_radius=mu(N)`. `None` = the runtime default rounding.
    pub border_radius: Option<i64>,
    /// Authored `alpha=permyriad(0..10000)`.
    pub alpha_pmy: Option<u16>,
    /// Authored `font=<family token>`.
    pub font: Option<String>,
    /// Authored `semantic=<ramp>` — a meter's meaning ramp, distinct from `color=`.
    pub semantic: Option<String>,
}

/// Authored paint attrs, resolved once for every face. Integer in, integer out —
/// unit conversion belongs to the sink.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleAtom {
    /// Corner radius in MilliUnit.
    pub radius_mu: Option<i64>,
    /// Opacity in permyriad (0..10000).
    pub alpha_pmy: Option<u16>,
    /// Palette slot name.
    pub chrome_color: Option<String>,
    /// Font family token.
    pub font: Option<String>,
    /// Meter meaning ramp.
    pub semantic: Option<String>,
}

impl WidgetNode {
    /// Resolve this node's authored paint attrs into one [`StyleAtom`], the
    /// single source every emitter reads.
    pub fn style_atom(&self) -> StyleAtom {
        StyleAtom {
            radius_mu: self.border_radius,
            alpha_pmy: self.alpha_pmy,
            chrome_color: self.chrome_color.clone(),
            font: self.font.clone(),
            semantic: self.semantic.clone(),
        }
    }

    fn name_is(&self, n: &str) -> bool {
        self.widget_name.as_ref().is_some_and(|w| w.0 == n)
    }

    /// Which slots take pointer hits. Widgets and Drawers do; chrome/text/
    /// image/region/sigil_corner/journal_text are non-interactive.
    pub fn accepts_pointer(&self) -> bool {
        matches!(self.kind, SlotKind::Widget | SlotKind::Drawer)
    }

    /// Keyboard-focusable interactive widgets.
    pub fn accepts_keyboard(&self) -> bool {
        if !matches!(self.kind, SlotKind::Widget) {
            return false;
        }
        const FOCUSABLE: &[&str] = &[
            "button", "text_field", "slider", "toggle", "dropdown", "tab", "tree", "tree_node",
            "list", "dialogue_choice", "menu_item", "command_row", "icon_button",
        ];
        self.widget_name.as_ref().is_some_and(|w| FOCUSABLE.contains(&w.0.as_str()))
    }

    /// Only `text_field` widgets get an edit session.
    pub fn is_text_input(&self) -> bool {
        matches!(self.kind, SlotKind::Widget) && self.name_is("text_field")
    }
}

// ---------------------------------------------------------------------------
// Geometry + the derived planes
// ---------------------------------------------------------------------------

/// Integer AABB in min/max form (`{min_x,min_y,max_x,max_y}`). i64 MilliUnit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct IrRect {
    /// Left edge, inclusive.
    pub min_x: i64,
    /// Top edge, inclusive.
    pub min_y: i64,
    /// Right edge, exclusive.
    pub max_x: i64,
    /// Bottom edge, exclusive.
    pub max_y: i64,
}

impl IrRect {
    /// Build a rect from its top-left corner and extent.
    pub const fn from_xywh(x: i64, y: i64, w: i64, h: i64) -> Self {
        Self { min_x: x, min_y: y, max_x: x + w, max_y: y + h }
    }

    /// Inclusive-min, EXCLUSIVE-max hit predicate. Zero alloc.
    #[inline]
    pub fn contains(&self, px: i64, py: i64) -> bool {
        px >= self.min_x && px < self.max_x && py >= self.min_y && py < self.max_y
    }
}

/// (2) LayoutIR — authoritative integer layout box.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutBox {
    /// The widget this box was solved for.
    pub widget_id: WidgetId,
    /// The widget's stable key, carried for reverse lookup.
    pub stable_key: StableKey,
    /// The solved integer rect.
    pub rect: IrRect,
    /// Paint order (higher paints later / on top).
    pub z: i32,
    /// Clip region this box is scoped to, if any.
    pub clip_id: Option<u32>,
    /// Scroll region this box rides, if any.
    pub scroll_id: Option<u32>,
    /// Text baseline offset in MilliUnit, if this box carries a text run.
    pub baseline: Option<i64>,
    /// The layout pass this box was solved in.
    pub layout_version: u32,
}

/// Pointer-event capabilities packed into a `u8` bitset (no `Vec` on hotpath).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitAccept {
    /// Accepts pointer press/release.
    Pointer,
    /// Accepts wheel/scroll input.
    Wheel,
    /// Accepts drag gestures.
    Drag,
    /// Accepts text input.
    Text,
    /// Accepts keyboard focus.
    Focus,
}

impl HitAccept {
    /// This capability's bit in a [`HitRegion::accepts`] bitset.
    pub const fn bit(self) -> u8 {
        match self {
            HitAccept::Pointer => 1,
            HitAccept::Wheel => 2,
            HitAccept::Drag => 4,
            HitAccept::Text => 8,
            HitAccept::Focus => 16,
        }
    }
}

/// How a hit region behaves while its widget is disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisabledPolicy {
    /// Consume the hit but do nothing (still opaque to hit-testing).
    Block,
    /// Ignore this region and test whatever is beneath it.
    PassThrough,
    /// Region is not present in hit-testing at all.
    Ignore,
}

/// (3) HitMap entry — flat, z-ordered, integer.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    /// The widget this region hit-tests for.
    pub widget_id: WidgetId,
    /// The region's rect.
    pub rect: IrRect,
    /// Paint order; hit-testing walks highest-`z` first.
    pub z: i32,
    /// OR of [`HitAccept::bit`] values.
    pub accepts: u8,
    /// Behavior while the widget is disabled.
    pub disabled_policy: DisabledPolicy,
    /// The layout pass this region was solved in.
    pub layout_version: u32,
}

/// Which candidate a focus restore should prefer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestorePriority {
    /// Restore the exact widget by stable key.
    ExactKey,
    /// Restore any widget in the same focus group.
    SameGroup,
    /// Fall back to the root default focus target.
    RootDefault,
    /// No restore candidate.
    None,
}

/// (4) FocusGraph node.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusNode {
    /// The widget this node describes.
    pub widget_id: WidgetId,
    /// The widget's stable key.
    pub stable_key: StableKey,
    /// Authored tab order, if any.
    pub tab_index: Option<i32>,
    /// Whether this widget accepts keyboard focus.
    pub accepts_keyboard: bool,
    /// The focus group this widget belongs to, if any.
    pub focus_group: Option<u32>,
    /// The next widget in tab order.
    pub next: Option<WidgetId>,
    /// The previous widget in tab order.
    pub prev: Option<WidgetId>,
    /// How to restore focus onto this widget after a reload.
    pub restore_priority: RestorePriority,
    /// The layout pass this node was solved in.
    pub layout_version: u32,
}

/// How a caret is drawn in a text input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretPolicy {
    /// A single-line caret.
    SingleLine,
    /// A multi-line caret that can wrap.
    MultiLine,
    /// No caret is drawn.
    Hidden,
    /// The host draws its own caret.
    Custom,
}

/// What kind of selection a text input supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// No selection.
    None,
    /// A caret only, no range selection.
    CaretOnly,
    /// A single contiguous range.
    Range,
    /// Multiple disjoint ranges.
    MultiRange,
}

/// Input-method-editor policy for a text input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImePolicy {
    /// IME composition is disabled.
    Disabled,
    /// IME composition is enabled.
    Enabled,
    /// IME composition is deferred to the host.
    Deferred,
}

/// (5) TextInput declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputDecl {
    /// The widget this declaration describes.
    pub widget_id: WidgetId,
    /// The widget's stable key.
    pub stable_key: StableKey,
    /// The text buffer backing this input.
    pub text_buffer_id: u32,
    /// The solved rect the input occupies.
    pub rect: IrRect,
    /// Maximum accepted character count.
    pub max_chars: u32,
    /// Caret rendering policy.
    pub caret_policy: CaretPolicy,
    /// Selection policy.
    pub selection_policy: SelectionPolicy,
    /// IME policy.
    pub ime_policy: ImePolicy,
    /// The layout pass this declaration was solved in.
    pub layout_version: u32,
}

/// Token resolution status for a paint command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenStatus {
    /// The token resolved to a concrete value.
    Resolved,
    /// The token is unresolved; paint with the magenta sentinel.
    MagentaUnresolved,
    /// Schema validation failed for this token.
    SchemaFailed,
}

/// (6) RenderIR draw command.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawCmd {
    /// Stable identity for this draw command within a frame.
    pub cmd_id: u32,
    /// The widget this command paints.
    pub widget_id: WidgetId,
    /// The rect this command paints into.
    pub bounds: IrRect,
    /// Clip region this command is scoped to, if any.
    pub clip_id: Option<u32>,
    /// Token resolution status.
    pub token_status: TokenStatus,
    /// The resolved paint token, if any.
    pub token_id: Option<u32>,
    /// The layout pass this command was solved from.
    pub layout_version: u32,
    /// The render pass this command belongs to.
    pub render_version: u32,
}

/// The fully-lowered UI bundle — every static plane a `.kit.vixi` source
/// compiles to, sharing one `layout_version`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoweredUi {
    /// The widget tree.
    pub widgets: Vec<WidgetNode>,
    /// The solved layout boxes.
    pub layout: Vec<LayoutBox>,
    /// The hit map.
    pub hits: Vec<HitRegion>,
    /// The focus graph.
    pub focus: Vec<FocusNode>,
    /// Text-input declarations.
    pub text_inputs: Vec<TextInputDecl>,
    /// The paint plane.
    pub draws: Vec<DrawCmd>,
    /// The layout pass every plane above shares.
    pub layout_version: u32,
    /// Authored `source=` binds, `(stable_key, context key)` — a slot listed here
    /// looks up the named context key instead of its stable-key leaf segment.
    pub source_binds: Vec<(String, String)>,
    /// Authored `text="…"` literals, `(stable_key, words)` — the STATIC label a
    /// slot carries when no host bind answers for it. A `source_binds` hit
    /// outranks this: live data beats the authored default.
    pub text_literals: Vec<(String, String)>,
    /// Authored `ramp=type.ramp[N]` binds, `(stable_key, stop)` — a slot listed
    /// here paints at its authored type-ramp stop instead of the leaf-name
    /// heuristic.
    pub ramp_binds: Vec<(String, forge_canvas_v3::text::FontSize)>,
    /// Authored `vibe_glow=`/`vibe_scale=`/`vibe_opacity=`/`vibe_offsety=` binds,
    /// `(stable_key, bind)` — the slot's live DRIVE, one VibeMatrix channel per
    /// UI property.
    ///
    /// `parse.rs:676-679` has typed these into [`crate::baked::VibeBind`] since
    /// the Patch-6 substrate and no emitter ever read them, so an authored
    /// audio-reactive surface rendered stone-still (2026-08-26). The channel
    /// VALUES are the host's to supply — `forge_shaderbind::ShaderBind::route`
    /// is the landed producer — which is why this carries an index, never a
    /// signal: the IR stays integer and this crate takes no shader dependency.
    pub vibe_binds: Vec<(String, crate::baked::VibeBind)>,
}

impl LoweredUi {
    /// Every dynamic plane's `layout_version` matches the bundle's own.
    pub fn versions_synced(&self) -> bool {
        let v = self.layout_version;
        self.layout.iter().all(|b| b.layout_version == v)
            && self.hits.iter().all(|h| h.layout_version == v)
            && self.focus.iter().all(|f| f.layout_version == v)
            && self.text_inputs.iter().all(|t| t.layout_version == v)
            && self.draws.iter().all(|d| d.layout_version == v)
    }

    /// Reverse-z walk, exclusive-max, zero heap allocation, respects disabled
    /// policy, returns the stable [`WidgetId`] at `(px, py)`.
    pub fn hit_test(&self, px: i64, py: i64) -> Option<WidgetId> {
        for region in self.hits.iter().rev() {
            if matches!(region.disabled_policy, DisabledPolicy::Ignore) {
                continue;
            }
            if region.rect.contains(px, py) {
                if matches!(region.disabled_policy, DisabledPolicy::PassThrough) {
                    continue;
                }
                return Some(region.widget_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget(name: &str) -> WidgetNode {
        WidgetNode {
            id: WidgetId(0),
            stable_key: StableKey("k".into()),
            kind: SlotKind::Widget,
            widget_name: Some(WidgetName(name.into())),
            layout: None,
            slot_list_max: None,
            parent: None,
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            brush: None,
            material: None,
            chrome_color: None,
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
        }
    }

    fn region(policy: LayoutPolicy) -> WidgetNode {
        WidgetNode {
            id: WidgetId(0),
            stable_key: StableKey("k".into()),
            kind: SlotKind::Region,
            widget_name: None,
            layout: Some(policy),
            slot_list_max: None,
            parent: None,
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            brush: None,
            material: None,
            chrome_color: None,
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
        }
    }

    fn hit(id: u32, x: i64, y: i64, w: i64, h: i64, policy: DisabledPolicy) -> HitRegion {
        HitRegion {
            widget_id: WidgetId(id),
            rect: IrRect::from_xywh(x, y, w, h),
            z: id as i32,
            accepts: HitAccept::Pointer.bit(),
            disabled_policy: policy,
            layout_version: 1,
        }
    }

    #[test]
    fn capabilities_follow_canonical_kind_and_name() {
        assert!(widget("button").accepts_pointer());
        assert!(widget("button").accepts_keyboard());
        assert!(widget("text_field").accepts_keyboard());
        assert!(widget("text_field").is_text_input());
        assert!(!widget("button").is_text_input());
        assert!(!region(LayoutPolicy::StackV).accepts_pointer());
        assert!(!region(LayoutPolicy::StackV).accepts_keyboard());
    }

    #[test]
    fn ir_rect_inclusive_min_exclusive_max() {
        let r = IrRect::from_xywh(0, 0, 100, 50);
        assert!(r.contains(0, 0));
        assert!(r.contains(99, 49));
        assert!(!r.contains(100, 0));
        assert!(!r.contains(0, 50));
    }

    #[test]
    fn hit_test_reverse_z_passthrough_ignore() {
        let ui = LoweredUi {
            hits: vec![
                hit(1, 0, 0, 100, 100, DisabledPolicy::Block),
                hit(2, 0, 0, 100, 100, DisabledPolicy::PassThrough),
                hit(3, 0, 0, 100, 100, DisabledPolicy::Ignore),
            ],
            ..Default::default()
        };
        assert_eq!(ui.hit_test(50, 50), Some(WidgetId(1)));
        assert_eq!(ui.hit_test(200, 200), None);
    }

    #[test]
    fn versions_synced_detects_drift() {
        let mut ui = LoweredUi {
            layout_version: 1,
            hits: vec![hit(1, 0, 0, 10, 10, DisabledPolicy::Block)],
            ..Default::default()
        };
        assert!(ui.versions_synced());
        ui.hits[0].layout_version = 0;
        assert!(!ui.versions_synced());
    }

    #[test]
    fn slot_kind_and_layout_policy_roundtrip_canonical_names() {
        for &k in SlotKind::ALL {
            assert_eq!(SlotKind::from_name(k.canonical_name()), Some(k));
        }
        for &p in LayoutPolicy::ALL {
            assert_eq!(LayoutPolicy::from_name(p.canonical_name()), Some(p));
        }
    }

    #[test]
    fn layout_policy_golden_corpus_aliases() {
        assert_eq!(LayoutPolicy::from_name("split_view"), Some(LayoutPolicy::StackH));
        assert_eq!(LayoutPolicy::from_name("quad_view"), Some(LayoutPolicy::Grid));
        assert_eq!(LayoutPolicy::from_name("timeline_tracks"), Some(LayoutPolicy::StackV));
        assert_eq!(LayoutPolicy::from_name("deck_mixer_deck"), Some(LayoutPolicy::StackH));
        assert_eq!(LayoutPolicy::from_name("dockspace"), Some(LayoutPolicy::Overlay));
    }
}
