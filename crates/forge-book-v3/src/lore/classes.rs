//! The twelve zodiac classes — drained from `2DAK/data/lore-seed.json`
//! (`classes` + `elements`, v1.0.0, last updated 2026-03-29).
//!
//! Each sign is an element, a role name, and the passive that role plays. The
//! element is [`crate::lore::Element`], the SAME four the actor×element
//! fragments key on, so a chart reading and a class share one vocabulary.
//!
//! Deliberately NOT drained from that seed here: its eight `stats` and
//! `stat_pools` are already `forge_items::stability::ItemStats` (active =
//! vigor+momentum+logic_depth, passive = shadow_weight+tarnish+resonance,
//! wild = guilt+clarity), and its nine `tinctures` are already
//! `forge_game_systems::arena_core::tinctures::TinctureType` on the same ids.

use crate::lore::fragments::Element;
use serde::Serialize;

/// One of the twelve. Ordered as the seed authors them — the zodiac's own order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Class {
    /// Lowercase sign name — the save key.
    pub sign: &'static str,
    /// Elemental affinity determining the shared mechanical passive.
    pub element: Element,
    /// The role's title. Every one is a noun phrase with a wound in it —
    /// "Hollow Crown", "Arrow That Never Lands" — never a bare job name.
    pub role: &'static str,
    /// What the role does mechanically. Shared by element: all three fire signs
    /// crit-scale, all three earth signs armour, and so on.
    pub passive: &'static str,
}

/// The passive each element grants. Three signs share each one — the element is
/// the mechanic and the sign is the character.
pub fn element_passive(element: Element) -> &'static str {
    match element {
        Element::Fire => "Crit scaling",
        Element::Earth => "Durability + hyper-armor",
        Element::Air => "Movement + cast speed",
        Element::Water => "Lifesteal + sustain",
    }
}

/// All twelve, in zodiac order.
pub const CLASSES: [Class; 12] = [
    Class { sign: "aries", element: Element::Fire, role: "Ram Charger", passive: "Crit scaling" },
    Class { sign: "taurus", element: Element::Earth, role: "Rooted Colossus", passive: "Durability + hyper-armor" },
    Class { sign: "gemini", element: Element::Air, role: "Severed Twin", passive: "Movement + cast speed" },
    Class { sign: "cancer", element: Element::Water, role: "Shell Guardian", passive: "Lifesteal + sustain" },
    Class { sign: "leo", element: Element::Fire, role: "Hollow Crown", passive: "Crit scaling" },
    Class { sign: "virgo", element: Element::Earth, role: "Perfect Archive", passive: "Durability + hyper-armor" },
    Class { sign: "libra", element: Element::Air, role: "Balanced Ruin", passive: "Movement + cast speed" },
    Class { sign: "scorpio", element: Element::Water, role: "Buried Sting", passive: "Lifesteal + sustain" },
    Class { sign: "sagittarius", element: Element::Fire, role: "Arrow That Never Lands", passive: "Crit scaling" },
    Class { sign: "capricorn", element: Element::Earth, role: "Eternal Warden", passive: "Durability + hyper-armor" },
    Class { sign: "aquarius", element: Element::Air, role: "Flood That Thinks", passive: "Movement + cast speed" },
    Class { sign: "pisces", element: Element::Water, role: "Grief That Swallows", passive: "Lifesteal + sustain" },
];

/// The class for a sign name (case-insensitive). Unknown sign = `None`.
pub fn class_of(sign: &str) -> Option<&'static Class> {
    let lower = sign.to_ascii_lowercase();
    CLASSES.iter().find(|c| c.sign == lower)
}

/// The three signs of an element, in zodiac order.
pub fn signs_of(element: Element) -> impl Iterator<Item = &'static Class> {
    CLASSES.iter().filter(move |c| c.element == element)
}

impl Class {
    /// The chart line this class reads when it is the Ascendant — the mask the
    /// player meets the world wearing. Joins the class to the actor×element
    /// fragments so a class screen and a reading cannot drift apart.
    pub fn mask(&self) -> &'static str {
        crate::lore::fragments::fragment(crate::lore::fragments::Actor::Asc, self.element)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_signs_three_per_element_all_distinct() {
        assert_eq!(CLASSES.len(), 12);
        let signs: std::collections::HashSet<&str> = CLASSES.iter().map(|c| c.sign).collect();
        assert_eq!(signs.len(), 12, "two classes share a sign");
        let roles: std::collections::HashSet<&str> = CLASSES.iter().map(|c| c.role).collect();
        assert_eq!(roles.len(), 12, "two classes share a role name");
        for element in Element::ALL {
            assert_eq!(signs_of(element).count(), 3, "{element:?} is not a triplicity");
        }
    }

    // The mechanic belongs to the ELEMENT, not the sign. If a class's passive
    // ever disagrees with its element's, one of the two silently forked.
    #[test]
    fn every_class_passive_is_its_elements_passive() {
        for c in CLASSES {
            assert_eq!(
                c.passive,
                element_passive(c.element),
                "{} forked from its element's mechanic",
                c.sign
            );
        }
    }

    #[test]
    fn lookup_is_case_insensitive_and_silent_on_unknown() {
        assert_eq!(class_of("LEO").expect("leo").role, "Hollow Crown");
        assert_eq!(class_of("pisces").expect("pisces").element, Element::Water);
        assert!(class_of("ophiuchus").is_none(), "the thirteenth sign is not authored");
    }

    // The class table and the fragment grid must speak ONE element vocabulary —
    // that join is the reason these live in the same module.
    #[test]
    fn every_class_mask_comes_from_the_fragment_grid() {
        for c in CLASSES {
            let mask = c.mask();
            assert!(mask.starts_with("The Mask is that of"), "{}: {mask}", c.sign);
            assert_eq!(mask, crate::lore::fragments::fragment(crate::lore::fragments::Actor::Asc, c.element));
        }
        assert_eq!(class_of("aries").expect("aries").mask(), class_of("leo").expect("leo").mask());
        assert_ne!(class_of("aries").expect("aries").mask(), class_of("cancer").expect("cancer").mask());
    }
}
