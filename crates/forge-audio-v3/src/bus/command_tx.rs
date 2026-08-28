#![allow(clippy::disallowed_types)] // @forge:allow_alloc — game-side cold path, not the realtime callback
//! Game-side command sender over the mixer bus — split out of the ironroot
//! cartridge (audio_bridge.rs T4b 2026-07-27, Sean: "Split Tx out to forge-audio").
//! Engine-pure: wraps a crossbeam Sender<MixerCommand>; game param computation
//! (session/brand/weather) stays cartridge-side.

use super::command::MixerCommand;

/// Error returned when the audio worker thread has disconnected.
#[derive(Debug)]
pub struct AudioSendError;

impl std::fmt::Display for AudioSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audio channel disconnected")
    }
}

impl std::error::Error for AudioSendError {}

/// Game-side audio command sender. Clone-able, Send-able.
/// Wraps `crossbeam_channel::Sender<MixerCommand>`.
#[derive(Clone)]
pub struct AudioCommandTx {
    inner: crossbeam_channel::Sender<MixerCommand>,
}

impl AudioCommandTx {
    /// Create a new `AudioCommandTx` from a crossbeam sender.
    pub fn new(inner: crossbeam_channel::Sender<MixerCommand>) -> Self {
        Self { inner }
    }

    /// Send a command to the audio worker. Never blocks.
    /// Returns `Err(AudioSendError)` only if the worker thread has
    /// panicked (channel disconnected).
    pub fn send(&self, cmd: MixerCommand) -> Result<(), AudioSendError> {
        self.inner.send(cmd).map_err(|_| {
            eprintln!("[AUDIO] warning: audio channel disconnected, command dropped");
            AudioSendError
        })
    }

    /// Convenience: send a parameter change.
    pub fn set_param(&self, target: &str, value: f32) {
        if self
            .send(MixerCommand::Param {
                target: target.to_owned(), // @forge:allow_alloc cold path
                value,
            })
            .is_err()
        {
            eprintln!("[AUDIO] warning: failed to send set_param({target})");
        }
    }

    /// Convenience: fire a one-shot action (SFX trigger).
    pub fn fire_action(&self, target: &str) {
        if self
            .send(MixerCommand::Action {
                target: target.to_owned(), // @forge:allow_alloc cold path
            })
            .is_err()
        {
            eprintln!("[AUDIO] warning: failed to send fire_action({target})");
        }
    }

    /// Convenience: switch audio preset/profile.
    pub fn set_preset(&self, name: &str, intensity: f32) {
        if self
            .send(MixerCommand::SetPreset {
                name: name.to_owned(), // @forge:allow_alloc cold path
                intensity,
            })
            .is_err()
        {
            eprintln!("[AUDIO] warning: failed to send set_preset({name})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_delivers_and_disconnect_errors() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let cmd_tx = AudioCommandTx::new(tx);
        cmd_tx.set_param("master_gain", 0.5);
        match rx.recv().unwrap() {
            MixerCommand::Param { target, value } => {
                assert_eq!(target, "master_gain");
                assert_eq!(value, 0.5);
            }
            other => panic!("unexpected command: {other:?}"),
        }
        drop(rx);
        assert!(cmd_tx.send(MixerCommand::Action { target: "x".into() }).is_err());
    }
}
