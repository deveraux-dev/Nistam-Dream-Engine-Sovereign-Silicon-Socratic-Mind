#![allow(clippy::disallowed_types)] // @forge:allow_alloc — cold-path module, init-time allocations permitted
// MidiEvent and JACK input routing — MIDI hardware to dead-drop-engine sequencer.

use crossbeam_channel::Sender;
use std::time::{Duration, Instant};

/// MIDI event from JACK input, quantized to audio sample position.
///
/// `sample_offset` must be less than the current audio buffer size —
/// this invariant is enforced at the call site (`jack_process_callback`),
/// not within the struct itself.
#[derive(Copy, Clone, Debug)]
pub struct MidiEvent {
    /// Sample offset within current buffer (must be < buffer size).
    pub sample_offset: u32,
    /// MIDI status byte (e.g. 0x90 = NoteOn channel 0).
    pub status: u8,
    /// MIDI data bytes (e.g. [note, velocity]).
    pub data: [u8; 2],
    /// Timestamp from JACK transport (frames).
    pub jack_frame: u64,
}

impl MidiEvent {
    /// Returns true when the sample_offset respects the buffer size bound.
    /// This is the invariant enforced by `jack_process_callback`.
    #[inline]
    pub fn offset_in_bounds(&self, buffer_size: u32) -> bool {
        self.sample_offset < buffer_size
    }
}

/// Raw MIDI event as delivered by JACK (or a simulation thereof).
/// Stack-only, no heap allocation.
#[derive(Copy, Clone, Debug)]
pub struct RawMidiEvent {
    /// Sample offset within the current audio buffer.
    pub time: u32,
    /// MIDI bytes: status + up to 2 data bytes.
    pub bytes: [u8; 3],
    /// Number of valid bytes in `bytes` (1–3).
    pub len: u8,
}

/// Process a buffer of raw MIDI events and forward them to the sequencer.
///
/// Mirrors the JACK process callback (Algorithm 3 in the design doc) but
/// operates on a generic slice instead of `jack::MidiInPort`, so the crate
/// compiles without a JACK dependency.
///
/// Guarantees:
/// - `sample_offset` is clamped to `< buffer_size`.
/// - `try_send` is used so the call never blocks; events are silently
///   dropped when the channel is full.
/// - Zero heap allocations.
#[inline]
pub fn jack_process_callback(
    events: &[RawMidiEvent],
    sequencer_tx: &Sender<MidiEvent>,
    jack_frame: u64,
    buffer_size: u32,
) {
    for raw in events {
        let sample_offset = if raw.time >= buffer_size {
            buffer_size.saturating_sub(1)
        } else {
            raw.time
        };

        let midi_event = MidiEvent {
            sample_offset,
            status: raw.bytes[0],
            data: [
                if raw.len > 1 { raw.bytes[1] } else { 0 },
                if raw.len > 2 { raw.bytes[2] } else { 0 },
            ],
            jack_frame,
        };

        // Non-blocking send — drop on full (Req 10.4).
        let _ = sequencer_tx.try_send(midi_event);
    }
}

// ---------------------------------------------------------------------------
// JACK server detection and MIDI lane fallback (Req 12.3, 12.4)
// ---------------------------------------------------------------------------

/// Tracks JACK server connection state and manages periodic reconnection.
///
/// When JACK is not running the MIDI lane is disabled and the mixer falls
/// back to its internal sequencer clock. `attempt_reconnect` should be
/// called once per game tick (or audio callback); it returns `true` when
/// the reconnect interval has elapsed and a reconnection attempt should be
/// made by the caller.
#[derive(Debug)]
pub struct JackConnectionState {
    /// Whether we currently have a live JACK connection.
    pub connected: bool,
    /// Timestamp of the last reconnection attempt (if any).
    pub last_attempt: Option<Instant>,
    /// How long to wait between reconnection attempts (default 5 s).
    pub reconnect_interval: Duration,
}

impl JackConnectionState {
    /// Create a new state assuming JACK is **not** connected.
    pub fn new_disconnected() -> Self {
        Self {
            connected: false,
            last_attempt: None,
            reconnect_interval: Duration::from_secs(5),
        }
    }

    /// Create a new state assuming JACK **is** connected.
    pub fn new_connected() -> Self {
        Self {
            connected: true,
            last_attempt: None,
            reconnect_interval: Duration::from_secs(5),
        }
    }

    /// Check whether enough time has elapsed to attempt a reconnection.
    ///
    /// Returns `true` when:
    /// - We are currently disconnected, **and**
    /// - Either no attempt has been made yet, or at least
    ///   `reconnect_interval` has elapsed since the last attempt.
    ///
    /// When `true` is returned the `last_attempt` timestamp is updated
    /// to `now` so the caller can proceed with the actual JACK probe.
    pub fn attempt_reconnect(&mut self) -> bool {
        if self.connected {
            return false;
        }

        let now = Instant::now();

        let should_try = match self.last_attempt {
            None => true,
            Some(last) => now.duration_since(last) >= self.reconnect_interval,
        };

        if should_try {
            self.last_attempt = Some(now);
        }

        should_try
    }

    /// Mark the connection as established.
    pub fn mark_connected(&mut self) {
        self.connected = true;
    }

    /// Mark the connection as lost.
    pub fn mark_disconnected(&mut self) {
        self.connected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel as channel;
    use proptest::prelude::*;

    /// Strategy that generates (buffer_size, sample_offset) where sample_offset < buffer_size.
    fn valid_offset_strategy() -> impl Strategy<Value = (u32, u32)> {
        (1u32..=4096).prop_flat_map(|buf_sz| {
            (Just(buf_sz), 0..buf_sz)
        })
    }

    /// Strategy that generates (buffer_size, sample_offset) where sample_offset >= buffer_size.
    fn invalid_offset_strategy() -> impl Strategy<Value = (u32, u32)> {
        (1u32..=4096).prop_flat_map(|buf_sz| {
            (Just(buf_sz), buf_sz..=u32::MAX)
        })
    }

    // **Validates: Requirements 10.2**
    //
    // Property 10: MIDI Sample Offset Bound
    // For arbitrary MIDI events with a given buffer size, sample_offset < buffer_size.
    proptest! {
        #[test]
        fn prop_midi_sample_offset_within_bound(
            (buffer_size, sample_offset) in valid_offset_strategy(),
            status in any::<u8>(),
            data in any::<[u8; 2]>(),
            jack_frame in any::<u64>(),
        ) {
            let event = MidiEvent {
                sample_offset,
                status,
                data,
                jack_frame,
            };

            prop_assert!(
                event.offset_in_bounds(buffer_size),
                "sample_offset {} must be < buffer_size {}",
                event.sample_offset,
                buffer_size
            );
        }
    }

    // **Validates: Requirements 10.2**
    //
    // Property 10 (negative): When sample_offset >= buffer_size, the bound is violated.
    proptest! {
        #[test]
        fn prop_midi_sample_offset_violates_bound_when_too_large(
            (buffer_size, sample_offset) in invalid_offset_strategy(),
            status in any::<u8>(),
            data in any::<[u8; 2]>(),
            jack_frame in any::<u64>(),
        ) {
            let event = MidiEvent {
                sample_offset,
                status,
                data,
                jack_frame,
            };

            prop_assert!(
                !event.offset_in_bounds(buffer_size),
                "sample_offset {} should NOT be < buffer_size {}",
                event.sample_offset,
                buffer_size
            );
        }
    }

    // ── jack_process_callback unit tests ──────────────────────────────

    #[test]
    fn callback_forwards_all_events() {
        let (tx, rx) = channel::bounded(16);
        let events = [
            RawMidiEvent { time: 0, bytes: [0x90, 60, 100], len: 3 },
            RawMidiEvent { time: 128, bytes: [0x80, 60, 0], len: 3 },
        ];

        jack_process_callback(&events, &tx, 44100, 256);

        let a = rx.try_recv().unwrap();
        assert_eq!(a.status, 0x90);
        assert_eq!(a.data, [60, 100]);
        assert_eq!(a.sample_offset, 0);
        assert_eq!(a.jack_frame, 44100);

        let b = rx.try_recv().unwrap();
        assert_eq!(b.status, 0x80);
        assert_eq!(b.data, [60, 0]);
        assert_eq!(b.sample_offset, 128);
    }

    #[test]
    fn callback_clamps_offset_to_buffer_size() {
        let (tx, rx) = channel::bounded(4);
        let events = [
            RawMidiEvent { time: 512, bytes: [0x90, 60, 100], len: 3 },
        ];

        jack_process_callback(&events, &tx, 0, 256);

        let ev = rx.try_recv().unwrap();
        assert!(ev.sample_offset < 256, "offset {} must be < 256", ev.sample_offset);
        assert_eq!(ev.sample_offset, 255);
    }

    #[test]
    fn callback_drops_on_full_channel() {
        // Channel capacity 1 — second event should be silently dropped.
        let (tx, rx) = channel::bounded(1);
        let events = [
            RawMidiEvent { time: 0, bytes: [0x90, 60, 100], len: 3 },
            RawMidiEvent { time: 1, bytes: [0x90, 62, 100], len: 3 },
        ];

        jack_process_callback(&events, &tx, 0, 256);

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err(), "second event should have been dropped");
    }

    #[test]
    fn callback_handles_short_midi_messages() {
        let (tx, rx) = channel::bounded(4);
        // Program Change is 2 bytes (status + program), len=2
        let events = [
            RawMidiEvent { time: 10, bytes: [0xC0, 5, 0], len: 2 },
        ];

        jack_process_callback(&events, &tx, 0, 256);

        let ev = rx.try_recv().unwrap();
        assert_eq!(ev.status, 0xC0);
        assert_eq!(ev.data, [5, 0]);
    }

    #[test]
    fn callback_handles_single_byte_message() {
        let (tx, rx) = channel::bounded(4);
        // System Real-Time (e.g. 0xF8 Timing Clock) — 1 byte
        let events = [
            RawMidiEvent { time: 0, bytes: [0xF8, 0, 0], len: 1 },
        ];

        jack_process_callback(&events, &tx, 0, 256);

        let ev = rx.try_recv().unwrap();
        assert_eq!(ev.status, 0xF8);
        assert_eq!(ev.data, [0, 0]);
    }

    #[test]
    fn callback_empty_slice_sends_nothing() {
        let (tx, rx) = channel::bounded(4);
        jack_process_callback(&[], &tx, 0, 256);
        assert!(rx.try_recv().is_err());
    }

    // ── JackConnectionState unit tests ────────────────────────────────

    #[test]
    fn jack_state_new_disconnected() {
        let state = JackConnectionState::new_disconnected();
        assert!(!state.connected);
        assert!(state.last_attempt.is_none());
        assert_eq!(state.reconnect_interval, Duration::from_secs(5));
    }

    #[test]
    fn jack_state_new_connected() {
        let state = JackConnectionState::new_connected();
        assert!(state.connected);
    }

    #[test]
    fn jack_state_attempt_reconnect_returns_false_when_connected() {
        let mut state = JackConnectionState::new_connected();
        assert!(!state.attempt_reconnect());
    }

    #[test]
    fn jack_state_attempt_reconnect_first_call_returns_true() {
        let mut state = JackConnectionState::new_disconnected();
        assert!(state.attempt_reconnect(), "first attempt should return true");
        assert!(state.last_attempt.is_some());
    }

    #[test]
    fn jack_state_attempt_reconnect_respects_interval() {
        let mut state = JackConnectionState::new_disconnected();
        // Use a very short interval for testing
        state.reconnect_interval = Duration::from_millis(10);

        // First attempt succeeds
        assert!(state.attempt_reconnect());

        // Immediate second attempt should fail (interval not elapsed)
        assert!(!state.attempt_reconnect());

        // After sleeping past the interval, it should succeed
        std::thread::sleep(Duration::from_millis(15));
        assert!(state.attempt_reconnect());
    }

    #[test]
    fn jack_state_mark_connected_disconnected() {
        let mut state = JackConnectionState::new_disconnected();
        assert!(!state.connected);

        state.mark_connected();
        assert!(state.connected);
        // Once connected, attempt_reconnect returns false
        assert!(!state.attempt_reconnect());

        state.mark_disconnected();
        assert!(!state.connected);
        // Now attempt_reconnect should return true again
        assert!(state.attempt_reconnect());
    }
}
