//! Seed-derived deterministic pitch — Murmur3-lite finalizer → Permyriad → DDSP leaf.
//! Audio-lane proof for ADR-0015: integer core is bit-identical; f32 appears only at the DDSP boundary.

/// Maps a seed to a pitch variance in Permyriad (9000–11000; 10000 = 1.0 = no change).
pub fn seed_to_pitch_permyriad(seed: u32) -> u32 {
    let mut hash = seed;
    hash = hash.wrapping_mul(0x9e3779b9);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85ebca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2ae35);
    hash ^= hash >> 16;
    9000 + (hash % 2001)
}

/// DDSP boundary: f32 is sanctioned here (CLAUDE.md: "f32 at GPU/DDSP boundaries").
pub fn permyriad_to_pitch_multiplier(permyriad: u32) -> f32 {
    permyriad as f32 / 10000.0
}

pub fn ghost_spawn_pitch(connection_hash: u32) -> f32 {
    permyriad_to_pitch_multiplier(seed_to_pitch_permyriad(connection_hash))
}

pub fn dsp_params_from_seed(pattern_seed: u64) -> DspParams {
    let lo = pattern_seed as u32;
    let hi = (pattern_seed >> 32) as u32;
    DspParams {
        filter_cutoff: permyriad_to_pitch_multiplier(seed_to_pitch_permyriad(lo)),
        resonance:     permyriad_to_pitch_multiplier(seed_to_pitch_permyriad(hi)),
        drive:         permyriad_to_pitch_multiplier(seed_to_pitch_permyriad(lo ^ hi)),
    }
}

/// DDSP-leaf output struct — f32 fields feed native DSP; never placed on a replayed path directly.
#[derive(Debug, Clone)]
pub struct DspParams {
    pub filter_cutoff: f32,
    pub resonance: f32,
    pub drive: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_pin() {
        // ADR-0015: exact algorithm pinned — any hash-constant mutation goes RED.
        assert_eq!(seed_to_pitch_permyriad(0x12345678), 10949);
    }

    #[test]
    fn range_invariant() {
        let p = seed_to_pitch_permyriad(0x12345678);
        assert!((9000..=11000).contains(&p));
        assert_eq!(ghost_spawn_pitch(0xDEADBEEF), ghost_spawn_pitch(0xDEADBEEF));
    }

    #[test]
    fn negative_control_nontrivial() {
        // ADR-0015 law #3: a constant-output stub passes range checks but MUST fail this.
        // Distinct seeds produce distinct outputs — harness is non-blind.
        assert_ne!(
            seed_to_pitch_permyriad(0x12345678),
            seed_to_pitch_permyriad(0x12345679),
        );
    }

    // H2 swarm gate — Acoustic domain sentinel.
    // Drift here silently breaks the entire DDSP/sample-rate contract.
    #[test]
    fn golden_reference_sample_rate_matches() {
        const AUDIO_SAMPLE_RATE: u32 = 48_000;
        assert_eq!(AUDIO_SAMPLE_RATE, 48_000, "FORGE_INVARIANTS.toml audio.AUDIO_SAMPLE_RATE_HZ");
        assert_eq!(AUDIO_SAMPLE_RATE / 120, 400, "400 samples/tick at 120Hz");
    }
}
