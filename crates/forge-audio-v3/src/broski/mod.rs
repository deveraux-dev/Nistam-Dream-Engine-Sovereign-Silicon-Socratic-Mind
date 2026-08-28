//! Broski DJ Brain — intelligent mixing assistant for harmonic transitions, banger learning,
//! and real-time DSP chain management.
//!
//! # Overview
//! Broski ("Bro's DJ") is the semantic brain of forge-audio's DJ mode, responsible for:
//! - **Harmonic mixing**: Camelot wheel compatibility, key matching, smooth transitions
//! - **Banger tracking**: Learning which tracks resonate with the mix (in-memory ephemeral stats)
//! - **DSP chain**: Native-Rust signal processing (blend gains, filters, compression, saturation)
//! - **Control semantics**: Enums and messages for deck control, FX, and notifications
//!
//! # Modules
//! - `theory`: Camelot wheel parsing, key compatibility, transition type detection
//! - `native_dsp`: Signal processing primitives (no Faust; all native Rust)
//! - `bangers`: Track scoring and "banger" ranking (ephemeral in-memory tracker)
//! - `types`: DJ control enums and state structures
//!
//! # Integration Point
//! Broski is wired into the forge-audio mixer via panels.rs (`BroskiPanel`), which is the
//! Face socket (L20, witness law). The conductor orchestrates all Broski output.
//!
//! # Design Notes
//! - DSP is f64-native (not quantized to f32 until final mix or effects sink)
//! - Banger stats are ephemeral and reset on app restart (not persisted)
//! - No regex, no recursive walks, no unbound I/O per CLAUDE.md
//! - All public types and functions carry doc-comments (workspace -D missing-docs)

pub mod theory;
pub mod native_dsp;
pub mod bangers;
pub mod types;
pub mod transition;
pub mod state_writer;
pub mod observation;
pub mod personality;
pub mod voice_commands;
pub mod dream_backend;
pub mod sovereign_focus;
pub mod companion_bridge;
pub mod starmonics;

pub use bangers::{BangerScore, BangerTracker};
pub use theory::{parse_camelot, keys_compatible, compatible_keys, transition_type, TransitionType};
pub use native_dsp::{blend_gains, filter_freq, filter_resonance, lowpass_1pole,
                     tension_dry_gain, saturate, compress};
pub use types::{DjMode, BrainMode, BroskiArchetype, DeckId, EqBand, ActiveFx, V2DeckState, MixerState,
                DjAction, DjNotification, GhostActivity, GhostState};
pub use transition::{DjAssistant, TransitionState, DjSuggestion};
pub use observation::ObservationBuffer;
pub use personality::BroskiPersonality;
pub use sovereign_focus::SovereignEngine;
pub use voice_commands::parse_voice_command;
pub use starmonics::{StarMonzo, STAR_MONZOS, star_monzo};
