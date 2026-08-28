//! Concrete panel implementations for the unified binary.
//!
//! Each panel holds an `AudioBusHandle` and panel-specific state.
//! Tick and render are stubs — real UI wiring happens in forge-gui.

use super::bus::AudioBusHandle;
use super::panel::{Panel, RenderContext, TickContext};

// ---------------------------------------------------------------------------
// DawPanel
// ---------------------------------------------------------------------------

/// DAW cockpit — 4-deck mixer, ghost panels, S2 controller.
pub struct DawPanel {
    bus: AudioBusHandle,
}

impl DawPanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self { bus }
    }
}

impl Panel for DawPanel {
    fn id(&self) -> &str {
        "daw"
    }
    fn display_name(&self) -> &str {
        "DAW Cockpit"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+1")
    }
    fn tick(&mut self, _ctx: &TickContext) {}
    fn render(&mut self, _ctx: &RenderContext) {}
}

// ---------------------------------------------------------------------------
// HudPanel
// ---------------------------------------------------------------------------

/// NeuroHUD — visualizers, playlist, command input.
pub struct HudPanel {
    bus: AudioBusHandle,
    pub last_rms: f32,
}

impl HudPanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self { bus, last_rms: 0.0 }
    }
}

impl Panel for HudPanel {
    fn id(&self) -> &str {
        "hud"
    }
    fn display_name(&self) -> &str {
        "NeuroHUD"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+2")
    }
    fn tick(&mut self, _ctx: &TickContext) {
        let snap = self.bus.snapshot.load();
        if !snap.waveform_buffer.is_empty() {
            let sum_sq: f32 = snap.waveform_buffer.iter().map(|s| s * s).sum();
            self.last_rms = (sum_sq / snap.waveform_buffer.len() as f32).sqrt();
        } else {
            self.last_rms = 0.0;
        }
    }
    fn render(&mut self, _ctx: &RenderContext) {}
}

// ---------------------------------------------------------------------------
// StudioPanel
// ---------------------------------------------------------------------------

/// Studio viewport — terrain, asset editing.
pub struct StudioPanel {
    bus: AudioBusHandle,
    /// Audio energy folded through a vested-decay forgetting curve (forge-core) —
    /// the viewport's ambient throb: rises with the room, forgets when it quiets.
    pulse: crate::vested_decay::VestedDecay<u8>,
    /// Latest ambient pulse, permyriad — the viewport reads this (cf. HudPanel::last_rms).
    pub last_pulse: u32,
}

impl StudioPanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self {
            bus,
            // ~0.5s half-life at 60fps (30 frames), forget past ~4s (240 frames).
            pulse: crate::vested_decay::VestedDecay::new(2, 30, 240, 40),
            last_pulse: 0,
        }
    }

    /// Fold one frame's audio energy (permyriad) into the ambient pulse, prune the
    /// forgotten, and publish `last_pulse`. Shared by `tick` and its test so both
    /// drive the exact same forgetting path.
    pub fn fold_pulse(&mut self, energy_pmy: u32, frame: u64) {
        if energy_pmy > 0 {
            self.pulse.observe(0u8, frame, energy_pmy);
        }
        self.pulse.prune(frame);
        self.last_pulse = self.pulse.total(frame);
    }
}

impl Panel for StudioPanel {
    fn id(&self) -> &str {
        "studio"
    }
    fn display_name(&self) -> &str {
        "Studio"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+3")
    }
    fn tick(&mut self, ctx: &TickContext) {
        // The bus snapshot IS the low-byte live poll: read the room's audio energy
        // (master RMS, as HudPanel does) and fold it through the forgetting curve.
        let snap = self.bus.snapshot.load();
        let energy = if snap.waveform_buffer.is_empty() {
            0.0
        } else {
            let sum_sq: f32 = snap.waveform_buffer.iter().map(|s| s * s).sum();
            (sum_sq / snap.waveform_buffer.len() as f32).sqrt()
        };
        let energy_pmy =
            (energy.clamp(0.0, 1.0) * crate::vested_decay::STRENGTH_MAX as f32) as u32;
        self.fold_pulse(energy_pmy, ctx.frame_number);
    }
    fn render(&mut self, _ctx: &RenderContext) {}
}

// ---------------------------------------------------------------------------
// BroskiPanel
// ---------------------------------------------------------------------------

/// Broski AI personality layer — the DJ decision-tree brain
/// (`crate::broski::transition::DjAssistant`) reading the live mixer bus and
/// emitting `DjNotification` receipts each tick (Chunk 4, Broski board-clear
/// 2026-08-17). No pixel work here — that's the shell's `broski_layer`
/// compositor-layer face (Chunk 2B), a separate crate/binary.
pub struct BroskiPanel {
    bus: AudioBusHandle,
    assistant: crate::broski::transition::DjAssistant,
    /// Wall-clock seconds since this panel started ticking — `DjAssistant::tick`
    /// wants a monotonic time axis, not a frame count (its 32-bar suggestion
    /// cadence is time-gated, not frame-gated).
    elapsed_secs: f64,
    /// The last suggestion's receipt, for `render()` to surface without
    /// re-running the decision tree (tick/render can run at different rates).
    last_notification: Option<crate::broski::types::DjNotification>,
}

impl BroskiPanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self {
            bus,
            assistant: crate::broski::transition::DjAssistant::new(),
            elapsed_secs: 0.0,
            last_notification: None,
        }
    }

    /// Map the live bus snapshot onto the DJ assistant's reduced observation
    /// state. Derives real metrics from Camelot compatibility, BPM alignment,
    /// and deck energy levels.
    fn observe(&self) -> crate::broski::transition::TransitionState {
        let snap = self.bus.snapshot.load();
        let deck_a = snap.decks.first();
        let deck_b = snap.decks.get(1);

        let key_a = deck_a.map(|d| d.key.as_str()).unwrap_or("");
        let key_b = deck_b.map(|d| d.key.as_str()).unwrap_or("");
        let harmonic_compat = crate::correspondence_bus::camelot_compat(key_a, key_b);

        let bpm_a = deck_a.map(|d| d.bpm).unwrap_or(snap.bpm);
        let bpm_b = deck_b.map(|d| d.bpm).unwrap_or(snap.bpm);
        let groove_lock = if bpm_a > 0.0 && bpm_b > 0.0 {
            let diff = (bpm_a - bpm_b).abs();
            (1.0 - (diff / ((bpm_a + bpm_b) * 0.5)).min(1.0)) as f32
        } else {
            1.0
        };

        let rms_a = deck_a.map(|d| d.rms_level).unwrap_or(0.0);
        let rms_b = deck_b.map(|d| d.rms_level).unwrap_or(0.0);

        crate::broski::transition::TransitionState {
            energy_left: rms_a,
            energy_right: rms_b,
            combined_energy: (rms_a + rms_b) / 2.0,
            crossfader: (snap.crossfader + 1.0) / 2.0,
            deck_a_bpm: bpm_a,
            deck_b_bpm: bpm_b,
            harmonic_compat,
            vocal_collision: 0.0,
            groove_lock,
            vocal_energy: [0.0; 4],
        }
    }
}

impl Panel for BroskiPanel {
    fn id(&self) -> &str {
        "broski"
    }
    fn display_name(&self) -> &str {
        "Broski"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+4")
    }
    fn tick(&mut self, ctx: &TickContext) {
        self.elapsed_secs += ctx.dt;
        let state = self.observe();
        if let Some(suggestion) = self.assistant.tick(&state, self.elapsed_secs) {
            use crate::broski::transition::DjSuggestion;
            use crate::broski::types::DjNotification;
            self.last_notification = Some(match suggestion {
                DjSuggestion::ObserveOnly => return,
                DjSuggestion::SuggestTransition { .. } => {
                    DjNotification::Suggestion("transition suggested".to_string())
                }
                DjSuggestion::FlagBeatMismatch { deck_a_bpm, deck_b_bpm } => DjNotification::Suggestion(
                    format!("beat mismatch: {deck_a_bpm:.1} vs {deck_b_bpm:.1} BPM"),
                ),
                DjSuggestion::InjectChaos { action } => {
                    DjNotification::Suggestion(format!("chaos: {action}"))
                }
                DjSuggestion::EnforceRule { rule, violation } => {
                    DjNotification::Suggestion(format!("rule {rule}: {violation}"))
                }
                DjSuggestion::StemMuteVocal(deck) => {
                    DjNotification::Suggestion(format!("suggest stem-mute vocal on {deck:?}"))
                }
                DjSuggestion::SuggestKey { compatible } => {
                    DjNotification::Suggestion(format!("compatible keys: {compatible:?}"))
                }
                DjSuggestion::GrooveNudge { target_bpm } => {
                    DjNotification::Suggestion(format!("groove nudge -> {target_bpm:.1} BPM"))
                }
                DjSuggestion::FlagVocalCollision { collision } => {
                    DjNotification::Suggestion(format!("vocal collision {collision:.2}"))
                }
                DjSuggestion::FlagKeyClash { compat } => {
                    DjNotification::Suggestion(format!("key clash, compat {compat:.2}"))
                }
            });
        }
    }
    fn render(&mut self, _ctx: &RenderContext) {
        if let Some(notification) = &self.last_notification {
            eprintln!("[broski] {notification:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// NdePanel
// ---------------------------------------------------------------------------

/// NDE byte scanner.
pub struct NdePanel {
    bus: AudioBusHandle,
}

impl NdePanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self { bus }
    }
}

impl Panel for NdePanel {
    fn id(&self) -> &str {
        "nde"
    }
    fn display_name(&self) -> &str {
        "NDE Scanner"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+5")
    }
    fn tick(&mut self, _ctx: &TickContext) {}
    fn render(&mut self, _ctx: &RenderContext) {}
}

// ---------------------------------------------------------------------------
// AdminPanel
// ---------------------------------------------------------------------------

/// Admin / diagnostics panel.
pub struct AdminPanel {
    bus: AudioBusHandle,
}

impl AdminPanel {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self { bus }
    }
}

impl Panel for AdminPanel {
    fn id(&self) -> &str {
        "admin"
    }
    fn display_name(&self) -> &str {
        "Admin"
    }
    fn hotkey(&self) -> Option<&str> {
        Some("Ctrl+6")
    }
    fn tick(&mut self, _ctx: &TickContext) {}
    fn render(&mut self, _ctx: &RenderContext) {}
}

// ---------------------------------------------------------------------------
// Frame loop helper
// ---------------------------------------------------------------------------

/// Run one frame: compute dt, tick all panels, render visible panels.
pub fn run_frame(
    pm: &mut super::panel_manager::PanelManager,
    bus: &AudioBusHandle,
    last_instant: &mut std::time::Instant,
    frame_number: &mut u64,
) {
    let now = std::time::Instant::now();
    let dt = now.duration_since(*last_instant).as_secs_f64();
    *last_instant = now;
    *frame_number += 1;

    let tick_ctx = super::panel::TickContext {
        dt,
        frame_number: *frame_number,
        bus: bus.clone(),
    };
    pm.tick_all(&tick_ctx);

    let render_ctx = super::panel::RenderContext {
        bus: bus.clone(),
        dt,
    };
    pm.render_visible(&render_ctx);
}

// ---------------------------------------------------------------------------
// Unified panel factory
// ---------------------------------------------------------------------------

/// Create all 6 panels and push them into a new `PanelManager`.
pub fn create_unified_panels(bus: &AudioBusHandle) -> super::panel_manager::PanelManager {
    let mut pm = super::panel_manager::PanelManager::new();
    pm.push(Box::new(DawPanel::new(bus.clone())), true);
    pm.push(Box::new(HudPanel::new(bus.clone())), false);
    pm.push(Box::new(StudioPanel::new(bus.clone())), false);
    pm.push(Box::new(BroskiPanel::new(bus.clone())), false);
    pm.push(Box::new(NdePanel::new(bus.clone())), false);
    pm.push(Box::new(AdminPanel::new(bus.clone())), false);
    pm
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::bus::AudioBus;

    fn make_bus() -> AudioBusHandle {
        AudioBus::spawn_stub().expect("AudioBus::spawn failed in test")
    }

    #[test]
    fn panel_ids_are_unique() {
        let bus = make_bus();
        let panels: Vec<Box<dyn Panel>> = vec![
            Box::new(DawPanel::new(bus.clone())),
            Box::new(HudPanel::new(bus.clone())),
            Box::new(StudioPanel::new(bus.clone())),
            Box::new(BroskiPanel::new(bus.clone())),
            Box::new(NdePanel::new(bus.clone())),
            Box::new(AdminPanel::new(bus.clone())),
        ];
        let ids: Vec<&str> = panels.iter().map(|p| p.id()).collect();
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "panel ids must be unique");
        let _ = bus.cmd_tx.send(super::super::command::MixerCommand::Shutdown);
    }

    #[test]
    fn every_panel_surfaces_the_hub_tape_bar() {
        // "Implemented in every panel": each of the 6 panels inherits the default
        // `hub_tape_bar` readout off the shared bus handle. Drive a command through
        // the master bus, then assert every panel sees the SAME live tape.
        let bus = make_bus();
        for _ in 0..4 {
            bus.cmd_tx
                .send(super::super::command::MixerCommand::SetMasterVolume { volume: 0.5 })
                .unwrap();
        }
        // Let the feeder thread seal at least one command moment.
        let mut live = false;
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            if bus.hub_tape.load().events > 0 {
                live = true;
                break;
            }
        }
        assert!(live, "feeder must seal the master-bus command onto the tape");

        let ctx = super::super::panel::RenderContext { bus: bus.clone(), dt: 0.0 };
        let panels: Vec<Box<dyn Panel>> = vec![
            Box::new(DawPanel::new(bus.clone())),
            Box::new(HudPanel::new(bus.clone())),
            Box::new(StudioPanel::new(bus.clone())),
            Box::new(BroskiPanel::new(bus.clone())),
            Box::new(NdePanel::new(bus.clone())),
            Box::new(AdminPanel::new(bus.clone())),
        ];
        for p in &panels {
            let bar = p.hub_tape_bar(&ctx);
            assert!(bar.is_live(), "panel '{}' must see a live tape", p.id());
            assert!(bar.events > 0, "panel '{}' must see captured commands", p.id());
            // The label contract is panel.rs:43 — "REC {moments} moments · {n} cmd · seal {:08x}".
            assert!(bar.label().starts_with("REC "), "panel '{}' label must show REC", p.id());
        }
        let _ = bus.cmd_tx.send(super::super::command::MixerCommand::Shutdown);
    }

    #[test]
    fn panel_hotkeys_match_spec() {
        let bus = make_bus();
        assert_eq!(DawPanel::new(bus.clone()).hotkey(), Some("Ctrl+1"));
        assert_eq!(HudPanel::new(bus.clone()).hotkey(), Some("Ctrl+2"));
        assert_eq!(StudioPanel::new(bus.clone()).hotkey(), Some("Ctrl+3"));
        assert_eq!(BroskiPanel::new(bus.clone()).hotkey(), Some("Ctrl+4"));
        assert_eq!(NdePanel::new(bus.clone()).hotkey(), Some("Ctrl+5"));
        assert_eq!(AdminPanel::new(bus.clone()).hotkey(), Some("Ctrl+6"));
        let _ = bus.cmd_tx.send(super::super::command::MixerCommand::Shutdown);
    }

    // VERIFY: the StudioPanel ambient pulse rises while the room has audio energy,
    // then decays to zero once it goes quiet — the same forgetting curve the world
    // pulse uses, now the studio viewport's ambient throb. Drives the SAME
    // fold_pulse path tick() runs.
    #[test]
    fn studio_ambient_pulse_rises_with_audio_then_forgets() {
        let bus = make_bus();
        let mut studio = StudioPanel::new(bus.clone());
        assert_eq!(studio.last_pulse, 0, "a silent studio has no ambient pulse");
        // Audio in the room: fold energy for 40 frames — the pulse rises.
        let mut peak = 0u32;
        for f in 0..40u64 {
            studio.fold_pulse(6_000, f);
            peak = peak.max(studio.last_pulse);
        }
        assert!(peak > 0, "audio energy raises the ambient pulse (peak={peak})");
        // The room goes quiet: no energy, advance past the 240-frame TTL.
        for f in 40..400u64 {
            studio.fold_pulse(0, f);
        }
        assert_eq!(studio.last_pulse, 0, "the pulse forgets when the room quiets");
        let _ = bus.cmd_tx.send(super::super::command::MixerCommand::Shutdown);
    }
}
