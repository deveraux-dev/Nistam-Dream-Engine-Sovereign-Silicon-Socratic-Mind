//! Typed UMP messages and encode/decode logic.

use crate::packet::{Channel, Group, Ump};

/// A UMP message, fully decoded.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Message {
    /// JR Clock.
    JrClock {
        /// Time units.
        time_units: u16,
    },
    /// JR Timestamp.
    JrTimestamp {
        /// Delta ticks.
        delta: u16,
    },
    /// MIDI 2.0 Note On.
    NoteOn {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Velocity (32-bit).
        velocity: u32,
        /// Attribute type.
        attribute_type: u8,
        /// Attribute data.
        attribute_data: u16,
    },
    /// MIDI 2.0 Note Off.
    NoteOff {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Velocity (32-bit).
        velocity: u32,
        /// Attribute type.
        attribute_type: u8,
        /// Attribute data.
        attribute_data: u16,
    },
    /// MIDI 2.0 Control Change (32-bit value).
    Cc32 {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Controller index.
        index: u8,
        /// Controller value.
        value: u32,
    },
    /// MIDI 2.0 Per-Note CC.
    PerNoteCc {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Controller index.
        index: u8,
        /// Controller value.
        value: u32,
    },
    /// MIDI 2.0 Per-Note Pitch Bend.
    PerNotePitchBend {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Pitch bend value.
        value: u32,
    },
    /// MIDI 2.0 Pitch Bend.
    PitchBend {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Pitch bend value.
        value: u32,
    },
    /// MIDI 2.0 Program Change.
    ProgramChange {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Program number.
        program: u8,
        /// Bank LSB.
        bank_lsb: u8,
        /// Bank MSB.
        bank_msb: u8,
    },
    /// MIDI 2.0 Channel Pressure.
    ChannelPressure {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Pressure value.
        value: u32,
    },
    /// MIDI 2.0 Per-Note Pressure.
    PerNotePressure {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Pressure value.
        value: u32,
    },
    /// MIDI 2.0 SysEx8.
    Sysex8 {
        /// Group nibble.
        group: Group,
        /// Status.
        status: u8,
        /// SysEx data.
        data: [u8; 13],
    },

    /// MIDI 1.0 Note Off.
    Midi1NoteOff {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Velocity.
        velocity: u8,
    },
    /// MIDI 1.0 Note On.
    Midi1NoteOn {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Velocity.
        velocity: u8,
    },
    /// MIDI 1.0 Poly Pressure.
    Midi1PolyPressure {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Note number.
        note: u8,
        /// Pressure value.
        pressure: u8,
    },
    /// MIDI 1.0 Control Change.
    Midi1ControlChange {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Controller index.
        index: u8,
        /// Controller value.
        value: u8,
    },
    /// MIDI 1.0 Program Change.
    Midi1ProgramChange {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Program number.
        program: u8,
    },
    /// MIDI 1.0 Channel Pressure.
    Midi1ChannelPressure {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Pressure value.
        pressure: u8,
    },
    /// MIDI 1.0 Pitch Bend.
    Midi1PitchBend {
        /// Group nibble.
        group: Group,
        /// Channel.
        channel: Channel,
        /// Pitch bend value.
        value: u16,
    },

    /// An unknown or unrecognized message type.
    Unknown {
        /// Message type nibble.
        mt: u8,
        /// Raw words.
        words: [u32; 4],
    },
}

/// Error parsing a UMP message.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParseError {
    /// The packet was truncated.
    TruncatedPacket,
    /// Invalid status nibble for the message type.
    InvalidStatus {
        /// Message type nibble.
        mt: u8,
        /// Status nibble.
        status: u8,
    },
}

impl Message {
    /// Decode a UMP packet into a Message.
    pub fn try_from_ump(ump: Ump) -> Result<Message, ParseError> {
        let w0 = ump.words[0];
        let mt = ((w0 >> 28) & 0x0f) as u8;
        let group = Group(((w0 >> 24) & 0x0f) as u8);
        let status = ((w0 >> 20) & 0x0f) as u8;
        let channel = Channel(((w0 >> 16) & 0x0f) as u8);
        let index_or_note = ((w0 >> 8) & 0xff) as u8;
        let attr_type = (w0 & 0xff) as u8;
        let w1 = ump.words[1];

        match mt {
            0x0 => match status {
                0x0 => Ok(Message::Unknown { mt, words: ump.words }),
                0x1 => Ok(Message::JrClock { time_units: (w0 & 0xffff) as u16 }),
                0x2 => Ok(Message::JrTimestamp { delta: (w0 & 0xffff) as u16 }),
                _ => Err(ParseError::InvalidStatus { mt, status }),
            },
            0x2 => match status {
                0x8 => Ok(Message::Midi1NoteOff { group, channel, note: index_or_note, velocity: attr_type }),
                0x9 => Ok(Message::Midi1NoteOn { group, channel, note: index_or_note, velocity: attr_type }),
                0xA => Ok(Message::Midi1PolyPressure { group, channel, note: index_or_note, pressure: attr_type }),
                0xB => Ok(Message::Midi1ControlChange { group, channel, index: index_or_note, value: attr_type }),
                0xC => Ok(Message::Midi1ProgramChange { group, channel, program: index_or_note }),
                0xD => Ok(Message::Midi1ChannelPressure { group, channel, pressure: index_or_note }),
                0xE => Ok(Message::Midi1PitchBend { group, channel, value: ((attr_type as u16) << 7) | index_or_note as u16 }),
                _ => Err(ParseError::InvalidStatus { mt, status }),
            },
            0x4 => match status {
                0x1 => Ok(Message::PerNoteCc {
                    group,
                    channel,
                    note: index_or_note,
                    index: attr_type,
                    value: w1,
                }),
                0x6 => Ok(Message::PerNotePitchBend { group, channel, note: index_or_note, value: w1 }),
                0x8 => Ok(Message::NoteOff {
                    group,
                    channel,
                    note: index_or_note,
                    velocity: w1 & 0xffff_0000,
                    attribute_type: attr_type,
                    attribute_data: (w1 & 0xffff) as u16,
                }),
                0x9 => Ok(Message::NoteOn {
                    group,
                    channel,
                    note: index_or_note,
                    velocity: w1 & 0xffff_0000,
                    attribute_type: attr_type,
                    attribute_data: (w1 & 0xffff) as u16,
                }),
                0xA => Ok(Message::PerNotePressure { group, channel, note: index_or_note, value: w1 }),
                0xB => Ok(Message::Cc32 { group, channel, index: index_or_note, value: w1 }),
                0xC => Ok(Message::ProgramChange {
                    group,
                    channel,
                    program: index_or_note,
                    bank_lsb: ((w1 >> 8) & 0xff) as u8,
                    bank_msb: (w1 & 0xff) as u8,
                }),
                0xD => Ok(Message::ChannelPressure { group, channel, value: w1 }),
                0xE => Ok(Message::PitchBend { group, channel, value: w1 }),
                _ => Err(ParseError::InvalidStatus { mt, status }),
            },
            0x5 => {
                let mut data = [0u8; 13];
                let bytes = words_to_bytes(ump.words);
                data.copy_from_slice(&bytes[3..16]);
                Ok(Message::Sysex8 { group, status, data })
            }
            0x1 | 0x3 | 0xF => Ok(Message::Unknown { mt, words: ump.words }),
            0x6..=0xE => Ok(Message::Unknown { mt, words: ump.words }),
            _ => Ok(Message::Unknown { mt, words: ump.words }),
        }
    }

    /// Encode a Message into a UMP packet.
    pub fn to_ump(self) -> Ump {
        match self {
            Message::JrClock { time_units } => Ump::new([pack_w0(0x0, Group(0), 0x1, Channel(0), 0, 0) | time_units as u32, 0, 0, 0]),
            Message::JrTimestamp { delta } => Ump::new([pack_w0(0x0, Group(0), 0x2, Channel(0), 0, 0) | delta as u32, 0, 0, 0]),
            Message::NoteOn { group, channel, note, velocity, attribute_type, attribute_data } => Ump::new([
                pack_w0(0x4, group, 0x9, channel, note, attribute_type),
                (velocity & 0xffff_0000) | attribute_data as u32,
                0,
                0,
            ]),
            Message::NoteOff { group, channel, note, velocity, attribute_type, attribute_data } => Ump::new([
                pack_w0(0x4, group, 0x8, channel, note, attribute_type),
                (velocity & 0xffff_0000) | attribute_data as u32,
                0,
                0,
            ]),
            Message::Cc32 { group, channel, index, value } => Ump::new([pack_w0(0x4, group, 0xB, channel, index, 0), value, 0, 0]),
            Message::PerNoteCc { group, channel, note, index, value } => Ump::new([pack_w0(0x4, group, 0x1, channel, note, index), value, 0, 0]),
            Message::PerNotePitchBend { group, channel, note, value } => Ump::new([pack_w0(0x4, group, 0x6, channel, note, 0), value, 0, 0]),
            Message::PitchBend { group, channel, value } => Ump::new([pack_w0(0x4, group, 0xE, channel, 0, 0), value, 0, 0]),
            Message::ProgramChange { group, channel, program, bank_lsb, bank_msb } => Ump::new([
                pack_w0(0x4, group, 0xC, channel, program, 0),
                ((bank_lsb as u32) << 8) | bank_msb as u32,
                0,
                0,
            ]),
            Message::ChannelPressure { group, channel, value } => Ump::new([pack_w0(0x4, group, 0xD, channel, 0, 0), value, 0, 0]),
            Message::PerNotePressure { group, channel, note, value } => Ump::new([pack_w0(0x4, group, 0xA, channel, note, 0), value, 0, 0]),
            Message::Midi1NoteOff { group, channel, note, velocity } => Ump::new([pack_w0(0x2, group, 0x8, channel, note, velocity), 0, 0, 0]),
            Message::Midi1NoteOn { group, channel, note, velocity } => Ump::new([pack_w0(0x2, group, 0x9, channel, note, velocity), 0, 0, 0]),
            Message::Midi1PolyPressure { group, channel, note, pressure } => Ump::new([pack_w0(0x2, group, 0xA, channel, note, pressure), 0, 0, 0]),
            Message::Midi1ControlChange { group, channel, index, value } => Ump::new([pack_w0(0x2, group, 0xB, channel, index, value), 0, 0, 0]),
            Message::Midi1ProgramChange { group, channel, program } => Ump::new([pack_w0(0x2, group, 0xC, channel, program, 0), 0, 0, 0]),
            Message::Midi1ChannelPressure { group, channel, pressure } => Ump::new([pack_w0(0x2, group, 0xD, channel, pressure, 0), 0, 0, 0]),
            Message::Midi1PitchBend { group, channel, value } => Ump::new([
                pack_w0(0x2, group, 0xE, channel, (value & 0x7f) as u8, ((value >> 7) & 0x7f) as u8),
                0,
                0,
                0,
            ]),
            Message::Sysex8 { group, status, data } => {
                let mut bytes = [0u8; 16];
                bytes[0] = (0x5 << 4) | (group.0 & 0x0f);
                bytes[1] = (status & 0x0f) << 4;
                bytes[2] = 0;
                bytes[3..16].copy_from_slice(&data);
                Ump::new(bytes_to_words(bytes))
            }
            Message::Unknown { words, .. } => Ump::new(words),
        }
    }
}

/// Pack the first word of a UMP packet.
#[inline]
const fn pack_w0(mt: u8, group: Group, status: u8, channel: Channel, index_or_note: u8, attr_type: u8) -> u32 {
    ((mt as u32 & 0x0f) << 28)
        | ((group.0 as u32 & 0x0f) << 24)
        | ((status as u32 & 0x0f) << 20)
        | ((channel.0 as u32 & 0x0f) << 16)
        | ((index_or_note as u32) << 8)
        | attr_type as u32
}

/// Convert 32-bit words to bytes.
#[inline]
fn words_to_bytes(words: [u32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&words[0].to_be_bytes());
    out[4..8].copy_from_slice(&words[1].to_be_bytes());
    out[8..12].copy_from_slice(&words[2].to_be_bytes());
    out[12..16].copy_from_slice(&words[3].to_be_bytes());
    out
}

/// Convert bytes to 32-bit words.
#[inline]
fn bytes_to_words(bytes: [u8; 16]) -> [u32; 4] {
    [
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_round_trip(message: Message) {
        let ump = message.to_ump();
        let decoded = Message::try_from_ump(ump).unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn note_on_midi2_round_trip() {
        assert_round_trip(Message::NoteOn {
            group: Group(0),
            channel: Channel(0),
            note: 64,
            velocity: 0x4000_0000,
            attribute_type: 0,
            attribute_data: 0,
        });
    }

    #[test]
    fn note_off_midi2_round_trip() {
        assert_round_trip(Message::NoteOff {
            group: Group(5),
            channel: Channel(7),
            note: 60,
            velocity: 0x8000_0000,
            attribute_type: 0,
            attribute_data: 0,
        });
    }

    #[test]
    fn program_change_round_trip() {
        assert_round_trip(Message::ProgramChange {
            group: Group(1),
            channel: Channel(2),
            program: 7,
            bank_lsb: 0x12,
            bank_msb: 0x34,
        });
    }

    #[test]
    fn midi1_note_on_round_trip() {
        assert_round_trip(Message::Midi1NoteOn {
            group: Group(3),
            channel: Channel(5),
            note: 60,
            velocity: 0x64,
        });
    }

    #[test]
    fn midi1_program_change_round_trip() {
        assert_round_trip(Message::Midi1ProgramChange {
            group: Group(0),
            channel: Channel(0),
            program: 42,
        });
    }

    #[test]
    fn jr_timestamp_round_trip() {
        assert_round_trip(Message::JrTimestamp { delta: 256 });
    }
}
