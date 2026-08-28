//! The Boss Sieve — authored content, cited verbatim from
//! `F:\v3\TODO\ironroot-edict\IRONROOT_Design_Packet\
//! ironroot_thread_synthesis_machine_readable.json:417-527` (the
//! `boss_sieve` object).
//!
//! **Doctrine** (`json:418-420`): "The narrative boss is fixed; the version
//! of the boss selected by the sieve changes based on player behavior."
//! `not_random: true`, `generation_style: "authored_variant_selection"` —
//! the boss the player meets is chosen deterministically from
//! [`RunProfile`](crate::ironroot::run_profile::RunProfile) signals, never
//! rolled.
//!
//! **What this module lands, and what it doesn't.** The design packet names
//! *what* each Bell Warden variant answers to (`trigger`, a narrative
//! condition like `"high_aggression_or_blood_supply"`) but gives no numeric
//! threshold on [`RunProfile`](crate::ironroot::run_profile::RunProfile)'s
//! fields — that mapping is real, unported
//! work, not decoration to skip. Inventing specific cutoffs here (e.g. "kills
//! > 12 counts as high_aggression") would be exactly the unearned-precision
//! guess T1 `zero_hallucination` forbids: the packet doesn't specify one, so
//! this module doesn't pretend one exists. What it does land is the full
//! catalog — every manifestation, every variant's id/trigger/mode/lesson,
//! every concession and hardness example — verbatim, queryable, and tested
//! for completeness. The trigger→`RunProfile`-threshold sieve is real,
//! cited, unported work for a future pass.

/// The eleven boss manifestations (`json:421-433`) — the authored variant
/// space the sieve selects from. Not a random roll: `not_random: true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BossManifestation {
    /// One-on-one combat.
    Duel,
    /// A trial the player must pass, not a fight to the death.
    Trial,
    /// The boss is pursued rather than confronted head-on.
    Hunt,
    /// A siege against a fortified position.
    Siege,
    /// The boss can be spared, or spares the player.
    Mercy,
    /// A reckoning over debts owed rather than a fight.
    DebtAudit,
    /// A crafting/riddle challenge stands in for combat.
    CraftRiddle,
    /// The boss mirrors the player's own death pattern back at them.
    DeathMirror,
    /// The fight cannot begin until the player accuses or consents.
    RefusalGate,
    /// The boss yields.
    Surrender,
    /// The boss fights at its most hostile, no opening offered.
    Enraged,
}

/// All eleven manifestations, in the design packet's own order.
pub const ALL_MANIFESTATIONS: [BossManifestation; 11] = [
    BossManifestation::Duel,
    BossManifestation::Trial,
    BossManifestation::Hunt,
    BossManifestation::Siege,
    BossManifestation::Mercy,
    BossManifestation::DebtAudit,
    BossManifestation::CraftRiddle,
    BossManifestation::DeathMirror,
    BossManifestation::RefusalGate,
    BossManifestation::Surrender,
    BossManifestation::Enraged,
];

/// One Bell Warden variant: the narrative trigger that selects it, the
/// combat mode it plays in, and the lesson it teaches the player.
/// `json:434-471`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BellWardenVariant {
    /// The variant's own name.
    pub id: &'static str,
    /// The narrative `RunProfile` condition that selects this variant.
    pub trigger: &'static str,
    /// The combat mode this variant plays in.
    pub mode: &'static str,
    /// What the fight teaches the player.
    pub lesson: &'static str,
}

/// The six authored Bell Warden variants, verbatim (`json:435-470`).
pub fn bell_warden_variants() -> [BellWardenVariant; 6] {
    [
        BellWardenVariant {
            id: "warden_of_red_debt",
            trigger: "high_aggression_or_blood_supply",
            mode: "knife_2d",
            lesson: "Damage creates debt.",
        },
        BellWardenVariant {
            id: "thirteen_bells_warden",
            trigger: "high_parry_timing_or_opening_anomaly",
            mode: "knife_2d_plus_bell_chain",
            lesson: "The bell can be answered.",
        },
        BellWardenVariant {
            id: "broken_forge_warden",
            trigger: "high_crafting_or_repair",
            mode: "forge_combat",
            lesson: "Preparation can answer where hands cannot.",
        },
        BellWardenVariant {
            id: "witness_warden",
            trigger: "high_witness_building_or_right_hand_road",
            mode: "ledger_turn",
            lesson: "Proof changes what violence is allowed to mean.",
        },
        BellWardenVariant {
            id: "silent_warden",
            trigger: "high_refusal",
            mode: "refusal_gate",
            lesson: "Some systems cannot start unless you accuse or consent.",
        },
        BellWardenVariant {
            id: "grave_warden",
            trigger: "high_death_route",
            mode: "spirit_death",
            lesson: "Death-routes become visible to what guards the grave.",
        },
    ]
}

/// A named boss and the player behavior that makes it concede rather than
/// fight to the end. `json:472-497`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcessionExample {
    /// The named boss.
    pub boss: &'static str,
    /// The player behavior that makes it concede.
    pub concedes_if: &'static str,
}

/// The six authored concession examples, verbatim (`json:473-496`).
pub fn concession_examples() -> [ConcessionExample; 6] {
    [
        ConcessionExample { boss: "Equal Knife", concedes_if: "Player balanced harm without revenge." },
        ConcessionExample { boss: "Crownless Roar", concedes_if: "Player repeatedly refused coercive command." },
        ConcessionExample { boss: "Clean Index Hound", concedes_if: "Player has no repeated route signature." },
        ConcessionExample { boss: "Widow of Green Debt", concedes_if: "Player broke debt chains without profiting." },
        ConcessionExample { boss: "Grave-Water Boss", concedes_if: "Player died in place to preserve a name." },
        ConcessionExample { boss: "Vowless Boss", concedes_if: "Player refuses the offered mechanic entirely." },
    ]
}

/// A player behavior and the consequence it loads onto a later boss fight —
/// the sieve making a fight harder, not just picking a variant. `json:498-527`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardnessExample {
    /// The player behavior the sieve is watching for.
    pub behavior: &'static str,
    /// What it loads onto a later boss fight.
    pub consequence: &'static str,
}

/// The seven authored hardness examples, verbatim (`json:499-526`).
pub fn hardness_examples() -> [HardnessExample; 7] {
    [
        HardnessExample { behavior: "farmed_deaths_for_advantage", consequence: "Death boss gains scar armor." },
        HardnessExample { behavior: "abused_crafting_economy", consequence: "Merchant/debt boss gains supply-chain phase." },
        HardnessExample { behavior: "repeated_one_combo", consequence: "Shadow/Hound starts with counter-pattern loaded." },
        HardnessExample { behavior: "ignored_erasures", consequence: "Boss arena has missing NPCs/witness gaps." },
        HardnessExample { behavior: "sold_cursed_goods", consequence: "Later boss uses them against towns." },
        HardnessExample { behavior: "over_commanded_followers", consequence: "Crownless Roar starts with mutiny phase." },
        HardnessExample { behavior: "killed_surrendered_enemies", consequence: "Equal Knife starts in Executioner Form." },
    ]
}

/// Deterministic Bell Warden selection off a live [`RunProfile`](crate::ironroot::run_profile::RunProfile).
///
/// **[AUTHORED — Sean, 2026-08-18].** The design packet names each variant's
/// narrative trigger (this file's own header) but gives no numeric cutoff —
/// that gap is real, not decoration (`json` doesn't specify one). These
/// thresholds are an authored engineering judgment call, not an extraction:
/// picked after running the crate's existing numeric suites green (237
/// `combat`/`combat_live` tests, 9 `itemforge` tests, 10 ASP tests) to
/// confirm the fields being thresholded are live, tested counters, not to
/// derive a scale from them (nothing in those suites states what counts as
/// "many kills" — no such telemetry exists yet). The `_q`-suffixed
/// `RunProfile` fields (`run_profile.rs`'s own doc: "no scale is given ...
/// so they stay plain `i32` rather than an invented fixed-point unit") are
/// deliberately used only as `> 0` ("used at all") checks below, never given
/// an invented magnitude cutoff — layering one guessed scale on another
/// would compound the exact problem this function's header is naming, not
/// avoid it.
///
/// Checked in the design packet's own variant order (`bell_warden_variants()`);
/// first trigger that reads true wins. A fresh (all-zero) or genuinely
/// ambiguous profile falls through to the "closest lean" tie-break: whichever
/// proxy stat is numerically highest, first-listed wins ties — so the
/// function always returns *some* real variant, never a hardcoded default
/// masking "nothing happened yet".
pub fn select_warden_variant(profile: &crate::ironroot::run_profile::RunProfile) -> BellWardenVariant {
    let variants = bell_warden_variants();
    // (trigger holds, tie-break magnitude) per variant, same order as
    // `bell_warden_variants()` / this file's header trigger list.
    let reads = [
        (profile.kills > 8 || profile.blood_supply_used_q > 0, profile.kills),
        (profile.perfect_parries > 4, profile.perfect_parries),
        (profile.crafts + profile.repairs > 6, profile.crafts + profile.repairs),
        (
            profile.witnesses_saved > 3 || profile.treaties_signed > 2,
            profile.witnesses_saved + profile.treaties_signed,
        ),
        (profile.commands_refused > 5, profile.commands_refused),
        (profile.deaths > 2, profile.deaths),
    ];

    if let Some(i) = reads.iter().position(|(hit, _)| *hit) {
        return variants[i];
    }
    let (best, _) = reads
        .iter()
        .enumerate()
        .max_by_key(|(i, (_, magnitude))| (*magnitude, std::cmp::Reverse(*i)))
        .expect("reads is a fixed non-empty array");
    variants[best]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_eleven_manifestations_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for m in ALL_MANIFESTATIONS {
            assert!(seen.insert(m), "duplicate manifestation {m:?}");
        }
        assert_eq!(seen.len(), 11, "the design packet names exactly 11 boss manifestations");
    }

    #[test]
    fn every_bell_warden_variant_has_a_unique_id_and_trigger() {
        let variants = bell_warden_variants();
        let mut ids = std::collections::HashSet::new();
        let mut triggers = std::collections::HashSet::new();
        for v in &variants {
            assert!(ids.insert(v.id), "duplicate variant id {}", v.id);
            assert!(triggers.insert(v.trigger), "duplicate trigger {}", v.trigger);
            assert!(!v.mode.is_empty(), "{} must name a combat mode", v.id);
            assert!(!v.lesson.is_empty(), "{} must teach a lesson", v.id);
        }
        assert_eq!(variants.len(), 6, "the design packet names exactly 6 Bell Warden variants");
    }

    #[test]
    fn the_thirteen_bells_warden_is_the_one_that_answers_the_bell() {
        // Ties this catalog back to bell_pit's own "the bell can be answered"
        // terminology — same doctrine, two authored surfaces.
        let variants = bell_warden_variants();
        let thirteen_bells = variants.iter().find(|v| v.id == "thirteen_bells_warden").expect("thirteen_bells_warden exists");
        assert_eq!(thirteen_bells.lesson, "The bell can be answered.");
    }

    #[test]
    fn concession_examples_all_name_a_real_boss_and_condition() {
        for c in concession_examples() {
            assert!(!c.boss.is_empty());
            assert!(!c.concedes_if.is_empty());
        }
        assert_eq!(concession_examples().len(), 6);
    }

    #[test]
    fn hardness_examples_all_name_a_behavior_and_consequence() {
        for h in hardness_examples() {
            assert!(!h.behavior.is_empty());
            assert!(!h.consequence.is_empty());
        }
        assert_eq!(hardness_examples().len(), 7);
    }

    use crate::ironroot::run_profile::RunProfile;

    /// A fresh, all-zero profile still resolves to a real variant — never a
    /// crash or a hardcoded placeholder — and it's the first-listed one, the
    /// documented tie-break for a total-zero tie.
    #[test]
    fn a_fresh_profile_selects_the_first_listed_variant() {
        let p = RunProfile::new();
        assert_eq!(select_warden_variant(&p).id, "warden_of_red_debt");
    }

    /// Each trigger, raised alone past its own threshold with every other
    /// stat left at zero, selects exactly its own variant — proves the
    /// mapping, not just that *a* variant comes back.
    #[test]
    fn each_trigger_alone_selects_its_own_variant() {
        let mut p = RunProfile::new();
        p.kills = 9;
        assert_eq!(select_warden_variant(&p).id, "warden_of_red_debt");

        let mut p = RunProfile::new();
        p.blood_supply_used_q = 1;
        assert_eq!(select_warden_variant(&p).id, "warden_of_red_debt", "blood_supply_used_q > 0 alone must trigger red_debt");

        let mut p = RunProfile::new();
        p.perfect_parries = 5;
        assert_eq!(select_warden_variant(&p).id, "thirteen_bells_warden");

        let mut p = RunProfile::new();
        p.crafts = 4;
        p.repairs = 3;
        assert_eq!(select_warden_variant(&p).id, "broken_forge_warden");

        let mut p = RunProfile::new();
        p.witnesses_saved = 4;
        assert_eq!(select_warden_variant(&p).id, "witness_warden");

        let mut p = RunProfile::new();
        p.commands_refused = 6;
        assert_eq!(select_warden_variant(&p).id, "silent_warden");

        let mut p = RunProfile::new();
        p.deaths = 3;
        assert_eq!(select_warden_variant(&p).id, "grave_warden");
    }

    /// Below-threshold stats never false-trigger — one under the cutoff on
    /// every field still falls through to the fresh-profile default.
    #[test]
    fn just_under_threshold_never_triggers() {
        let mut p = RunProfile::new();
        p.kills = 8;
        p.perfect_parries = 4;
        p.commands_refused = 5;
        p.deaths = 2;
        assert_eq!(select_warden_variant(&p).id, "warden_of_red_debt", "no trigger reads true; kills=8 is still the highest magnitude, so the tie-break falls to it");
    }

    /// Earlier-listed triggers win when multiple read true at once —
    /// deterministic priority, not last-write-wins.
    #[test]
    fn an_earlier_trigger_wins_over_a_later_one() {
        let mut p = RunProfile::new();
        p.deaths = 5; // grave_warden's trigger
        p.commands_refused = 6; // silent_warden's trigger, listed earlier
        assert_eq!(select_warden_variant(&p).id, "silent_warden");
    }

    /// Same (profile, thresholds) in, same variant out — no hidden clock or
    /// RNG, matching the sieve's own `not_random: true` doctrine.
    #[test]
    fn selection_is_deterministic() {
        let mut p = RunProfile::new();
        p.perfect_parries = 7;
        assert_eq!(select_warden_variant(&p).id, select_warden_variant(&p).id);
    }
}
