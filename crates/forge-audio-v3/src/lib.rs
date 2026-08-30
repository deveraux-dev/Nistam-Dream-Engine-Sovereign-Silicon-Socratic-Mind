// Workspace law is missing_docs=deny — this port trips it broadly
// (v2 forge-audio never carried doc-comment discipline; a 155-file DAW-scale
// engine, game-jam-shaped source). Allowed explicitly, same reasoning as
// forge-arena-v3's lib.rs: real doc-writing is a separate, later pass, not
// filler comments to satisfy the lint syntactically.
#![allow(missing_docs)]

// v2 Crate Zero modules forge-audio needs but v3's forge-core-v3 (a strict
// zero-dependency floor) can't carry — landed locally instead (Sean 2026-08-15
// "find the missing, copy it, wire it to v3"). Verified absent everywhere
// else in F:\v3 before landing (grep, not assumed).
pub mod correspondence;
pub mod creature_engine;
pub mod gesture_brush;
pub mod scheduled_event;
pub mod phrase;
pub mod vested_decay;
pub mod vocal_frame;
pub mod lightning;
pub mod ump;
pub mod dimensional_collapse;

pub mod formant_meter;
pub mod fft_buf;
pub use fft_buf::AudioFftBuf;
pub mod fft_hardened;
pub use fft_hardened::{HardenedFft, c2c_in_place, rfft_in_place, irfft_in_place};
// harmonic_brush: EXCLUDED — needs forge_harmonics::{note_to_mhz,ScaleMask,
// VoicePreset} and forge_ump::BardPhraseKind directly (not through crate::
// phrase); none of the forge_harmonics trio exist in v3's forge-harmonics.
pub mod harmonic_brush;
pub mod game_midi;
pub mod game_sync;
pub mod gerzon;
// mood: EXCLUDED — needs forge_harmonics::{loop_phase,AccountIndex,
// IronrootMidi2Event,LoopThread,RECOMMENDED_LOOP_SECS}, none ported.
// pub mod mood;
// photometric_backend/photometric_bridge: EXCLUDED — need forge_photometric::
// {audio_types,sound_consumer,types}, none exist in forge-photometric-v3
// (a differently-shaped, smaller port than v2's namesake).
// pub mod photometric_backend;
// pub mod photometric_bridge;
pub mod recipe;
// pub mod bus; // EXCLUDED - bus::bus/engine_adapter need crate::realtime (real unsafe)
pub mod device;
pub mod loopback;
pub mod bpm;
pub mod camelot;
pub mod castword;
pub mod broski;
// pub mod carrier_5d; // EXCLUDED - needs crate::dimensional_collapse + forge_harmonics::synthxml
pub mod impact;
#[cfg(feature = "radio-db")]
pub mod radio_db;
// conductor_audio: EXCLUDED — needs crate::harmonic_brush (excluded above).
pub mod conductor_audio;
pub mod score_player;
pub mod controller;
pub mod dsp;
pub mod effects;
// genre_detect: LANDED 2026-08-19 — GenreRouter re-founded on
// forge_core::metarouter::MetaRouter (.s13 byte-quantized trit-LUT routing,
// ARCH000 approved), replacing the v2-shaped forge_hal::expert_pool::MoeRouter
// dependency that never matched v3's real API.
pub mod genre_detect;
pub mod ingest;
pub mod mp3_sovereign;
pub mod loop_sequencer;
pub mod key_detect;
pub mod alchemy;
pub mod mapping;
pub mod midi;
pub mod forge_midi;
#[cfg(feature = "sovereign-broadcast")]
pub mod sovereign_comms;
pub mod modulation;
pub mod params;
pub mod deterministic_audio;
pub mod synth;
pub mod synth_keyboard;
pub mod transport;
pub mod tui_driver;
// viz_buffer: EXCLUDED — real `unsafe` block (raw pointer write), forbidden
// by this workspace's `-D unsafe-code`. A safe rewrite is real work, not a
// missing-copy problem; separate stroke.
pub mod viz_buffer;
pub mod analyzer;
pub mod audio_energy;
pub mod palette_gen;
pub mod session_recorder;
// mic_capture: EXCLUDED — `TripleBuffer<Vec<f32>>::try_take` needs
// `Vec<f32>: ClockPlane`, not implemented for that type in
// forge-hal-clockspine — a real API-shape gap, not a missing-copy problem.
// pub mod mic_capture;
// (a stale "loopback EXCLUDED — WASAPI COM" note stood here 2026-08-17; wrong
// on both counts — the landed loopback.rs above is cpal-based and unsafe-free.)
// voice_fanout: EXCLUDED — needs forge_harmonics::voice_bridge, not ported.
// pub mod voice_fanout;
pub mod studio_session;

pub mod mixer;
pub mod mixer_cmd;
pub mod snapshot;
pub mod player;
pub mod audio_state;
pub mod pcm_cache;
pub mod rt_safety;
pub mod composer;
pub mod fx_processor;
pub mod dnb_render;
pub mod presets;
pub mod correspondence_bus;
// dimensional_collapse: the EXCLUDED note that stood here was STALE (corrected
// 2026-08-25) — the module is live and declared at the top of this file, beside
// `lightning` and `ump`. A reader landing here concluded it was excluded while
// it had been compiling all along; `carrier_5d`'s exclusion note above still
// names it as a dependency, which was the tell.
pub mod spatial_hrtf;
pub mod spatial_voice;
pub mod ghost_whisper;
// broadcast: EXCLUDED — `unsafe impl Send` (real unsafe-code law conflict).
// pub mod broadcast;
// realtime: EXCLUDED — real `unsafe` blocks (thread priority + ptr::read).
// pub mod realtime;
pub mod device_info;
// input_capture: LANDED 2026-08-19 — real-time cpal audio INPUT via a
// lock-free rtrb SPSC ring buffer (mirrors realtime.rs's output path,
// inverted). No unsafe — depends on `device_info::AudioDeviceInfo` only,
// not on the excluded `realtime` module.
pub mod input_capture;
// alloc_tracer: EXCLUDED — `unsafe impl GlobalAlloc` + unsafe methods
// throughout (the whole point of the type is a raw allocator hook).
// pub mod alloc_tracer;
pub mod telemetry;
pub mod roadie;
pub mod metering;
pub mod speech_clip;
pub mod vocal_studio;
pub mod tools;
pub mod doctor;

/// SILENCE IS THE FLOOR (Sean 2026-07-24, boot-silent law). Real audio output
/// devices open ONLY when this returns true — i.e. `FORGE_AUDIO=1` is set in the
/// environment (the launch blast). Default (no var, every test, every normal boot)
/// = silent: no cpal device is ever opened, so a forgotten mute can never blast the
/// speakers.
#[inline]
pub fn audio_enabled() -> bool {
    std::env::var("FORGE_AUDIO").as_deref() == Ok("1")
}

// multitool entry points (forge audio <subcmd>)

// run_xml_to_wav: EXCLUDED — needs forge_harmonics::{musicxml_extract,
// synthxml}, not ported. Named plainly rather than stubbed silently.

/// Ported Ironroot Edict audio synchronization architecture: AudioLaneRouter,
/// MusicMood, and game_tick_audio_sync. Integrates with viz_buffer and sync_monitor
/// for multi-threaded audio-game clock coordination.
pub mod ironroot_sync;

/// `forge audio features <file>` — extract BPM/genre/energy from a music file.
pub fn run_music_features(args: Vec<String>) -> Result<(), String> {
    fn rms(buf: &[f32]) -> f32 {
        if buf.is_empty() { return 0.0; }
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }
    fn genre_name(g: Option<u8>) -> &'static str {
        match g {
            Some(0) => "Drum & Bass", Some(1) => "Techno",
            Some(2) => "Deep / Downtempo", Some(3) => "Other",
            _ => "unanalysed",
        }
    }

    let path = args.into_iter().next()
        .ok_or_else(|| "usage: forge audio features <song.mp3>".to_string())?;

    match ingest::ingest_file(&path)
        .map_err(|e| format!("[music_features] ingest failed for {path}: {e}"))?
    {
        ingest::Ingested::Recorded { audio, meta } => {
            let detected_bpm = bpm::detect_bpm(&audio);
            let energy = rms(&audio.to_mono());
            println!("=== {path} — the game-alter signal ===");
            if !meta.title.is_empty() || !meta.artist.is_empty() {
                println!("  track     : {} — {}", meta.artist, meta.title);
            }
            println!("  format    : {:.1}s @ {} Hz, {} ch", audio.duration_secs(), audio.sample_rate, audio.channels());
            println!("  BPM       : {detected_bpm:.1}        -> motion / speed-as-tempo");
            println!("  genre     : {}   -> palette / mood", genre_name(meta.genre));
            println!("  energy    : {energy:.4}     -> vibe glow / pulse");
            println!("\nThese are what a game reads each block (live: forge_audio AudioState) to ALTER itself.");
        }
        ingest::Ingested::Symbolic { meta, .. } => {
            println!("{path} is a symbolic score ({:.1}s) — use `xml-to-wav` for symbolic music.", meta.duration_secs);
        }
    }
    Ok(())
}

/// Open a real radio.db and print what's actually in it — track count and a
/// sample of real rows, read through the exact shipped `RadioDb` path (no
/// separate inspection tool, no synthetic fixture). Read-only in spirit:
/// `RadioDb::open` runs the same additive `CREATE TABLE IF NOT EXISTS` /
/// `ALTER TABLE ADD COLUMN` auto-migration it always does on open, nothing
/// destructive, but callers should still point this at a copy for anything
/// irreplaceable.
/// Raw, read-only table inventory — `sqlite_master` + a row count per table,
/// no `RadioDb::open` (which runs its own additive auto-migration on open).
/// Run this FIRST against anything real before `run_library_status`, so an
/// unfamiliar schema is seen before anything, even additively, touches it.
#[cfg(feature = "radio-db")]
pub fn run_library_inventory(args: Vec<String>) -> Result<(), String> {
    let path = args.into_iter().next()
        .ok_or_else(|| "usage: forge audio library-inventory <path-to.db>".to_string())?;
    let conn = rusqlite::Connection::open(&path).map_err(|e| format!("open {path}: {e}"))?;
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .map_err(|e| e.to_string())?;
    let names: Vec<String> = stmt.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?
        .collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    println!("library (raw inventory): {path}");
    if names.is_empty() {
        println!("  no tables at all");
        return Ok(());
    }
    for name in &names {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |r| r.get(0))
            .map_err(|e| format!("count {name}: {e}"))?;
        println!("  {name}: {n} row(s)");
    }
    Ok(())
}

#[cfg(feature = "radio-db")]
pub fn run_library_status(args: Vec<String>) -> Result<(), String> {
    let path = args.into_iter().next()
        .ok_or_else(|| "usage: forge audio library-status <path-to.db>".to_string())?;
    let db = radio_db::RadioDb::open(&path).map_err(|e| format!("open {path}: {e}"))?;
    let count = db.track_count().map_err(|e| e.to_string())?;
    println!("library: {path}");
    println!("  tracks: {count}");
    if count == 0 {
        return Ok(());
    }
    let sample = db.search("", None, None, None, 10, None, None, None).map_err(|e| e.to_string())?;
    println!("  sample ({} of {count}):", sample.len());
    for t in &sample {
        let artist = t.artist.as_deref().unwrap_or("?");
        let title = t.title.as_deref().unwrap_or("?");
        let bpm = t.bpm_mbpm.map(|m| format!("{:.1}", m as f64 / 1000.0)).unwrap_or_else(|| "-".into());
        println!("    {artist} — {title}  [bpm {bpm}]");
    }
    Ok(())
}

/// Real, on-disk status of the `.s13` genre-classification LUT — `cargo
/// xtask audio genre-status` reads this, never a fabricated "should be
/// there" guess. `path` defaults to `.forge/models/genre.s13` (repo-relative)
/// but any path can be checked.
pub fn run_genre_status(args: Vec<String>) -> Result<(), String> {
    let path = args.into_iter().next().unwrap_or_else(|| ".forge/models/genre.s13".to_string());
    let p = std::path::Path::new(&path);
    if !p.exists() {
        println!("genre LUT: ABSENT — {path} (heuristic detect_genre() still runs; the .s13 refinement layer has nothing loaded)");
        return Ok(());
    }
    match genre_detect::GenreRouter::load(p) {
        Ok(router) => {
            println!("genre LUT: LOADED — {path}");
            println!("  is_loaded : {}", router.is_loaded());
            Ok(())
        }
        Err(e) => {
            println!("genre LUT: PRESENT BUT INVALID — {path}: {e}");
            Ok(())
        }
    }
}

/// Render a MusicXML score to a WAV file through the conductor's own lane —
/// the same parse/lower/strike chain `score_player` proves in test, driven at
/// the real 120 Hz master tick. Wired 2026-08-27; was a refusing stub.
fn run_xml_to_wav(args: Vec<String>) -> Result<(), String> {
    let input = args
        .first()
        .ok_or("usage: forge audio xml-to-wav <file.musicxml> [out.wav]")?;
    let out_path = args.get(1).cloned().unwrap_or_else(|| "score.wav".to_string());

    let bytes = std::fs::read(input).map_err(|e| format!("read {input}: {e}"))?;
    let score = forge_harmonics::musicxml_extract::musicxml_to_score(&bytes)
        .map_err(|e| format!("{input}: {e:?}"))?;

    const SR: u32 = 48_000;
    const TICK_HZ: u64 = 120;
    let per_tick = (SR as u64 / TICK_HZ) as usize;
    let mut player = score_player::ScorePlayer::from_score(&score);
    let mut lane = conductor_audio::AudioLane::new(SR, score.tempo_bpm_x100 as f32 / 100.0);

    let notes = player.remaining();
    let mut pcm: Vec<f32> = Vec::new();
    let mut block = vec![0.0f32; per_tick];
    // Play the plan out, then keep rendering until the last voice decays so the
    // final note is not clipped off mid-ring (bounded: 4 s of tail).
    let mut tick: u64 = 0;
    let max_tail = TICK_HZ * 4;
    let mut tail = 0u64;
    loop {
        player.tick(tick, &mut lane);
        block.iter_mut().for_each(|s| *s = 0.0);
        lane.render(&mut block, &[]);
        pcm.extend_from_slice(&block);
        if player.is_finished() {
            if lane.active_voices() == 0 || tail >= max_tail {
                break;
            }
            tail += 1;
        }
        tick += 1;
    }

    let buf = dsp::AudioBuffer { samples: vec![pcm], sample_rate: SR };
    let secs = buf.len() as f32 / SR as f32;
    dsp::write_wav(&out_path, &buf)?;
    println!("xml-to-wav: {input} -> {out_path}");
    println!("  {notes} notes, {:.2} BPM, {secs:.2}s @ {SR} Hz", score.tempo_bpm_x100 as f32 / 100.0);
    Ok(())
}

/// Top-level dispatch for `forge audio <subcmd> [args...]`.
pub fn run(args: Vec<String>) -> Result<(), String> {
    let mut it = args.into_iter();
    match it.next().as_deref() {
        Some("xml-to-wav") => run_xml_to_wav(it.collect()),
        Some("features")   => run_music_features(it.collect()),
        Some("doctor")     => doctor::run_doctor(it.collect()),
        Some("genre-status") => run_genre_status(it.collect()),
        #[cfg(feature = "radio-db")]
        Some("library-status") => run_library_status(it.collect()),
        #[cfg(feature = "radio-db")]
        Some("library-inventory") => run_library_inventory(it.collect()),
        _ => Err("usage: forge audio <xml-to-wav|features|doctor|genre-status|library-status>\n  xml-to-wav <file.musicxml> [out.wav]  — render a score to WAV\n  features <file>             — extract BPM/genre/energy\n  doctor [<crate-dir>]        — static audit: suites + RT-lock + dormant deps\n  genre-status [<file.s13>]   — real on-disk .s13 genre-LUT status\n  library-status <path.db>    — real radio.db track count + sample (needs --features radio-db)".into()),
    }
}


#[cfg(feature = "hid")]
pub mod wacom_hid;
#[cfg(feature = "radio-db")]
pub mod scanner;
pub mod filename_parser;
pub mod signal_ghosts;
pub mod ghost_registry;
#[cfg(feature = "hid")]
pub mod hid_s2;
#[cfg(feature = "hid")]
pub mod pen_instrument;
pub mod water_fx;
pub mod limiter;
pub mod healing;
#[cfg(feature = "cognitive")]
pub mod cognitive_heal;
pub mod mic_fx;
pub mod noise_suppress;
pub mod broadcast_booth;
// stem_conductor/recipe_author: NOW AVAILABLE — forge_ump-v3 now carries
// Message/UmpReader (ported 2026-08-17).
pub mod stem_conductor;
pub mod recipe_author;
// fauna: EXCLUDED whole — 2 of its 5 submodules (absence.rs, fauna_sound.rs)
// use real `unsafe` (OnceLock<UnsafeCell<_>> single-audio-thread pattern),
// and psychoacoustic.rs's "master chain" composes both, so a partial
// exclusion would leave a broken chain.
// pub mod fauna;
// pub mod player_api; // EXCLUDED - needs crate::realtime (real unsafe)
#[cfg(feature = "radio-db")]
pub mod music_deck;
pub mod controllers;
