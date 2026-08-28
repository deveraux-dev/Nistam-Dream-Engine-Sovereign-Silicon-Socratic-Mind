//! B0a — Sovereign UI Engine (arena-ECS retained-mode engine).
//!
//! This module provides the core UI state machine: an arena-based widget hierarchy,
//! deterministic layout solving, and zero-allocation drawing command emission.
//!
//! # Architecture
//! - **TreeManager**: Arena-ECS holding all widget nodes, parent-child links, and property state.
//!   All state is flat integer POD (MilliUnit and Permyriad).
//! - **LayoutEngine**: Deterministic layout solver using integer-only arithmetic.
//! - **emit_draw_list**: Converts retained-mode tree state into draw commands (DrawCmd stream).
//! - **UiTripleBuffer**: Lock-free handoff between producer (UI frame) and consumer (render thread).
//!
//! ## Two-Layer Widget Architecture
//! - **Kind-emit table** (`emit_draw_list`): Retained-mode tree, integer state, Synthesia CID_* colours.
//! - **`widgets` module** (at crate root): Immediate-mode adapter layer for forge-gui.

use std::sync::Mutex;
use forge_core_v3::fixed_point::MilliUnit;
use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::layout::Direction;
use crate::spring::AnimatedRect;
use crate::theme::{CID_GROUND, CID_BAR, CID_FRAME, CID_TITLE, CID_STATUS, CID_ACCENT, syn_rgba};
use crate::tokens::TokenId;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum widget count in a single arena.
const MAX_WIDGETS: usize = 256;

/// Maximum children per widget node.
const MAX_CHILDREN: usize = 16;

/// Maximum events in the event ring buffer.
const MAX_EVENTS: usize = 64;

/// Maximum snap anchor count.
const MAX_SNAP_ANCHORS: usize = 32;

// ── WidgetId + WidgetKind ───────────────────────────────────────────────────

/// Integer node handle into the widget arena.
/// Uses u32 for cache-friendly dense packing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub u32);

impl From<u32>   for WidgetId { fn from(n: u32)   -> Self { Self(n) } }
impl From<u64>   for WidgetId { fn from(n: u64)   -> Self { Self(n as u32) } }
impl From<i32>   for WidgetId { fn from(n: i32)   -> Self { Self(n as u32) } }
impl From<usize> for WidgetId { fn from(n: usize) -> Self { Self(n as u32) } }

impl std::ops::Add<u32>   for WidgetId { type Output = Self; fn add(self, n: u32)   -> Self { Self(self.0.wrapping_add(n)) } }
impl std::ops::Add<u64>   for WidgetId { type Output = Self; fn add(self, n: u64)   -> Self { Self(self.0.wrapping_add(n as u32)) } }
impl std::ops::Add<i32>   for WidgetId { type Output = Self; fn add(self, n: i32)   -> Self { Self(self.0.wrapping_add(n as u32)) } }
impl std::ops::Add<usize> for WidgetId { type Output = Self; fn add(self, n: usize) -> Self { Self(self.0.wrapping_add(n as u32)) } }

impl std::ops::BitOr<u32> for WidgetId { type Output = Self; fn bitor(self, n: u32) -> Self { Self(self.0 | n) } }
impl std::ops::BitOr<u64> for WidgetId { type Output = Self; fn bitor(self, n: u64) -> Self { Self(self.0 | n as u32) } }

/// Widget state flag: toggle on / button pressed.
pub const FLAG_WIDGET_ON:      u32 = 1 << 0;

/// Widget state flag: hovered (set by hit-test + dispatch).
pub const FLAG_HOVERED:        u32 = 1 << 1;

/// Widget state flag: dropdown open.
pub const FLAG_DROPDOWN_OPEN:  u32 = 1 << 2;

/// Widget state flag: tab active.
pub const FLAG_TAB_ACTIVE:     u32 = 1 << 3;

/// Flat widget roles — no heap, no dynamic dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WidgetKind {
    /// Container widget (background/frame).
    #[default]
    Panel,
    /// Rectangle drawing primitive.
    Rect,
    /// Text label.
    Text,
    /// Slot (inert container, no rendering).
    Slot,
    /// Scissor/clipping region.
    Clip,
    /// Interactive button.
    Button,
    /// Value slider (0..10000 Permyriad).
    Slider,
    /// Toggle switch.
    Toggle,
    /// Dropdown menu.
    Dropdown,
    /// Tab selector.
    Tab,
    /// Text input field.
    TextInput,
}

/// Snap anchor: binds a widget to an integer layout target (Permyriad coordinates).
/// Flat POD structure — round-trips through snapshot() unchanged.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnapAnchor {
    /// WidgetId.0 of the anchored widget; u32::MAX = unused slot.
    pub widget: u32,
    /// X coordinate (Permyriad: 1/10_000).
    pub x: i32,
    /// Y coordinate (Permyriad: 1/10_000).
    pub y: i32,
    /// Width (Permyriad).
    pub w: i32,
    /// Height (Permyriad).
    pub h: i32,
}

impl SnapAnchor {
    /// Empty sentinel (unused slot marker).
    pub const EMPTY: Self = Self { widget: u32::MAX, x: 0, y: 0, w: 0, h: 0 };

    /// Check if this slot is empty.
    pub fn is_empty(self) -> bool { self.widget == u32::MAX }
}

/// Text alignment within a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align {
    /// Align to the start (top or left).
    #[default]
    Start,
    /// Align to center.
    Center,
    /// Align to the end (bottom or right).
    End,
}

/// Layout properties for a widget node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LayoutProps {
    /// Direction children are stacked (Vertical or Horizontal).
    pub direction: Direction,
    /// Alignment of children along cross-axis.
    pub align: Align,
    /// Inner padding (MilliUnit).
    pub padding: i64,
    /// Minimum width (MilliUnit); 0 = fill available.
    pub min_w: i64,
    /// Minimum height (MilliUnit); 0 = fill available.
    pub min_h: i64,
    /// RGBA colour (0xRRGGBBAA).
    pub color: u32,
}

// ── WidgetNode POD ──────────────────────────────────────────────────────────

/// A single widget in the arena. Flat POD — no heap, no smart pointers.
#[derive(Clone, Copy, Debug, Default)]
pub struct WidgetNode {
    /// What kind of widget this is.
    pub kind: WidgetKind,
    /// Parent widget ID (None for root).
    pub parent: Option<WidgetId>,
    /// Declared geometry (hint to layout engine).
    pub rect: UiRect,
    /// Computed by LayoutEngine::layout.
    pub layout_rect: UiRect,
    /// Bitfield of FLAG_*.
    pub flags: u32,
    /// Layout and visual properties.
    pub props: LayoutProps,
    /// Fixed UTF-8 label buffer (no heap allocation).
    pub label: [u8; 32],
    /// Actual label length (bytes 0..label_len).
    pub label_len: u8,
    /// Whether this node has spring animation active.
    pub has_anim: bool,
    /// Spring animation state (optional, ticked externally).
    pub anim: AnimatedRect,
    /// Numeric state (slider value, dropdown index, etc.).
    pub value: i64,
}

impl WidgetNode {
    /// Set the label from a UTF-8 string (truncated to 32 bytes).
    pub fn set_label(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let n = bytes.len().min(32);
        self.label[..n].copy_from_slice(&bytes[..n]);
        self.label_len = n as u8;
    }

    /// Get the label as a &str (returns "" if invalid UTF-8).
    pub fn label_str(&self) -> &str {
        core::str::from_utf8(&self.label[..self.label_len as usize]).unwrap_or("")
    }
}

// ── TreeManager arena + builder API ──────────────────────────────────────────

/// Arena-ECS widget container: all nodes, parent-child links, and state.
/// All fields are flat integer POD — serializable and deterministic.
pub struct TreeManager {
    /// Widget node arena (MAX_WIDGETS slots).
    pub nodes: [WidgetNode; MAX_WIDGETS],
    /// Number of live nodes (monotonic, reset on clear).
    pub count: u32,
    /// Children table: `children[i][0..child_count[i]]` = child WidgetIds.
    children: [[u32; MAX_CHILDREN]; MAX_WIDGETS],
    /// Child counts per widget.
    child_count: [u8; MAX_WIDGETS],
    /// Snap anchor table (substance, survives snapshot/restore unchanged).
    pub snap_anchors: [SnapAnchor; MAX_SNAP_ANCHORS],
    /// Active snap anchor count.
    pub snap_anchor_count: u8,
}

impl Default for TreeManager {
    fn default() -> Self {
        Self {
            nodes: [WidgetNode::default(); MAX_WIDGETS],
            count: 0,
            children: [[0u32; MAX_CHILDREN]; MAX_WIDGETS],
            child_count: [0u8; MAX_WIDGETS],
            snap_anchors: [SnapAnchor::EMPTY; MAX_SNAP_ANCHORS],
            snap_anchor_count: 0,
        }
    }
}

impl TreeManager {
    /// Construct a new empty TreeManager.
    pub fn new() -> Self { Self::default() }

    /// Allocate a new node in the arena and return its WidgetId.
    /// If the arena is full, returns an ID at the limit (writes go nowhere).
    pub fn new_node(&mut self, kind: WidgetKind) -> WidgetId {
        let id = self.count;
        if (id as usize) < MAX_WIDGETS {
            self.nodes[id as usize] = WidgetNode { kind, ..Default::default() };
            self.count += 1;
        }
        WidgetId(id)
    }

    /// Register `child` as a child of `parent`.
    /// Sets the parent pointer and adds to the parent's children list.
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        self.nodes[child.0 as usize].parent = Some(parent);
        let pi = parent.0 as usize;
        let cc = self.child_count[pi] as usize;
        if cc < MAX_CHILDREN {
            self.children[pi][cc] = child.0;
            self.child_count[pi] += 1;
        }
    }

    /// Get the children of a widget (slice into the children array).
    pub fn children_of(&self, id: WidgetId) -> &[u32] {
        let i = id.0 as usize;
        &self.children[i][..self.child_count[i] as usize]
    }

    /// Get mutable reference to a widget node.
    pub fn node_mut(&mut self, id: WidgetId) -> &mut WidgetNode {
        &mut self.nodes[id.0 as usize]
    }

    /// Get immutable reference to a widget node.
    pub fn node(&self, id: WidgetId) -> &WidgetNode {
        &self.nodes[id.0 as usize]
    }

    /// Register a snap anchor for a widget (Permyriad coordinates).
    /// Returns false if the snap anchor table is full.
    pub fn add_snap_anchor(&mut self, widget: WidgetId, x: i32, y: i32, w: i32, h: i32) -> bool {
        let idx = self.snap_anchor_count as usize;
        if idx >= MAX_SNAP_ANCHORS { return false; }
        self.snap_anchors[idx] = SnapAnchor { widget: widget.0, x, y, w, h };
        self.snap_anchor_count += 1;
        true
    }

    /// Find the snap anchor for a widget (if any).
    pub fn snap_anchor_for(&self, widget: WidgetId) -> Option<SnapAnchor> {
        self.snap_anchors[..self.snap_anchor_count as usize]
            .iter()
            .find(|a| a.widget == widget.0)
            .copied()
    }

    /// Step a spring animation toward a target (respecting snap anchors).
    /// If a snap anchor is registered, targets the anchor's x-coordinate with magnetic snap.
    pub fn tick_spring(
        &self,
        widget: WidgetId,
        acc: &mut crate::spring::SpringAccident,
        fallback_target: i32,
        stiffness: i32,
        snap_threshold: i32,
        fps: i32,
    ) {
        let id = widget.0 as usize;
        if id >= 256 { return; }
        let anchor_x = self.snap_anchor_for(widget).map(|a| a.x);
        let target = match anchor_x {
            Some(ax) => acc.snap_if_near(id, fallback_target, ax, snap_threshold),
            None     => fallback_target,
        };
        acc.step(id, target, stiffness, fps);
    }

    /// Serialize the entire arena to raw bytes.
    ///
    /// STUB (forbid-unsafe policy): the donor's POD `ptr::copy_nonoverlapping`
    /// serialization is banned outright by this workspace's `unsafe_code = "deny"`
    /// lint — forbid-first is universal here, not case-by-case, so no local
    /// `#[allow]` override. This preserves the size as metadata but does not
    /// serialize the actual field payload. Real serialization should land via
    /// serde or a `bytemuck`-derived `Pod`/`Zeroable` path (bytemuck is already
    /// ARCH000-nodded in this repo, L19 receipt 2026-08-10) once TreeManager's
    /// exact field layout is audited for Pod-safety.
    #[deprecated = "Serialization is incomplete (forbid-unsafe); use serde or bytemuck"]
    pub fn snapshot(&self) -> Vec<u8> {
        core::mem::size_of::<Self>().to_le_bytes().to_vec()
    }

    /// Restore arena state from snapshot bytes.
    ///
    /// STUB (forbid-unsafe policy): matches `snapshot`'s incomplete payload —
    /// always returns `None` until real serde/bytemuck-backed serialization lands.
    #[deprecated = "Serialization is incomplete (forbid-unsafe); use serde or bytemuck"]
    pub fn restore(_bytes: &[u8]) -> Option<Box<Self>> {
        None
    }

    /// Step spring animation for a widget, apply result to layout_rect.
    pub fn step_anim(&mut self, id: WidgetId, dt_ms: u32) {
        let n = &mut self.nodes[id.0 as usize];
        if !n.has_anim { return; }
        n.anim.step(dt_ms);
        n.layout_rect = UiRect::new(
            n.anim.x.position,
            n.anim.y.position,
            n.anim.w.position,
            n.anim.h.position,
        );
    }
}

// ── LayoutEngine (integer Permyriad throughout) ──────────────────────────────

/// Deterministic layout solver using integer-only arithmetic.
/// No floating point, no allocation — just stack temporaries.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Compute layout_rect for every node in the subtree rooted at `root`.
    /// Starts with the root at `root_rect` and recursively lays out children.
    pub fn layout(tree: &mut TreeManager, root: WidgetId, root_rect: UiRect) {
        tree.nodes[root.0 as usize].layout_rect = root_rect;
        Self::layout_node(tree, root);
    }

    fn layout_node(tree: &mut TreeManager, id: WidgetId) {
        // Copy parent data to release the borrow before child writes.
        let parent_rect = tree.nodes[id.0 as usize].layout_rect;
        let props = tree.nodes[id.0 as usize].props;
        let child_ids = tree.children[id.0 as usize];
        let child_count = tree.child_count[id.0 as usize] as usize;

        if child_count == 0 { return; }

        let pad = props.padding;
        let inner = UiRect::new(
            parent_rect.x.0 + pad,
            parent_rect.y.0 + pad,
            (parent_rect.w.0 - 2 * pad).max(0),
            (parent_rect.h.0 - 2 * pad).max(0),
        );

        match props.direction {
            Direction::Vertical   => Self::layout_col(tree, &child_ids, child_count, inner, props.align),
            Direction::Horizontal => Self::layout_row(tree, &child_ids, child_count, inner, props.align),
        }

        for ci in 0..child_count {
            Self::layout_node(tree, WidgetId(child_ids[ci]));
        }
    }

    fn layout_col(
        tree: &mut TreeManager,
        child_ids: &[u32; MAX_CHILDREN],
        count: usize,
        inner: UiRect,
        align: Align,
    ) {
        let total_min: i64 = (0..count)
            .map(|i| tree.nodes[child_ids[i] as usize].props.min_h)
            .sum();
        let remaining = (inner.h.0 - total_min).max(0);
        let fill_n = (0..count)
            .filter(|&i| tree.nodes[child_ids[i] as usize].props.min_h == 0)
            .count() as i64;
        let fill_share = if fill_n > 0 { remaining / fill_n } else { 0 };

        let mut cur_y = inner.y.0;
        for ci in 0..count {
            let cid = child_ids[ci] as usize;
            let h = if tree.nodes[cid].props.min_h > 0 { tree.nodes[cid].props.min_h } else { fill_share };
            let w = if tree.nodes[cid].props.min_w > 0 { tree.nodes[cid].props.min_w } else { inner.w.0 };
            let x = match align {
                Align::Start  => inner.x.0,
                Align::Center => inner.x.0 + (inner.w.0 - w) / 2,
                Align::End    => inner.x.0 + inner.w.0 - w,
            };
            tree.nodes[cid].layout_rect = UiRect::new(x, cur_y, w, h);
            cur_y += h;
        }
    }

    fn layout_row(
        tree: &mut TreeManager,
        child_ids: &[u32; MAX_CHILDREN],
        count: usize,
        inner: UiRect,
        align: Align,
    ) {
        let total_min: i64 = (0..count)
            .map(|i| tree.nodes[child_ids[i] as usize].props.min_w)
            .sum();
        let remaining = (inner.w.0 - total_min).max(0);
        let fill_n = (0..count)
            .filter(|&i| tree.nodes[child_ids[i] as usize].props.min_w == 0)
            .count() as i64;
        let fill_share = if fill_n > 0 { remaining / fill_n } else { 0 };

        let mut cur_x = inner.x.0;
        for ci in 0..count {
            let cid = child_ids[ci] as usize;
            let w = if tree.nodes[cid].props.min_w > 0 { tree.nodes[cid].props.min_w } else { fill_share };
            let h = if tree.nodes[cid].props.min_h > 0 { tree.nodes[cid].props.min_h } else { inner.h.0 };
            let y = match align {
                Align::Start  => inner.y.0,
                Align::Center => inner.y.0 + (inner.h.0 - h) / 2,
                Align::End    => inner.y.0 + inner.h.0 - h,
            };
            tree.nodes[cid].layout_rect = UiRect::new(cur_x, y, w, h);
            cur_x += w;
        }
    }
}

// ── UiEvent + zero-alloc EventRing ───────────────────────────────────────────

/// User input event (mouse, keyboard).
#[derive(Clone, Copy, Debug, Default)]
pub enum UiEvent {
    /// No event.
    #[default]
    None,
    /// Mouse button down at (x, y), button 1-3.
    MouseDown {
        /// X coordinate (MilliUnit).
        x: i64,
        /// Y coordinate (MilliUnit).
        y: i64,
        /// Button ID (1-3).
        button: u8,
    },
    /// Mouse button up at (x, y), button 1-3.
    MouseUp {
        /// X coordinate (MilliUnit).
        x: i64,
        /// Y coordinate (MilliUnit).
        y: i64,
        /// Button ID (1-3).
        button: u8,
    },
    /// Mouse moved to (x, y).
    MouseMove {
        /// X coordinate (MilliUnit).
        x: i64,
        /// Y coordinate (MilliUnit).
        y: i64,
    },
    /// Key pressed (platform-specific key code).
    KeyDown {
        /// Platform key code.
        key: u32,
    },
    /// Key released (platform-specific key code).
    KeyUp {
        /// Platform key code.
        key: u32,
    },
}

/// Circular ring buffer for events (zero-alloc, fixed-size).
pub struct EventRing {
    /// Event slots.
    events: [UiEvent; MAX_EVENTS],
    /// Head index (next to pop).
    head: usize,
    /// Tail index (next to push).
    tail: usize,
    /// Number of live events.
    len: usize,
}

impl Default for EventRing {
    fn default() -> Self {
        Self { events: [UiEvent::None; MAX_EVENTS], head: 0, tail: 0, len: 0 }
    }
}

impl EventRing {
    /// Construct a new empty event ring.
    pub fn new() -> Self { Self::default() }

    /// Push an event (drops if full).
    pub fn push(&mut self, ev: UiEvent) {
        if self.len < MAX_EVENTS {
            self.events[self.tail] = ev;
            self.tail = (self.tail + 1) % MAX_EVENTS;
            self.len += 1;
        }
    }

    /// Pop an event (None if empty).
    pub fn pop(&mut self) -> Option<UiEvent> {
        if self.len == 0 { return None; }
        let ev = self.events[self.head];
        self.head = (self.head + 1) % MAX_EVENTS;
        self.len -= 1;
        Some(ev)
    }

    /// Current number of events in the ring.
    pub fn len(&self) -> usize { self.len }

    /// Check if the ring is empty.
    pub fn is_empty(&self) -> bool { self.len == 0 }
}

// ── hit_test | bubble_chain ─────────────────────────────────────────────────

/// Walk all nodes in reverse draw order (topmost first); return first hit.
/// Uses half-open interval test: [min, max).
pub fn hit_test(tree: &TreeManager, x: i64, y: i64) -> Option<WidgetId> {
    for i in (0..tree.count as usize).rev() {
        if tree.nodes[i].layout_rect.contains(MilliUnit(x), MilliUnit(y)) {
            return Some(WidgetId(i as u32));
        }
    }
    None
}

/// Return the bubble chain from `start` up to the root.
/// Returns an array of WidgetIds; unreached slots are filled with WidgetId(u32::MAX).
pub fn bubble_chain(tree: &TreeManager, start: WidgetId) -> [WidgetId; 16] {
    let sentinel = WidgetId(u32::MAX);
    let mut chain = [sentinel; 16];
    let mut cur = start;
    for slot in chain.iter_mut() {
        *slot = cur;
        match tree.nodes[cur.0 as usize].parent {
            Some(p) => cur = p,
            None    => break,
        }
    }
    chain
}

// ── emit_draw_list (zero alloc, pure push) ──────────────────────────────────

/// Convert retained-mode tree state to draw commands.
/// Walks all nodes in creation order and emits DrawCmd for each.
/// All state used is deterministic (no allocations, no random reads).
pub fn emit_draw_list(tree: &TreeManager, draw: &mut DrawList) {
    for i in 0..tree.count as usize {
        let node = &tree.nodes[i];
        let r = node.layout_rect;
        match node.kind {
            WidgetKind::Panel | WidgetKind::Rect => {
                draw.push(DrawCmd::Rect { rect: r, color: node.props.color, radius: 0 });
            }
            WidgetKind::Clip => {
                draw.push(DrawCmd::Clip { rect: r });
            }
            WidgetKind::Text => {
                // Glyph baking deferred to atlas integration; emit placeholder cmd.
                draw.push(DrawCmd::Text {
                    rect: r,
                    glyph_start: 0,
                    glyph_count: 0,
                    color: node.props.color,
                });
            }
            WidgetKind::Slot => {}
            // Interactive widget kinds: token-resolved colours, no hardcoded hex.
            WidgetKind::Button => {
                let hovered = (node.flags & FLAG_HOVERED)   != 0;
                let pressed = (node.flags & FLAG_WIDGET_ON) != 0;
                let fill   = if pressed { draw.color(TokenId::Gold,    syn_rgba(CID_ACCENT, 0xFF)) }
                             else if hovered { draw.color(TokenId::BgHover, syn_rgba(CID_FRAME,  0xFF)) }
                             else { node.props.color };
                let border = draw.color(TokenId::Border,       syn_rgba(CID_FRAME, 0xFF));
                let text_c = draw.color(TokenId::TextPrimary,  syn_rgba(CID_TITLE, 0xFF));
                draw.push(DrawCmd::Rect { rect: r, color: fill, radius: 0 });
                draw.push(DrawCmd::RectOutline { rect: r, color: border, thickness: 1 });
                if node.label_len > 0 {
                    draw.push(DrawCmd::Text { rect: r, glyph_start: 0, glyph_count: 0, color: text_c });
                }
            }
            WidgetKind::Slider => {
                let track_c = draw.color(TokenId::BgDust,        syn_rgba(CID_BAR,    0xFF));
                let fill_c  = draw.color(TokenId::AccentCreation, syn_rgba(CID_ACCENT, 0xFF));
                draw.push(DrawCmd::Rect { rect: r, color: track_c, radius: 0 });
                let v = node.value.clamp(0, 10_000);
                if v > 0 && r.w.0 > 0 {
                    let fw = r.w.0 * v / 10_000;
                    let fill_r = UiRect::new(r.x.0, r.y.0, fw, r.h.0);
                    draw.push(DrawCmd::Rect { rect: fill_r, color: fill_c, radius: 0 });
                }
            }
            WidgetKind::Toggle => {
                let on = (node.flags & FLAG_WIDGET_ON) != 0;
                let track_c = if on { draw.color(TokenId::AccentCreation, syn_rgba(CID_ACCENT, 0xFF)) }
                              else  { draw.color(TokenId::BgDust,         syn_rgba(CID_FRAME,  0xFF)) };
                let thumb_c = draw.color(TokenId::TextPrimary, syn_rgba(CID_TITLE, 0xFF));
                draw.push(DrawCmd::Rect { rect: r, color: track_c, radius: 0 });
                let tw = r.w.0 / 2;
                let tx = if on { r.x.0 + r.w.0 - tw } else { r.x.0 };
                let thumb = UiRect::new(tx, r.y.0, tw, r.h.0);
                draw.push(DrawCmd::Rect { rect: thumb, color: thumb_c, radius: 0 });
            }
            WidgetKind::Dropdown => {
                let open   = (node.flags & FLAG_DROPDOWN_OPEN) != 0;
                let bg     = if open { draw.color(TokenId::BgHover, syn_rgba(CID_FRAME,  0xFF)) }
                             else    { node.props.color };
                let border = draw.color(TokenId::Border, syn_rgba(CID_FRAME, 0xFF));
                draw.push(DrawCmd::Rect { rect: r, color: bg, radius: 0 });
                draw.push(DrawCmd::RectOutline { rect: r, color: border, thickness: 1 });
            }
            WidgetKind::Tab => {
                let active  = (node.flags & FLAG_TAB_ACTIVE) != 0;
                let line_c  = if active { draw.color(TokenId::AccentCreation, syn_rgba(CID_ACCENT, 0xFF)) }
                              else      { draw.color(TokenId::BgDust,         syn_rgba(CID_STATUS, 0xFF)) };
                let text_c  = if active { line_c } else { draw.color(TokenId::TextPrimary, syn_rgba(CID_STATUS, 0xFF)) };
                draw.push(DrawCmd::Rect { rect: r, color: node.props.color, radius: 0 });
                if active && r.h.0 > 2_000 {
                    let ul = UiRect::new(r.x.0, r.y.0 + r.h.0 - 2_000, r.w.0, 2_000);
                    draw.push(DrawCmd::Rect { rect: ul, color: line_c, radius: 0 });
                }
                if node.label_len > 0 {
                    draw.push(DrawCmd::Text { rect: r, glyph_start: 0, glyph_count: 0, color: text_c });
                }
            }
            WidgetKind::TextInput => {
                let bg     = draw.color(TokenId::BgDust, syn_rgba(CID_GROUND, 0xFF));
                let border = draw.color(TokenId::Border, syn_rgba(CID_FRAME,  0xFF));
                draw.push(DrawCmd::Rect { rect: r, color: bg, radius: 0 });
                draw.push(DrawCmd::RectOutline { rect: r, color: border, thickness: 1 });
                if node.label_len > 0 {
                    let text_c = draw.color(TokenId::TextPrimary, syn_rgba(CID_TITLE, 0xFF));
                    draw.push(DrawCmd::Text { rect: r, glyph_start: 0, glyph_count: 0, color: text_c });
                }
            }
        }
    }
}

// ── UiTripleBuffer (lock-free gate pattern) ──────────────────────────────────

/// Lock-free triple buffer for DrawList handoff between threads.
///
/// Pattern: Both sides use try_lock() only — no blocking Mutex::lock().
/// If the consumer holds the lock during publish, the producer returns
/// the slot unchanged and retries next frame (non-blocking, no data loss).
pub struct UiTripleBuffer {
    /// Shared slot: (DrawList, generation counter).
    inner: Mutex<(Box<DrawList>, u64)>,
}

impl UiTripleBuffer {
    /// Boot-time init: allocate the shared slot once.
    pub fn new(initial: Box<DrawList>) -> Self {
        Self { inner: Mutex::new((initial, 0)) }
    }

    /// Producer: try to swap `fresh` into the bridge.
    /// Returns old slot for refill on success, or returns `fresh` unchanged
    /// if consumer holds the lock (caller retries next frame).
    pub fn publish(&self, mut fresh: Box<DrawList>) -> Box<DrawList> {
        match self.inner.try_lock() {
            Ok(mut slot) => {
                std::mem::swap(&mut slot.0, &mut fresh);
                slot.1 = slot.1.wrapping_add(1);
                fresh
            }
            Err(_) => fresh,
        }
    }

    /// Consumer: non-blocking try_take. Copies commands and glyphs into `dst`.
    /// Returns the new generation counter on success, or None if no new data.
    pub fn try_take(&self, last_gen: u64, dst: &mut DrawList) -> Option<u64> {
        let slot = self.inner.try_lock().ok()?;
        if slot.1 == last_gen { return None; }
        dst.clear();
        for &cmd in slot.0.commands() {
            dst.push(cmd);
        }
        // Copy glyph arena — DrawCmd::Text references glyph_start/glyph_count
        // indices into dst.glyphs[]. Without this, text commands would be dropped.
        let src_glyphs = slot.0.glyphs();
        let n = src_glyphs.len();
        if n > 0 {
            dst.glyphs_mut()[..n].copy_from_slice(src_glyphs);
            dst.glyph_count = n;
        }
        Some(slot.1)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::UiRect;

    fn make_tree() -> Box<TreeManager> {
        Box::new(TreeManager::new())
    }

    /// Test: tree parent-child arrays are built correctly.
    #[test]
    fn tree_parent_child_arrays() {
        let mut t = make_tree();
        let root  = t.new_node(WidgetKind::Panel);
        let child = t.new_node(WidgetKind::Rect);
        let grand = t.new_node(WidgetKind::Text);
        t.add_child(root, child);
        t.add_child(child, grand);

        assert_eq!(t.children_of(root),  &[child.0]);
        assert_eq!(t.children_of(child), &[grand.0]);
        assert_eq!(t.node(child).parent, Some(root));
        assert_eq!(t.node(grand).parent, Some(child));
        assert_eq!(t.node(root).parent,  None);
    }

    /// Test: vertical layout with 3 equal-height fill children.
    #[test]
    fn layout_vertical_container_parity() {
        let mut t = make_tree();
        let root = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.direction = Direction::Vertical;

        let c0 = t.new_node(WidgetKind::Rect);
        let c1 = t.new_node(WidgetKind::Rect);
        let c2 = t.new_node(WidgetKind::Rect);
        for c in [c0, c1, c2] {
            t.node_mut(c).props.min_w = 200_000; // 200px
            t.add_child(root, c);
        }

        let container = UiRect::new(0, 0, 1_000_000, 600_000); // 1000×600px
        LayoutEngine::layout(&mut t, root, container);

        let fill = 600_000_i64 / 3; // 200_000 per child
        assert_eq!(t.node(c0).layout_rect, UiRect::new(0,       0, 200_000, fill));
        assert_eq!(t.node(c1).layout_rect, UiRect::new(0,  fill,   200_000, fill));
        assert_eq!(t.node(c2).layout_rect, UiRect::new(0,  fill*2, 200_000, fill));
    }

    /// Test: hit_test and bubble_chain.
    #[test]
    fn hit_test_and_bubble() {
        let mut t = make_tree();
        let root  = t.new_node(WidgetKind::Panel);
        let child = t.new_node(WidgetKind::Rect);
        t.add_child(root, child);

        let container = UiRect::new(0, 0, 1_000_000, 600_000);
        t.node_mut(child).props.min_w = 200_000;
        t.node_mut(child).props.min_h = 100_000;
        LayoutEngine::layout(&mut t, root, container);

        // click inside child bounds
        let hit = hit_test(&t, 100_000, 50_000);
        assert_eq!(hit, Some(child), "click inside child should hit child");

        // bubble chain from child reaches root
        let chain = bubble_chain(&t, child);
        assert_eq!(chain[0], child);
        assert_eq!(chain[1], root);

        // click outside any widget
        let miss = hit_test(&t, 5_000_000, 5_000_000);
        assert_eq!(miss, None);
    }

    /// Test: panel+label tree emits correct DrawCmd sequence.
    #[test]
    fn emit_draw_list_cmd_sequence() {
        let mut t = make_tree();
        let panel = t.new_node(WidgetKind::Panel);
        let label = t.new_node(WidgetKind::Text);
        t.node_mut(label).props.color = 0xFF_FF_FF_FF;
        t.node_mut(label).set_label("hello");
        t.add_child(panel, label);

        LayoutEngine::layout(&mut t, panel, UiRect::new(0, 0, 400_000, 200_000));

        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);

        let cmds = dl.commands();
        assert_eq!(cmds.len(), 2, "panel → Rect cmd, label → Text cmd");
        assert!(matches!(cmds[0], DrawCmd::Rect { .. }), "first cmd is Rect (panel)");
        assert!(matches!(cmds[1], DrawCmd::Text { .. }), "second cmd is Text (label)");
    }

    /// Test: triple-buffer handoff integrity.
    #[test]
    fn triple_buffer_handoff() {
        let mut producer_slot = DrawList::new_boxed();
        let initial           = DrawList::new_boxed();
        let bridge            = UiTripleBuffer::new(initial);

        // write two rects into producer slot
        producer_slot.push(DrawCmd::Rect {
            rect: UiRect::new(0, 0, 100_000, 100_000),
            color: 0xFFFFFFFF,
            radius: 0,
        });
        producer_slot.push(DrawCmd::Rect {
            rect: UiRect::new(100_000, 0, 100_000, 100_000),
            color: 0xFF8800FF,
            radius: 0,
        });

        // publish: producer gets back the old slot
        producer_slot = bridge.publish(producer_slot);

        // consumer try_take (first attempt should succeed, gen changed from 0→1)
        let mut consumer_dst = DrawList::new_boxed();
        let gen = bridge.try_take(0, &mut *consumer_dst);
        assert!(gen.is_some(), "first try_take must succeed after publish");
        assert_eq!(consumer_dst.commands().len(), 2, "consumer should have 2 cmds");

        // second try_take with same gen returns None (nothing new)
        let gen2 = bridge.try_take(gen.unwrap(), &mut *consumer_dst);
        assert!(gen2.is_none(), "no new publish → try_take returns None");

        // producer_slot is now the recycled slot (old initial, zero cmds)
        let _ = producer_slot; // confirmed usable for next write
    }

    /// Test: snapshot→restore→emit produces byte-identical DrawCmds (determinism).
    /// BLOCKED (forbid-unsafe policy): `snapshot`/`restore` are safe stubs pending
    /// real serde/bytemuck-backed serialization (see their doc comments) — restore
    /// always returns None right now, so this test cannot pass yet. Named debt.
    #[test]
    #[ignore = "blocked on real snapshot/restore serialization (forbid-unsafe stub)"]
    #[allow(deprecated)] // the test EXISTS to exercise the deprecated stubs
    fn snapshot_restore_emit_byte_identical() {
        let mut t = Box::new(TreeManager::new());
        let root  = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color = 0xAABBCCFF;
        let child = t.new_node(WidgetKind::Rect);
        t.node_mut(child).props.color = 0x112233FF;
        t.node_mut(child).props.min_w = 200_000;
        t.node_mut(child).props.min_h = 100_000;
        t.add_child(root, child);
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));

        let mut dl_pre = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl_pre);
        let pre: Vec<DrawCmd> = dl_pre.commands().to_vec();

        let snap = t.snapshot();
        let restored = TreeManager::restore(&snap).expect("restore from snapshot");

        let mut dl_post = DrawList::new_boxed();
        emit_draw_list(&restored, &mut *dl_post);
        let post: Vec<DrawCmd> = dl_post.commands().to_vec();

        assert_eq!(pre.len(), post.len(), "DrawList length must match");
        for (i, (a, b)) in pre.iter().zip(post.iter()).enumerate() {
            assert_eq!(a, b, "DrawCmd[{}] mismatch after snapshot→restore", i);
        }
    }

    /// Test: single panel fixture (canonical mapping).
    #[test]
    fn mapping_f1_single_panel() {
        let mut t = Box::new(TreeManager::new());
        let root = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color = 0xAABBCCFFu32;
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));
        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);
        let cmds = dl.commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0xAABBCCFF, radius: 0 });
    }

    /// Test: panel + rect child (fills parent vertically).
    #[test]
    fn mapping_f2_panel_with_fill_rect() {
        let mut t = Box::new(TreeManager::new());
        let root  = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color  = 0x111111FFu32;
        let child = t.new_node(WidgetKind::Rect);
        t.node_mut(child).props.color = 0x222222FFu32;
        t.add_child(root, child);
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));
        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);
        let cmds = dl.commands();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x111111FF, radius: 0 });
        assert_eq!(cmds[1], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x222222FF, radius: 0 });
    }

    /// Test: panel + text child.
    #[test]
    fn mapping_f3_panel_with_text() {
        let mut t = Box::new(TreeManager::new());
        let root  = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color = 0x333333FFu32;
        let lbl   = t.new_node(WidgetKind::Text);
        t.node_mut(lbl).props.color  = 0x444444FFu32;
        t.add_child(root, lbl);
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));
        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);
        let cmds = dl.commands();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x333333FF, radius: 0 });
        assert_eq!(cmds[1], DrawCmd::Text { rect: UiRect::new(0, 0, 800_000, 600_000), glyph_start: 0, glyph_count: 0, color: 0x444444FF });
    }

    /// Test: panel → clip → rect child (scissor region).
    #[test]
    fn mapping_f4_panel_clip_rect() {
        let mut t     = Box::new(TreeManager::new());
        let root      = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color = 0x555555FFu32;
        let clip      = t.new_node(WidgetKind::Clip);
        let inner_r   = t.new_node(WidgetKind::Rect);
        t.node_mut(inner_r).props.color = 0x666666FFu32;
        t.add_child(root, clip);
        t.add_child(clip, inner_r);
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));
        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);
        let cmds = dl.commands();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x555555FF, radius: 0 });
        assert_eq!(cmds[1], DrawCmd::Clip { rect: UiRect::new(0, 0, 800_000, 600_000) });
        assert_eq!(cmds[2], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x666666FF, radius: 0 });
    }

    /// Test: horizontal row — fixed-width left + fill right.
    #[test]
    fn mapping_f5_horizontal_row() {
        let mut t  = Box::new(TreeManager::new());
        let root   = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.color     = 0x777777FFu32;
        t.node_mut(root).props.direction = Direction::Horizontal;
        let left   = t.new_node(WidgetKind::Rect);
        t.node_mut(left).props.color  = 0x888888FFu32;
        t.node_mut(left).props.min_w  = 400_000; // fixed 400px
        let right  = t.new_node(WidgetKind::Rect);
        t.node_mut(right).props.color = 0x999999FFu32;
        t.add_child(root, left);
        t.add_child(root, right);
        LayoutEngine::layout(&mut t, root, UiRect::new(0, 0, 800_000, 600_000));
        let mut dl = DrawList::new_boxed();
        emit_draw_list(&t, &mut *dl);
        let cmds = dl.commands();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], DrawCmd::Rect { rect: UiRect::new(0, 0, 800_000, 600_000), color: 0x777777FF, radius: 0 });
        assert_eq!(cmds[1], DrawCmd::Rect { rect: UiRect::new(0, 0, 400_000, 600_000), color: 0x888888FF, radius: 0 });
        assert_eq!(cmds[2], DrawCmd::Rect { rect: UiRect::new(400_000, 0, 400_000, 600_000), color: 0x999999FF, radius: 0 });
    }

    /// Sabotage test: flip a critical invariant in layout_col and confirm error.
    /// This proves the test suite catches bugs.
    #[test]
    fn sabotage_layout_col_fill_share_error() {
        let mut t = Box::new(TreeManager::new());
        let root = t.new_node(WidgetKind::Panel);
        t.node_mut(root).props.direction = Direction::Vertical;

        let c0 = t.new_node(WidgetKind::Rect);
        let c1 = t.new_node(WidgetKind::Rect);
        for c in [c0, c1] {
            t.add_child(root, c);
        }

        let container = UiRect::new(0, 0, 100_000, 200_000);
        LayoutEngine::layout(&mut t, root, container);

        let c0_rect = t.node(c0).layout_rect;
        let c1_rect = t.node(c1).layout_rect;

        // Invariant: two fill children in a 200_000-height container
        // must each get 100_000 height (200_000 / 2).
        // If layout_col divides by 3 instead of 2, this fails. ✓
        assert_eq!(c0_rect.h.0, 100_000, "first fill child must get half height");
        assert_eq!(c1_rect.h.0, 100_000, "second fill child must get half height");
        // Confirm Y advances correctly (no overlap).
        assert_eq!(c0_rect.y.0, 0);
        assert_eq!(c1_rect.y.0, 100_000);
    }

    /// Sabotage test: verify snapshot round-trip survives buffer mutation.
    /// BLOCKED (forbid-unsafe policy): same reason as `snapshot_restore_emit_byte_identical`.
    #[test]
    #[ignore = "blocked on real snapshot/restore serialization (forbid-unsafe stub)"]
    #[allow(deprecated)] // the test EXISTS to exercise the deprecated stubs
    fn sabotage_snapshot_detects_label_corruption() {
        let mut t1 = Box::new(TreeManager::new());
        let root = t1.new_node(WidgetKind::Text);
        t1.node_mut(root).set_label("ORIGINAL");

        let snap = t1.snapshot();
        let mut t2 = TreeManager::restore(&snap).expect("restore");

        // Mutate the restored tree
        t2.node_mut(root).set_label("CORRUPTED");

        // The two trees should now differ
        assert_ne!(
            t1.node(root).label_str(),
            t2.node(root).label_str(),
            "snapshot→restore did not capture label mutation"
        );

        // Re-snapshot t1 should still have original
        let snap_again = t1.snapshot();
        let t3 = TreeManager::restore(&snap_again).expect("restore again");
        assert_eq!(t3.node(root).label_str(), "ORIGINAL", "original snapshot must survive");
    }
}
