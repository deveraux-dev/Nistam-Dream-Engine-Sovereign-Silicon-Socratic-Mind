//! Weaver/Arbiter cartridge and deterministic gating engine.
//!
//! The Weaver generates game assets (items, enemies, locations) with rich lore,
//! stats, reach extensions, hermetic principles, and color swatches.
//!
//! The Arbiter deterministically judges each entity against hard mechanical laws:
//! 1. Power level ceiling: 0..=255 (unless licensed with `uses_byte_overflow`).
//! 2. 7 Hermetic Principles: closed set validation.
//! 3. Reach extension bounds: Precise (<50), Standard (50..=199), Ghost (>=200).
//! 4. Swatch hex format: strict #RRGGBB.
//! 5. Entity classification: item, enemy, location.

use serde::{Deserialize, Serialize};

/// Canonical power ceiling (0..=255).
pub const POWER_MAX: u32 = 255;

/// The seven hermetic principles admitted by the engine.
pub const HERMETIC_PRINCIPLES: [&str; 7] = [
    "Mentalism",
    "Correspondence",
    "Vibration",
    "Polarity",
    "Rhythm",
    "Cause/Effect",
    "Gender",
];

/// The three entity kinds admitted by the schema.
pub const ENTITY_TYPES: [&str; 3] = ["item", "enemy", "location"];

/// Mandatory schema keys.
pub const REQUIRED_KEYS: [&str; 8] = [
    "name",
    "type",
    "power_level",
    "reach_extension",
    "hermetic_principle",
    "shadow_trait",
    "description",
    "metadata",
];

/// Chaos-mode tag licensing an overflow past 255.
pub const OVERFLOW_TAG: &str = "uses_byte_overflow";

/// Reach extension classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reach {
    /// Hitbox strictly matches sprite geometry (<50).
    Precise,
    /// Standard 16-bit bounding box (50..=199).
    Standard,
    /// Ghost extension reaching past sprite boundary (>=200).
    Ghost,
}

impl Reach {
    /// Map a reach byte to its canonical reach band.
    pub const fn of(byte: u8) -> Self {
        if byte >= 200 {
            Self::Ghost
        } else if byte < 50 {
            Self::Precise
        } else {
            Self::Standard
        }
    }
}

/// Metadata payload for Weaver entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WeaverMetadata {
    /// Hex palette color swatches (#RRGGBB).
    #[serde(default)]
    pub hex_palette: Vec<String>,
    /// Logical capability and behavior tags.
    #[serde(default)]
    pub logic_flags: Vec<String>,
}

/// Strongly-typed Weaver entity in RON or JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaverEntity {
    /// Canonical name of the entity.
    pub name: String,
    /// Classification (item, enemy, location).
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Power level (0..=255, or unbound with overflow tag).
    pub power_level: u32,
    /// Reach extension metric.
    pub reach_extension: u32,
    /// Governed hermetic principle.
    pub hermetic_principle: String,
    /// Required psychological / shadow vulnerability trait.
    pub shadow_trait: String,
    /// Narrative sensory description.
    pub description: String,
    /// Palette and logic metadata.
    #[serde(default)]
    pub metadata: WeaverMetadata,
}

/// Complete Weaver/Arbiter cartridge structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WeaverCart {
    /// Title of the cartridge.
    pub title: String,
    /// Author or architect signature.
    pub author: String,
    /// Closed set of hermetic principles defined in cart.
    #[serde(default)]
    pub principles: Vec<String>,
    /// Collection of authored or woven entities.
    #[serde(default)]
    pub entities: Vec<WeaverEntity>,
}

/// Validate #RRGGBB hex swatch.
pub fn is_hex16(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Deterministic Arbiter judge for a typed Weaver entity.
pub fn judge_entity(entity: &WeaverEntity) -> Result<(), &'static str> {
    if !ENTITY_TYPES.contains(&entity.entity_type.as_str()) {
        return Err("invented an entity type outside item/enemy/location");
    }

    let licensed = entity.metadata.logic_flags.iter().any(|f| f == OVERFLOW_TAG);
    if entity.power_level > POWER_MAX && !licensed {
        return Err("blew past the powercurve ceiling without the byte-overflow tag");
    }
    if entity.reach_extension > POWER_MAX && !licensed {
        return Err("reach extension exceeded byte bounds without overflow tag");
    }

    if !HERMETIC_PRINCIPLES.contains(&entity.hermetic_principle.as_str()) {
        return Err("named a principle outside the seven");
    }

    if entity.shadow_trait.trim().is_empty() {
        return Err("left the shadow trait empty");
    }

    for swatch in &entity.metadata.hex_palette {
        if !is_hex16(swatch) {
            return Err("emitted a palette entry that is not a #RRGGBB hex code");
        }
    }

    Ok(())
}

/// Parse and validate a complete Weaver/Arbiter RON cartridge.
pub fn load_ron(ron_src: &str) -> Result<WeaverCart, String> {
    let cart: WeaverCart = ron::from_str(ron_src)
        .map_err(|e| format!("Weaver/Arbiter RON parse refusal: {e}"))?;

    for (idx, entity) in cart.entities.iter().enumerate() {
        judge_entity(entity).map_err(|err| {
            format!("Arbiter refusal on entity #{idx} ('{}'): {err}", entity.name)
        })?;
    }

    Ok(cart)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entity() -> WeaverEntity {
        WeaverEntity {
            name: "Vitreous Slag Gauntlet".to_string(),
            entity_type: "item".to_string(),
            power_level: 180,
            reach_extension: 210,
            hermetic_principle: "Vibration".to_string(),
            shadow_trait: "the strength it will not admit wanting".to_string(),
            description: "Rusted iron, hammered over something that screamed.".to_string(),
            metadata: WeaverMetadata {
                hex_palette: vec!["#2A1B3D".to_string(), "#8C4A2F".to_string()],
                logic_flags: vec!["echo_strike".to_string()],
            },
        }
    }

    #[test]
    fn lawful_entity_passes_judge() {
        assert_eq!(judge_entity(&sample_entity()), Ok(()));
    }

    #[test]
    fn reach_bands_classified_correctly() {
        assert_eq!(Reach::of(10), Reach::Precise);
        assert_eq!(Reach::of(100), Reach::Standard);
        assert_eq!(Reach::of(220), Reach::Ghost);
    }

    #[test]
    fn unlawful_power_refused_without_tag() {
        let mut e = sample_entity();
        e.power_level = 350;
        assert_eq!(
            judge_entity(&e),
            Err("blew past the powercurve ceiling without the byte-overflow tag")
        );

        e.metadata.logic_flags.push(OVERFLOW_TAG.to_string());
        assert_eq!(judge_entity(&e), Ok(()));
    }

    #[test]
    fn ron_cartridge_roundtrip() {
        let ron_str = r##"(
            title: "Ironroot Weaver Grimoire",
            author: "Sean Morin",
            principles: ["Mentalism", "Correspondence", "Vibration", "Polarity", "Rhythm", "Cause/Effect", "Gender"],
            entities: [
                (
                    name: "Vitreous Slag Gauntlet",
                    type: "item",
                    power_level: 180,
                    reach_extension: 210,
                    hermetic_principle: "Vibration",
                    shadow_trait: "the strength it will not admit wanting",
                    description: "Rusted iron, hammered over something that screamed.",
                    metadata: (
                        hex_palette: ["#2A1B3D", "#8C4A2F"],
                        logic_flags: ["echo_strike"],
                    ),
                ),
            ],
        )"##;

        let cart = load_ron(ron_str).expect("lawful RON must parse and validate");
        assert_eq!(cart.entities.len(), 1);
        assert_eq!(cart.entities[0].name, "Vitreous Slag Gauntlet");
    }

    #[test]
    fn load_ironroot_weaver_arbiter_ron_file() {
        let ron_src = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../carts/ironroot/weaver_arbiter.ron"),
        )
        .expect("weaver_arbiter.ron must be readable");
        let cart = load_ron(&ron_src).expect("weaver_arbiter.ron must pass Arbiter judge");
        assert_eq!(cart.entities.len(), 5);
        assert_eq!(cart.entities[0].name, "Vitreous Slag Gauntlet");
        assert_eq!(cart.entities[3].name, "Void-Born Singularity Relic");
        assert_eq!(cart.entities[3].power_level, 480);
    }
}
