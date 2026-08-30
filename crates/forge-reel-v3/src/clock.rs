//! `ReelClock` — the DROP-LAW dwell clock, v3. Traced from v2's
//! `youtube-forgev1` skill note (`F:\NewRepo\.claude\skills\youtube-forgev1
//! \SKILL.md:65-83`): v2's `reel/clock.rs:28` hardcoded 300 BPM
//! (`bpm_x100: 30_000`) -> 50ms/column, uniform -- too fast for any DROP-LAW
//! dwell floor (13/100/300/500ms) to ever retain a frame, which was the
//! whole "nothing lands" defect. v2's own table named 30 BPM (500ms/column)
//! as the fix -- chant tempo, Gregorian's 40-60 BPM range.
//!
//! v3 does not reimplement a standalone bpm/fps knob: it maps directly onto
//! `EngineTick8`'s 120Hz carrier `frame` counter. `EngineTick8::encode` is a
//! pure function of `frame` (C14 firewall: integer-only, tick-stamped core),
//! so scrubbing to any column is a direct O(1) `encode` call, never a walk
//! forward from frame 0 -- the tape/tick substrate this crate rides already
//! carries replay for free via `RUN_STATE_REPLAY`.

use forge_engine_v3::{EngineTick8, RUN_STATE_REPLAY, RUN_STATE_RUN};

/// The 120Hz carrier `EngineTick8::frame` advances at (`ENGINE-SPINE-BRIEF.md`).
pub const CARRIER_HZ: u32 = 120;

/// DROP-LAW dwell floors, traced from `youtube-forgev1\SKILL.md:71`: below
/// `SEEN_MS` a frame is not perceived at all; the four floors are cumulative
/// evidence for how long a column must hold before it counts as "kept".
pub const SEEN_MS: u32 = 13;
/// A dwell long enough to read text on the frame.
pub const READ_MS: u32 = 100;
/// A dwell long enough to enter short-term memory.
pub const MEMORY_MS: u32 = 300;
/// A dwell long enough to be retained -- chant tempo, ~30 BPM, the v2-traced
/// fix for the 300 BPM/50ms defect (`SKILL.md:65-83`).
pub const KEPT_MS: u32 = 500;

/// One dwell-quantized reel clock. `dwell_ms` is the column width; every
/// `EngineTick8` frame maps to exactly one column via integer division, and
/// every column maps back to exactly one tick via direct `encode` -- no
/// state, no walk, both directions O(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReelClock {
    dwell_ms: u32,
}

impl ReelClock {
    /// A clock at the given dwell floor. `dwell_ms == 0` collapses to one
    /// frame per column (`frames_per_column` floors to 1, never 0 -- a
    /// zero-width column is a division-by-zero waiting to happen, refused
    /// at construction rather than at the first scrub).
    pub const fn new(dwell_ms: u32) -> Self {
        Self { dwell_ms }
    }

    /// The KEPT_MS clock -- the v2-traced fix, not the shipped 300 BPM
    /// defect. Callers porting `ReelClock::DROP` verbatim should reach for
    /// this constructor, not `new(50)`.
    pub const fn kept() -> Self {
        Self::new(KEPT_MS)
    }

    /// The authored column width in milliseconds. Read-only: a consumer that
    /// needs the dwell (the GIF lane converts it to centiseconds) asks the clock
    /// rather than keeping a second copy of it.
    pub const fn dwell_ms(&self) -> u32 {
        self.dwell_ms
    }

    /// How many 120Hz carrier frames one column holds. Floored to 1 so a
    /// `dwell_ms` below one carrier period (`1000/120 ≈ 8.3ms`) still
    /// advances instead of dividing by zero.
    pub const fn frames_per_column(&self) -> u32 {
        let fpc = (self.dwell_ms * CARRIER_HZ) / 1000;
        if fpc == 0 { 1 } else { fpc }
    }

    /// The column a tick's `frame` falls in.
    pub const fn column_at(&self, tick: EngineTick8) -> u32 {
        tick.frame / self.frames_per_column()
    }

    /// Scrub directly to a column's first frame, `RUN_STATE_RUN`. O(1): no
    /// walk from frame 0, `encode` is pure in `frame`.
    pub const fn seek(&self, column: u32) -> Option<EngineTick8> {
        let frame = column * self.frames_per_column();
        EngineTick8::encode(frame, RUN_STATE_RUN, forge_engine_v3::REGISTER_PURGATORIO)
    }

    /// Scrub in replay mode (`RUN_STATE_REPLAY`) -- the tape-scrub path: the
    /// caller is stepping through recorded columns rather than advancing
    /// the live clock.
    pub const fn scrub(&self, column: u32) -> Option<EngineTick8> {
        let frame = column * self.frames_per_column();
        EngineTick8::encode(frame, RUN_STATE_REPLAY, forge_engine_v3::REGISTER_PURGATORIO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_tempo_is_sixty_frames_per_column() {
        // 500ms * 120Hz / 1000 == 60 frames/column.
        assert_eq!(ReelClock::kept().frames_per_column(), 60);
    }

    #[test]
    fn v2_defect_tempo_is_six_frames_per_column() {
        // The shipped-but-defective 50ms/column: 6 frames, far under every
        // DROP-LAW floor at 120Hz (SEEN_MS=13 alone needs ~2 frames min).
        assert_eq!(ReelClock::new(50).frames_per_column(), 6);
    }

    #[test]
    fn zero_dwell_floors_to_one_frame() {
        assert_eq!(ReelClock::new(0).frames_per_column(), 1);
    }

    #[test]
    fn column_at_matches_seek_round_trip() {
        let clock = ReelClock::kept();
        for column in [0u32, 1, 2, 30, 1000] {
            let tick = clock.seek(column).expect("valid seek");
            assert_eq!(clock.column_at(tick), column);
        }
    }

    #[test]
    fn scrub_is_replay_state_seek_is_run_state() {
        let clock = ReelClock::kept();
        let run = clock.seek(5).unwrap();
        let replay = clock.scrub(5).unwrap();
        assert_eq!(run.frame, replay.frame);
        assert_ne!(run.mode, replay.mode);
    }

    #[test]
    fn scrub_is_direct_not_a_walk() {
        // Scrubbing column 10_000 costs the same as column 0 -- one encode
        // call, no intermediate frames touched. The test only asserts the
        // API shape (Option, no loop needed to reach it); the O(1) claim is
        // structural (encode is a pure fn of frame), not separately timed.
        let clock = ReelClock::kept();
        assert!(clock.scrub(10_000).is_some());
    }
}
