//! Typed UMP messages and encode/decode logic.

use crate::packet::{Channel, Group, Ump};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Message {
    JrClock { time_units: u16 },
    JrTimestamp { delta: u16 },
    NoteOn { group: Group, channel: Channel, note: u8, velocity: u32, attribute_type: u8, attribute_data: u16 },
    NoteOff { group: Group, channel: Channel, note: u8, velocity: u32, attribute_type: u8, attribute_data: u16 },
    Cc32 { group: Group, channel: Channel, index: u8, value: u32 },
    PerNoteCc { group: Group, channel: Channel, note: u8, index: u8, value: u32 },
    PerNotePitchBend { group: Group, channel: Channel, note: u8, value: u32 },
    PitchBend { group: Group, channel: Channel, value: u32 },
    ProgramChange { group: Group, channel: Channel, program: u8, bank_lsb: u8, bank_msb: u8 },
    ChannelPressure { group: Group, channel: Channel, value: u32 },
    PerNotePressure { group: Group, channel: Channel, note: u8, value: u32 },
    Sysex8 { group: Group, status: u8, data: [u8; 13] },
    Unknown { mt: u8, words: [u32; 4] },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParseError {
    TruncatedPacket,
    InvalidStatus { mt: u8, status: u8 },
}

impl Message {
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
            0x1 | 0x2 | 0x3 | 0xF => Ok(Message::Unknown { mt, words: ump.words }),
            0x6..=0xE => Ok(Message::Unknown { mt, words: ump.words }),
            _ => Ok(Message::Unknown { mt, words: ump.words }),
        }
    }

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

#[inline]
const fn pack_w0(mt: u8, group: Group, status: u8, channel: Channel, index_or_note: u8, attr_type: u8) -> u32 {
    ((mt as u32 & 0x0f) << 28)
        | ((group.0 as u32 & 0x0f) << 24)
        | ((status as u32 & 0x0f) << 20)
        | ((channel.0 as u32 & 0x0f) << 16)
        | ((index_or_note as u32) << 8)
        | attr_type as u32
}

#[inline]
fn words_to_bytes(words: [u32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&words[0].to_be_bytes());
    out[4..8].copy_from_slice(&words[1].to_be_bytes());
    out[8..12].copy_from_slice(&words[2].to_be_bytes());
    out[12..16].copy_from_slice(&words[3].to_be_bytes());
    out
}

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
        assert_round_trip(Message::NoteOn { group: Group(2), channel: Channel(3), note: 64, velocity: 0x7fff_0000, attribute_type: 1, attribute_data: 9 });
    }

    #[test]
    fn note_off_midi2_round_trip() {
        assert_round_trip(Message::NoteOff { group: Group(1), channel: Channel(4), note: 60, velocity: 0x4000_0000, attribute_type: 0, attribute_data: 0 });
    }

    #[test]
    fn cc32_round_trip() {
        assert_round_trip(Message::Cc32 { group: Group(0), channel: Channel(2), index: 74, value: 0x1234_5678 });
    }

    #[test]
    fn per_note_cc_round_trip() {
        assert_round_trip(Message::PerNoteCc { group: Group(3), channel: Channel(2), note: 64, index: 11, value: 0xfeed_beef });
    }

    #[test]
    fn pitch_bend_round_trip() {
        assert_round_trip(Message::PitchBend { group: Group(1), channel: Channel(1), value: 0x8000_0000 });
    }

    #[test]
    fn sysex8_round_trip() {
        assert_round_trip(Message::Sysex8 { group: Group(7), status: 3, data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13] });
    }

    #[test]
    fn unknown_mt_forward_compat() {
        let ump = Ump::new([0x6000_0000, 1, 2, 3]);
        assert_eq!(Message::try_from_ump(ump), Ok(Message::Unknown { mt: 0x6, words: [0x6000_0000, 1, 2, 3] }));
    }

    #[test]
    fn invalid_status_in_known_mt_errors() {
        let ump = Ump::new([0x4070_0000, 0, 0, 0]);
        assert_eq!(Message::try_from_ump(ump), Err(ParseError::InvalidStatus { mt: 0x4, status: 0x7 }));
    }
}
