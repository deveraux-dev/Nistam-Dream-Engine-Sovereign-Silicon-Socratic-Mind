// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! MIDI 2.0 Universal MIDI Packet (UMP) Flux Stream Ingestion.
//!
//! Provides zero-heap decoding of 32-bit, 64-bit, 96-bit, and 128-bit UMP packets
//! running at a 200 Hz event sampling frequency with 32-bit high-resolution per-note controllers.

/// UMP Message Types (4-bit header in bits 31..28 of word 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UmpMessageType {
    /// 32-bit Utility Messages (Clock, NOP, Jitter Reduction).
    Utility = 0x0,
    /// 32-bit System Real-Time / System Common.
    System = 0x1,
    /// 64-bit MIDI 1.0 Channel Voice.
    Midi1ChannelVoice = 0x2,
    /// 64-bit Data Messages (64-bit SysEx7).
    Data64 = 0x3,
    /// 64-bit MIDI 2.0 Channel Voice (32-bit High-Res Controllers).
    Midi2ChannelVoice = 0x4,
    /// 128-bit Extended Data Messages (SysEx8).
    Data128 = 0x5,
    /// 96-bit Extended Message Format.
    Extended96 = 0x6,
    /// 128-bit Flex Data Messages.
    FlexData = 0xD,
    /// 128-bit UMP Stream Messages (Endpoint Discovery, Protocol Negotiation).
    UmpStream = 0xF,
    /// Other reserved message types.
    Reserved(u8),
}

impl From<u8> for UmpMessageType {
    fn from(nibble: u8) -> Self {
        match nibble & 0x0F {
            0x0 => Self::Utility,
            0x1 => Self::System,
            0x2 => Self::Midi1ChannelVoice,
            0x3 => Self::Data64,
            0x4 => Self::Midi2ChannelVoice,
            0x5 => Self::Data128,
            0x6 => Self::Extended96,
            0xD => Self::FlexData,
            0xF => Self::UmpStream,
            other => Self::Reserved(other),
        }
    }
}

/// Universal MIDI Packet (UMP) with 32/64/96/128-bit word layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmpPacket {
    /// 32-bit single word packet.
    Word32(u32),
    /// 64-bit two word packet.
    Word64([u32; 2]),
    /// 96-bit three word packet.
    Word96([u32; 3]),
    /// 128-bit four word packet.
    Word128([u32; 4]),
}

impl UmpPacket {
    /// Parse a UMP packet from a slice of 32-bit big-endian/native words.
    pub fn parse(words: &[u32]) -> Option<(Self, usize)> {
        if words.is_empty() {
            return None;
        }
        let w0 = words[0];
        let msg_type = UmpMessageType::from(((w0 >> 28) & 0x0F) as u8);
        match msg_type {
            UmpMessageType::Utility | UmpMessageType::System => {
                Some((Self::Word32(w0), 1))
            }
            UmpMessageType::Midi1ChannelVoice | UmpMessageType::Data64 | UmpMessageType::Midi2ChannelVoice => {
                if words.len() >= 2 {
                    Some((Self::Word64([w0, words[1]]), 2))
                } else {
                    None
                }
            }
            UmpMessageType::Extended96 => {
                if words.len() >= 3 {
                    Some((Self::Word96([w0, words[1], words[2]]), 3))
                } else {
                    None
                }
            }
            UmpMessageType::Data128 | UmpMessageType::FlexData | UmpMessageType::UmpStream | UmpMessageType::Reserved(_) => {
                if words.len() >= 4 {
                    Some((Self::Word128([w0, words[1], words[2], words[3]]), 4))
                } else {
                    None
                }
            }
        }
    }

    /// Extract 32-bit high-resolution controller data value (if MIDI 2.0 Channel Voice).
    pub fn midi2_controller_value(&self) -> Option<u32> {
        match self {
            Self::Word64([w0, w1]) => {
                let msg_type = (w0 >> 28) & 0x0F;
                if msg_type == 0x4 {
                    Some(*w1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// 200 Hz UMP Flux Stream Ingestion Queue.
pub struct UmpFluxStream {
    /// Nominal sampling rate in Hz (default: 200 Hz).
    pub sample_rate_hz: u32,
    /// Ring buffer capacity.
    buffer: [u32; 1024],
    head: usize,
    tail: usize,
}

impl Default for UmpFluxStream {
    fn default() -> Self {
        Self::new(200)
    }
}

impl UmpFluxStream {
    /// Create a new UMP flux stream with target sampling rate.
    pub const fn new(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz,
            buffer: [0u32; 1024],
            head: 0,
            tail: 0,
        }
    }

    /// Push a raw 32-bit word into the flux ring buffer.
    #[inline]
    pub fn push_word(&mut self, word: u32) -> bool {
        let next = (self.head + 1) & 1023;
        if next == self.tail {
            return false; // Buffer full
        }
        self.buffer[self.head] = word;
        self.head = next;
        true
    }

    /// Pop next available UMP packet.
    pub fn pop_packet(&mut self) -> Option<UmpPacket> {
        let available = (self.head + 1024 - self.tail) & 1023;
        if available == 0 {
            return None;
        }

        let w0 = self.buffer[self.tail];
        let msg_type = UmpMessageType::from(((w0 >> 28) & 0x0F) as u8);
        let needed_words = match msg_type {
            UmpMessageType::Utility | UmpMessageType::System => 1,
            UmpMessageType::Midi1ChannelVoice | UmpMessageType::Data64 | UmpMessageType::Midi2ChannelVoice => 2,
            UmpMessageType::Extended96 => 3,
            _ => 4,
        };

        if available < needed_words {
            return None; // Wait for full packet
        }

        let packet = match needed_words {
            1 => UmpPacket::Word32(w0),
            2 => UmpPacket::Word64([w0, self.buffer[(self.tail + 1) & 1023]]),
            3 => UmpPacket::Word96([
                w0,
                self.buffer[(self.tail + 1) & 1023],
                self.buffer[(self.tail + 2) & 1023],
            ]),
            4 => UmpPacket::Word128([
                w0,
                self.buffer[(self.tail + 1) & 1023],
                self.buffer[(self.tail + 2) & 1023],
                self.buffer[(self.tail + 3) & 1023],
            ]),
            _ => unreachable!(),
        };

        self.tail = (self.tail + needed_words) & 1023;
        Some(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ump_packet_parsing() {
        // MIDI 2.0 64-bit Channel Voice packet (Message Type 0x4)
        let w0 = 0x4010_4000; // Type 4, Group 0, Status 0x1 (Note On), Note 0x40
        let w1 = 0xFFFF_0000; // 32-bit Velocity (16-bit MSB normalized)
        let words = [w0, w1];

        let (packet, len) = UmpPacket::parse(&words).expect("valid packet");
        assert_eq!(len, 2);
        assert_eq!(packet.midi2_controller_value(), Some(0xFFFF_0000));
    }

    #[test]
    fn test_flux_stream_ring_buffer() {
        let mut stream = UmpFluxStream::new(200);
        assert_eq!(stream.sample_rate_hz, 200);

        // Push 32-bit Utility word (Type 0x0)
        assert!(stream.push_word(0x0010_0000));
        let p = stream.pop_packet().expect("packet available");
        assert_eq!(p, UmpPacket::Word32(0x0010_0000));
    }
}
