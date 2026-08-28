//! Audio domain — "speed locked to the music" (the moat). Every layer rides ONE
//! 120Hz clock, so beats are deterministic TICK positions and motion scales with
//! tempo, sample-exact. Brain-side: a deterministic beat clock + a tempo->motion
//! mapping fed through the `MotionSink` (`forge_vix::kinetic::phrase_motion`).
//! Sample generation lives host-side, below the `HarmonicsSink` (the 3-rate
//! model: this domain only ever emits 120Hz EVENTS).

/// Ticks per minute at 120Hz (120 * 60).
const TICKS_PER_MINUTE: u64 = 7_200;

/// phrase_motion reference tempo (matches `forge_vix::kinetic::PHRASE_TEMPO_REFERENCE`).
pub const TEMPO_REFERENCE_Q: u32 = 10_000;
/// The reference BPM (DnB tempo) that maps to [`TEMPO_REFERENCE_Q`].
pub const REFERENCE_BPM: u32 = 170;

/// A deterministic beat clock: maps the 120Hz tick stream to musical beats at a
/// given tempo. Sample-exact because beats are integer tick positions — this is
/// why the engine can lock speed to music when nothing else can.
#[derive(Clone, Copy, Debug)]
pub struct BeatClock {
    /// Beats per minute.
    pub bpm: u16,
    /// Ticks per beat, computed from BPM.
    ticks_per_beat: u64,
    /// The tick number of the next beat boundary.
    next_beat_tick: u64,
    /// Zero-indexed beat counter.
    beat_index: u64,
}

impl BeatClock {
    /// Create a new beat clock at the given BPM, clamped to [30, 300].
    pub fn new(bpm: u16) -> Self {
        let bpm = bpm.clamp(30, 300);
        let ticks_per_beat = (TICKS_PER_MINUTE / bpm as u64).max(1);
        Self { bpm, ticks_per_beat, next_beat_tick: ticks_per_beat, beat_index: 0 }
    }

    /// True iff `tick` reaches the next beat boundary — fires once per beat as
    /// the sequential tick stream crosses each integer multiple of the period.
    pub fn is_beat(&mut self, tick: u64) -> bool {
        if tick >= self.next_beat_tick {
            self.beat_index += 1;
            self.next_beat_tick += self.ticks_per_beat;
            true
        } else {
            false
        }
    }

    /// Number of ticks per beat at the current BPM.
    pub fn ticks_per_beat(&self) -> u64 {
        self.ticks_per_beat
    }

    /// The current zero-indexed beat number.
    pub fn beat_index(&self) -> u64 {
        self.beat_index
    }
}

/// Map a musical tempo (BPM) to the `tempo_q` permyriad that drives
/// phrase_motion: higher BPM -> higher `tempo_q` -> faster, snappier motion.
/// **Speed = tempo.** Clamped to phrase_motion's accepted range [1_000, 40_000].
pub fn tempo_q_from_bpm(bpm: u16) -> u16 {
    ((bpm as u32 * TEMPO_REFERENCE_Q) / REFERENCE_BPM).clamp(1_000, 40_000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_fire_at_the_tempo() {
        let mut clock = BeatClock::new(170); // DnB
        let tpb = clock.ticks_per_beat();
        assert!(tpb > 0);
        let mut beats = 0u64;
        for t in 1..=tpb * 10 {
            if clock.is_beat(t) {
                beats += 1;
            }
        }
        assert_eq!(beats, 10, "10 beat boundaries over 10 beats' worth of ticks");
    }

    #[test]
    fn faster_tempo_fires_more_beats() {
        // Speed = tempo: a DnB break fires more beats than half-time over the
        // same tick span, deterministically.
        fn beats_over(bpm: u16, span: u64) -> u64 {
            let mut clock = BeatClock::new(bpm);
            (1..=span).filter(|&t| clock.is_beat(t)).count() as u64
        }
        let fast = beats_over(170, 1200);
        let slow = beats_over(85, 1200);
        assert!(fast > slow, "higher tempo => more beats (fast={fast} slow={slow})");
    }

    #[test]
    fn tempo_q_scales_with_bpm_and_is_bounded() {
        assert!(tempo_q_from_bpm(170) > tempo_q_from_bpm(85), "speed scales with tempo");
        assert_eq!(tempo_q_from_bpm(170), 10_000, "reference DnB tempo maps to the reference q");
        assert_eq!(tempo_q_from_bpm(5), 1_000, "floor clamp");
        assert_eq!(tempo_q_from_bpm(700), 40_000, "ceiling clamp");
    }
}
