//! Unified controller abstraction — all input devices emit ControllerEvent.

/// A unified input event from any controller (MIDI, HID, gamepad).
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    /// Knobs, faders — absolute position (0.0–1.0).
    /// @forge:allow_float — hardware protocol boundary: MIDI CC 0-127 normalised to 0.0-1.0.
    Analog { source_id: String, value: f32 },
    /// Buttons, pads — press/release.
    Button { source_id: String, pressed: bool },
    /// Jog wheels, encoders — relative movement (signed delta).
    /// @forge:allow_float — hardware protocol boundary: signed encoder tick delta.
    Relative { source_id: String, delta: f32 },
}

/// Resolved parameter change from a mapping bind.
/// @forge:allow_float — mapping target values are config params (dB, gain ratios), not hot-path compute.
#[derive(Debug, Clone)]
pub struct ParamChange {
    pub target: String,
    pub value: f32, // @forge:allow_float
}

/// Resolved action trigger from a mapping bind.
#[derive(Debug, Clone)]
pub struct ActionTrigger {
    pub target: String,
}

/// Trait for any input device driver.
pub trait ControllerDriver {
    /// Human-readable name for this driver.
    fn name(&self) -> &str;

    /// Poll for new events. Non-blocking.
    fn poll(&mut self) -> Vec<ControllerEvent>;

    /// Is the device still connected?
    fn connected(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_event_analog() {
        let evt = ControllerEvent::Analog {
            source_id: "midi:0:cc:4".to_string(),
            value: 0.5,
        };
        assert!(matches!(evt, ControllerEvent::Analog { .. }));
    }

    #[test]
    fn controller_event_button() {
        let evt = ControllerEvent::Button {
            source_id: "midi:0:note:42".to_string(),
            pressed: true,
        };
        assert!(matches!(evt, ControllerEvent::Button { pressed: true, .. }));
    }

    #[test]
    fn controller_event_relative() {
        let evt = ControllerEvent::Relative {
            source_id: "s2:a:jog".to_string(),
            delta: -0.5,
        };
        if let ControllerEvent::Relative { delta, .. } = evt {
            assert!((delta - (-0.5)).abs() < 0.001);
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn param_change_fields() {
        let pc = ParamChange { target: "deck_a.eq_high".to_string(), value: 6.0 };
        assert_eq!(pc.target, "deck_a.eq_high");
        assert_eq!(pc.value, 6.0);
    }

    #[test]
    fn action_trigger_fields() {
        let at = ActionTrigger { target: "deck_a.play_pause".to_string() };
        assert_eq!(at.target, "deck_a.play_pause");
    }
}
