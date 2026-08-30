//! Metronome — the 120Hz integer sim clock. Ticks are the sim SoT; beats derive
//! from ticks by BPM. No float time.

use serde::{Deserialize, Serialize};

/// Fixed sim rate: 120 integer ticks per second.
pub const TICKS_PER_SECOND: u64 = 120;

/// The integer metronome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metronome {
    /// Current tick in the simulation.
    pub tick: u64,
    /// Beats per minute for this metronome.
    pub bpm: u32,
}

impl Metronome {
    /// Creates a new metronome with the given BPM, clamped to at least 1.
    pub fn new(bpm: u32) -> Self {
        Self { tick: 0, bpm: bpm.max(1) }
    }

    /// Advances the clock by n ticks and returns the new tick count.
    pub fn advance(&mut self, n: u64) -> u64 {
        self.tick += n;
        self.tick
    }

    /// Ticks in one beat at the current BPM.
    pub fn ticks_per_beat(&self) -> u64 {
        (TICKS_PER_SECOND * 60) / self.bpm as u64
    }

    /// The beat index the clock is on.
    pub fn beat(&self) -> u64 {
        self.tick / self.ticks_per_beat()
    }

    /// Is the clock exactly on a beat boundary?
    pub fn on_beat(&self) -> bool {
        self.tick % self.ticks_per_beat() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_derive_from_ticks() {
        let mut m = Metronome::new(120); // 120bpm -> 60 ticks/beat
        assert_eq!(m.ticks_per_beat(), 60);
        assert!(m.on_beat());
        m.advance(60);
        assert_eq!(m.beat(), 1);
        assert!(m.on_beat());
        m.advance(30);
        assert!(!m.on_beat());
        assert_eq!(m.beat(), 1);
    }

    #[test]
    fn faster_bpm_shorter_beat() {
        assert_eq!(Metronome::new(240).ticks_per_beat(), 30);
    }
}
