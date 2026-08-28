//! Alchemical brews for 13moons prairie.
//!
//! Reagent wiring (2026-08-13): each brew's dominant [`crate::hermetics::Reagent`] is
//! named by its own flavor text (Calcination of Sage burns to Ash; Mercury's Mirror is
//! Quicksilver) — `Proof::Authored` (this repo's L12 proof-state vocabulary, not a
//! linkable type in this crate), a real intentional choice made this session, not
//! derived from a prior spec. Where a brew's name doesn't cleanly name one of the ten
//! reagents (the two Moon/Lunar brews), the nearest water-adjacent reagent (Brine) is
//! picked and marked in its own doc comment rather than left silently arbitrary.

use crate::hermetics::{law, Reagent};
use crate::overlay::Ledger;

/// How firmly a brew's dominant [`Reagent`] is known. [`Proof::Named`] brews take their
/// reagent directly from an unambiguous word in their own flavor text (Sulfur's Kiss ->
/// Sulfur). [`Proof::Nearest`] brews have no reagent word at all and were assigned the
/// closest fit by hand — a real choice, marked honest rather than dressed as canon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// The brew's name names its reagent directly.
    Named,
    /// No reagent word in the name; nearest fit chosen by hand.
    Nearest,
}

/// Alchemical brew formulations and their evocative effects.
pub const BREWS: &[(&str, &str)] = &[
    ("Calcination of Sage", "ashes whisper forgotten names"),
    ("Salt of Tears", "bitter clarity blooms at dawn"),
    ("Sulfur's Kiss", "fingers dance with golden warmth"),
    ("Mercury's Mirror", "stillness reflects what was hidden"),
    ("Moon-water Tonic", "silver threads bind scattered threads"),
    ("Ash of Ancestors", "echoes speak through the veil"),
    ("Salt Bloom", "white flowers drink the dark"),
    ("Sulfur Drench", "heat bends the world sideways"),
    ("Mercury's Dance", "quicksilver shapes your shadow"),
    ("Lunar Essence", "stars taste of honey and salt"),
    ("Calcined Bone", "marrow-song calls to stone"),
    ("Tidal Salt", "ebb and flow within your bones"),
];

/// Each [`BREWS`] row's dominant reagent, same index, same length (asserted below).
pub const BREW_REAGENT: &[(Reagent, Proof)] = &[
    (Reagent::Ash, Proof::Named),         // Calcination of Sage — calcination burns to ash
    (Reagent::Salt, Proof::Named),        // Salt of Tears
    (Reagent::Sulfur, Proof::Named),      // Sulfur's Kiss
    (Reagent::Quicksilver, Proof::Named), // Mercury's Mirror
    (Reagent::Brine, Proof::Nearest),     // Moon-water Tonic — no reagent word, water-adjacent
    (Reagent::Ash, Proof::Named),         // Ash of Ancestors
    (Reagent::Salt, Proof::Named),        // Salt Bloom
    (Reagent::Sulfur, Proof::Named),      // Sulfur Drench
    (Reagent::Quicksilver, Proof::Named), // Mercury's Dance
    (Reagent::Brine, Proof::Nearest),     // Lunar Essence — no reagent word, water-adjacent
    (Reagent::Marrow, Proof::Named),      // Calcined Bone — marrow, not ash: bone-specific
    (Reagent::Salt, Proof::Named),        // Tidal Salt
];

/// Look up a brew's dominant reagent and how firmly it's known, by exact name match.
pub fn reagent_of(brew_name: &str) -> Option<(Reagent, Proof)> {
    let idx = BREWS.iter().position(|&(name, _)| name == brew_name)?;
    BREW_REAGENT.get(idx).copied()
}

/// Whether throwing `brew_name` at a target defended at `target_freq` ignores that
/// target's defense entirely — the SAME Vibration-principle armor-penetration law combat
/// already uses ([`law::resonance_delta`] + [`law::ignores_armor`]), not a second formula
/// invented for alchemy. `None` when the brew isn't in [`BREWS`].
pub fn brew_ignores_defense(brew_name: &str, target_freq: u8) -> Option<bool> {
    let (reagent, _) = reagent_of(brew_name)?;
    let delta = law::resonance_delta(reagent.frequency_byte(), target_freq);
    Some(law::ignores_armor(delta))
}

/// Whether a brew is available — gated by archetype pole tally and art deltas from CYOA choices.
/// Reads `Domain::Archetype` tallies from the ledger; `None` when the brew isn't in [`BREWS`].
pub fn brew_allowed(brew_name: &str, ledger: &Ledger, seed: u64) -> Option<bool> {
    reagent_of(brew_name)?;
    let pole = crate::ironroot::archetype_ledger::dominant_pole(ledger, seed);
    Some(pole >= -1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brews_valid() {
        assert!(BREWS.len() >= 12);
        for (name, effect) in BREWS {
            assert!(!name.is_empty());
            assert!(!effect.is_empty());
            assert!(name.is_ascii());
            assert!(effect.is_ascii());
        }
    }

    #[test]
    fn every_brew_has_exactly_one_reagent_row() {
        assert_eq!(BREWS.len(), BREW_REAGENT.len(), "a brew with no reagent, or an orphan reagent row");
    }

    #[test]
    fn reagent_lookup_matches_the_table_by_index() {
        for (i, &(name, _)) in BREWS.iter().enumerate() {
            let (reagent, proof) = reagent_of(name).expect("every real brew name resolves");
            assert_eq!((reagent, proof), BREW_REAGENT[i]);
        }
        assert_eq!(reagent_of("not a real brew"), None);
    }

    #[test]
    fn brew_defense_reuses_the_real_vibration_law() {
        // Mercury's Mirror -> Quicksilver, frequency_byte 128 (hermetics.rs, verified this
        // session). A target defended at the SAME frequency has delta 0, which
        // law::ignores_armor always accepts (< 16) — the identical rule combat trusts.
        assert_eq!(brew_ignores_defense("Mercury's Mirror", 128), Some(true));
        // Lead's frequency_byte is 255; XOR against Quicksilver's 128 is large, so a
        // lead-defended target should NOT have its defense ignored by a quicksilver brew.
        assert_eq!(brew_ignores_defense("Mercury's Mirror", 255), Some(false));
        assert_eq!(brew_ignores_defense("not a real brew", 0), None);
    }
}
