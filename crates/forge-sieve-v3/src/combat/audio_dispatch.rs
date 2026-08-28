//! Ported verbatim from E:\.airgap\2026-05-17-dsp-hrtf-p00-loop\ironroot-edict\game\src\combat\audio_dispatch.rs (2026-08-17 fake-enum-audit lineage port).
//!
//! Audio Integration — non-blocking integer-typed event dispatch to AudioBus.
//!
//! Wraps a bounded crossbeam SPSC channel. Commands are silently dropped if the
//! channel is full. Combat evaluation NEVER blocks waiting for audio.
//!
//! No f32/f64 permitted. All payloads are integer-typed.

use super::AudioCommand;
use crossbeam::channel::{Sender, TrySendError};

/// Non-blocking audio command sender.
/// Wraps a bounded crossbeam channel. Silently drops commands if full.
pub struct AudioCommandSender {
    tx: Sender<AudioCommand>,
}

impl AudioCommandSender {
    /// Create a new sender wrapping the given channel transmitter.
    pub fn new(tx: Sender<AudioCommand>) -> Self {
        Self { tx }
    }

    /// Create a no-op sender that silently drops all commands.
    /// The receiver is immediately dropped, so try_send always returns false.
    /// Allocation happens once at init time — zero per-tick cost.
    pub fn noop() -> Self {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        // _rx is dropped here, so all try_send calls will return Disconnected (silent drop)
        Self { tx }
    }

    /// Try to send an audio command. Returns true if sent, false if dropped.
    /// NEVER blocks. Combat evaluation continues regardless.
    pub fn try_send(&self, cmd: AudioCommand) -> bool {
        match self.tx.try_send(cmd) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => false,         // silently drop
            Err(TrySendError::Disconnected(_)) => false,  // silently drop
        }
    }

    /// Dispatch a HitStop command from strike evaluation.
    /// `duration_ticks` comes from `compute_hit_stop(resonance_hz)`.
    pub fn dispatch_hit_stop(&self, duration_ticks: u16) -> bool {
        self.try_send(AudioCommand::HitStop { duration_ticks })
    }

    /// Dispatch a Silence command from perfect parry (always 12 ticks).
    pub fn dispatch_silence(&self) -> bool {
        self.try_send(AudioCommand::Silence { duration_ticks: 12 })
    }

    /// Dispatch a StrikeImpact command with the attacker's resonance_hz.
    pub fn dispatch_strike_impact(&self, resonance_hz: u16) -> bool {
        self.try_send(AudioCommand::StrikeImpact { resonance_hz })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::bounded;

    /// Unit test: audio channel full implies combat continues without blocking.
    ///
    /// Creates a channel with capacity 1, fills it, then verifies that
    /// subsequent try_send calls return false (dropped) without blocking.
    #[test]
    fn audio_channel_full_does_not_block() {
        // Channel capacity of 1
        let (tx, rx) = bounded(1);
        let sender = AudioCommandSender::new(tx);

        // First send succeeds
        let sent1 = sender.try_send(AudioCommand::HitStop { duration_ticks: 4 });
        assert!(sent1, "First send should succeed");

        // Second send should be silently dropped (channel full)
        let sent2 = sender.try_send(AudioCommand::Silence { duration_ticks: 12 });
        assert!(!sent2, "Second send should be dropped (channel full)");

        // Third send also dropped
        let sent3 = sender.dispatch_strike_impact(440);
        assert!(!sent3, "Third send should be dropped (channel full)");

        // Verify the first command is still in the channel
        let received = rx.try_recv().unwrap();
        assert_eq!(received, AudioCommand::HitStop { duration_ticks: 4 });

        // Now channel is empty, next send succeeds
        let sent4 = sender.dispatch_silence();
        assert!(sent4, "Send after drain should succeed");
    }

    /// Unit test: disconnected channel does not panic, returns false.
    #[test]
    fn disconnected_channel_does_not_panic() {
        let (tx, rx) = bounded(4);
        let sender = AudioCommandSender::new(tx);

        // Drop receiver to disconnect
        drop(rx);

        // Should return false, not panic
        let sent = sender.try_send(AudioCommand::HitStop { duration_ticks: 2 });
        assert!(!sent, "Send on disconnected channel should return false");
    }

    /// Unit test: dispatch_hit_stop sends correct command.
    #[test]
    fn dispatch_hit_stop_sends_correct_command() {
        let (tx, rx) = bounded(4);
        let sender = AudioCommandSender::new(tx);

        sender.dispatch_hit_stop(6);
        let cmd = rx.try_recv().unwrap();
        assert_eq!(cmd, AudioCommand::HitStop { duration_ticks: 6 });
    }

    /// Unit test: dispatch_silence sends Silence{12}.
    #[test]
    fn dispatch_silence_sends_silence_12() {
        let (tx, rx) = bounded(4);
        let sender = AudioCommandSender::new(tx);

        sender.dispatch_silence();
        let cmd = rx.try_recv().unwrap();
        assert_eq!(cmd, AudioCommand::Silence { duration_ticks: 12 });
    }

    /// Unit test: dispatch_strike_impact sends correct resonance_hz.
    #[test]
    fn dispatch_strike_impact_sends_resonance() {
        let (tx, rx) = bounded(4);
        let sender = AudioCommandSender::new(tx);

        sender.dispatch_strike_impact(440);
        let cmd = rx.try_recv().unwrap();
        assert_eq!(cmd, AudioCommand::StrikeImpact { resonance_hz: 440 });
    }
}
