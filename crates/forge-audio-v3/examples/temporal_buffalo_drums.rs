//! "Temporal Buffalo" drum bed -- Sean 2026-08-20 ("I want drums"). Generates
//! a deterministic DnB pattern via `forge_harmonics::dnb::generate` and
//! bounces it to a real `.wav` through `dnb_render` (just re-landed this
//! session -- see forge-harmonics/src/{dnb,euclid}.rs).
//!
//! Run: `cargo run --example temporal_buffalo_drums -p forge-audio-v3`

use forge_audio_v3::dnb_render::{render_pcm, write_wav};
use forge_harmonics::dnb::generate;

fn main() {
    // Seed spells the working title in hex-ish digits (b0f4a10 ~ "buffalo");
    // root 38 = D2, the sub-register tonic dnb.rs's tests already assume.
    let pattern = generate(0x0B0FFA10, 38);
    let pcm = render_pcm(&pattern);

    let out_path = r"F:\v3\.forge\reel-out\temporal-buffalo-drums.wav";
    write_wav(out_path, &pcm).expect("write temporal-buffalo-drums.wav");

    let secs = pcm.len() as f32 / 44_100.0;
    println!(
        "{} drum hits, {} bass notes, {:.2}s @ {} BPM -> {out_path}",
        pattern.drums.len(),
        pattern.bass.len(),
        secs,
        pattern.bpm
    );
}
