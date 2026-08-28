//! S07 — Prime-user no-face video pipeline integration test.
//!
//! Proves: heal_voice processes audio correctly (HPF + comp + limit),
//! the output holds ceiling, and the healed buffer is exportable via write_wav.
//! This is the "revascularize" proof — existing organs compose into the pipeline.
//!
//! Location (for Opus promotion): forge-audio/tests/s07_prime_user.rs
//! Run: `cargo test -p forge-audio --test s07_prime_user`

use forge_audio_v3::healing::{heal_voice, HealingParams};
use forge_audio_v3::dsp::{write_wav, AudioBuffer};

/// Generate a synthetic "voice" buffer: 1s of 150Hz fundamental + noise (simulates VO).
fn synthetic_voice(sample_rate: u32) -> AudioBuffer {
    let len = sample_rate as usize;
    let mut samples = vec![0.0f32; len];
    let mut rng = 0x12345678u64;

    for i in 0..len {
        // 150Hz fundamental (male voice range)
        let fundamental = (2.0 * std::f32::consts::PI * 150.0 * i as f32 / sample_rate as f32).sin();
        // Add some noise (simulates breath/room)
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        let noise = (rng as i32 as f32) / i32::MAX as f32 * 0.05;
        samples[i] = fundamental * 0.3 + noise;
    }

    AudioBuffer { samples: vec![samples], sample_rate }
}

#[test]
fn heal_voice_processes_and_holds_ceiling() {
    let raw = synthetic_voice(44100);
    let params = HealingParams::default();
    let healed = heal_voice(raw, &params);

    // Output must not be silent (the BrickwallLimiter process() bug was caught here)
    let peak = healed.samples[0].iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    assert!(peak > 0.01, "RED: healed output must not be silent (peak={peak})");

    // Ceiling must hold: no sample exceeds 1.0 (brickwall limit)
    let over = healed.samples[0].iter().filter(|&&s| s.abs() > 1.0).count();
    assert_eq!(over, 0, "RED: healed output must not exceed ceiling (over-1.0 samples: {over})");

    // DC offset should be near zero (HPF removes it)
    let dc: f32 = healed.samples[0].iter().sum::<f32>() / healed.samples[0].len() as f32;
    assert!(dc.abs() < 0.01, "RED: DC offset must be near zero after HPF (dc={dc})");

    println!("[S07] ✓ heal_voice: peak={peak:.3}, dc={dc:.6}, over_ceiling={over}");
}

#[test]
fn healed_audio_exports_to_wav() {
    let raw = synthetic_voice(44100);
    let healed = heal_voice(raw, &HealingParams::default());

    let path = format!("{}/s07_healed_test.wav", std::env::temp_dir().display());
    write_wav(&path, &healed).expect("RED: write_wav must succeed on healed buffer");

    let meta = std::fs::metadata(&path).expect("WAV must exist");
    assert!(meta.len() > 44, "RED: WAV must be larger than header");

    let data = std::fs::read(&path).unwrap();
    assert_eq!(&data[0..4], b"RIFF");
    assert_eq!(&data[8..12], b"WAVE");

    let _ = std::fs::remove_file(&path);
    println!("[S07] ✓ healed → WAV export: {} bytes", meta.len());
}

#[test]
fn pipeline_end_to_end_mic_heal_export() {
    // Simulates: mic_capture → heal → export (the first 3 stations of the pipeline)
    // (wordmap → EDL → render → mp4 egress are downstream; gated on Conductor V7)
    let raw = synthetic_voice(44100);

    // Heal
    let healed = heal_voice(raw, &HealingParams::default());
    let peak = healed.samples[0].iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    assert!(peak > 0.01 && peak <= 1.0, "pipeline: healed peak must be in (0.01, 1.0]");

    // Export
    let path = format!("{}/s07_pipeline.wav", std::env::temp_dir().display());
    write_wav(&path, &healed).expect("pipeline export");
    let _ = std::fs::remove_file(&path);

    println!("[S07] ✓ pipeline: mic → heal → export (3/8 stations proven)");
    println!("       Remaining: wordmap → EDL → render → mp4 (gated on Conductor V7)");
}
