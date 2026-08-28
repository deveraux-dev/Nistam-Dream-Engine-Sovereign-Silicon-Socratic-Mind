//! Lazy byte-stream parser for big-endian UMP wire data.

use crate::message::{Message, ParseError};
use crate::packet::{Stamped, Ump};

/// A stateful reader that parses a stream of UMP packets from big-endian bytes.
pub struct UmpReader<'a> {
    /// Input byte buffer.
    bytes: &'a [u8],
    /// Current read position in bytes.
    cursor: usize,
    /// Accumulated universal tick in microseconds.
    universal_tick_us: i64,
    /// Whether parsing has stopped.
    stopped: bool,
}

impl<'a> UmpReader<'a> {
    /// Create a new UMP reader from a byte slice.
    #[inline]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0, universal_tick_us: 0, stopped: false }
    }
}

impl<'a> Iterator for UmpReader<'a> {
    type Item = Result<Stamped<Message>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped || self.cursor >= self.bytes.len() {
            return None;
        }

        if self.bytes.len() - self.cursor < 4 {
            self.stopped = true;
            return Some(Err(ParseError::TruncatedPacket));
        }

        let mt = self.bytes[self.cursor] >> 4;
        let len = packet_len(mt);
        if self.bytes.len() - self.cursor < len {
            self.stopped = true;
            return Some(Err(ParseError::TruncatedPacket));
        }

        let mut words = [0u32; 4];
        let word_count = len / 4;
        let mut i = 0;
        while i < word_count {
            let off = self.cursor + (i * 4);
            words[i] = u32::from_be_bytes([
                self.bytes[off],
                self.bytes[off + 1],
                self.bytes[off + 2],
                self.bytes[off + 3],
            ]);
            i += 1;
        }
        self.cursor += len;

        let message = match Message::try_from_ump(Ump::new(words)) {
            Ok(message) => message,
            Err(error) => return Some(Err(error)),
        };

        match message {
            Message::JrClock { time_units } => {
                self.universal_tick_us = time_units as i64 * 32;
            }
            Message::JrTimestamp { delta } => {
                self.universal_tick_us += delta as i64 * 32;
            }
            _ => {}
        }

        Some(Ok(Stamped { universal_tick_us: self.universal_tick_us, payload: message }))
    }
}

/// Append one message to a big-endian UMP byte stream — the writer half of
/// [`UmpReader`]'s contract: `UmpReader::new(&out)` yields back exactly what
/// was appended (the L07 bijection the round-trip tests pin).
pub fn append_message(out: &mut Vec<u8>, message: Message) {
    let ump = message.to_ump();
    let mt = (ump.words[0] >> 28) as u8;
    for word in ump.words.iter().take(packet_len(mt) / 4) {
        out.extend_from_slice(&word.to_be_bytes());
    }
}

/// Get the packet size in bytes based on the message type nibble.
#[inline]
const fn packet_len(mt: u8) -> usize {
    match mt {
        0x0 | 0x1 | 0x2 => 4,
        0x3 | 0x4 => 8,
        0x5 => 16,
        0x6..=0xF => 16,
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::packet::{Channel, Group};

    fn append_ump_bytes(out: &mut Vec<u8>, message: Message, _bytes: usize) {
        append_message(out, message);
    }

    #[test]
    fn empty_input_returns_none() {
        assert!(UmpReader::new(&[]).next().is_none());
    }

    #[test]
    fn truncated_packet_yields_truncated_packet_err_then_none() {
        let bytes = [0x40, 0x90, 0x40, 0x00];
        let mut reader = UmpReader::new(&bytes);
        assert_eq!(reader.next(), Some(Err(ParseError::TruncatedPacket)));
        assert_eq!(reader.next(), None);
    }

    #[test]
    fn jr_timestamp_accumulates_universal_tick_us() {
        let mut bytes = Vec::new();
        append_ump_bytes(&mut bytes, Message::JrClock { time_units: 10 }, 4);
        append_ump_bytes(&mut bytes, Message::JrTimestamp { delta: 5 }, 4);
        append_ump_bytes(&mut bytes, Message::JrTimestamp { delta: 2 }, 4);

        let events: Vec<_> = UmpReader::new(&bytes).collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(events[0].universal_tick_us, 320);
        assert_eq!(events[1].universal_tick_us, 480);
        assert_eq!(events[2].universal_tick_us, 544);
    }

    #[test]
    fn note_on_after_jr_timestamp_carries_correct_tick() {
        let mut bytes = Vec::new();
        append_ump_bytes(&mut bytes, Message::JrTimestamp { delta: 3 }, 4);
        append_ump_bytes(&mut bytes, Message::NoteOn { group: Group(0), channel: Channel(1), note: 64, velocity: 0x4000_0000, attribute_type: 0, attribute_data: 0 }, 8);

        let events: Vec<_> = UmpReader::new(&bytes).collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(events[1].universal_tick_us, 96);
    }

    #[test]
    fn variable_length_dispatch_4_8_16_bytes() {
        let mut bytes = Vec::new();
        append_ump_bytes(&mut bytes, Message::JrTimestamp { delta: 1 }, 4);
        append_ump_bytes(&mut bytes, Message::Cc32 { group: Group(0), channel: Channel(0), index: 1, value: 2 }, 8);
        append_ump_bytes(&mut bytes, Message::Sysex8 { group: Group(0), status: 0, data: [7; 13] }, 16);

        let events: Vec<_> = UmpReader::new(&bytes).collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0].payload, Message::JrTimestamp { .. }));
        assert!(matches!(events[1].payload, Message::Cc32 { .. }));
        assert!(matches!(events[2].payload, Message::Sysex8 { .. }));
    }
}
