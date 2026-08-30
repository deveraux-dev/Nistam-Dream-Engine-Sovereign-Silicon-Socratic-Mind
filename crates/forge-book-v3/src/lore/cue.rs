//! Cue derivation — turn a [`LineEntry`] into a `LoreCue` (the lore-side
//! shape that mirrors `forge_harmonics::HarmonicDialogueCue` 1:1).
//!
//! forge-lore stays bottom-of-graph (no dep on forge-harmonics). The bridge
//! `impl From<LoreCue> for HarmonicDialogueCue` belongs in forge-harmonics
//! and is a future chip; pure-data shape lives here.

use crate::lore::entry::LineEntry;
use serde::{Deserialize, Serialize};

/// The lore-side cue shape. Field-for-field mirror of
/// `forge_harmonics::HarmonicDialogueCue` so a one-line `From` impl in a
/// future forge-harmonics module can convert without re-mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreCue {
    /// Mirrors `HarmonicDialogueCue::cue_id`. Set to `LineEntry::line_id`.
    pub cue_id: u64,
    /// Mirrors `HarmonicDialogueCue::speaker_id`. Set to `LineEntry::voice_id`.
    pub speaker_id: u64,
    /// Mirrors `HarmonicDialogueCue::synthxml_fragment`. Cartridge supplies
    /// the per-voice fragment handle; forge-lore does not invent fragments.
    pub synthxml_fragment: u64,
    /// Mirrors `HarmonicDialogueCue::required_dialogue_tags`.
    pub required_dialogue_tags: Vec<u64>,
    /// Mirrors `HarmonicDialogueCue::required_sieve_tags`. Node-level; tree
    /// wires these in from the [`DialogueNode`](crate::lore::tree::DialogueNode).
    pub required_sieve_tags: Vec<u64>,
    /// Mirrors `HarmonicDialogueCue::cooldown_ticks`. Derived from `line_pace`
    /// via [`derive_cooldown_from_pace`].
    pub cooldown_ticks: u32,
    /// Mirrors `HarmonicDialogueCue::priority`. Defaults to 0; cartridge can
    /// override per-line via metadata (future).
    pub priority: i32,
}

/// Derive a [`LoreCue`] from a [`LineEntry`]. Pure function, no state.
///
/// `synthxml_fragment_for_voice` is a per-voice resource handle supplied by
/// the cartridge — forge-lore does not invent SynthXML fragments.
pub fn derive_cue(entry: &LineEntry, synthxml_fragment_for_voice: u64) -> LoreCue {
    LoreCue {
        cue_id: entry.line_id,
        speaker_id: entry.voice_id,
        synthxml_fragment: synthxml_fragment_for_voice,
        required_dialogue_tags: entry.dialogue_tags.clone(),
        required_sieve_tags: Vec::new(),
        cooldown_ticks: derive_cooldown_from_pace(entry.line_pace),
        priority: 0,
    }
}

/// Map a Permyriad pace value to a cue cooldown in ticks.
///
/// Nominal pace (`5000`) → 60 ticks (the legacy default). Slower paces
/// stretch the cooldown linearly via integer division; faster paces
/// compress it. Bounded `[15, 60]` so a misconfigured pace can't escape.
pub fn derive_cooldown_from_pace(line_pace_permyriad: u16) -> u32 {
    // Pace `5000` should yield `60`. The formula keeps that fixed point:
    // ticks ≈ (60 * 5000) / max(pace, 1000), clamped.
    let pace = (line_pace_permyriad as u32).max(1000);
    let raw = (60u32 * 5000u32) / pace;
    raw.clamp(15, 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_nominal_pace_is_60() {
        assert_eq!(derive_cooldown_from_pace(5000), 60);
    }

    #[test]
    fn cooldown_is_clamped_low() {
        // Very fast pace would otherwise undershoot.
        assert_eq!(derive_cooldown_from_pace(10000), 30);
        assert_eq!(derive_cooldown_from_pace(20000), 15); // u16::MAX-ish path
    }

    #[test]
    fn cooldown_is_clamped_high() {
        // Very slow pace would otherwise overshoot.
        assert_eq!(derive_cooldown_from_pace(0), 60); // pace floored to 1000 -> 300 -> clamp 60
        assert_eq!(derive_cooldown_from_pace(1000), 60); // 300 -> clamp 60
    }

    #[test]
    fn cooldown_monotonic_within_bounds() {
        // Faster pace should yield a smaller or equal cooldown.
        let slow = derive_cooldown_from_pace(3000);
        let mid = derive_cooldown_from_pace(5000);
        let fast = derive_cooldown_from_pace(7500);
        assert!(slow >= mid);
        assert!(mid >= fast);
    }

    #[test]
    fn derive_cue_copies_identity_fields() {
        let mut e = LineEntry::new_with_defaults(42, 100, "hello");
        e.dialogue_tags = vec![7, 8, 9];
        e.line_pace = 5000;
        let cue = derive_cue(&e, 999);
        assert_eq!(cue.cue_id, 42);
        assert_eq!(cue.speaker_id, 100);
        assert_eq!(cue.synthxml_fragment, 999);
        assert_eq!(cue.required_dialogue_tags, vec![7, 8, 9]);
        assert_eq!(cue.required_sieve_tags, Vec::<u64>::new());
        assert_eq!(cue.cooldown_ticks, 60);
        assert_eq!(cue.priority, 0);
    }

    #[test]
    fn derive_cue_propagates_pace_to_cooldown() {
        let mut e = LineEntry::new_with_defaults(1, 2, "x");
        e.line_pace = 10000;
        let cue = derive_cue(&e, 0);
        assert_eq!(cue.cooldown_ticks, 30);
    }
}
