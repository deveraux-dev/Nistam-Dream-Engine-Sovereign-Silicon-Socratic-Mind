//! The Magic Word lexicon — the canonical sung words, put to Schools of Magic
//! (Sean 2026-08-18: "MAGICWORDS need Defined and Put to Schools of Magic").
//!
//! Before this module any string was a magic word (`cdk::word_world_line`
//! sings arbitrary bytes — bible 003's Rosetta chain: `forge_harmonics::
//! word_note` note + `forge_sieve_v3::prime_seed` world, deterministic,
//! always in-scale). That stays true — the chain refuses nothing. What this
//! module adds is the CANON: which words the game itself teaches, and which
//! school each belongs to.
//!
//! Schools are NOT a new taxonomy (L05): a school IS one of the seven
//! hermetic principles, riding the SEVENFOLD spine (`hermetics.rs:120` —
//! stat, planet, metal, colour, principle, all already bound). Combat
//! already speaks one glyph-word per register (`casting::GLYPH_WORDS`,
//! casting.rs:11); those seven are each school's WAR word. This lexicon adds
//! five SUNG words per school — five, the aperture ceiling (4±1 law), never
//! more per group.
//!
//! The canonical test trio of bible 003 (`tests/magic_word_chain.rs`:
//! "thorn", "bell", "ash") is placed, not orphaned: thorn → Edge (Polarity),
//! bell → Bell (Vibration), ash → River (Gender).

use crate::hermetics::{Principle, Stat};

/// The seven Schools of Magic — one per hermetic principle, SEVENFOLD order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum School {
    /// School of the Mirror — Mentalism (thought floors the roll).
    Mirror,
    /// School of the Map — Correspondence (as above, so below).
    Map,
    /// School of the Bell — Vibration (frequency pierces armor).
    Bell,
    /// School of the Edge — Polarity (alignment difference is power).
    Edge,
    /// School of the Tide — Rhythm (the global turn tide).
    Tide,
    /// School of the Ledger — Cause & Effect (the toll ledger).
    Ledger,
    /// School of the River — Gender (active and passive fuse).
    River,
}

/// Number of schools — exactly the seven principles, no eighth.
pub const SCHOOL_COUNT: usize = 7;

/// Sung words per school — the aperture ceiling (4±1 law), held exactly.
pub const WORDS_PER_SCHOOL: usize = 5;

impl School {
    /// Every school, spine order.
    pub const ALL: [School; SCHOOL_COUNT] = [
        School::Mirror,
        School::Map,
        School::Bell,
        School::Edge,
        School::Tide,
        School::Ledger,
        School::River,
    ];

    /// The hermetic principle this school IS.
    pub fn principle(self) -> Principle {
        match self {
            School::Mirror => Principle::Mentalism,
            School::Map => Principle::Correspondence,
            School::Bell => Principle::Vibration,
            School::Edge => Principle::Polarity,
            School::Tide => Principle::Rhythm,
            School::Ledger => Principle::CauseEffect,
            School::River => Principle::Gender,
        }
    }

    /// The register the school's magic rides (SEVENFOLD row for its principle).
    pub fn stat(self) -> Stat {
        match self {
            School::Mirror => Stat::LogicDepth,
            School::Map => Stat::ShadowWeight,
            School::Bell => Stat::Resonance,
            School::Edge => Stat::Vigor,
            School::Tide => Stat::Momentum,
            School::Ledger => Stat::Guilt,
            School::River => Stat::Tarnish,
        }
    }

    /// The school's spoken name.
    pub fn as_str(self) -> &'static str {
        match self {
            School::Mirror => "School of the Mirror",
            School::Map => "School of the Map",
            School::Bell => "School of the Bell",
            School::Edge => "School of the Edge",
            School::Tide => "School of the Tide",
            School::Ledger => "School of the Ledger",
            School::River => "School of the River",
        }
    }
}

/// The canon: thirty-five sung words, five per school, every word unique.
/// Each is a real input to the Rosetta chain (`cdk::word_world_line`) — a
/// note, a seed, an island — and now also a school, a principle, a register.
pub const MAGIC_WORDS: [(&str, School); SCHOOL_COUNT * WORDS_PER_SCHOOL] = [
    // Mirror — Mentalism: thought, reflection, the quicksilver mind.
    ("mind", School::Mirror),
    ("mirror", School::Mirror),
    ("glass", School::Mirror),
    ("dream", School::Mirror),
    ("quicksilver", School::Mirror),
    // Map — Correspondence: as above, so below.
    ("above", School::Map),
    ("below", School::Map),
    ("shadow", School::Map),
    ("map", School::Map),
    ("lead", School::Map),
    // Bell — Vibration: the note that pierces.
    ("bell", School::Bell),
    ("hum", School::Bell),
    ("song", School::Bell),
    ("chord", School::Bell),
    ("gold", School::Bell),
    // Edge — Polarity: difference as power.
    ("thorn", School::Edge),
    ("iron", School::Edge),
    ("ember", School::Edge),
    ("frost", School::Edge),
    ("edge", School::Edge),
    // Tide — Rhythm: the turn tide.
    ("tide", School::Tide),
    ("moon", School::Tide),
    ("silver", School::Tide),
    ("pulse", School::Tide),
    ("dawn", School::Tide),
    // Ledger — Cause & Effect: everything costs.
    ("debt", School::Ledger),
    ("ledger", School::Ledger),
    ("root", School::Ledger),
    ("toll", School::Ledger),
    ("seed", School::Ledger),
    // River — Gender: active and passive fuse (Chaos Feminine's water).
    ("ash", School::River),
    ("river", School::River),
    ("forge", School::River),
    ("copper", School::River),
    ("bloom", School::River),
];

/// The school a canonical word belongs to; `None` for words outside the canon
/// (they still sing — the chain refuses nothing — but no school teaches them).
pub fn school_of(word: &str) -> Option<School> {
    MAGIC_WORDS.iter().find(|&&(w, _)| w == word).map(|&(_, s)| s)
}

/// The five sung words a school teaches, canon order.
pub fn words_of(school: School) -> [&'static str; WORDS_PER_SCHOOL] {
    let mut out = [""; WORDS_PER_SCHOOL];
    let mut i = 0;
    for &(w, s) in MAGIC_WORDS.iter() {
        if s == school {
            out[i] = w;
            i += 1;
        }
    }
    out
}

// ── Subclasses: schools matched to Elements and Alchemy (Sean 2026-08-18) ───

use crate::combat_brain::dissonance::{AlchemicalTier, ClassicalElement};
use crate::content::alchemy::Proof;
use crate::hermetics::Reagent;

/// One school's unique alchemical identity: element + tier + reagent +
/// signature brew, all drawn from LIVE tables (ClassicalElement/AlchemicalTier
/// from combat_brain::dissonance, Reagent from hermetics, brews from
/// content::alchemy::BREWS). Rides alchemy's own honesty vocabulary: `proof`
/// is [`Proof::Named`] when the signature brew's dominant reagent IS the
/// subclass reagent, [`Proof::Nearest`] when no brew of that reagent exists
/// and the closest fit was chosen by hand — a real choice, marked.
#[derive(Debug, Clone, Copy)]
pub struct Subclass {
    /// The school this subclass belongs to.
    pub school: School,
    /// The subclass's spoken name.
    pub name: &'static str,
    /// Its classical element.
    pub element: ClassicalElement,
    /// Its alchemical tier (Nigredo low-Hz … Rubedo high-Hz, the same axis
    /// terrain_waveform already rides).
    pub tier: AlchemicalTier,
    /// Its reagent — unique per subclass, frequency-armed (Vibration law).
    pub reagent: Reagent,
    /// Its signature brew — an exact name from [`alchemy::BREWS`].
    pub brew: &'static str,
    /// Whether the brew names the reagent directly, or is the nearest fit.
    pub proof: Proof,
}

/// The seven subclasses, spine order. Uniqueness law (test-held): names,
/// (element, tier) pairs, reagents, and brews are all distinct — seven
/// schools over four elements MUST share elements, so identity lives in the
/// (element, tier) pair, never element alone.
pub const SUBCLASSES: [Subclass; SCHOOL_COUNT] = [
    Subclass {
        school: School::Mirror,
        name: "Quicksilver Seer",
        element: ClassicalElement::Air,
        tier: AlchemicalTier::Citrinitas,
        reagent: Reagent::Quicksilver,
        brew: "Mercury's Mirror",
        proof: Proof::Named,
    },
    Subclass {
        school: School::Map,
        name: "Lead Cartographer",
        element: ClassicalElement::Earth,
        tier: AlchemicalTier::Nigredo,
        reagent: Reagent::Lead,
        // No lead brew exists in BREWS; ancestors-below fits as-above-so-below.
        brew: "Ash of Ancestors",
        proof: Proof::Nearest,
    },
    Subclass {
        school: School::Bell,
        name: "Brass Cantor",
        element: ClassicalElement::Fire,
        tier: AlchemicalTier::Rubedo,
        reagent: Reagent::Brass,
        // No brass brew exists; the fire kiss is the nearest voice for a bell.
        brew: "Sulfur's Kiss",
        proof: Proof::Nearest,
    },
    Subclass {
        school: School::Edge,
        name: "Sulfur Duelist",
        element: ClassicalElement::Fire,
        tier: AlchemicalTier::Nigredo,
        reagent: Reagent::Sulfur,
        brew: "Sulfur Drench",
        proof: Proof::Named,
    },
    Subclass {
        school: School::Tide,
        name: "Brine Moonspeaker",
        element: ClassicalElement::Water,
        tier: AlchemicalTier::Albedo,
        reagent: Reagent::Brine,
        brew: "Moon-water Tonic",
        proof: Proof::Named,
    },
    Subclass {
        school: School::Ledger,
        name: "Marrow Debtkeeper",
        element: ClassicalElement::Earth,
        tier: AlchemicalTier::Citrinitas,
        reagent: Reagent::Marrow,
        brew: "Calcined Bone",
        proof: Proof::Named,
    },
    Subclass {
        school: School::River,
        name: "Ichor Riverwright",
        element: ClassicalElement::Water,
        tier: AlchemicalTier::Rubedo,
        reagent: Reagent::Ichor,
        // No ichor brew exists; the tide's salt is the river's nearest water.
        brew: "Tidal Salt",
        proof: Proof::Nearest,
    },
];

/// The subclass a school confers.
pub fn subclass_of(school: School) -> Subclass {
    // Spine order is enum order; the test below holds this true.
    SUBCLASSES[school as usize]
}

const _: () = assert!(School::ALL.len() == SCHOOL_COUNT);
const _: () = assert!(MAGIC_WORDS.len() == SCHOOL_COUNT * WORDS_PER_SCHOOL);
const _: () = assert!(SUBCLASSES.len() == SCHOOL_COUNT);

#[cfg(test)]
mod tests {
    use super::*;

    /// Every word is unique, every school holds exactly the aperture five.
    #[test]
    fn magic_words_canon_is_unique_and_aperture_bounded() {
        for (i, &(a, _)) in MAGIC_WORDS.iter().enumerate() {
            for &(b, _) in &MAGIC_WORDS[i + 1..] {
                assert_ne!(a, b, "two schools teach the same word");
            }
        }
        for school in School::ALL {
            let n = MAGIC_WORDS.iter().filter(|&&(_, s)| s == school).count();
            assert_eq!(n, WORDS_PER_SCHOOL, "{} breaks the aperture law", school.as_str());
        }
    }

    /// The bible-003 canonical trio is placed, and lookups agree with the canon.
    #[test]
    fn magic_words_canonical_trio_is_schooled() {
        assert_eq!(school_of("thorn"), Some(School::Edge));
        assert_eq!(school_of("bell"), Some(School::Bell));
        assert_eq!(school_of("ash"), Some(School::River));
        assert_eq!(school_of("not-a-canon-word"), None);
        assert_eq!(words_of(School::Bell), ["bell", "hum", "song", "chord", "gold"]);
    }

    /// Each school rides its own SEVENFOLD row: principle and stat agree with
    /// the spine (hermetics.rs:120), no row shared, no row skipped.
    #[test]
    fn magic_words_schools_ride_the_sevenfold_spine() {
        use crate::hermetics::SEVENFOLD;
        for school in School::ALL {
            let row = SEVENFOLD
                .iter()
                .find(|r| r.principle == school.principle())
                .expect("every school's principle has a spine row");
            assert_eq!(row.stat, school.stat(), "{} rides the wrong register", school.as_str());
        }
    }

    /// Subclass uniqueness law: names, (element, tier) pairs, reagents, and
    /// brews all distinct; every brew is a real BREWS row; the proof mark is
    /// honest (Named ⇔ the brew's dominant reagent IS the subclass reagent,
    /// checked against alchemy's own table); spine order matches the enum so
    /// `subclass_of` never misindexes.
    #[test]
    fn magic_words_subclasses_are_unique_and_honestly_proven() {
        for (i, a) in SUBCLASSES.iter().enumerate() {
            assert_eq!(a.school, School::ALL[i], "SUBCLASSES out of spine order");
            assert_eq!(subclass_of(a.school).name, a.name);
            for b in &SUBCLASSES[i + 1..] {
                assert_ne!(a.name, b.name, "two subclasses share a name");
                assert!(
                    !(a.element == b.element && a.tier == b.tier),
                    "{} and {} share (element, tier)",
                    a.name,
                    b.name
                );
                assert_ne!(a.reagent, b.reagent, "{} and {} share a reagent", a.name, b.name);
                assert_ne!(a.brew, b.brew, "{} and {} share a brew", a.name, b.name);
            }
            let (brew_reagent, _) = crate::content::alchemy::reagent_of(a.brew)
                .unwrap_or_else(|| panic!("{}'s brew '{}' is not in BREWS", a.name, a.brew));
            match a.proof {
                Proof::Named => assert_eq!(
                    brew_reagent, a.reagent,
                    "{} claims Named but its brew's reagent differs",
                    a.name
                ),
                Proof::Nearest => assert_ne!(
                    brew_reagent, a.reagent,
                    "{} claims Nearest but its brew actually names its reagent",
                    a.name
                ),
            }
        }
    }

    /// Every canonical word actually sings: the Rosetta chain accepts it and
    /// the note lands in the pentatonic scale (bible 003's invariant, now
    /// asserted over the whole canon, not just the test trio).
    #[test]
    fn magic_words_every_canon_word_sings_in_scale() {
        for &(word, _) in MAGIC_WORDS.iter() {
            let note = forge_harmonics::word_note(word.as_bytes());
            assert!(
                forge_harmonics::PENTATONIC_C.contains(&note),
                "canon word '{word}' sings off-scale"
            );
            let line = crate::cdk::word_world_line(word);
            assert!(!line.is_empty(), "canon word '{word}' failed to birth a world line");
        }
    }
}
