//! forge-lore — Dialogue Lore Book authoring data model.
//!
//! Bottom-of-graph typed-contract crate for the Dialogue Lore Book (see
//! `work/dialogue-lore-book-spec-2026-05-28.md`). Holds:
//!
//! - [`Voice`] — a typed speaker with register + per-voice defaults
//! - [`LineEntry`] — one spoken line OR one piece of static lore, with per-char
//!   emphasis and (optional) ink-trace annotations
//! - [`DialogueTree`] — branching NPC exchange (data, not control flow)
//! - [`LoreCodex`] — static narrative artifacts (essays, found-text, codex pages)
//! - [`LoreCue`] — derived audio cue shape; mirrors `HarmonicDialogueCue` 1:1
//!   so a future bridge module can `impl From<LoreCue> for HarmonicDialogueCue`
//!   without forge-lore taking a dep on forge-harmonics
//! - [`lint`] — quality gates (missing voice, empty line, branch coverage,
//!   cultural-boundary; cultural lint delegates to local implementation)
//!
//! ## Doctrine
//!
//! - **Integer-only above DSP boundary.** Permyriad (`u16`, `[0, 10000]`) for
//!   emphasis and pace. MilliUnit (`i64`, `1000 = 1`) for ink-point coordinates.
//! - **No OCR.** Ink traces are sibling annotations, never parsed into text.
//! - **Cultural doctrine §5** is a hard gate; see `lint::check_line`.
//! - **Identity via `forge_core::BrutalHash::of(stable_key)`** for `voice_id`,
//!   `line_id`, `tree_id`. Same hash forge-harmonics already consumes.

#![forbid(unsafe_code)]

// ── Folded from forge-lorekeeper (ARCH-011 Phase 1) ──────────────────────────
/// Lore keeper module for persisting and managing dialogue content.
pub mod keeper;

/// Multi-persona NPC narrator engine (Era/Persona/ZalgoRng/ChimeraEngine).
pub mod chimera;  // multi-persona NPC narrator (Era/Persona/ZalgoRng/ChimeraEngine)
/// Zodiac classes from 2DAK lore-seed.json — the 12 zodiac classes over the 4 elements.
pub mod classes; // 2DAK lore-seed.json — the 12 zodiac classes over the 4 elements
/// Lore codex for static narrative artifacts (essays, found-text, codex pages).
pub mod codex;
/// Derived audio cue shapes that mirror HarmonicDialogueCue 1:1.
pub mod cue;
/// Line entries: spoken lines and static lore with emphasis and ink-trace annotations.
pub mod entry;
/// Seed lore fragments — 3 actors × 4 elements + 3 pattern vignettes from seed_lore.sql.
pub mod fragments; // seed_lore.sql — 3 actors × 4 elements + 3 pattern vignettes
/// Guardian spirits and sound signatures from 13moons guardian_lore.json.
pub mod guardians; // 13moons guardian_lore.json — 8 Cree zone spirits + sound signatures
/// Lineage resolution: star band × spectral class → Sevenfold metal on artifacts.
pub mod lineage; // star band x spectral class -> the Sevenfold metal an artifact carries
/// Quality gates for dialogue and lore content (voice, line, branch coverage, cultural checks).
pub mod lint;
/// Brightness, moon sky, and spectral bands from 13moons.stars_lore_rules.v1 with Walker effect.
pub mod stars; // 13moons.stars_lore_rules.v1 — brightness/spectral bands, 13 moon skies, Walker effect
/// Branching NPC dialogue trees (data structures, not control flow).
pub mod tree;
/// Speaker voices with register and per-voice defaults.
pub mod voice;
/// The 12-zone ladder from 2DAK lore-seed.json (distinct from forge-game-systems' 14-zone world).
pub mod zones; // 2DAK lore-seed.json — the 12-zone ladder (NOT forge-game-systems' 14-zone world)

/// Class types and metadata from the zodiac classification system.
pub use classes::{class_of, signs_of, Class, CLASSES};
/// Static narrative artifact container.
pub use codex::LoreCodex;
/// Audio cue derivation and cue type.
pub use cue::{derive_cue, LoreCue};
/// Line entry types with ink-point annotations.
pub use entry::{InkPoint, InkSegment, LineEntry};
/// Fragment seeding and elemental actor types.
pub use fragments::{fragment, fragment_codex, vignette, Actor, Element};
/// Guardian spirit queries and the full guardian roster.
pub use guardians::{guardian, guardians_of, Guardian, Zone, GUARDIANS};
/// Lineage resolution, gem/metal types, and quality thresholds.
pub use lineage::{cut_gem_under_sky, lineage_of, lineage_under_walker, insistence_q, tempering_metal, Lineage, SkyGem, Metal, MARKING_FLOOR_Q, MARKED_GEM_BONUS_Q};
/// Linting gates for dialogue and tree validation.
pub use lint::{check_line, check_tree, GateError};
/// Star brightness, moon sky, and spectral classification.
pub use stars::{brightness_of, moon_sky, spectral_of, Brightness, MoonSky, Spectral};
/// Dialogue tree structures and node identifiers.
pub use tree::{Choice, DialogueNode, DialogueTree, NodeId};
/// Voice and register types.
pub use voice::{Voice, VoiceRegister};
/// Zone ladder, holdings, era, and faction types.
pub use zones::{holdings, zone, Era, Faction, ZoneEntry, ZONE_LADDER};

/// Compute a stable identity hash from a string key. Wraps
/// `forge_core::BrutalHash::of` so callers get a `u64` directly without
/// needing to import BrutalHash.
///
/// Matches the speaker_id / cue_id format forge-harmonics expects.
pub fn id_of(stable_key: &str) -> u64 {
    use forge_vcs_v3::hash::BrutalHashExt;
    forge_core_v3::BrutalHash::of(stable_key.as_bytes()).as_u64()
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn id_is_stable() {
        let a = id_of("innkeeper_morrigan");
        let b = id_of("innkeeper_morrigan");
        assert_eq!(a, b);
    }

    #[test]
    fn id_differs_for_different_keys() {
        assert_ne!(id_of("a"), id_of("b"));
    }

    #[test]
    fn id_is_nonzero_for_nontrivial_keys() {
        // Sentinel zero is reserved for "unset"; real keys must not collide.
        // (No formal guarantee, but worth a sanity-test.)
        assert_ne!(id_of("innkeeper_morrigan"), 0);
    }
}
