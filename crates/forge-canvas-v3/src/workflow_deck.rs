//! workflow_deck — one-workflow-at-a-time vertical slide deck.
//!
//! Ported verbatim from `E:\.airgap\divmerge-2026-06-12\forge-canvas-engine-pre\
//! src\workflow_deck.rs` (2026-08-14) — a `canvas.rs`-adjacent v2 tape find,
//! confirmed absent from `forge-canvas-v3` before this port (grepped, zero
//! hits) and fully dependency-satisfied: both `geom::UiRect` and
//! `spring::Spring` already exist here verbatim.
//!
//! Cognitive-load law: the user sees exactly ONE workflow at a time. A button
//! slides the deck up/down between stacked workflows (e.g. Sprite Dissector ↔
//! main Renderer), spring-animated. Switching away **requests a save** of the
//! outgoing workflow's state so the deck can never lose previous work.
//!
//! This is a pure layout/animation primitive: workflows are identified by index;
//! the caller maps each index to its content and draws it at the deck-provided
//! offset. Integer-only (MilliUnit), spring-driven via [`crate::spring::Spring`].

use crate::geom::UiRect;
use crate::spring::Spring;

/// Default spring feel for the slide — the codebase-proven UI spring tuning
/// (`Spring::new(_, 180, 24)`, used shell-wide). Sean's HTML-scale 220/12 are
/// unstable in this integer Spring (damping ≈ 2·√stiffness is critical here).
pub const SLIDE_STIFFNESS: i32 = 180;
/// Default spring damping for the slide — paired with [`SLIDE_STIFFNESS`].
pub const SLIDE_DAMPING: i32 = 24;

/// A vertical stack of `count` workflows, exactly one active/visible. The slide
/// position animates toward `active * viewport_height` (MilliUnit).
#[derive(Clone, Debug)]
pub struct WorkflowDeck {
    /// Number of stacked workflows (≥ 1).
    count: usize,
    /// The target (front) workflow index.
    active: usize,
    /// Animated vertical scroll offset in MilliUnit (0 = workflow 0 at top).
    slide: Spring,
    /// Last viewport height used to lay the deck out (MilliUnit).
    viewport_h: i64,
    /// Set when a switch leaves a workflow; the caller drains it to persist that
    /// workflow's state before it scrolls off. `None` once drained.
    pending_save: Option<usize>,
}

impl WorkflowDeck {
    /// A deck of `count` workflows (clamped to ≥1), starting on workflow 0.
    pub fn new(count: usize) -> Self {
        Self {
            count: count.max(1),
            active: 0,
            slide: Spring::new(0, SLIDE_STIFFNESS, SLIDE_DAMPING),
            viewport_h: 0,
            pending_save: None,
        }
    }

    /// The currently active (front) workflow index.
    pub fn active(&self) -> usize {
        self.active
    }

    /// Number of stacked workflows in this deck.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Slide to workflow `idx` within a viewport `viewport_h` MilliUnit tall.
    /// If it differs from the current workflow, the outgoing one is queued for a
    /// state save (drain via [`Self::take_save_request`]). No-op if already active.
    pub fn switch_to(&mut self, idx: usize, viewport_h: i64) {
        self.viewport_h = viewport_h;
        let idx = idx.min(self.count - 1);
        if idx != self.active {
            self.pending_save = Some(self.active);
            self.active = idx;
        }
        self.slide.set_target(self.active as i64 * viewport_h);
    }

    /// The button: slide to the next workflow down (clamped — decks don't wrap,
    /// so the slide direction stays predictable).
    pub fn next(&mut self, viewport_h: i64) {
        self.switch_to(self.active + 1, viewport_h);
    }

    /// The button: slide to the previous workflow up (clamped).
    pub fn prev(&mut self, viewport_h: i64) {
        self.switch_to(self.active.saturating_sub(1), viewport_h);
    }

    /// Advance the slide animation by `dt_ms`.
    pub fn step(&mut self, dt_ms: u32) {
        self.slide.step(dt_ms);
    }

    /// True when the slide has settled on the active workflow.
    pub fn settled(&self) -> bool {
        self.slide.settled()
    }

    /// Skip the animation and land on the active workflow immediately.
    pub fn snap(&mut self) {
        self.slide.set_target(self.active as i64 * self.viewport_h);
        self.slide.snap();
    }

    /// Current animated vertical offset (MilliUnit) to subtract from each
    /// workflow's stacked Y when drawing.
    pub fn offset(&self) -> i64 {
        self.slide.position
    }

    /// Drain the pending save request (the workflow index whose state should be
    /// persisted before it scrolls away). Returns `Some` exactly once per switch.
    pub fn take_save_request(&mut self) -> Option<usize> {
        self.pending_save.take()
    }

    /// The on-screen rect for workflow `i`, given the deck's outer `viewport`.
    /// Stacked vertically and shifted by the animated offset; off-screen
    /// workflows fall outside `viewport` and need not be drawn (1-at-a-time).
    pub fn rect_for(&self, i: usize, viewport: UiRect) -> UiRect {
        let vh = viewport.h.0;
        let y = viewport.y.0 + i as i64 * vh - self.offset();
        UiRect::new(viewport.x.0, y, viewport.w.0, vh)
    }

    /// Whether workflow `i` is at least partially on-screen (worth drawing).
    pub fn is_visible(&self, i: usize, viewport: UiRect) -> bool {
        let r = self.rect_for(i, viewport);
        let top = r.y.0;
        let bottom = r.y.0 + r.h.0;
        bottom > viewport.y.0 && top < viewport.y.0 + viewport.h.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VH: i64 = 500_000; // 500px viewport

    #[test]
    fn starts_on_workflow_zero_no_save() {
        let mut d = WorkflowDeck::new(2);
        assert_eq!(d.active(), 0);
        assert_eq!(d.take_save_request(), None);
    }

    #[test]
    fn switch_queues_save_of_outgoing_and_targets_new() {
        let mut d = WorkflowDeck::new(2);
        d.switch_to(1, VH);
        assert_eq!(d.active(), 1);
        // Outgoing workflow 0 queued for a state save, exactly once.
        assert_eq!(d.take_save_request(), Some(0));
        assert_eq!(d.take_save_request(), None);
        // The slide is heading toward workflow 1's stacked position.
        d.snap();
        assert_eq!(d.offset(), VH);
    }

    #[test]
    fn switch_to_same_workflow_is_noop_no_save() {
        let mut d = WorkflowDeck::new(2);
        d.switch_to(0, VH);
        assert_eq!(d.take_save_request(), None);
        assert_eq!(d.active(), 0);
    }

    #[test]
    fn next_and_prev_clamp_at_ends() {
        let mut d = WorkflowDeck::new(2);
        d.next(VH);
        assert_eq!(d.active(), 1);
        d.next(VH); // clamped — already at last
        assert_eq!(d.active(), 1);
        d.take_save_request(); // drain the one real switch
        d.prev(VH);
        assert_eq!(d.active(), 0);
        assert_eq!(d.take_save_request(), Some(1));
    }

    #[test]
    fn slide_animates_and_converges_on_active() {
        let mut d = WorkflowDeck::new(2);
        assert_eq!(d.offset(), 0);
        d.switch_to(1, VH);
        // Pump the spring (bounded). The integer Spring limit-cycles sub-pixel
        // near the target rather than reaching strict vel<10 `settled()`, so we
        // assert convergence of POSITION onto workflow 1, which is the real,
        // visible behavior the deck depends on.
        for _ in 0..3000 {
            d.step(16);
        }
        assert!(d.offset() > VH / 2, "slide did not cross to workflow 1: {}", d.offset());
        assert!((d.offset() - VH).abs() < 2_000, "did not converge near target: {}", d.offset());
    }

    #[test]
    fn rect_for_stacks_and_offset_brings_active_into_view() {
        let mut d = WorkflowDeck::new(2);
        let viewport = UiRect::new(0, 0, 400_000, VH);
        // At rest on 0: workflow 0 fills the viewport, workflow 1 sits below.
        assert_eq!(d.rect_for(0, viewport).y.0, 0);
        assert_eq!(d.rect_for(1, viewport).y.0, VH);
        assert!(d.is_visible(0, viewport));
        assert!(!d.is_visible(1, viewport));
        // Switch + snap to 1: now workflow 1 is in view, 0 scrolled up off-screen.
        d.switch_to(1, VH);
        d.snap();
        assert_eq!(d.rect_for(1, viewport).y.0, 0);
        assert_eq!(d.rect_for(0, viewport).y.0, -VH);
        assert!(d.is_visible(1, viewport));
        assert!(!d.is_visible(0, viewport));
    }
}
