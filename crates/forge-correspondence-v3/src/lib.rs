//! The Correspondence Engine — Color-to-Physics-to-Stats bridge, ported
//! verbatim from `F:\NewRepo\crates\forge-core\src\{correspondence,
//! material_binding,creature_engine}.rs`. Scans pixel art (feeding from
//! forge-vision) and derives deterministic physics + game stats from color
//! distribution — one shared scan/physics stage feeding two intentionally
//! distinct, non-merging stat outputs (decided 2026-08-13, no callers yet
//! to force a premature pick):
//!
//! - `correspondence::StatProfile` (vigor/logic_depth/momentum/
//!   shadow_weight/spirit/resilience) — direct material-palette-ratio math,
//!   no physics round-trip. Reach for this for **appearance/art-facing**
//!   consumers: palette-driven flavor stats, cosmetic tinting, anything that
//!   wants "how much iron/void/ash is in this sprite" without caring about
//!   a simulated body.
//! - `creature_engine::{CoreStats, GameEntity, AiType}` (STR/STA/AGI/DEX/
//!   WIS/INT/CHA, HP/mana/level/AC, AI archetype) — physics-derived from a
//!   `PhysicalProfile` (mass/height/volume/limb geometry). Reach for this
//!   for **gameplay-facing** consumers: entity spawning, combat, AI
//!   behavior selection — anything that needs a creature to behave like it
//!   has a body and a mass.
//!
//! Both bridges are still v1's shipped numbers verbatim (exponents,
//! reference constants, AiType thresholds) — untuned, unbalanced against
//! real gameplay. Fine to wire callers against the shapes now; treat the
//! actual coefficients as provisional until there's a real game loop to
//! tune them against.
//!
//! A sibling of `forge-core-v3`, not folded into it: this engine needs
//! `serde`, and `forge-core-v3` is Crate Zero (zero deps by law). The
//! `material_registry` module depends on `forge_core_v3::music_sieve::AcousticRegistry`
//! for acoustic profile bridging; the other modules use only types defined locally.

/// Color-to-Physics-to-Stats bridge: palette scan → material map → spatial
/// analysis → `FramePhysics` + `StatProfile`.
pub mod correspondence;
/// Palette-index → `MaterialAtom` (physical properties: metallic, roughness,
/// Mohs hardness, mass, friction, resonance, elasticity) + derived
/// `PhysicalStats`, plus `MaterialBinding`'s force-propagation model.
pub mod material_binding;
/// `PhysicalProfile` derivation from a `MaterialScan` + `FramePhysics` pair —
/// the creature-stat leg of the same art-is-the-entity pipeline.
pub mod creature_engine;
/// The 64-slot material registry (palette_idx → MaterialAtom physics), ported
/// verbatim from forge-core v2. Bridges to acoustic profiles via
/// `PaletteAcousticRegistry` (derives `forge_core_v3::music_sieve::AcousticRegistry`).
pub mod material_registry;
/// The 64-slot SEMANTIC palette (essence_id → EssenceAtom rpg_stats), ported
/// verbatim from forge-core v2. Composes with `material_registry` via
/// `whole_entity_stats` (physical + rpg layers from one material + one essence).
pub mod essence_registry;
/// `SpriteAtom` — pairs `forge_core_v3::vixel_automata::VixelAtom` (position/
/// material/physics) with `forge_core_v3::sprite_blob::SpriteInstance`
/// (atlas/palette) for each `material_registry::MATERIALS` slot.
pub mod sprite_atom;
