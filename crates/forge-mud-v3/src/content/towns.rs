//! Towns in the 13moons prairie-alchemy setting.

/// A collection of mystical towns scattered across the prairie realm.
pub const TOWNS: &[(&str, &str)] = &[
    ("SpiritFire Hold", "Moon-blessed forge where spirits dance in flames."),
    ("Meskanaw Ridge", "Bone-crowned summit overlooking frozen wastes."),
    ("Frostbone Shrine", "Sacred site where frost and bone meet in prayer."),
    ("Prairie's Heart", "Central gathering where traders meet and bonds form."),
    ("Moonshadow Reach", "Twilight realm where moonlight bends the mist."),
    ("Bonewrought Cairn", "Ancestral forge where spirit-steel takes shape."),
    ("SilentMoon Vale", "Peaceful hollow blessed by the silver moon's rest."),
    ("Spiritwalk Crossing", "Gateway where wandering spirits cross the prairie."),
    ("FrostFire Junction", "Convergence of ice and forge meet as extremes."),
    ("Ancestor's Breath", "Windswept place where ancestors whisper truths."),
    ("Ironprairie Citadel", "Stronghold where prairie and forgefire become one."),
    ("Duskwind Settlement", "Haven sheltered from the prairie's harsh winds."),
    ("Starbone Sanctuary", "Night-blessed temple carved from frost and light."),
    ("Faelight Hollow", "A hollow where fae lanterns burn cold and never gutter."),
    ("Thornveil Crossing", "Thorned gate to the fae roads, toll paid in old names."),
    ("Glamourfall", "Waterfall that shows travelers a face not their own."),
    ("Whisperbrook Fen", "Fen where fae bargains ripple outward in the reeds."),
    ("Sundered Ringwood", "A broken fairy ring, its circle still half-open."),
    ("Mothlight Reach", "Reach lit by moth-fae drawn to unspent wishes."),
    ("Starborne Thorn", "Fae settlement where starlight grows strange and thorn-thick."),
    ("Moonthread Market", "Where fae merchants trade in promises instead of coin."),
    ("Bonelight Grove", "A circle of ancient trees where bones glow like lanterns."),
    ("Silentveil Tor", "High place where sound stops and whispers cost a breath."),
    ("Dreambone Ridge", "A ridge where sleep comes swift and dreams cost more than sleep."),
    ("Windprice Flat", "Flat lands where the wind demands a secret for each gust."),
    ("Starforgotten Mere", "A lake where stars are forgotten and reflections lie."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_towns_valid() {
        assert!(TOWNS.len() >= 26);
        for (name, desc) in TOWNS {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
            assert!(name.is_ascii());
            assert!(desc.is_ascii());
        }
    }
}
