//! TEMPO RUN — geometry-as-sheet-music: the level IS the score.
//!
//! The Rosetta Stone is `BAY_PITCH_MM = 14_000`: **1 Gothic bay = 1 musical
//! measure** (4/4). With speed locked to the 174 BPM conductor, a measure lasts
//! `4 × 60_000_000 / 174 ≈ 1.379 s`, so one bay traversed per measure pins the
//! runner's speed at ~10_150 MilliUnit/s. Crossing a `OneWayExit` (a barline)
//! advances the measure; position *within* a bay projects to the beat (0..3).
//!
//! This is the integer race core, translated by hand from the PROVEN quarry
//! runner `forge-game-systems/examples/tempo_run.rs` (`cargo run` demo:
//! VICTORY 749t / 84.6% on-beat). Re-homed off its `forge_game_systems`
//! topology/gate dep onto the cart's own [`crate::zone_topology`] + a
//! `finish_crossed` flag, so the brain stays firewall-lean and WASM-clean.
//! On-beat tap → permanent `vel_mm` boost; mistime → stumble (×7/10). Base
//! velocity alone never crosses the line — you must ride the beat.

// ── The Rosetta Stone + race tuning (integer, from the proven runner) ─────────

/// One Gothic bay = one musical measure, in MilliUnit. The geometry↔score pitch.
pub const BAY_PITCH_MM: i64 = 14_000;
/// Bays in the run (start + 6 bays + victory in the topology; 6 traversed bays).
pub const N_BAYS: usize = 6;
/// Finish line: the far edge of the last bay.
pub const TRACK_LEN_MM: i64 = N_BAYS as i64 * BAY_PITCH_MM;
/// Baseline scroll per 120Hz tick — cannot finish within a song on its own.
pub const BASE_VEL_MM: i64 = 50;
/// Permanent velocity gained per on-beat tap.
pub const BOOST_MM: i64 = 18;
/// Stumble: a mistimed tap scales velocity by `STUMBLE_NUM/STUMBLE_DEN`.
pub const STUMBLE_NUM: i64 = 7;
/// Denominator for stumble scaling.
pub const STUMBLE_DEN: i64 = 10;

/// The locked conductor tempo (DnB). 4/4 throughout.
pub const REFERENCE_BPM: i64 = 174;
/// Beats per measure (4/4 time).
pub const BEATS_PER_MEASURE: i64 = 4;

/// Musical measure duration in microseconds at the locked tempo.
/// `4 beats × 60_000_000µs / 174 BPM ≈ 1_379_310µs (1.379 s)`.
pub const fn measure_duration_us() -> i64 {
    BEATS_PER_MEASURE * 60_000_000 / REFERENCE_BPM
}

/// The locked traversal speed (MilliUnit per second) = exactly one bay per
/// measure. `14_000 MU / 1.379 s ≈ 10_150 MU/s` (the design figure ~10_152
/// differs only by how the measure µs is rounded).
pub const fn locked_speed_mm_per_s() -> i64 {
    BAY_PITCH_MM * 1_000_000 / measure_duration_us()
}

// ── Race state — pure integer, deterministic, zero-alloc ──────────────────────

/// The tempo-run race state. Advanced one 120Hz tick at a time; `&self` queries
/// project the spatial position onto the musical timeline (measure + beat).
#[derive(Debug, Clone, Copy)]
pub struct TempoRun {
    /// Current position in MilliUnits.
    pos_mm: i64,
    /// Current velocity in MilliUnits per tick.
    vel_mm: i64,
    /// Total number of taps so far.
    taps: i64,
    /// Number of on-beat taps.
    on_beat: i64,
    /// Current tick number (incremented each frame).
    tick: i64,
    /// The tick on which the finish line was crossed, if any.
    finished_tick: Option<i64>,
}

impl Default for TempoRun {
    fn default() -> Self { Self::new() }
}

impl TempoRun {
    /// Create a new race state at the start position with baseline velocity.
    pub fn new() -> Self {
        Self { pos_mm: 0, vel_mm: BASE_VEL_MM, taps: 0, on_beat: 0, tick: 0, finished_tick: None }
    }

    /// Advance one 120Hz tick. `beat` = the conductor lands a beat this tick;
    /// `tapped_on_beat` = the player's input was judged on-beat for that beat.
    /// Scoring + velocity update happen only on a beat tick; position always
    /// integrates. Returns `true` once (the tick the finish line is crossed).
    pub fn tick(&mut self, beat: bool, tapped_on_beat: bool) -> bool {
        if beat {
            self.taps += 1;
            if tapped_on_beat {
                self.on_beat += 1;
                self.vel_mm += BOOST_MM; // on-beat → permanent boost
            } else {
                self.vel_mm = (self.vel_mm * STUMBLE_NUM / STUMBLE_DEN).max(BASE_VEL_MM); // stumble
            }
        }
        self.pos_mm += self.vel_mm;
        let just_crossed = self.finished_tick.is_none() && self.pos_mm >= TRACK_LEN_MM;
        if just_crossed {
            self.finished_tick = Some(self.tick);
        }
        self.tick += 1;
        just_crossed
    }

    /// Current position in MilliUnits.
    pub fn pos_mm(&self) -> i64 { self.pos_mm }
    /// Current velocity in MilliUnits per tick.
    pub fn vel_mm(&self) -> i64 { self.vel_mm }
    /// Total number of taps so far.
    pub fn taps(&self) -> i64 { self.taps }
    /// Number of on-beat taps.
    pub fn on_beat(&self) -> i64 { self.on_beat }
    /// The tick on which the finish line was crossed, if finished.
    pub fn finished_tick(&self) -> Option<i64> { self.finished_tick }

    /// The `finish_crossed` flag the topology gate reads (0/1 in the proof).
    /// Once true, the host flips the `zone_topology` SkillGate edge into victory.
    pub fn finish_crossed(&self) -> bool { self.pos_mm >= TRACK_LEN_MM }

    /// GEOMETRY → SHEET MUSIC. Which measure (= bay index, 0-based) the runner
    /// occupies. Crossing a barline (bay pitch) increments this.
    pub fn current_measure(&self) -> usize {
        ((self.pos_mm / BAY_PITCH_MM) as usize).min(N_BAYS)
    }

    /// GEOMETRY → SHEET MUSIC. Beat within the current measure, `0..BEATS_PER_MEASURE`.
    /// The position inside the bay maps linearly onto the 4 beats of the measure.
    pub fn beat_in_measure(&self) -> i64 {
        let within = self.pos_mm.rem_euclid(BAY_PITCH_MM);
        (within * BEATS_PER_MEASURE / BAY_PITCH_MM).min(BEATS_PER_MEASURE - 1)
    }

    /// On-beat ratio in Permyriad (10000 = 100%): the run's accuracy score.
    pub fn on_beat_ratio_pmy(&self) -> i64 {
        if self.taps > 0 { self.on_beat * 10_000 / self.taps } else { 0 }
    }
}

/// Deterministic ~82% on-beat tap feed (seeded LCG) — stands in for the real
/// input/MP3-onset feed so a run is TIER-1 replayable (verbatim from the proven
/// runner: the score carries fairness, not the gameplay code).
pub fn tap_on_beat(beat_idx: i64, seed: u64) -> bool {
    let x = seed
        .wrapping_add(beat_idx as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (x >> 33) % 100 < 82
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rosetta_one_bay_per_measure_locks_speed() {
        // 4 beats @ 174 BPM ≈ 1.379 s per measure; one bay per measure.
        let us = measure_duration_us();
        assert!((1_370_000..=1_390_000).contains(&us), "measure ≈ 1.379s, got {us}µs");
        let v = locked_speed_mm_per_s();
        assert!((10_100..=10_200).contains(&v), "≈10_150 MU/s (design ~10_152), got {v}");
    }

    #[test]
    fn beat_in_measure_projects_position_onto_the_four_beats() {
        let mut r = TempoRun::new();
        // Walk the runner to known fractions of bay 0 and read the beat.
        let cases = [(0, 0), (BAY_PITCH_MM / 4, 1), (BAY_PITCH_MM / 2, 2), (3 * BAY_PITCH_MM / 4, 3)];
        for (target, want_beat) in cases {
            r.pos_mm = target;
            assert_eq!(r.beat_in_measure(), want_beat, "pos {target} → beat {want_beat}");
        }
    }

    #[test]
    fn crossing_a_barline_advances_the_measure() {
        let mut r = TempoRun::new();
        r.pos_mm = BAY_PITCH_MM - 1;
        assert_eq!(r.current_measure(), 0);
        r.pos_mm = BAY_PITCH_MM; // exactly on the next barline
        assert_eq!(r.current_measure(), 1, "crossing the bay pitch crosses a barline");
    }

    #[test]
    fn riding_the_beat_wins_where_base_velocity_cannot() {
        // 1440-tick song, a beat every 60 ticks (the proven demo cadence).
        const SONG: i64 = 1_440;
        const BEAT_EVERY: i64 = 60;

        // Always on-beat → boosts accumulate → finish before the song ends.
        let mut hot = TempoRun::new();
        for t in 0..SONG {
            let beat = t % BEAT_EVERY == 0;
            if hot.tick(beat, beat) { break; }
        }
        assert!(hot.finish_crossed(), "on-beat run must cross the line");

        // Never on-beat → only base velocity → cannot finish in the song window.
        let mut cold = TempoRun::new();
        for t in 0..SONG {
            let beat = t % BEAT_EVERY == 0;
            cold.tick(beat, false);
        }
        assert!(!cold.finish_crossed(), "base velocity alone must fall short ({}mm / {}mm)", cold.pos_mm(), TRACK_LEN_MM);
    }

    #[test]
    fn deterministic_run_is_replayable() {
        const SONG: i64 = 1_440;
        const BEAT_EVERY: i64 = 60;
        let run = |seed: u64| {
            let mut r = TempoRun::new();
            for t in 0..SONG {
                if t % BEAT_EVERY == 0 {
                    let bi = t / BEAT_EVERY;
                    if r.tick(true, tap_on_beat(bi, seed)) { break; }
                } else {
                    r.tick(false, false);
                }
            }
            (r.finished_tick(), r.on_beat_ratio_pmy())
        };
        // Same seed → bit-identical outcome (TIER-1 replay rides the score).
        assert_eq!(run(0x7E_4D_07), run(0x7E_4D_07));
        // The seeded ~82% feed clears the run.
        let (ft, ratio) = run(0x7E_4D_07);
        assert!(ft.is_some(), "the proven seed must finish");
        assert!(ratio >= 7_000, "on-beat ratio ~82%, got {}.{:02}%", ratio / 100, ratio % 100);
    }
}
