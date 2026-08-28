//! Prairie-alchemy MUD talent currents: masculine force/forge/strike and feminine attunement/moon/water.

/// Masculine current talents: force, forge, and strike.
pub const MASCULINE: &[(&str, &str)] = &[
    ("Cleave", "Split shield and bone with one swing."),
    ("Forge", "Mend steel with bare hands."),
    ("Iron Will", "Resist charm and compulsion."),
    ("Rend", "Tear through armor like cloth."),
    ("Bastion", "Absorb harm meant for allies."),
    ("Forge Bond", "Strengthen nearby steel."),
    ("Strike True", "Never miss vital target."),
    ("Endure", "Shrug off crushing blow."),
];

/// Feminine current talents: attunement, moon, and water.
pub const FEMININE: &[(&str, &str)] = &[
    ("Moon's Blessing", "Heal under starlight."),
    ("Flow", "Move like water through obstacles."),
    ("Tide Rise", "Lift allies on lunar surge."),
    ("Attune", "Hear whispers of the world."),
    ("Reflect", "Turn foe's force back on them."),
    ("Harvest", "Draw power from the living land."),
    ("Soothe", "Calm rage and quiet screams."),
    ("Drift", "Pass unseen by hostile eyes."),
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_talent_currents() {
        assert!(super::MASCULINE.len() >= 8, "masculine has {} entries", super::MASCULINE.len());
        assert!(super::FEMININE.len() >= 8, "feminine has {} entries", super::FEMININE.len());

        for (name, effect) in super::MASCULINE.iter().chain(super::FEMININE.iter()) {
            assert!(!name.is_empty(), "talent name is empty");
            assert!(!effect.is_empty(), "effect is empty");
            assert!(name.is_ascii(), "talent name not ASCII: {}", name);
            assert!(effect.is_ascii(), "effect not ASCII: {}", effect);
        }
    }
}
