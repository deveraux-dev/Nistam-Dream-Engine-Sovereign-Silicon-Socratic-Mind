//! SynthXML semantic score types — engine-side only, no I/O.
//! Ported 2026-08-27 verbatim-in-behaviour from
//! F:\NewRepo\crates\forge-harmonics\src\synthxml.rs (deps inlined, docs added).

/// Which account a thread or event is charged against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AccountIndex(pub u8);

/// What kind of line a thread carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SynthThreadType {
    /// Struck, decaying.
    Bell,
    /// Sung line.
    FolkMelody,
    /// Held pedal.
    Drone,
    /// Observer commentary.
    Witness,
    /// Absence given a voice.
    Hollow,
    /// Navigation marker line.
    Route,
    /// Deliberate quiet.
    Silence,
    /// Beat-locked pulse.
    DjPulse,
    /// Announcement channel.
    Broadcast,
}

/// What a single event does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SynthEventKind {
    /// A pitched note.
    Note,
    /// A measured gap.
    Rest,
    /// Silence as content.
    Silence,
    /// A struck bell, pitched.
    BellToll,
    /// A raw audio primitive.
    Primitive,
    /// Phrase opening marker.
    PhraseStart,
    /// Phrase closing marker.
    PhraseEnd,
    /// Cadence marker.
    Cadence,
    /// Route waypoint marker.
    RouteMarker,
    /// Ledger checkpoint marker.
    LedgerMarker,
    /// First-lock hint marker.
    FirstLockHint,
}

/// The timbral family a primitive event draws from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AudioPrimitiveCategory {
    /// Nothing sounding.
    #[default]
    Silence,
    /// Struck metal, decaying.
    Bell,
    /// Human voice.
    Voice,
    /// Breath noise.
    Breath,
    /// Struck wood.
    Wood,
    /// Rope and rigging.
    Rope,
    /// Moving air.
    Wind,
    /// Dry particulate.
    Ash,
    /// Liquid.
    Water,
    /// Massed voices.
    Choir,
    /// Low fundamental.
    Root,
    /// Struck metal, ringing.
    Metal,
    /// Drum kick.
    Kick,
    /// Sub bass.
    Sub,
    /// Hi-hat.
    Hat,
    /// Hand clap.
    Clap,
    /// Vinyl surface noise.
    VinylHiss,
    /// Tape speed wobble.
    TapeDrift,
    /// Sweeping filter.
    FilterSweep,
    /// Delay repeats.
    DelayTail,
    /// Room tone.
    RoomRumble,
    /// Broadband noise.
    Static,
    /// Weather as voice.
    WeatherVoice,
}

/// One independent line in a score.
#[derive(Debug, Clone, Copy)]
pub struct SynthThread {
    /// Stable identifier.
    pub thread_id: u64,
    /// Hash of the authored name.
    pub name_hash: u64,
    /// What kind of line this is.
    pub thread_type: SynthThreadType,
    /// Account charged for this thread.
    pub account: AccountIndex,
    /// Loop length in music ticks, 0 for non-looping.
    pub loop_ticks: u64,
    /// Timing drift, permyriad (10_000 = 1.0).
    pub drift_q: i32,
}

/// One event on one thread.
#[derive(Debug, Clone, Copy)]
pub struct SynthEvent {
    /// Stable identifier.
    pub event_id: u64,
    /// Which thread this rides.
    pub thread_id: u64,
    /// What this event does.
    pub kind: SynthEventKind,
    /// Onset in music ticks.
    pub t_music: u64,
    /// Duration in music ticks.
    pub dur_music: u64,
    /// MIDI note, `None` for anything unpitched.
    pub pitch: Option<u8>,
    /// Strike velocity, permyriad (10_000 = 1.0).
    pub velocity_q: i32,
    /// Continuous pressure, permyriad.
    pub pressure_q: i32,
    /// Timbre position, permyriad.
    pub timbre_q: i32,
    /// Account charged for this event.
    pub account: AccountIndex,
    /// Determinism proof of the event's inputs.
    pub proof_hash: u64,
}

/// A whole born-symbolic score.
#[derive(Debug, Clone, Default)]
pub struct SynthScore {
    /// Stable identifier.
    pub score_id: u64,
    /// Hash of the source this was compiled from.
    pub source_hash: u64,
    /// Tempo in BPM times 100.
    pub tempo_bpm_x100: u32,
    /// Every line.
    pub threads: Vec<SynthThread>,
    /// Every event, any order.
    pub events: Vec<SynthEvent>,
    /// Determinism proof of the whole score.
    pub score_hash: u64,
}

/// Music-tick resolution: ticks per quarter note.
pub const MUSIC_TICKS_PER_QUARTER: u32 = 960;
/// The conductor's master tick rate.
pub const GAME_TICKS_PER_SECOND: u32 = 120;

/// Music-tick to game-tick conversion. Deterministic, integer-only.
pub fn music_ticks_to_game_ticks(music_ticks: u64, bpm_x100: u32) -> u64 {
    let game_ticks_per_quarter =
        (GAME_TICKS_PER_SECOND as u128 * 60u128 * 100u128) / bpm_x100.max(1) as u128;
    ((music_ticks as u128 * game_ticks_per_quarter) / MUSIC_TICKS_PER_QUARTER as u128) as u64
}

/// A note scheduled for playback on the conductor's audio lane: an absolute
/// 120 Hz game tick to fire at, plus what the synth voice needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduledNote {
    /// Absolute master tick (120 Hz) to strike on.
    pub fire_tick: u64,
    /// MIDI note number.
    pub note: u8,
    /// Strike velocity, 0..=127.
    pub vel: u8,
    /// Voice duration in milliseconds, at least 1.
    pub dur_ms: u32,
}

/// Lower a [`SynthScore`] into a tick-ordered playback plan. Note-bearing
/// events become [`ScheduledNote`]s; rests, silences and markers drop out.
/// Deterministic, integer-only.
pub fn score_to_note_plan(score: &SynthScore) -> Vec<ScheduledNote> {
    let bpm = score.tempo_bpm_x100;
    let mut plan: Vec<ScheduledNote> = Vec::with_capacity(score.events.len());
    for ev in &score.events {
        let note = match ev.pitch {
            Some(p) => p,
            None => continue,
        };
        if !matches!(ev.kind, SynthEventKind::Note | SynthEventKind::BellToll) {
            continue;
        }
        let fire_tick = music_ticks_to_game_ticks(ev.t_music, bpm);
        let dur_ticks = music_ticks_to_game_ticks(ev.dur_music, bpm);
        let dur_ms = ((dur_ticks * 1000) / GAME_TICKS_PER_SECOND as u64).max(1) as u32;
        let vel = ((ev.velocity_q.clamp(0, 10_000) as i64 * 127) / 10_000) as u8;
        plan.push(ScheduledNote { fire_tick, note, vel, dur_ms });
    }
    plan.sort_by_key(|n| n.fire_tick);
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_at_120_bpm_is_60_game_ticks() {
        assert_eq!(music_ticks_to_game_ticks(960, 12000), 60);
    }

    #[test]
    fn zero_bpm_does_not_panic() {
        let _ = music_ticks_to_game_ticks(960, 0);
    }

    fn note_ev(pitch: u8, t: u64, dur: u64, vel_q: i32) -> SynthEvent {
        SynthEvent {
            event_id: 0,
            thread_id: 1,
            kind: SynthEventKind::Note,
            t_music: t,
            dur_music: dur,
            pitch: Some(pitch),
            velocity_q: vel_q,
            pressure_q: 0,
            timbre_q: 0,
            account: AccountIndex(0),
            proof_hash: 0,
        }
    }

    #[test]
    fn note_plan_skips_rests_and_orders_by_tick() {
        let mut s = SynthScore { tempo_bpm_x100: 12_000, ..Default::default() };
        s.events.push(note_ev(67, 960, 480, 10_000));
        s.events.push(SynthEvent {
            kind: SynthEventKind::Rest,
            pitch: None,
            ..note_ev(0, 0, 480, 0)
        });
        s.events.push(note_ev(60, 0, 480, 5_000));
        let plan = score_to_note_plan(&s);
        assert_eq!(plan.len(), 2, "the rest carries no pitch and is skipped");
        assert_eq!((plan[0].fire_tick, plan[0].note), (0, 60), "downbeat sorts first");
        assert_eq!((plan[1].fire_tick, plan[1].note), (60, 67), "960 mt @120bpm = 60 game ticks");
        assert_eq!(plan[1].vel, 127, "10000 permyriad -> full MIDI velocity");
        assert!(plan[0].vel < plan[1].vel, "5000 permyriad is quieter than 10000");
        assert!(plan[0].dur_ms >= 1, "duration floored to >=1ms");
    }
}
