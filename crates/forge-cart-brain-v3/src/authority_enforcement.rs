// Ported by translation from quarry ironroot-edict (pure leaf) — RunDevRun cart World/Level sprint.
//! Authority Enforcement — executes Bonification/Maltreatment verdicts.
//!
//! The DissonanceVerdict determines WHO wins. This module determines WHAT HAPPENS.
//!
//! Bonification (superior benefic): difficulty smoothing, resource injection, crit guarantee.
//! Maltreatment (superior malefic): state veto, stat corruption, progress halt.
//! Mitigation (superior neutral): partial advantage, no corruption.
//!
//! Hidden bypasses (antiscia/reception) allow inferior entities to escape maltreatment.
//!
//! Stateless, deterministic, no alloc.

use crate::dissonance_sieve::{AuthorityOutcome, DissonanceVerdict, HarmonicBody, ClassicalElement};

// ── Enforcement Results ──────────────────────────────────────────────────────

/// What actually happens to the target after authority resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementEffect {
    /// No positional advantage. Standard symmetric exchange.
    Neutral,
    /// Benefic bonification: smooth difficulty, grant resources.
    Bonification {
        /// Permyriad crit chance bonus (added to base). 10000 = guaranteed crit.
        crit_bonus_q: i32,
        /// Permyriad damage reduction on self (incoming damage softened).
        damage_reduction_q: i32,
        /// Ticks of invulnerability granted (0 = none).
        grace_ticks: u16,
    },
    /// Malefic maltreatment: state veto, corruption.
    Maltreatment {
        /// Permyriad stat corruption applied to target (reduces all stats).
        stat_corruption_q: i32,
        /// Ticks the target's inputs are vetoed (cannot act).
        veto_ticks: u16,
        /// Whether target's active ability is cancelled.
        ability_cancel: bool,
    },
    /// Neutral mitigation: partial advantage, no corruption.
    Mitigation {
        /// Permyriad damage bonus.
        damage_bonus_q: i32,
    },
}

// ── Bypass Mechanics (Antiscia / Reception) ──────────────────────────────────

/// Hidden symmetry that allows an inferior entity to mitigate maltreatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MitigationBypass {
    /// No bypass available. Full maltreatment applies.
    None,
    /// Reception: entities share a mutual elemental affinity (e.g., Fire in Fire's sign).
    /// Reduces maltreatment severity by 50%.
    Reception,
    /// Antiscia: entities are linked across an axis of equal daylight.
    /// Converts maltreatment into mitigation (no corruption, just damage).
    Antiscia,
}

/// Check if a bypass exists between attacker and defender.
/// Reception: same element = mutual affinity.
/// Antiscia: complementary elements (Fire↔Air, Water↔Earth) across the solstice axis.
pub fn check_bypass(attacker: &HarmonicBody, defender: &HarmonicBody) -> MitigationBypass {
    // Reception: defender's element matches attacker's element (mutual dignity)
    if attacker.element == defender.element {
        return MitigationBypass::Reception;
    }

    // Antiscia: complementary pairs across solstice axis
    let antiscia = matches!(
        (attacker.element, defender.element),
        (ClassicalElement::Fire, ClassicalElement::Air)
        | (ClassicalElement::Air, ClassicalElement::Fire)
        | (ClassicalElement::Water, ClassicalElement::Earth)
        | (ClassicalElement::Earth, ClassicalElement::Water)
    );

    if antiscia {
        return MitigationBypass::Antiscia;
    }

    MitigationBypass::None
}

// ── Enforcement Resolution ───────────────────────────────────────────────────

/// Given a DissonanceVerdict, resolve what actually happens.
/// This is the "what" after the "who" is decided.
pub fn enforce(
    verdict: &DissonanceVerdict,
    attacker: &HarmonicBody,
    defender: &HarmonicBody,
) -> EnforcementEffect {
    match verdict.authority {
        AuthorityOutcome::None | AuthorityOutcome::Clash => EnforcementEffect::Neutral,

        AuthorityOutcome::Bonification => {
            // Scale by power modifier (higher = more grace)
            let power = verdict.power_modifier_q;
            EnforcementEffect::Bonification {
                crit_bonus_q: (power - 1000).max(0), // excess over neutral = crit
                damage_reduction_q: (power / 4).min(5000), // up to 50% DR
                grace_ticks: if power > 2000 { 12 } else { 0 }, // 100ms grace at high power
            }
        }

        AuthorityOutcome::Maltreatment => {
            // Check for hidden bypasses
            let bypass = check_bypass(attacker, defender);

            match bypass {
                MitigationBypass::Antiscia => {
                    // Antiscia converts maltreatment → mitigation
                    EnforcementEffect::Mitigation {
                        damage_bonus_q: verdict.power_modifier_q / 2,
                    }
                }
                MitigationBypass::Reception => {
                    // Reception halves the severity
                    let power = verdict.power_modifier_q;
                    EnforcementEffect::Maltreatment {
                        stat_corruption_q: ((power - 1000).max(0)) / 2,
                        veto_ticks: if power > 2500 { 6 } else { 0 }, // halved
                        ability_cancel: false, // reception prevents cancel
                    }
                }
                MitigationBypass::None => {
                    // Full maltreatment
                    let power = verdict.power_modifier_q;
                    EnforcementEffect::Maltreatment {
                        stat_corruption_q: (power - 1000).max(0),
                        veto_ticks: if power > 2000 { 12 } else if power > 1500 { 6 } else { 0 },
                        ability_cancel: power > 2500,
                    }
                }
            }
        }

        AuthorityOutcome::Mitigation => {
            EnforcementEffect::Mitigation {
                damage_bonus_q: (verdict.power_modifier_q - 1000).max(0),
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dissonance_sieve::*;

    fn fire_body() -> HarmonicBody {
        HarmonicBody { element: ClassicalElement::Fire, tier: AlchemicalTier::Rubedo, resonance_hz: 800, inverse: false, mass_q: 5000 }
    }

    fn water_body() -> HarmonicBody {
        HarmonicBody { element: ClassicalElement::Water, tier: AlchemicalTier::Albedo, resonance_hz: 432, inverse: false, mass_q: 5000 }
    }

    fn earth_body() -> HarmonicBody {
        HarmonicBody { element: ClassicalElement::Earth, tier: AlchemicalTier::Nigredo, resonance_hz: 40, inverse: false, mass_q: 15000 }
    }

    #[test]
    fn clash_gives_neutral() {
        let v = DissonanceVerdict {
            authority: AuthorityOutcome::Clash,
            power_modifier_q: 1000,
            dissonance_pressure: 0,
            entropy_cost: 0,
        };
        assert_eq!(enforce(&v, &fire_body(), &water_body()), EnforcementEffect::Neutral);
    }

    #[test]
    fn bonification_grants_crit_and_grace() {
        let v = DissonanceVerdict {
            authority: AuthorityOutcome::Bonification,
            power_modifier_q: 2500,
            dissonance_pressure: 0,
            entropy_cost: 0,
        };
        let e = enforce(&v, &fire_body(), &water_body());
        match e {
            EnforcementEffect::Bonification { crit_bonus_q, grace_ticks, .. } => {
                assert_eq!(crit_bonus_q, 1500); // 2500 - 1000
                assert_eq!(grace_ticks, 12);    // power > 2000
            }
            _ => panic!("expected bonification"),
        }
    }

    #[test]
    fn maltreatment_vetoes_and_corrupts() {
        let v = DissonanceVerdict {
            authority: AuthorityOutcome::Maltreatment,
            power_modifier_q: 3000,
            dissonance_pressure: 0,
            entropy_cost: 0,
        };
        // Fire vs Water = no bypass (different elements, not complementary pair for antiscia)
        // Wait — Fire↔Air is antiscia, not Fire↔Water. So Fire vs Water = no bypass. Good.
        let e = enforce(&v, &fire_body(), &water_body());
        match e {
            EnforcementEffect::Maltreatment { stat_corruption_q, veto_ticks, ability_cancel } => {
                assert_eq!(stat_corruption_q, 2000); // 3000 - 1000
                assert_eq!(veto_ticks, 12);          // power > 2000
                assert!(ability_cancel);             // power > 2500
            }
            _ => panic!("expected maltreatment"),
        }
    }

    #[test]
    fn reception_halves_maltreatment() {
        let v = DissonanceVerdict {
            authority: AuthorityOutcome::Maltreatment,
            power_modifier_q: 3000,
            dissonance_pressure: 0,
            entropy_cost: 0,
        };
        // Same element = reception
        let e = enforce(&v, &fire_body(), &fire_body());
        match e {
            EnforcementEffect::Maltreatment { stat_corruption_q, ability_cancel, .. } => {
                assert_eq!(stat_corruption_q, 1000); // (3000-1000)/2
                assert!(!ability_cancel);            // reception prevents cancel
            }
            _ => panic!("expected halved maltreatment"),
        }
    }

    #[test]
    fn antiscia_converts_maltreatment_to_mitigation() {
        let v = DissonanceVerdict {
            authority: AuthorityOutcome::Maltreatment,
            power_modifier_q: 3000,
            dissonance_pressure: 0,
            entropy_cost: 0,
        };
        // Water vs Earth = antiscia (complementary pair)
        let e = enforce(&v, &water_body(), &earth_body());
        match e {
            EnforcementEffect::Mitigation { damage_bonus_q } => {
                assert_eq!(damage_bonus_q, 1500); // 3000/2
            }
            _ => panic!("expected mitigation via antiscia"),
        }
    }

    #[test]
    fn bypass_detection_same_element_is_reception() {
        assert_eq!(check_bypass(&fire_body(), &fire_body()), MitigationBypass::Reception);
    }

    #[test]
    fn bypass_detection_fire_air_is_antiscia() {
        let air = HarmonicBody { element: ClassicalElement::Air, tier: AlchemicalTier::Citrinitas, resonance_hz: -1, inverse: true, mass_q: 3500 };
        assert_eq!(check_bypass(&fire_body(), &air), MitigationBypass::Antiscia);
    }

    #[test]
    fn bypass_detection_fire_water_is_none() {
        assert_eq!(check_bypass(&fire_body(), &water_body()), MitigationBypass::None);
    }
}
