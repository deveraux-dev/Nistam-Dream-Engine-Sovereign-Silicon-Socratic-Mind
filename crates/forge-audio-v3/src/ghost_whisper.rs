//! Ghost-whisper hook: trait-only integration surface for proprietary whisper DSP.
//!
//! Signal Ghosts are a proprietary invention. The actual whisper synthesis and
//! psychoacoustic coupling live behind a closed-source wall in a separate crate.
//! This module exposes the hook contract: a trait, a registration slot, and the
//! per-block call shape the mixer uses. No DSP here.

/// Hook invoked by the mixer once per block. The implementation receives a
/// mono scratch bus initialized to zero and writes whisper samples into it.
/// The mixer sums the bus into the master output.
pub trait GhostWhisperHook: Send + Sync {
    /// Process one block. `bus` length is the block frame count. `sample_rate`
    /// is the mixer's output sample rate (44100 or 48000 in practice).
    fn tick(&mut self, bus: &mut [f32], sample_rate: u32);
}

/// Registration slot on the mixer. Exactly one hook can be registered at a time.
/// Lives on the audio worker thread; never accessed concurrently.
pub struct WhisperSlot {
    inner: Option<Box<dyn GhostWhisperHook>>,
}

impl WhisperSlot {
    pub fn new() -> Self {
        Self { inner: None }
    }

    /// Register a hook. Replaces any previously-registered hook.
    pub fn register(&mut self, hook: Box<dyn GhostWhisperHook>) {
        self.inner = Some(hook);
    }

    /// Drop the registered hook, if any.
    pub fn clear(&mut self) {
        self.inner = None;
    }

    pub fn is_registered(&self) -> bool {
        self.inner.is_some()
    }

    /// Invoke the registered hook if one is set. Returns true when a hook ran.
    /// `bus` must be zero-filled by the caller before the call.
    pub fn tick(&mut self, bus: &mut [f32], sample_rate: u32) -> bool {
        if let Some(ref mut hook) = self.inner {
            hook.tick(bus, sample_rate);
            true
        } else {
            false
        }
    }
}

impl Default for WhisperSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal hook impl used to verify call shape. Adds a fixed offset to every
    /// sample so the test can confirm the bus was mutated in place.
    struct OffsetHook {
        offset: f32,
        last_sample_rate: u32,
        last_frames: usize,
    }

    impl GhostWhisperHook for OffsetHook {
        fn tick(&mut self, bus: &mut [f32], sample_rate: u32) {
            self.last_sample_rate = sample_rate;
            self.last_frames = bus.len();
            for s in bus.iter_mut() {
                *s += self.offset;
            }
        }
    }

    #[test]
    fn slot_empty_by_default() {
        let slot = WhisperSlot::new();
        assert!(!slot.is_registered());
    }

    #[test]
    fn tick_returns_false_when_empty() {
        let mut slot = WhisperSlot::new();
        let mut bus = [0.0f32; 8];
        assert!(!slot.tick(&mut bus, 48000));
        for s in &bus {
            assert_eq!(*s, 0.0);
        }
    }

    #[test]
    fn register_then_tick_mutates_bus() {
        let mut slot = WhisperSlot::new();
        slot.register(Box::new(OffsetHook {
            offset: 0.125,
            last_sample_rate: 0,
            last_frames: 0,
        }));
        assert!(slot.is_registered());
        let mut bus = [0.0f32; 16];
        assert!(slot.tick(&mut bus, 44100));
        for s in &bus {
            assert!((*s - 0.125).abs() < 1e-6);
        }
    }

    #[test]
    fn clear_removes_hook() {
        let mut slot = WhisperSlot::new();
        slot.register(Box::new(OffsetHook {
            offset: 0.5,
            last_sample_rate: 0,
            last_frames: 0,
        }));
        slot.clear();
        assert!(!slot.is_registered());
    }

    #[test]
    fn register_replaces_previous_hook() {
        let mut slot = WhisperSlot::new();
        slot.register(Box::new(OffsetHook {
            offset: 0.25,
            last_sample_rate: 0,
            last_frames: 0,
        }));
        slot.register(Box::new(OffsetHook {
            offset: 0.75,
            last_sample_rate: 0,
            last_frames: 0,
        }));
        let mut bus = [0.0f32; 4];
        assert!(slot.tick(&mut bus, 48000));
        for s in &bus {
            assert!((*s - 0.75).abs() < 1e-6);
        }
    }
}
