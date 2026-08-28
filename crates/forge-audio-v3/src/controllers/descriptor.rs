//! Controller descriptor — TOML-driven controller definition.
//! Ported from dreadpirateradio/src/controllers/descriptor.rs (2026-06-29).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::led_state::{LedRole, Deck as LedDeck};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ControllerDescriptor {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub protocol: Protocol,
    pub capabilities: Capabilities,
    pub leds: HashMap<String, LedMapping>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Protocol {
    Hid { frame_size: usize },
    Midi { channel: u8 },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Capabilities {
    pub decks: u8,
    pub pads_per_deck: u8,
    pub has_rgb_pads: bool,
    pub has_rgb_buttons: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedMapping {
    // SF-011: LOCKED — role and deck are typed enums; unknown values fail loudly
    // at TOML load time via LedRole::from_str / Deck::deserialize.
    pub role: LedRole,
    pub deck: Option<LedDeck>,
    pub led_type: LedType,
    /// HID: byte offset in frame. MIDI: CC number.
    pub address: usize,
    /// For RGB LEDs: [r_offset, g_offset, b_offset]
    pub rgb_offsets: Option<[usize; 3]>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LedType {
    Rgb,
    Brightness,
}

impl ControllerDescriptor {
    pub fn from_toml(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_toml(&src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_s2_mk3_toml() -> ControllerDescriptor {
        let toml_path = concat!(env!("CARGO_MANIFEST_DIR"), "/controllers/s2_mk3.toml");
        ControllerDescriptor::from_file(toml_path)
            .unwrap_or_else(|e| panic!("S2 MK3 TOML failed to parse: {e}"))
    }

    #[test]
    fn s2_mk3_toml_parses_successfully() {
        let desc = load_s2_mk3_toml();
        assert_eq!(desc.name, "Traktor Kontrol S2 MK3");
        assert_eq!(desc.vendor_id, 0x17CC);
    }

    #[test]
    fn s2_mk3_has_hid_protocol() {
        let desc = load_s2_mk3_toml();
        assert!(matches!(desc.protocol, Protocol::Hid { .. }));
    }

    #[test]
    fn s2_mk3_has_expected_capabilities() {
        let desc = load_s2_mk3_toml();
        assert_eq!(desc.capabilities.decks, 2);
    }

    #[test]
    fn protocol_hid_and_midi_round_trip() {
        let hid: Protocol = toml::from_str("type = \"hid\"\nframe_size = 62").unwrap();
        assert!(matches!(hid, Protocol::Hid { frame_size: 62 }));
        let midi: Protocol = toml::from_str("type = \"midi\"\nchannel = 1").unwrap();
        assert!(matches!(midi, Protocol::Midi { channel: 1 }));
    }

    #[test]
    fn from_toml_unknown_protocol_fails() {
        let src = "name = \"X\"\nvendor_id = 1\nproduct_id = 2\n[protocol]\ntype = \"usb\"\n[capabilities]\ndecks = 2\npads_per_deck = 8\nhas_rgb_pads = false\nhas_rgb_buttons = false\n";
        assert!(ControllerDescriptor::from_toml(src).is_err());
    }

    #[test]
    fn led_types_deserialize() {
        #[derive(serde::Deserialize)]
        struct W { t: LedType }
        let rgb: W = toml::from_str("t = \"rgb\"").unwrap();
        assert_eq!(rgb.t, LedType::Rgb);
        let bri: W = toml::from_str("t = \"brightness\"").unwrap();
        assert_eq!(bri.t, LedType::Brightness);
    }
}
