//! Deterministic integer-only text transducer for prompt embedding and specialist routing.
//!
//! Maps prompts to fixed 512-element integer vectors and crate names to specialist IDs,
//! using FNV-1a hashing and Mersenne prime reduction for deterministic, heap-free operation.

/// Embed a text prompt into a fixed [i8; 512] vector.
///
/// Uses FNV-1a hash to compute a deterministic seed from the prompt bytes,
/// then generates each dimension via Mersenne prime reduction of a mixed value.
/// The spread of signs across dimensions serves as the basis for binarization
/// in the router's hamming distance calculations.
///
/// # Arguments
/// * `prompt` - Text to embed
///
/// # Returns
/// A 512-element vector where each element is derived deterministically from the prompt.
/// Identical prompts always produce identical vectors.
///
/// # Determinism
/// Pure function; no RNG, no heap allocation, no floats. Same input → same output,
/// always. Used downstream by `BqRouter::route` to classify requests.
pub fn embed_prompt(prompt: &str) -> [i8; 512] {
    // FNV-1a over the bytes.
    let mut seed: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    const FNV_PRIME: u64 = 0x100000001b3;

    for byte in prompt.as_bytes() {
        seed ^= *byte as u64;
        seed = seed.wrapping_mul(FNV_PRIME);
    }

    // Generate each dimension by reducing a mixed value modulo M61.
    let mut out = [0i8; 512];
    for d in 0..512 {
        let mix = forge_core_v3::reduce_m61(seed ^ (d as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15));
        out[d] = (mix & 0xFF) as u8 as i8;
    }

    out
}

/// Embed a token-id query into a fixed [i8; 512] vector.
///
/// The token-space twin of [`embed_prompt`]: same FNV-1a seed accumulation and
/// same M61 dimension reduction, folding each token's four little-endian bytes
/// instead of the prompt's UTF-8 bytes. A token slice and the text it decodes
/// to are NOT expected to embed alike — the two are separate entry points into
/// the same geometry, not a round trip.
///
/// # Arguments
/// * `tokens` - Token ids to embed
///
/// # Returns
/// A 512-element vector derived deterministically from the token sequence.
///
/// # Determinism
/// Pure function; no RNG, no heap allocation, no floats. Same input → same
/// output, always.
pub fn embed_tokens(tokens: &[u32]) -> [i8; 512] {
    let mut seed: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    for token in tokens {
        for byte in token.to_le_bytes() {
            seed ^= byte as u64;
            seed = seed.wrapping_mul(FNV_PRIME);
        }
    }

    let mut out = [0i8; 512];
    for d in 0..512 {
        let mix = forge_core_v3::reduce_m61(seed ^ (d as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15));
        out[d] = (mix & 0xFF) as u8 as i8;
    }

    out
}

/// Map a crate name to a Modality specialist ID, if recognized.
///
/// Returns `Some(id)` where id ∈ [0, 7) for known specialist domains,
/// or `None` if the crate does not match any known keyword pattern.
/// Unknown crates are never mislabeled; they are skipped in routing.
///
/// # MVP Mapping
/// Substring matching (case-insensitive) against the following patterns, in order:
/// - 0 (Sound): "audio", "sound", "sing", "harmon"
/// - 1 (SeeHear): "seehair", "seehear", "synesth"
/// - 2 (Technothesia): "technothesia", "termi", "tui", "pty"
/// - 3 (Animation): "anim", "sprite"
/// - 4 (Camera): "camera", "render", "gpu", "shader", "vision"
/// - 5 (Weather): "weather", "world", "zone", "climate"
/// - 6 (Script): "script", "vix", "lang", "grammar", "book", "corpus"
///
/// First matching pattern wins. This is a lightweight string-based heuristic;
/// a real semantic map is future work (TECH-DEBT).
///
/// # Arguments
/// * `crate_name` - The crate name to classify
///
/// # Returns
/// `Some(id)` if a pattern matches (0 ≤ id < 7), or `None` if unrecognized.
pub fn specialist_of(crate_name: &str) -> Option<u8> {
    let lower = crate_name.to_lowercase();

    // Order matters: first match wins.
    if lower.contains("audio") || lower.contains("sound") || lower.contains("sing") || lower.contains("harmon") {
        return Some(0);
    }
    if lower.contains("seehair") || lower.contains("seehear") || lower.contains("synesth") {
        return Some(1);
    }
    if lower.contains("technothesia") || lower.contains("termi") || lower.contains("tui") || lower.contains("pty") {
        return Some(2);
    }
    if lower.contains("anim") || lower.contains("sprite") {
        return Some(3);
    }
    if lower.contains("camera") || lower.contains("render") || lower.contains("gpu") || lower.contains("shader") || lower.contains("vision") {
        return Some(4);
    }
    if lower.contains("weather") || lower.contains("world") || lower.contains("zone") || lower.contains("climate") {
        return Some(5);
    }
    if lower.contains("script") || lower.contains("vix") || lower.contains("lang") || lower.contains("grammar") || lower.contains("book") || lower.contains("corpus") {
        return Some(6);
    }

    None
}

/// The Hermetic-7 keyword floor, ported verbatim (vocab and id order) from v2
/// `forge-daemon/src/flywheel_log.rs:140` `classify_specialist` — the
/// classifier that trained the NDE pair corpus this router harvests
/// (`pairs_{a,b,d,h,s,v,sieve}.jsonl`, see `NDE-CRATE-EXPERT-MAP-2026-04-12.md`).
/// One `(room_id, keywords)` row per specialist.
const HERMETIC_FLOOR: [(u8, &str); 7] = [
    (0, "audio camelot harmonic bpm daw sound music crossfade biquad lowpass highpass"),
    (1, "render shader wgpu gpu visual canvas glyph vixel vibe"),
    (2, "physics collision game rigidbody forge_geo geometry"),
    (3, "conductor sieve behavior behaviour forge_sieve conductor_host lane"),
    (4, "lore invention memory handoff decision roadmap architecture meta ledger provenance"),
    (5, "world terrain zone biome creature quest npc dialogue weather ecology"),
    (6, "system studio config daemon forge_daemon platform"),
];

/// Classify free text to a Hermetic-7 specialist id by keyword-hit count;
/// most hits wins, first row wins ties, zero hits is `None` (never mislabeled).
///
/// TAXONOMY NOTE, stated not hidden: these ids follow the v2 Hermetic seven
/// (0 Sound, 1 Visual, 2 Physics, 3 Sieve, 4 Lorekeeper, 5 World, 6 System) —
/// the id space the NDE corpus and its checkpoints were trained in. This
/// COLLIDES with [`specialist_of`]'s MVP crate-name taxonomy above (SeeHear/
/// Technothesia/Animation/...). Never merge pairs classified by both into one
/// `.bqr`; reconciling the two taxonomies is a named ARCH000 fork, not
/// something this function resolves.
pub fn specialist_of_text(query: &str) -> Option<u8> {
    let lower = query.to_lowercase();
    let mut best: Option<(u8, usize)> = None;
    for (id, vocab) in HERMETIC_FLOOR {
        let hits = vocab.split(' ').filter(|kw| lower.contains(kw)).count();
        if hits > 0 && best.map(|(_, b)| hits > b).unwrap_or(true) {
            best = Some((id, hits));
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_is_deterministic() {
        let v1 = embed_prompt("hello");
        let v2 = embed_prompt("hello");
        assert_eq!(v1, v2);
    }

    #[test]
    fn distinct_prompts_differ() {
        let v1 = embed_prompt("a");
        let v2 = embed_prompt("b");
        assert_ne!(v1, v2);
    }

    #[test]
    fn embed_is_512() {
        let v = embed_prompt("test");
        assert_eq!(v.len(), 512);
        // Verify a couple of dimensions are actually set.
        assert!(v[0] != 0 || v[1] != 0 || v[2] != 0);
    }

    #[test]
    fn specialist_known_and_unknown() {
        assert_eq!(specialist_of("forge-audio"), Some(0));
        assert_eq!(specialist_of("forge-core-v3"), None);
    }

    #[test]
    fn text_floor_routes_each_room_and_refuses_unknown() {
        assert_eq!(specialist_of_text("crossfade the audio stems at matched bpm"), Some(0));
        assert_eq!(specialist_of_text("the render pass binds a wgpu shader"), Some(1));
        assert_eq!(specialist_of_text("rigidbody collision response"), Some(2));
        assert_eq!(specialist_of_text("the sieve promotes a behavior lane"), Some(3));
        assert_eq!(specialist_of_text("record the decision in the provenance ledger"), Some(4));
        assert_eq!(specialist_of_text("spawn a creature in the terrain biome"), Some(5));
        assert_eq!(specialist_of_text("the daemon reads its platform config"), Some(6));
        // Zero hits: refused, never mislabeled.
        assert_eq!(specialist_of_text("quantum knitting"), None);
    }

    #[test]
    fn text_floor_most_hits_wins() {
        // One 'world' hit vs three audio hits — audio must win.
        assert_eq!(
            specialist_of_text("world-class audio: bpm and harmonic mix"),
            Some(0)
        );
    }

    #[test]
    fn specialist_is_total_range() {
        // Check that all possible returns from specialist_of are in the range [0, 7).
        let test_cases = vec![
            "audio-lib",
            "seehear-engine",
            "technothesia-tui",
            "sprite-anim",
            "gpu-renderer",
            "weather-sim",
            "script-lang",
            "unknown-crate",
        ];

        for name in test_cases {
            if let Some(id) = specialist_of(name) {
                assert!(id < 7, "specialist_of returned {} for {} (must be < 7)", id, name);
            }
        }
    }
}
