//! XP milestone bosses for the 13-moon prairie-alchemy year.

/// The 13 seasonal XP milestone bosses, one per moon of the year.
pub const BOSSES: &[(&str, &str)] = &[
    ("Coyote Shade", "Trickster of the first frost"),
    ("Bone Keeper", "Guardian of the bone paths"),
    ("Frost Singer", "Whisperer in the winter dark"),
    ("Wisakedjak's Echo", "The trickster's shadow cast"),
    ("Forge Tender", "Smith of the burning bone"),
    ("Meskanaw's Kin", "Dweller of the deep earth"),
    ("SpiritFire Seeker", "Chaser of the elder flame"),
    ("Midnight Forge", "Where the ice learns to burn"),
    ("Bone Prophet", "Voice of the ancient frost"),
    ("Wisakedjak Unbound", "The trickster unleashed"),
    ("Meskanaw Rising", "Deep earth's bitter breath"),
    ("SpiritFire Crowned", "The elder flame incarnate"),
    ("The Thirteenth Moon", "All nine powers converge"),
];

/// Achievement names drained from forge-insights
/// `lib.rs:152-239` (`Achievements::default_milestones()`), source order.
/// Wired as flavour text onto the MUD's own milestone-gate reply
/// (`game::Game::process`) — a different milestone SYSTEM (forge-insights
/// tracks Academy actions, this MUD tracks XP gates); the pairing is
/// `[ASSUMED]` flavour-only, borrowed lore text, not a shared mechanic.
pub const MILESTONE_NAMES: &[&str] = &[
    "First Touch",
    "Physics Explorer",
    "Sieve Apprentice",
    "Script Writer",
    "Code Sovereign",
    "Agent Silver",
    "Agent Gold",
    "First Toll",
    "Witness Rail",
    "Rootcalling",
    "Master Mark",
    "Thirteen Bells",
    "Contract Closed",
    "Warden Felled",
    "Came Back Up",
    "Written Down",
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_bosses_milestone() {
        assert_eq!(super::BOSSES.len(), 13);
        for &(name, epitaph) in super::BOSSES {
            assert!(!name.is_empty(), "boss name must not be empty");
            assert!(!epitaph.is_empty(), "epitaph must not be empty");
            assert!(name.is_ascii(), "boss name must be ASCII");
            assert!(epitaph.is_ascii(), "epitaph must be ASCII");
        }
    }

    #[test]
    fn milestone_names_are_exactly_sixteen_and_word_only() {
        assert_eq!(super::MILESTONE_NAMES.len(), 16);
        for name in super::MILESTONE_NAMES {
            assert!(!name.is_empty() && name.is_ascii());
        }
    }
}
