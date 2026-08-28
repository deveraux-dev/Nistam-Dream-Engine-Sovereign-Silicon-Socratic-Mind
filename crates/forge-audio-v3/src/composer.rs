//! Effect pipeline composer: chain DSP effects from TOML.

use crate::dsp::{self, AudioBuffer};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectDef {
    TimeStretch { factor: f64 },
    PitchShift { semitones: f32 },
    Reverb { room_size: f32, damping: f32, mix: f32 },
    Delay { time_ms: f32, feedback: f32, mix: f32 },
    Lowpass { cutoff_hz: f32 },
    Highpass { cutoff_hz: f32 },
    Granular { grain_ms: f32, density: f32, scatter: f32, seed: u64 },
    Reverse,
    PhaseVocoder { stretch_factor: f32 },
    NotchFilter { freq_hz: f32, q: f32, #[serde(default)] gain_db: f32 },
    LfoModulate { rate_hz: f32, depth: f32 },
    ReversePreswell { segment_ms: f32 },
    Bitcrush { bit_depth: u32, sample_rate: u32 },
    Bandpass { low_hz: f32, high_hz: f32 },
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectsPipeline {
    pub effects: Vec<EffectDef>,
}

impl EffectsPipeline {
    pub fn from_toml(input: &str) -> Result<Self, String> {
        toml::from_str(input).map_err(|e| format!("Effects TOML error: {}", e))
    }

    pub fn apply(&self, mut buf: AudioBuffer) -> Result<AudioBuffer, String> {
        for fx in self.effects.iter() {
            buf = match fx {
                EffectDef::TimeStretch { factor } => dsp::time_stretch(&buf, *factor)?,
                EffectDef::PitchShift { semitones } => dsp::pitch_shift(&buf, *semitones)?,
                EffectDef::Reverb { room_size, damping, mix } => dsp::reverb(buf, *room_size, *damping, *mix),
                EffectDef::Delay { time_ms, feedback, mix } => dsp::delay(buf, *time_ms, *feedback, *mix),
                EffectDef::Lowpass { cutoff_hz } => dsp::lowpass(buf, *cutoff_hz),
                EffectDef::Highpass { cutoff_hz } => dsp::highpass(buf, *cutoff_hz),
                EffectDef::Granular { grain_ms, density, scatter, seed } => dsp::granular(&buf, *grain_ms, *density, *scatter, *seed),
                EffectDef::Reverse => dsp::reverse(&buf),
                EffectDef::PhaseVocoder { stretch_factor } => dsp::phase_vocoder(&buf, *stretch_factor),
                EffectDef::NotchFilter { freq_hz, q, gain_db } => dsp::notch_filter(&buf, *freq_hz, *q, *gain_db),
                EffectDef::LfoModulate { rate_hz, depth } => dsp::lfo_modulate(&buf, *rate_hz, *depth),
                EffectDef::ReversePreswell { segment_ms } => dsp::reverse_preswell(&buf, *segment_ms),
                EffectDef::Bitcrush { bit_depth, sample_rate } => dsp::bitcrush(&buf, *bit_depth, *sample_rate),
                EffectDef::Bandpass { low_hz, high_hz } => dsp::bandpass(&buf, *low_hz, *high_hz),
            };
        }
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_effect_types() {
        let toml = r#"
[[effects]]
type = "phase_vocoder"
stretch_factor = 2.5

[[effects]]
type = "notch_filter"
freq_hz = 3000.0
q = 8.0

[[effects]]
type = "bitcrush"
bit_depth = 8
sample_rate = 11025

[[effects]]
type = "bandpass"
low_hz = 200.0
high_hz = 4000.0

[[effects]]
type = "reverse_preswell"
segment_ms = 150.0
"#;
        let p = EffectsPipeline::from_toml(toml).unwrap();
        assert_eq!(p.effects.len(), 5);
    }
}
