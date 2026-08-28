//! TuiDriver — routes forge-tui keyboard char events into the ControllerDriver pipeline.
//!
//! The event loop (forge-gui) pushes `TuiKeyEvent`s from `InputState::drain_keys()`
//! into a `TuiKeyQueue`; `TuiDriver::poll()` drains it and emits `ControllerEvent::Button`.
//!
//! No dep on forge-tui here — the conversion from `KeyAction` to `TuiKeyEvent` happens
//! at the event-loop boundary.

use std::sync::{Arc, Mutex};
use crate::controller::{ControllerEvent, ControllerDriver};

/// A single key press or release from the TUI event loop.
pub struct TuiKeyEvent {
    pub ch: char,
    /// `true` = key down, `false` = key up.
    ///
    /// NOTE: `forge-tui::InputState` currently only emits key-down events.
    /// Key-up support requires extending `InputState` to also capture
    /// `ElementState::Released` — at that point `pressed: false` will fire here.
    pub pressed: bool,
}

/// Thread-safe key queue — filled by the event loop, drained by `TuiDriver::poll()`.
pub type TuiKeyQueue = Arc<Mutex<Vec<TuiKeyEvent>>>;

pub fn new_tui_queue() -> TuiKeyQueue {
    Arc::new(Mutex::new(Vec::new()))
}

/// Push one key event into the queue (call from the event loop per frame).
pub fn push_key(queue: &TuiKeyQueue, ch: char, pressed: bool) {
    if let Ok(mut q) = queue.lock() {
        q.push(TuiKeyEvent { ch, pressed });
    }
}

/// ControllerDriver that wraps a `TuiKeyQueue`.
///
/// Source IDs use the format `"key:{char}"` — e.g. `"key:a"`, `"key:;"`.
/// These match the `source` fields in `SYNTH_KEYBOARD_DEFAULT_TOML`.
pub struct TuiDriver {
    queue: TuiKeyQueue,
    connected: bool,
}

impl TuiDriver {
    pub fn new(queue: TuiKeyQueue) -> Self {
        Self { queue, connected: true }
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }
}

impl ControllerDriver for TuiDriver {
    fn name(&self) -> &str { "TUI Keyboard" }

    fn poll(&mut self) -> Vec<ControllerEvent> {
        let events: Vec<TuiKeyEvent> = self.queue.lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default();

        events.into_iter().map(|evt| {
            ControllerEvent::Button {
                source_id: format!("key:{}", evt.ch),
                pressed: evt.pressed,
            }
        }).collect()
    }

    fn connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_press_emits_button_pressed() {
        let q = new_tui_queue();
        push_key(&q, 'a', true);
        let mut driver = TuiDriver::new(q);
        let events = driver.poll();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "key:a");
                assert!(*pressed);
            }
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn key_release_emits_button_released() {
        let q = new_tui_queue();
        push_key(&q, 'a', false);
        let mut driver = TuiDriver::new(q);
        let events = driver.poll();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, pressed } => {
                assert_eq!(source_id, "key:a");
                assert!(!*pressed);
            }
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn semicolon_routes_correctly() {
        let q = new_tui_queue();
        push_key(&q, ';', true);
        let mut driver = TuiDriver::new(q);
        let events = driver.poll();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, .. } => assert_eq!(source_id, "key:;"),
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn poll_drains_queue() {
        let q = new_tui_queue();
        push_key(&q, 'a', true);
        push_key(&q, 's', true);
        let mut driver = TuiDriver::new(q);
        assert_eq!(driver.poll().len(), 2);
        assert_eq!(driver.poll().len(), 0);
    }

    #[test]
    fn unrecognised_chars_still_route() {
        // The driver does not filter — MappingEngine handles routing misses.
        let q = new_tui_queue();
        push_key(&q, 'z', true);
        let mut driver = TuiDriver::new(q);
        let events = driver.poll();
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControllerEvent::Button { source_id, .. } => assert_eq!(source_id, "key:z"),
            other => panic!("Expected Button, got {:?}", other),
        }
    }

    #[test]
    fn driver_name() {
        let driver = TuiDriver::new(new_tui_queue());
        assert_eq!(driver.name(), "TUI Keyboard");
    }

    #[test]
    fn connected_and_disconnect() {
        let mut driver = TuiDriver::new(new_tui_queue());
        assert!(driver.connected());
        driver.disconnect();
        assert!(!driver.connected());
    }
}
