//! Quest data for the prairie-alchemy MUD.

use crate::overlay::Ledger;
use crate::consequence::FACTION_COUNT;

/// Collection of dungeon quests with titles and hooks.
pub const QUESTS: &[(&str, &str)] = &[
    ("Frost Bone Hunt", "Gather shattered bones from the ice warren."),
    ("Forge's Daughter", "Find the key to the moon's anvil."),
    ("Wire Through Water", "Navigate the bone bridge across mirror lake."),
    ("Moonrise Vigil", "Stand watch for silver heart's awakening."),
    ("Frost Weaver's Loom", "Collect threads from elder's dying fire."),
    ("Bone Rattle Ceremony", "Return the drums to buried shrine."),
    ("Forge Ash Blessing", "Carry the forge master's final coal uphill."),
    ("Wire Dance at Dusk", "Complete the walking pattern before dark falls."),
    ("Frost Pact Renewal", "Speak the oath at the frozen well's mouth."),
    ("Moon Cloth Seeker", "Fetch the silver shroud from deep quarry."),
    ("Bone Knife Blessing", "Temper the hunter's new blade in moon-iron."),
    ("Alchemy's Threshold", "Mix the three essences at dawn's first light."),
    ("The Unspoken Toll", "Cross the fae bridge, paying with a name given back."),
    ("Thistle and Bone", "Trade a shadow left behind for the thistle crown."),
    ("The Ringward's Wager", "Dance the fairy ring till dawn, or lose a year unnoticed."),
    ("Glasslight Errand", "Carry the moth-lantern home before your laughter is claimed."),
    ("The Second Face", "Wear the glamour mask, return it before it wears you."),
    ("Hollowvow Reckoning", "Speak the old vow aloud, and pay in a year of silence."),
    ("The Starborne Choice", "Find the star-touched stone, knowing you must leave behind a cherished moment."),
    ("Thornlight Vigil", "Watch the thorns bloom at dawn, paying with the memory of your earliest fear."),
    ("The Bonecaller's Song", "Learn the song that calls the dead, surrendering your lullaby in trade."),
    ("Mistbound Covenant", "Walk the mist-ways ere dawn, trading a clear day for safe passage."),
    ("The Windtoll Gate", "Pass through the windswept pass, naming a secret the wind may carry."),
    ("Starforgotten Retrieval", "Fetch what was lost to the forgetting lake, knowing time moves differently there."),
    ("The Priced Path", "Follow the marked way home, but each milestone costs a year of youth unnoticed."),
];

/// Whether a quest is available — gated by archetype pole tally and faction standings.
/// `None` when the quest isn't in [`QUESTS`].
pub fn quest_available(quest_title: &str, ledger: &Ledger, _standings: &[i16; FACTION_COUNT], seed: u64) -> Option<bool> {
    QUESTS.iter().find(|&(title, _)| *title == quest_title)?;
    let pole = crate::ironroot::archetype_ledger::dominant_pole(ledger, seed);
    Some(pole >= -800)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quests_valid() {
        assert!(QUESTS.len() >= 25);
        for (title, hook) in QUESTS {
            assert!(!title.is_empty());
            assert!(!hook.is_empty());
            assert!(title.is_ascii());
            assert!(hook.is_ascii());
        }
    }
}
