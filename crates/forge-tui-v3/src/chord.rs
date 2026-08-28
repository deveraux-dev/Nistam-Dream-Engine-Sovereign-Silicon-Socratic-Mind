//! Sticky-key latching and macro chording over `KeyAction` — motor-assist
//! input. Tick-based throughout (caller-supplied integer tick, never a wall
//! clock) so replay stays deterministic.

use crate::event::KeyAction;

/// Latch-and-clear modifier state. A motor-assist operator taps a modifier
/// once instead of holding it through a chord — `toggle_*` latches it,
/// `consume` reads and clears all three in one shot so the next real key
/// picks them up exactly once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickyModifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl StickyModifiers {
    /// Latch or unlatch Ctrl.
    pub fn toggle_ctrl(&mut self) {
        self.ctrl = !self.ctrl;
    }

    /// Latch or unlatch Shift.
    pub fn toggle_shift(&mut self) {
        self.shift = !self.shift;
    }

    /// Latch or unlatch Alt.
    pub fn toggle_alt(&mut self) {
        self.alt = !self.alt;
    }

    /// Read the latched `(ctrl, shift, alt)` state and clear it — one-shot,
    /// applies to exactly the next real keystroke.
    pub fn consume(&mut self) -> (bool, bool, bool) {
        let out = (self.ctrl, self.shift, self.alt);
        *self = Self::default();
        out
    }
}

/// A declarative chord binding: tapping `sequence` in order emits `output`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChordMacro {
    /// The exact tap order that must be matched, in full, to fire.
    pub sequence: Vec<KeyAction>,
    /// The action emitted once `sequence` completes.
    pub output: KeyAction,
}

impl ChordMacro {
    /// Build a macro from a sequence of taps and the action it resolves to.
    pub fn new(sequence: Vec<KeyAction>, output: KeyAction) -> Self {
        Self { sequence, output }
    }
}

/// Feeds `KeyAction`s through registered `ChordMacro`s. A single-switch or
/// motor-assist operator taps a sequence instead of holding simultaneous
/// keys; a debounce floor drops an accidental double-strike of the same key.
pub struct ChordEngine {
    macros: Vec<ChordMacro>,
    buffer: Vec<KeyAction>,
    last_tick: u32,
    /// Ticks a partial chord may sit idle before it resets.
    window_ticks: u32,
    /// Ticks below which a repeated identical key is treated as bounce, not intent.
    debounce_ticks: u32,
    last_key: Option<(KeyAction, u32)>,
}

impl ChordEngine {
    /// Create an engine with the given macros, chord timeout window, and
    /// debounce floor (all in ticks — the caller owns the clock).
    pub fn new(macros: Vec<ChordMacro>, window_ticks: u32, debounce_ticks: u32) -> Self {
        Self {
            macros,
            buffer: Vec::new(),
            last_tick: 0,
            window_ticks,
            debounce_ticks,
            last_key: None,
        }
    }

    /// Feed one key at `tick`. Returns `Some(output)` the instant a
    /// registered sequence completes; `None` while buffering, debouncing, or
    /// after a timeout/mismatch reset.
    pub fn feed(&mut self, key: KeyAction, tick: u32) -> Option<KeyAction> {
        if let Some((last, last_tick)) = self.last_key {
            if last == key && tick.saturating_sub(last_tick) < self.debounce_ticks {
                self.last_key = Some((key, tick));
                return None;
            }
        }
        self.last_key = Some((key, tick));

        if !self.buffer.is_empty() && tick.saturating_sub(self.last_tick) > self.window_ticks {
            self.buffer.clear();
        }
        self.last_tick = tick;
        self.buffer.push(key);

        let mut any_prefix = false;
        for m in &self.macros {
            if m.sequence.len() < self.buffer.len() {
                continue;
            }
            if m.sequence[..self.buffer.len()] == self.buffer[..] {
                any_prefix = true;
                if m.sequence.len() == self.buffer.len() {
                    self.buffer.clear();
                    return Some(m.output);
                }
            }
        }
        if !any_prefix {
            self.buffer.clear();
            self.buffer.push(key);
            // The single key itself might still be a length-1 prefix or full match.
            for m in &self.macros {
                if m.sequence == self.buffer {
                    self.buffer.clear();
                    return Some(m.output);
                }
                if !m.sequence.is_empty() && m.sequence[0] == key {
                    any_prefix = true;
                }
            }
            if !any_prefix {
                self.buffer.clear();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticky_modifier_consume_is_one_shot() {
        let mut s = StickyModifiers::default();
        s.toggle_ctrl();
        s.toggle_shift();
        assert_eq!(s.consume(), (true, true, false));
        assert_eq!(s.consume(), (false, false, false), "consume must clear the latch");
    }

    #[test]
    fn toggle_twice_unlatches() {
        let mut s = StickyModifiers::default();
        s.toggle_alt();
        s.toggle_alt();
        assert_eq!(s.consume(), (false, false, false));
    }

    #[test]
    fn chord_sequence_emits_output_on_full_match() {
        let macros = vec![ChordMacro::new(
            vec![KeyAction::Char('j'), KeyAction::Char('j')],
            KeyAction::Escape,
        )];
        let mut engine = ChordEngine::new(macros, 100, 0);
        assert_eq!(engine.feed(KeyAction::Char('j'), 0), None);
        assert_eq!(engine.feed(KeyAction::Char('j'), 1), Some(KeyAction::Escape));
    }

    #[test]
    fn partial_sequence_times_out_without_firing() {
        let macros = vec![ChordMacro::new(
            vec![KeyAction::Char('j'), KeyAction::Char('j')],
            KeyAction::Escape,
        )];
        let mut engine = ChordEngine::new(macros, 5, 0);
        assert_eq!(engine.feed(KeyAction::Char('j'), 0), None);
        // Second 'j' arrives long after the window — must not fire the chord.
        assert_eq!(engine.feed(KeyAction::Char('j'), 100), None);
    }

    #[test]
    fn debounce_drops_a_rapid_repeat() {
        let macros = vec![ChordMacro::new(vec![KeyAction::Char('x')], KeyAction::Enter)];
        let mut engine = ChordEngine::new(macros, 100, 10);
        assert_eq!(engine.feed(KeyAction::Char('x'), 0), Some(KeyAction::Enter));
        // Rapid repeat within the debounce floor is dropped, not re-fired.
        assert_eq!(engine.feed(KeyAction::Char('x'), 1), None);
    }

    #[test]
    fn unrelated_key_resets_the_buffer() {
        let macros = vec![ChordMacro::new(
            vec![KeyAction::Char('j'), KeyAction::Char('j')],
            KeyAction::Escape,
        )];
        let mut engine = ChordEngine::new(macros, 100, 0);
        assert_eq!(engine.feed(KeyAction::Char('j'), 0), None);
        assert_eq!(engine.feed(KeyAction::Char('k'), 20), None);
        assert_eq!(engine.feed(KeyAction::Char('j'), 40), None, "buffer must have reset, not resumed");
    }
}
