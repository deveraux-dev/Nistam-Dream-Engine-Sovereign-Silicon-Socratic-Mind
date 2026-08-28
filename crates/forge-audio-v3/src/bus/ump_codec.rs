//! ump_codec — MixerCommand ⇆ Universal MIDI Packet (MIDI 2.0) codec for the
//! master-bus flight recorder.
//!
//! Every command the feeder thread applies is also encoded to one `Ump` and fed to
//! a `forge_ump::Recorder`, so the master bus grows a sealed, scrubbable tape of its
//! own command history (ZD-003 timeline-scrubber substrate). This is a PRIVATE
//! application mapping, NOT standard MIDI voice traffic: it rides message-type
//! nibble `HUB_MT` (0xE, reserved in MIDI 2.0) and group `HUB_GROUP` (0xE) so a HUB
//! command atom can never be mistaken for a real Note/CC message (MT 0x4).
//!
//! Packet layout (16 bytes, POD):
//!   word0 = [ MT:4 | GRP:4 | TAG:8 | DECK:8 | AUX:8 ]
//!   word1 = scalar payload (f32::to_bits for gains/pan/seek, or packed ints)
//!   word2 = extended payload (weather fog; else 0)
//!   word3 = reserved (0)
//!
//! `encode` is TOTAL — every `MixerCommand` yields exactly one packet. `decode`
//! recovers a `HubEvent` (tag + deck + aux + scalar) for the losslessly-encodable
//! set; opaque variants (track/buffer payloads) round-trip their tag+deck only,
//! which is all a command scrubber needs to re-seek deck state.

use forge_ump::packet::{Stamped, Ump};

use super::command::MixerCommand;

/// Message-type nibble for HUB command atoms (reserved in MIDI 2.0 → private lane).
pub const HUB_MT: u8 = 0xE;
/// UMP group carrying the HUB command lane.
pub const HUB_GROUP: u8 = 0xE;
/// Sentinel deck byte for commands with no deck (master/global).
pub const DECK_NONE: u8 = 0xFF;

/// Command discriminant carried in the TAG byte. `essence_id` on the tape is
/// `tag & 0x3F`, so scrub-by-codeword lands on the command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HubTag {
    Play = 0x01,
    Pause = 0x02,
    Stop = 0x03,
    SetVolume = 0x04,
    SetPan = 0x05,
    ToggleMute = 0x06,
    ToggleSolo = 0x07,
    SetCrossfader = 0x08,
    SetMasterVolume = 0x09,
    Seek = 0x0A,
    ApplyEffect = 0x0B,
    RemoveEffect = 0x0C,
    Enqueue = 0x0D,
    SetSync = 0x0E,
    ToggleLoop = 0x0F,
    SequencerPlay = 0x10,
    SequencerStop = 0x11,
    SequencerSetBpm = 0x12,
    SequencerSetStep = 0x13,
    AmbientWeather = 0x14,
    TriggerSampler = 0x15,
    StopSampler = 0x16,
    ToggleFx = 0x17,
    SetFxIntensity = 0x18,
    ToggleMic = 0x19,
    ToggleBroadcast = 0x1A,
    ToggleRecording = 0x1B,
    Shutdown = 0x1C,
    /// Any variant not given a distinct scalar encoding (Param/Action/LoadDeck/…).
    Opaque = 0x3F,
}

impl HubTag {
    /// Recover a tag from its byte, or `None` if it is not a known HUB tag.
    pub fn from_u8(b: u8) -> Option<HubTag> {
        use HubTag::*;
        Some(match b {
            0x01 => Play, 0x02 => Pause, 0x03 => Stop, 0x04 => SetVolume,
            0x05 => SetPan, 0x06 => ToggleMute, 0x07 => ToggleSolo,
            0x08 => SetCrossfader, 0x09 => SetMasterVolume, 0x0A => Seek,
            0x0B => ApplyEffect, 0x0C => RemoveEffect, 0x0D => Enqueue,
            0x0E => SetSync, 0x0F => ToggleLoop, 0x10 => SequencerPlay,
            0x11 => SequencerStop, 0x12 => SequencerSetBpm, 0x13 => SequencerSetStep,
            0x14 => AmbientWeather, 0x15 => TriggerSampler, 0x16 => StopSampler,
            0x17 => ToggleFx, 0x18 => SetFxIntensity, 0x19 => ToggleMic,
            0x1A => ToggleBroadcast, 0x1B => ToggleRecording, 0x1C => Shutdown,
            0x3F => Opaque,
            _ => return None,
        })
    }

    /// Tape codeword (0..=63) — `essence_id` for scrub-by-family.
    #[inline]
    pub fn essence(self) -> u8 {
        (self as u8) & 0x3F
    }
}

/// A decoded HUB command atom — enough to re-seek master/deck state on scrub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HubEvent {
    pub tag: HubTag,
    /// Deck index, or [`DECK_NONE`] for master/global commands.
    pub deck: u8,
    /// Small auxiliary byte (fx/sampler slot, sync mode, sequencer track).
    pub aux: u8,
    /// Raw scalar payload word (word1) — interpret via [`HubEvent::value_f32`].
    pub value_bits: u32,
    /// Extended payload word (word2) — weather fog, else 0.
    pub ext_bits: u32,
}

impl HubEvent {
    /// Interpret the scalar payload as the f32 it was encoded from.
    #[inline]
    pub fn value_f32(self) -> f32 {
        f32::from_bits(self.value_bits)
    }
}

#[inline]
fn word0(tag: HubTag, deck: u8, aux: u8) -> u32 {
    ((HUB_MT as u32) << 28)
        | ((HUB_GROUP as u32) << 24)
        | ((tag as u32) << 16)
        | ((deck as u32) << 8)
        | (aux as u32)
}

#[inline]
fn deck_u8(deck: usize) -> u8 {
    if deck < 0xFF { deck as u8 } else { 0xFE }
}

/// Encode one command into a single UMP-shaped packet. Total over `MixerCommand`.
pub fn encode_ump(cmd: &MixerCommand) -> Ump {
    use HubTag as T;
    let (tag, deck, aux, w1, w2) = match cmd {
        MixerCommand::Play { deck, .. } => (T::Play, deck_u8(*deck), 0, 0, 0),
        MixerCommand::Pause { deck } => (T::Pause, deck_u8(*deck), 0, 0, 0),
        MixerCommand::Stop { deck } => (T::Stop, deck_u8(*deck), 0, 0, 0),
        MixerCommand::SetVolume { deck, volume } => {
            (T::SetVolume, deck_u8(*deck), 0, volume.to_bits(), 0)
        }
        MixerCommand::SetPan { deck, pan } => (T::SetPan, deck_u8(*deck), 0, pan.to_bits(), 0),
        MixerCommand::ToggleMute { deck } => (T::ToggleMute, deck_u8(*deck), 0, 0, 0),
        MixerCommand::ToggleSolo { deck } => (T::ToggleSolo, deck_u8(*deck), 0, 0, 0),
        MixerCommand::SetCrossfader { position } => {
            (T::SetCrossfader, DECK_NONE, 0, position.to_bits(), 0)
        }
        MixerCommand::SetMasterVolume { volume } => {
            (T::SetMasterVolume, DECK_NONE, 0, volume.to_bits(), 0)
        }
        MixerCommand::Seek { deck, position_secs } => {
            (T::Seek, deck_u8(*deck), 0, (*position_secs as f32).to_bits(), 0)
        }
        MixerCommand::ApplyEffect { deck, .. } => (T::ApplyEffect, deck_u8(*deck), 0, 0, 0),
        MixerCommand::RemoveEffect { deck, effect_id } => {
            (T::RemoveEffect, deck_u8(*deck), 0, *effect_id as u32, 0)
        }
        MixerCommand::Enqueue { deck, .. } => (T::Enqueue, deck_u8(*deck), 0, 0, 0),
        MixerCommand::SetSync { deck, mode } => {
            let m = match mode.as_str() { "leader" => 2, "follower" => 1, _ => 0 };
            (T::SetSync, deck_u8(*deck), m, 0, 0)
        }
        MixerCommand::ToggleLoop { deck } => (T::ToggleLoop, deck_u8(*deck), 0, 0, 0),
        MixerCommand::SequencerPlay => (T::SequencerPlay, DECK_NONE, 0, 0, 0),
        MixerCommand::SequencerStop => (T::SequencerStop, DECK_NONE, 0, 0, 0),
        MixerCommand::SequencerSetBpm { bpm } => {
            (T::SequencerSetBpm, DECK_NONE, 0, bpm.to_bits(), 0)
        }
        MixerCommand::SequencerSetStep { track, step, note } => (
            T::SequencerSetStep,
            DECK_NONE,
            *track as u8,
            ((*step as u32) << 8) | (*note as u8 as u32),
            0,
        ),
        MixerCommand::SequencerSetStepVel { track, step, note, velocity } => (
            T::SequencerSetStep,
            DECK_NONE,
            *track as u8,
            ((*step as u32) << 16) | ((*note as u8 as u32) << 8) | (*velocity as u32),
            0,
        ),
        MixerCommand::AmbientWeather { rain_permyriad, wind_permyriad, fog_permyriad } => (
            T::AmbientWeather,
            DECK_NONE,
            0,
            ((*rain_permyriad as u32) << 16) | (*wind_permyriad as u32),
            *fog_permyriad as u32,
        ),
        MixerCommand::TriggerSampler { slot } => (T::TriggerSampler, DECK_NONE, *slot as u8, 0, 0),
        MixerCommand::StopSampler { slot } => (T::StopSampler, DECK_NONE, *slot as u8, 0, 0),
        MixerCommand::ToggleFx { slot } => (T::ToggleFx, DECK_NONE, *slot as u8, 0, 0),
        MixerCommand::SetFxIntensity { slot, intensity } => {
            (T::SetFxIntensity, DECK_NONE, *slot as u8, intensity.to_bits(), 0)
        }
        MixerCommand::ToggleMic => (T::ToggleMic, DECK_NONE, 0, 0, 0),
        MixerCommand::ToggleBroadcast => (T::ToggleBroadcast, DECK_NONE, 0, 0, 0),
        MixerCommand::ToggleRecording { .. } => (T::ToggleRecording, DECK_NONE, 0, 0, 0),
        MixerCommand::Shutdown => (T::Shutdown, DECK_NONE, 0, 0, 0),
        // Everything else records as an opaque command moment (tag only).
        _ => (T::Opaque, DECK_NONE, 0, 0, 0),
    };
    Ump::new([word0(tag, deck, aux), w1, w2, 0])
}

/// Stamp an encoded command with the master-bus universal tick (µs).
#[inline]
pub fn stamp(cmd: &MixerCommand, universal_tick_us: i64) -> Stamped<Ump> {
    Stamped { universal_tick_us, payload: encode_ump(cmd) }
}

/// Tape codeword for a command — `essence_id` passed to `Recorder::commit`.
#[inline]
pub fn essence_of(cmd: &MixerCommand) -> u8 {
    HubTag::from_u8(((encode_ump(cmd).words[0] >> 16) & 0xff) as u8)
        .map(HubTag::essence)
        .unwrap_or(HubTag::Opaque.essence())
}

/// Decode a HUB command atom. Returns `None` for any packet that is not on the
/// private HUB lane (wrong MT/group) or carries an unknown tag.
pub fn decode(ump: Ump) -> Option<HubEvent> {
    if ump.mt() != HUB_MT || ((ump.words[0] >> 24) & 0x0f) as u8 != HUB_GROUP {
        return None;
    }
    let tag = HubTag::from_u8(((ump.words[0] >> 16) & 0xff) as u8)?;
    Some(HubEvent {
        tag,
        deck: ((ump.words[0] >> 8) & 0xff) as u8,
        aux: (ump.words[0] & 0xff) as u8,
        value_bits: ump.words[1],
        ext_bits: ump.words[2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_lane_is_not_standard_midi_voice() {
        // A real MIDI 2.0 note-on is MT 0x4; HUB atoms must never collide.
        let u = encode_ump(&MixerCommand::SetMasterVolume { volume: 0.5 });
        assert_eq!(u.mt(), HUB_MT);
        assert_ne!(u.mt(), 0x4, "HUB lane must not masquerade as MIDI voice");
    }

    #[test]
    fn master_volume_round_trips_scalar() {
        let u = encode_ump(&MixerCommand::SetMasterVolume { volume: 0.73 });
        let e = decode(u).unwrap();
        assert_eq!(e.tag, HubTag::SetMasterVolume);
        assert_eq!(e.deck, DECK_NONE);
        assert!((e.value_f32() - 0.73).abs() < 1e-6);
    }

    #[test]
    fn deck_volume_and_pan_carry_deck_and_value() {
        let e = decode(encode_ump(&MixerCommand::SetVolume { deck: 2, volume: 0.4 })).unwrap();
        assert_eq!(e.tag, HubTag::SetVolume);
        assert_eq!(e.deck, 2);
        assert!((e.value_f32() - 0.4).abs() < 1e-6);

        let p = decode(encode_ump(&MixerCommand::SetPan { deck: 3, pan: -0.5 })).unwrap();
        assert_eq!(p.tag, HubTag::SetPan);
        assert_eq!(p.deck, 3);
        assert!((p.value_f32() + 0.5).abs() < 1e-6);
    }

    #[test]
    fn crossfader_and_seek_round_trip() {
        let x = decode(encode_ump(&MixerCommand::SetCrossfader { position: 0.9 })).unwrap();
        assert_eq!(x.tag, HubTag::SetCrossfader);
        assert!((x.value_f32() - 0.9).abs() < 1e-6);

        let s = decode(encode_ump(&MixerCommand::Seek { deck: 1, position_secs: 12.5 })).unwrap();
        assert_eq!(s.tag, HubTag::Seek);
        assert_eq!(s.deck, 1);
        assert!((s.value_f32() - 12.5).abs() < 1e-4);
    }

    #[test]
    fn sync_mode_packs_into_aux() {
        let e = decode(encode_ump(&MixerCommand::SetSync { deck: 0, mode: "leader".into() })).unwrap();
        assert_eq!(e.tag, HubTag::SetSync);
        assert_eq!(e.aux, 2);
    }

    #[test]
    fn weather_packs_three_permyriad_lanes() {
        let e = decode(encode_ump(&MixerCommand::AmbientWeather {
            rain_permyriad: 8000,
            wind_permyriad: 3000,
            fog_permyriad: 1500,
        }))
        .unwrap();
        assert_eq!(e.tag, HubTag::AmbientWeather);
        assert_eq!(e.value_bits >> 16, 8000);
        assert_eq!(e.value_bits & 0xffff, 3000);
        assert_eq!(e.ext_bits, 1500);
    }

    #[test]
    fn opaque_variants_record_tag_only() {
        let e = decode(encode_ump(&MixerCommand::ToggleBroadcast)).unwrap();
        assert_eq!(e.tag, HubTag::ToggleBroadcast);
        // Param has no scalar encoding → Opaque.
        let o = decode(encode_ump(&MixerCommand::Param { target: "x".into(), value: 1.0 })).unwrap();
        assert_eq!(o.tag, HubTag::Opaque);
    }

    #[test]
    fn essence_lands_in_codeword_range() {
        for cmd in [
            MixerCommand::SetMasterVolume { volume: 1.0 },
            MixerCommand::Shutdown,
            MixerCommand::Param { target: "x".into(), value: 0.0 },
        ] {
            assert!(essence_of(&cmd) < 64);
        }
    }

    #[test]
    fn non_hub_packet_decodes_to_none() {
        // A plain MIDI 2.0 note-on (MT 0x4) is not on the HUB lane.
        assert!(decode(Ump::new([0x4090_4000, 0x7fff_0000, 0, 0])).is_none());
    }
}
