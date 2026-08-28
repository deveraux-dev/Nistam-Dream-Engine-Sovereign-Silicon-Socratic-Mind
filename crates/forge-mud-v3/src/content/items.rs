//! Prairie alchemy items for the 13moons lore.

/// A collection of prairie-alchemy items with their flavors.
pub const ITEMS: &[(&str, &str)] = &[
    ("Bone Blade", "sword carved from ancient prairie bones"),
    ("Frost Whisper", "reagent of ice and whispered secrets"),
    ("Moon Charm", "charm blessed by the pale night guardian"),
    ("Forge Hammer", "tool of smiths who work with starlight"),
    ("Prairie Sage", "herb that grows only under twin moons"),
    ("Bone Dust", "reagent of ancestral earth and memory"),
    ("Frost Shard", "charm that carries winter's heart"),
    ("Moon Stone", "tool for grinding reagents under starlight"),
    ("Prairie Blade", "sword forged in moonlit prairie fires"),
    ("Whisper Charm", "charm of ancient prairie voices"),
    ("Bone Cutter", "tool for harvesting bone reagents"),
    ("Frost Flame", "reagent of impossible burning ice"),
    ("Moon Dust", "reagent that glows in darkest night"),
    ("Prairie Forge", "tool blessed by moon and bone"),
    ("Sage Blade", "sword tempered in prairie morning dew"),
    ("Frost Moon", "charm linking frost and night magic"),
    ("Fae Lantern", "reagent that burns cold, traded for a name given back"),
    ("Glamour Mask", "charm that shows a false face, worn at the cost of your own"),
    ("Thistle Crown", "charm woven from thorn and promise, itches with old debts"),
    ("Ringward Coin", "tool minted in the fairy ring, spent only in fae bargains"),
    ("Mothlight Vial", "reagent of captured wishes, dims when a debt comes due"),
    ("Hollow Bell", "tool that rings once for every vow left unpaid"),
    ("Starborne Crystal", "charm that shows the night sky, captured from a fae bargain for starlight itself"),
    ("Thornlight Blade", "sword that cuts only what bargains bind, paying in old scars"),
    ("Bonecall Flute", "tool that summons with sound, traded for the player's lullaby"),
    ("Mistweave Cloth", "reagent woven from morning mist, costs the weaver one clear memory"),
    ("Windprice Talisman", "charm that asks the wind's permission, worn only when you have secrets to trade"),
    ("Starforgotten Fragment", "reagent of something lost, sparks with forgotten moments"),
    ("Pricewhisper Vial", "reagent containing the words of costs not yet paid, glows when debts come due"),
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_items_valid() {
        assert!(super::ITEMS.len() >= 29);
        for (name, flavor) in super::ITEMS {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!flavor.is_empty() && flavor.is_ascii());
        }
    }
}
