//! Companion types — OODA state machine and animation commands.
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-companion\src\lib.rs` lines 17-60.

use std::sync::mpsc;

/// Commands sent from link-behavior to drive the companion.
#[derive(Debug, Clone)]
pub enum AnimCommand {
    /// Transition to a new behavior state (selects animation clip).
    SetState(BehaviorState),
    /// One-shot reactive animation (interrupts current state).
    React(Reaction),
    /// Set the active context (drives additive pose layer).
    SetContext(ActiveContext),
    /// Update overlay text (speech bubble content).
    SetPreview {
        /// Raw transcript from voice capture.
        raw: String,
        /// Interpreted/formatted command.
        interpreted: String,
    },
    /// Clear the preview bubble.
    ClearPreview,
    /// Shut down the companion window.
    Quit,
}

/// Maps 1:1 to animation clips from the design bible.
#[derive(Debug, Clone, PartialEq)]
pub enum BehaviorState {
    /// Idle state (default).
    Idle,
    /// Sleep state.
    Sleep,
    /// Listening for voice input.
    Listening,
    /// Previewing interpreted command.
    Previewing,
    /// Executing command.
    Executing,
}

/// One-shot reactive animations.
#[derive(Debug, Clone, PartialEq)]
pub enum Reaction {
    /// Build failure — double middle fingers (react_error).
    Error,
    /// Tests pass — fist pump (react_success).
    Success,
    /// Abort — flinch + air burst (abort_flinch).
    Abort,
}

/// Active application context — drives additive pose layer.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveContext {
    /// Global/default context.
    Global,
    /// Code editor context.
    Coding,
    /// Digital Audio Workstation context.
    Daw,
    /// Terminal/shell context.
    Terminal,
}

/// Spawn the companion window on a dedicated thread.
/// Returns a sender for AnimCommands.
/// If glb_path is None, generates the 3rd-Year Painter procedurally.
#[must_use]
pub fn spawn_companion(glb_path: Option<&str>) -> mpsc::Sender<AnimCommand> {
    let (tx, _rx) = mpsc::channel();
    let path = glb_path.map(|s| s.to_string());

    std::thread::Builder::new()
        .name("companion".into())
        .spawn(move || {
            // In v3, the renderer is not yet ported. This stub holds the thread alive.
            let _path = path;
            // renderer::run(path, rx);
        })
        .expect("spawn companion thread");

    tx
}
