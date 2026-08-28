//! Broski personality system — archetype selection from aggression level.

use crate::broski::types::{DjMode, BrainMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archetype { Shadow, Senex, Trickster }

/// Authored narrative mix over the archetypes, Permyriad — drained from
/// `dirge-of-ironroot/lore/narrative_weights.json` (`archetype_distribution`,
/// authored as percentages). The aggression ladder above picks ONE archetype
/// for a given level; this is the opposite question — across a whole run, how
/// often should each voice be the one talking.
///
/// The seed also names a fourth slot, `THE_429`, at 1%. It is deliberately NOT
/// an [`Archetype`] variant: 429 is the rate-limit status, the voice that shows
/// up only when the companion is being throttled, and it has no aggression
/// level that selects it. It rides [`RARE_429_PERMYRIAD`] instead.
pub const ARCHETYPE_MIX: [(Archetype, u16); 3] =
    [(Archetype::Senex, 6_000), (Archetype::Shadow, 3_000), (Archetype::Trickster, 900)];

/// The 1% that belongs to no archetype (`THE_429`). Permyriad.
pub const RARE_429_PERMYRIAD: u16 = 100;

/// Authored global entropy, Permyriad (`0.15` in the seed) — how much the
/// narrative lane is allowed to drift from the mix on any single choice.
pub const GLOBAL_ENTROPY_PERMYRIAD: u16 = 1_500;

/// This archetype's authored share of the mix, Permyriad.
pub fn mix_weight(archetype: Archetype) -> u16 {
    ARCHETYPE_MIX
        .iter()
        .find(|(a, _)| *a == archetype)
        .map(|(_, w)| *w)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct BroskiPersonality {
    pub mode: DjMode,
    pub brain: BrainMode,
    pub aggression: u8,
    pub archetype: Archetype,
}

impl Default for BroskiPersonality {
    fn default() -> Self {
        Self::new()
    }
}

impl BroskiPersonality {
    pub fn new() -> Self {
        Self { mode: DjMode::Sidekick, brain: BrainMode::Hybrid, aggression: 0, archetype: Archetype::Shadow }
    }

    pub fn archetype_from_aggression(level: u8) -> Archetype {
        match level {
            0 => Archetype::Shadow,
            1..=6 => Archetype::Senex,
            _ => Archetype::Trickster,
        }
    }

    pub fn set_aggression(&mut self, level: u8) {
        self.aggression = level.min(10);
        self.archetype = Self::archetype_from_aggression(self.aggression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetype_shadow() { assert_eq!(BroskiPersonality::archetype_from_aggression(0), Archetype::Shadow); }
    #[test]
    fn test_archetype_trickster() { assert_eq!(BroskiPersonality::archetype_from_aggression(10), Archetype::Trickster); }
    #[test]
    fn test_mode_toggle() {
        let mut p = BroskiPersonality::new();
        p.mode = DjMode::Autopilot;
        assert_eq!(p.mode, DjMode::Autopilot);
    }
    #[test]
    fn test_brain_toggle() {
        let mut p = BroskiPersonality::new();
        p.brain = BrainMode::CeRules;
        assert_eq!(p.brain, BrainMode::CeRules);
    }
    // The mix is a DISTRIBUTION — it must close. Senex 60 + Shadow 30 +
    // Trickster 9 + the 429's 1 = 100%, so a drifting weight cannot silently
    // steal probability from another voice.
    #[test]
    fn the_narrative_mix_closes_at_one_hundred_percent() {
        let sum: u32 = ARCHETYPE_MIX.iter().map(|(_, w)| *w as u32).sum::<u32>()
            + RARE_429_PERMYRIAD as u32;
        assert_eq!(sum, 10_000, "the archetype mix does not close");
        assert_eq!(mix_weight(Archetype::Senex), 6_000, "Senex is the default voice");
        assert!(
            mix_weight(Archetype::Senex) > mix_weight(Archetype::Shadow),
            "the elder talks more than the shadow"
        );
        assert!(GLOBAL_ENTROPY_PERMYRIAD < 10_000, "entropy is a fraction, not a multiplier");
    }

    // Every archetype the aggression ladder can SELECT must have a share of the
    // mix — a voice that can be chosen but never scheduled is a dead voice.
    #[test]
    fn every_selectable_archetype_carries_weight() {
        for level in 0u8..=10 {
            let a = BroskiPersonality::archetype_from_aggression(level);
            assert!(mix_weight(a) > 0, "{a:?} is selectable at {level} but has no share");
        }
    }

    #[test]
    fn test_aggression_clamp() {
        let mut p = BroskiPersonality::new();
        p.set_aggression(255);
        assert_eq!(p.aggression, 10);
    }
}
