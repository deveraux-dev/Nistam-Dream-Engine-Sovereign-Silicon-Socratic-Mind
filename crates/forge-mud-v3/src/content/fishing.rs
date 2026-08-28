//! Prairie river fishing catches for 13moons lore.

/// Catches available in the Prairie river.
pub const CATCHES: &[(&str, &str)] = &[
    ("sturgeon", "ancient river giant, scales like moonlight"),
    ("pike", "fierce predator of the shallows"),
    ("goldeye", "small swift fish that gleams golden"),
    ("burbot", "strange bottom-dweller of dark waters"),
    ("moonwhisper", "glows faintly under the three moons"),
    ("duskfin", "twilight phantom that appears at dusk"),
    ("starscale", "rare catch studded with luminous points"),
    ("silverbrook", "fleet-footed sprite of rushing waters"),
    ("nightpulse", "deep dweller with rhythmic glow"),
    ("frostfang", "chilled predator from the depths"),
    ("tidewheel", "spiraling fish that moves like water"),
    ("veilsong", "elusive catch that hums softly"),
    ("glimmerscale", "fae-touched catch that flickers between colors"),
    ("wishtail", "grants one glance of what almost was, then swims on"),
    ("thornfin", "prickled with thistle spines, smells of old bargains"),
    ("hollowgill", "hollow-eyed catch that hums a fae lullaby"),
    ("mothkoi", "moth-winged fish drawn to lantern light on the water"),
    ("ringscale", "scaled in perfect circles, never caught the same way twice"),
    ("starborn", "shimmering catch pulled from the deepest starlit waters, forgets what it saw"),
    ("thornscale", "armored in thorn-spines, swims in circles the bargain-bound must follow"),
    ("bonecall", "deep dweller with a call like a funeral song"),
    ("mistwhisper", "fades at noon, appears only in dawn's haze and dusk's veil"),
    ("windtread", "swift phantom that moves with the breath of air itself"),
    ("starforgot", "pale catch that leaves no memory of being caught"),
    ("pricedeep", "dark dweller that hums the true cost of wishes"),
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_catches() {
        assert!(super::CATCHES.len() >= 25);
        for (name, note) in super::CATCHES {
            assert!(!name.is_empty(), "catch name must not be empty");
            assert!(!note.is_empty(), "catch note must not be empty");
            assert!(name.is_ascii(), "catch name must be ASCII");
            assert!(note.is_ascii(), "catch note must be ASCII");
        }
    }
}
