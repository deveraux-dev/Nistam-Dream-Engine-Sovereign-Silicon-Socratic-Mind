//! Procedural Drum & Bass generator — **math as the base**.
//!
//! "A better 8-bit — a 64-bit banger." The point isn't the bit-depth, it's that
//! harmonics and rhythm are correct *by construction*, not by luck:
//!   * RHYTHM = Euclidean distribution ([`crate::euclid::EuclidBresenham`]) — onsets
//!     spread as evenly as math allows; the snare anchors the DnB backbeat.
//!   * HARMONY = a minor scale — every bass note is `root + a scale degree`, so it is
//!     **in-key by construction** (no out-of-scale note can be emitted).
//!   * "HIT RANGE" = enforced structurally: DnB tempo (170–176 BPM), a 16-step bar,
//!     snare on the backbeat, bass in the scale, sub-register tonic.
//!
//! Deterministic + integer-only above the DSP boundary (the seed varies the groove).
//! Output is DATA (a pattern of stepped events); a player routes it to the 808/synth.
//!
//! Ported verbatim from v2 `forge-harmonics/src/dnb.rs` (2026-08-20) to unblock
//! `forge-audio-v3::dnb_render`, which was `EXCLUDED` pending this module (its
//! own comment: "needs forge_harmonics::dnb, not ported").

use crate::euclid::EuclidBresenham;

/// 16 sixteenth-note steps per bar.
pub const STEPS_PER_BAR: u8 = 16;
/// A pattern is 2 bars = 32 steps.
pub const PATTERN_STEPS: u8 = 32;

/// Canonical DnB tempo (the genre sits ~170–176 BPM; 174 is the home key).
pub const DNB_BPM: u16 = 174;

/// Natural-minor scale degrees in semitones — the harmonic constraint. A bass note
/// is ALWAYS `root + MINOR_SCALE[d]`, so it can never leave the key.
pub const MINOR_SCALE: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Which drum voice a [`DrumHit`] triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumVoice {
    /// Downbeat / syncopated low hit.
    Kick,
    /// Backbeat anchor.
    Snare,
    /// Euclidean-distributed high hit.
    Hat,
}

/// One drum onset at a given 16th-note step.
#[derive(Debug, Clone, Copy)]
pub struct DrumHit {
    /// 16th-note step index (0..[`PATTERN_STEPS`]).
    pub step: u8,
    /// Which voice fires.
    pub voice: DrumVoice,
}

/// One bass note in the rolling sub-bass line.
#[derive(Debug, Clone, Copy)]
pub struct BassNote {
    /// 16th-note step index this note starts on.
    pub step: u8,
    /// MIDI note (in the minor scale, sub register).
    pub note: u8,
    /// Length in 16th-note steps.
    pub dur_steps: u8,
}

/// A full generated DnB pattern — drums plus in-key bass, ready for a player
/// to route to the 808/synth.
#[derive(Debug, Clone)]
pub struct DnbPattern {
    /// Tempo in BPM (always [`DNB_BPM`] from [`generate`]).
    pub bpm: u16,
    /// Total 16th-note steps in the pattern (always [`PATTERN_STEPS`]).
    pub steps: u8,
    /// Bass tonic, MIDI note number.
    pub root: u8,
    /// All drum onsets.
    pub drums: Vec<DrumHit>,
    /// All bass notes.
    pub bass: Vec<BassNote>,
}

impl DnbPattern {
    /// Sixteenth-note step duration in ms at this BPM — the playback clock.
    /// `60_000 ms/min / bpm / 4 sixteenths-per-beat`.
    pub fn step_ms(&self) -> u32 {
        (60_000 / self.bpm as u32) / 4
    }
}

/// Generate a DnB pattern. `seed` varies the groove deterministically; `root` is the
/// bass tonic (MIDI, e.g. 38 = D2 sub). Reuses [`EuclidBresenham`] for the hats.
pub fn generate(seed: u64, root: u8) -> DnbPattern {
    let mut drums = Vec::new(); // @forge:allow_alloc — cold generation, not a hot path
    let mut bass = Vec::new();

    // ── Rhythm (Euclidean + the DnB two-step skeleton) ──────────────────────
    let mut hat = EuclidBresenham::new(11, STEPS_PER_BAR as u32); // 11-in-16 driving hats
    let bars = PATTERN_STEPS / STEPS_PER_BAR;
    for bar in 0..bars {
        let base = bar * STEPS_PER_BAR;
        // Snare on the backbeat (steps 4 & 12) — the DnB anchor.
        drums.push(DrumHit { step: base + 4, voice: DrumVoice::Snare });
        drums.push(DrumHit { step: base + 12, voice: DrumVoice::Snare });
        // Kick on 1, plus a seed-chosen syncopated kick in the 2nd half (8/10/12/14).
        drums.push(DrumHit { step: base, voice: DrumVoice::Kick });
        let sync = 8 + ((seed >> (bar as u64 * 3)) % 4) as u8 * 2;
        drums.push(DrumHit { step: base + sync, voice: DrumVoice::Kick });
        // Hats — Euclidean fill across the bar.
        for s in 0..STEPS_PER_BAR {
            if hat.next_step() {
                drums.push(DrumHit { step: base + s, voice: DrumVoice::Hat });
            }
        }
    }

    // ── Harmony (minor scale → bass in-key by construction) ─────────────────
    // A rolling sub-bass: one note per beat (every 4 steps), walking the minor
    // scale by a seed-chosen degree, locked to the tonic octave (sub register).
    let mut step = 0u8;
    let mut degree = 0usize;
    while step < PATTERN_STEPS {
        let semis = MINOR_SCALE[degree % MINOR_SCALE.len()];
        bass.push(BassNote { step, note: root + semis, dur_steps: 4 });
        let walk = 1 + ((seed >> step as u64) % 3) as usize; // walk 1..=3 scale degrees
        degree += walk;
        step += 4;
    }

    DnbPattern { bpm: DNB_BPM, steps: PATTERN_STEPS, root, drums, bass }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempo_is_in_the_dnb_hit_range() {
        let p = generate(0xD0B, 38);
        assert!((170..=176).contains(&p.bpm), "DnB sits 170–176 BPM, got {}", p.bpm);
        // 60000/174/4 = 86 ms per 16th.
        assert_eq!(p.step_ms(), 86);
    }

    #[test]
    fn every_bass_note_is_in_the_minor_scale() {
        // The harmonic guarantee: no bass note can leave the key.
        for seed in 0..64u64 {
            let root = 38;
            let p = generate(seed, root);
            for b in &p.bass {
                let degree = (b.note - root) % 12;
                assert!(
                    MINOR_SCALE.contains(&degree),
                    "seed {seed}: bass note {} (degree {degree}) is out of the minor scale",
                    b.note
                );
            }
        }
    }

    #[test]
    fn snare_anchors_the_backbeat() {
        let p = generate(7, 38);
        for &expect in &[4u8, 12, 20, 28] {
            assert!(
                p.drums.iter().any(|d| d.voice == DrumVoice::Snare && d.step == expect),
                "snare must hit the backbeat step {expect}"
            );
        }
    }

    #[test]
    fn kick_always_lands_on_the_downbeat() {
        let p = generate(123, 38);
        assert!(p.drums.iter().any(|d| d.voice == DrumVoice::Kick && d.step == 0));
        assert!(p.drums.iter().any(|d| d.voice == DrumVoice::Kick && d.step == 16));
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate(42, 40);
        let b = generate(42, 40);
        assert_eq!(a.drums.len(), b.drums.len());
        assert_eq!(a.bass.len(), b.bass.len());
        assert_eq!(a.bass[3].note, b.bass[3].note);
    }

    #[test]
    fn hats_use_the_euclidean_count() {
        // 11-in-16 per bar × 2 bars = 22 hats.
        let p = generate(1, 38);
        let hats = p.drums.iter().filter(|d| d.voice == DrumVoice::Hat).count();
        assert_eq!(hats, 22, "Euclidean E(11,16) over 2 bars = 22 hats");
    }
}
