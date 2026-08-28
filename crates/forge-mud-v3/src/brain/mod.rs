//! Brain module — pure integer deterministic logic for terrain, building, AI, and gameplay systems.
//!
//! **Architecture:**
//! Brain modules are pure state machines that never import game.rs or world.rs.
//! They speak through trait callbacks (Sink pattern, following combat_brain's firewall).
//! All logic is integer-only, deterministic (same seed → same output byte-for-byte),
//! and testable in isolation without the game engine.

pub mod terrain;
pub mod terrain_sieve;
pub mod terrain_waveform;
pub mod hermite;
pub mod building;
pub mod ai;
pub mod movement;
pub mod skill_book;
pub mod loot;
pub mod authority_enforcement;
pub mod tempo_run;
pub mod run_dev_run;
