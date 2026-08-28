//! Fauna / absence / chromabind — procedural prairie psychoacoustics.
//!
//! Ported into forge-audio from the `dead-drop-private` DSP crate (Rust,
//! sample-level), itself a port of the 13moons GDScript audio engine:
//! `CricketAmbientField` / `BirdChorusField` / `AbsenceEngine` / `chromabeat_synth`.
//! Fully procedural — **no sample files**. Stream-safe: the source compiles into
//! the binary, only the effect is audible.
//!
//! ## Master chain
//! [`psychoacoustic::process`] is the entry point. It runs **fauna synthesis
//! (additive)** then **absence sculpting (subtractive)** in order: add living-world
//! texture first, then carve specific expected frequencies out so the brain
//! registers the *missing* sound as unease. Call once per audio buffer.
//!
//! ## Threading invariant (SR&ED #35)
//! Each sub-module keeps DSP state in a `OnceLock<UnsafeCell<_>>` that assumes a
//! **single audio thread** — `process` must never be called concurrently. This
//! matches forge-audio's lock-free audio-callback model ([`crate::realtime`]).
//!
//! ## Params (serde_json, merged superset)
//! `vibe_position` 0..1 · `walker_active` bool · `tick_id` u64 · `moon` 0..12 ·
//! `time_of_day` dawn|morning|midday|afternoon|dusk|night|deep_night ·
//! `temperature_c` f64 · `fauna_gain` 0..1 · `fauna_density{species:0..1}` ·
//! `fauna_silent[]` · `fauna_absent[]`.

pub mod absence;
pub mod chromabind;
pub mod code_voice;
pub mod fauna_sound;
pub mod psychoacoustic;

/// Master psychoacoustic chain (fauna additive → absence subtractive).
pub use psychoacoustic::process as process_psychoacoustic;
/// Map an RGBA colour to a bound audio frequency (Cree moon-cycle root key).
pub use chromabind::bind as color_to_frequency;
/// L5 LATERAL: Age-adaptive band selection for the AbsenceEngine.
pub use absence::ListenerProfile;
/// CodeVoice — build/compile outcome → deterministic fauna params (the sound of your code).
/// VoiceBank — the lock-free, alloc-free live bridge the audio thread borrows from.
pub use code_voice::{CodeVoice, VoiceBank};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn energy(buf: &[f32]) -> f32 {
        buf.iter().map(|s| s * s).sum::<f32>()
    }

    #[test]
    fn chromabind_deterministic() {
        // Pure fn, no shared state — safe in parallel.
        // R=0, A=0 → base 220 Hz, moon 0 → no shift.
        assert!((chromabind::bind([0, 0, 0, 0]) - 220.0).abs() < 0.01);
        // R=255 → 880 Hz at moon 0.
        assert!((chromabind::bind([255, 0, 0, 0]) - 880.0).abs() < 0.01);
        // Same colour → same frequency, always.
        assert_eq!(chromabind::bind([128, 9, 9, 5]), chromabind::bind([128, 9, 9, 5]));
        // Wave-type buckets.
        assert_eq!(chromabind::wave_type([0, 0, 40, 0]), 0); // sine
        assert_eq!(chromabind::wave_type([0, 0, 120, 0]), 1); // square
        assert_eq!(chromabind::wave_type([0, 0, 250, 0]), 2); // saw
    }

    // All stateful (OnceLock<UnsafeCell>) DSP runs in ONE test fn so the
    // single-audio-thread invariant holds (no parallel-test race on the statics).
    #[test]
    fn fauna_chain_synthesizes_then_sculpts() {
        let sr = 44_100u32;
        // 2.2s buffer — long enough for staggered crickets to fire at least once.
        let mut buf = vec![0.0f32; sr as usize * 2 + sr as usize / 5];

        let fauna_params = json!({
            "temperature_c": 25.0,
            "time_of_day": "dawn",
            "fauna_gain": 1.0,
            "vibe_position": 0.0,
            "walker_active": false,
            "fauna_density": { "cricket": 1.0, "meadowlark": 1.0, "raven": 1.0 }
        });

        // RED→GREEN: a silent buffer gains energy once fauna is synthesized in.
        assert_eq!(energy(&buf), 0.0, "buffer must start silent");
        fauna_sound::process(&mut buf, sr, &fauna_params);
        let after_fauna = energy(&buf);
        assert!(after_fauna > 0.0, "fauna_sound must add procedural texture");
        assert!(buf.iter().all(|s| s.is_finite()), "no NaN/inf from fauna");

        // Absence is subtractive + must stay finite (biquad stability under sweep).
        let absence_params = json!({
            "vibe_position": 1.0,
            "walker_active": true,
            "tick_id": 7u64,
            "moon": 3u64,
            "fauna_density": { "cricket": 1.0, "bird_song": 1.0 }
        });
        // Run several buffers so the notch sweep reaches its targets, then prove
        // it carves energy out rather than blowing up.
        let mut sculpt = buf.clone();
        for _ in 0..40 {
            absence::process(&mut sculpt, sr, &absence_params);
        }
        assert!(sculpt.iter().all(|s| s.is_finite()), "absence biquad stayed stable");

        // Master chain runs both in order without panicking.
        let mut chain = vec![0.0f32; sr as usize / 2];
        psychoacoustic::process(&mut chain, sr, &fauna_params);
        assert!(chain.iter().all(|s| s.is_finite()), "psychoacoustic chain finite");
    }
}
</content>
