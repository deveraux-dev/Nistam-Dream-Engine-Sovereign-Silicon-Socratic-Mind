//! NeuroDrop Arena-Ready integration tests (Spec: neurodrop-arena-ready).
//! Tests the forge-audio `Mixer` data layer directly — no egui, no GPU, no
//! audio device. This is the Phase-0 RED gate for the engine port: it goes RED
//! until the real `crate::mixer::Mixer` engine is live, and GREEN proves the
//! deck/hotcue/loop/beatgrid/sync/slip/keylock/sampler/mic data layer.
//!
//! Ported from `13engine/dreadpirateradio/tests/arena_test.rs` (2026-06-20).
//! The two app-level cases (a11 settings serde, a12 analyzer snapshot) tested
//! `dreadpirateradio::{settings,analyzer}` — a different crate not in NewRepo —
//! so they are intentionally NOT ported here (out of scope, not suppressed).

use forge_audio_v3::mixer::{Mixer, HotcueSlot, SyncMode};
use forge_audio_v3::mixer_cmd::{MixerCommand, apply_command};
use forge_audio_v3::dsp::AudioBuffer;
use forge_audio_v3::bpm::BeatGrid;

fn test_buf(samples: usize) -> AudioBuffer {
    AudioBuffer {
        samples: vec![vec![0.5f32; samples]],
        sample_rate: 44100,
    }
}

// ── A1: Hotcue Set and Jump ───────────────────────────────────────────────────

#[test]
fn a1_hotcue_set_and_jump() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100));

    apply_command(&mut mixer, MixerCommand::SetHotcue {
        deck: "a".into(), index: 0, position: Some(1000),
    });
    assert!(matches!(mixer.decks[0].hotcues[0], Some(HotcueSlot::Cue(1000))),
        "Hotcue 0 should be set at 1000");

    apply_command(&mut mixer, MixerCommand::SeekDeck { deck: "a".into(), position: 1000 });
    assert_eq!(mixer.decks[0].playback_pos, 1000);
}

#[test]
fn a1_hotcue_clear() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100));
    apply_command(&mut mixer, MixerCommand::SetHotcue { deck: "a".into(), index: 2, position: Some(5000) });
    assert!(mixer.decks[0].hotcues[2].is_some());
    apply_command(&mut mixer, MixerCommand::ClearHotcue { deck: "a".into(), index: 2 });
    assert!(mixer.decks[0].hotcues[2].is_none(), "Hotcue should be cleared");
}

// ── A2: Hotcue Loop ───────────────────────────────────────────────────────────

#[test]
fn a2_hotcue_loop_save() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100));
    apply_command(&mut mixer, MixerCommand::SetLoopRegion { deck: "a".into(), start: 5000, end: 10000 });
    apply_command(&mut mixer, MixerCommand::ToggleLoop { deck: "a".into() });
    assert!(mixer.decks[0].params.looping, "Looping must be enabled before SaveLoop");
    apply_command(&mut mixer, MixerCommand::SaveLoop { deck: "a".into(), slot: 1 });
    assert!(matches!(mixer.decks[0].hotcues[1], Some(HotcueSlot::Loop { start: 5000, end: 10000 })),
        "Hotcue 1 should be a loop from 5000 to 10000");
}

// ── A3: Beat Grid Phase Tracking ─────────────────────────────────────────────

#[test]
fn a3_beatgrid_phase_tracking() {
    let bg = BeatGrid::from_bpm(120.0, 0, 44100, 44100 * 10);
    assert_eq!(bg.beat_interval, 22050, "120 BPM at 44100Hz = 22050 samples/beat");
    assert_eq!(bg.beat_pos(0), 0);
    assert_eq!(bg.beat_pos(1), 22050);
    let phase = bg.phase_at(11025);
    assert!((phase - 0.5).abs() < 0.01, "Halfway through beat = phase 0.5");
}

#[test]
fn a3_beatgrid_snap_to_beat() {
    let bg = BeatGrid::from_bpm(120.0, 0, 44100, 44100 * 10);
    let snapped = bg.snap_to_beat(22100); // slightly past beat 1
    assert_eq!(snapped, 22050, "Should snap to beat 1 at 22050");
}

#[test]
fn a3_beatgrid_set_via_command() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100 * 10));
    apply_command(&mut mixer, MixerCommand::SetBpm { deck: "a".into(), bpm: 128.0 });
    assert_eq!(mixer.decks[0].bpm, Some(128.0));
    assert!(mixer.decks[0].beatgrid.is_some(), "Beatgrid should be set after SetBpm");
}

// ── A4: Sync Mode Transitions ─────────────────────────────────────────────────

#[test]
fn a4_sync_mode_leader() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::SetSync { deck: "a".into(), mode: "leader".into() });
    assert_eq!(mixer.decks[0].sync_mode, SyncMode::Leader);
}

#[test]
fn a4_sync_mode_follower() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::SetSync { deck: "b".into(), mode: "follower".into() });
    assert_eq!(mixer.decks[1].sync_mode, SyncMode::Follower);
}

#[test]
fn a4_sync_mode_off() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::SetSync { deck: "a".into(), mode: "leader".into() });
    apply_command(&mut mixer, MixerCommand::SetSync { deck: "a".into(), mode: "off".into() });
    assert_eq!(mixer.decks[0].sync_mode, SyncMode::Off);
}

// ── A5: Slip Mode ─────────────────────────────────────────────────────────────

#[test]
fn a5_slip_mode_toggle() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100));
    mixer.decks[0].params.playing = true;
    assert!(!mixer.decks[0].slip_active);
    apply_command(&mut mixer, MixerCommand::ToggleSlip { deck: "a".into() });
    assert!(mixer.decks[0].slip_active, "Slip should be active after toggle");
    apply_command(&mut mixer, MixerCommand::ToggleSlip { deck: "a".into() });
    assert!(!mixer.decks[0].slip_active, "Slip should be off after second toggle");
}

// ── A6: Keylock Toggle ────────────────────────────────────────────────────────

#[test]
fn a6_keylock_toggle() {
    let mut mixer = Mixer::default();
    assert!(!mixer.decks[0].params.keylock);
    apply_command(&mut mixer, MixerCommand::ToggleKeylock { deck: "a".into() });
    assert!(mixer.decks[0].params.keylock, "Keylock should be on");
    apply_command(&mut mixer, MixerCommand::ToggleKeylock { deck: "a".into() });
    assert!(!mixer.decks[0].params.keylock, "Keylock should be off");
}

// ── A7: Ghost Fire Viz Mode ───────────────────────────────────────────────────

#[test]
fn a7_ghost_fire_viz_mode_valid() {
    let ghost_fire_mode: usize = 5;
    assert!(ghost_fire_mode < 10, "Ghost Fire viz mode must be in 0-9 range");
    assert_eq!((ghost_fire_mode + 1) % 10, 6);
    assert_eq!((9 + 1) % 10, 0, "Mode 9 wraps to 0");
}

#[test]
fn a7_ghost_toggle_command() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::ToggleGhost);
    // No assertion — just verifying it doesn't panic.
}

// ── A8: Sampler Trigger ───────────────────────────────────────────────────────

#[test]
fn a8_sampler_trigger() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::LoadSampler {
        slot: 0,
        buffer: test_buf(44100),
    });
    apply_command(&mut mixer, MixerCommand::TriggerSampler { slot: 0 });
    assert!(mixer.samplers[0].playing, "Sampler slot 0 should be playing after trigger");
}

#[test]
fn a8_sampler_stop() {
    let mut mixer = Mixer::default();
    apply_command(&mut mixer, MixerCommand::LoadSampler { slot: 1, buffer: test_buf(44100) });
    apply_command(&mut mixer, MixerCommand::TriggerSampler { slot: 1 });
    assert!(mixer.samplers[1].playing);
    apply_command(&mut mixer, MixerCommand::StopSampler { slot: 1 });
    assert!(!mixer.samplers[1].playing, "Sampler should stop after StopSampler");
}

// ── A9: Talkover / Mic Duck ───────────────────────────────────────────────────

#[test]
fn a9_talkover_toggle() {
    let mut mixer = Mixer::default();
    assert!(!mixer.mic.active);
    apply_command(&mut mixer, MixerCommand::ToggleMic);
    assert!(mixer.mic.active, "Talkover should be active after ToggleMic");
    apply_command(&mut mixer, MixerCommand::ToggleMic);
    assert!(!mixer.mic.active, "Talkover should be off after second toggle");
}

// ── A10: Loop Half / Double ───────────────────────────────────────────────────

#[test]
fn a10_loop_resize_half() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100));
    apply_command(&mut mixer, MixerCommand::SetLoopRegion { deck: "a".into(), start: 0, end: 44100 });
    assert_eq!(mixer.decks[0].params.loop_end, 44100);
    apply_command(&mut mixer, MixerCommand::LoopResize { deck: "a".into(), direction: -1 });
    assert_eq!(mixer.decks[0].params.loop_end, 22050, "Loop should be halved to 22050");
}

#[test]
fn a10_loop_resize_double() {
    let mut mixer = Mixer::default();
    mixer.decks[0].load(test_buf(44100 * 4));
    apply_command(&mut mixer, MixerCommand::SetLoopRegion { deck: "a".into(), start: 0, end: 44100 });
    apply_command(&mut mixer, MixerCommand::LoopResize { deck: "a".into(), direction: 1 });
    assert_eq!(mixer.decks[0].params.loop_end, 88200, "Loop should be doubled to 88200");
}
