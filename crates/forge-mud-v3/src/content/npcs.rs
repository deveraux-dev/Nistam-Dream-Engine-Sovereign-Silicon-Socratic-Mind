//! NPCs and their greetings for the 13moons prairie-alchemy realm.

/// A collection of NPCs with their initial greetings.
pub const NPCS: &[(&str, &str)] = &[
    ("SpiritFire", "The flames remember your bones."),
    ("Meskanaw", "Frost whispers through the iron trails."),
    ("Wisakedjak", "The trickster's forge burns endless."),
    ("Bone Keeper", "Your ancestors speak in the wind."),
    ("Frost Weaver", "Cold threads bind the prairie whole."),
    ("Iron Heart", "The forge demands its ancient price."),
    ("Shadow Walker", "Darkness flows where light once burned."),
    ("Storm Rider", "Thunder carries the old medicine."),
    ("Root Speaker", "The earth hums with forgotten songs."),
    ("Ash Tender", "Embers hold the stories untold."),
    ("Starlight", "Constellations map the hidden paths."),
    ("Dusk Bearer", "Twilight holds the sacred threshold."),
    ("Boulder Sage", "Stone remembers every step taken."),
    ("Wind Singer", "The prairie breathes with your heartbeat."),
    ("Smoke Guide", "Smoke carries messages to the beyond."),
    ("Silver Tongue", "Words weave power through the grass."),
    ("Moon Keeper", "Moonlight reveals what day conceals."),
    ("Hollow Piper", "A tune for a tune - what will you trade to hear it end?"),
    ("Ringward Fae", "Step outside the circle and the bargain is sealed."),
    ("Glass Antler", "Antlers of glass, a gift that always wants returning."),
    ("Thistlewhisper", "Every kindness here is a debt wearing a smile."),
    ("Nightcap Sprite", "Sip the dew, forget a name, wake up somewhere new."),
    ("Loamtongue", "Speaks in riddles that cost a memory to answer."),
    ("Thornwhisper", "Speaks in thorns that prick but reveal hidden truths."),
    ("Starwarden", "Guards a secret that swallows stars whole."),
    ("Bonecaller", "Summons the restless dead with a song that costs hearing."),
    ("Dawnthief", "Steals the morning to pay for her evening debts."),
    ("Riddlekeeper", "Holds the answer to a question you forgot you asked."),
    ("Mistbender", "Bends the mist to show you what your heart desires most."),
    ("Pricewhisper", "Whispers the true cost of your wishes before you make them."),
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_npcs_valid() {
        assert!(super::NPCS.len() >= 30);
        for (name, greeting) in super::NPCS {
            assert!(!name.is_empty());
            assert!(!greeting.is_empty());
            assert!(name.is_ascii());
            assert!(greeting.is_ascii());
        }
    }
}
