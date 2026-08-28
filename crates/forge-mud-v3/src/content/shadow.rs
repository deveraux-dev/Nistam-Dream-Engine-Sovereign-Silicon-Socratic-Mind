//! The shadow-tier ladder, drained from forge-insights `rpg/dauer.rs:11-73`
//! (`ShadowTier` enum, `.name()`/`.whisper()`). [ASSUMED: flavour link
//! only] — wired in `game::Game::status` off the operator's EXISTING
//! `heat` register (consequence.rs law-escalation counter), not a new
//! fail-streak counter; this session's recon found no fail-streak state in
//! forge-mud-v3, so `heat` is the closest real analog to bucket.

/// The six shadow tiers: (name, whisper), Unseen..Harbinger order.
pub const SHADOW_TIERS: &[(&str, &str)] = &[
    ("Unseen", "Nothing follows you."),
    ("Replay", "Something walked your road twice."),
    ("Pattern", "It knows where you turn."),
    ("Counterpart", "It moves when you move."),
    ("Witness", "It is taking notes."),
    ("Harbinger", "It gets there first now."),
];

#[cfg(test)]
mod tests {
    #[test]
    fn shadow_tiers_are_exactly_six_and_word_only() {
        assert_eq!(super::SHADOW_TIERS.len(), 6);
        for (name, whisper) in super::SHADOW_TIERS {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!whisper.is_empty() && whisper.is_ascii());
            assert!(!name.chars().any(|c| c.is_ascii_digit()));
            assert!(!whisper.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
