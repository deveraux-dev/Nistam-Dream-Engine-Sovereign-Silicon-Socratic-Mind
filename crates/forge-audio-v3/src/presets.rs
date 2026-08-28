//! Named macro presets: intensity 0.0-1.0 scales all effect parameters.

use crate::composer::EffectsPipeline;

pub const PRESET_NAMES: &[&str] = &["echo", "glitch", "far-away", "shatter", "warm", "ghost", "freeze", "hollow", "dread", "distance", "warmth", "combat_mood", "ancient"];

/// Load a preset TOML, then scale all numeric parameters by intensity.
pub fn load_preset(name: &str, intensity: f32) -> Result<EffectsPipeline, String> {
    let toml_str = preset_toml(name)?;
    let mut pipeline = EffectsPipeline::from_toml(&toml_str)?;
    scale_intensity(&mut pipeline, intensity);
    Ok(pipeline)
}

fn preset_toml(name: &str) -> Result<String, String> {
    match name {
        "echo" => Ok(ECHO.into()),
        "glitch" => Ok(GLITCH.into()),
        "far-away" => Ok(FAR_AWAY.into()),
        "shatter" => Ok(SHATTER.into()),
        "warm" | "warmth" => Ok(WARM.into()),
        "ghost" => Ok(GHOST.into()),
        "freeze" => Ok(FREEZE.into()),
        "hollow" => Ok(HOLLOW.into()),
        "dread" => Ok(DREAD.into()),
        "distance" => Ok(DISTANCE.into()),
        "combat_mood" => Ok(DREAD.into()), // combat uses dread preset as base
        "ancient" => Ok(HOLLOW.into()), // ancient era uses hollow preset as base
        _ => Err(format!("Unknown preset: '{}'. Available: {:?}", name, PRESET_NAMES)),
    }
}

fn scale_intensity(pipeline: &mut EffectsPipeline, intensity: f32) {
    use crate::composer::EffectDef;
    for fx in &mut pipeline.effects {
        match fx {
            EffectDef::Reverb { mix, .. } => *mix *= intensity,
            EffectDef::Delay { mix, feedback, .. } => { *mix *= intensity; *feedback *= intensity; }
            EffectDef::PitchShift { semitones } => *semitones *= intensity,
            EffectDef::PhaseVocoder { stretch_factor } => *stretch_factor = 1.0 + (*stretch_factor - 1.0) * intensity,
            EffectDef::Granular { density, scatter, .. } => { *density *= intensity; *scatter *= intensity; }
            EffectDef::NotchFilter { q, .. } => *q = 1.0 + (*q - 1.0) * intensity,
            EffectDef::LfoModulate { depth, .. } => *depth *= intensity,
            EffectDef::Bitcrush { bit_depth, .. } => *bit_depth = 16 - ((16 - *bit_depth) as f32 * intensity) as u32,
            EffectDef::ReversePreswell { segment_ms } => *segment_ms *= intensity,
            EffectDef::Lowpass { cutoff_hz } => *cutoff_hz = 16000.0 - (16000.0 - *cutoff_hz) * intensity,
            EffectDef::Highpass { cutoff_hz } => *cutoff_hz *= intensity,
            EffectDef::Bandpass { .. }
            | EffectDef::TimeStretch { .. } | EffectDef::Reverse => {}
        }
    }
}

const ECHO: &str = r#"
[[effects]]
type = "pitch_shift"
semitones = -5.0

[[effects]]
type = "reverb"
room_size = 3.0
damping = 0.3
mix = 0.5

[[effects]]
type = "lowpass"
cutoff_hz = 3000.0

[[effects]]
type = "delay"
time_ms = 400.0
feedback = 0.4
mix = 0.3
"#;

const GLITCH: &str = r#"
[[effects]]
type = "phase_vocoder"
stretch_factor = 4.0

[[effects]]
type = "bitcrush"
bit_depth = 12
sample_rate = 22050

[[effects]]
type = "reverb"
room_size = 2.5
damping = 0.2
mix = 0.4
"#;

const FAR_AWAY: &str = r#"
[[effects]]
type = "lowpass"
cutoff_hz = 1500.0

[[effects]]
type = "reverb"
room_size = 3.5
damping = 0.6
mix = 0.6

[[effects]]
type = "delay"
time_ms = 300.0
feedback = 0.2
mix = 0.15
"#;

const SHATTER: &str = r#"
[[effects]]
type = "bitcrush"
bit_depth = 4
sample_rate = 6000

[[effects]]
type = "lfo_modulate"
rate_hz = 15.0
depth = 0.8

[[effects]]
type = "reverb"
room_size = 1.5
damping = 0.4
mix = 0.35
"#;

const WARM: &str = r#"
[[effects]]
type = "lowpass"
cutoff_hz = 6000.0

[[effects]]
type = "reverb"
room_size = 1.2
damping = 0.7
mix = 0.2

[[effects]]
type = "delay"
time_ms = 150.0
feedback = 0.15
mix = 0.1
"#;

const GHOST: &str = r#"
[[effects]]
type = "pitch_shift"
semitones = -7.0

[[effects]]
type = "reverb"
room_size = 4.0
damping = 0.2
mix = 0.65

[[effects]]
type = "lowpass"
cutoff_hz = 2000.0

[[effects]]
type = "delay"
time_ms = 500.0
feedback = 0.5
mix = 0.35
"#;

const FREEZE: &str = r#"
[[effects]]
type = "delay"
time_ms = 80.0
feedback = 0.92
mix = 0.7

[[effects]]
type = "reverb"
room_size = 5.0
damping = 0.3
mix = 0.75

[[effects]]
type = "lowpass"
cutoff_hz = 2000.0
"#;

const HOLLOW: &str = r#"
[[effects]]
type = "highpass"
cutoff_hz = 400.0

[[effects]]
type = "notch_filter"
freq_hz = 1000.0
q = 8.0

[[effects]]
type = "reverb"
room_size = 3.5
damping = 0.3
mix = 0.55

[[effects]]
type = "delay"
time_ms = 250.0
feedback = 0.3
mix = 0.2
"#;

const DREAD: &str = r#"
[[effects]]
type = "lowpass"
cutoff_hz = 800.0

[[effects]]
type = "reverb"
room_size = 3.0
damping = 0.2
mix = 0.6

[[effects]]
type = "delay"
time_ms = 600.0
feedback = 0.6
mix = 0.4

[[effects]]
type = "bitcrush"
bit_depth = 12
sample_rate = 22050
"#;

const DISTANCE: &str = r#"
[[effects]]
type = "lowpass"
cutoff_hz = 800.0

[[effects]]
type = "reverb"
room_size = 4.0
damping = 0.7
mix = 0.7

[[effects]]
type = "delay"
time_ms = 350.0
feedback = 0.25
mix = 0.2
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_parse() {
        for name in PRESET_NAMES {
            let p = load_preset(name, 1.0);
            assert!(p.is_ok(), "preset '{}' failed: {:?}", name, p.err());
        }
    }

    #[test]
    fn intensity_zero_neutralizes() {
        let p = load_preset("ghost", 0.0).unwrap();
        // At intensity 0, reverb mix should be 0
        for fx in &p.effects {
            if let crate::composer::EffectDef::Reverb { mix, .. } = fx {
                assert_eq!(*mix, 0.0);
            }
        }
    }

    #[test]
    fn unknown_preset_errors() {
        assert!(load_preset("banana", 1.0).is_err());
    }
}
