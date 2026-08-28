//! Psychoacoustic transforms — the master fauna/absence chain.
//!
//! Runs fauna synthesis then absence sculpting in the correct order: add texture
//! first, then carve absence from it. The result carries living-world texture
//! with specific frequencies missing — the psychoacoustic signature of wrongness.
//!
//! Pipeline:
//!   1. [`super::fauna_sound::process`] — additive: synthesized prairie creatures
//!   2. [`super::absence::process`]     — subtractive: notch expected bands out
//!
//! Params: the merged superset of both sub-systems (see [`super`] module docs).

pub fn process(samples: &mut [f32], sample_rate: u32, params: &serde_json::Value) {
    // 1. Additive: synthesize fauna and mix in.
    super::fauna_sound::process(samples, sample_rate, params);

    // 2. Subtractive: carve absence from the expected frequency bands.
    super::absence::process(samples, sample_rate, params);
}
</content>
