//! MixerCommand — lock-free command channel for the audio worker thread.

use crate::dsp::AudioBuffer;
use crate::mixer::{Mixer, DeckId};

/// Stem component identifier.
#[derive(Debug, Clone, Copy)]
pub enum StemPart {
    Drums,
    Vocal,
    Instruments,
}

/// Parse a deck identifier string ("a", "b", "c", or "d") into a DeckId.
// SF-005: LOCKED — unknown deck string now logs loudly instead of returning None silently.
// New callers should use forge_audio::params::parse_deck_id() and propagate the Result;
// this shim keeps the 30 legacy callsites inside mixer_cmd.rs working.
// See forge-audio/src/params.rs::tests::sf_005_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
fn deck_id_from_str(deck: &str) -> Option<DeckId> {
    match deck {
        "a" => Some(DeckId::A),
        "b" => Some(DeckId::B),
        "c" => Some(DeckId::C),
        "d" => Some(DeckId::D),
        _ => {
            eprintln!("[mixer_cmd] SF-005: unrecognized deck {:?} — expected a, b, c, or d", deck);
            None
        }
    }
}

/// GUARDRAIL: This is the ONLY way to mutate mixer state.
/// Send commands via the crossbeam channel. The feeder thread owns the Mixer
/// and calls drain_commands() each cycle. Never lock the Mixer directly.
pub enum MixerCommand {
    /// Set a named parameter (from controller mapping).
    Param { target: String, value: f32 },
    /// SF-001: LOCKED — typed parameter variant. External IPC callers must
    /// parse through MixerParam::from_str and send this variant. No string fallthrough possible.
    /// See docs/plans/2026-04-09-ipc-silent-failure-audit.md.
    ParamTyped { param: crate::params::MixerParam, value: f32 },
    /// Trigger an action (play/pause, cue, stem toggle).
    Action { target: String },
    /// SF-002: LOCKED — typed action variant. External IPC callers must parse through
    /// MixerAction::from_str and send this variant. No string fallthrough possible.
    /// See docs/plans/2026-04-09-ipc-silent-failure-audit.md.
    ActionTyped { action: crate::params::MixerAction },
    /// Load audio onto a deck.
    ///
    /// `title` / `artist` are the display strings shown in the deck UI.
    /// Callers are responsible for resolving them (DB tags first, filename
    /// parser fallback). Empty strings render as "no metadata" in the UI.
    LoadDeck { deck: String, buffer: AudioBuffer, title: String, artist: String },
    /// Set horror preset on master bus.
    SetPreset { name: String, intensity: f32 },
    /// Set FX slot preset.
    SetFxSlot { slot: usize, preset: String },
    /// Toggle a stem mute on a deck.
    StemToggle { deck: String, stem: StemPart },
    /// Toggle FX slot enabled state.
    ToggleFx { slot: usize },
    /// Set FX slot intensity.
    SetFxIntensity { slot: usize, intensity: f32 },
    /// Request: take the recording buffer (for export). Response sent back on channel.
    TakeRecording { resp: crossbeam_channel::Sender<Option<AudioBuffer>> },
    /// Request: clone all 4 deck buffers. Response sent back on channel.
    GetDeckBuffers { resp: crossbeam_channel::Sender<[Option<AudioBuffer>; 4]> },
    /// Smoothly ramp crossfader to target over the given duration.
    CrossfadeRamp { target: f32, seconds: f32 },
    /// Set hotcue at index for a deck (position = sample index, or None to toggle)
    SetHotcue { deck: String, index: usize, position: Option<usize> },
    /// Clear a hotcue
    ClearHotcue { deck: String, index: usize },
    /// Toggle loop on a deck
    ToggleLoop { deck: String },
    /// Set loop region (sample indices)
    SetLoopRegion { deck: String, start: usize, end: usize },
    /// Seek deck to sample position
    SeekDeck { deck: String, position: usize },
    // StartBroadcast/StopBroadcast: EXCLUDED — need crate::broadcast (excluded,
    // unsafe impl Send, real unsafe-code law conflict).
    /// Jog wheel nudge (delta in samples)
    JogNudge { deck: String, delta: isize },
    /// Set BPM for a deck (from background BPM detection)
    SetBpm { deck: String, bpm: f32 },
    /// Set musical key for a deck (from background key detection)
    SetKey { deck: String, key: String },
    /// Set genre for a deck (from background genre detection)
    SetGenre { deck: String, genre: u8 },
    /// Set playing state explicitly (not toggle)
    SetPlaying { deck: String, playing: bool },
    /// Toggle AutoDJ on/off. Broski or any channel sender can use this.
    ToggleAutoDj { enabled: bool },
    /// Toggle slip mode. On: start shadow position. Off: snap playback to shadow.
    ToggleSlip { deck: String },
    /// Save current loop region to a hotcue slot.
    SaveLoop { deck: String, slot: usize },
    /// Set ReplayGain for a deck (from analysis).
    SetReplayGain { deck: String, gain_db: f32 },
    /// Set sync mode for a deck (off/follower/leader).
    SetSync { deck: String, mode: String },
    /// Toggle pre-fader listen (headphone cue) for a deck.
    TogglePfl { deck: String },
    /// Toggle keylock (pitch-independent tempo).
    ToggleKeylock { deck: String },
    /// Set scratch rate (vinyl jog wheel).
    Scratch { deck: String, rate: f32 },
    /// Set scratching state (platter touch).
    SetScratching { deck: String, active: bool },
    /// Toggle reverse playback for a deck.
    ToggleReverse { deck: String },
    /// Toggle mic active state.
    ToggleMic,
    /// Toggle mic monitor (loopback): route mic into the headphone bus.
    /// Independent of `ToggleMic` so the monitor can be on while the mic is muted to master.
    ToggleMicMonitor,
    /// Toggle envelope-driven auto-duck. When on, master is sidechain-ducked by mic level.
    /// Overrides the static talkover duck path when armed.
    ToggleAutoDuck,
    /// Toggle Ghost Fire rendering.
    ToggleGhost,
    /// Load audio into a sampler slot (0-7).
    LoadSampler { slot: usize, buffer: AudioBuffer },
    /// Trigger a sampler pad.
    TriggerSampler { slot: usize },
    /// Stop a sampler pad.
    StopSampler { slot: usize },
    /// Assign/unassign an FX slot to a deck.
    SetFxAssign { deck: String, fx_slot: usize, enabled: bool },
    /// Scroll the library browser (positive = down, negative = up).
    LibraryScroll { delta: i8 },
    /// Load the highlighted library track onto the focused deck.
    LibraryLoadHighlighted { deck: String },
    /// Flush a deck's playback state — instant silence, no buffer drain.
    FlushDeck { deck: String },
    /// Hot-switch audio output device without restarting the app.
    SwitchAudioDevice { device: String },
    /// Halve or double loop region. direction: -1 = halve, +1 = double.
    LoopResize { deck: String, direction: i8 },
    /// Beat jump: move playback position by exact beat multiples.
    BeatJump { deck: String, beats: f64 },
    /// Loop roll start: save slip position, enter loop.
    LoopRollStart { deck: String },
    /// Loop roll stop: return to slip position + elapsed.
    LoopRollStop { deck: String },
    /// Toggle heal plugin active state.
    HealToggle,
    /// Set heal intensity (0.0-1.0).
    HealIntensity { intensity: f32 },
    /// Set heal mode (0=alpha, 1=beta, 2=theta, 3=gamma).
    HealMode { mode: u8 },
    /// Shift beatgrid by offset samples (positive = right, negative = left).
    ShiftBeatgrid { deck: String, offset: i64 },
    /// Manually set BPM and rebuild beatgrid.
    SetBpmManual { deck: String, bpm: f32 },
    /// Halve or double detected BPM.
    BpmMultiply { deck: String, factor: f32 },
    /// SF-017: LOCKED — surfaces a decode error from the spawned load thread into the mixer
    /// snapshot. UI polls the snapshot and sees the error instead of a silent empty deck.
    /// See docs/plans/2026-04-09-ipc-silent-failure-audit.md.
    DeckLoadFailed { deck: String, error: String },
    /// Set crossfader curve shape: "smooth" or "sharp".
    SetCrossfaderCurve { curve: String },
    /// Ambient weather modulation (integer-only payload, f32 conversion on audio thread).
    /// Permyriad: 0 = none, 10000 = maximum.
    AmbientWeather { rain_permyriad: u16, wind_permyriad: u16, fog_permyriad: u16 },
}

/// Apply a single command to the mixer.
pub fn apply_command(mixer: &mut Mixer, cmd: MixerCommand) {
    match cmd {
        MixerCommand::Param { target, value } => {
            mixer.apply_param(&target, value);
        }
        // SF-001: LOCKED — typed path, no string fallthrough possible.
        MixerCommand::ParamTyped { param, value } => {
            mixer.apply_param_typed(param, value);
        }
        MixerCommand::Action { target } => {
            mixer.apply_action(&target);
        }
        // SF-002: LOCKED — typed path, no string fallthrough possible.
        MixerCommand::ActionTyped { action } => {
            mixer.apply_action_typed(action);
        }
        MixerCommand::LoadDeck { deck, buffer, title, artist } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.load(buffer);
                d.title = title;
                d.artist = artist;
                // Clear any previous load error for this deck on successful load.
                mixer.deck_load_errors[deck_id as usize] = None;
            } else {
                eprintln!("[mixer_cmd] Unknown deck: {}", deck);
            }
        }
        MixerCommand::SetPreset { name, intensity } => {
            if let Err(e) = mixer.set_preset(&name, intensity) {
                eprintln!("[mixer_cmd] Preset error: {}", e);
            }
        }
        MixerCommand::SetFxSlot { slot, preset } => {
            mixer.set_fx_slot(slot, &preset);
        }
        MixerCommand::ToggleFx { slot } => {
            let was = if slot < 4 { mixer.fx_slots[slot].enabled } else { false };
            mixer.toggle_fx_slot(slot);
            let now = if slot < 4 { mixer.fx_slots[slot].enabled } else { false };
            eprintln!("[FX] ToggleFx slot={} {} -> {}", slot, was, now);
        }
        MixerCommand::SetFxIntensity { slot, intensity } => {
            mixer.set_fx_intensity(slot, intensity);
        }
        MixerCommand::StemToggle { deck, stem } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                match stem {
                    StemPart::Drums => d.stems.drums_muted = !d.stems.drums_muted,
                    StemPart::Vocal => d.stems.vocal_muted = !d.stems.vocal_muted,
                    StemPart::Instruments => d.stems.instruments_muted = !d.stems.instruments_muted,
                }
            }
        }
        MixerCommand::TakeRecording { resp } => {
            let buf = mixer.take_recording();
            let _ = resp.send(buf);
        }
        MixerCommand::GetDeckBuffers { resp } => {
            let buffers = [
                mixer.decks[0].buffer.clone(),
                mixer.decks[1].buffer.clone(),
                mixer.decks[2].buffer.clone(),
                mixer.decks[3].buffer.clone(),
            ];
            let _ = resp.send(buffers);
        }
        MixerCommand::SetHotcue { deck, index, position } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if index >= 8 { return; }
                match position {
                    Some(pos) => {
                        let snapped = d.snap_if_quantized(pos);
                        d.hotcues[index] = Some(crate::mixer::HotcueSlot::Cue(snapped));
                    }
                    None => {
                        match d.hotcues[index] {
                            Some(crate::mixer::HotcueSlot::Cue(pos)) => {
                                d.playback_pos = pos;
                            }
                            Some(crate::mixer::HotcueSlot::Loop { start, end }) => {
                                d.playback_pos = start;
                                d.params.loop_start = start;
                                d.params.loop_end = end;
                                d.params.looping = true;
                            }
                            None => {
                                let snapped = d.snap_if_quantized(d.playback_pos);
                                d.hotcues[index] = Some(crate::mixer::HotcueSlot::Cue(snapped));
                            }
                        }
                    }
                }
            }
        }
        MixerCommand::ClearHotcue { deck, index } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if index < 8 { d.hotcues[index] = None; }
            }
        }
        MixerCommand::ToggleLoop { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.looping = !d.params.looping;
                if d.params.looping && d.params.loop_start == 0 {
                    let bar = if let Some(bpm) = d.bpm {
                        if bpm > 0.0 { (44100.0 * 60.0 * 4.0 / bpm) as usize } else { 44100 * 4 }
                    } else { 44100 * 4 };
                    let start = d.snap_if_quantized(d.playback_pos);
                    d.params.loop_start = start;
                    d.params.loop_end = start + bar;
                }
            }
        }
        MixerCommand::SetLoopRegion { deck, start, end } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.loop_start = d.snap_if_quantized(start);
                d.params.loop_end = d.snap_if_quantized(end);
            }
        }
        MixerCommand::SeekDeck { deck, position } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                let total = d.buffer.as_ref().map(|b| b.len()).unwrap_or(0);
                d.playback_pos = position.min(total.saturating_sub(1));
            }
        }
        // StartBroadcast/StopBroadcast match arms EXCLUDED - need crate::broadcast (real unsafe: unsafe impl Send).
        MixerCommand::JogNudge { deck, delta } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                let new_pos = d.playback_pos as isize + delta;
                d.playback_pos = new_pos.max(0) as usize;
            }
        }
        MixerCommand::SetBpm { deck, bpm } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.bpm = Some(bpm);
                // Build beatgrid from BPM + buffer length
                if let Some(ref buf) = d.buffer {
                    let sr = buf.sample_rate;
                    let len = buf.len();
                    d.beatgrid = Some(crate::bpm::BeatGrid::from_bpm(bpm, 0, sr, len));
                }
            }
        }
        MixerCommand::SetKey { deck, key } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                mixer.deck_mut(deck_id).key = Some(key);
            }
        }
        MixerCommand::SetGenre { deck, genre } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                mixer.deck_mut(deck_id).genre = Some(genre);
            }
        }
        MixerCommand::SetPlaying { deck, playing } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.playing = playing;
            }
        }
        MixerCommand::ToggleAutoDj { .. } => {
            // Handled by feeder thread directly — needs Arc<AutoDj>, not &mut Mixer
        }
        MixerCommand::ToggleSlip { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if d.slip_active {
                    // Deactivate: snap playback to where it would have been
                    d.playback_pos = d.slip_pos;
                    d.slip_active = false;
                } else {
                    // Activate: start shadow tracking from current position
                    d.slip_pos = d.playback_pos;
                    d.slip_active = true;
                }
            }
        }
        MixerCommand::SaveLoop { deck, slot } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if slot < 8 && d.params.looping && d.params.loop_end > d.params.loop_start {
                    d.hotcues[slot] = Some(crate::mixer::HotcueSlot::Loop {
                        start: d.params.loop_start,
                        end: d.params.loop_end,
                    });
                }
            }
        }
        MixerCommand::SetReplayGain { deck, gain_db } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.replay_gain_db = gain_db;
            }
        }
        MixerCommand::TogglePfl { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.pfl = !d.params.pfl;
            }
        }
        MixerCommand::ToggleKeylock { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.params.keylock = !d.params.keylock;
            }
        }
        MixerCommand::ToggleReverse { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.reverse = !d.reverse;
            }
        }
        MixerCommand::Scratch { deck, rate } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                mixer.deck_mut(deck_id).scratch_rate = rate.clamp(-3.0, 3.0);
            }
        }
        MixerCommand::SetScratching { deck, active } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                mixer.deck_mut(deck_id).scratching = active;
                if !active {
                    // When releasing platter, start decay
                    mixer.deck_mut(deck_id).scratch_rate *= 0.5;
                }
            }
        }
        MixerCommand::SetSync { deck, mode } => {
            // SF-004: LOCKED — parse_sync_mode is case-insensitive; "Leader" now correctly
            // maps to Leader instead of silently falling through to Off.
            // See forge-audio/src/params.rs::tests::sf_004_* and docs/plans/2026-04-09-ipc-silent-failure-audit.md.
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let sync = match crate::params::parse_sync_mode(&mode) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[mixer_cmd] SF-004: {}", e);
                        return;
                    }
                };
                // If setting as leader, demote any existing leader
                if sync == crate::mixer::SyncMode::Leader {
                    for d in &mut mixer.decks {
                        if d.sync_mode == crate::mixer::SyncMode::Leader {
                            d.sync_mode = crate::mixer::SyncMode::Follower;
                        }
                    }
                }
                mixer.deck_mut(deck_id).sync_mode = sync;
                // Immediately adjust tempo to match reference
                mixer.sync_tempos();
            }
        }
        MixerCommand::ToggleMic => {
            mixer.mic.active = !mixer.mic.active;
        }
        MixerCommand::ToggleMicMonitor => {
            mixer.mic.monitor_enabled = !mixer.mic.monitor_enabled;
        }
        MixerCommand::ToggleAutoDuck => {
            mixer.mic.auto_duck_enabled = !mixer.mic.auto_duck_enabled;
        }
        MixerCommand::CrossfadeRamp { target, seconds } => {
            let blocks_per_sec = 93.75; // 48000 / 512
            let total_blocks = seconds * blocks_per_sec;
            mixer.crossfade_rate = (target - mixer.crossfader) / total_blocks;
            mixer.crossfade_target = Some(target);
        }
        MixerCommand::ToggleGhost => {
            mixer.ghost_enabled = !mixer.ghost_enabled;
        }
        MixerCommand::LoadSampler { slot, buffer } => {
            if slot < 8 {
                mixer.samplers[slot].buffer = Some(buffer);
                mixer.samplers[slot].volume = 1.0;
            }
        }
        MixerCommand::TriggerSampler { slot } => {
            if slot < 8 { mixer.samplers[slot].trigger(); }
        }
        MixerCommand::StopSampler { slot } => {
            if slot < 8 {
                mixer.samplers[slot].playing = false;
                mixer.samplers[slot].playback_pos = 0;
            }
        }
        MixerCommand::SetFxAssign { deck, fx_slot, enabled } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                if fx_slot < 4 {
                    mixer.fx_assign[deck_id as usize][fx_slot] = enabled;
                }
            }
        }
        MixerCommand::FlushDeck { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let idx = deck_id as usize;
                {
                    let d = mixer.deck_mut(deck_id);
                    d.params.playing = false;
                    d.playback_pos = 0;
                    d.params.looping = false;
                    d.slip_active = false;
                    d.slip_pos = 0;
                    d.scratch_rate = 0.0;
                    d.scratching = false;
                }
                // Zero the peak/RMS so meters drop instantly
                mixer.peak_levels[idx] = 0.0;
                mixer.rms_levels[idx] = 0.0;
            }
        }
        MixerCommand::LibraryScroll { .. } | MixerCommand::LibraryLoadHighlighted { .. } | MixerCommand::SwitchAudioDevice { .. } => {
            // Library navigation / device switch — handled outside the mixer
        }
        MixerCommand::LoopResize { deck, direction } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                let len = d.params.loop_end.saturating_sub(d.params.loop_start);
                if len > 0 {
                    let new_len = if direction > 0 { len * 2 } else { (len / 2).max(512) };
                    d.params.loop_end = d.params.loop_start + new_len;
                }
            }
        }
        MixerCommand::BeatJump { deck, beats } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if let Some(ref bg) = d.beatgrid {
                    let offset = (beats * bg.beat_interval as f64) as isize;
                    let new_pos = (d.playback_pos as isize + offset).max(0) as usize;
                    let total = d.buffer.as_ref().map(|b| b.len()).unwrap_or(0);
                    d.playback_pos = new_pos.min(total.saturating_sub(1));
                    if d.params.quantize {
                        d.playback_pos = bg.snap_to_beat(d.playback_pos);
                    }
                }
            }
        }
        MixerCommand::LoopRollStart { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                d.loop_roll_return = Some(d.playback_pos);
                if !d.params.looping {
                    let bar = d.beatgrid.as_ref()
                        .map(|bg| bg.beat_interval * 4)
                        .unwrap_or(44100 * 4);
                    d.params.loop_start = d.playback_pos;
                    d.params.loop_end = d.playback_pos + bar;
                }
                d.params.looping = true;
            }
        }
        MixerCommand::LoopRollStop { deck } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if let Some(return_pos) = d.loop_roll_return.take() {
                    // Calculate where playback would have been
                    let elapsed = d.playback_pos.saturating_sub(d.params.loop_start);
                    let loops = if d.params.loop_end > d.params.loop_start {
                        elapsed / (d.params.loop_end - d.params.loop_start)
                    } else { 0 };
                    let loop_len = d.params.loop_end.saturating_sub(d.params.loop_start);
                    let total_elapsed = d.playback_pos - d.params.loop_start + loops * loop_len;
                    d.playback_pos = return_pos + total_elapsed;
                }
                d.params.looping = false;
            }
        }
        MixerCommand::HealToggle => { mixer.heal.active = !mixer.heal.active; }
        MixerCommand::HealIntensity { intensity } => { mixer.heal.intensity = intensity.clamp(0.0, 1.0); }
        MixerCommand::HealMode { mode } => { mixer.heal.mode = mode.min(3); }
        MixerCommand::ShiftBeatgrid { deck, offset } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if let Some(ref mut bg) = d.beatgrid {
                    bg.first_beat = (bg.first_beat as i64 + offset).max(0) as usize;
                }
            }
        }
        MixerCommand::SetBpmManual { deck, bpm } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                let total = d.buffer.as_ref().map(|b| b.len()).unwrap_or(0);
                let sr = d.buffer.as_ref().map(|b| b.sample_rate).unwrap_or(44100);
                let first = d.beatgrid.as_ref().map(|bg| bg.first_beat).unwrap_or(0);
                d.bpm = Some(bpm);
                d.beatgrid = Some(crate::bpm::BeatGrid::from_bpm(bpm, first, sr, total));
            }
        }
        MixerCommand::BpmMultiply { deck, factor } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                let d = mixer.deck_mut(deck_id);
                if let Some(old_bpm) = d.bpm {
                    let new_bpm = (old_bpm * factor).clamp(30.0, 300.0);
                    let total = d.buffer.as_ref().map(|b| b.len()).unwrap_or(0);
                    let sr = d.buffer.as_ref().map(|b| b.sample_rate).unwrap_or(44100);
                    let first = d.beatgrid.as_ref().map(|bg| bg.first_beat).unwrap_or(0);
                    d.bpm = Some(new_bpm);
                    d.beatgrid = Some(crate::bpm::BeatGrid::from_bpm(new_bpm, first, sr, total));
                }
            }
        }
        // SF-017: LOCKED — decode error from spawned load thread stored per-deck so the snapshot
        // can surface it to the UI. Clears on successful load (LoadDeck command resets to None).
        MixerCommand::DeckLoadFailed { deck, error } => {
            if let Some(deck_id) = deck_id_from_str(&deck) {
                mixer.deck_load_errors[deck_id as usize] = Some(error);
            } else {
                eprintln!("[mixer] SF-017: DeckLoadFailed for unknown deck {:?}", deck);
            }
        }
        MixerCommand::SetCrossfaderCurve { curve } => {
            mixer.crossfader_curve = match curve.as_str() {
                "sharp" => crate::mixer::CrossfaderCurve::SharpCut,
                _ => crate::mixer::CrossfaderCurve::SmoothBlend,
            };
        }
        MixerCommand::AmbientWeather { rain_permyriad, wind_permyriad, fog_permyriad } => {
            mixer.apply_ambient_weather(rain_permyriad, wind_permyriad, fog_permyriad);
        }
    }
}

/// Drain all pending commands from the channel and apply them.
/// Returns the number of commands applied.
pub fn drain_commands(rx: &crossbeam_channel::Receiver<MixerCommand>, mixer: &mut Mixer) -> usize {
    let mut count = 0;
    while let Ok(cmd) = rx.try_recv() {
        apply_command(mixer, cmd);
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_command_applies() {
        let mut mixer = Mixer::default();
        let cmd = MixerCommand::Param { target: "crossfader".into(), value: 0.7 };
        apply_command(&mut mixer, cmd);
        assert!((mixer.crossfader - 0.7).abs() < 0.01);
    }

    #[test]
    fn action_command_applies() {
        let mut mixer = Mixer::default();
        assert!(!mixer.decks[0].params.playing);
        let cmd = MixerCommand::Action { target: "deck_a.play_pause".into() };
        apply_command(&mut mixer, cmd);
        assert!(mixer.decks[0].params.playing);
    }

    #[test]
    fn load_deck_command() {
        let mut mixer = Mixer::default();
        let buf = crate::dsp::AudioBuffer {
            samples: vec![vec![0.0; 44100]],
            sample_rate: 44100,
        };
        let cmd = MixerCommand::LoadDeck {
            deck: "a".into(),
            buffer: buf,
            title: "Test Title".into(),
            artist: "Test Artist".into(),
        };
        apply_command(&mut mixer, cmd);
        assert!(mixer.decks[0].buffer.is_some());
        assert_eq!(mixer.decks[0].title, "Test Title");
        assert_eq!(mixer.decks[0].artist, "Test Artist");
    }

    #[test]
    fn stem_toggle_command() {
        let mut mixer = Mixer::default();
        assert!(!mixer.decks[0].stems.drums_muted);
        let cmd = MixerCommand::StemToggle { deck: "a".into(), stem: StemPart::Drums };
        apply_command(&mut mixer, cmd);
        assert!(mixer.decks[0].stems.drums_muted);
        // Toggle again
        let cmd = MixerCommand::StemToggle { deck: "a".into(), stem: StemPart::Drums };
        apply_command(&mut mixer, cmd);
        assert!(!mixer.decks[0].stems.drums_muted);
    }

    #[test]
    fn drain_applies_all_pending() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut mixer = Mixer::default();

        tx.send(MixerCommand::Param { target: "crossfader".into(), value: 0.3 }).unwrap();
        tx.send(MixerCommand::Action { target: "deck_a.play_pause".into() }).unwrap();
        tx.send(MixerCommand::Param { target: "deck_a.volume".into(), value: 0.8 }).unwrap();

        let count = drain_commands(&rx, &mut mixer);
        assert_eq!(count, 3);
        assert!((mixer.crossfader - 0.3).abs() < 0.01);
        assert!(mixer.decks[0].params.playing);
        assert!((mixer.decks[0].params.volume - 0.8).abs() < 0.01);
    }

    #[test]
    fn drain_empty_channel_returns_zero() {
        let (_tx, rx) = crossbeam_channel::unbounded::<MixerCommand>();
        let mut mixer = Mixer::default();
        assert_eq!(drain_commands(&rx, &mut mixer), 0);
    }

    #[test]
    fn toggle_fx_command() {
        let mut mixer = Mixer::default();
        assert!(!mixer.fx_slots[0].enabled);
        apply_command(&mut mixer, MixerCommand::ToggleFx { slot: 0 });
        assert!(mixer.fx_slots[0].enabled);
    }

    #[test]
    fn toggle_autodj_is_noop_in_apply() {
        // ToggleAutoDj is handled by feeder thread, not apply_command
        let mut mixer = Mixer::default();
        apply_command(&mut mixer, MixerCommand::ToggleAutoDj { enabled: true });
        // No panic, no side effect — handled elsewhere
    }
}