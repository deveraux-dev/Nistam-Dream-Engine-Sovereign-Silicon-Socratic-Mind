//! healing.rs — the signature vocal-healing chain.
//!
//! "It came from my healing. This is what I'm doing to heal."
//!
//! ONE named pass that composes forge-audio's OWN healers — nothing third-party:
//!   high-pass (clear the rumble + plosives)
//!     -> soft-knee compression (steady the delivery)
//!     -> brickwall limit (-1 dBFS, never clip).
//!
//! v2 folds in `crate::alchemy::restoration` (geometric spectral subtraction with
//! Minimum-Statistics noise floor) for damaged/noisy takes. A clean re-record
//! needs only HPF -> comp -> limit, which is this v1.

use crate::dsp::{self, AudioBuffer};
use crate::limiter::BrickwallLimiter;

/// Vocal-chain parameters. Defaults tuned for a close, dry voice take.
#[derive(Debug, Clone, Copy)]
pub struct HealingParams {
    /// High-pass cutoff (Hz) — clears handling rumble + plosives. ~90 for voice.
    pub hpf_hz: f32,
    /// Compressor threshold (dBFS).
    pub comp_threshold_db: f32,
    /// Compressor ratio (>= 1.0).
    pub comp_ratio: f32,
    /// Compressor attack (ms).
    pub comp_attack_ms: f32,
    /// Compressor release (ms).
    pub comp_release_ms: f32,
    /// Make-up gain (dB) after compression.
    pub makeup_db: f32,
    /// Brickwall ceiling (dBFS).
    pub ceiling_db: f32,
}

impl Default for HealingParams {
    fn default() -> Self {
        Self {
            hpf_hz: 90.0,
            comp_threshold_db: -18.0,
            comp_ratio: 3.0,
            comp_attack_ms: 8.0,
            comp_release_ms: 120.0,
            makeup_db: 4.0,
            ceiling_db: -1.0,
        }
    }
}

#[inline]
fn db_to_lin(db: f32) -> f32 { 10.0f32.powf(db / 20.0) }
#[inline]
fn lin_to_db(x: f32) -> f32 { 20.0 * x.max(1e-9).log10() }

/// Feed-forward peak compressor — one channel, in place. Our own DSP.
fn compress(ch: &mut [f32], sr: u32, thr_db: f32, ratio: f32, atk_ms: f32, rel_ms: f32, makeup_db: f32) {
    let atk = (-1.0f32 / ((atk_ms / 1000.0) * sr as f32)).exp();
    let rel = (-1.0f32 / ((rel_ms / 1000.0) * sr as f32)).exp();
    let makeup = db_to_lin(makeup_db);
    let mut env = 0.0f32;
    for s in ch.iter_mut() {
        let x = s.abs();
        let coeff = if x > env { atk } else { rel };
        env = coeff * env + (1.0 - coeff) * x;
        let env_db = lin_to_db(env);
        let gain_db = if env_db > thr_db {
            let over = env_db - thr_db;
            over / ratio - over // negative => gain reduction
        } else {
            0.0
        };
        *s *= db_to_lin(gain_db) * makeup;
    }
}

/// Brickwall-limit the whole buffer (reuses the master limiter), in place.
fn limit(buf: &mut AudioBuffer, ceiling_db: f32) {
    let ch = buf.channels().max(1);
    let frames = buf.len();
    if frames == 0 {
        return;
    }
    let mut input = vec![0.0f64; frames * ch];
    for c in 0..ch {
        for i in 0..frames {
            input[i * ch + c] = buf.samples[c][i] as f64;
        }
    }
    // Use process_block (correct look-ahead impl). NOT process() — that one is a broken
    // stub: it never advances write_pos and stores the post-gain value back into the delay
    // slot, so it outputs digital silence (the source even comments "Oops"). Retire process().
    let mut output = vec![0.0f64; frames * ch];
    let mut lim = BrickwallLimiter::new(ceiling_db as f64, 1.0, 50.0, buf.sample_rate, ch);
    lim.process_block(&input, &mut output, frames);
    for c in 0..ch {
        for i in 0..frames {
            buf.samples[c][i] = output[i * ch + c] as f32;
        }
    }
}

/// Heal a voice take: high-pass -> soft-knee compression -> brickwall limit.
pub fn heal_voice(buf: AudioBuffer, p: &HealingParams) -> AudioBuffer {
    let mut buf = dsp::highpass(buf, p.hpf_hz);
    let sr = buf.sample_rate;
    for ch in buf.samples.iter_mut() {
        compress(ch, sr, p.comp_threshold_db, p.comp_ratio, p.comp_attack_ms, p.comp_release_ms, p.makeup_db);
    }
    limit(&mut buf, p.ceiling_db);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heal_voice_removes_dc_and_holds_ceiling() {
        let sr = 48_000u32;
        let n = sr as usize; // 1 second
        let mut ch = vec![0.0f32; n];
        for (i, s) in ch.iter_mut().enumerate() {
            let t = i as f32 / sr as f32;
            *s = 0.5 + 1.4 * (2.0 * std::f32::consts::PI * 200.0 * t).sin(); // DC + hot 200 Hz
        }
        let out = heal_voice(
            AudioBuffer { samples: vec![ch], sample_rate: sr },
            &HealingParams::default(),
        );
        let peak = out.samples[0].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        let rms = (out.samples[0].iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>()
            / out.samples[0].len() as f64)
            .sqrt() as f32;
        // GUARD the false-green that bit us: a broken limiter zeroed the output and the old
        // test still passed because silence satisfies peak<=ceiling AND dc~0. The signal MUST
        // survive — assert it is audibly present, not just within bounds.
        assert!(peak > 0.2, "output is silent / collapsed: peak {peak}");
        assert!(rms > 0.02, "output RMS collapsed (near-silent): rms {rms}");
        assert!(peak <= 0.95, "peak {peak} exceeds ceiling");
        let mean: f32 = out.samples[0].iter().sum::<f32>() / out.samples[0].len() as f32;
        assert!(mean.abs() < 0.05, "DC not removed: mean {mean}");
    }
}
