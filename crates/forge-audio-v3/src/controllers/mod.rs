//! Controller abstraction — TOML-driven multi-controller support.
//! Ported from dreadpirateradio/src/controllers/ (2026-06-29).
//! HID + MIDI LED routing from MixerSnapshot → physical controller LEDs.

pub mod descriptor;
pub mod led_state;
pub mod protocol_hid;
pub mod protocol_midi;

pub use descriptor::{ControllerDescriptor, Protocol, Capabilities, LedMapping, LedType};
pub use led_state::{LedRole, Deck, LedState, FunctionalColor, UniversalLedEngine};
pub use protocol_hid::HidWriter;
pub use protocol_midi::MidiWriter;

use crate::snapshot::MixerSnapshot;

pub struct ControllerManager {
    descriptors: Vec<ControllerDescriptor>,
    active: Option<usize>,
    hid_writer: Option<HidWriter>,
    midi_writer: MidiWriter,
}

impl Default for ControllerManager {
    fn default() -> Self { Self::new() }
}

impl ControllerManager {
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            active: None,
            hid_writer: None,
            midi_writer: MidiWriter::new(),
        }
    }

    pub fn load_descriptors(&mut self, dir: &str) -> Result<usize, String> {
        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                match ControllerDescriptor::from_file(
                    path.to_str().ok_or_else(|| "invalid path".to_string())?
                ) {
                    Ok(desc) => {
                        eprintln!("[controllers] Loaded: {} (vid={:#06x} pid={:#06x})",
                            desc.name, desc.vendor_id, desc.product_id);
                        self.descriptors.push(desc);
                        count += 1;
                    }
                    Err(e) => eprintln!("[controllers] Failed to load {:?}: {}", path, e),
                }
            }
        }
        Ok(count)
    }

    pub fn match_device(&mut self, vendor_id: u16, product_id: u16) -> Option<usize> {
        let idx = self.descriptors.iter().position(|d| {
            d.vendor_id == vendor_id && d.product_id == product_id
        });
        if let Some(i) = idx {
            self.active = Some(i);
            match &self.descriptors[i].protocol {
                Protocol::Hid { frame_size } => {
                    self.hid_writer = Some(HidWriter::new(*frame_size));
                }
                Protocol::Midi { .. } => {
                    self.hid_writer = None;
                }
            }
            eprintln!("[controllers] Matched: {}", self.descriptors[i].name);
        }
        idx
    }

    pub fn update_leds(&mut self, snap: &MixerSnapshot, beat_phase: f32) -> LedOutput {
        let active_idx = match self.active {
            Some(i) => i,
            None => return LedOutput::None,
        };
        let descriptor = &self.descriptors[active_idx];
        let states = UniversalLedEngine::compute(snap);

        match &descriptor.protocol {
            Protocol::Hid { .. } => {
                if let Some(ref mut writer) = self.hid_writer {
                    writer.apply(&states, descriptor, beat_phase);
                    LedOutput::Hid(writer.frame().to_vec())
                } else {
                    LedOutput::None
                }
            }
            Protocol::Midi { .. } => {
                self.midi_writer.apply(&states, descriptor, beat_phase);
                LedOutput::Midi(self.midi_writer.messages.clone())
            }
        }
    }

    pub fn active_descriptor(&self) -> Option<&ControllerDescriptor> {
        self.active.map(|i| &self.descriptors[i])
    }

    pub fn descriptors(&self) -> &[ControllerDescriptor] {
        &self.descriptors
    }
}

#[derive(Debug, Clone)]
pub enum LedOutput {
    None,
    Hid(Vec<u8>),
    Midi(Vec<[u8; 3]>),
}
