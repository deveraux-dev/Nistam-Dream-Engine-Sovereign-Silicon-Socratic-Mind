//! ONE-BIN FOLD (Sean 2026-07-08 "1 webexport 1 html page maker 1 bin").
//! The `bake` + `mirror` `[[bin]]`s were dropped; their bodies live here as
//! `pub fn run() -> i32`, dispatched by `13forge-studio bake` / `… mirror`.
//! Firewall-lean: exit codes, no `anyhow` edge (serde/serde_json/sha2 only).

pub mod bake;
pub mod mirror;
