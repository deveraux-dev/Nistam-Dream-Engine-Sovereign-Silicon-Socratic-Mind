//! `music-features` — folded into `13forge-studio music-features <song.mp3>`.
//! Drag-and-drop a music file → see the signal the game alters to.
//!
//! The MP3 half of "alter the game to your music": ingest the file through the ONE load
//! seam (`ingest_file`), then extract the features a game reads to alter itself — tempo
//! (→ motion / speed-as-tempo), genre (→ palette / mood), energy (→ vibe glow/pulse).
//! The live realtime path publishes these same values per-block as `AudioState`; this
//! tool shows them for a whole file at load time, host-free (carve-out).

use crate::bpm;
use crate::ingest::{ingest_file, Ingested};

fn rms(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
}

/// TrackMeta genre byte → name (genre_detect: 0=DnB 1=Techno 2=Deep 3=Other).
fn genre_name(g: Option<u8>) -> &'static str {
    match g {
        Some(0) => "Drum & Bass",
        Some(1) => "Techno",
        Some(2) => "Deep / Downtempo",
        Some(3) => "Other",
        _ => "unanalysed",
    }
}

/// `13forge-studio music-features <song.mp3>`. Returns the process exit code.
pub fn run() -> i32 {
    // argv under the umbrella: [exe, "music-features", <song>] — the path is nth(2),
    // shifted +1 vs the standalone bin (which read nth(1)).
    let path = match std::env::args().nth(2) {
        Some(p) => p,
        None => {
            eprintln!("usage: 13forge-studio music-features <song.mp3>");
            return 2;
        }
    };

    match ingest_file(&path) {
        Ok(Ingested::Recorded { audio, meta }) => {
            let bpm = bpm::detect_bpm(&audio);
            let energy = rms(&audio.to_mono());
            println!("=== {path} — the game-alter signal ===");
            if !meta.title.is_empty() || !meta.artist.is_empty() {
                println!("  track     : {} — {}", meta.artist, meta.title);
            }
            println!(
                "  format    : {:.1}s @ {} Hz, {} ch",
                audio.duration_secs(),
                audio.sample_rate,
                audio.channels()
            );
            println!("  BPM       : {bpm:.1}        → motion / speed-as-tempo");
            println!("  genre     : {}   → palette / mood", genre_name(meta.genre));
            println!("  energy    : {energy:.4}     → vibe glow / pulse");
            println!("\nThese are what a game reads each block (live: forge_audio AudioState) to ALTER itself.");
            0
        }
        Ok(Ingested::Symbolic { meta, .. }) => {
            println!(
                "{path} is a symbolic score ({:.1}s) — use `xml_to_wav` / the conductor for symbolic music.",
                meta.duration_secs
            );
            0
        }
        Err(e) => {
            eprintln!("[music-features] ingest failed for {path}: {e}");
            1
        }
    }
}
