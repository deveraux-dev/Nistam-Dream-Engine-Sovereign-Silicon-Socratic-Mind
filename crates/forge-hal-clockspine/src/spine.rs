//! Event-stream spine — the in-memory model of ADR-012, built on the metronome.
//!
//! Every modality (sound, light, colour, motion, camera, weather, script) is a
//! tick-stamped, laned, integer [`Primitive`]. Primitives STACK per unit per tick
//! (the "combo per unit"); a [`Stack`]'s deterministic hash is its replay
//! signature; the [`Sequencer`]'s playhead IS the metronome tick. Integer-only,
//! no-alloc (fixed arrays). The heavy subsystems consume this contract rather
//! than re-inventing time.
//!
//! Build ladder (each rung proven in `tests`, on the proven rung below):
//! r2 event@tick · r3 stack/unit · r4 replay-hash · r5 thesias · r6 subdivisions · r7 sequencer.

use crate::fixed::{Permyriad, SimTick};
use crate::metronome::MetronomeClock;

/// The subsystem axis — WHICH system renders/applies a primitive. Orthogonal to
/// priority lanes; this is the modality axis (ADR-012 D3/D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Modality {
    /// Audible (UMP note).
    Sound = 0,
    /// Sound -> motion/light (render vibe matrix).
    SeeHear = 1,
    /// Sound/semantic -> colour (render signal palette).
    Technothesia = 2,
    /// Pose / keyframe (animation).
    Animation = 3,
    /// Camera push / shake.
    Camera = 4,
    /// Weather intensity.
    Weather = 5,
    /// Script hook (semantic dispatch).
    Script = 6,
}

impl Modality {
    /// Number of modalities.
    pub const COUNT: usize = 7;

    /// Stable wire byte.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire byte; `None` if out of range.
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Modality> {
        match v {
            0 => Some(Modality::Sound),
            1 => Some(Modality::SeeHear),
            2 => Some(Modality::Technothesia),
            3 => Some(Modality::Animation),
            4 => Some(Modality::Camera),
            5 => Some(Modality::Weather),
            6 => Some(Modality::Script),
            _ => None,
        }
    }
}

/// One tick-stamped, laned, integer event applied to a unit. `Copy`, no alloc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Primitive {
    /// When it starts (metronome tick).
    pub tick: SimTick,
    /// Who it targets (unit / cell id).
    pub unit: u32,
    /// Which subsystem renders it.
    pub modality: Modality,
    /// Integer value (Permyriad, 0..=10000).
    pub param: i32,
    /// How many ticks it lasts (0 = instantaneous single-tick).
    pub duration_ticks: u16,
}

impl Primitive {
    /// The zero primitive (slot filler for fixed stacks).
    pub const ZERO: Primitive = Primitive {
        tick: SimTick::ZERO,
        unit: 0,
        modality: Modality::Sound,
        param: 0,
        duration_ticks: 0,
    };

    /// Construct explicitly.
    #[inline]
    pub const fn new(tick: SimTick, unit: u32, modality: Modality, param: i32, duration_ticks: u16) -> Self {
        Self { tick, unit, modality, param, duration_ticks }
    }

    /// r2 — stamp a primitive at the clock's CURRENT tick: the event rides the heartbeat.
    #[inline]
    pub fn at(clock: &MetronomeClock, unit: u32, modality: Modality, param: i32, duration_ticks: u16) -> Self {
        Self::new(clock.tick(), unit, modality, param, duration_ticks)
    }

    /// Active at `tick` iff `tick` is within `[self.tick, self.tick + duration)`.
    /// A 0-duration primitive is active only on its own tick.
    #[inline]
    pub const fn active_at(&self, tick: SimTick) -> bool {
        let span = if self.duration_ticks == 0 { 1 } else { self.duration_ticks as u64 };
        tick.0 >= self.tick.0 && tick.0 < self.tick.0 + span
    }

    /// Fold this primitive into a running FNV-1a-style integer hash. Integer-only,
    /// so no float can leak in — the hash is the exact replay signature.
    #[inline]
    const fn fold(&self, mut h: u64) -> u64 {
        const PRIME: u64 = 0x0000_0100_0000_01B3;
        h = (h ^ self.tick.0).wrapping_mul(PRIME);
        h = (h ^ self.unit as u64).wrapping_mul(PRIME);
        h = (h ^ self.modality.as_u8() as u64).wrapping_mul(PRIME);
        h = (h ^ (self.param as u32 as u64)).wrapping_mul(PRIME);
        h = (h ^ self.duration_ticks as u64).wrapping_mul(PRIME);
        h
    }
}

/// r5 — fan a Sound primitive into its two render-thesias at the same tick/unit:
/// SeeHear (motion/light) + Technothesia (colour). One packet -> both fire.
#[inline]
pub fn derive_thesias(sound: Primitive) -> (Primitive, Primitive) {
    let syn = Primitive::new(sound.tick, sound.unit, Modality::SeeHear, sound.param, sound.duration_ticks);
    let col = Primitive::new(sound.tick, sound.unit, Modality::Technothesia, sound.param, sound.duration_ticks);
    (syn, col)
}

/// Hook an **audio-brush stroke** onto the spine. A pen sample (`pressure` ->
/// loudness, `hue` -> colour) becomes a Sound primitive plus its two render
/// thesias, all stamped at the current metronome tick: one stroke -> one frame
/// that sounds, moves (seehear brightness from pressure), and tints
/// (technothesia from the brush hue-shift). `pressure`/`hue` are clamped to
/// Permyriad range [0, 10_000].
pub fn audio_brush_stroke(clock: &MetronomeClock, unit: u32, pressure: i32, hue: i32) -> Stack {
    let loud = Permyriad::clamp(pressure).0;
    let tint = Permyriad::clamp(hue).0;
    let sound = Primitive::at(clock, unit, Modality::Sound, loud, 1);
    let (syn, mut col) = derive_thesias(sound);
    col.param = tint;
    let mut frame = Stack::new();
    frame.add(sound);
    frame.add(syn);
    frame.add(col);
    frame
}

/// r6 — does a consumer on divisor `div` fire at metronome `tick`? Every rate is
/// an integer division of 120 Hz. `div == 0` never fires (guard).
#[inline]
pub const fn fires_on(tick: u64, div: u64) -> bool {
    div != 0 && tick % div == 0
}

/// Max primitives stacked on ONE unit at ONE tick (ADR-012 no-alloc fixed stack).
pub const MAX_STACK: usize = 8;

/// All primitives on one unit at one tick — the "combo per unit" frame. Fixed
/// array, no heap. Apply order = insertion order (stable -> deterministic).
#[derive(Debug, Clone, Copy)]
pub struct Stack {
    /// The fixed array of primitives in apply order.
    items: [Primitive; MAX_STACK],
    /// Number of valid primitives in `items`.
    len: usize,
}

impl Stack {
    /// Empty stack.
    #[inline]
    pub const fn new() -> Self {
        Self { items: [Primitive::ZERO; MAX_STACK], len: 0 }
    }

    /// r3 — add a primitive. Returns `false` (drops it) if the fixed stack is full;
    /// never allocates, never panics.
    #[inline]
    pub fn add(&mut self, p: Primitive) -> bool {
        if self.len < MAX_STACK {
            self.items[self.len] = p;
            self.len += 1;
            true
        } else {
            false
        }
    }

    /// Number of stacked primitives.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True if empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The active primitives, in apply order.
    #[inline]
    pub fn as_slice(&self) -> &[Primitive] {
        &self.items[..self.len]
    }

    /// r5 — does this frame fire `modality`? One drop can fire several -> one frame
    /// lights seehear AND technothesia at once.
    pub fn fires(&self, modality: Modality) -> bool {
        let mut i = 0;
        while i < self.len {
            if self.items[i].modality == modality {
                return true;
            }
            i += 1;
        }
        false
    }

    /// r4 — deterministic, order-sensitive hash = the replay signature. Same stack
    /// -> same hash; change any field or order -> different hash. Integer-only.
    pub const fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < self.len {
            h = self.items[i].fold(h);
            i += 1;
        }
        h
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

/// Max scheduled primitives in the sequencer's fixed ring.
pub const SEQ_CAP: usize = 64;

/// r7 — the bottom-bar sequencer: schedules primitives on the metronome grid; its
/// playhead IS the clock tick. Fixed-capacity, no alloc.
#[derive(Debug, Clone, Copy)]
pub struct Sequencer {
    /// The scheduled primitives in insertion order.
    events: [Primitive; SEQ_CAP],
    /// Number of scheduled primitives.
    len: usize,
}

impl Sequencer {
    /// An empty sequencer.
    #[inline]
    pub const fn new() -> Self {
        Self { events: [Primitive::ZERO; SEQ_CAP], len: 0 }
    }

    /// The playhead = the metronome tick (there is no separate clock).
    #[inline]
    pub const fn playhead(clock: &MetronomeClock) -> SimTick {
        clock.tick()
    }

    /// Drop a placeholder onto the timeline = schedule a primitive. `false` if full.
    #[inline]
    pub fn schedule(&mut self, p: Primitive) -> bool {
        if self.len < SEQ_CAP {
            self.events[self.len] = p;
            self.len += 1;
            true
        } else {
            false
        }
    }

    /// Number of scheduled primitives.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True if nothing is scheduled.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The [`Stack`] of primitives due at `tick` (those `active_at` it). Fresh,
    /// fixed, no alloc; excess beyond [`MAX_STACK`] is dropped.
    pub fn due(&self, tick: SimTick) -> Stack {
        let mut s = Stack::new();
        let mut i = 0;
        while i < self.len {
            let p = self.events[i];
            if p.active_at(tick) {
                let _ = s.add(p);
            }
            i += 1;
        }
        s
    }
}

impl Default for Sequencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modality_wire_roundtrips() {
        for v in 0..Modality::COUNT as u8 {
            let m = Modality::from_u8(v).expect("valid modality byte");
            assert_eq!(m.as_u8(), v, "as_u8/from_u8 round-trip");
        }
        assert!(Modality::from_u8(Modality::COUNT as u8).is_none(), "out-of-range byte rejected");
    }

    #[test]
    fn r2_primitive_stamps_the_current_tick() {
        let mut clk = MetronomeClock::new();
        for _ in 0..42 {
            clk.advance();
        }
        let p = Primitive::at(&clk, 7, Modality::Sound, 8000, 4);
        assert_eq!(p.tick, SimTick(42), "stamped with the clock's current tick");
        assert_eq!(p.unit, 7);
        assert_eq!(p.modality, Modality::Sound);
        assert!(p.active_at(SimTick(42)) && p.active_at(SimTick(45)) && !p.active_at(SimTick(46)), "active across its duration only");
        assert!(!p.active_at(SimTick(41)), "not before its tick");
    }

    #[test]
    fn r3_stack_is_fixed_and_overflow_safe() {
        let mut s = Stack::new();
        assert!(s.is_empty());
        for i in 0..MAX_STACK {
            assert!(s.add(Primitive::new(SimTick(10), 1, Modality::Animation, i as i32, 1)), "fits within capacity");
        }
        assert_eq!(s.len(), MAX_STACK);
        assert!(!s.add(Primitive::new(SimTick(10), 1, Modality::Sound, 0, 1)), "overflow drops, never allocs/panics");
        assert_eq!(s.len(), MAX_STACK, "len unchanged after overflow");
    }

    #[test]
    fn r4_stack_hash_is_the_replay_signature() {
        let build = || {
            let mut s = Stack::new();
            s.add(Primitive::new(SimTick(10), 1, Modality::Sound, 8000, 2));
            s.add(Primitive::new(SimTick(10), 1, Modality::SeeHear, 8000, 2));
            s
        };
        assert_eq!(build().hash(), build().hash(), "same stack => same hash (replayable)");

        let mut c = Stack::new();
        c.add(Primitive::new(SimTick(10), 1, Modality::Sound, 8001, 2));
        c.add(Primitive::new(SimTick(10), 1, Modality::SeeHear, 8000, 2));
        assert_ne!(build().hash(), c.hash(), "any change => different hash");

        let mut d = Stack::new();
        d.add(Primitive::new(SimTick(10), 1, Modality::SeeHear, 8000, 2));
        d.add(Primitive::new(SimTick(10), 1, Modality::Sound, 8000, 2));
        assert_ne!(build().hash(), d.hash(), "apply order is part of the signature");
    }

    #[test]
    fn r5_one_sound_packet_fires_both_thesias() {
        let sound = Primitive::new(SimTick(10), 3, Modality::Sound, 7000, 1);
        let (syn, col) = derive_thesias(sound);
        assert_eq!((syn.modality, col.modality), (Modality::SeeHear, Modality::Technothesia));
        assert_eq!((syn.tick, syn.unit), (sound.tick, sound.unit), "same tick + unit");

        let mut frame = Stack::new();
        frame.add(sound);
        frame.add(syn);
        frame.add(col);
        assert!(
            frame.fires(Modality::Sound) && frame.fires(Modality::SeeHear) && frame.fires(Modality::Technothesia),
            "one drop -> sound + motion + colour on one frame"
        );
        assert!(!frame.fires(Modality::Weather), "absent lanes do not fire");
    }

    #[test]
    fn audio_brush_stroke_hooks_the_spine() {
        let mut clk = MetronomeClock::new();
        for _ in 0..9 {
            clk.advance();
        }
        let frame = audio_brush_stroke(&clk, 4, 8200, 3300);
        assert_eq!(frame.len(), 3, "one stroke -> sound + seehear + technothesia");
        assert!(
            frame.fires(Modality::Sound) && frame.fires(Modality::SeeHear) && frame.fires(Modality::Technothesia)
        );
        let s = frame.as_slice();
        assert_eq!(s[0].tick, SimTick(9), "stamped at the current heartbeat tick");
        assert_eq!(s[0].param, 8200, "sound loudness = brush pressure");
        assert_eq!(s[1].param, 8200, "seehear brightness rides loudness");
        assert_eq!(s[2].param, 3300, "technothesia hue = brush hue-shift");

        let clamped = audio_brush_stroke(&clk, 4, 99_999, -5);
        assert_eq!(clamped.as_slice()[0].param, 10_000, "pressure clamps to Permyriad max");
        assert_eq!(clamped.as_slice()[2].param, 0, "hue clamps to 0");
    }

    #[test]
    fn r6_subdivisions_are_integer_exact() {
        let (mut phys, mut film) = (0u64, 0u64);
        for t in 0..120u64 {
            if fires_on(t, 2) {
                phys += 1;
            }
            if fires_on(t, 5) {
                film += 1;
            }
        }
        assert_eq!(phys, 60, "60 physics steps / 120 ticks");
        assert_eq!(film, 24, "24 film frames / 120 ticks");
        assert!(!fires_on(5, 0), "divisor 0 never fires (guard)");

        let mut clk = MetronomeClock::new();
        for _ in 0..120 {
            clk.advance();
        }
        assert_eq!(clk.sample_index(), 48_000, "120 ticks = 1 s = 48000 samples");
    }

    #[test]
    fn r7_sequencer_playhead_is_the_tick() {
        let mut clk = MetronomeClock::new();
        for _ in 0..5 {
            clk.advance();
        }
        assert_eq!(Sequencer::playhead(&clk), SimTick(5), "playhead == clock tick");

        let mut seq = Sequencer::new();
        assert!(seq.schedule(Primitive::new(SimTick(5), 1, Modality::Sound, 9000, 1)));
        assert!(seq.schedule(Primitive::new(SimTick(5), 1, Modality::Technothesia, 9000, 1)));
        assert!(seq.schedule(Primitive::new(SimTick(9), 1, Modality::Animation, 5000, 1)));

        let due_now = seq.due(SimTick(5));
        assert_eq!(due_now.len(), 2, "two drops at tick 5 are due at tick 5");
        assert!(due_now.fires(Modality::Sound) && due_now.fires(Modality::Technothesia));
        assert_eq!(seq.due(SimTick(0)).len(), 0, "nothing due before its tick");
        assert_eq!(seq.due(SimTick(9)).len(), 1, "the tick-9 drop is due at 9");
    }

    #[test]
    fn proving_slice_clock_to_replay() {
        let make = || {
            let mut clk = MetronomeClock::new();
            for _ in 0..12 {
                clk.advance();
            }
            let sound = Primitive::at(&clk, 2, Modality::Sound, 6000, 3);
            let (syn, col) = derive_thesias(sound);
            let mut frame = Stack::new();
            frame.add(sound);
            frame.add(syn);
            frame.add(col);
            (clk.tick(), frame.hash())
        };
        assert_eq!(make(), make(), "same inputs on the metronome => bit-identical (tick, hash) — replayable");
    }
}
