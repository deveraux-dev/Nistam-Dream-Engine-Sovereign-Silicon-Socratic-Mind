//! Oath Disciplines — 8 weapon-bound combat identities replacing zodiac classes.
//!
//! Each discipline defines combat style, traversal ability, risk profile,
//! and Shadow counter-bias.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The 8 weapon-bound combat identities (replaces zodiac classes).
pub enum OathDiscipline {
    /// Knife discipline: precision, bleed, counters.
    Knife,
    /// Hammer discipline: stagger, armor break, weight.
    Hammer,
    /// Chain discipline: spacing, pull, bind, disarm.
    Chain,
    /// Veil discipline: dodge, feint, misdirection.
    Veil,
    /// Brand discipline: heat, ash, pressure, marking.
    Brand,
    /// Bell discipline: parry, rhythm, resonance.
    Bell,
    /// Nail discipline: defense, pinning, endurance.
    Nail,
    /// Thread discipline: lifeline, tether, recall.
    Thread,
}

impl OathDiscipline {
    /// Convert a numeric index (0-7) to an oath discipline (wraps with modulo).
    pub fn from_index(i: u8) -> Self {
        match i % 8 {
            0 => Self::Knife,
            1 => Self::Hammer,
            2 => Self::Chain,
            3 => Self::Veil,
            4 => Self::Brand,
            5 => Self::Bell,
            6 => Self::Nail,
            _ => Self::Thread,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Combat style/capability granted by each oath discipline.
pub enum CombatIdentity {
    /// Knife's core combat style: precision, bleed, counters.
    PrecisionBleedCounters,
    /// Hammer's core combat style: stagger, armor break, weight.
    StaggerArmorBreakWeight,
    /// Chain's core combat style: spacing, pull, bind, disarm.
    SpacingPullBindDisarm,
    /// Veil's core combat style: dodge, feint, misdirection.
    DodgeFeintMisdirection,
    /// Brand's core combat style: heat, ash, pressure, marking.
    HeatAshPressureMarking,
    /// Bell's core combat style: parry, rhythm, resonance.
    ParryRhythmResonance,
    /// Nail's core combat style: defense, pinning, endurance.
    DefensePinningEndurance,
    /// Thread's core combat style: lifeline, tether, recall.
    LifelineTetherRecall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Traversal/mobility ability granted by each oath discipline.
pub enum TraversalAbility {
    /// Knife's traversal: cut seams, secret stitches (fabric/spatial pathways).
    CutSeamsSecretStitches,
    /// Hammer's traversal: break walls, floors, latches (brute force).
    BreakWallsFloorsLatches,
    /// Chain's traversal: swing, drag, counterweight (rope/leverage).
    SwingDragCounterweight,
    /// Veil's traversal: phase through spirit seams (spiritual passage).
    PhaseThroughSpiritSeams,
    /// Brand's traversal: burn root locks and seal wax (thermal barriers).
    BurnRootLocksAndSealWax,
    /// Bell's traversal: open sound gates, shatter walls (sonic vibration).
    OpenSoundGatesShatterWalls,
    /// Nail's traversal: anchor against force or flood (absolute fixation).
    AnchorAgainstForceOrFlood,
    /// Thread's traversal: create return points, pull memory (time/space folding).
    CreateReturnPointsPullMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Strategic risk/weakness of each oath discipline.
pub enum DisciplineRisk {
    /// Knife's risk: brittle if overcommitted.
    FragilityOvercommitment,
    /// Hammer's risk: slow recovery after attacks.
    SlowRecovery,
    /// Chain's risk: positioning gaps expose player.
    PositionalExposure,
    /// Veil's risk: cannot deliver direct/raw force.
    LowDirectForce,
    /// Brand's risk: heat/bloom accelerates root cycle.
    RaisesRootBloom,
    /// Bell's risk: resonance attracts shadow attention.
    AttractsShadow,
    /// Nail's risk: high commitment, low repositioning.
    LowMobility,
    /// Thread's risk: tether mechanics accumulate entropy debt.
    RaisesEntropyDebt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// How the Shadow fights back against each discipline's strengths.
pub enum ShadowCounterBias {
    /// Against Knife: baited parry at inside angle.
    BaitedParryInsideAngle,
    /// Against Hammer: whiff punish with delayed grab.
    WhiffPunishDelayedGrab,
    /// Against Chain: chain cut, reversal, pull.
    ChainCutReversalPull,
    /// Against Veil: delayed pursuit, false openings.
    DelayedPursuitFalseOpenings,
    /// Against Brand: ash mirror, thermal afterimage.
    AshMirrorThermalAfterimage,
    /// Against Bell: offbeat strikes, silence fields.
    OffbeatStrikesSilenceFields,
    /// Against Nail: rooted stance breaker.
    RootedStanceBreaker,
    /// Against Thread: tether inversion.
    TetherInversion,
}

#[derive(Debug, Clone, Copy)]
/// Complete attribute profile for an oath discipline.
pub struct DisciplineProfile {
    /// Which oath discipline this profile describes.
    pub discipline: OathDiscipline,
    /// Combat style granted by this discipline.
    pub combat: CombatIdentity,
    /// Traversal ability granted by this discipline.
    pub traversal: TraversalAbility,
    /// Strategic risk of this discipline.
    pub risk: DisciplineRisk,
    /// How the shadow counters this discipline.
    pub shadow_counter: ShadowCounterBias,
    /// Base stat bias: [str, dex, con, int, wis, cha, luck, will].
    pub stat_bias: [i16; 8],
}

/// Retrieve the complete profile for an oath discipline.
pub fn profile(d: OathDiscipline) -> DisciplineProfile {
    match d {
        OathDiscipline::Knife => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::PrecisionBleedCounters,
            traversal: TraversalAbility::CutSeamsSecretStitches,
            risk: DisciplineRisk::FragilityOvercommitment,
            shadow_counter: ShadowCounterBias::BaitedParryInsideAngle,
            stat_bias: [2, 5, -2, 1, 0, 0, 2, 0],
        },
        OathDiscipline::Hammer => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::StaggerArmorBreakWeight,
            traversal: TraversalAbility::BreakWallsFloorsLatches,
            risk: DisciplineRisk::SlowRecovery,
            shadow_counter: ShadowCounterBias::WhiffPunishDelayedGrab,
            stat_bias: [5, -2, 3, 0, 0, 0, 0, 2],
        },
        OathDiscipline::Chain => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::SpacingPullBindDisarm,
            traversal: TraversalAbility::SwingDragCounterweight,
            risk: DisciplineRisk::PositionalExposure,
            shadow_counter: ShadowCounterBias::ChainCutReversalPull,
            stat_bias: [1, 3, 0, 0, 2, 0, 0, 2],
        },
        OathDiscipline::Veil => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::DodgeFeintMisdirection,
            traversal: TraversalAbility::PhaseThroughSpiritSeams,
            risk: DisciplineRisk::LowDirectForce,
            shadow_counter: ShadowCounterBias::DelayedPursuitFalseOpenings,
            stat_bias: [-1, 4, -1, 2, 1, 2, 1, 0],
        },
        OathDiscipline::Brand => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::HeatAshPressureMarking,
            traversal: TraversalAbility::BurnRootLocksAndSealWax,
            risk: DisciplineRisk::RaisesRootBloom,
            shadow_counter: ShadowCounterBias::AshMirrorThermalAfterimage,
            stat_bias: [3, 0, 1, 2, 0, 0, 0, 2],
        },
        OathDiscipline::Bell => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::ParryRhythmResonance,
            traversal: TraversalAbility::OpenSoundGatesShatterWalls,
            risk: DisciplineRisk::AttractsShadow,
            shadow_counter: ShadowCounterBias::OffbeatStrikesSilenceFields,
            stat_bias: [0, 2, 1, 0, 3, 1, 0, 1],
        },
        OathDiscipline::Nail => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::DefensePinningEndurance,
            traversal: TraversalAbility::AnchorAgainstForceOrFlood,
            risk: DisciplineRisk::LowMobility,
            shadow_counter: ShadowCounterBias::RootedStanceBreaker,
            stat_bias: [2, -2, 5, 0, 0, 0, 0, 3],
        },
        OathDiscipline::Thread => DisciplineProfile {
            discipline: d,
            combat: CombatIdentity::LifelineTetherRecall,
            traversal: TraversalAbility::CreateReturnPointsPullMemory,
            risk: DisciplineRisk::RaisesEntropyDebt,
            shadow_counter: ShadowCounterBias::TetherInversion,
            stat_bias: [0, 1, 0, 3, 2, 0, 1, 1],
        },
    }
}

/// Apply oath stat bias to a base combat profile.
pub fn apply_oath_bias(base: &mut [u16; 8], discipline: OathDiscipline) {
    let p = profile(discipline);
    for (stat, bias) in base.iter_mut().zip(p.stat_bias.iter()) {
        *stat = (*stat as i16 + bias).max(0) as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_disciplines_have_profiles() {
        let all = [
            OathDiscipline::Knife, OathDiscipline::Hammer, OathDiscipline::Chain,
            OathDiscipline::Veil, OathDiscipline::Brand, OathDiscipline::Bell,
            OathDiscipline::Nail, OathDiscipline::Thread,
        ];
        for d in all {
            let p = profile(d);
            assert_eq!(p.discipline, d);
        }
    }

    #[test]
    fn stat_bias_applies_correctly() {
        let mut base = [10u16; 8];
        apply_oath_bias(&mut base, OathDiscipline::Knife);
        assert_eq!(base[0], 12); // str +2
        assert_eq!(base[1], 15); // dex +5
        assert_eq!(base[2], 8);  // con -2
    }

    #[test]
    fn stat_bias_floors_at_zero() {
        let mut base = [1u16; 8];
        apply_oath_bias(&mut base, OathDiscipline::Hammer);
        assert_eq!(base[1], 0); // dex -2 from 1 = floors at 0
    }

    #[test]
    fn each_discipline_has_unique_combat_identity() {
        let all = [
            OathDiscipline::Knife, OathDiscipline::Hammer, OathDiscipline::Chain,
            OathDiscipline::Veil, OathDiscipline::Brand, OathDiscipline::Bell,
            OathDiscipline::Nail, OathDiscipline::Thread,
        ];
        let combats: Vec<_> = all.iter().map(|d| profile(*d).combat).collect();
        for i in 0..combats.len() {
            for j in (i+1)..combats.len() {
                assert_ne!(combats[i], combats[j]);
            }
        }
    }
}
