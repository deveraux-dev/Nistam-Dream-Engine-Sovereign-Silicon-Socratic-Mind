//! `clip-edit` — folded into `13forge-studio clip-edit <input> [output.wav] [OPTIONS]`.
//! Speech clip editor: cut ums/uhs and quantize to beat grid.
//!
//! Pipeline: decode input (MP3/WAV/FLAC/OGG) → detect pauses + filler words (energy +
//! spectral) → remove them → quantize remaining speech clips to a beat grid for video
//! sync → write output WAV with crossfaded clip boundaries. LOAD-TIME (carve-out).

use crate::speech_clip::{self, ClipConfig, ClipKind};

/// `13forge-studio clip-edit …`. Returns the process exit code.
pub fn run() -> i32 {
    // argv under the umbrella: [exe, "clip-edit", <input>, <output?>, <flags…>] — strip
    // BOTH the exe name and the subcmd token so the positional index math below is
    // identical to the standalone bin (which used `.skip(1)`).
    let args: Vec<String> = std::env::args().skip(2).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return 0;
    }

    let input = &args[0];
    let output = args.get(1).map(|s| s.as_str()).unwrap_or("output.wav");

    // Parse optional flags
    let mut config = ClipConfig::default();
    let mut manual_bpm: Option<f32> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--bpm" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    manual_bpm = val.parse().ok();
                }
            }
            "--subdivision" | "--sub" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    config.beat_subdivision = val.parse().unwrap_or(4);
                }
            }
            "--pause" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    config.min_pause_secs = val.parse().unwrap_or(0.3);
                }
            }
            "--threshold" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    config.silence_threshold = val.parse().unwrap_or(0.025);
                }
            }
            "--filler-max" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    config.max_filler_duration_secs = val.parse().unwrap_or(0.6);
                }
            }
            "--crossfade" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    config.crossfade_secs = val.parse().unwrap_or(0.005);
                }
            }
            "--verbose" | "-v" => {
                // handled below
            }
            other => {
                eprintln!("[clip-edit] unknown flag: {other}");
                print_usage();
                return 1;
            }
        }
        i += 1;
    }

    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");

    println!("[clip-edit] loading: {input}");
    let buf = match crate::dsp::load_audio(input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[clip-edit] FATAL: failed to decode {input}: {e}");
            return 1;
        }
    };

    let duration = buf.duration_secs();
    println!("[clip-edit] decoded: {:.1}s, {}ch, {} Hz", duration, buf.channels(), buf.sample_rate);

    // Detect or use manual BPM
    let bpm = manual_bpm.unwrap_or_else(|| {
        let detected = speech_clip::detect_speech_tempo(&buf);
        println!("[clip-edit] detected tempo: {detected:.0} BPM");
        detected
    });
    if manual_bpm.is_some() {
        println!("[clip-edit] using manual BPM: {bpm:.0}");
    }

    println!("[clip-edit] processing (pause={:.2}s, threshold={:.3}, subdivision=1/{})...",
        config.min_pause_secs, config.silence_threshold, config.beat_subdivision);

    let result = speech_clip::edit_speech(&buf, bpm, &config);

    // Print clip map
    if verbose {
        println!("\n[clip-edit] clip map:");
        println!("  {:>5} {:>8} {:>8} {:>8} {:>10}  {}", "#", "start", "end", "dur(s)", "centroid", "kind");
        for (i, clip) in result.all_clips.iter().enumerate() {
            let marker = if result.kept_indices.contains(&i) { "✓" } else { "✗" };
            let kind_str = match clip.kind {
                ClipKind::Speech => "SPEECH",
                ClipKind::Filler => "FILLER",
                ClipKind::Pause => "PAUSE",
            };
            println!("  {:>4}{} {:>8} {:>8} {:>8.2} {:>8.0} Hz  {}",
                i, marker,
                clip.start, clip.end,
                clip.duration_secs(buf.sample_rate),
                clip.centroid_hz,
                kind_str,
            );
        }
        println!();
    }

    // Summary
    let fillers = result.all_clips.iter().filter(|c| c.kind == ClipKind::Filler).count();
    let pauses = result.all_clips.iter().filter(|c| c.kind == ClipKind::Pause).count();
    let kept = result.kept_indices.len();

    println!("[clip-edit] results:");
    println!("  clips detected:   {}", result.all_clips.len());
    println!("  speech kept:      {kept}");
    println!("  fillers removed:  {fillers}");
    println!("  pauses removed:   {pauses}");
    println!("  removed duration: {:.2}s", result.removed_secs);
    println!("  output duration:  {:.2}s", result.output.duration_secs());
    println!("  beat grid:        {bpm:.0} BPM, 1/{} subdivision", config.beat_subdivision);

    // Write output
    println!("[clip-edit] writing: {output}");
    if let Err(e) = speech_clip::process_speech_file(input, output, Some(config)) {
        eprintln!("[clip-edit] FATAL: write failed: {e}");
        return 1;
    }

    println!("[clip-edit] done ✓");
    0
}

fn print_usage() {
    eprintln!(r#"
13forge-studio clip-edit — speech clip editor

USAGE:
  13forge-studio clip-edit <input> [output.wav] [OPTIONS]

OPTIONS:
  --bpm <N>          Override auto-detected BPM (default: auto-detect)
  --subdivision <N>  Beat subdivision for quantization (default: 4 = quarter-beat)
  --pause <secs>     Minimum pause duration to split on (default: 0.3)
  --threshold <f32>  Silence RMS threshold (default: 0.025)
  --filler-max <s>   Max filler duration in seconds (default: 0.6)
  --crossfade <s>    Crossfade duration at boundaries (default: 0.005)
  --verbose, -v      Print detailed clip map

EXAMPLES:
  13forge-studio clip-edit podcast.mp3 clean.wav
  13forge-studio clip-edit talk.mp3 synced.wav --bpm 120 --subdivision 8
  13forge-studio clip-edit ramble.wav tight.wav --filler-max 0.4 --pause 0.2 -v
"#);
}
