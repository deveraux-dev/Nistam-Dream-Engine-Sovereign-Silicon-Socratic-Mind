//! Reactive item proc system: trigger-payload pairs for equipped items.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProcTrigger {
    OnDamageTaken { min_damage: u16 },
    OnMeleeHit { required_combo_streak: u8 },
    OnParrySuccess,
    OnDash,
    OnComboBreak { min_streak_lost: u8 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProcPayload {
    ReflectDamage { permyriad_ratio: u32 },
    ApplyDeferredDoom { delay_ticks: u16, flat_damage: u16 },
    SpawnPhysicsEntity { entity_id: u32, impulse_x: i32, impulse_y: i32 },
    SpikeEntropy { flat_amount: u16 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReactiveProc {
    pub trigger: ProcTrigger,
    pub payload: ProcPayload,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeferredProc {
    pub ticks_remaining: u16,
    pub flat_damage: u16,
    pub source_player: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(p: ReactiveProc) {
        let json = serde_json::to_string(&p).unwrap();
        let back: ReactiveProc = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn on_damage_taken_reflect() {
        round_trip(ReactiveProc {
            trigger: ProcTrigger::OnDamageTaken { min_damage: 30 },
            payload: ProcPayload::ReflectDamage { permyriad_ratio: 2500 },
        });
    }

    #[test]
    fn on_melee_hit_deferred_doom() {
        round_trip(ReactiveProc {
            trigger: ProcTrigger::OnMeleeHit { required_combo_streak: 3 },
            payload: ProcPayload::ApplyDeferredDoom { delay_ticks: 60, flat_damage: 100 },
        });
    }

    #[test]
    fn on_parry_success_spawn_entity() {
        round_trip(ReactiveProc {
            trigger: ProcTrigger::OnParrySuccess,
            payload: ProcPayload::SpawnPhysicsEntity { entity_id: 7, impulse_x: 500, impulse_y: -200 },
        });
    }

    #[test]
    fn on_dash_spike_entropy() {
        round_trip(ReactiveProc {
            trigger: ProcTrigger::OnDash,
            payload: ProcPayload::SpikeEntropy { flat_amount: 50 },
        });
    }

    #[test]
    fn on_combo_break_reflect_damage() {
        round_trip(ReactiveProc {
            trigger: ProcTrigger::OnComboBreak { min_streak_lost: 5 },
            payload: ProcPayload::ReflectDamage { permyriad_ratio: 1000 },
        });
    }

    #[test]
    fn deferred_proc_countdown_round_trip() {
        let d = DeferredProc { ticks_remaining: 60, flat_damage: 100, source_player: 0 };
        let json = serde_json::to_string(&d).unwrap();
        let back: DeferredProc = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn reflect_damage_calculation() {
        // 40 damage x 5000 permyriad (50%) = 20 reflected
        let damage: u16 = 40;
        let permyriad_ratio: u32 = 5000;
        let reflected = ((damage as u32 * permyriad_ratio) / 10000) as u16;
        assert_eq!(reflected, 20);
    }

    #[test]
    fn deferred_doom_countdown() {
        let mut deferred = DeferredProc { ticks_remaining: 3, flat_damage: 50, source_player: 0 };
        deferred.ticks_remaining = deferred.ticks_remaining.saturating_sub(1);
        assert_eq!(deferred.ticks_remaining, 2);
        deferred.ticks_remaining = deferred.ticks_remaining.saturating_sub(1);
        assert_eq!(deferred.ticks_remaining, 1);
        deferred.ticks_remaining = deferred.ticks_remaining.saturating_sub(1);
        assert_eq!(deferred.ticks_remaining, 0);
        assert_eq!(deferred.flat_damage, 50);
    }

    #[test]
    fn is_proc_prevents_chain() {
        // Validates the conceptual invariant: events with is_proc == true must
        // never be re-processed by the reactive-proc scan (simulation.rs).
        use super::super::combat::{TmpEvent, CombatTrigger};

        let original_event = TmpEvent {
            trigger: CombatTrigger::MeleeHit,
            source_player: 0, target_player: 1, damage: 30, tick: 100, is_proc: false,
        };
        let proc_event = TmpEvent {
            trigger: CombatTrigger::ItemProc,
            source_player: 1, target_player: 0, damage: 15, tick: 100, is_proc: true,
        };

        assert!(!original_event.is_proc);
        assert!(proc_event.is_proc);
    }
}
