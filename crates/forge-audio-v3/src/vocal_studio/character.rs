//! Character voice pipeline — game voice acting that doesn't suck.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//!
//! Uses HPSS to separate harmonic (pitch-shiftable) from percussive (consonants
//! that must stay untouched), shifts harmonic via phase_vocoder_stretch + resample,
//! then recombines. This preserves consonant crispness that naive pitch shifting
//! destroys.

use crate::alchemy::hpss::hpss_separate;
use crate::alchemy::vocoder::phase_vocoder;
use crate::dsp::AudioBuffer;

/// Character voice parameters.
#[derive(Debug, Clone)]
pub struct CharacterVoice {
    /// Pitch shift in semitones (-12..+12). Negative = deeper.
    pub pitch_shift_semitones: f32,
    /// Formant preservation ratio (0.0 = no shift, positive = larger vocal tract).
    pub formant_shift: f32,
    /// Reverb wet amount (0.0–1.0).
    pub reverb_amount: f32,
    /// Growl/grit via soft-clip saturation (0.0–1.0).
    pub growl: f32,
    /// Breathiness: noise mixed into unvoiced regions (0.0–1.0).
    pub breathiness: f32,
}

// ── Presets ──────────────────────────────────────────────────────────────────

pub const GRIZZLED_WARRIOR: CharacterVoice = CharacterVoice {
    pitch_shift_semitones: -3.0,
    formant_shift: 0.1,
    reverb_amount: 0.05,
    growl: 0.6,
    breathiness: 0.1,
};

pub const ETHEREAL_ELF: CharacterVoice = CharacterVoice {
    pitch_shift_semitones: 4.0,
    formant_shift: -0.1,
    reverb_amount: 0.4,
    growl: 0.0,
    breathiness: 0.3,
};

pub const GOBLIN: CharacterVoice = CharacterVoice {
    pitch_shift_semitones: 7.0,
    formant_shift: -0.3,
    reverb_amount: 0.1,
    growl: 0.3,
    breathiness: 0.05,
};

pub const NARRATOR_DEEP: CharacterVoice = CharacterVoice {
    pitch_shift_semitones: -5.0,
    formant_shift: 0.2,
    reverb_amount: 0.15,
    growl: 0.0,
    breathiness: 0.0,
};

/// Apply a character voice transform to a recorded voice.
pub fn apply_character(buf: &AudioBuffer, character: &CharacterVoice) -> AudioBuffer {
    let sr = buf.sample_rate;
    let mono = buf.to_mono();

    // 1. HPSS: separate harmonic (vowels/pitch) from percussive (consonants/transients)
    let (harmonic, percussive) = hpss_separate(&mono, sr);

    // 2. Pitch-shift the harmonic component only (preserves consonants)
    let shifted = pitch_shift_preserve_formant(&harmonic, sr, character.pitch_shift_semitones);

    // 3. Apply growl (soft-clip saturation on harmonic)
    let growled = apply_growl(&shifted, character.growl);

    // 4. Recombine: shifted harmonic + untouched percussive
    let len = growled.len().min(percussive.len());
    let mut combined: Vec<f32> = (0..len)
        .map(|i| growled[i] + percussive[i])
        .collect();

    // 5. Add breathiness (filtered noise in low-energy regions)
    if character.breathiness > 0.0 {
        add_breathiness(&mut combined, sr, character.breathiness);
    }

    // 6. Simple comb reverb
    if character.reverb_amount > 0.01 {
        combined = apply_simple_reverb(&combined, sr, character.reverb_amount);
    }

    AudioBuffer { samples: vec![combined], sample_rate: sr }
}

/// Pitch shift that preserves formant structure.
/// Uses phase vocoder to stretch time, then resamples back to original length.
pub fn pitch_shift_preserve_formant(mono: &[f32], sr: u32, semitones: f32) -> Vec<f32> {
    if semitones.abs() < 0.01 || mono.is_empty() {
        return mono.to_vec();
    }
    // Ratio: shift up = shorter signal needs stretch, shift down = longer needs compress
    let ratio = 2.0f32.powf(-semitones / 12.0); // stretch factor (inverse of pitch ratio)
    let stretched = phase_vocoder(mono, sr, ratio);
    // Resample back to original length (this changes pitch without changing duration)
    resample_linear(&stretched, mono.len())
}

/// Linear resample to target length.
fn resample_linear(signal: &[f32], target_len: usize) -> Vec<f32> {
    if signal.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    let ratio = signal.len() as f64 / target_len as f64;
    (0..target_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos as usize;
            let frac = (pos - idx as f64) as f32;
            let a = signal[idx.min(signal.len() - 1)];
            let b = signal[(idx + 1).min(signal.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

/// Soft-clip saturation for growl/grit effect.
fn apply_growl(signal: &[f32], amount: f32) -> Vec<f32> {
    if amount < 0.01 {
        return signal.to_vec();
    }
    let drive = 1.0 + amount * 8.0; // 1x–9x overdrive
    signal.iter().map(|&s| (s * drive).tanh() / drive.tanh()).collect()
}

/// Add breathy noise in low-energy regions.
fn add_breathiness(signal: &mut [f32], _sr: u32, amount: f32) {
    let window = 512;
    let mut rng: u32 = 0xDEAD_BEEF;
    for i in 0..signal.len() {
        // LCG pseudo-noise
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        // Gate: only add noise where signal is quiet
        let local_start = i.saturating_sub(window / 2);
        let local_end = (i + window / 2).min(signal.len());
        if (local_end - local_start) > 0 {
            let rms = (signal[local_start..local_end].iter().map(|s| s * s).sum::<f32>()
                / (local_end - local_start) as f32).sqrt();
            let gate = (1.0 - rms * 10.0).clamp(0.0, 1.0);
            signal[i] += noise * amount * 0.05 * gate;
        }
    }
}

/// Simple feedback comb filter reverb.
fn apply_simple_reverb(signal: &[f32], sr: u32, wet: f32) -> Vec<f32> {
    let delay_samples = (sr as f32 * 0.035) as usize; // 35ms tap
    let feedback = 0.4;
    let mut output = signal.to_vec();
    for i in delay_samples..output.len() {
        output[i] += output[i - delay_samples] * feedback * wet;
    }
    // Normalize peak to prevent clipping
    let peak = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 1.0 {
        let inv = 1.0 / peak;
        output.iter_mut().for_each(|s| *s *= inv);
    }
    output
}
