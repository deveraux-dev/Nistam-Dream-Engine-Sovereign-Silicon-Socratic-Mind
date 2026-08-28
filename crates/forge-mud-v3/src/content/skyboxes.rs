//! Skybox and vibe engine definitions for forge-mud-v3.

/// Catalog of skyboxes with descriptions tied to 13moons prairie-alchemy lore.
pub const SKYBOXES: &[(&str, &str)] = &[
    ("dawn_verdant", "Soft amber glow over emerald plains"),
    ("high_noon_sapphire", "Brilliant blue vault unmarred by cloud"),
    ("dusk_crimson", "Deep red bleeding toward purple shadows"),
    // The one skybox with a real renderer behind it (not just a display
    // string): `shell/src/mud.rs` paints the HYG-baked star catalog
    // (`shell/src/celestial_hyg.rs`) every frame, tier-2 criticality by
    // default — this description names what's actually on glass, not an
    // aspiration.
    ("midnight_void", "The real HYG-catalogued sky, tier-adaptive, named lights carrying their boons"),
    ("stormhead", "Roiling grey mass with fractured light"),
    ("hardfrost_still", "Cold cloudless dark, everything under it rimed and holding"),
    ("blood_moon", "Sanguine satellite rising through haze"),
    ("frost_dawn", "Silver-white chill breaking into day"),
    ("drought_bronze", "Weathered tan stretched over badlands"),
    ("green_murmur", "Chlorophyll-dense canopy-reflected glow"),
    ("iron_twilight", "Rust-toned atmosphere settling inward"),
];

/// Catalog of vibe engines that sculpt emotional tone across the world.
pub const VIBES: &[(&str, &str)] = &[
    ("chimera", "Fractured harmony of ancient contradictions"),
    ("vibe_vector", "Linear gradient of pure emotional thrust"),
    ("vibe_matrix", "Layered emotional field forming coherence"),
    ("glyph_tracer", "Sigil-etched whispers threading all space"),
    ("ember_hum", "Warmth rising as soft vibration"),
    ("void_echo", "Reverb of silence bouncing endlessly"),
    ("brass_knell", "Resonant bell-tone marking epoch"),
    ("moss_breath", "Living slow exhalation of green"),
    ("sand_drag", "Friction-hiss of deep earth grinding"),
    ("siren_peak", "High crystalline call piercing clarity"),
    ("root_deep", "Grounded tone anchoring all above"),
];

#[cfg(test)]
mod tests {
    #[test]
    fn validate_skyboxes_and_vibes() {
        assert!(super::SKYBOXES.len() >= 10, "SKYBOXES must have >= 10 entries");
        assert!(super::VIBES.len() >= 10, "VIBES must have >= 10 entries");

        let vibe_names: Vec<&str> = super::VIBES.iter().map(|v| v.0).collect();
        assert!(vibe_names.contains(&"chimera"), "VIBES must contain 'chimera'");
        assert!(vibe_names.contains(&"vibe_vector"), "VIBES must contain 'vibe_vector'");
        assert!(vibe_names.contains(&"vibe_matrix"), "VIBES must contain 'vibe_matrix'");
        assert!(vibe_names.contains(&"glyph_tracer"), "VIBES must contain 'glyph_tracer'");

        for entry in super::SKYBOXES {
            assert!(!entry.0.is_empty(), "SKYBOXES name must not be empty");
            assert!(!entry.1.is_empty(), "SKYBOXES description must not be empty");
            assert!(entry.0.is_ascii(), "SKYBOXES name must be ASCII");
            assert!(entry.1.is_ascii(), "SKYBOXES description must be ASCII");
        }

        for entry in super::VIBES {
            assert!(!entry.0.is_empty(), "VIBES name must not be empty");
            assert!(!entry.1.is_empty(), "VIBES mood must not be empty");
            assert!(entry.0.is_ascii(), "VIBES name must be ASCII");
            assert!(entry.1.is_ascii(), "VIBES mood must be ASCII");
        }
    }
}
