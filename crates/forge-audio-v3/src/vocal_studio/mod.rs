//! Vocal Studio — the professional audio expression pipeline.
//!
//! Five use cases, one seam:
//! 1. **Character voices** (game devs) — pitch-shift + formant preserve via HPSS
//! 2. **Singers** — auto-tune + harmony generation via Camelot scale targeting
//! 3. **Storytellers** — pacing normalization + emphasis boost + beat-grid quantize
//! 4. **Remix/Remaster** — tempo/key match + loudness normalization
//! 5. **Sound FX** — layered professional synthesis (not toy beeps)
//!
//! All DSP is pure Rust (no Faust runtime). Uses:
//! - `alchemy::{hpss, pitch, vocoder, ducking, restoration}` for spectral DSP
//! - `bpm + camelot + key_detect` for harmonic/rhythmic intelligence
//! - `speech_clip` for filler detection + beat quantization
//! - `effects + synth` for processing + synthesis layers

pub mod character;
pub mod deadpan;
pub mod performer;
pub mod remix;
pub mod sfx;
// vocal_synth: EXCLUDED — needs forge_calligraphy::cremantic::Glyph, which
// doesn't exist anywhere in F:\v3 (verified by grep before landing).
// pub mod vocal_synth;
pub mod youtube;

// Re-export the primary entry points
pub use character::{apply_character, CharacterVoice, ETHEREAL_ELF, GOBLIN, GRIZZLED_WARRIOR, NARRATOR_DEEP};
pub use deadpan::{apply_deadpan, DeadpanParams};
pub use performer::{process_performance, PerformanceMode};
pub use remix::{remix_track, stem_split, match_master, RemixTarget};
pub use sfx::{synthesize_sfx, SfxKind};
pub use youtube::{build_scene_map, scene_map_to_json, SceneMap, SceneBeat, SceneMapConfig};