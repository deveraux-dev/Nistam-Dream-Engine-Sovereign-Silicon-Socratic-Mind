//! PanelManager — hosts all panels, manages visibility, drives tick/render.
//!
//! Panels are persistent: created once at startup, toggled visible/invisible,
//! never destroyed. The audio feeder thread runs independently of panel state.

use super::panel::{Panel, PanelEntry, RenderContext, TickContext};

/// Manages all panels in the unified application.
///
/// Panels are stored in push order. `tick_all` iterates every panel;
/// `render_visible` iterates only those with `visible == true`.
pub struct PanelManager {
    panels: Vec<PanelEntry>,
    active_panel: Option<usize>,
}

impl PanelManager {
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            active_panel: None,
        }
    }

    /// Add a panel with an initial visibility flag.
    ///
    /// The panel's order is set to its push index.
    pub fn push(&mut self, panel: Box<dyn Panel>, visible: bool) {
        let order = self.panels.len() as u32;
        self.panels.push(PanelEntry {
            panel,
            visible,
            order,
        });
    }

    /// Switch to the panel with the given id.
    ///
    /// Sets the target panel visible, all others invisible, and updates
    /// `active_panel` to point to the target. If `panel_id` is not found,
    /// this is a no-op.
    ///
    /// No MixerCommands are sent — the audio feeder thread is unaffected.
    pub fn switch_to(&mut self, panel_id: &str) {
        for (i, entry) in self.panels.iter_mut().enumerate() {
            if entry.panel.id() == panel_id {
                entry.visible = true;
                self.active_panel = Some(i);
            } else {
                entry.visible = false;
            }
        }
    }

    /// Toggle the visibility of the panel with the given id.
    ///
    /// If `panel_id` is not found, this is a no-op.
    pub fn toggle(&mut self, panel_id: &str) {
        for entry in &mut self.panels {
            if entry.panel.id() == panel_id {
                entry.visible = !entry.visible;
                break;
            }
        }
    }

    /// Tick every panel regardless of visibility.
    ///
    /// Iteration order matches push order. Invisible panels are ticked
    /// so they can perform background work.
    pub fn tick_all(&mut self, ctx: &TickContext) {
        for entry in &mut self.panels {
            entry.panel.tick(ctx);
        }
    }

    /// Render only visible panels.
    ///
    /// Iteration order matches push order. Invisible panels are skipped.
    pub fn render_visible(&mut self, ctx: &RenderContext) {
        for entry in &mut self.panels {
            if entry.visible {
                entry.panel.render(ctx);
            }
        }
    }

    /// Return the id of the currently active panel, if any.
    pub fn active_panel_id(&self) -> Option<&str> {
        self.active_panel
            .and_then(|i| self.panels.get(i))
            .map(|entry| entry.panel.id())
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Return a reference to the panels vec (for testing/inspection).
    pub fn panels(&self) -> &[PanelEntry] {
        &self.panels
    }
}

impl Default for PanelManager {
    fn default() -> Self {
        Self::new()
    }
}
