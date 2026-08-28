//! Ported systems from the real `ironroot` game (`F:\NewRepo\crates\ironroot`,
//! 46 files) — the user's own game, triaged 2026-08-13 (20-agent fan-out).
//! Each submodule's own doc comment names its v2 source file and what, if
//! anything, was cut or adapted at the port boundary (L15).

pub mod affect;
pub mod archetype_ledger;
pub mod audio_bridge;
pub mod bell_pit;
pub mod boss_sieve;
pub mod brand;
pub mod brand_aspect;
pub mod consequence;
pub mod cyoa;
pub mod dialogue;
pub mod mud_world;
pub mod lexicon;
pub mod discipline_overlay;
pub mod platform;
pub mod run_profile;
pub mod scene_loader;
pub mod session;
pub mod tags;
pub mod trit_grammar;
pub mod weather_state;
