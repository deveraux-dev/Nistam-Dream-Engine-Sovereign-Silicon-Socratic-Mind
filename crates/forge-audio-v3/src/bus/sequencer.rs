//! 16-step × 4-track step sequencer.
//! BPM-locked 16th notes. Outputs SeqTrigger for mixer consumption.

/// A single step in the grid. -1 = off, 0-127 = MIDI note.
pub type StepNote = i8;

/// Pending note trigger from sequencer — consumed by mixer in same callback.
#[derive(Debug, Clone)]
pub struct SeqTrigger {
    pub note: u8,
    pub velocity: u8,
    pub duration_ms: u32,
}

/// 4 tracks × 16 steps.
pub struct Sequencer {
    pub grid: [[StepNote; 16]; 4],
    /// Per-step MIDI velocity (0-127). Parallel to `grid`.
    /// Defaults to 100 so steps set via the note-only `set_step` path keep
    /// the historical behaviour; the OMR quantizer writes detected velocity
    /// via `set_step_vel` (OMR-STUDIO-SEQUENCER-BIND-001).
    pub velocities: [[u8; 16]; 4],
    pub bpm: f32,
    pub playing: bool,
    current_step: usize,
    samples_per_step: f32,
    sample_counter: f32,
    sample_rate: f32,
    /// Pending triggers for this callback — consumed by mixer after advance().
    pub pending_triggers: Vec<SeqTrigger>,
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Self {
        let bpm = 120.0;
        Self {
            grid: [[-1; 16]; 4],
            velocities: [[100; 16]; 4],
            bpm,
            playing: false,
            current_step: 0,
            samples_per_step: Self::calc_samples_per_step(sample_rate as f32, bpm),
            sample_counter: 0.0,
            sample_rate: sample_rate as f32,
            pending_triggers: Vec::with_capacity(4),
        }
    }

    fn calc_samples_per_step(sr: f32, bpm: f32) -> f32 {
        // 16th notes: 4 steps per beat
        sr * 60.0 / bpm / 4.0
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(30.0, 300.0);
        self.samples_per_step = Self::calc_samples_per_step(self.sample_rate, self.bpm);
    }

    pub fn set_step(&mut self, track: usize, step: usize, note: StepNote) {
        if track < 4 && step < 16 {
            self.grid[track][step] = note;
        }
    }

    /// Set a step with an explicit MIDI velocity (0-127, clamped).
    /// Used by the OMR -> sequencer quantizer to preserve scanned dynamics
    /// instead of the historical hardcoded velocity (OMR-STUDIO-SEQUENCER-BIND-001).
    pub fn set_step_vel(&mut self, track: usize, step: usize, note: StepNote, velocity: u8) {
        if track < 4 && step < 16 {
            self.grid[track][step] = note;
            self.velocities[track][step] = velocity.min(127);
        }
    }

    pub fn start(&mut self) {
        self.playing = true;
        self.current_step = 0;
        self.sample_counter = 0.0;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_step = 0;
        self.sample_counter = 0.0;
    }

    /// Call from the audio thread each frame. Populates pending_triggers.
    pub fn advance(&mut self, num_frames: usize) {
        if !self.playing { return; }
        self.pending_triggers.clear();

        for _ in 0..num_frames {
            self.sample_counter += 1.0;
            if self.sample_counter >= self.samples_per_step {
                self.sample_counter -= self.samples_per_step;
                self.trigger_step();
                self.current_step = (self.current_step + 1) % 16;
            }
        }
    }

    fn trigger_step(&mut self) {
        let step = self.current_step;
        let dur = (self.samples_per_step / self.sample_rate * 800.0) as u32;
        for track in 0..4 {
            let note = self.grid[track][step];
            if note >= 0 {
                self.pending_triggers.push(SeqTrigger {
                    note: note as u8,
                    velocity: self.velocities[track][step],
                    duration_ms: dur,
                });
            }
        }
    }

    pub fn current_step(&self) -> usize { self.current_step }
}

// ── rms → velocity accent (CARTRIDGE-MODULATION-SNAPSHOT-001) ──────────────
//
// Bridges the portable `ModulationSnapshot` into the step grid: louder audio
// lifts every active step's velocity toward the MIDI ceiling. The mapping is
// PURE and DETERMINISTIC — same `(base, rms)` always yields the same accented
// velocity — and bounded to the MIDI range `[0, 127]` for the full f32 input
// domain.

use crate::modulation::ModulationSnapshot;

/// Maximum velocity lift, in MIDI velocity units, applied at full-scale
/// (`rms == 1.0`) audio. At `rms == 0.0` the accent is zero — silence leaves
/// the base velocity untouched.
pub const MODULATION_ACCENT_MAX: u8 = 27;

/// Pure, deterministic `rms → accented velocity` mapping.
///
/// `accent = round(clamp(rms,0,1) * MODULATION_ACCENT_MAX)`, added to
/// `base_velocity` and clamped to the MIDI range `[0, 127]`. Non-finite `rms`
/// is treated as `0.0` (no accent). No state, no allocation.
#[inline]
pub fn accent_velocity(base_velocity: u8, rms: f32) -> u8 {
    let rms = if rms.is_finite() { rms.clamp(0.0, 1.0) } else { 0.0 };
    // `+ 0.5` then truncate = round-half-up; deterministic for all inputs.
    let accent = (rms * MODULATION_ACCENT_MAX as f32 + 0.5) as u16;
    let v = base_velocity as u16 + accent;
    v.min(127) as u8
}

impl Sequencer {
    /// Apply a [`ModulationSnapshot`] to the velocity grid as a loudness
    /// accent, computed deterministically from an explicit `base` grid.
    ///
    /// `base` is the un-modulated velocity grid (e.g. the OMR-quantized
    /// velocities). For every ON step (`grid[t][s] >= 0`) the accented
    /// velocity is written; OFF steps are skipped. Passing the base grid in —
    /// rather than accenting `self.velocities` in place — makes the operation
    /// IDEMPOTENT: re-applying the same snapshot reproduces the same grid
    /// instead of compounding the accent.
    ///
    /// NO-ALLOC / audio-hot-path safe: the body is two nested fixed-count
    /// loops over the inline `[[u8; 16]; 4]` arrays. No heap, no `Vec`
    /// growth, no `pending_triggers` mutation — nothing that can allocate.
    pub fn apply_modulation(
        &mut self,
        snapshot: &ModulationSnapshot,
        base: &[[u8; 16]; 4],
    ) {
        for track in 0..4 {
            for step in 0..16 {
                if self.grid[track][step] >= 0 {
                    self.velocities[track][step] =
                        accent_velocity(base[track][step], snapshot.rms);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the sequencer until it emits a trigger for `step` 0, return it.
    fn trigger_for_first_step(seq: &mut Sequencer) -> Option<SeqTrigger> {
        seq.start();
        // Step 0 fires once the sample counter crosses one step's worth of
        // samples. Advance generously past one full step boundary.
        let frames = seq.samples_per_step.ceil() as usize + 1;
        seq.advance(frames);
        seq.pending_triggers.first().cloned()
    }

    #[test]
    fn set_step_vel_preserves_velocity_into_trigger() {
        let mut seq = Sequencer::new(44_100);
        // OMR-detected velocity 73 on track 0, step 0.
        seq.set_step_vel(0, 0, 60, 73);
        let trig = trigger_for_first_step(&mut seq).expect("step 0 should trigger");
        assert_eq!(trig.note, 60);
        assert_eq!(trig.velocity, 73, "velocity must be preserved, not hardcoded 100");
        assert_ne!(trig.velocity, 100, "velocity must NOT be the old hardcoded default");
    }

    #[test]
    fn set_step_vel_clamps_velocity() {
        let mut seq = Sequencer::new(44_100);
        seq.set_step_vel(1, 0, 64, 200);
        assert_eq!(seq.velocities[1][0], 127, "velocity clamps to MIDI max 127");
    }

    #[test]
    fn note_only_set_step_keeps_default_velocity() {
        let mut seq = Sequencer::new(44_100);
        // The UI note-only path leaves velocity at the default.
        seq.set_step(0, 0, 48);
        assert_eq!(seq.velocities[0][0], 100);
    }

    #[test]
    fn set_step_vel_ignores_out_of_range_indices() {
        let mut seq = Sequencer::new(44_100);
        seq.set_step_vel(9, 99, 60, 50); // both out of range — must not panic
        assert_eq!(seq.grid[0][0], -1);
    }

    // ── rms → velocity accent (CARTRIDGE-MODULATION-SNAPSHOT-001) ──────────

    #[test]
    fn accent_velocity_is_deterministic() {
        // (4b) same (base, rms) -> same accented velocity, every time.
        for base in [0u8, 40, 100, 127] {
            for i in 0..=100 {
                let rms = i as f32 / 100.0;
                let a = accent_velocity(base, rms);
                let b = accent_velocity(base, rms);
                assert_eq!(a, b, "accent must be deterministic (base={base}, rms={rms})");
            }
        }
    }

    #[test]
    fn accent_velocity_endpoints_and_monotonic() {
        // Silence => no accent. Full scale => base + MODULATION_ACCENT_MAX.
        assert_eq!(accent_velocity(60, 0.0), 60, "silence leaves base untouched");
        assert_eq!(
            accent_velocity(60, 1.0),
            (60 + MODULATION_ACCENT_MAX as u16).min(127) as u8,
            "full-scale rms lifts by MODULATION_ACCENT_MAX"
        );
        // Louder is never quieter — monotonic non-decreasing in rms.
        let mut prev = accent_velocity(50, 0.0);
        for i in 1..=100 {
            let v = accent_velocity(50, i as f32 / 100.0);
            assert!(v >= prev, "accent must be non-decreasing in rms");
            prev = v;
        }
    }

    #[test]
    fn accent_velocity_is_bounded_for_full_f32_domain() {
        let rmss = [
            -1e9_f32, -1.0, -0.001, 0.0, 0.5, 1.0, 1.001, 1e9,
            f32::NAN, f32::INFINITY, f32::NEG_INFINITY,
        ];
        for base in [0u8, 110, 120, 127] {
            for &rms in &rmss {
                let v = accent_velocity(base, rms);
                assert!(v <= 127, "velocity {v} exceeds MIDI max (base={base}, rms={rms})");
            }
        }
        // Non-finite rms must behave exactly like silence.
        assert_eq!(accent_velocity(64, f32::NAN), accent_velocity(64, 0.0));
        assert_eq!(accent_velocity(64, f32::INFINITY), accent_velocity(64, 0.0));
    }

    #[test]
    fn apply_modulation_accents_on_steps_and_skips_off_steps() {
        let mut seq = Sequencer::new(44_100);
        // Track 0 step 0 ON; everything else OFF.
        seq.set_step_vel(0, 0, 60, 80);
        let base = seq.velocities; // capture un-modulated base (Copy)

        let loud = ModulationSnapshot::from_rms(1.0);
        seq.apply_modulation(&loud, &base);

        // ON step accented.
        assert_eq!(seq.velocities[0][0], accent_velocity(80, 1.0));
        assert!(seq.velocities[0][0] > 80, "loud audio must lift the ON step");
        // OFF step untouched — still the default base velocity.
        assert_eq!(seq.velocities[1][5], base[1][5]);
    }

    #[test]
    fn apply_modulation_is_idempotent() {
        // Re-applying the same snapshot from the same base must not compound.
        let mut seq = Sequencer::new(44_100);
        seq.set_step_vel(0, 0, 60, 70);
        seq.set_step_vel(2, 9, 64, 50);
        let base = seq.velocities;

        let m = ModulationSnapshot::from_rms(0.6);
        seq.apply_modulation(&m, &base);
        let once = seq.velocities;
        seq.apply_modulation(&m, &base);
        let twice = seq.velocities;
        assert_eq!(once, twice, "apply_modulation must be idempotent for a fixed base");
    }

    #[test]
    fn apply_modulation_silence_is_identity_on_velocities() {
        let mut seq = Sequencer::new(44_100);
        seq.set_step_vel(0, 0, 60, 88);
        seq.set_step_vel(3, 15, 72, 41);
        let base = seq.velocities;
        seq.apply_modulation(&ModulationSnapshot::silent(), &base);
        assert_eq!(seq.velocities, base, "a silent snapshot must not change velocities");
    }
}
