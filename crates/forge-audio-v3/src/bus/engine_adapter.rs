//! Engine adapter — bridges forge-audio (real audio engine) into forge-audio::bus (public API).
//!
//! This module is ONE-WAY: forge-audio::bus commands → forge-audio commands.
//! forge-audio does not know about forge-audio::bus.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use forge_ump::provenance_tag::Tier;
use forge_ump::Recorder;

use super::bus::{tick_us, HubTapeStat, HUB_TAPE, HUB_TAPE_JR_US};
use super::snapshot::{BeatGridInfo, DeckSnapshot, DeckState, LiveMixerState};
use super::track::{AudioFormat, TrackInfo};
use super::ump_codec;

use crate::params::{MixerParam, MixerAction, DeckParam};
use crate::mixer::DeckId;

// ── Deck index → string helper ──────────────────────────────────────────

/// Map a forge-audio::bus deck index (0–3) to the forge-audio deck letter.
fn deck_id_to_str(deck: DeckId) -> &'static str {
    match deck {
        DeckId::A => "a",
        DeckId::B => "b",
        DeckId::C => "c",
        DeckId::D => "d",
    }
}

fn format_param(param: &MixerParam) -> String {
    match param {
        MixerParam::Crossfader => "crossfader".to_string(), // @forge:allow_alloc
        MixerParam::MasterVolume => "master_volume".to_string(), // @forge:allow_alloc
        MixerParam::Deck { id, param } => {
            let p = match param {
                DeckParam::Volume => "volume",
                DeckParam::Tempo => "tempo",
                DeckParam::EqLow => "eq_low",
                DeckParam::EqMid => "eq_mid",
                DeckParam::EqHigh => "eq_high",
                _ => "volume",
            };
            format!("deck_{}.{}", deck_id_to_str(*id), p) // @forge:allow_alloc
        }
        _ => "master_volume".to_string(), // @forge:allow_alloc
    }
}

fn format_action(action: &MixerAction) -> String {
    match action {
        MixerAction::DeckPlayPause(id) => format!("deck_{}.play_pause", deck_id_to_str(*id)), // @forge:allow_alloc
        MixerAction::DeckCue(id) => format!("deck_{}.cue", deck_id_to_str(*id)), // @forge:allow_alloc
        _ => "nop".to_string(), // @forge:allow_alloc
    }
}

// ── Deck index → string helper ──────────────────────────────────────────

/// Map a forge-audio::bus deck index (0–3) to the forge-audio deck letter.
fn deck_index_to_str(deck: usize) -> &'static str {
    match deck {
        0 => "a",
        1 => "b",
        2 => "c",
        3 => "d",
        _ => "a",
    }
}

// ── Command translation ─────────────────────────────────────────────────

/// Translate a single forge-audio::bus `MixerCommand` into zero or more
/// forge-audio `MixerCommand`s.
pub fn translate_command(
    cmd: &super::command::MixerCommand,
    sample_rate: u32,
) -> Vec<crate::mixer_cmd::MixerCommand> {
    match cmd {
        super::command::MixerCommand::Play { deck, track } => {
            let deck_str = deck_index_to_str(*deck).to_string();

            if track.path.is_empty() {
                return vec![crate::mixer_cmd::MixerCommand::SetPlaying {
                    deck: deck_str,
                    playing: true,
                }];
            }

            match crate::pcm_cache::get_or_load(&track.path) {
                Ok(buffer) => {
                    vec![
                        crate::mixer_cmd::MixerCommand::LoadDeck {
                            deck: deck_str.clone(),
                            buffer,
                            title: track.title.clone(),
                            artist: track.artist.clone(),
                        },
                        crate::mixer_cmd::MixerCommand::SetPlaying {
                            deck: deck_str,
                            playing: true,
                        },
                    ]
                }
                Err(e) => {
                    eprintln!(
                        "[engine_adapter] decode failed for {:?}: {}",
                        track.path, e
                    );
                    vec![crate::mixer_cmd::MixerCommand::DeckLoadFailed {
                        deck: deck_str,
                        error: e.to_string(),
                    }]
                }
            }
        }

        super::command::MixerCommand::Pause { deck } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![crate::mixer_cmd::MixerCommand::SetPlaying {
                deck: deck_str,
                playing: false,
            }]
        }

        super::command::MixerCommand::Stop { deck } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![
                crate::mixer_cmd::MixerCommand::SetPlaying {
                    deck: deck_str.clone(),
                    playing: false,
                },
                crate::mixer_cmd::MixerCommand::SeekDeck {
                    deck: deck_str,
                    position: 0,
                },
            ]
        }

        super::command::MixerCommand::SetSync { deck, mode } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![crate::mixer_cmd::MixerCommand::SetSync {
                deck: deck_str,
                mode: mode.clone(),
            }]
        }

        super::command::MixerCommand::ToggleLoop { deck } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![crate::mixer_cmd::MixerCommand::ToggleLoop {
                deck: deck_str,
            }]
        }

        super::command::MixerCommand::SetVolume { deck, volume } => {
            let target = format!("deck_{}.volume", deck_index_to_str(*deck));
            vec![crate::mixer_cmd::MixerCommand::Param {
                target,
                value: *volume,
            }]
        }

        super::command::MixerCommand::SetPan { deck, pan } => {
            let target = format!("deck_{}.pan", deck_index_to_str(*deck));
            vec![crate::mixer_cmd::MixerCommand::Param {
                target,
                value: *pan,
            }]
        }

        super::command::MixerCommand::ToggleMute { deck } => {
            vec![crate::mixer_cmd::MixerCommand::Action {
                target: format!("deck_{}.toggle_mute", deck_index_to_str(*deck)),
            }]
        }

        super::command::MixerCommand::ToggleSolo { deck } => {
            vec![crate::mixer_cmd::MixerCommand::Action {
                target: format!("deck_{}.toggle_solo", deck_index_to_str(*deck)),
            }]
        }

        super::command::MixerCommand::SetCrossfader { position } => {
            vec![crate::mixer_cmd::MixerCommand::Param {
                target: "crossfader".into(),
                value: *position,
            }]
        }

        super::command::MixerCommand::SetMasterVolume { volume } => {
            vec![crate::mixer_cmd::MixerCommand::Param {
                target: "master_volume".into(),
                value: *volume,
            }]
        }

        super::command::MixerCommand::Seek { deck, position_secs } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            let position = (*position_secs * sample_rate as f64) as usize;
            vec![crate::mixer_cmd::MixerCommand::SeekDeck {
                deck: deck_str,
                position,
            }]
        }

        super::command::MixerCommand::ApplyEffect { deck, effect } => {
            eprintln!(
                "[engine_adapter] ApplyEffect deck={} effect={:?} — no forge-audio mapping, skipped",
                deck, effect
            );
            vec![]
        }

        super::command::MixerCommand::RemoveEffect { deck, effect_id } => {
            eprintln!(
                "[engine_adapter] RemoveEffect deck={} id={} — no forge-audio mapping, skipped",
                deck, effect_id
            );
            vec![]
        }

        super::command::MixerCommand::Enqueue { deck, track } => {
            eprintln!(
                "[engine_adapter] Enqueue deck={} track={:?} — no forge-audio mapping, skipped",
                deck, track.title
            );
            vec![]
        }

        super::command::MixerCommand::Shutdown => {
            // Intercepted by `real_feeder_loop`'s recv match (it returns before
            // calling translate_command), so reaching this arm is defensive
            // only — it is also exercised directly by the unit tests below.
            vec![] // @forge:allow_alloc -- command translation
        }

        super::command::MixerCommand::SetParam(param, value) => {
            let target = format_param(param);
            vec![crate::mixer_cmd::MixerCommand::Param { // @forge:allow_alloc -- command translation
                target,
                value: *value,
            }]
        }

        super::command::MixerCommand::SetAction(action) => {
            let target = format_action(action);
            vec![crate::mixer_cmd::MixerCommand::Action { // @forge:allow_alloc -- command translation
                target,
            }]
        }

        super::command::MixerCommand::Param { target, value } => {
            vec![crate::mixer_cmd::MixerCommand::Param {
                target: target.clone(),
                value: *value,
            }]
        }

        super::command::MixerCommand::Action { target } => {
            vec![crate::mixer_cmd::MixerCommand::Action {
                target: target.clone(),
            }]
        }

        super::command::MixerCommand::LoadDeck { deck, buffer, title, artist } => {
            vec![crate::mixer_cmd::MixerCommand::LoadDeck {
                deck: deck.clone(),
                buffer: buffer.clone(),
                title: title.clone(),
                artist: artist.clone(),
            }]
        }

        super::command::MixerCommand::ToggleFx { slot } => {
            vec![crate::mixer_cmd::MixerCommand::ToggleFx { slot: *slot }]
        }

        super::command::MixerCommand::SetFxIntensity { slot, intensity } => {
            vec![crate::mixer_cmd::MixerCommand::SetFxIntensity {
                slot: *slot,
                intensity: *intensity,
            }]
        }

        super::command::MixerCommand::SetPreset { name, intensity } => {
            vec![crate::mixer_cmd::MixerCommand::SetPreset {
                name: name.clone(),
                intensity: *intensity,
            }]
        }

        super::command::MixerCommand::SetFxSlot { slot, preset } => {
            vec![crate::mixer_cmd::MixerCommand::SetFxSlot {
                slot: *slot,
                preset: preset.clone(),
            }]
        }

        super::command::MixerCommand::SetHotcue { deck, slot, position } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![crate::mixer_cmd::MixerCommand::SetHotcue {
                deck: deck_str,
                index: *slot as usize,
                position: Some(*position),
            }]
        }

        super::command::MixerCommand::DeleteHotcue { deck, slot } => {
            let deck_str = deck_index_to_str(*deck).to_string();
            vec![crate::mixer_cmd::MixerCommand::ClearHotcue {
                deck: deck_str,
                index: *slot as usize,
            }]
        }

        super::command::MixerCommand::ToggleBroadcast => {
            // Unwired: no low-level mixer_cmd mapping, and the bus Mixer defers
            // it to the engine adapter (bus/mixer.rs apply_command) — so it
            // lands nowhere. Surface the drop instead of dying silent (§0).
            eprintln!(
                "[engine_adapter] ToggleBroadcast — unwired (no low-level mapping; bus Mixer defers here), dropped"
            );
            vec![]
        }

        super::command::MixerCommand::ToggleRecording { output_dir } => {
            // Unwired: produced by the forge-gui export panel but has no
            // low-level mapping and the bus Mixer defers it here — dropped.
            eprintln!(
                "[engine_adapter] ToggleRecording dir={:?} — unwired (no low-level mapping), dropped",
                output_dir
            );
            vec![]
        }
        super::command::MixerCommand::ToggleMic => {
            vec![crate::mixer_cmd::MixerCommand::ToggleMic]
        }
        super::command::MixerCommand::ToggleMicMonitor => {
            vec![crate::mixer_cmd::MixerCommand::ToggleMicMonitor]
        }
        super::command::MixerCommand::PlaySfx { .. } => {
            vec![] // Handled directly by bus Mixer, not forwarded to low-level engine
        }

        super::command::MixerCommand::LoadSampler { slot, buffer } => {
            vec![crate::mixer_cmd::MixerCommand::LoadSampler {
                slot: *slot,
                buffer: buffer.clone(),
            }]
        }

        super::command::MixerCommand::TriggerSampler { slot } => {
            vec![crate::mixer_cmd::MixerCommand::TriggerSampler { slot: *slot }]
        }

        super::command::MixerCommand::StopSampler { slot } => {
            vec![crate::mixer_cmd::MixerCommand::StopSampler { slot: *slot }]
        }

        // Sequencer commands are owned by the bus Mixer (`MixerCommandHub`
        // drives `self.sequencer` in bus/mixer.rs apply_command), NOT the
        // low-level engine — so translating them here is intentionally a no-op.
        super::command::MixerCommand::SequencerPlay
        | super::command::MixerCommand::SequencerStop
        | super::command::MixerCommand::SequencerSetStep { .. }
        | super::command::MixerCommand::SequencerSetStepVel { .. }
        | super::command::MixerCommand::SequencerSetBpm { .. } => vec![],

        super::command::MixerCommand::AmbientWeather { rain_permyriad, wind_permyriad, fog_permyriad } => {
            vec![crate::mixer_cmd::MixerCommand::AmbientWeather {
                rain_permyriad: *rain_permyriad,
                wind_permyriad: *wind_permyriad,
                fog_permyriad: *fog_permyriad,
            }]
        }

        // Recording/export: produced by the forge-gui mic-sampler + export
        // panels but have no low-level mapping and the bus Mixer defers them
        // here — genuinely unwired, so make the drop audible, not silent.
        super::command::MixerCommand::StartRecord { .. }
        | super::command::MixerCommand::StopRecord { .. }
        | super::command::MixerCommand::QuickExport { .. } => {
            eprintln!(
                "[engine_adapter] recording/export command {:?} — unwired, dropped",
                cmd
            );
            vec![]
        }
    }
}

// ── Snapshot conversion ─────────────────────────────────────────────────

pub fn convert_snapshot(
    src: &crate::snapshot::MixerSnapshot,
    frame: u64,
    waveform_len: usize,
    spectrum_len: usize,
) -> LiveMixerState {
    let mut decks = [
        DeckSnapshot::default(),
        DeckSnapshot::default(),
        DeckSnapshot::default(),
        DeckSnapshot::default(),
    ];

    let mut first_playing_bpm: Option<f32> = None;
    let mut first_playing_waveform: Option<&Vec<f32>> = None;
    let mut first_playing_fft: Option<&Vec<f32>> = None;
    let mut any_playing = false;

    for (i, src_deck) in src.decks.iter().enumerate() {
        let state = if src_deck.playing {
            any_playing = true;
            if first_playing_bpm.is_none() {
                first_playing_bpm = src_deck.bpm;
                first_playing_waveform = Some(&src_deck.waveform);
                first_playing_fft = Some(&src_deck.fft_bins);
            }
            DeckState::Playing
        } else if src_deck.duration > 0.0 {
            DeckState::Paused
        } else {
            DeckState::Empty
        };

        let track = if !src_deck.title.is_empty() || !src_deck.artist.is_empty() {
            Some(TrackInfo {
                path: String::new(),
                title: src_deck.title.clone(),
                artist: src_deck.artist.clone(),
                duration_secs: src_deck.duration,
                bpm: src_deck.bpm,
                key: src_deck.key.clone(),
                format: AudioFormat::Wav,
                genre: None,
            })
        } else if src_deck.duration > 0.0 {
            Some(TrackInfo {
                path: String::new(),
                title: String::new(),
                artist: String::new(),
                duration_secs: src_deck.duration,
                bpm: src_deck.bpm,
                key: src_deck.key.clone(),
                format: AudioFormat::Wav,
                genre: None,
            })
        } else {
            None
        };

        decks[i] = DeckSnapshot {
            track,
            state,
            position_secs: src_deck.pos,
            duration_secs: src_deck.duration,
            volume: src_deck.volume,
            waveform_peak: src_deck.peak_level,
            effects: Vec::new(),
            error_message: src.deck_load_errors[i].clone(),
            rms_level: src_deck.rms_level,
            spectral_energy: src_deck.spectral_energy,
            eq_bands: src_deck.eq,
            beat_phase: src_deck.beat_phase,
            genre: src_deck.genre,
            waveform_bands: {
                let mut bands = [[0.0f32; 3]; 200];
                let copy_len = src_deck.waveform_bands.len().min(200);
                bands[..copy_len].copy_from_slice(&src_deck.waveform_bands[..copy_len]);
                bands
            },
            pan: 0.0,
            muted: false,
            solo: false,
        };
    }

    let waveform_buffer = match first_playing_waveform {
        Some(w) => {
            let mut buf = vec![0.0f32; waveform_len];
            let copy_len = w.len().min(waveform_len);
            buf[..copy_len].copy_from_slice(&w[..copy_len]);
            Arc::from(buf.into_boxed_slice())
        }
        None => Arc::from(vec![0.0f32; waveform_len].into_boxed_slice()),
    };

    let spectrum = match first_playing_fft {
        Some(fft) => {
            let mut buf = vec![0.0f32; spectrum_len];
            let copy_len = fft.len().min(spectrum_len);
            buf[..copy_len].copy_from_slice(&fft[..copy_len]);
            Arc::from(buf.into_boxed_slice())
        }
        None => Arc::from(vec![0.0f32; spectrum_len].into_boxed_slice()),
    };

    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let beat_phase_delta = {
        let mut playing: Vec<(usize, f32)> = src.decks.iter().enumerate()
            .filter(|(_, d)| d.playing)
            .map(|(i, d)| (i, d.rms_level))
            .collect();
        playing.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if playing.len() >= 2 {
            let phase_a = decks[playing[0].0].beat_phase;
            let phase_b = decks[playing[1].0].beat_phase;
            let raw = (phase_a - phase_b).abs();
            raw.min(1.0 - raw)
        } else {
            0.0
        }
    };

    LiveMixerState {
        decks,
        master_volume: src.master_volume,
        crossfader: src.crossfader,
        bpm: first_playing_bpm.unwrap_or(0.0),
        is_playing: any_playing,
        frame,
        waveform_buffer,
        spectrum,
        timestamp_ns,
        underrun_count: 0,
        waveform_overviews: {
            let mut overviews = [[0.0f32; 200]; 4];
            for (i, src_deck) in src.decks.iter().enumerate() {
                let copy_len = src_deck.waveform.len().min(200);
                overviews[i][..copy_len].copy_from_slice(&src_deck.waveform[..copy_len]);
            }
            overviews
        },
        waveform_bands_overviews: {
            let mut bands = [[[0.0f32; 3]; 200]; 4];
            for (i, src_deck) in src.decks.iter().enumerate() {
                let copy_len = src_deck.waveform_bands.len().min(200);
                bands[i][..copy_len].copy_from_slice(&src_deck.waveform_bands[..copy_len]);
            }
            bands
        },
        beatgrid_info: {
            let mut info: [Option<BeatGridInfo>; 4] = [None, None, None, None];
            for (i, src_deck) in src.decks.iter().enumerate() {
                if src_deck.has_beatgrid {
                    if let Some(bpm) = src_deck.bpm {
                        let beat_interval_frac = if bpm > 0.0 {
                            60.0 / bpm as f64 / src_deck.duration.max(0.001)
                        } else {
                            0.0
                        };
                        info[i] = Some(BeatGridInfo {
                            bpm,
                            first_beat_frac: 0.0,
                            beat_interval_frac,
                        });
                    }
                }
            }
            info
        },
        beat_phase_delta,
        pfl_active: [
            src.decks[0].pfl,
            src.decks[1].pfl,
            src.decks[2].pfl,
            src.decks[3].pfl,
        ],
        mic_active: false,
        mic_volume: 1.0,
        mic_peak: 0.0,
        mic_rms: 0.0,
        mic_talkover_db: 0.0,
        mic_attached: false,
        mic_auto_duck_enabled: false,
        mic_monitor_enabled: false,
        mic_duck_applied_db: 0.0,
        seq_step: None,
        last_mix_us: 0,
    }
}

// ── Real feeder loop ────────────────────────────────────────────────────

pub fn real_feeder_loop(
    cmd_rx: crossbeam_channel::Receiver<super::command::MixerCommand>,
    snapshot: Arc<ArcSwap<LiveMixerState>>,
    mut producer: rtrb::Producer<f32>,
    underruns: Arc<AtomicU64>,
    mut hp_producer: Option<rtrb::Producer<f32>>,
    audio_state_pub: Option<crate::audio_state::AudioStatePublisher>,
    hub_tape: Arc<ArcSwap<HubTapeStat>>,
) {
    crate::realtime::raise_audio_thread_priority();

    let mut mixer = crate::mixer::Mixer::default();
    mixer.device_sample_rate = 48000;

    let mut frame: u64 = 0;

    // Master-bus flight recorder (real path) — mirror of the stub loop's tape. Every
    // command is encoded to a UMP and sealed into a scrubbable tape (Tier::Local),
    // published to the per-handle ArcSwap AND the process-global HUB_TAPE that every
    // panel's REC bar reads. Commit fires only on iterations that drained commands.
    let mut recorder = Recorder::new(HUB_TAPE_JR_US, Tier::Local);
    let mut tape_events: u64 = 0;
    let mut last_essence: u8 = 0;
    const WAVEFORM_LEN: usize = 1024;
    const SPECTRUM_LEN: usize = 512;

    // FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19): pre-allocated Arc pool to
    // avoid `Arc::new` on the hot publish path. `Arc::get_mut` succeeds when no
    // consumer is holding the previous Arc (the common case for ArcSwap with
    // `load()` Guards); falls back to one `Arc::new` on rare contention.
    let mut snap_slots: [Arc<LiveMixerState>; 2] = [
        Arc::new(LiveMixerState::default()),
        Arc::new(LiveMixerState::default()),
    ];
    let mut snap_idx = 0usize;
    let mut state_slots: [Arc<crate::audio_state::AudioState>; 2] = [
        Arc::new(crate::audio_state::AudioState::default()),
        Arc::new(crate::audio_state::AudioState::default()),
    ];
    let mut state_idx = 0usize;

    // FORGE-AUDIO-HOTPATH-CLEAN-001: SPSC ring for deferred FX-apply error
    // logs. Producer goes into the Mixer; consumer drained at end of each
    // loop iteration (cold-path eprintln on the feeder thread — NOT the cpal
    // callback). Ring size 16 — drops silently on overflow.
    let (fx_err_tx, mut fx_err_rx) = rtrb::RingBuffer::<crate::mixer::FxError>::new(16);
    mixer.set_fx_error_sink(fx_err_tx);

    // SFX one-shot queue: (mono PCM, cursor). Filled cold-path from PlaySfx
    // commands; iterated read-only inside the RT region; drained post-RT.
    let mut sfx_queue: Vec<(Vec<f32>, usize)> = Vec::new(); // @forge:allow_alloc: cold-path SFX list, not the cpal callback

    loop {
        // AUDIO-ONE-BUS follow-up (2026-07-19): whole-iteration timer, not just
        // `mix_block` — `mix_us` alone proved the stall isn't compute cost inside
        // mix_block (22us at every underrun cycle vs a 2500us deadline), so this
        // times the full trip around the outer loop (command drain, recorder
        // commit, mix, snapshot publish, cold-path drains) on every exit path,
        // including the early-`continue` sleep branches below. Pairs with
        // `mix_us` in telemetry to pin whether a stall is inside the rest of the
        // loop body or is a scheduling gap between iterations.
        let iter_start = std::time::Instant::now();
        // In-flight probe (2026-07-19): publish this trip's start + phase marks so
        // the cpal callback can stamp (age, section) at underrun time — the
        // completed-iteration iter_us above can never see a stall still running.
        crate::telemetry::telemetry().record_iter_start();

        loop {
            match cmd_rx.try_recv() {
                Ok(super::command::MixerCommand::Shutdown) => {
                    recorder.observe(ump_codec::stamp(&super::command::MixerCommand::Shutdown, tick_us()));
                    let _ = recorder.commit(frame, 0, ump_codec::essence_of(&super::command::MixerCommand::Shutdown));
                    return;
                }
                Ok(cmd) => {
                    // Record every command onto the flight-recorder tape (cold path).
                    recorder.observe(ump_codec::stamp(&cmd, tick_us()));
                    last_essence = ump_codec::essence_of(&cmd);
                    tape_events += 1;
                    if let super::command::MixerCommand::PlaySfx { buffer } = &cmd {
                        // Intercept before translate_command (which drops PlaySfx).
                        // Queue mono PCM; mixed into output below. Cold path — allocation here is outside RT.
                        sfx_queue.push((buffer.to_mono(), 0)); // @forge:allow_alloc: cold path
                        continue;
                    }
                    let translated = translate_command(&cmd, mixer.device_sample_rate);
                    for fa_cmd in translated {
                        crate::mixer_cmd::apply_command(&mut mixer, fa_cmd);
                    }
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => return,
            }
        }

        // Seal a command moment only when this iteration drained commands.
        if recorder.pending_len() > 0 {
            if let Ok(sealed) = recorder.commit(frame, 0, last_essence) {
                let stat = HubTapeStat {
                    moments: recorder.len() as u64,
                    events: tape_events,
                    last_seal: sealed.content_seal,
                    last_tick: sealed.tick_id,
                };
                hub_tape.store(Arc::new(stat));
                HUB_TAPE.store(stat);
            }
        }

        crate::telemetry::telemetry().set_iter_phase(crate::telemetry::iter_phase::CMD_DRAINED);

        let available = producer.slots();
        if available < 128 {
            crate::telemetry::telemetry().record_iter_us(iter_start.elapsed().as_micros() as u64);
            crate::telemetry::telemetry().set_iter_phase(crate::telemetry::iter_phase::SLEEP_LOW_SLOTS);
            std::thread::sleep(std::time::Duration::from_micros(500));
            continue;
        }

        let frames = (available / 2).min(512);
        if frames == 0 {
            crate::telemetry::telemetry().record_iter_us(iter_start.elapsed().as_micros() as u64);
            crate::telemetry::telemetry().set_iter_phase(crate::telemetry::iter_phase::SLEEP_ZERO_FRAMES);
            std::thread::sleep(std::time::Duration::from_micros(500));
            continue;
        }

        // ── RT region begins (FORGE-AUDIO-HOTPATH-CLEAN-001) ────────────────
        // No `Arc::new`, no `eprintln!`, no allocation between here and
        // `exit_rt()` below. `convert_snapshot` still allocates internally;
        // that's a separate (known) violation tracked outside this ticket.
        crate::rt_safety::enter_rt();

        let mix_start = std::time::Instant::now();
        let block = mixer.mix_block(frames);
        let mix_us = mix_start.elapsed().as_micros() as u64;

        let channels = block.channels();
        let consumed = block.len().min(frames);
        for f in 0..consumed {
            // Fold SFX one-shots into this frame sample (read-only iteration — no alloc).
            let sfx: f32 = sfx_queue.iter().map(|(mono, pos)| mono.get(pos + f).copied().unwrap_or(0.0)).sum();
            for ch in 0..2.min(channels) {
                let sample = block.samples[ch][f] + sfx;
                if producer.push(sample).is_err() {
                    break;
                }
            }
            if channels < 2 {
                let _ = producer.push(block.samples[0][f] + sfx);
            }
        }

        mixer.recycle_output(block);

        if let Some(ref mut hp_tx) = hp_producer {
            if let Some(ref hp_mix) = mixer.headphone_mix {
                let hp_channels = hp_mix.channels();
                for f in 0..hp_mix.len() {
                    for ch in 0..2.min(hp_channels) {
                        let _ = hp_tx.push(hp_mix.samples[ch][f]);
                    }
                    if hp_channels < 2 {
                        let _ = hp_tx.push(hp_mix.samples[0][f]);
                    }
                }
            }
        }

        let fa_snap = crate::snapshot::MixerSnapshot::capture(&mixer);
        let mut bus_snap = convert_snapshot(&fa_snap, frame, WAVEFORM_LEN, SPECTRUM_LEN);
        bus_snap.underrun_count = underruns.load(Ordering::Relaxed);
        bus_snap.last_mix_us = mix_us;

        // AUDIO-ONE-BUS underrun-position probe (2026-07-19 follow-up): publish the
        // loudest deck's sample cursor every feeder tick (~120 Hz) so the cpal
        // callback can stamp "which sample was last known" at underrun time —
        // deck-cursor-accurate, independent of the OS-decided callback cadence.
        crate::telemetry::telemetry().last_deck_sample_pos.store(mixer.sample_position() as u64, Ordering::Relaxed);
        crate::telemetry::telemetry().record_mix_us(mix_us);
        crate::telemetry::telemetry().set_iter_phase(crate::telemetry::iter_phase::MIXED);

        // Double-buffer publish — `Arc::get_mut` succeeds when no consumer is
        // holding the previous Arc (consumer.load() Guards do not retain a
        // refcount). On rare contention, fall back to one `Arc::new`.
        {
            let slot = &mut snap_slots[snap_idx];
            match Arc::get_mut(slot) {
                Some(slot_mut) => *slot_mut = bus_snap,
                None => *slot = Arc::new(bus_snap),
            }
            snapshot.store(slot.clone());
            snap_idx = (snap_idx + 1) & 1;
        }

        // Publish AudioState for vibe pipeline (lock-free, ~5ns)
        if let Some(ref pub_handle) = audio_state_pub {
            let state = crate::audio_state::AudioState {
                energy: mixer.master_rms(),
                spectral_centroid: mixer.spectral_centroid(),
                drop_detected: mixer.drop_detected(),
                genre: mixer.active_genre(),
                beat_grid: mixer.active_beat_grid(),
                sample_pos: mixer.sample_position(),
                spectrum: mixer.spectrum_snapshot(),
            };
            let slot = &mut state_slots[state_idx];
            match Arc::get_mut(slot) {
                Some(slot_mut) => *slot_mut = state,
                None => *slot = Arc::new(state),
            }
            pub_handle.store(slot.clone());
            state_idx = (state_idx + 1) & 1;
        }

        crate::rt_safety::exit_rt();
        // ── RT region ends ──────────────────────────────────────────────────
        crate::telemetry::telemetry().set_iter_phase(crate::telemetry::iter_phase::PUBLISHED);

        // Advance SFX one-shot cursors and discard finished buffers (cold path — outside RT).
        for sfx in sfx_queue.iter_mut() { sfx.1 += consumed; }
        sfx_queue.retain(|(mono, pos)| *pos < mono.len());

        // Cold-path FX error drain. `eprintln!` runs on the feeder thread,
        // not the cpal callback; no-error iterations cost one pop() check.
        while let Ok(err) = fx_err_rx.pop() {
            eprintln!(
                "[FX] slot {} apply failed on deck {} -- passing dry",
                err.fx_idx, err.deck_id
            );
        }

        crate::telemetry::telemetry().record_iter_us(iter_start.elapsed().as_micros() as u64);
        frame += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_index_to_str_maps_correctly() {
        assert_eq!(deck_index_to_str(0), "a");
        assert_eq!(deck_index_to_str(1), "b");
        assert_eq!(deck_index_to_str(2), "c");
        assert_eq!(deck_index_to_str(3), "d");
        assert_eq!(deck_index_to_str(99), "a");
    }

    #[test]
    fn real_feeder_records_commands_onto_the_tape() {
        use super::super::command::MixerCommand;

        // Headless: rtrb is just a ring — no cpal device needed. Drive the REAL
        // feeder loop, send commands, and assert the flight recorder sealed them.
        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<MixerCommand>(64);
        let snapshot = Arc::new(ArcSwap::from_pointee(LiveMixerState::default()));
        let underruns = Arc::new(AtomicU64::new(0));
        let hub_tape = Arc::new(ArcSwap::from_pointee(HubTapeStat::default()));

        // Keep the consumer alive so the producer stays connected.
        let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(8192);

        let tape_read = Arc::clone(&hub_tape);
        for _ in 0..3 {
            cmd_tx.send(MixerCommand::SetMasterVolume { volume: 0.5 }).unwrap();
        }

        let handle = std::thread::spawn(move || {
            real_feeder_loop(cmd_rx, snapshot, producer, underruns, None, None, hub_tape);
        });

        // Wait for the first activity commit to land on the tape.
        let mut ok = false;
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            if tape_read.load().events >= 3 {
                ok = true;
                break;
            }
        }
        cmd_tx.send(MixerCommand::Shutdown).unwrap();
        let _ = handle.join();

        let stat = tape_read.load();
        assert!(ok, "real feeder must seal the 3 commands onto the tape; got {}", stat.events);
        assert!(stat.moments >= 1, "at least one sealed moment expected");
        assert_ne!(stat.last_seal, 0, "sealed moment must carry a non-trivial content seal");
    }

    #[test]
    fn translate_pause_produces_set_playing_false() {
        let cmd = super::super::command::MixerCommand::Pause { deck: 1 };
        let translated = translate_command(&cmd, 48000);
        assert_eq!(translated.len(), 1);
        match &translated[0] {
            crate::mixer_cmd::MixerCommand::SetPlaying { deck, playing } => {
                assert_eq!(deck, "b");
                assert!(!playing);
            }
            _ => panic!("expected SetPlaying"),
        }
    }

    #[test]
    fn translate_stop_produces_two_commands() {
        let cmd = super::super::command::MixerCommand::Stop { deck: 2 };
        let translated = translate_command(&cmd, 48000);
        assert_eq!(translated.len(), 2);
        match &translated[0] {
            crate::mixer_cmd::MixerCommand::SetPlaying { deck, playing } => {
                assert_eq!(deck, "c");
                assert!(!playing);
            }
            _ => panic!("expected SetPlaying"),
        }
        match &translated[1] {
            crate::mixer_cmd::MixerCommand::SeekDeck { deck, position } => {
                assert_eq!(deck, "c");
                assert_eq!(*position, 0);
            }
            _ => panic!("expected SeekDeck"),
        }
    }

    #[test]
    fn translate_set_crossfader() {
        let cmd = super::super::command::MixerCommand::SetCrossfader { position: 0.75 };
        let translated = translate_command(&cmd, 48000);
        assert_eq!(translated.len(), 1);
        match &translated[0] {
            crate::mixer_cmd::MixerCommand::Param { target, value } => {
                assert_eq!(target, "crossfader");
                assert_eq!(*value, 0.75);
            }
            _ => panic!("expected Param"),
        }
    }

    #[test]
    fn translate_seek_converts_seconds_to_samples() {
        let cmd = super::super::command::MixerCommand::Seek {
            deck: 0,
            position_secs: 2.5,
        };
        let translated = translate_command(&cmd, 48000);
        assert_eq!(translated.len(), 1);
        match &translated[0] {
            crate::mixer_cmd::MixerCommand::SeekDeck { deck, position } => {
                assert_eq!(deck, "a");
                assert_eq!(*position, 120000);
            }
            _ => panic!("expected SeekDeck"),
        }
    }

    #[test]
    fn translate_unmapped_commands_return_empty() {
        let effect_cmd = super::super::command::MixerCommand::ApplyEffect {
            deck: 0,
            effect: super::super::effect::EffectType::Placeholder,
        };
        assert!(translate_command(&effect_cmd, 48000).is_empty());

        let remove_cmd = super::super::command::MixerCommand::RemoveEffect {
            deck: 0,
            effect_id: 1,
        };
        assert!(translate_command(&remove_cmd, 48000).is_empty());

        assert!(
            translate_command(&super::super::command::MixerCommand::Shutdown, 48000).is_empty()
        );
    }

    #[test]
    fn convert_snapshot_default_produces_empty_decks() {
        let fa_mixer = crate::mixer::Mixer::default();
        let fa_snap = crate::snapshot::MixerSnapshot::capture(&fa_mixer);
        let bus_snap = convert_snapshot(&fa_snap, 42, 1024, 512);

        assert_eq!(bus_snap.frame, 42);
        assert!(!bus_snap.is_playing);
        assert_eq!(bus_snap.waveform_buffer.len(), 1024);
        assert_eq!(bus_snap.spectrum.len(), 512);
        for deck in &bus_snap.decks {
            assert_eq!(deck.state, DeckState::Empty);
            assert!(deck.track.is_none());
        }
    }
}
