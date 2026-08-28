//! Relic materials, drained from forge-insights `rpg/relic.rs:9-27`
//! (`Material` enum, `.name()`). Provenance is Weld I's (`itemforge.rs`)
//! concern — L05 one-home, not repeated here. Wired as a material tag
//! rolled alongside every item grant in `game.rs` (`fight()`'s abyss-loot
//! line and `kit()`'s birth-kit line).

/// The three material families: BellBronze..BellDiamond order.
pub const MATERIALS: &[&str] = &["Bell Bronze", "Grave Iron", "Bell Diamond"];

#[cfg(test)]
mod tests {
    #[test]
    fn materials_are_exactly_three_and_word_only() {
        assert_eq!(super::MATERIALS.len(), 3);
        for name in super::MATERIALS {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!name.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
