#![allow(clippy::disallowed_types)] // @forge:allow_alloc — cold-path module, init-time allocations permitted
// MusicMood dispatch — sieve-driven mood and director intensity routing.

use crate::bus::command_tx::AudioCommandTx;
use crate::dimensional_collapse::{collapse_5d_to_surround, Point5D, SurroundBus, REF_DIST_MU};
use crate::game_midi::MidiEvent;
use forge_hal::metronome::MetronomeClock;
use forge_harmonics::{loop_phase, AccountIndex, IronrootMidi2Event, LoopThread, RECOMMENDED_LOOP_SECS};

/// Sieve-driven music mood states. Each variant maps to a mixer preset name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicMood {
    Calm,
    Tension,
    Combat,
    Boss,
    Victory,
    Death,
    Exploration,
}

impl MusicMood {
    /// Return the mixer preset name for this mood.
    pub fn preset_name(self) -> &'static str {
        match self {
            MusicMood::Calm => "mood_calm",
            MusicMood::Tension => "mood_tension",
            MusicMood::Combat => "mood_combat",
            MusicMood::Boss => "mood_boss",
            MusicMood::Victory => "mood_victory",
            MusicMood::Death => "mood_death",
            MusicMood::Exploration => "mood_exploration",
        }
    }
}

/// Convert sieve MusicMood + DirectorIntensity into MixerCommands.
/// Called from CartridgeArena::tick() after sieve dispatch.
///
/// Sends:
/// 1. `MixerCommand::SetPreset` with mood-derived preset name
/// 2. `MixerCommand::Param("director_intensity", intensity)` clamped to [0.0, 1.0]
pub fn dispatch_music_mood(
    mood: MusicMood,
    intensity: f32,
    tx: &AudioCommandTx,
) {
    let clamped = intensity.clamp(0.0, 1.0);
    tx.set_preset(mood.preset_name(), clamped);
    tx.set_param("director_intensity", clamped);
}

// ── Layered bed — which loop lanes a mood plays, and when they wrap ──────────
// Ported from the dirge director (E:/.airgap/dirge-sprite-wire-2026-06-05-1210/
// src/director.rs:68-95). The ladder shape is verbatim: four co-prime lanes in
// stable order 0=base 1=rhythm 2=tension 3=chase, cumulative per mood. The
// quarry enum had 4 moods; this crate's MusicMood has 7, so the 7→4 assignment
// below is a mapping onto that ladder, not a ported table.

/// The four phase-locked lanes, in game ticks. Lengths are the first four
/// [`RECOMMENDED_LOOP_SECS`] (29/43/61/97 — pairwise coprime, so the lanes never
/// re-phase into an audible short repeat). Stable order: lane `i` here is lane
/// `i` in [`layer_mask`] and [`layer_edges`]. Counted in ticks of the one master
/// clock, [`MetronomeClock::TICK_HZ`] — the bed phase-locks to the same tick the
/// metronome runs on, which is the whole point of the seam.
pub fn layer_threads() -> [LoopThread; 4] {
    core::array::from_fn(|i| {
        LoopThread::new(
            i as u64,
            RECOMMENDED_LOOP_SECS[i] as u64 * MetronomeClock::TICK_HZ,
            AccountIndex(i as u8),
        )
    })
}

/// Which lanes a mood plays. Cumulative: every mood holds the base lane, and
/// heavier moods stack rhythm, tension and chase on top of it.
pub const fn layer_mask(mood: MusicMood) -> [bool; 4] {
    match mood {
        // Near-silence — base drone only.
        MusicMood::Calm | MusicMood::Death => [true, false, false, false],
        // Walking the world.
        MusicMood::Exploration => [true, true, false, false],
        // Something is coming.
        MusicMood::Tension => [true, true, true, false],
        // Full stack.
        MusicMood::Combat | MusicMood::Boss | MusicMood::Victory => [true, true, true, true],
    }
}

/// Which ACTIVE lanes cross their loop boundary at `game_tick` — the seam the
/// layered bed reads to (re)fire a voice. Lane `i` is `true` iff the mood plays
/// lane `i` AND its phase wraps to 0 on this tick. Pure, integer, no alloc.
pub fn layer_edges(threads: &[LoopThread; 4], mood: MusicMood, game_tick: u64) -> [bool; 4] {
    let mask = layer_mask(mood);
    core::array::from_fn(|i| mask[i] && loop_phase(&threads[i], game_tick) == 0)
}

/// The firing lanes as MIDI 2.0 events — the semantic carrier the audio layer
/// already speaks ([`IronrootMidi2Event`]), rather than a bare bool array.
///
/// Only the facts the seam actually knows are filled: which lane fired
/// (`thread_id`/`account`), when (`t_game`), and for how long the lane runs
/// (`dur_game` = its loop length). `note`/`pitch_32`/`velocity_32` stay 0 —
/// UNVOICED. Pitch is cartridge data, bound through
/// [`InstrumentBase::note_for_state`](forge_harmonics::instrument_base::InstrumentBase::note_for_state),
/// and T3 fills it when the real voices load. Inventing a lane→pitch table here
/// would put theory in the engine, which is exactly where it must not live.
pub fn layer_edge_events(
    threads: &[LoopThread; 4],
    mood: MusicMood,
    game_tick: u64,
) -> [Option<IronrootMidi2Event>; 4] {
    let edges = layer_edges(threads, mood, game_tick);
    core::array::from_fn(|i| {
        edges[i].then(|| IronrootMidi2Event {
            event_id: game_tick.wrapping_mul(4).wrapping_add(i as u64),
            thread_id: threads[i].thread_id,
            t_game: game_tick,
            dur_game: threads[i].loop_len_game,
            account: threads[i].account,
            ..IronrootMidi2Event::default()
        })
    })
}

/// The firing lanes as 5D source points, ready for
/// [`collapse_5d_to_stereo`](crate::dimensional_collapse::collapse_5d_to_stereo).
///
/// The bed's own facts map straight onto the axes the collapse already defines
/// (`dimensional_collapse.rs:8-13`): `W` is chrono-tick lineage, so it takes the
/// game tick; `θ` is the harmonic-codeword angle, so it takes the lane's own
/// position around its loop, one full turn per cycle; `Z` is semantic depth →
/// root note, so it takes the lane index — base is the root and each lane above
/// sits one scale degree up.
///
/// `X`/`Y` are spatial and the bed has no world position: they sit at centre and
/// the reference distance. Those two lanes are UNEXERCISED (root#rank) until a
/// caller gives the bed a place to sound from.
pub fn layer_edge_points(
    threads: &[LoopThread; 4],
    mood: MusicMood,
    game_tick: u64,
) -> [Option<Point5D>; 4] {
    layer_edge_points_at(BED_AT_LISTENER, threads, mood, game_tick)
}

/// The bed with no place to sound from: centred, at the reference distance, at
/// the root degree. X/Y here are UNEXERCISED lanes (root#rank) — a caller that
/// knows where the bed lives passes its own anchor to [`layer_edge_points_at`].
pub const BED_AT_LISTENER: Point5D =
    Point5D { x_mu: 0, y_mu: REF_DIST_MU, z_semantic: 0, w_tick: 0, theta_mdeg: 0 };

/// The firing lanes as 5D points seated on `anchor` — the bed's place in the
/// world. X/Y come from the anchor untouched (where it sounds from); Z is the
/// anchor's degree plus the lane index (the lanes stack upward from wherever the
/// anchor sits); W and θ stay the bed's own clock facts.
///
/// A GhostMoon impulse is a legal anchor: its `layer_z` rides the same semantic-z
/// range by construction (`forge_ml::nearest_neighbor::lambda_z_family_to_layer`),
/// and `embed_cree_cell` fills real x/y — so an anchored bed is a GhostMoon-placed
/// bed, not a centred default.
pub fn layer_edge_points_at(
    anchor: Point5D,
    threads: &[LoopThread; 4],
    mood: MusicMood,
    game_tick: u64,
) -> [Option<Point5D>; 4] {
    let edges = layer_edges(threads, mood, game_tick);
    core::array::from_fn(|i| {
        edges[i].then(|| {
            let len = threads[i].loop_len_game.max(1);
            let phase = (loop_phase(&threads[i], game_tick) * 360_000 / len) as i32;
            Point5D {
                x_mu: anchor.x_mu,
                y_mu: anchor.y_mu,
                z_semantic: anchor.z_semantic + i as i32,
                w_tick: game_tick,
                theta_mdeg: (anchor.theta_mdeg + phase).rem_euclid(360_000),
            }
        })
    })
}

/// Sound the bed: every lane wrapping at `game_tick` collapses through the 5D
/// decoder and strikes the live 5.1 bus. Returns how many lanes fired.
///
/// This is the audible path — no cartridge data, no MIDI hop, no invented pitch:
/// the lane's own geometry IS the voice, via
/// [`collapse_5d_to_surround`](crate::dimensional_collapse::collapse_5d_to_surround).
pub fn strike_bed_layers(
    bus: &mut SurroundBus,
    threads: &[LoopThread; 4],
    mood: MusicMood,
    game_tick: u64,
    sample_rate: u32,
) -> usize {
    let mut fired = 0;
    for point in layer_edge_points(threads, mood, game_tick).into_iter().flatten() {
        bus.strike(collapse_5d_to_surround(point, sample_rate));
        fired += 1;
    }
    fired
}

/// Downgrade one bed edge to a MIDI 1.0 NoteOn for the sequencer lane, which
/// speaks raw status/data bytes ([`crate::game_midi::MidiEvent`]).
///
/// `note` and `velocity` are supplied by the caller from cartridge data
/// (`InstrumentBase::note_for_state` → [`InstrumentNote::velocity_8`]) — this
/// function never picks a pitch. Channel is the lane's account, so lane `i`
/// lands on channel `i`; byte layout matches the parser at
/// `forge_harmonics::ump::midi1_channel_voice` (status 0x9 = NoteOn).
pub fn bed_note_on(event: &IronrootMidi2Event, note: u8, velocity: u8) -> MidiEvent {
    MidiEvent {
        sample_offset: 0,
        status: 0x90 | (event.account.0 & 0x0F),
        data: [note & 0x7F, velocity & 0x7F],
        jack_frame: event.t_game,
    }
}

#[cfg(test)]
mod layer_tests {
    use super::*;

    const HZ: u64 = MetronomeClock::TICK_HZ;

    #[test]
    fn lanes_are_the_coprime_lengths_in_ticks() {
        let t = layer_threads();
        for (i, thread) in t.iter().enumerate() {
            assert_eq!(thread.loop_len_game, RECOMMENDED_LOOP_SECS[i] as u64 * HZ);
            assert_eq!(thread.thread_id, i as u64);
        }
    }

    #[test]
    fn mask_is_cumulative_from_the_base_lane() {
        for mood in [
            MusicMood::Calm, MusicMood::Death, MusicMood::Exploration,
            MusicMood::Tension, MusicMood::Combat, MusicMood::Boss, MusicMood::Victory,
        ] {
            let m = layer_mask(mood);
            assert!(m[0], "{mood:?} must hold the base lane");
            for i in 1..4 {
                assert!(!m[i] || m[i - 1], "{mood:?} lane {i} played without lane {}", i - 1);
            }
        }
    }

    #[test]
    fn tick_zero_fires_every_played_lane() {
        let t = layer_threads();
        assert_eq!(layer_edges(&t, MusicMood::Victory, 0), [true, true, true, true]);
        assert_eq!(layer_edges(&t, MusicMood::Calm, 0), [true, false, false, false]);
    }

    #[test]
    fn a_lane_fires_only_on_its_own_wrap() {
        let t = layer_threads();
        let base = RECOMMENDED_LOOP_SECS[0] as u64 * HZ;
        assert_eq!(layer_edges(&t, MusicMood::Victory, base), [true, false, false, false]);
        assert_eq!(layer_edges(&t, MusicMood::Victory, base - 1), [false; 4]);
    }

    #[test]
    fn events_carry_lane_identity_timing_and_no_invented_pitch() {
        let t = layer_threads();
        let ev = layer_edge_events(&t, MusicMood::Victory, 0);
        for (i, slot) in ev.iter().enumerate() {
            let e = slot.expect("every Victory lane fires at tick 0");
            assert_eq!(e.thread_id, i as u64);
            assert_eq!(e.account, t[i].account);
            assert_eq!(e.t_game, 0);
            assert_eq!(e.dur_game, t[i].loop_len_game);
            assert_eq!(e.note, 0, "pitch is cartridge data, never invented here");
            assert_eq!(e.velocity_32, 0);
        }
    }

    #[test]
    fn events_match_the_edges_they_come_from() {
        let t = layer_threads();
        for tick in [0u64, 1, 3480, RECOMMENDED_LOOP_SECS[1] as u64 * HZ] {
            let edges = layer_edges(&t, MusicMood::Combat, tick);
            let events = layer_edge_events(&t, MusicMood::Combat, tick);
            for i in 0..4 {
                assert_eq!(edges[i], events[i].is_some(), "lane {i} disagrees at tick {tick}");
            }
        }
    }

    #[test]
    fn note_on_carries_the_lane_channel_and_the_caller_pitch() {
        let t = layer_threads();
        let ev = layer_edge_events(&t, MusicMood::Tension, 0);
        let lane2 = ev[2].expect("Tension plays lane 2");
        let m = bed_note_on(&lane2, 67, 100);
        assert_eq!(m.status, 0x92, "lane 2 -> channel 2 NoteOn");
        assert_eq!(m.data, [67, 100]);
        assert_eq!(m.jack_frame, 0);
    }

    #[test]
    fn note_on_masks_out_of_range_bytes() {
        let t = layer_threads();
        let lane0 = layer_edge_events(&t, MusicMood::Calm, 0)[0].expect("base lane fires");
        let m = bed_note_on(&lane0, 200, 255);
        assert_eq!(m.data, [72, 127], "note/velocity clamped into 7-bit MIDI range");
    }

    #[test]
    fn edge_points_seat_the_bed_facts_on_the_collapse_axes() {
        use crate::dimensional_collapse::collapse_5d_to_stereo;
        let t = layer_threads();
        let pts = layer_edge_points(&t, MusicMood::Victory, 0);
        for (i, slot) in pts.iter().enumerate() {
            let p = slot.expect("every Victory lane fires at tick 0");
            assert_eq!(p.z_semantic, i as i32, "lane index is the semantic depth");
            assert_eq!(p.w_tick, 0);
            assert_eq!(p.theta_mdeg, 0, "a firing lane is at phase 0 by definition");
            assert_eq!(p.y_mu, REF_DIST_MU);
        }
        // Z climbing the lanes must climb the pitch — the collapse owns that map.
        let mut prev = 0i64;
        for slot in pts.iter() {
            let f = collapse_5d_to_stereo(slot.unwrap(), 48_000).root_freq_mhz;
            assert!(f > prev, "lane pitch must rise, got {f} after {prev}");
            prev = f;
        }
    }

    #[test]
    fn edge_points_agree_with_the_edges() {
        let t = layer_threads();
        let tick = RECOMMENDED_LOOP_SECS[0] as u64 * HZ;
        let edges = layer_edges(&t, MusicMood::Tension, tick);
        let pts = layer_edge_points(&t, MusicMood::Tension, tick);
        for i in 0..4 {
            assert_eq!(edges[i], pts[i].is_some(), "lane {i} disagrees");
        }
    }

    #[test]
    fn striking_the_bed_puts_audible_signal_on_the_bus() {
        const SR: u32 = 48_000;
        let t = layer_threads();
        let mut bus = SurroundBus::new(SR);
        assert!(!bus.is_active(), "a fresh bus is silent");

        assert_eq!(strike_bed_layers(&mut bus, &t, MusicMood::Exploration, 0, SR), 2);
        assert!(bus.is_active(), "two lanes struck, the bus must be ringing");

        let mut out = [0.0f32; 64 * 2];
        bus.render_block(&mut out, 2, 64);
        assert!(out.iter().any(|s| *s != 0.0), "a struck bed must render non-silence");
    }

    #[test]
    fn between_boundaries_the_bed_strikes_nothing() {
        const SR: u32 = 48_000;
        let t = layer_threads();
        let mut bus = SurroundBus::new(SR);
        assert_eq!(strike_bed_layers(&mut bus, &t, MusicMood::Victory, 1, SR), 0);
        assert!(!bus.is_active());
    }

    /// An anchored bed sounds FROM somewhere: the same lanes, panned and
    /// attenuated by the anchor's place, and lifted to its degree.
    #[test]
    fn an_anchor_places_the_bed_in_the_field() {
        use crate::dimensional_collapse::collapse_5d_to_stereo;
        const SR: u32 = 48_000;
        let t = layer_threads();
        let anchor = Point5D { x_mu: -8_000, y_mu: 9_000, z_semantic: 5, w_tick: 0, theta_mdeg: 90_000 };

        let centred = layer_edge_points(&t, MusicMood::Calm, 0)[0].unwrap();
        let placed = layer_edge_points_at(anchor, &t, MusicMood::Calm, 0)[0].unwrap();

        assert_eq!(placed.x_mu, anchor.x_mu, "X is the anchor's, untouched");
        assert_eq!(placed.z_semantic, 5, "lane 0 sits at the anchor's degree");
        assert_eq!(placed.theta_mdeg, 90_000, "phase 0 rides the anchor angle");
        assert_eq!(centred.x_mu, 0, "unanchored stays at the listener");

        let a = collapse_5d_to_stereo(placed, SR);
        let c = collapse_5d_to_stereo(centred, SR);
        assert!(a.pan_pmy < c.pan_pmy, "an off-centre anchor must pan");
        assert!(a.gain_pmy < c.gain_pmy, "a further anchor must attenuate");
        assert!(a.root_freq_mhz > c.root_freq_mhz, "a higher degree must raise pitch");
    }

    #[test]
    fn a_silent_lane_never_fires_even_on_its_wrap() {
        let t = layer_threads();
        let chase = RECOMMENDED_LOOP_SECS[3] as u64 * HZ;
        assert_eq!(layer_edges(&t, MusicMood::Victory, chase)[3], true);
        assert_eq!(layer_edges(&t, MusicMood::Calm, chase)[3], false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::command::MixerCommand;
    use proptest::prelude::*;

    #[test]
    fn preset_names_are_correct() {
        assert_eq!(MusicMood::Calm.preset_name(), "mood_calm");
        assert_eq!(MusicMood::Tension.preset_name(), "mood_tension");
        assert_eq!(MusicMood::Combat.preset_name(), "mood_combat");
        assert_eq!(MusicMood::Boss.preset_name(), "mood_boss");
        assert_eq!(MusicMood::Victory.preset_name(), "mood_victory");
        assert_eq!(MusicMood::Death.preset_name(), "mood_death");
        assert_eq!(MusicMood::Exploration.preset_name(), "mood_exploration");
    }

    #[test]
    fn dispatch_sends_set_preset_and_param() {
        let (raw_tx, rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(raw_tx);

        dispatch_music_mood(MusicMood::Combat, 0.75, &tx);

        let cmds: Vec<MixerCommand> = rx.try_iter().collect();
        assert_eq!(cmds.len(), 2);

        // First command: SetPreset
        match &cmds[0] {
            MixerCommand::SetPreset { name, intensity } => {
                assert_eq!(name, "mood_combat");
                assert!((intensity - 0.75).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetPreset, got something else"),
        }

        // Second command: Param("director_intensity")
        match &cmds[1] {
            MixerCommand::Param { target, value } => {
                assert_eq!(target, "director_intensity");
                assert!((value - 0.75).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Param, got something else"),
        }
    }

    #[test]
    fn dispatch_clamps_intensity_above_one() {
        let (raw_tx, rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(raw_tx);

        dispatch_music_mood(MusicMood::Boss, 2.5, &tx);

        let cmds: Vec<MixerCommand> = rx.try_iter().collect();
        assert_eq!(cmds.len(), 2);

        match &cmds[0] {
            MixerCommand::SetPreset { intensity, .. } => {
                assert!((intensity - 1.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetPreset"),
        }
        match &cmds[1] {
            MixerCommand::Param { value, .. } => {
                assert!((value - 1.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Param"),
        }
    }

    #[test]
    fn dispatch_clamps_intensity_below_zero() {
        let (raw_tx, rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(raw_tx);

        dispatch_music_mood(MusicMood::Death, -0.5, &tx);

        let cmds: Vec<MixerCommand> = rx.try_iter().collect();
        assert_eq!(cmds.len(), 2);

        match &cmds[0] {
            MixerCommand::SetPreset { intensity, .. } => {
                assert!((intensity - 0.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetPreset"),
        }
        match &cmds[1] {
            MixerCommand::Param { value, .. } => {
                assert!((value - 0.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Param"),
        }
    }

    // ── 6.2: Property 12 — MusicMood Dispatch Completeness ──────────────

    /// Strategy for arbitrary MusicMood variant.
    fn arb_music_mood() -> impl Strategy<Value = MusicMood> {
        prop_oneof![
            Just(MusicMood::Calm),
            Just(MusicMood::Tension),
            Just(MusicMood::Combat),
            Just(MusicMood::Boss),
            Just(MusicMood::Victory),
            Just(MusicMood::Death),
            Just(MusicMood::Exploration),
        ]
    }

    // **Validates: Requirements 7.1, 7.2**
    //
    // Property 12: For arbitrary (MusicMood, intensity in [0.0, 1.0]),
    // verify exactly one SetPreset and one Param("director_intensity") sent,
    // the SetPreset name matches mood.preset_name(), and the Param value
    // matches the clamped intensity.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn prop_music_mood_dispatch_completeness(
            mood in arb_music_mood(),
            intensity in 0.0f32..=1.0f32,
        ) {
            let (raw_tx, rx) = crossbeam_channel::unbounded();
            let tx = AudioCommandTx::new(raw_tx);

            dispatch_music_mood(mood, intensity, &tx);

            let cmds: Vec<MixerCommand> = rx.try_iter().collect();

            // Exactly 2 commands total
            prop_assert_eq!(cmds.len(), 2, "Expected exactly 2 commands, got {}", cmds.len());

            // Exactly one SetPreset
            let set_presets: Vec<&MixerCommand> = cmds.iter().filter(|c| {
                matches!(c, MixerCommand::SetPreset { .. })
            }).collect();
            prop_assert_eq!(set_presets.len(), 1, "Expected exactly 1 SetPreset, got {}", set_presets.len());

            // Exactly one Param("director_intensity")
            let dir_params: Vec<&MixerCommand> = cmds.iter().filter(|c| {
                matches!(c, MixerCommand::Param { target, .. } if target == "director_intensity")
            }).collect();
            prop_assert_eq!(dir_params.len(), 1, "Expected exactly 1 director_intensity Param, got {}", dir_params.len());

            // SetPreset name matches mood.preset_name()
            match set_presets[0] {
                MixerCommand::SetPreset { name, intensity: preset_intensity } => {
                    prop_assert_eq!(name.as_str(), mood.preset_name(),
                        "SetPreset name mismatch: expected {}, got {}", mood.preset_name(), name);
                    prop_assert!((preset_intensity - intensity).abs() < f32::EPSILON,
                        "SetPreset intensity mismatch: expected {}, got {}", intensity, preset_intensity);
                }
                _ => unreachable!(),
            }

            // Param value matches the clamped intensity
            match dir_params[0] {
                MixerCommand::Param { value, .. } => {
                    prop_assert!((value - intensity).abs() < f32::EPSILON,
                        "director_intensity value mismatch: expected {}, got {}", intensity, value);
                }
                _ => unreachable!(),
            }
        }
    }
}
