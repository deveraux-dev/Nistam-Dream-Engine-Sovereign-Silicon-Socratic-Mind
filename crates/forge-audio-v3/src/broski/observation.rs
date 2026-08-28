//! Broski Observation Engine — ephemeral buffer, no persistence.

use crate::broski::types::{DjMode, DeckId, EqBand};

#[derive(Debug, Clone)]
pub enum SkipReason { Replaced, Rejected, AutoSkip }

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionStyle { BassSwap, FullFade, HardCut, FilterSweep }

#[derive(Debug, Clone)]
pub enum DjEvent {
    TrackLoaded { deck: DeckId, path: String, bpm: f64, key: String, genre: u8 },
    TrackPlayed { deck: DeckId, path: String, timestamp_secs: f64 },
    TrackSkipped { deck: DeckId, path: String, reason: SkipReason },
    TransitionStarted { from_deck: DeckId, to_deck: DeckId, style: TransitionStyle },
    TransitionCompleted { duration_bars: u32 },
    EqAdjusted { deck: DeckId, band: EqBand, from: i32, to: i32 },
    FaderMoved { deck: DeckId, from: i32, to: i32 },
    CrossfaderMoved { from: i32, to: i32 },
    FxEngaged { deck: DeckId, fx_name: String },
    FxDisengaged { deck: DeckId, fx_name: String },
    EnergyPeak { master_rms: f64, timestamp_secs: f64 },
    ModeChanged { from: DjMode, to: DjMode },
    VoiceCommand { text: String, confidence: f32, timestamp_secs: f64 },
}

// ── Domain event types for the other 6 experts ────────────────────────────

#[derive(Debug, Clone)]
pub struct RenderEvent  { pub kind: String }
#[derive(Debug, Clone)]
pub struct PhysicsEvent { pub kind: String }
#[derive(Debug, Clone)]
pub struct SieveEvent   { pub kind: String }
#[derive(Debug, Clone)]
pub struct LoreEvent    { pub kind: String }
#[derive(Debug, Clone)]
pub struct WorldEvent   { pub kind: String }
#[derive(Debug, Clone)]
pub struct SystemEvent  { pub kind: String }

/// Typed event envelope. Variant tag is the routing hint. Discriminants
/// 0..=6 are canonical and match `forge_intent_v3::RouteExpert` and
/// dispatch routing.
#[derive(Debug, Clone)]
pub enum NdeEvent {
    Dj(DjEvent),          // 0: Sound
    Render(RenderEvent),  // 1: Visual
    Physics(PhysicsEvent),// 2: Physics
    Sieve(SieveEvent),    // 3: Sieve
    Lore(LoreEvent),      // 4: Lorekeeper
    World(WorldEvent),    // 5: World
    System(SystemEvent),  // 6: HumanInterface
}


/// Fixed-size observation buffer with circular write semantics.
/// Capacity is 512 elements; when full, oldest events are overwritten.
/// No heap allocation on hot record path — uses stack-allocated array.
pub struct ObservationBuffer<E> {
    events: [Option<E>; 512],
    write_index: usize,
    count: usize,
}

impl<E: Clone> ObservationBuffer<E> {
    /// Create a new observation buffer with capacity 512.
    pub fn new() -> Self {
        Self {
            events: [(); 512].map(|_| None),
            write_index: 0,
            count: 0,
        }
    }

    /// Push an event into the buffer. If buffer is full, overwrites the oldest event.
    pub fn push(&mut self, event: E) {
        self.events[self.write_index] = Some(event);
        self.write_index = (self.write_index + 1) % 512;
        if self.count < 512 {
            self.count += 1;
        }
    }

    /// Drain all events from the buffer, returning them in order and resetting state.
    pub fn drain(&mut self) -> Vec<E> {
        let mut result = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let idx = if self.count == 512 {
                (self.write_index + i) % 512
            } else {
                i
            };
            if let Some(event) = self.events[idx].take() {
                result.push(event);
            }
        }
        self.write_index = 0;
        self.count = 0;
        result
    }

    /// Return a slice of current events in the buffer (in push order).
    pub fn events(&self) -> Vec<E> {
        let mut result = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let idx = if self.count == 512 {
                (self.write_index + i) % 512
            } else {
                i
            };
            if let Some(event) = self.events[idx].clone() {
                result.push(event);
            }
        }
        result
    }

    /// Return the number of events currently in the buffer.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Return true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl<E: Clone> Default for ObservationBuffer<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_push_drain() {
        let mut buf = ObservationBuffer::new();
        for i in 0..10 {
            buf.push(DjEvent::EnergyPeak { master_rms: i as f64, timestamp_secs: 0.0 });
        }
        let drained = buf.drain();
        assert_eq!(drained.len(), 10);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_buffer_capacity() {
        let mut buf: ObservationBuffer<DjEvent> = ObservationBuffer::new();
        for i in 0..10 {
            buf.push(DjEvent::EnergyPeak { master_rms: i as f64, timestamp_secs: 0.0 });
        }
        assert_eq!(buf.len(), 10);
    }

    #[test]
    fn nde_event_buffer_works() {
        let mut buf: ObservationBuffer<NdeEvent> = ObservationBuffer::new();
        buf.push(NdeEvent::Dj(DjEvent::EnergyPeak { master_rms: 0.9, timestamp_secs: 1.0 }));
        buf.push(NdeEvent::Lore(LoreEvent { kind: "invention".into() }));
        assert_eq!(buf.len(), 2);
        let drained = buf.drain();
        assert_eq!(drained.len(), 2);
    }

    #[test]
    fn nde_event_variants_exist() {
        // Verify all 7 NdeEvent variants can be constructed.
        let _dj = NdeEvent::Dj(DjEvent::EnergyPeak { master_rms: 0.0, timestamp_secs: 0.0 });
        let _render = NdeEvent::Render(RenderEvent { kind: String::new() });
        let _physics = NdeEvent::Physics(PhysicsEvent { kind: String::new() });
        let _sieve = NdeEvent::Sieve(SieveEvent { kind: String::new() });
        let _lore = NdeEvent::Lore(LoreEvent { kind: String::new() });
        let _world = NdeEvent::World(WorldEvent { kind: String::new() });
        let _system = NdeEvent::System(SystemEvent { kind: String::new() });
        // If we reach here, all variants constructed successfully.
    }
}
