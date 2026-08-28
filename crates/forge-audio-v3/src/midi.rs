//! MIDI event types and live device input via midir.
//!
//! Two layers:
//! - `MidiEvent` / `MidiDriver` — MIDI 1.0 hardware tap via midir → ControllerEvent
//! - `MIDI_CC_20_27_*` — canonical 8-knob CC 20-27 performance mapping for DJ/studio surfaces

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    CC { channel: u8, controller: u8, value: u8 },
}

impl MidiEvent {
    /// @forge:allow_float — MIDI CC 0-127 normalised to 0.0-1.0 at the hardware boundary.
    pub fn cc_to_f32(value: u8) -> f32 { value as f32 / 127.0 }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 2 { return None; }
        let status = data[0] & 0xF0;
        let channel = data[0] & 0x0F;
        match status {
            0x90 if data.len() >= 3 && data[2] > 0 => Some(MidiEvent::NoteOn { channel, note: data[1], velocity: data[2] }),
            0x80 | 0x90 if data.len() >= 3 => Some(MidiEvent::NoteOff { channel, note: data[1] }),
            0xB0 if data.len() >= 3 => Some(MidiEvent::CC { channel, controller: data[1], value: data[2] }),
            _ => None,
        }
    }
}

/// Thread-safe event queue filled by midir callback.
pub type MidiQueue = Arc<Mutex<Vec<MidiEvent>>>;

pub fn new_queue() -> MidiQueue { Arc::new(Mutex::new(Vec::new())) }

pub fn list_ports() -> Vec<String> {
    let Ok(midi_in) = midir::MidiInput::new("forge-list") else { return vec![] };
    midi_in.ports().iter().filter_map(|p| midi_in.port_name(p).ok()).collect()
}

/// Connect to a MIDI port by name substring. Pushes events to the queue.
/// Returns the connection (drop it to disconnect).
pub fn connect(port_name: &str, queue: MidiQueue) -> Result<midir::MidiInputConnection<()>, String> {
    let midi_in = midir::MidiInput::new("forge").map_err(|e| format!("MIDI init: {}", e))?;
    let port = midi_in.ports().into_iter()
        .find(|p| midi_in.port_name(p).map(|n| n.to_lowercase().contains(&port_name.to_lowercase())).unwrap_or(false))
        .ok_or_else(|| format!("MIDI port '{}' not found", port_name))?;

    let conn = midi_in.connect(&port, "forge-in", move |_ts, data, _| {
        if let Some(evt) = MidiEvent::from_bytes(data) {
            if let Ok(mut q) = queue.lock() { q.push(evt); }
        }
    }, ()).map_err(|e| format!("MIDI connect: {}", e))?;

    Ok(conn)
}

pub fn drain(queue: &MidiQueue) -> Vec<MidiEvent> {
    queue.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
}

/// Drain into a caller-owned buffer (pump-loop path: no per-tick Vec alloc).
pub fn drain_into(queue: &MidiQueue, out: &mut Vec<MidiEvent>) {
    if let Ok(mut q) = queue.lock() {
        out.append(&mut q);
    }
}

use crate::controller::{ControllerEvent, ControllerDriver};

/// MIDI controller driver — adapts midir events to ControllerEvent.
pub struct MidiDriver {
    queue: MidiQueue,
    _connection: Option<midir::MidiInputConnection<()>>,
}

impl MidiDriver {
    pub fn connect(port_name: &str) -> Result<Self, String> {
        let queue = new_queue();
        let conn = connect(port_name, queue.clone())?;
        Ok(Self { queue, _connection: Some(conn) })
    }

    /// Create from an existing queue (for testing).
    pub fn new_from_queue(queue: MidiQueue) -> Self {
        Self { queue, _connection: None }
    }

    fn translate(event: &MidiEvent) -> ControllerEvent {
        match event {
            MidiEvent::CC { channel, controller, value } => {
                ControllerEvent::Analog {
                    source_id: format!("midi:{}:cc:{}", channel, controller),
                    value: MidiEvent::cc_to_f32(*value),
                }
            }
            MidiEvent::NoteOn { channel, note, .. } => {
                ControllerEvent::Button {
                    source_id: format!("midi:{}:note:{}", channel, note),
                    pressed: true,
                }
            }
            MidiEvent::NoteOff { channel, note } => {
                ControllerEvent::Button {
                    source_id: format!("midi:{}:note:{}", channel, note),
                    pressed: false,
                }
            }
        }
    }
}

impl ControllerDriver for MidiDriver {
    fn name(&self) -> &str { "MIDI" }

    fn poll(&mut self) -> Vec<ControllerEvent> {
        drain(&self.queue).iter().map(Self::translate).collect()
    }

    fn connected(&self) -> bool {
        self._connection.is_some()
    }
}

// ── CC 20-27 canonical performance-knob mapping ──────────────────────────────
//
// Generic 8-knob layout: CC 20-23 drive Deck A (low/mid/high EQ + fx),
// CC 24-27 drive Deck B. Load MIDI_CC_20_27_DEFAULT_TOML via MappingEngine::from_toml.

/// Canonical CC -> (channel, cc, mixer target, min, max) for CC 20-27.
/// @forge:allow_float — dB/gain config ranges, not hot-path compute.
pub const MIDI_CC_20_27_MAP: [(u8, u8, &str, f32, f32); 8] = [
    (0, 20, "deck_a.eq_low",    -12.0, 12.0),
    (0, 21, "deck_a.eq_mid",    -12.0, 12.0),
    (0, 22, "deck_a.eq_high",   -12.0, 12.0),
    (0, 23, "deck_a.fx_amount",   0.0,  1.0),
    (0, 24, "deck_b.eq_low",    -12.0, 12.0),
    (0, 25, "deck_b.eq_mid",    -12.0, 12.0),
    (0, 26, "deck_b.eq_high",   -12.0, 12.0),
    (0, 27, "deck_b.fx_amount",   0.0,  1.0),
];

/// Default TOML for the MappingEngine `[[bind]]` schema, covering CC 20-27.
pub const MIDI_CC_20_27_DEFAULT_TOML: &str = r#"device = "Generic CC 20-27"

[[bind]]
source = "midi:0:cc:20"
target = "deck_a.eq_low"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:21"
target = "deck_a.eq_mid"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:22"
target = "deck_a.eq_high"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:23"
target = "deck_a.fx_amount"
min = 0.0
max = 1.0

[[bind]]
source = "midi:0:cc:24"
target = "deck_b.eq_low"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:25"
target = "deck_b.eq_mid"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:26"
target = "deck_b.eq_high"
min = -12.0
max = 12.0

[[bind]]
source = "midi:0:cc:27"
target = "deck_b.fx_amount"
min = 0.0
max = 1.0
"#;

/// Format a MIDI CC source id matching `midi:{channel}:cc:{controller}`.
pub fn cc_source_id(channel: u8, controller: u8) -> String {
    format!("midi:{}:cc:{}", channel, controller)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cc() {
        let evt = MidiEvent::from_bytes(&[0xB0, 1, 64]).unwrap();
        assert_eq!(evt, MidiEvent::CC { channel: 0, controller: 1, value: 64 });
    }

    #[test]
    fn parse_note_on() {
        let evt = MidiEvent::from_bytes(&[0x90, 60, 100]).unwrap();
        assert_eq!(evt, MidiEvent::NoteOn { channel: 0, note: 60, velocity: 100 });
    }

    #[test]
    fn cc_to_f32_range() {
        assert_eq!(MidiEvent::cc_to_f32(0), 0.0);
        assert!((MidiEvent::cc_to_f32(127) - 1.0).abs() < 0.001);
    }

    #[test]
    fn midi_driver_cc_to_controller_event() {
        let queue = new_queue();
        queue.lock().unwrap().push(MidiEvent::CC { channel: 0, controller: 4, value: 64 });

        let mut driver = MidiDriver::new_from_queue(queue);
        let events = driver.poll();

        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Analog { source_id, value } => {
                assert_eq!(source_id, "midi:0:cc:4");
                assert!((value - 0.5039).abs() < 0.01);
            }
            other => panic!("Expected Analog, got {:?}", other),
        }
    }

    #[test]
    fn midi_driver_note_on_to_button() {
        let queue = new_queue();
        queue.lock().unwrap().push(MidiEvent::NoteOn { channel: 1, note: 42, velocity: 100 });

        let mut driver = MidiDriver::new_from_queue(queue);
        let events = driver.poll();

        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "midi:1:note:42");
                assert!(*pressed);
            }
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn midi_driver_note_off_to_button_released() {
        let queue = new_queue();
        queue.lock().unwrap().push(MidiEvent::NoteOff { channel: 0, note: 60 });

        let mut driver = MidiDriver::new_from_queue(queue);
        let events = driver.poll();

        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "midi:0:note:60");
                assert!(!*pressed);
            }
            other => panic!("Expected Button(released), got {:?}", other),
        }
    }

    #[test]
    fn midi_driver_name() {
        let queue = new_queue();
        let driver = MidiDriver::new_from_queue(queue);
        assert_eq!(driver.name(), "MIDI");
    }

    #[test]
    fn cc_source_id_matches_driver_format() {
        let queue = new_queue();
        queue.lock().unwrap().push(MidiEvent::CC { channel: 3, controller: 22, value: 64 });
        let mut driver = MidiDriver::new_from_queue(queue);
        let events = driver.poll();
        match &events[0] {
            ControllerEvent::Analog { source_id, .. } => {
                assert_eq!(source_id, &cc_source_id(3, 22));
            }
            other => panic!("Expected Analog, got {:?}", other),
        }
    }

    #[test]
    fn cc_20_27_map_covers_each_cc_once() {
        let mut seen = [false; 8];
        for (_ch, cc, target, min, max) in MIDI_CC_20_27_MAP.iter() {
            assert!((20..=27).contains(cc), "CC {} outside 20-27 range", cc);
            let idx = (cc - 20) as usize;
            assert!(!seen[idx], "duplicate CC {}", cc);
            seen[idx] = true;
            assert!(!target.is_empty(), "empty target for CC {}", cc);
            assert!(max > min, "CC {}: max must exceed min", cc);
        }
        assert!(seen.iter().all(|&s| s), "CC 20-27 must all be covered");
    }

    #[test]
    fn cc_20_27_default_toml_parses() {
        use crate::mapping::MappingEngine;
        let engine = MappingEngine::from_toml(MIDI_CC_20_27_DEFAULT_TOML)
            .expect("default CC 20-27 TOML must parse");
        assert_eq!(engine.binds.len(), 8, "expect 8 binds for CC 20-27");
    }

    #[test]
    fn cc_20_27_toml_and_table_agree() {
        use crate::mapping::MappingEngine;
        let engine = MappingEngine::from_toml(MIDI_CC_20_27_DEFAULT_TOML).unwrap();
        for (ch, cc, target, min, max) in MIDI_CC_20_27_MAP.iter() {
            let src = cc_source_id(*ch, *cc);
            let bind = engine.binds.iter().find(|b| b.source == src)
                .unwrap_or_else(|| panic!("no bind for {}", src));
            assert_eq!(&bind.target, target, "target mismatch for {}", src);
            assert!((bind.min - min).abs() < 1e-6, "min mismatch for {}", src);
            assert!((bind.max - max).abs() < 1e-6, "max mismatch for {}", src);
        }
    }

    #[test]
    fn cc_22_midpoint_yields_zero_db() {
        use crate::controller::ControllerEvent;
        use crate::mapping::MappingEngine;
        let mut engine = MappingEngine::from_toml(MIDI_CC_20_27_DEFAULT_TOML).unwrap();
        let evt = ControllerEvent::Analog {
            source_id: "midi:0:cc:22".to_string(),
            value: 0.5,
        };
        let (params, _) = engine.apply(&evt);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].target, "deck_a.eq_high");
        assert!(params[0].value.abs() < 0.2, "expected ~0.0 dB, got {}", params[0].value);
    }

    #[test]
    fn cc_27_full_yields_unity_fx() {
        use crate::controller::ControllerEvent;
        use crate::mapping::MappingEngine;
        let mut engine = MappingEngine::from_toml(MIDI_CC_20_27_DEFAULT_TOML).unwrap();
        let evt = ControllerEvent::Analog {
            source_id: "midi:0:cc:27".to_string(),
            value: 1.0,
        };
        let (params, _) = engine.apply(&evt);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].target, "deck_b.fx_amount");
        assert!((params[0].value - 1.0).abs() < 0.01);
    }
}
