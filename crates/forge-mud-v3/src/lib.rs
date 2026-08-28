//! W15 — the Operator's MUD. The singing terminal's own game: an operator
//! (harness name + 13-moon birthday) walks an 8x8 squares worldmap generated
//! from a node seed, XP rides the terminal's own bytes, death reseeds the
//! node — new map, new theme, new vibe, new grind. Position is ONE 8-byte
//! word: `forge_core_v3::ramus_prime::MortonKey5D` (5 axes x 12 bits).
//!
//! Lore authority: forge-book (v2 codex). The content tables under
//! `content/` are haiku-authored PLACEHOLDERS pending a forge-book drain —
//! flavour, never mechanics; nothing in them decides an outcome.

pub mod abyss;
pub mod actions;
pub mod bdo_controller;
pub mod brain;
pub mod casting;
pub mod cdk;
pub mod combat;
pub mod combat_brain;
pub mod combat_live;
pub mod consequence;
pub mod console;
pub mod content;
pub mod dash_direction;
pub mod death_scar;
pub mod discipline_combat;
pub mod dm;
pub mod dream;
pub mod ecology;
pub mod explore;
pub mod faction_mind;
pub mod ledger_drift;
/// The SUPERMAX-ATOM field sim (sand/water/fire/alchemy/charge/heat), ported
/// from v2 sf-wasm 2026-08-17 — the shell FieldOrgan's engine. Distinct from
/// `reactions` (crafting ethics): `field::reactions` transmutes field cells.
pub mod field;
pub mod game;
pub mod genesis;
pub mod hermetics;
pub mod itemforge;
pub mod live;
pub mod live_encounter;
pub mod magic;
pub mod magic_words;
pub mod memory;
pub mod mind;
/// The reactions corpus — v2's `forge-reactions` world content as RON. Substrates and the
/// crafting ethics, whose five paths resolve as two mirror pairs plus one fixed point
/// (`PARARITY.md` §3, n = 2m + k).
pub mod reactions;
pub mod haunt;
pub mod ironroot;
pub mod operator;
pub mod overlay;
pub mod physics_telemetry;
pub mod physics_tune;
pub mod rite;
pub mod sense;
pub mod umwelt_loom;
pub mod rng;
pub mod shadow_counterpart;
pub mod skills;
pub mod socketing;
pub mod spell_gems;
pub mod topology;
pub mod weapon_wireframes;
pub mod vendor;
pub mod voices;
pub mod weather;
pub mod witness_mirror;
pub mod world;
pub mod world5d;
pub mod zone;
pub mod organs;

pub use game::Game;
pub use operator::Operator;
pub use haunt::ShadowMemory;
pub use organs::MudChat;
