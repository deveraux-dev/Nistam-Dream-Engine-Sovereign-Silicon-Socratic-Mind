//! Recipe Serialization — TOML round-trip for RecipeDefinition.

use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// RecipeDefinition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OscillatorType { Sine, Square, Sawtooth, Noise }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterDef {
    pub filter_type: String,
    pub cutoff_hz: f32,
    pub resonance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeDef {
    pub attack_secs: f32,
    pub decay_secs: f32,
    pub sustain_level: f32,
    pub release_secs: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModulationDef {
    pub mod_type: String,
    pub ratio: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDefinition {
    pub recipe_id: String,
    pub oscillator: OscillatorType,
    pub frequency_hz: f32,
    pub filter: Option<FilterDef>,
    pub envelope: EnvelopeDef,
    pub modulation: Option<ModulationDef>,
    pub material_class: String,
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

impl std::fmt::Display for OscillatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OscillatorType::Sine => write!(f, "Sine"),
            OscillatorType::Square => write!(f, "Square"),
            OscillatorType::Sawtooth => write!(f, "Saw"),
            OscillatorType::Noise => write!(f, "Noise"),
        }
    }
}

impl std::fmt::Display for RecipeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}({:.0}Hz)", self.material_class, self.recipe_id, self.oscillator, self.frequency_hz)?;
        if let Some(ref filter) = self.filter {
            write!(f, " → {}({:.0}Hz, Q={:.1})", filter.filter_type, filter.cutoff_hz, filter.resonance)?;
        }
        write!(f, " → AR({:.3}s, {:.3}s)", self.envelope.attack_secs, self.envelope.release_secs)?;
        if let Some(ref modulation) = self.modulation {
            write!(f, " [{}(ratio={:.2}, depth={:.2})]", modulation.mod_type, modulation.ratio, modulation.depth)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_recipe() -> RecipeDefinition {
        RecipeDefinition {
            recipe_id: "fm_clang".to_string(),
            oscillator: OscillatorType::Sine,
            frequency_hz: 800.0,
            filter: Some(FilterDef {
                filter_type: "lowpass".to_string(),
                cutoff_hz: 4000.0,
                resonance: 2.0,
            }),
            envelope: EnvelopeDef {
                attack_secs: 0.001,
                decay_secs: 0.1,
                sustain_level: 0.0,
                release_secs: 1.5,
            },
            modulation: Some(ModulationDef {
                mod_type: "FM".to_string(),
                ratio: 3.57,
                depth: 0.6,
            }),
            material_class: "Iron".to_string(),
        }
    }

    #[test]
    fn toml_round_trip() {
        let recipe = sample_recipe();
        let toml_str = toml::to_string(&recipe).unwrap();
        let back: RecipeDefinition = toml::from_str(&toml_str).unwrap();
        assert_eq!(recipe, back);
    }

    #[test]
    fn display_format() {
        let recipe = sample_recipe();
        let display = format!("{}", recipe);
        assert!(display.contains("Iron"));
        assert!(display.contains("fm_clang"));
        assert!(display.contains("Sine"));
        assert!(display.contains("800Hz"));
        assert!(display.contains("FM"));
    }

    #[test]
    fn display_no_filter_no_mod() {
        let recipe = RecipeDefinition {
            recipe_id: "additive".to_string(),
            oscillator: OscillatorType::Sine,
            frequency_hz: 220.0,
            filter: None,
            envelope: EnvelopeDef {
                attack_secs: 0.005,
                decay_secs: 0.5,
                sustain_level: 0.6,
                release_secs: 2.0,
            },
            modulation: None,
            material_class: "Organic".to_string(),
        };
        let display = format!("{}", recipe);
        assert!(display.contains("Organic additive"));
        assert!(!display.contains("→ lowpass"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_oscillator_type() -> impl Strategy<Value = OscillatorType> {
        prop_oneof![
            Just(OscillatorType::Sine),
            Just(OscillatorType::Square),
            Just(OscillatorType::Sawtooth),
            Just(OscillatorType::Noise),
        ]
    }

    fn arb_recipe_definition() -> impl Strategy<Value = RecipeDefinition> {
        (
            "[a-z][a-z0-9_]{0,10}",
            arb_oscillator_type(),
            20.0f32..20000.0,
            proptest::option::of(("[a-z]{3,10}", 20.0f32..20000.0, 0.5f32..20.0)),
            0.001f32..1.0,
            0.001f32..5.0,
            0.0f32..=1.0,
            0.001f32..10.0,
            proptest::option::of(("[a-z]{2,8}", 0.1f32..10.0, 0.0f32..1.0)),
            "[A-Z][a-z]{2,10}",
        ).prop_map(|(id, osc, freq, filter, a, d, s, r, modulation, mat)| {
            RecipeDefinition {
                recipe_id: id,
                oscillator: osc,
                frequency_hz: freq,
                filter: filter.map(|(ft, c, q)| FilterDef { filter_type: ft, cutoff_hz: c, resonance: q }),
                envelope: EnvelopeDef { attack_secs: a, decay_secs: d, sustain_level: s, release_secs: r },
                modulation: modulation.map(|(mt, ratio, depth)| ModulationDef { mod_type: mt, ratio, depth }),
                material_class: mat,
            }
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p9_toml_round_trip(recipe in arb_recipe_definition()) {
            let toml_str = toml::to_string(&recipe).unwrap();
            let back: RecipeDefinition = toml::from_str(&toml_str).unwrap();
            prop_assert_eq!(recipe, back);
        }
    }
}
