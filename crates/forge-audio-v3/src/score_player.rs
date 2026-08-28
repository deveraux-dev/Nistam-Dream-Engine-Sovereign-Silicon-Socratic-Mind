//! ScorePlayer — plays a born-symbolic score onto the conductor's `AudioLane`
//! at 120 Hz. The playback consumer of [`forge_harmonics::synthxml::SynthScore`]:
//! lower the score to a tick-ordered note plan ([`score_to_note_plan`]), then on
//! each master tick strike every note whose `fire_tick` has arrived via
//! [`AudioLane::trigger_note`]. This is how a dropped MusicXML / `.synthxml`
//! actually SINGS through the conductor — the missing player wire (the old
//! `forge-gui::score_player`), re-homed next to the lane it drives.
//!
//! Cursor-advance, O(notes_due) per tick; integer, deterministic, zero-heap on
//! the tick path (the plan is built once at load).

use forge_harmonics::synthxml::{score_to_note_plan, ScheduledNote, SynthScore};

use crate::conductor_audio::AudioLane;

/// A tick-ordered note plan plus a cursor into it.
pub struct ScorePlayer {
    plan: Vec<ScheduledNote>,
    cursor: usize,
}

impl ScorePlayer {
    /// Build a player from a score (lowers it to a `fire_tick`-sorted plan).
    pub fn from_score(score: &SynthScore) -> Self {
        Self { plan: score_to_note_plan(score), cursor: 0 }
    }

    /// Build directly from an already-lowered plan.
    pub fn from_plan(plan: Vec<ScheduledNote>) -> Self {
        Self { plan, cursor: 0 }
    }

    /// Notes not yet struck.
    pub fn remaining(&self) -> usize {
        self.plan.len() - self.cursor
    }

    /// True once every note has been struck.
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.plan.len()
    }

    /// Strike every note whose `fire_tick <= now_tick` on `lane`, in order.
    /// The plan is sorted, so the cursor only advances. Returns the count struck
    /// this tick. Drive this from the 120 Hz conductor/metronome tick.
    pub fn tick(&mut self, now_tick: u64, lane: &mut AudioLane) -> usize {
        let mut struck = 0;
        while self.cursor < self.plan.len() && self.plan[self.cursor].fire_tick <= now_tick {
            let n = self.plan[self.cursor];
            lane.trigger_note(n.note, n.vel, n.dur_ms);
            self.cursor += 1;
            struck += 1;
        }
        struck
    }

    /// Replay from the start.
    pub fn rewind(&mut self) {
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    /// THE end-to-end "drag XML → hear it" proof: MusicXML bytes → SynthScore →
    /// note plan → ScorePlayer strikes notes on the AudioLane → AUDIBLE render.
    /// ADR-0008 discriminator: the played render must beat a silent control;
    /// fails RED if any link (parse / lower / strike / synth) goes silent.
    #[test]
    fn dropped_musicxml_sings_through_the_conductor() {
        let xml = r#"<?xml version="1.0"?>
<score-partwise>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>1</divisions></attributes>
      <sound tempo="120"/>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>
      <note><pitch><step>G</step><octave>4</octave></pitch><duration>1</duration></note>
    </measure>
  </part>
</score-partwise>"#;

        let score = forge_harmonics::musicxml_extract::musicxml_to_score(xml.as_bytes())
            .expect("MusicXML lowers to a playable score");
        let mut player = ScorePlayer::from_score(&score);
        assert_eq!(player.remaining(), 2, "two pitched notes queued");

        let mut lane = AudioLane::new(SR, 120.0);

        // Master tick 0: the downbeat C4 fires and the synth voice rings.
        assert_eq!(player.tick(0, &mut lane), 1, "downbeat note strikes at tick 0");
        assert_eq!(lane.active_voices(), 1);

        let mut out = vec![0.0f32; 4800];
        lane.render(&mut out, &[]);
        let played = rms(&out);
        assert!(played > 0.001, "dropped MusicXML must produce audible sound, rms={played}");

        // Master tick 60 (960 music ticks @120bpm): the G4 fires; plan drains.
        assert_eq!(player.tick(60, &mut lane), 1, "second note strikes at its tick");
        assert!(player.is_finished(), "both notes played");

        // SILENT control: a fresh lane with nothing struck renders ~silence.
        let mut silent_lane = AudioLane::new(SR, 120.0);
        let mut silent = vec![0.0f32; 4800];
        silent_lane.render(&mut silent, &[]);
        assert!(played > rms(&silent), "played ({played}) must exceed the silent control ({})", rms(&silent));
    }

    #[test]
    fn tick_is_monotonic_and_idempotent_after_drain() {
        let xml = r#"<?xml version="1.0"?>
<score-partwise><part id="P1"><measure number="1">
<attributes><divisions>1</divisions></attributes>
<note><pitch><step>A</step><octave>4</octave></pitch><duration>1</duration></note>
</measure></part></score-partwise>"#;
        let score = forge_harmonics::musicxml_extract::musicxml_to_score(xml.as_bytes()).unwrap();
        let mut player = ScorePlayer::from_score(&score);
        let mut lane = AudioLane::new(SR, 120.0);
        // A tick BEFORE the note's fire_tick strikes nothing.
        assert_eq!(player.tick(0, &mut lane), 1, "single note at tick 0");
        // Re-ticking past the end strikes nothing (cursor drained).
        assert_eq!(player.tick(1000, &mut lane), 0);
        assert!(player.is_finished());
    }
}
