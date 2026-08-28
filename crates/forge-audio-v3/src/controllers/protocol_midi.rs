//! MIDI LED writer — ported from dreadpirateradio/src/controllers/protocol_midi.rs (2026-06-29).
//! SF-012: MidiWriter on HID descriptor produces zero messages.

use super::descriptor::ControllerDescriptor;
use super::led_state::{LedState, LedRole, Deck};

pub struct MidiWriter {
    pub messages: Vec<[u8; 3]>,
}

impl Default for MidiWriter {
    fn default() -> Self { Self::new() }
}

impl MidiWriter {
    pub fn new() -> Self { Self { messages: Vec::new() } }

    pub fn apply(&mut self, states: &[LedState], descriptor: &ControllerDescriptor, beat_phase: f32) {
        self.messages.clear();
        // SF-012: LOCKED — non-MIDI protocol is a caller error; return empty.
        let channel = match &descriptor.protocol {
            super::descriptor::Protocol::Midi { channel } => *channel,
            _ => {
                eprintln!("[midi_writer] SF-012: MidiWriter::apply called with non-MIDI protocol — no output");
                return;
            }
        };
        for state in states {
            let key = led_key(&state.role, &state.deck);
            if let Some(mapping) = descriptor.leds.get(&key) {
                let cc = mapping.address as u8;
                let val = state.to_u8(beat_phase) >> 1;
                self.messages.push([0xB0 | (channel & 0x0F), cc, val]);
            }
        }
    }
}

fn led_key(role: &LedRole, deck: &Option<Deck>) -> String {
    let deck_str = match deck {
        Some(Deck::A) => "_a", Some(Deck::B) => "_b",
        Some(Deck::C) => "_c", Some(Deck::D) => "_d", None => "",
    };
    let role_str = match role {
        LedRole::Play => "play".to_string(), LedRole::Cue => "cue".to_string(),
        LedRole::Sync => "sync".to_string(), LedRole::Loop => "loop".to_string(),
        LedRole::Pad(n) => format!("pad_{}", n), LedRole::Vu(n) => format!("vu_{}", n),
        LedRole::MasterLevel => "master_level".to_string(),
        LedRole::DeckFocus => "deck_focus".to_string(),
        LedRole::Fx(n) => format!("fx_{}", n),
        LedRole::Shift => "shift".to_string(), LedRole::Browse => "browse".to_string(),
    };
    format!("{}{}", role_str, deck_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::descriptor::{ControllerDescriptor, Protocol, Capabilities};
    use std::collections::HashMap;

    fn load_s2_mk3_hid_descriptor() -> ControllerDescriptor {
        let toml_path = concat!(env!("CARGO_MANIFEST_DIR"), "/controllers/s2_mk3.toml");
        ControllerDescriptor::from_file(toml_path)
            .unwrap_or_else(|e| panic!("S2 MK3 TOML failed to parse: {e}"))
    }

    // 25.6 — MidiWriter on HID descriptor produces zero messages (SF-012)
    #[test]
    fn sf_012_midi_writer_on_hid_descriptor_produces_zero_messages() {
        let desc = load_s2_mk3_hid_descriptor();
        let states = vec![
            LedState::standby(LedRole::Play, Some(Deck::A)),
            LedState::standby(LedRole::Cue, Some(Deck::A)),
        ];
        let mut writer = MidiWriter::new();
        writer.apply(&states, &desc, 0.0);
        assert_eq!(writer.messages.len(), 0, "SF-012: MidiWriter on HID descriptor must produce zero messages");
    }

    #[test]
    fn midi_writer_on_midi_descriptor_emits_messages() {
        let desc = ControllerDescriptor {
            name: "Test MIDI".to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
            protocol: Protocol::Midi { channel: 0 },
            capabilities: Capabilities { decks: 2, pads_per_deck: 8, has_rgb_pads: false, has_rgb_buttons: false },
            leds: {
                let mut m = HashMap::new();
                m.insert("play_a".to_string(), crate::controllers::descriptor::LedMapping {
                    role: LedRole::Play,
                    deck: Some(Deck::A),
                    led_type: crate::controllers::descriptor::LedType::Brightness,
                    address: 10,
                    rgb_offsets: None,
                });
                m
            },
        };
        let states = vec![LedState { role: LedRole::Play, deck: Some(Deck::A), color: [0.0, 1.0, 0.2], brightness: 0.8, pulse_depth: 0.0 }];
        let mut writer = MidiWriter::new();
        writer.apply(&states, &desc, 0.0);
        assert_eq!(writer.messages.len(), 1);
        assert_eq!(writer.messages[0][0], 0xB0);
    }
}
