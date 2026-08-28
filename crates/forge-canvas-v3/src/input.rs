//! Input routing — hover, click, drag, focus state machine.
//! Replaces egui::Response without any egui dependency.
//!
//! This module defines UI-level input state and routing, layered over
//! a local hardware-level abstraction (no dependency on v2 `forge_input` crate).
//!
//! `RawInputState` is THE per-frame hardware struct (L05 one home,
//! 2026-08-14 receipt) — a windowing host (`shell/src/main.rs`'s tao
//! window + `shell/src/pen_input.rs`'s WM_POINTER collector) writes
//! directly into it, including the pen fields, rather than a separate
//! `forge-input-v3`-owned struct: `forge-input-v3` stays quantization-only
//! (`PadQuantizer`/`WacomQuantizer`), this struct is the sink both quantized
//! streams land in.

use forge_core_v3::fixed_point::MilliUnit;
use crate::geom::UiRect;

// ── Hardware Input Events (Local Definition, no forge_input dependency) ──────

/// A unique widget identifier for input routing (hover, active, focus).
/// Zero-sized, fast to copy. Most IDs are statically assigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetId(pub u32);

impl From<u32> for WidgetId {
    /// Convert a raw u32 to a WidgetId.
    fn from(id: u32) -> Self {
        WidgetId(id)
    }
}

impl From<usize> for WidgetId {
    /// Convert a usize to a WidgetId (truncated to u32).
    fn from(id: usize) -> Self {
        WidgetId(id as u32)
    }
}

/// Raw hardware input state — pointer, keyboard, gamepad events.
/// Written by the windowing layer (OS host / platform abstraction) before layout.
/// All hardware fields are deterministic integers; no floating point, no wall-clock timestamps.
#[derive(Clone, Debug, Default)]
pub struct RawInputState {
    /// Mouse position in MilliUnits (1000 = 1px). Deterministic integer coordinates.
    pub mouse_pos: (i64, i64),
    /// Mouse button state: [left, middle, right]. true = down this frame.
    pub mouse_down: [bool; 3],
    /// Mouse button just-pressed flags: [left, middle, right]. true = pressed this frame only.
    pub mouse_just_pressed: [bool; 3],
    /// Mouse button just-released flags: [left, middle, right]. true = released this frame only.
    pub mouse_just_released: [bool; 3],
    /// Mouse delta from last frame: (dx, dy) in MilliUnits.
    pub mouse_delta: (i32, i32),
    /// Scroll wheel delta this frame: (dx, dy). Positive = scroll up/right.
    pub scroll_delta: (i32, i32),
    /// Gamepad connected flag.
    pub gamepad_connected: bool,
    /// Left stick position: [x, y] in [-1.0, 1.0] (may include floating point per-axis).
    pub gamepad_stick_left: [f32; 2],
    /// Right stick position: [x, y] in [-1.0, 1.0].
    pub gamepad_stick_right: [f32; 2],
    /// Gamepad button bitmask (0x0000 = no buttons pressed).
    pub gamepad_buttons: u16,
    /// Left trigger (0.0 = unpressed, 1.0 = fully pressed).
    pub gamepad_left_trigger: f32,
    /// Right trigger (0.0 = unpressed, 1.0 = fully pressed).
    pub gamepad_right_trigger: f32,
    /// Text input characters typed this frame (Unicode).
    pub typed_chars: Vec<char>,
    /// Virtual key codes pressed this frame (WinAPI VK_* codes).
    pub keys_pressed: Vec<u32>,
    /// Virtual key codes currently held down.
    pub keys_held: Vec<u32>,
    /// Pen pressure in Permyriad (0..=10000). 0 = no pressure / mouse input.
    /// Same convention `forge-input-v3::wacom::QuantizedTabletSample::
    /// pressure` and `forge-brush-v3::engine::BrushEngine::effective_size`'s
    /// `pressure_permyriad` param already use — the host (e.g. shell's
    /// `pen_input.rs` WM_POINTER collector) writes this directly, no
    /// adapter needed.
    pub pen_pressure: u16,
    /// Pen tilt X in degrees (-90..=90). 0 = perpendicular.
    pub pen_tilt_x: i8,
    /// Pen tilt Y in degrees (-90..=90). 0 = perpendicular.
    pub pen_tilt_y: i8,
}

impl RawInputState {
    /// Clear per-frame transient state before the next frame.
    /// Keeps persistent state (mouse_pos, keys_held, gamepad_connected).
    pub fn begin_frame(&mut self) {
        self.mouse_just_pressed = [false; 3];
        self.mouse_just_released = [false; 3];
        self.mouse_delta = (0, 0);
        self.scroll_delta = (0, 0);
        self.typed_chars.clear();
        self.keys_pressed.clear();
    }
}

// ── Cursor Shape Hint ────────────────────────────────────────────────────────

/// Cursor shape requested by the UI layer. The OS host reads this after each
/// frame and calls SetCursor() accordingly. Zero-alloc: just a Copy enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorShape {
    /// Default arrow cursor.
    #[default]
    Arrow,
    /// Hand cursor (clickable widget).
    Hand,
    /// Text input cursor (I-beam).
    Text,
    /// Horizontal resize cursor (split border).
    ResizeH,
    /// Vertical resize cursor (split border).
    ResizeV,
    /// Crosshair cursor (drawing tools).
    Crosshair,
    /// Grab cursor (dragging).
    Grab,
    /// Forbidden/no-drop cursor.
    Forbidden,
}

// ── UI-Layer Input State ─────────────────────────────────────────────────────

/// UI-layer input state = raw hardware state + widget routing.
///
/// The `raw` field is written by the windowing layer (SovereignWindow / winit).
/// The routing fields (`hovered`, `active`, `focused`) and `interact()` method
/// are UI-only concerns that live here in forge-canvas.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// Raw hardware state — mouse, keyboard, gamepad.
    /// Written by the windowing layer before layout.
    pub raw: RawInputState,

    /// Currently hovered widget (if any).
    pub hovered: Option<WidgetId>,
    /// Currently active (pressed) widget (if any).
    pub active: Option<WidgetId>,
    /// Currently focused widget (if any).
    pub focused: Option<WidgetId>,

    /// Cursor shape to display this frame (set by widgets during render).
    pub cursor: CursorShape,
}

// ── Convenience accessors — keep existing field-access patterns working ──────

impl InputState {
    /// Mouse position as MilliUnits. Delegates to raw.mouse_pos.
    #[inline]
    pub fn mouse_pos(&self) -> (MilliUnit, MilliUnit) {
        (MilliUnit(self.raw.mouse_pos.0), MilliUnit(self.raw.mouse_pos.1))
    }

    /// Reference to mouse button states: [left, middle, right].
    #[inline]
    pub fn mouse_down(&self) -> &[bool; 3] {
        &self.raw.mouse_down
    }

    /// Reference to mouse just-pressed states: [left, middle, right].
    #[inline]
    pub fn mouse_just_pressed(&self) -> &[bool; 3] {
        &self.raw.mouse_just_pressed
    }

    /// Reference to mouse just-released states: [left, middle, right].
    #[inline]
    pub fn mouse_just_released(&self) -> &[bool; 3] {
        &self.raw.mouse_just_released
    }

    /// Mouse delta from last frame (dx, dy).
    #[inline]
    pub fn scroll_delta(&self) -> (i32, i32) {
        self.raw.scroll_delta
    }

    /// Gamepad connected flag.
    #[inline]
    pub fn gamepad_connected(&self) -> bool {
        self.raw.gamepad_connected
    }

    /// Left gamepad stick position [x, y].
    #[inline]
    pub fn gamepad_stick_left(&self) -> [f32; 2] {
        self.raw.gamepad_stick_left
    }

    /// Right gamepad stick position [x, y].
    #[inline]
    pub fn gamepad_stick_right(&self) -> [f32; 2] {
        self.raw.gamepad_stick_right
    }

    /// Gamepad button bitmask.
    #[inline]
    pub fn gamepad_buttons(&self) -> u16 {
        self.raw.gamepad_buttons
    }

    /// Left gamepad trigger (0.0 = unpressed, 1.0 = fully pressed).
    #[inline]
    pub fn gamepad_left_trigger(&self) -> f32 {
        self.raw.gamepad_left_trigger
    }

    /// Right gamepad trigger (0.0 = unpressed, 1.0 = fully pressed).
    #[inline]
    pub fn gamepad_right_trigger(&self) -> f32 {
        self.raw.gamepad_right_trigger
    }

    /// Text input characters typed this frame.
    #[inline]
    pub fn typed_chars(&self) -> &[char] {
        &self.raw.typed_chars
    }

    /// Virtual key codes pressed this frame.
    #[inline]
    pub fn keys_pressed(&self) -> &[u32] {
        &self.raw.keys_pressed
    }

    /// Pen pressure in Permyriad (0..=10000). Delegates to `raw.pen_pressure`.
    #[inline]
    pub fn pen_pressure(&self) -> u16 {
        self.raw.pen_pressure
    }

    /// Pen tilt in degrees: `(x, y)`, each `-90..=90`. Delegates to
    /// `raw.pen_tilt_x`/`raw.pen_tilt_y`.
    #[inline]
    pub fn pen_tilt(&self) -> (i8, i8) {
        (self.raw.pen_tilt_x, self.raw.pen_tilt_y)
    }
}

// ── Interaction Result ───────────────────────────────────────────────────────

/// Result of an interaction query for a specific widget.
#[derive(Clone, Copy, Debug, Default)]
pub struct Interaction {
    /// Widget is hovered by the mouse.
    pub hovered: bool,
    /// Left-clicked this frame.
    pub clicked: bool,
    /// Right-clicked this frame.
    pub right_clicked: bool,
    /// Widget is being dragged (left mouse down).
    pub dragging: bool,
    /// Mouse delta during drag (dx, dy).
    pub drag_delta: (i32, i32),
    /// Scroll delta over this widget.
    pub scroll: (i32, i32),
    /// Widget has keyboard focus.
    pub focused: bool,
}

impl InputState {
    /// Query interaction state for a widget at the given rect.
    /// Call once per widget per frame. Updates internal routing state (hovered, active, focused).
    pub fn interact(&mut self, id: impl Into<WidgetId>, rect: &UiRect) -> Interaction {
        let id: WidgetId = id.into();
        let mx = MilliUnit(self.raw.mouse_pos.0);
        let my = MilliUnit(self.raw.mouse_pos.1);
        let hit = rect.contains(mx, my);

        let mut result = Interaction::default();

        if hit {
            self.hovered = Some(id);
            result.hovered = true;
            result.scroll = self.raw.scroll_delta;
        }
        if hit && self.raw.mouse_just_pressed[0] {
            self.active = Some(id);
            self.focused = Some(id);
            result.clicked = true;
        }
        if hit && self.raw.mouse_just_pressed[2] {
            result.right_clicked = true;
        }
        if self.active == Some(id) && self.raw.mouse_down[0] {
            result.dragging = true;
            result.drag_delta = self.raw.mouse_delta;
        }
        if self.active == Some(id) && self.raw.mouse_just_released[0] {
            self.active = None;
        }
        result.focused = self.focused == Some(id);

        result
    }

    /// Begin a new frame — clear per-frame state.
    /// Delegates hardware clearing to `RawInputState::begin_frame()`,
    /// then clears UI routing state.
    pub fn begin_frame(&mut self) {
        self.raw.begin_frame();
        self.hovered = None;
    }
}

// ── Focus Chain — Tab/Shift+Tab navigation ──────────────────────────────────

/// Maximum widgets in a focus chain.
pub const FOCUS_CHAIN_MAX: usize = 64;

/// Fixed-size focus order for Tab navigation. Zero-alloc, statically bounded.
#[derive(Clone, Debug)]
pub struct FocusChain {
    /// Ordered array of widget IDs (static allocation).
    order: [WidgetId; FOCUS_CHAIN_MAX],
    /// Current count of registered widgets.
    count: usize,
    /// Current focus index into the order array.
    current: usize,
}

impl Default for FocusChain {
    fn default() -> Self {
        Self { order: [WidgetId(0); FOCUS_CHAIN_MAX], count: 0, current: 0 }
    }
}

impl FocusChain {
    /// Register a widget in the focus order. Call during layout, in visual order.
    /// Does nothing if the chain is already full.
    pub fn register(&mut self, id: WidgetId) {
        if self.count < FOCUS_CHAIN_MAX {
            self.order[self.count] = id;
            self.count += 1;
        }
    }

    /// Clear registrations for next frame (call in begin_frame).
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Advance focus forward (Tab).
    pub fn advance(&mut self) {
        if self.count > 0 {
            self.current = (self.current + 1) % self.count;
        }
    }

    /// Retreat focus backward (Shift+Tab).
    pub fn retreat(&mut self) {
        if self.count > 0 {
            self.current = if self.current == 0 { self.count - 1 } else { self.current - 1 };
        }
    }

    /// Currently focused widget ID, or None if chain is empty.
    pub fn current_id(&self) -> Option<WidgetId> {
        if self.count > 0 { Some(self.order[self.current]) } else { None }
    }

    /// Point the chain's own cursor at `id` directly (a mouse click focused
    /// it, bypassing Tab/Shift+Tab). No-op if `id` isn't registered. Keeps
    /// a subsequent Tab press continuing from where the click landed instead
    /// of the chain's last Tab-driven position.
    pub fn focus(&mut self, id: WidgetId) {
        if let Some(pos) = self.order[..self.count].iter().position(|&w| w == id) {
            self.current = pos;
        }
    }

    /// Process Tab/Shift+Tab from InputState. Returns new focused WidgetId if changed.
    pub fn process(&mut self, input: &InputState) -> Option<WidgetId> {
        const VK_TAB: u32 = 0x09;
        const VK_SHIFT: u32 = 0x10;
        if input.raw.keys_pressed.contains(&VK_TAB) {
            if input.raw.keys_held.contains(&VK_SHIFT) {
                self.retreat();
            } else {
                self.advance();
            }
            return self.current_id();
        }
        None
    }
}

// ── Drag-and-Drop State ──────────────────────────────────────────────────────

/// Drag-and-drop payload state. One per frame, stored in shell.
#[derive(Clone, Debug, Default)]
pub struct DragState {
    /// Drag operation is active.
    pub active: bool,
    /// App-defined type tag for payload compatibility.
    pub payload_type: u16,
    /// App-defined ID of the thing being dragged.
    pub payload_id: u64,
    /// Origin position (MilliUnit).
    pub origin: (i64, i64),
    /// Current mouse position during drag (MilliUnit).
    pub current: (i64, i64),
}

impl DragState {
    /// Begin a drag operation. Call from a drag source widget.
    pub fn begin(&mut self, payload_type: u16, payload_id: u64, origin: (i64, i64)) {
        self.active = true;
        self.payload_type = payload_type;
        self.payload_id = payload_id;
        self.origin = origin;
        self.current = origin;
    }

    /// Update drag position. Call each frame while active.
    pub fn update(&mut self, mouse_pos: (i64, i64)) {
        if self.active {
            self.current = mouse_pos;
        }
    }

    /// Check if a drop target accepts this drag. Returns payload_id if compatible and released.
    pub fn check_drop(&mut self, rect: &UiRect, accepts_type: u16, mouse_released: bool) -> Option<u64> {
        if !self.active || self.payload_type != accepts_type {
            return None;
        }
        let hit = rect.contains_raw(self.current.0, self.current.1);
        if hit && mouse_released {
            let id = self.payload_id;
            self.cancel();
            return Some(id);
        }
        None
    }

    /// Is the drag hovering over this rect with a compatible type?
    pub fn is_hovering(&self, rect: &UiRect, accepts_type: u16) -> bool {
        self.active && self.payload_type == accepts_type
            && rect.contains_raw(self.current.0, self.current.1)
    }

    /// Cancel the drag.
    pub fn cancel(&mut self) {
        self.active = false;
        self.payload_type = 0;
        self.payload_id = 0;
    }

    /// Render ghost rect during drag: a 32x32px rect centered on the current
    /// drag position, filled with `COLOR_DRAG_ACCENT_FILL`, outlined with
    /// `COLOR_DRAG_ACCENT` (1px, 4px radius) — exactly this fn's own
    /// long-standing doc spec, landed now that `crate::draw` (always
    /// available) is no longer a reason to defer it. No-op while inactive.
    pub fn render_ghost(&self, draw: &mut crate::draw::DrawList) {
        if !self.active {
            return;
        }
        const HALF_PX: i64 = 16_000; // 16px half-size, MilliUnit (1000 = 1px)
        let rect = UiRect::new(
            self.current.0 - HALF_PX,
            self.current.1 - HALF_PX,
            HALF_PX * 2,
            HALF_PX * 2,
        );
        draw.push(crate::draw::DrawCmd::Rect {
            rect,
            color: crate::widgets::COLOR_DRAG_ACCENT_FILL,
            radius: 4,
        });
        draw.push(crate::draw::DrawCmd::RoundedOutline {
            rect,
            color: crate::widgets::COLOR_DRAG_ACCENT,
            thickness: 1,
            radius: 4,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `render_ghost` is a no-op while inactive (no stray draws from idle
    /// state), and while dragging emits exactly the fill+outline pair its
    /// own doc comment specifies, centered on `current`.
    #[test]
    fn render_ghost_draws_centered_pair_only_while_active() {
        let mut drag = DragState::default();
        let mut draw = crate::draw::DrawList::new_boxed();

        drag.render_ghost(&mut draw);
        assert_eq!(draw.commands().len(), 0, "inactive drag must emit nothing");

        drag.begin(1, 42, (100_000, 100_000));
        drag.update((200_000, 300_000));
        drag.render_ghost(&mut draw);

        let cmds = draw.commands();
        assert_eq!(cmds.len(), 2, "active drag must emit exactly fill + outline");
        match cmds[0] {
            crate::draw::DrawCmd::Rect { rect, color, radius } => {
                assert_eq!(rect.x.0, 200_000 - 16_000, "rect must be centered on current.x");
                assert_eq!(rect.y.0, 300_000 - 16_000, "rect must be centered on current.y");
                assert_eq!(rect.w.0, 32_000, "ghost must be 32px wide");
                assert_eq!(rect.h.0, 32_000, "ghost must be 32px tall");
                assert_eq!(color, crate::widgets::COLOR_DRAG_ACCENT_FILL);
                assert_eq!(radius, 4);
            }
            other => panic!("expected Rect fill, got {other:?}"),
        }
        match cmds[1] {
            crate::draw::DrawCmd::RoundedOutline { color, thickness, radius, .. } => {
                assert_eq!(color, crate::widgets::COLOR_DRAG_ACCENT);
                assert_eq!(thickness, 1);
                assert_eq!(radius, 4);
            }
            other => panic!("expected RoundedOutline, got {other:?}"),
        }
    }

    #[test]
    fn hover_detection() {
        let mut input = InputState::default();
        input.raw.mouse_pos = (500, 500);

        let rect = UiRect::new(0, 0, 1000, 1000);
        let result = input.interact(1u32, &rect);
        assert!(result.hovered, "point inside rect should hover");
    }

    #[test]
    fn click_detection() {
        let mut input = InputState::default();
        input.raw.mouse_pos = (500, 500);
        input.raw.mouse_just_pressed = [true, false, false];

        let rect = UiRect::new(0, 0, 1000, 1000);
        let result = input.interact(1u32, &rect);
        assert!(result.clicked, "left-click inside rect should register");
        assert_eq!(input.active, Some(WidgetId(1)));
        assert_eq!(input.focused, Some(WidgetId(1)));
    }

    #[test]
    fn miss_no_interaction() {
        let mut input = InputState::default();
        input.raw.mouse_pos = (5000, 5000);
        input.raw.mouse_just_pressed = [true, false, false];

        let rect = UiRect::new(0, 0, 1000, 1000);
        let result = input.interact(1u32, &rect);
        assert!(!result.hovered, "point far outside rect should not hover");
        assert!(!result.clicked, "click outside rect should not register");
    }

    // ── L07-style determinism: interact() idempotence ──────────────────────
    // Calling interact() twice with the same input must yield the same result
    // (except for state side-effects, which are deterministic).
    #[test]
    fn interact_is_deterministic() {
        let mut input = InputState::default();
        input.raw.mouse_pos = (500, 500);
        input.raw.mouse_just_pressed = [true, false, false];

        let rect = UiRect::new(0, 0, 1000, 1000);
        let result1 = input.interact(1u32, &rect);

        // Reset to same state and call again
        input.hovered = None;
        input.active = None;
        input.focused = None;
        input.raw.mouse_just_pressed = [true, false, false];

        let result2 = input.interact(1u32, &rect);
        assert_eq!(result1.hovered, result2.hovered);
        assert_eq!(result1.clicked, result2.clicked);
    }

    // ── L18-style sabotage: interaction state transitions ──────────────────
    // If we accidentally made active() always return true regardless of click,
    // the test would fail. We verify the invariant: active is set only on click.
    #[test]
    fn interaction_state_sabotage_test() {
        let mut input = InputState::default();
        input.raw.mouse_pos = (500, 500);
        // No click: mouse_just_pressed = [false, false, false]

        let rect = UiRect::new(0, 0, 1000, 1000);
        let result = input.interact(1u32, &rect);

        // Hovering but not clicking should NOT set active
        assert!(result.hovered);
        assert!(!result.clicked);
        assert_eq!(input.active, None, "active must only be set on actual click");
    }

    #[test]
    fn focus_chain_tab_navigation() {
        let mut chain = FocusChain::default();
        chain.register(WidgetId(10));
        chain.register(WidgetId(20));
        chain.register(WidgetId(30));

        assert_eq!(chain.current_id(), Some(WidgetId(10)), "initial focus is first widget");

        chain.advance();
        assert_eq!(chain.current_id(), Some(WidgetId(20)), "tab advances to next");

        chain.advance();
        assert_eq!(chain.current_id(), Some(WidgetId(30)));

        chain.advance();
        assert_eq!(chain.current_id(), Some(WidgetId(10)), "wraps around to first");
    }

    #[test]
    fn focus_chain_shift_tab_navigation() {
        let mut chain = FocusChain::default();
        chain.register(WidgetId(10));
        chain.register(WidgetId(20));
        chain.register(WidgetId(30));

        chain.advance(); // Move to 20
        chain.advance(); // Move to 30

        chain.retreat();
        assert_eq!(chain.current_id(), Some(WidgetId(20)), "shift+tab goes backward");

        chain.retreat();
        assert_eq!(chain.current_id(), Some(WidgetId(10)));

        chain.retreat();
        assert_eq!(chain.current_id(), Some(WidgetId(30)), "wraps backward to last");
    }

    #[test]
    fn drag_state_basic() {
        let mut drag = DragState::default();
        assert!(!drag.active, "drag starts inactive");

        drag.begin(1, 100, (1000, 2000));
        assert!(drag.active);
        assert_eq!(drag.payload_type, 1);
        assert_eq!(drag.payload_id, 100);
        assert_eq!(drag.origin, (1000, 2000));

        drag.update((1500, 2500));
        assert_eq!(drag.current, (1500, 2500));

        drag.cancel();
        assert!(!drag.active);
    }

    #[test]
    fn drag_state_type_filtering() {
        let mut drag = DragState::default();
        drag.begin(1, 42, (0, 0));

        let rect = UiRect::new(0, 0, 1000, 1000);

        // Try to drop on a rect that only accepts type 2
        let result = drag.check_drop(&rect, 2, true);
        assert_eq!(result, None, "type mismatch should not drop");
        assert!(drag.active, "drag remains active on type mismatch");

        // Now try with matching type
        let result = drag.check_drop(&rect, 1, true);
        assert_eq!(result, Some(42), "matching type and released should drop");
        assert!(!drag.active, "drag cleared after successful drop");
    }

    #[test]
    fn raw_input_state_begin_frame_clears_transient() {
        let mut raw = RawInputState::default();
        raw.mouse_just_pressed = [true, false, false];
        raw.mouse_delta = (10, 20);
        raw.scroll_delta = (5, 0);
        raw.typed_chars.push('a');
        raw.keys_pressed.push(65); // 'A'

        raw.begin_frame();

        assert_eq!(raw.mouse_just_pressed, [false; 3], "just_pressed cleared");
        assert_eq!(raw.mouse_delta, (0, 0), "delta cleared");
        assert_eq!(raw.scroll_delta, (0, 0), "scroll cleared");
        assert!(raw.typed_chars.is_empty(), "typed_chars cleared");
        assert!(raw.keys_pressed.is_empty(), "keys_pressed cleared");
    }

    /// A live pen sample (host-written, e.g. from a WM_POINTER collector)
    /// reads back through `InputState`'s accessors — the wire this fix
    /// exists for actually works end to end.
    #[test]
    fn pen_fields_read_back_through_input_state() {
        let mut input = InputState::default();
        assert_eq!(input.pen_pressure(), 0, "no pen activity yet");
        assert_eq!(input.pen_tilt(), (0, 0));

        input.raw.pen_pressure = 7500;
        input.raw.pen_tilt_x = -30;
        input.raw.pen_tilt_y = 45;

        assert_eq!(input.pen_pressure(), 7500);
        assert_eq!(input.pen_tilt(), (-30, 45));
    }

    /// Pen state is persistent-until-updated (like `mouse_pos`), not a
    /// per-frame edge — `begin_frame` must not zero it, matching v2's own
    /// convention (the host explicitly zeroes pressure on WM_POINTERUP,
    /// `begin_frame` never touches it).
    #[test]
    fn begin_frame_does_not_clear_pen_state() {
        let mut input = InputState::default();
        input.raw.pen_pressure = 4200;
        input.raw.pen_tilt_x = 12;
        input.begin_frame();
        assert_eq!(input.pen_pressure(), 4200, "pen pressure persists across begin_frame");
        assert_eq!(input.pen_tilt(), (12, 0));
    }

    #[test]
    fn widget_id_conversions() {
        let id1: WidgetId = 42u32.into();
        assert_eq!(id1, WidgetId(42));

        let id2: WidgetId = 100usize.into();
        assert_eq!(id2, WidgetId(100));
    }
}
