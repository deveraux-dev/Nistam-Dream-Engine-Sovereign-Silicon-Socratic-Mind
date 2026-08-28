//! Spawn score calculation and stage selection.

use crate::spawn::{WorldBossSpawnInputs, FaeLayerInputs};
use crate::world_boss::{RootStateThresholds, SpawnStage, HEALTHY_ROOT, CORRUPTED_ROOT, VOID_LEAK};

/// Compute weighted spawn score from universal world boss inputs.
pub fn world_boss_spawn_score(inputs: &WorldBossSpawnInputs) -> i32 {
    let base = inputs.faction_pressure_q as i32
        + inputs.crime_pressure_q as i32
        + inputs.ecology_pressure_q as i32
        + inputs.economy_pressure_q as i32
        + inputs.raid_echo_pressure_q as i32
        + inputs.erasure_pressure_q as i32
        + inputs.artifact_provenance_pressure_q as i32
        + inputs.reputation_delta_q as i32;
    base + inputs.chaos_perturb_q as i32
}

/// Compute fae layer spawn score from fae-specific inputs.
pub fn fae_spawn_score(inputs: &FaeLayerInputs) -> i32 {
    let shared = inputs.faction_pressure_q as i32
        + inputs.ecology_pressure_q as i32
        + inputs.economy_pressure_q as i32
        + inputs.artifact_provenance_pressure_q as i32;
    let fae = inputs.obligation_pressure_q as i32
        + inputs.fae_exploitation_q as i32
        + inputs.consent_integrity_q as i32
        + inputs.source_suffering_q as i32;
    let reduction = inputs.replacement_quality_q as i32 / 2;
    shared + fae - reduction + inputs.chaos_perturb_q as i32
}

/// Select spawn stage based on score and root state thresholds.
pub fn stage_select(score_q: i32, thresholds: &RootStateThresholds, current: SpawnStage) -> SpawnStage {
    if current.is_terminal() { return current; }
    if score_q >= thresholds.callable_q as i32 {
        SpawnStage::Callable
    } else if score_q >= thresholds.omen_q as i32 {
        SpawnStage::Omen
    } else if score_q >= thresholds.rumor_q as i32 {
        SpawnStage::Rumor
    } else {
        SpawnStage::Hidden
    }
}

/// Pick thresholds based on root health (0=healthy, 1=corrupted, 2+=void_leak).
pub fn thresholds_for_root_state(root_health: u8) -> RootStateThresholds {
    match root_health {
        0 => HEALTHY_ROOT,
        1 => CORRUPTED_ROOT,
        _ => VOID_LEAK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_root_stages() {
        assert_eq!(stage_select(3999, &HEALTHY_ROOT, SpawnStage::Hidden), SpawnStage::Hidden);
        assert_eq!(stage_select(4000, &HEALTHY_ROOT, SpawnStage::Hidden), SpawnStage::Rumor);
        assert_eq!(stage_select(5500, &HEALTHY_ROOT, SpawnStage::Hidden), SpawnStage::Omen);
        assert_eq!(stage_select(7000, &HEALTHY_ROOT, SpawnStage::Hidden), SpawnStage::Callable);
    }

    #[test]
    fn corrupted_root_lower_thresholds() {
        assert_eq!(stage_select(3000, &CORRUPTED_ROOT, SpawnStage::Hidden), SpawnStage::Rumor);
        assert_eq!(stage_select(2999, &CORRUPTED_ROOT, SpawnStage::Hidden), SpawnStage::Hidden);
    }

    #[test]
    fn terminal_state_sticky() {
        assert_eq!(stage_select(10000, &HEALTHY_ROOT, SpawnStage::Succeeded), SpawnStage::Succeeded);
        assert_eq!(stage_select(10000, &HEALTHY_ROOT, SpawnStage::Failed), SpawnStage::Failed);
    }

    #[test]
    fn world_boss_score_basic() {
        let inputs = WorldBossSpawnInputs {
            faction_pressure_q: 5000,
            ecology_pressure_q: 3000,
            chaos_perturb_q: -500,
            ..Default::default()
        };
        assert_eq!(world_boss_spawn_score(&inputs), 7500);
    }

    #[test]
    fn fae_score_replacement_reduces() {
        let base = FaeLayerInputs {
            obligation_pressure_q: 4000,
            fae_exploitation_q: 3000,
            consent_integrity_q: 2000,
            source_suffering_q: 1000,
            ..Default::default()
        };
        let score_no_replace = fae_spawn_score(&base);
        let with_replace = FaeLayerInputs { replacement_quality_q: 6000, ..base };
        assert_eq!(score_no_replace - fae_spawn_score(&with_replace), 3000);
    }
}
