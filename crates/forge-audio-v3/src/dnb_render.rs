//! DnbPattern → mono 16-bit PCM → .wav — the Motif PREVIEW bridge.
//! Drained 2026-07-17 (15k block) from `_quarry/attic-merge-2026-07-16/
//! forge-studio-recovered/src/main.rs:118-182` (organ absent live, 0 hits).
//! Offline / cold path: f32 + alloc are fine here (the audio-render boundary;
//! integer purity lives in forge-harmonics). Ports dnb-racer's hand-rolled
//! bouncer. Caller: `examples/motif_wav.rs` (renders a generated motif).

use forge_harmonics::dnb::{DnbPattern, DrumVoice};

const DNB_SR: u32 = 44_100;

/// One decaying voice rendered additively into `buf` from sample `start`.
fn voice(buf: &mut [i32], start: usize, freq_hz: f32, len: usize, amp: i32, noise: i32, seed: &mut u64) {
    for i in 0..len {
        let idx = start + i;
        if idx >= buf.len() {
            break;
        }
        let env = amp * (len - i) as i32 / len.max(1) as i32; // linear decay
        let phase = std::f32::consts::TAU * freq_hz * i as f32 / DNB_SR as f32;
        let tone = (phase.sin() * env as f32) as i32;
        *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let hiss = (((*seed >> 40) as i32) - 8_388_608) / 256;
        let n = hiss * noise / 10_000 * env / 32_768;
        buf[idx] = buf[idx].saturating_add(tone + n);
    }
}

/// MIDI note → frequency (Hz). A4(69)=440.
fn midi_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

/// Frequency (Hz) → nearest MIDI note (A4 = 440 Hz = note 69), clamped 0..=127.
/// The colour→sound seam's landing pad (drained 07-17 with its inverse above,
/// ex studio-recovered main.rs:187-193; organ absent live).
pub fn freq_to_midi(freq: f32) -> u8 {
    if freq <= 0.0 {
        return 60;
    }
    let n = 69.0 + 12.0 * (freq / 440.0).log2();
    n.round().clamp(0.0, 127.0) as u8
}

/// Render a DnB pattern (drums + in-key bass) to mono 16-bit PCM.
pub fn render_pcm(p: &DnbPattern) -> Vec<i16> {
    let spf = (DNB_SR * p.step_ms() / 1000) as usize; // samples per 16th-step
    let mut acc = vec![0i32; spf * p.steps as usize + DNB_SR as usize]; // tail for decay
    let mut seed = 0x13F0_6541u64;
    for d in &p.drums {
        let start = d.step as usize * spf;
        match d.voice {
            DrumVoice::Kick => voice(&mut acc, start, 55.0, DNB_SR as usize * 18 / 100, 26_000, 0, &mut seed),
            DrumVoice::Snare => voice(&mut acc, start, 190.0, DNB_SR as usize * 12 / 100, 14_000, 9_000, &mut seed),
            DrumVoice::Hat => voice(&mut acc, start, 320.0, DNB_SR as usize * 4 / 100, 6_000, 10_000, &mut seed),
        }
    }
    for b in &p.bass {
        voice(&mut acc, b.step as usize * spf, midi_hz(b.note), b.dur_steps as usize * spf, 18_000, 0, &mut seed);
    }
    acc.iter().map(|&s| s.clamp(-32_768, 32_767) as i16).collect()
}

/// Write mono 16-bit PCM to a `.wav` (hand-rolled RIFF; pure std, zero deps).
pub fn write_wav(path: &str, samples: &[i16]) -> std::io::Result<()> {
    let data_bytes = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&DNB_SR.to_le_bytes());
    out.extend_from_slice(&(DNB_SR * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits/sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_renders_nonsilent_bounded_pcm() {
        let p = forge_harmonics::dnb::generate(4242, 38);
        let pcm = render_pcm(&p);
        assert!(!pcm.is_empty());
        let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
        assert!(peak > 500, "a full pattern must actually sound (peak {peak})");
        assert!(peak <= 32_768, "clamp law holds (i16::MIN's unsigned_abs is 32768)");
        // Determinism: same pattern, same bounce.
        let pcm2 = render_pcm(&forge_harmonics::dnb::generate(4242, 38));
        assert_eq!(pcm, pcm2);
    }

    #[test]
    fn freq_and_midi_round_trip_on_the_grid() {
        for note in 21..=108u8 {
            assert_eq!(freq_to_midi(midi_hz(note)), note, "note {note} must survive Hz round-trip");
        }
        assert_eq!(freq_to_midi(0.0), 60, "non-positive Hz lands on middle C");
    }
}
