//! Prairie companion data for 13moons lore.

/// Prairie companions with their temperaments.
pub const PETS: &[(&str, &str)] = &[
    ("Coyote", "Wily and resourceful, keen ears catch whispers on wind."),
    ("Magpie", "Clever collector of shiny things and bold secrets."),
    ("Prairie Hare", "Swift as lightning, nervous energy coiled tight."),
    ("Sturgeon", "Ancient and patient, dwelling deep beneath waters."),
    ("Bison Calf", "Young and strong, learning the prairie's rhythms."),
    ("Prairie Dog", "Alert sentinel, watching vast grasslands below."),
    ("Hawk", "Sky-rider with piercing gaze and hunting focus."),
    ("Badger", "Fierce digger, proud and ready for any fight."),
    ("Fox", "Cunning tracker, red as sunset flames."),
    ("Owl", "Nocturnal wisdom keeper, silent and knowing."),
    ("Glamour Fox", "Wears a borrowed face, loyal only after a bargain struck."),
    ("Thistle Hare", "Quick and thorned, nibbles debts instead of clover."),
    ("Moth Sprite", "Tiny fae light, flits close only for an unspent wish."),
    ("Ringward Pup", "Born inside a fairy ring, herds you back to the path."),
    ("Hollow Owl", "Silent flier that remembers every vow you've made."),
    ("Wisp Kit", "A fox-shaped ember, warm only while a promise holds."),
    ("Starborne Stag", "Horns that shift with starlight, loyal only till the debt is named."),
    ("Thorn Sprite", "Small and prickly, warns you when your bargains begin to bind."),
    ("Bonewhisper Cat", "Silent shadow that speaks only in the voices of the dead."),
    ("Mistbender Elk", "Wanders the mist-edges, shows you the paths others cannot see."),
    ("Windprice Crow", "Black wing that demands a secret for each mile traveled."),
    ("Starforgot Fox", "White-furred and strange, leads travelers only to forgotten places."),
    ("Priceworn Badger", "Scarred and wise, digs truths from the earth at terrible cost."),
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_pets_valid() {
        let pets = super::PETS;
        assert!(pets.len() >= 23);
        for (name, temperament) in pets {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!temperament.is_empty() && temperament.is_ascii());
        }
    }
}
