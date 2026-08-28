//! HID LED frame writer — ported from dreadpirateradio/src/controllers/protocol_hid.rs (2026-06-29).
//! Law 1: Frame initialized with STANDBY_BRIGHTNESS (0x12) — LEDs never fully dark.

use super::descriptor::{ControllerDescriptor, LedType};
use super::led_state::{LedState, LedRole, Deck, FunctionalColor};

pub struct HidWriter {
    pub frame: Vec<u8>,
    pub frame_size: usize,
}

impl HidWriter {
    pub fn new(frame_size: usize) -> Self {
        Self {
            frame: vec![0u8; frame_size],
            frame_size,
        }
    }

    /// Apply LED states to frame using descriptor mapping.
    /// Law 1: resets frame to STANDBY_BRIGHTNESS before applying states.
    pub fn apply(&mut self, states: &[LedState], descriptor: &ControllerDescriptor, beat_phase: f32) {
        for b in &mut self.frame {
            if *b == 0 { *b = FunctionalColor::STANDBY_BRIGHTNESS; }
        }

        for state in states {
            let key = led_key(&state.role, &state.deck);
            if let Some(mapping) = descriptor.leds.get(&key) {
                match mapping.led_type {
                    LedType::Brightness => {
                        if mapping.address < self.frame.len() {
                            self.frame[mapping.address] = state.to_u8(beat_phase);
                        }
                    }
                    LedType::Rgb => {
                        if let Some(offsets) = mapping.rgb_offsets {
                            let rgb = state.to_rgb_u8(beat_phase);
                            for (i, &off) in offsets.iter().enumerate() {
                                if off < self.frame.len() {
                                    self.frame[off] = rgb[i];
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn frame(&self) -> &[u8] { &self.frame }
}

fn led_key(role: &LedRole, deck: &Option<Deck>) -> String {
    let deck_str = match deck {
        Some(Deck::A) => "_a",
        Some(Deck::B) => "_b",
        Some(Deck::C) => "_c",
        Some(Deck::D) => "_d",
        None => "",
    };
    let role_str = match role {
        LedRole::Play => "play".to_string(),
        LedRole::Cue => "cue".to_string(),
        LedRole::Sync => "sync".to_string(),
        LedRole::Loop => "loop".to_string(),
        LedRole::Pad(n) => format!("pad_{}", n),
        LedRole::Vu(n) => format!("vu_{}", n),
        LedRole::MasterLevel => "master_level".to_string(),
        LedRole::DeckFocus => "deck_focus".to_string(),
        LedRole::Fx(n) => format!("fx_{}", n),
        LedRole::Shift => "shift".to_string(),
        LedRole::Browse => "browse".to_string(),
    };
    format!("{}{}", role_str, deck_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::descriptor::{ControllerDescriptor, Protocol, Capabilities};
    use std::collections::HashMap;

    fn midi_descriptor() -> ControllerDescriptor {
        ControllerDescriptor {
            name: "Test MIDI Controller".to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
            protocol: Protocol::Midi { channel: 0 },
            capabilities: Capabilities { decks: 2, pads_per_deck: 8, has_rgb_pads: false, has_rgb_buttons: false },
            leds: HashMap::new(),
        }
    }

    // 25.7 — HidWriter on MIDI descriptor: standby frame, no crash
    #[test]
    fn hid_writer_on_midi_descriptor_produces_standby_frame() {
        let desc = midi_descriptor();
        let frame_size = 62;
        let mut writer = HidWriter::new(frame_size);
        let states = vec![
            LedState::standby(LedRole::Play, Some(Deck::A)),
            LedState::standby(LedRole::Cue, Some(Deck::B)),
        ];
        writer.apply(&states, &desc, 0.0);
        assert_eq!(writer.frame.len(), frame_size);
        for &byte in &writer.frame {
            assert_eq!(byte, FunctionalColor::STANDBY_BRIGHTNESS,
                "all bytes should be standby brightness on MIDI descriptor");
        }
    }
}
