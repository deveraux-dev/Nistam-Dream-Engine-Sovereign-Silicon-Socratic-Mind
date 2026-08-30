//! OODA behavioral state machine for the 3rd-Year Painter.
//!
//! Consumes SensorEvents from voice, mouse hook, and window focus systems.
//! Produces AnimCommands for the companion renderer.
//! Exposes state via Mutex for the REST API.
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-behavior\src\lib.rs` (234 LOC).
//! Adaptations:
//! - Replaced `arc_swap::ArcSwap` with `std::sync::Mutex<Arc<BehaviorSnapshot>>`
//! - Replaced `log::info!` with gated `debug_log!` (eprintln! behind debug feature)
//! - Hand-rolled JSON serialization for `BehaviorSnapshot::to_json()`
//! - Zero dependencies (removed serde, arc_swap, log)

use std::sync::{Arc, Mutex, mpsc};

use crate::types::{ActiveContext, AnimCommand, BehaviorState, Reaction};

/// Debug logging macro — no-op in release builds.
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}

/// Debug logging macro — no-op in release builds.
#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        let _ = ($($arg)*);
    };
}

/// Raw events fed into the behavior channel from existing systems.
#[derive(Debug, Clone)]
pub enum SensorEvent {
    /// Active window changed — exe name and window title.
    WindowFocusChanged {
        /// Executable name of the focused window.
        exe: String,
        /// Window title of the focused window.
        title: String,
    },
    /// Push-to-talk pressed (Mouse4).
    PttPressed,
    /// Push-to-talk released (Mouse4).
    PttReleased,
    /// Partial transcript from Vosk (live update during capture).
    TranscriptPartial(String),
    /// Final transcript from Vosk (capture complete).
    TranscriptFinal(String),
    /// Abort gesture (scroll-wheel during M4 hold).
    AbortCapture,
    /// External trigger: build failed.
    BuildFailed,
    /// External trigger: build/tests passed.
    BuildSuccess,
    /// Vocal-formant energy crossed its hysteresis threshold (`true` = risen
    /// above, `false` = fallen below) — a hands-free alternative to PTT,
    /// sourced from `forge_shaderbind::SignalValues.audio_formant_energy`
    /// (2026-08-24). Only a rise while Idle/Sleep currently transitions
    /// anything; the fall edge is sent for symmetry and future use.
    VocalEnergyThreshold(bool),
}

/// Snapshot of current behavior state — shared with hyper via Mutex.
///
/// This struct contains four string fields representing the state of the OODA engine.
#[derive(Debug, Clone)]
pub struct BehaviorSnapshot {
    /// Current OODA state (e.g., "Idle", "Listening").
    pub state: String,
    /// Current application context (e.g., "Coding", "Terminal").
    pub context: String,
    /// Raw transcript from voice capture.
    pub transcript: String,
    /// Interpreted/formatted command.
    pub interpreted: String,
}

impl Default for BehaviorSnapshot {
    fn default() -> Self {
        Self {
            state: "idle".into(),
            context: "global".into(),
            transcript: String::new(),
            interpreted: String::new(),
        }
    }
}

impl BehaviorSnapshot {
    /// Hand-rolled JSON serialization (four string fields, trivial escaping).
    ///
    /// Returns a JSON string representation of this snapshot.
    /// Escapes only `"`, `\`, and control characters.
    pub fn to_json(&self) -> String {
        fn escape_string(s: &str) -> String {
            s.chars()
                .flat_map(|c| match c {
                    '"' => vec!['\\', '"'],
                    '\\' => vec!['\\', '\\'],
                    '\n' => vec!['\\', 'n'],
                    '\r' => vec!['\\', 'r'],
                    '\t' => vec!['\\', 't'],
                    c if c.is_control() => format!("\\u{:04x}", c as u32).chars().collect(),
                    c => vec![c],
                })
                .collect()
        }

        format!(
            r#"{{"state":"{}","context":"{}","transcript":"{}","interpreted":"{}"}}"#,
            escape_string(&self.state),
            escape_string(&self.context),
            escape_string(&self.transcript),
            escape_string(&self.interpreted)
        )
    }
}

/// The OODA engine — runs on a dedicated thread.
pub struct OodaEngine {
    rx: mpsc::Receiver<SensorEvent>,
    companion_tx: mpsc::Sender<AnimCommand>,
    shared_state: Arc<Mutex<BehaviorSnapshot>>,

    state: BehaviorState,
    context: ActiveContext,
    transcript_buffer: String,
    interpreted: String,
}

impl OodaEngine {
    /// Create a new OODA engine.
    ///
    /// # Arguments
    /// * `rx` - Receiver for sensor events
    /// * `companion_tx` - Sender for animation commands
    /// * `shared_state` - Shared state for REST API access
    pub fn new(
        rx: mpsc::Receiver<SensorEvent>,
        companion_tx: mpsc::Sender<AnimCommand>,
        shared_state: Arc<Mutex<BehaviorSnapshot>>,
    ) -> Self {
        Self {
            rx,
            companion_tx,
            shared_state,
            state: BehaviorState::Idle,
            context: ActiveContext::Global,
            transcript_buffer: String::new(),
            interpreted: String::new(),
        }
    }

    /// Run the OODA loop. Blocks until channel closes.
    pub fn run(mut self) {
        debug_log!("[Behavior] OODA engine started");

        while let Ok(event) = self.rx.recv() {
            // OBSERVE: raw event received.
            // ORIENT: update context if needed.
            self.orient(&event);
            // DECIDE: state transition.
            self.decide(event);
            // ACT: send commands to companion + update shared state.
            self.act();
        }

        debug_log!("[Behavior] OODA engine stopped (channel closed)");
    }

    /// ORIENT — update context from window focus events.
    fn orient(&mut self, event: &SensorEvent) {
        if let SensorEvent::WindowFocusChanged { exe, .. } = event {
            let exe_lower = exe.to_lowercase();
            let new_ctx = if exe_lower.contains("code") || exe_lower.contains("rustrover")
                || exe_lower.contains("vim") || exe_lower.contains("nvim")
            {
                ActiveContext::Coding
            } else if exe_lower.contains("dead-drop") || exe_lower.contains("ableton")
                || exe_lower.contains("reaper")
            {
                ActiveContext::Daw
            } else if exe_lower.contains("cmd") || exe_lower.contains("powershell")
                || exe_lower.contains("windowsterminal") || exe_lower.contains("wezterm")
                || exe_lower.contains("alacritty")
            {
                ActiveContext::Terminal
            } else {
                ActiveContext::Global
            };

            if new_ctx != self.context {
                debug_log!(
                    "[Behavior] Context: {:?} → {:?} ({})",
                    &self.context,
                    &new_ctx,
                    exe
                );
                self.context = new_ctx.clone();
                let _ = self.companion_tx.send(AnimCommand::SetContext(new_ctx));
            }
        }
    }

    /// DECIDE — state machine transitions.
    fn decide(&mut self, event: SensorEvent) {
        match (&self.state, event) {
            // PTT pressed → start listening.
            (BehaviorState::Idle | BehaviorState::Sleep, SensorEvent::PttPressed) => {
                self.transcript_buffer.clear();
                self.interpreted.clear();
                self.state = BehaviorState::Listening;
                let _ = self
                    .companion_tx
                    .send(AnimCommand::SetState(BehaviorState::Listening));
            }

            // Partial transcript → update buffer.
            (BehaviorState::Listening, SensorEvent::TranscriptPartial(text)) => {
                self.transcript_buffer = text.clone();
                let _ = self.companion_tx.send(AnimCommand::SetPreview {
                    raw: text,
                    interpreted: String::new(),
                });
            }

            // Final transcript → preview the interpreted command.
            (BehaviorState::Listening, SensorEvent::TranscriptFinal(text)) => {
                self.transcript_buffer = text.clone();
                self.interpreted = self.interpret(&text);
                self.state = BehaviorState::Previewing;
                let _ = self
                    .companion_tx
                    .send(AnimCommand::SetState(BehaviorState::Previewing));
                let _ = self.companion_tx.send(AnimCommand::SetPreview {
                    raw: text,
                    interpreted: self.interpreted.clone(),
                });
            }

            // Abort during listening or previewing.
            (BehaviorState::Listening | BehaviorState::Previewing, SensorEvent::AbortCapture) => {
                self.transcript_buffer.clear();
                self.interpreted.clear();
                self.state = BehaviorState::Idle;
                let _ = self
                    .companion_tx
                    .send(AnimCommand::React(Reaction::Abort));
                let _ = self.companion_tx.send(AnimCommand::ClearPreview);
            }

            // PTT released during preview → execute (commit).
            (BehaviorState::Previewing, SensorEvent::PttReleased) => {
                self.state = BehaviorState::Executing;
                let _ = self
                    .companion_tx
                    .send(AnimCommand::SetState(BehaviorState::Executing));
                let _ = self.companion_tx.send(AnimCommand::ClearPreview);
                // Actual text injection happens in link-voice, not here.
                // We just signal the companion.
                self.state = BehaviorState::Idle;
            }

            // PTT released during listening (no final transcript yet) → back to idle.
            (BehaviorState::Listening, SensorEvent::PttReleased) => {
                // Vosk hasn't produced a final yet — wait for it or go idle.
                // In practice, PttReleased triggers Vosk to finalize.
            }

            // Sustained vocal-formant energy → start listening, hands-free
            // (same entry semantics as PttPressed; the fall edge is a no-op
            // today, mirroring PttReleased-during-Listening below).
            (BehaviorState::Idle | BehaviorState::Sleep, SensorEvent::VocalEnergyThreshold(true)) => {
                self.transcript_buffer.clear();
                self.interpreted.clear();
                self.state = BehaviorState::Listening;
                let _ = self
                    .companion_tx
                    .send(AnimCommand::SetState(BehaviorState::Listening));
            }

            // Build events → reactive animations.
            (_, SensorEvent::BuildFailed) => {
                let _ = self
                    .companion_tx
                    .send(AnimCommand::React(Reaction::Error));
            }
            (_, SensorEvent::BuildSuccess) => {
                let _ = self
                    .companion_tx
                    .send(AnimCommand::React(Reaction::Success));
            }

            _ => {}
        }
    }

    /// ACT — update shared state for REST API.
    fn act(&self) {
        let snapshot = BehaviorSnapshot {
            state: format!("{:?}", self.state),
            context: format!("{:?}", self.context),
            transcript: self.transcript_buffer.clone(),
            interpreted: self.interpreted.clone(),
        };
        if let Ok(mut guard) = self.shared_state.lock() {
            *guard = snapshot;
        }
    }

    /// Interpret a transcript based on current context.
    /// This is the semantic preview — raw dictation becomes a formatted command.
    fn interpret(&self, text: &str) -> String {
        match &self.context {
            ActiveContext::Terminal => {
                // In terminal context, pass through as command.
                format!("> {}", text)
            }
            ActiveContext::Coding => {
                // In coding context, could map voice to code patterns.
                // For now, pass through with indicator.
                format!("[code] {}", text)
            }
            ActiveContext::Daw => {
                // DAW commands: transport, mix, etc.
                let lower = text.to_lowercase();
                if lower.contains("play") {
                    return "[DAW] transport:play".into();
                }
                if lower.contains("stop") {
                    return "[DAW] transport:stop".into();
                }
                if lower.contains("record") {
                    return "[DAW] transport:record".into();
                }
                if lower.contains("crossfade") {
                    return "[DAW] crossfade".into();
                }
                format!("[DAW] {}", text)
            }
            ActiveContext::Global => text.to_string(),
        }
    }
}

/// Spawn the behavior engine on a dedicated thread.
///
/// # Returns
/// A tuple of (SensorEvent sender, shared state reader).
pub fn spawn_behavior(
    companion_tx: mpsc::Sender<AnimCommand>,
) -> (mpsc::SyncSender<SensorEvent>, Arc<Mutex<BehaviorSnapshot>>) {
    // Bounded channel — 128 events max, backpressure if overwhelmed.
    let (tx, rx) = mpsc::sync_channel(128);
    let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));
    let shared_clone = shared_state.clone();

    std::thread::Builder::new()
        .name("behavior-ooda".into())
        .spawn(move || {
            let engine = OodaEngine::new(rx, companion_tx, shared_clone);
            engine.run();
        })
        .expect("spawn behavior thread");

    (tx, shared_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Idle → Listening transition on PttPressed.
    #[test]
    fn test_idle_to_listening_on_ptt_pressed() {
        let (companion_tx, companion_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::sync_channel(128);
        let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));
        let shared_clone = shared_state.clone();

        let engine = OodaEngine::new(event_rx, companion_tx, shared_clone);
        let state_before = engine.state.clone();
        assert_eq!(state_before, BehaviorState::Idle);

        // Manually drive the state machine
        let mut engine = engine;
        engine.decide(SensorEvent::PttPressed);
        assert_eq!(engine.state, BehaviorState::Listening);

        // Verify AnimCommand was sent
        let cmd = companion_rx.try_recv();
        assert!(matches!(
            cmd,
            Ok(AnimCommand::SetState(BehaviorState::Listening))
        ));
    }

    /// Sustained vocal-formant energy triggers Listening exactly like PttPressed
    /// (hands-free trigger, 2026-08-24); the fall edge from Idle is a no-op.
    #[test]
    fn test_idle_to_listening_on_vocal_energy_rise() {
        let (companion_tx, companion_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::sync_channel(128);
        let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));

        let mut engine = OodaEngine::new(event_rx, companion_tx, shared_state);
        assert_eq!(engine.state, BehaviorState::Idle);

        engine.decide(SensorEvent::VocalEnergyThreshold(true));
        assert_eq!(engine.state, BehaviorState::Listening);
        assert!(matches!(
            companion_rx.try_recv(),
            Ok(AnimCommand::SetState(BehaviorState::Listening))
        ));

        // The fall edge from Idle/Sleep is unmatched today — no transition, no panic.
        let mut idle_engine = OodaEngine::new(
            mpsc::sync_channel(128).1,
            mpsc::channel().0,
            Arc::new(Mutex::new(BehaviorSnapshot::default())),
        );
        idle_engine.decide(SensorEvent::VocalEnergyThreshold(false));
        assert_eq!(idle_engine.state, BehaviorState::Idle);
    }

    /// Test Listening → Previewing on TranscriptFinal.
    #[test]
    fn test_listening_to_previewing_on_transcript_final() {
        let (companion_tx, companion_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::sync_channel(128);
        let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));
        let shared_clone = shared_state.clone();

        let mut engine = OodaEngine::new(event_rx, companion_tx, shared_clone);

        // Move to Listening state first
        engine.state = BehaviorState::Listening;

        // Simulate final transcript
        engine.decide(SensorEvent::TranscriptFinal("hello world".into()));

        assert_eq!(engine.state, BehaviorState::Previewing);
        assert_eq!(engine.transcript_buffer, "hello world");
        assert_eq!(engine.interpreted, "hello world"); // Global context

        // Check that SetState(Previewing) and SetPreview commands were sent
        let cmd1 = companion_rx.try_recv();
        assert!(matches!(
            cmd1,
            Ok(AnimCommand::SetState(BehaviorState::Previewing))
        ));
    }

    /// Test Previewing → Idle on AbortCapture.
    #[test]
    fn test_previewing_to_idle_on_abort_capture() {
        let (companion_tx, companion_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::sync_channel(128);
        let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));
        let shared_clone = shared_state.clone();

        let mut engine = OodaEngine::new(event_rx, companion_tx, shared_clone);

        // Move to Previewing state
        engine.state = BehaviorState::Previewing;
        engine.transcript_buffer = "test".into();
        engine.interpreted = "test output".into();

        engine.decide(SensorEvent::AbortCapture);

        assert_eq!(engine.state, BehaviorState::Idle);
        assert!(engine.transcript_buffer.is_empty());
        assert!(engine.interpreted.is_empty());

        // Check that Reaction::Abort was sent
        let cmd1 = companion_rx.try_recv();
        assert!(matches!(cmd1, Ok(AnimCommand::React(Reaction::Abort))));
    }

    /// Test BuildFailed → React(Error) command emission.
    #[test]
    fn test_build_failed_emits_error_reaction() {
        let (companion_tx, companion_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::sync_channel(128);
        let shared_state = Arc::new(Mutex::new(BehaviorSnapshot::default()));
        let shared_clone = shared_state.clone();

        let mut engine = OodaEngine::new(event_rx, companion_tx, shared_clone);

        // Can emit from any state
        engine.state = BehaviorState::Idle;
        engine.decide(SensorEvent::BuildFailed);

        // Check that Reaction::Error was sent
        let cmd = companion_rx.try_recv();
        assert!(matches!(cmd, Ok(AnimCommand::React(Reaction::Error))));
    }

    /// Test BehaviorSnapshot JSON serialization with string escaping.
    #[test]
    fn test_behavior_snapshot_to_json() {
        let snapshot = BehaviorSnapshot {
            state: r#"Idle"With"Quote"#.into(),
            context: "Coding\\Backslash".into(),
            transcript: "Line\nBreak\tTab".into(),
            interpreted: "Normal".into(),
        };

        let json = snapshot.to_json();

        // Verify it contains the escaped sequences
        assert!(json.contains(r#"\"With\""#));
        assert!(json.contains(r#"\\"#)); // backslash
        assert!(json.contains(r#"\n"#)); // newline
        assert!(json.contains(r#"\t"#)); // tab

        // Verify it's valid JSON-like structure
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }
}
