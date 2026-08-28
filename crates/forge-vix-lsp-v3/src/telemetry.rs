//! Editor-telemetry → CognitiveSignal intake (FORGE-COGNITIVE-IDE Phase-2, 2026-06-11).
//!
//! The LSP is the one process that sees keystrokes / file-switches / AST results,
//! so it is where raw editor events become `forge_sieve::cognitive::CognitiveSignal`s
//! and feed a live `AdhdLens`. The studio then reacts — IDE `adapt()` (truncate
//! output, cap lists) and the heal synth (`forge_audio::cognitive_heal::heal_for`).
//!
//! WIRE-not-invent: the lens, signals, and states all pre-exist in
//! `forge_sieve::cognitive`. The only genuinely new code here is the editor-event
//! DETECTION. Integer-only, zero alloc on `push`, runs off the editor thread —
//! never the audio callback.

use crate::cognitive::{
    guided_adapt, AdhdLens, Adaptation, CognitiveCeiling, CognitiveLens, CognitiveSignal,
    CognitiveState, Guidance,
};

/// Sustained keystrokes within one `Minute` that count as "in the flow".
const FLOW_KEYSTROKES_PER_MIN: u32 = 30;
/// Unbroken in-flow minutes before the lens is told the user has hit flow.
const FLOW_MINUTES_FOR_FLOW: u32 = 10;
/// An error arriving within this window of the previous run = a rapid edit-rerun
/// thrash (the frustration tell from the spec: run→error→1-char-edit→rerun).
const RAPID_RERUN_MS: u32 = 3_000;

/// A raw editor event streamed from the client (one `forge/telemetry` notify).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorEvent {
    /// A normal character keystroke.
    Keystroke,
    /// Focus moved to a different file / pane.
    FileSwitch,
    /// A script run / parse errored, `since_last_run_ms` after the previous run.
    AstError { since_last_run_ms: u32 },
    /// A script evaluated clean start→finish (no panic) — a completion.
    AstOk,
    /// All diagnostics in the active file cleared to zero — a completion.
    DiagnosticsCleared,
    /// One minute of wall time elapsed (client heartbeat).
    Minute,
}

/// Holds the live `AdhdLens` and the editor-side detection accumulators.
pub struct TelemetryTracker {
    lens: AdhdLens,
    rapid_error_runs: u32,
    flow_minutes: u32,
    keystrokes_this_minute: u32,
    errored_this_minute: bool,
    switched_this_minute: bool,
    task_switches: u32,
    session_minutes: u32,
    completions: u32,
    /// The user's guidance slider. Default Off = the lens NEVER forces an
    /// adaptation (completion caps, pacing) — it only observes. Turned on, the
    /// slider scales how hard `adapt()` bites (forge_sieve::cognitive doctrine).
    guidance: Guidance,
    /// The operator's rung dial (`forge/ceiling`). Set, never inferred — the lens
    /// observes state; it has NO authority over the rung.
    ceiling: CognitiveCeiling,
}

impl Default for TelemetryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryTracker {
    pub fn new() -> Self {
        Self {
            lens: AdhdLens::with_defaults(),
            rapid_error_runs: 0,
            flow_minutes: 0,
            keystrokes_this_minute: 0,
            errored_this_minute: false,
            switched_this_minute: false,
            task_switches: 0,
            session_minutes: 0,
            completions: 0,
            guidance: Guidance::default(), // Off — never force until the user opts in
            ceiling: CognitiveCeiling::default(), // Child — entry rung until the dial moves
        }
    }

    /// Set the guidance slider (the studio's off/intensity control, C2). Off
    /// disarms every adaptation; `On(pmy)` arms them scaled by intensity.
    pub fn set_guidance(&mut self, guidance: Guidance) {
        self.guidance = guidance;
    }

    /// The current guidance slider — pass to `completion_cap` so a cap is only
    /// ever applied when the user has armed it.
    pub fn guidance(&self) -> Guidance {
        self.guidance
    }

    /// Move the rung dial (`forge/ceiling` notification — the one rung source).
    pub fn set_ceiling(&mut self, ceiling: CognitiveCeiling) {
        self.ceiling = ceiling;
    }

    pub fn ceiling(&self) -> CognitiveCeiling {
        self.ceiling
    }

    /// Feed one raw editor event; returns the freshly-evaluated cognitive state.
    pub fn push(&mut self, event: EditorEvent) -> CognitiveState {
        let mut sig = CognitiveSignal::default();
        match event {
            EditorEvent::Keystroke => {
                self.keystrokes_this_minute = self.keystrokes_this_minute.saturating_add(1);
                // Keystrokes accumulate — only the Minute tick pushes to the lens.
                return self.lens.state();
            }
            EditorEvent::FileSwitch => {
                self.task_switches = self.task_switches.saturating_add(1);
                self.switched_this_minute = true;
                self.flow_minutes = 0;
                // question_count repurposed as context-switch counter; >= 16 → Fatigued in lens.
                sig.question_count = self.task_switches.min(255) as u8;
                sig.ticks_since_last = 1;
            }
            EditorEvent::AstError { since_last_run_ms } => {
                self.errored_this_minute = true;
                self.flow_minutes = 0;
                if since_last_run_ms <= RAPID_RERUN_MS {
                    self.rapid_error_runs = self.rapid_error_runs.saturating_add(1);
                    // Max error + zero gap → Frustrated in lens.
                    sig.error_rate_pmy = 10000;
                    sig.ticks_since_last = 0;
                } else {
                    sig.error_rate_pmy = 5000;
                    sig.ticks_since_last = 1;
                }
            }
            EditorEvent::AstOk | EditorEvent::DiagnosticsCleared => {
                self.completions = self.completions.saturating_add(1);
                self.rapid_error_runs = 0;
                sig.accept_rate_pmy = 9000;
                sig.error_rate_pmy = 0;
                sig.ticks_since_last = 1;
            }
            EditorEvent::Minute => {
                self.session_minutes = self.session_minutes.saturating_add(1);
                let in_flow = self.keystrokes_this_minute >= FLOW_KEYSTROKES_PER_MIN
                    && !self.errored_this_minute
                    && !self.switched_this_minute;
                if in_flow {
                    self.flow_minutes = self.flow_minutes.saturating_add(1);
                    if self.flow_minutes >= FLOW_MINUTES_FOR_FLOW {
                        // Deep flow: code + high accept + zero errors → Hyperfocused in lens.
                        sig.code_blocks = 1;
                        sig.accept_rate_pmy = 10000;
                        sig.error_rate_pmy = 0;
                    } else {
                        sig.code_blocks = 1;
                        sig.accept_rate_pmy = 10000;
                        sig.error_rate_pmy = 0;
                    }
                }
                sig.ticks_since_last = 1;
                self.keystrokes_this_minute = 0;
                self.errored_this_minute = false;
                self.switched_this_minute = false;
            }
        }
        self.lens.ingest(&sig);
        self.lens.state()
    }

    /// Current cognitive state without feeding an event.
    pub fn state(&self) -> CognitiveState {
        self.lens.state()
    }

    /// Interface adaptations for the current state (truncate output, cap lists,
    /// suggest a break) — for the IDE-side consumer.
    pub fn adaptations(&self) -> Vec<Adaptation> {
        vec![self.lens.adapt()]
    }

    /// Publish the current lens reading to the on-disk cognitive bus the studio
    /// neuro-hud reads (C5). Best-effort — a write failure never blocks the editor.
    /// The bus carries the guidance slider so the reader can never force past it, and
    /// resolves the path through `forge_sieve::cognitive` so producer and consumer
    /// share one authority (never a hand-copied path string).
    pub fn publish_bus(&self) -> std::io::Result<()> {
        let bus = crate::cognitive::CognitiveBus {
            state: self.lens.state(),
            adaptation: self.lens.adapt(),
            ceiling: self.ceiling,
            guidance: self.guidance,
        };
        crate::cognitive::write_cognitive_bus(&crate::cognitive::bus_root(), &bus)
    }
}

/// The hard cap on completion-list length under the current adaptations
/// (`None` = no cap). `SlowPace`/`Clarify` surfaces a 3-item cap (Fatigued/Frustrated
/// → kill option-scan friction). This is the IDE-side `adapt()` consumer.
///
/// NEVER-FORCE GATE (C1): every adaptation is routed through
/// `forge_sieve::cognitive::guided_adapt` first, so with the guidance slider Off
/// (the default) the cap is always `None` — the lens observes but never trims the
/// user's completion list behind their back. A raw `adapt()`-to-cap map (the
/// pre-C1 shape) was a forcing bug by that module's own doctrine.
pub fn completion_cap(adaptations: &[Adaptation], guidance: Guidance) -> Option<usize> {
    adaptations.iter().find_map(|a| match guided_adapt(*a, guidance) {
        Some(Adaptation::SlowPace) | Some(Adaptation::Clarify) => Some(3),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_switches_drain_to_fatigue() {
        let mut t = TelemetryTracker::new();
        let mut state = t.state();
        for _ in 0..17 {
            state = t.push(EditorEvent::FileSwitch);
        }
        assert_eq!(
            state,
            CognitiveState::Fatigued,
            "17 task switches overflow the context-switch counter → Fatigued, got {state:?}"
        );
    }

    #[test]
    fn rapid_error_reruns_breed_frustration() {
        let mut t = TelemetryTracker::new();
        // run → error → quick re-run, three times inside the rapid window.
        t.push(EditorEvent::AstError { since_last_run_ms: 400 });
        t.push(EditorEvent::AstError { since_last_run_ms: 600 });
        let state = t.push(EditorEvent::AstError { since_last_run_ms: 500 });
        assert_eq!(
            state,
            CognitiveState::Frustrated,
            "rapid error-rerun thrash → Frustrated, got {state:?}"
        );
    }

    #[test]
    fn slow_error_reruns_do_not_frustrate() {
        let mut t = TelemetryTracker::new();
        // errors spaced out (thoughtful editing) must NOT read as frustration.
        for _ in 0..4 {
            t.push(EditorEvent::AstError { since_last_run_ms: 30_000 });
        }
        assert_ne!(
            t.state(),
            CognitiveState::Frustrated,
            "slow, spaced reruns are not thrash"
        );
    }

    #[test]
    fn sustained_typing_enters_flow() {
        let mut t = TelemetryTracker::new();
        let mut state = t.state();
        for _ in 0..FLOW_MINUTES_FOR_FLOW {
            for _ in 0..FLOW_KEYSTROKES_PER_MIN {
                t.push(EditorEvent::Keystroke);
            }
            state = t.push(EditorEvent::Minute);
        }
        assert_eq!(
            state,
            CognitiveState::Hyperfocused,
            "10 sustained-typing minutes with no error/switch → flow"
        );
    }

    #[test]
    fn a_clean_run_clears_frustration() {
        let mut t = TelemetryTracker::new();
        t.push(EditorEvent::AstError { since_last_run_ms: 300 });
        t.push(EditorEvent::AstError { since_last_run_ms: 300 });
        t.push(EditorEvent::AstError { since_last_run_ms: 300 });
        assert_eq!(t.state(), CognitiveState::Frustrated);
        let state = t.push(EditorEvent::AstOk);
        assert_ne!(
            state,
            CognitiveState::Frustrated,
            "a clean eval resets frustration"
        );
    }

    #[test]
    fn fatigue_caps_the_completion_list() {
        let mut t = TelemetryTracker::new();
        t.set_guidance(Guidance::On(10000)); // user armed the slider
        for _ in 0..17 {
            t.push(EditorEvent::FileSwitch);
        }
        assert_eq!(t.state(), CognitiveState::Fatigued);
        assert_eq!(
            completion_cap(&t.adaptations(), t.guidance()),
            Some(3),
            "fatigue caps autocomplete to 3 items when guidance is armed"
        );
    }

    #[test]
    fn nominal_leaves_completions_uncapped() {
        let t = TelemetryTracker::new();
        assert_eq!(completion_cap(&t.adaptations(), Guidance::On(10000)), None);
    }

    #[test]
    fn ceiling_dial_round_trips_and_defaults_child() {
        let mut t = TelemetryTracker::new();
        assert_eq!(t.ceiling(), CognitiveCeiling::Child);
        t.set_ceiling(CognitiveCeiling::Maker);
        assert_eq!(t.ceiling(), CognitiveCeiling::Maker);
    }

    // [BOARD: C1]
    #[test]
    fn completion_cap_never_forces_when_guidance_off() {
        // Drive the lens hard into Fatigued (SlowPace) — the state that WOULD cap.
        let mut t = TelemetryTracker::new();
        for _ in 0..17 {
            t.push(EditorEvent::FileSwitch);
        }
        assert_eq!(t.state(), CognitiveState::Fatigued);
        assert_eq!(t.adaptations(), vec![Adaptation::SlowPace]);

        // Guidance Off (the default): the never-force gate must return NO cap even
        // though the raw adaptation is SlowPace. This is the C1 contract.
        assert_eq!(t.guidance(), Guidance::Off);
        assert_eq!(
            completion_cap(&t.adaptations(), Guidance::Off),
            None,
            "guidance Off must never trim the user's completion list"
        );
        assert_eq!(
            completion_cap(&t.adaptations(), Guidance::On(0)),
            None,
            "zero intensity is Off — still no forcing"
        );
        // Only once the user arms the slider does the cap apply.
        assert_eq!(
            completion_cap(&t.adaptations(), Guidance::On(5000)),
            Some(3),
            "armed guidance lets the fatigue cap through"
        );
    }
}
