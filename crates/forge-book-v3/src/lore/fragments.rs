//! Actor × element lore fragments and pattern vignettes — drained from
//! `AKWEB/workers/api/scripts/seed_lore.sql` (the `lore_fragments` and
//! `vignettes` seed rows), which had no in-repo home.
//!
//! The grid is TOTAL: 3 actors × 4 elements = 12 fragments, every cell filled.
//! A reading never falls through to a default, because there is no cell to fall
//! through to — that totality is what [`fragment`] returns without an `Option`.

use crate::lore::{codex::LoreCodex, entry::LineEntry};
use serde::{Deserialize, Serialize};

fn id_of(key: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() & 0xFFFFFFFFFFFFFF
}

/// Who is speaking in the chart — the three points a reading turns on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    /// The will — sovereignty, direct action.
    Sun,
    /// The mood — instinct, what moves in secret.
    Moon,
    /// The Ascendant: the mask you meet the void wearing.
    Asc,
}

/// The four classical elements the fragments are keyed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    /// The element of transformation and direct force.
    Fire,
    /// The element of intellect and communication.
    Air,
    /// The element of stability and foundation.
    Earth,
    /// The element of emotion and intuition.
    Water,
}

impl Actor {
    /// The seed's own token (`'SUN'`, `'MOON'`, `'ASC'`) — the AKWEB rows and
    /// this table must key alike or a reading silently changes voice.
    pub fn token(self) -> &'static str {
        match self {
            Self::Sun => "SUN",
            Self::Moon => "MOON",
            Self::Asc => "ASC",
        }
    }
    /// All three actors in canonical order.
    pub const ALL: [Actor; 3] = [Actor::Sun, Actor::Moon, Actor::Asc];

    /// The actor's voice id. Each actor SPEAKS differently — will, mood, mask —
    /// so each is its own voice, not a shared narrator. `voice_id == 0` is the
    /// unset sentinel and fails `lint::check_line`'s MissingVoice gate.
    pub fn voice_id(self) -> u64 {
        id_of(&format!("lore.voice:{}", self.token()))
    }
}

impl Element {
    /// The seed table's own token string, matching the AKWEB row key for lore fragments.
    pub fn token(self) -> &'static str {
        match self {
            Self::Fire => "FIRE",
            Self::Air => "AIR",
            Self::Earth => "EARTH",
            Self::Water => "WATER",
        }
    }
    /// All four elements in canonical order.
    pub const ALL: [Element; 4] = [Element::Fire, Element::Air, Element::Earth, Element::Water];
}

/// The body text for one (actor, element) cell. Total over the 3×4 grid.
pub fn fragment(actor: Actor, element: Element) -> &'static str {
    use Actor::*;
    use Element::*;
    match (actor, element) {
        (Sun, Fire) => "The Solar fire demands expression through direct action. It is the sovereignty of the will, forged in the heat of creative tension.",
        (Sun, Air) => "The Solar mind seeks the high altitudes of objectivity. It is the visionary eye, detached yet encompassing.",
        (Sun, Earth) => "The Solar presence is anchored in the architectural integrity of the material world. It is the master of form and function.",
        (Sun, Water) => "The Solar heart flows into the depths of the collective unconscious. It is the empathic light that warms the cold void.",
        (Moon, Fire) => "The Lunar mood burns with an unquenchable zeal. It is the instinctual protector, fiercely loyal to the inner flame.",
        (Moon, Air) => "The Lunar reflection is filtered through the sage's wisdom. It is the cool breeze of reason in the night of the soul.",
        (Moon, Earth) => "The Lunar comfort is found in the stoic silence of the stone. It is the enduring foundation that requires no external validation.",
        (Moon, Water) => "The Lunar depth is a bottomless well of mystic intuition. It is the tides of feeling that move the world in secret.",
        (Asc, Fire) => "The Mask is that of the Warrior. You meet the void with courage and a thirst for the frontier.",
        (Asc, Air) => "The Mask is that of the Herald. You are the messenger, the bridge between ideas and execution.",
        (Asc, Earth) => "The Mask is that of the Guardian. You are the wall that protects the sacred space within.",
        (Asc, Water) => "The Mask is that of the Oracle. You see through the surface into the true resonance of things.",
    }
}

/// Pattern vignettes — the line a whole-chart shape earns, keyed by pattern id.
/// Unknown pattern = `None`: a shape nobody wrote gets silence, not filler.
pub fn vignette(pattern_id: &str) -> Option<&'static str> {
    Some(match pattern_id {
        "FIRE_AIR_ALIGNMENT" => "The spark meets the wind. A sudden illumination of the path ahead.",
        "EARTH_WATER_COHESION" => "The clay is formed. Depth find purpose in structure.",
        "VOID_STALL" => "The signals are crossing in the dark. Patience is the only currency here.",
        _ => return None,
    })
}

/// Every authored pattern id, in seed order.
pub const VIGNETTE_PATTERNS: [&str; 3] =
    ["FIRE_AIR_ALIGNMENT", "EARTH_WATER_COHESION", "VOID_STALL"];

/// Build the whole grid as a [`LoreCodex`] — 12 slots in `ALL × ALL` order, each
/// keyed `"lore.fragment:<ACTOR>.<ELEMENT>"` so a slot's identity survives a
/// reorder. This is what the lore browser reads.
pub fn fragment_codex() -> LoreCodex {
    let mut codex = LoreCodex::new(id_of("lore.codex.fragments"), "Actor × Element");
    for actor in Actor::ALL {
        for element in Element::ALL {
            let key = format!("lore.fragment:{}.{}", actor.token(), element.token());
            codex.add_slot(LineEntry::new_with_defaults(
                id_of(&key),
                actor.voice_id(),
                fragment(actor, element),
            ));
        }
    }
    codex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_is_total_and_every_cell_is_distinct() {
        let mut seen: Vec<&str> = Vec::new();
        for actor in Actor::ALL {
            for element in Element::ALL {
                let body = fragment(actor, element);
                assert!(!body.trim().is_empty(), "{:?}×{:?} is blank", actor, element);
                assert!(
                    !seen.contains(&body),
                    "{:?}×{:?} repeats another cell's body",
                    actor,
                    element
                );
                seen.push(body);
            }
        }
        assert_eq!(seen.len(), 12, "3 actors × 4 elements");
    }

    // The codex is what the browser reads, so a dropped slot is a blank page.
    #[test]
    fn the_codex_carries_all_twelve_with_emphasis_in_sync() {
        let codex = fragment_codex();
        assert_eq!(codex.slots.len(), 12);
        for slot in &codex.slots {
            assert!(slot.emphasis_in_sync(), "emphasis drifted from text");
            assert!(crate::lore::lint::check_line(slot).iter().all(|e| !e.is_blocking()));
        }
        // Slot ids are keyed by (actor, element), never by position.
        let ids: std::collections::HashSet<u64> = codex.slots.iter().map(|s| s.line_id).collect();
        assert_eq!(ids.len(), 12, "two cells collided on one id");
    }

    #[test]
    fn vignettes_answer_authored_patterns_and_stay_silent_otherwise() {
        for p in VIGNETTE_PATTERNS {
            assert!(vignette(p).is_some_and(|v| !v.is_empty()), "{p} has no prose");
        }
        assert!(vignette("NO_SUCH_PATTERN").is_none(), "an unwritten shape gets silence");
    }
}
