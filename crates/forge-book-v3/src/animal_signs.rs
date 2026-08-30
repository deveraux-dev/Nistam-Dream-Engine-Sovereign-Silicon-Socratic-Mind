//! Animal-Sign Archetypes — nehiyaw-reviewed corrections only (13moons cultural
//! redline, 2026-03). The pre-redline names (Coyote, "Snow Demon", "Berserker")
//! are cultural faux pas and are not represented here.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// (cree_name, sign_name, role, note) — corrected archetypes only.
pub const ANIMAL_SIGNS: &[(&str, &str, &str, &str)] = &[
    ("kihkwahahkew", "The Unyielding Spirit", "Relentless Tenacity",
     "Wolverine: fierce solitary protector, fights to hold territory, not out of rage."),
    ("mahkesis", "The Fox", "Trickster / Ember sign",
     "Fox, not Coyote (Southwest US connotation) — Treaty 6's actual trickster figure."),
    ("paskwawi-mostos (Provider)", "The Great Provider", "Support / buff-through-sacrifice",
     "Buffalo gave his whole self so the people could live — reciprocity, not raw power."),
    ("paskwawi-mostos (Shield)", "The Ancient Shield", "Tank",
     "Buffalo's second spec. The Stampede is a co-op move, never a solo power."),
];

/// One lore line per corrected archetype.
pub fn animal_signs_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Animal Signs".into()));
    for (cree, sign, role, note) in ANIMAL_SIGNS {
        ch.add_lore(format!("{} ({}) — {}. {}", sign, cree, role, note));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animal_signs_chapter_has_four_corrected_archetypes() {
        let ch = animal_signs_chapter("Animal Signs");
        assert_eq!(ch.lore_count(), 4);
    }

    #[test]
    fn no_faux_pas_names_survive_as_the_archetype_itself() {
        // sign/role are the presented archetype identity — must be the corrected
        // names. `note` legitimately cites the corrected-from term for context.
        for (_, sign, role, _) in ANIMAL_SIGNS {
            let text = format!("{sign} {role}").to_lowercase();
            assert!(!text.contains("coyote"), "Coyote should not appear as the sign itself: {text}");
            assert!(!text.contains("demon"), "Demon should not appear as the sign itself: {text}");
            assert!(!text.contains("berserker"), "Berserker should not appear as the sign itself: {text}");
        }
    }
}
