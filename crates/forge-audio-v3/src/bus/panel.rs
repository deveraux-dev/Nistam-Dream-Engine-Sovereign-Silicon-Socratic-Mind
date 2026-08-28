//! Panel trait, PanelEntry, TickContext, and RenderContext.
//!
//! These types define the shared panel abstraction used by the unified binary.
//! Every studio module (DAW, HUD, Studio, Broski, NDE, Admin) implements `Panel`.

use super::bus::{AudioBusHandle, HubTapeStat};

/// Panel-agnostic view model for the master-bus UMP flight recorder, rendered as a
/// scrubber/REC readout in EVERY panel (ZD-003 timeline-scrubber). Built from the
/// shared `hub_tape` stat that rides the `AudioBusHandle` every panel already holds,
/// so no panel needs bespoke wiring — it inherits [`Panel::hub_tape_bar`] for free.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HubTapeBar {
    /// Sealed command moments available to scrub.
    pub moments: u64,
    /// Total commands captured onto the tape.
    pub events: u64,
    /// `content_seal` of the latest moment — the scrubber playhead identity.
    pub last_seal: u64,
    /// Master-bus frame of the latest seal.
    pub last_tick: u64,
}

impl HubTapeBar {
    #[inline]
    pub fn from_stat(s: HubTapeStat) -> Self {
        Self { moments: s.moments, events: s.events, last_seal: s.last_seal, last_tick: s.last_tick }
    }

    /// True once the recorder has captured at least one command — drives the REC dot.
    #[inline]
    pub fn is_live(self) -> bool {
        self.events > 0
    }

    /// One-line status string for a panel's chrome (e.g. a title-bar readout).
    /// The dot is DRAWN, not spelled (2026-07-30): `vix_runtime::draw_hub_tape_bar`
    /// already pushes a real REC dot quad beside this label, and U+25CF/U+25CB are
    /// outside the studio atlas — so the strip shipped a tofu box next to a
    /// perfectly good dot. One mark, one drawer (bind XOR hand-draw).
    pub fn label(self) -> String {
        format!(
            "REC {} moments · {} cmd · seal {:08x}",
            self.moments,
            self.events,
            self.last_seal as u32,
        )
    }
}

/// Context passed to every panel's `tick()` method each frame.
pub struct TickContext {
    /// Time delta since last frame in seconds.
    pub dt: f64,
    /// Monotonic frame counter.
    pub frame_number: u64,
    /// Shared audio bus handle for reading snapshots and sending commands.
    pub bus: AudioBusHandle,
}

/// Context passed to visible panels during `render()`.
///
/// We keep this lightweight — egui/wgpu references will be added later
/// when wiring the actual render loop.
pub struct RenderContext {
    /// Shared audio bus handle for reading snapshots and sending commands.
    pub bus: AudioBusHandle,
    /// Time delta since last frame in seconds.
    pub dt: f64,
}

/// A persistent studio panel that is created once and toggled visible/invisible.
///
/// Panels are never destroyed during the application lifetime.
/// `tick()` is called every frame regardless of visibility (for background work).
/// `render()` is called only when the panel is visible.
pub trait Panel: Send {
    /// Unique panel identifier (e.g. "daw", "hud", "studio").
    fn id(&self) -> &str;

    /// Human-readable name for UI display.
    fn display_name(&self) -> &str;

    /// Optional keyboard shortcut string (e.g. "Ctrl+1").
    fn hotkey(&self) -> Option<&str> {
        None
    }

    /// Called every frame regardless of visibility.
    /// Use for background work (e.g. Broski dream cycle, playlist auto-advance).
    fn tick(&mut self, ctx: &TickContext);

    /// Called only when visible. Renders the panel UI.
    fn render(&mut self, ctx: &RenderContext);

    /// The master-bus flight-recorder readout for THIS panel. Default impl reads the
    /// shared `hub_tape` off the bus handle, so every panel surfaces the same live
    /// scrubber state without per-panel wiring. Panels that draw pixels (forge-gui
    /// kits) call this to place the REC dot / scrub bar in their chrome.
    fn hub_tape_bar(&self, ctx: &RenderContext) -> HubTapeBar {
        HubTapeBar::from_stat(**ctx.bus.hub_tape.load())
    }
}

/// A panel entry in the PanelManager, wrapping a `Panel` with metadata.
pub struct PanelEntry {
    pub panel: Box<dyn Panel>,
    /// Whether this panel is currently visible (rendered).
    pub visible: bool,
    /// Display order (lower = earlier in tick/render iteration).
    pub order: u32,
}
