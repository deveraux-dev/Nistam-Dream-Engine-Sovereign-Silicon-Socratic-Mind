//! EffectType and ActiveEffect — audio effect definitions.

use serde::{Deserialize, Serialize};

use super::track::ValidationError;

/// Audio effect variants with associated parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EffectType {
    /// Low-pass filter. `cutoff_hz` in [20.0, 20000.0].
    LowPass { cutoff_hz: f32 },
    /// High-pass filter. `cutoff_hz` in [20.0, 20000.0].
    HighPass { cutoff_hz: f32 },
    /// Reverb effect. `decay` and `wet` in [0.0, 1.0].
    Reverb { decay: f32, wet: f32 },
    /// Delay effect. `delay_ms` > 0.0, `feedback` in [0.0, 1.0].
    Delay { delay_ms: f32, feedback: f32 },
    /// Compressor. `ratio` >= 1.0.
    Compressor { threshold: f32, ratio: f32 },
    /// Flanger effect. `rate_hz` > 0.0, `depth` in [0.0, 1.0].
    Flanger { rate_hz: f32, depth: f32 },
    /// Echo effect. `time_ms` > 0.0, `feedback` in [0.0, 1.0].
    Echo { time_ms: f32, feedback: f32 },
    /// Placeholder variant for backward compatibility with existing tests.
    Placeholder,
}

impl EffectType {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            EffectType::LowPass { cutoff_hz } | EffectType::HighPass { cutoff_hz } => {
                validate_cutoff(*cutoff_hz)?;
            }
            EffectType::Reverb { decay, wet } => {
                validate_unit("decay", *decay)?;
                validate_unit("wet", *wet)?;
            }
            EffectType::Delay { delay_ms, feedback } => {
                validate_positive("delay_ms", *delay_ms)?;
                validate_unit("feedback", *feedback)?;
            }
            EffectType::Compressor { threshold: _, ratio } => {
                if *ratio < 1.0 || ratio.is_nan() {
                    return Err(ValidationError::InvalidField {
                        field: "ratio",
                        message: format!("must be >= 1.0, got {ratio}"),
                    });
                }
            }
            EffectType::Flanger { rate_hz, depth } => {
                validate_positive("rate_hz", *rate_hz)?;
                validate_unit("depth", *depth)?;
            }
            EffectType::Echo { time_ms, feedback } => {
                validate_positive("time_ms", *time_ms)?;
                validate_unit("feedback", *feedback)?;
            }
            EffectType::Placeholder => {}
        }
        Ok(())
    }

    pub fn validated(self) -> Result<Self, ValidationError> {
        self.validate()?;
        Ok(self)
    }
}

/// An effect instance applied to a deck.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEffect {
    /// Unique identifier for this effect instance.
    pub id: usize,
    /// The effect type and its parameters.
    pub effect: EffectType,
    /// Whether this effect is currently enabled.
    pub enabled: bool,
}

// ── helpers ──────────────────────────────────────────────────────────

fn validate_cutoff(hz: f32) -> Result<(), ValidationError> {
    if !(20.0..=20000.0).contains(&hz) || hz.is_nan() {
        return Err(ValidationError::InvalidField {
            field: "cutoff_hz",
            message: format!("must be in [20.0, 20000.0], got {hz}"),
        });
    }
    Ok(())
}

fn validate_unit(field: &'static str, v: f32) -> Result<(), ValidationError> {
    if !(0.0..=1.0).contains(&v) || v.is_nan() {
        return Err(ValidationError::InvalidField {
            field,
            message: format!("must be in [0.0, 1.0], got {v}"),
        });
    }
    Ok(())
}

fn validate_positive(field: &'static str, v: f32) -> Result<(), ValidationError> {
    if v <= 0.0 || v.is_nan() || v.is_infinite() {
        return Err(ValidationError::InvalidField {
            field,
            message: format!("must be > 0.0, got {v}"),
        });
    }
    Ok(())
}
