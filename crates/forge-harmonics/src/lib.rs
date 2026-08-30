//! Integer-only just-intonation pitch lattice and cents floor.
//!
//! Port of the v2 audio-reactive harmonics engine: Mersenne string physics
//! (prime-exponent monzos) and cents-to-millihertz DSP boundary crossing.
//! No floating-point arithmetic; all computation is integer-deterministic.

pub mod analysis;
pub mod camelot;
pub mod cents_floor;
pub mod delay_line;
pub mod dnb;
pub mod euclid;
pub mod gammatone;
pub mod mask;
pub mod mersenne_lattice;
pub mod scale_voice;
pub mod scale_mask;
pub mod scc_bridge;
pub mod shaderbind_bridge;
pub mod starmonics;
pub mod synthxml;
pub mod theory;
pub mod tonnetz;

#[cfg(feature = "musicxml")]
pub mod musicxml_extract;

pub use analysis::compute_dissonance;
pub use camelot::{
    is_audio_muted, set_audio_muted, toggle_audio_mute, init_mute_from_env,
    CamelotKey, HarmonicPreset, HarmonicVoiceNote, InteractiveHarmonicRouter,
    StreamClassifier, StreamKind, MUTE_AUDIO,
};
pub use cents_floor::Cents;
pub use delay_line::DelayLine;
pub use mersenne_lattice::{Monzo, Monzo11, PRIMES_11, CENTS_MICRO_11};
pub use scale_mask::ScaleMask;
pub use scale_voice::{
    note_to_mhz, VoicePreset, handle_note_on, word_note, word_note_in_key, word_degree,
    answer_melody, answer_melody_in_key, raw_word_pitch, PENTATONIC_C,
};
pub use starmonics::{nearest_star_monzo, star_monzo, StarMonzo, STAR_MONZOS};
pub use synthxml::{score_to_note_plan, ScheduledNote};

#[cfg(feature = "musicxml")]
pub use musicxml_extract::musicxml_to_score;
