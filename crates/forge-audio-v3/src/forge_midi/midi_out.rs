//! midi_out — cross-platform MIDI output sink (midir-backed).
//!
//! The "hear it NOW" path: opens the first available MIDI output port via
//! `midir` (WinMM/WinRT on Windows, CoreMIDI on macOS, ALSA on Linux) and
//! sends Channel Voice messages as stack byte slices — no allocation per send.
//!
//! ## Feature gate
//! Gated behind `winmm-out` in `lib.rs` (name kept for cfg compatibility —
//! the backend here is midir, not WinMM). No `unsafe` in this module; midir
//! owns its own platform FFI internally.
//!
//! ## Invariants (Sound Gate)
//! - No heap alloc in the send path (message = one stack byte array/slice).
//! - No blocking after open — sends `try_lock` the connection, never `lock`
//!   (Lock-Free Gate); lock contention surfaces as `MidiOutError`, not a stall.
//! - Connection close-on-drop is midir's own `Drop` impl.
//!
//! ## Wire
//! `forge-calligraphy::syllabic_to_event` produces `(channel, note, velocity)`;
//! this module's `MidiOut::note_on` / `note_off` transmits it.

use std::fmt;
use std::sync::Mutex;

/// Error from the MIDI output subsystem. The `u32` is a forge-midi sentinel
/// code (not a raw OS/driver code — midir backends don't expose one uniformly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiOutError(pub u32);

/// No MIDI output port is available on this machine.
const ERR_NO_DEVICE: u32 = 0xFFFF_FFFF;
/// The midir backend failed to initialize.
const ERR_INIT: u32 = 0xFFFF_FFFE;
/// Failed to open a connection to the selected port.
const ERR_CONNECT: u32 = 0xFFFF_FFFD;
/// Output connection is busy (concurrent send in flight); never blocks.
const ERR_BUSY: u32 = 0xFFFF_FFFC;
/// The underlying midir send call failed.
const ERR_SEND: u32 = 0xFFFF_FFFB;

impl fmt::Display for MidiOutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MIDI output error code {}", self.0)
    }
}

impl std::error::Error for MidiOutError {}

/// Live MIDI output handle. Opens the first available MIDI output port on
/// construction; the connection closes on drop. Send notes with the
/// zero-alloc `note_on` / `note_off` / `control_change` / `program_change`.
///
/// ```no_run
/// # #[cfg(feature = "winmm-out")]
/// # fn main() -> Result<(), forge_audio::forge_midi::midi_out::MidiOutError> {
/// use forge_audio::forge_midi::midi_out::MidiOut;
/// let midi = MidiOut::open()?;
/// midi.note_on(0, 60, 100)?;   // Middle C, channel 0, velocity 100
/// std::thread::sleep(std::time::Duration::from_millis(500));
/// midi.note_off(0, 60)?;
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "winmm-out"))]
/// # fn main() {}
/// ```
pub struct MidiOut {
    conn: Mutex<midir::MidiOutputConnection>,
}

impl MidiOut {
    /// Open the first available MIDI output port.
    pub fn open() -> Result<Self, MidiOutError> {
        let client = midir::MidiOutput::new("forge-midi-out").map_err(|_| MidiOutError(ERR_INIT))?;
        let port = client.ports().into_iter().next().ok_or(MidiOutError(ERR_NO_DEVICE))?;
        let conn = client
            .connect(&port, "forge-midi-out")
            .map_err(|_| MidiOutError(ERR_CONNECT))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Send a raw Channel Voice message. `try_lock` only — never blocks (Sound Gate).
    fn send(&self, bytes: &[u8]) -> Result<(), MidiOutError> {
        let mut conn = self.conn.try_lock().map_err(|_| MidiOutError(ERR_BUSY))?;
        conn.send(bytes).map_err(|_| MidiOutError(ERR_SEND))
    }

    /// Send Note On: `0x90 | channel`.
    #[inline]
    pub fn note_on(&self, channel: u8, note: u8, velocity: u8) -> Result<(), MidiOutError> {
        self.send(&channel_voice(0x90, channel, note, velocity))
    }

    /// Send Note Off: `0x80 | channel`.
    #[inline]
    pub fn note_off(&self, channel: u8, note: u8) -> Result<(), MidiOutError> {
        self.send(&channel_voice(0x80, channel, note, 0))
    }

    /// Send Control Change: `0xB0 | channel`, cc number, value.
    #[inline]
    pub fn control_change(&self, channel: u8, cc: u8, value: u8) -> Result<(), MidiOutError> {
        self.send(&channel_voice(0xB0, channel, cc, value))
    }

    /// Send Program Change: `0xC0 | channel`, program number.
    /// (Program Change is a genuine 2-byte wire message; the padding byte
    /// from `channel_voice` is dropped before it reaches midir.)
    #[inline]
    pub fn program_change(&self, channel: u8, program: u8) -> Result<(), MidiOutError> {
        let msg = channel_voice(0xC0, channel, program, 0);
        self.send(&msg[..2])
    }

    /// All Notes Off on a channel (CC 123 = All Notes Off).
    #[inline]
    pub fn all_notes_off(&self, channel: u8) -> Result<(), MidiOutError> {
        self.control_change(channel, 123, 0)
    }
}

/// Build a 3-byte Channel Voice message: status nibble | channel, then two
/// data bytes masked to 7 bits (MIDI data bytes are always 0..=127).
#[inline]
fn channel_voice(status: u8, channel: u8, data1: u8, data2: u8) -> [u8; 3] {
    [status | (channel & 0x0F), data1 & 0x7F, data2 & 0x7F]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_voice_encoding() {
        // Note On C4 (60) velocity 100 on channel 0 = [0x90, 0x3C, 0x64]
        assert_eq!(channel_voice(0x90, 0, 60, 100), [0x90, 60, 100]);
        // Channel and data bytes are masked, never overflow into the wrong field.
        assert_eq!(channel_voice(0x90, 0xFF, 200, 200), [0x9F, 72, 72]);
    }
}
