//! ONE-BIN FOLD WAVE 6 (Sean 2026-07-09). forge-audio's `clip_edit` + `music_features`
//! AUTOBINS (`src/bin/`, no `[[bin]]` entry) were relocated to `_folded_bins/`; their
//! bodies live here as `pub fn run() -> i32`, dispatched by `13forge-studio clip-edit`
//! / `… music-features` before GUI/brain init. Exit codes, no `anyhow` edge.
//!
//! SOUND GATE: these are LOAD-TIME tools (decode/ingest/analyse a whole file, host-free) —
//! squarely inside forge-audio's documented zero-alloc CARVE-OUT (only the realtime
//! callback is gated). They never touch the 2 ms hot-path. `audio_smoke` (◐ smoke
//! harness) is intentionally left as a dev autobin per the board triage.

pub mod clip_edit;
pub mod music_features;
