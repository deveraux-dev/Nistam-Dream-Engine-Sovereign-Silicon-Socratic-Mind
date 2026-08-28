//! The Shadow Counterpart / Jungian Nemesis system — authored against two
//! companion design docs:
//! `F:\13forge-super\_merged\reposold\_attic\2026-06-07\ironroot-edict-condense\
//! MVP\docs\design-bible\001-the-shadow-nemesis.md` (the LOCKED design
//! bible, rationale + the concrete promotion/grudge numbers) and
//! `F:\13forge-super\_merged\reposold\ironroot-edict\IRONROOT_Design_Packet\
//! IRONROOT_Rust_Markdown_Specs\08_shadow_counterpart_system.md` (the
//! Rust-shaped spec — struct/fn signatures, verbatim below).
//!
//! **Distinct from [`crate::haunt::ShadowMemory`].** That's an already-landed,
//! separate system (scar-count awareness ladder, drained from
//! `forge-game-systems::sim_harness.rs`). This is a different design: not a
//! replay of past deaths, but a live counter-build AI reading the player's
//! *pattern* (`001-the-shadow-nemesis.md:10`: "The Shadow does not only copy
//! how the player fights. It learns what the player keeps becoming.") The
//! spec's own top struct is also named `ShadowMemory` — renamed here to
//! [`CounterpartMemory`] specifically to avoid colliding with the unrelated,
//! already-tested `haunt::ShadowMemory`.
//!
//! **Already half-landed, unknowingly.** `crate::combat::PatternMap`
//! (`direction_freq: [u16; 8]`, `aspect_freq: [u16; 8]`, `total_observations`,
//! degrades at 60k) is this exact system's counter-build tracker
//! (`001-the-shadow-nemesis.md:12-18`: "PatternMap tracks your 8 attack
//! directions + 8 ability aspects... dominant_direction() reveals your most
//! predictable combo — Shadow blocks it"), landed already but never wired to
//! anything Shadow-shaped until now.
//!
//! **Scope cut, named plainly (not invented):** `08_shadow_counterpart_
//! system.md`'s `ShadowMemory.preferred_plane: Option<CombatPlane>` and
//! `ending_pressure: EndingPressure` (with a `.total()` method) reference
//! types neither design doc ever defines — no field/variant list for either
//! anywhere in the source material. Both are cut from [`CounterpartMemory`]
//! rather than guessed at. Same for `ShadowFrame`/`ShadowFile`/
//! `CounterpartProfile`/`checksum_shadow` — fully spec'd in shape, but their
//! fields lean on `CombatAspect`, `Vec2i`, `Zodiac`, `AlchemicalTier`,
//! `EntityId`, `PlayerChoiceEvent`, `EndingVector`, none of which exist in
//! this crate yet. Real, cited, unported — not this module's job to fake
//! into existence.

/// The three Shadow manifestation forms, verbatim
/// (`08_shadow_counterpart_system.md:17-21`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowForm {
    /// Reads repeated attacks/dodges/route habits; punishes habit.
    Stalker,
    /// Reads dominant build behavior and gear reliance; punishes optimization.
    Blighted,
    /// Reads ending philosophy and grief-conversion pattern; punishes identity.
    Harbinger,
}

impl ShadowForm {
    /// The form's own name, matching the design bible's promotion table
    /// (`001-the-shadow-nemesis.md:29-36`).
    pub const fn name(self) -> &'static str {
        match self {
            ShadowForm::Stalker => "Stalker",
            ShadowForm::Blighted => "Blighted",
            ShadowForm::Harbinger => "Harbinger",
        }
    }

    /// What this form punishes, per the design bible's own table
    /// (`001-the-shadow-nemesis.md:26-30`).
    pub const fn punishes(self) -> &'static str {
        match self {
            ShadowForm::Stalker => "habit",
            ShadowForm::Blighted => "optimization",
            ShadowForm::Harbinger => "identity",
        }
    }
}

/// The Shadow's memory of the player — the subset of
/// `08_shadow_counterpart_system.md:53-62`'s `ShadowMemory` whose fields
/// resolve to real, already-existing types (see module doc for what's cut
/// and why).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CounterpartMemory {
    /// How many times the player has repeated the same attack.
    pub repeated_attack_count: u16,
    /// How many finishing executions the player has landed.
    pub execution_count: u16,
    /// How many times the player refused an offered execution.
    pub refused_execution_count: u16,
    /// Hash of the player's most-used item — nonzero once a habit forms.
    pub most_used_item_hash: u64,
    /// The player's dominant combat resonance, Hz — the spec's own field
    /// type (`i16`), kept verbatim rather than widened to match
    /// `combat::CombatState::resonance_hz`'s `u16`.
    pub dominant_resonance_hz: i16,
    /// A hash of the player's most-traveled route.
    pub route_hash: u64,
    /// Grudge, permyriad (0..=10_000) — builds on both wins and losses
    /// (`001-the-shadow-nemesis.md:20-27`).
    pub grudge_q: i32,
}

/// Grudge gained when the Shadow kills the player, verbatim
/// (`001-the-shadow-nemesis.md:23`).
pub const GRUDGE_ON_ENTITY_WIN: i32 = 1_000;
/// Grudge gained when the player kills the Shadow, verbatim
/// (`001-the-shadow-nemesis.md:24`).
pub const GRUDGE_ON_PLAYER_WIN: i32 = 500;
/// Grudge permyriad ceiling, verbatim (`001-the-shadow-nemesis.md:22`).
pub const GRUDGE_MAX: i32 = 10_000;

/// Encounter counts at which the Shadow promotes, verbatim
/// (`001-the-shadow-nemesis.md:37`: "Promotion thresholds: [1, 3, 5, 8, 12]
/// encounters").
pub const PROMOTION_THRESHOLDS: [u16; 5] = [1, 3, 5, 8, 12];

impl CounterpartMemory {
    /// A fresh, unmet Shadow.
    pub fn new() -> Self {
        Self::default()
    }

    /// The Shadow kills the player: `+1000` grudge, clamped at
    /// [`GRUDGE_MAX`].
    pub fn record_entity_win(&mut self) {
        self.grudge_q = (self.grudge_q + GRUDGE_ON_ENTITY_WIN).min(GRUDGE_MAX);
    }

    /// The player kills the Shadow: `+500` grudge, clamped at [`GRUDGE_MAX`].
    pub fn record_player_win(&mut self) {
        self.grudge_q = (self.grudge_q + GRUDGE_ON_PLAYER_WIN).min(GRUDGE_MAX);
    }

    /// Grudge decays over time out of combat (`001-the-shadow-nemesis.md:25`
    /// states the rule but names no rate — the caller supplies `amount` per
    /// tick/step rather than this module inventing an unspecified constant).
    pub fn decay_grudge(&mut self, amount: i32) {
        self.grudge_q = (self.grudge_q - amount).max(0);
    }

    /// Promotion tier index (0..=5) — how many of [`PROMOTION_THRESHOLDS`]
    /// the encounter count has cleared.
    pub fn promotion_tier(&self, encounters: u16) -> usize {
        PROMOTION_THRESHOLDS.iter().filter(|&&t| encounters >= t).count()
    }
}

/// Classify the Shadow's current form from its memory, verbatim
/// (`08_shadow_counterpart_system.md:107-115`) minus the `ending_pressure`
/// branch (module doc explains the cut — `EndingPressure` is never defined
/// in the source material).
pub fn classify_shadow(memory: &CounterpartMemory) -> ShadowForm {
    if memory.execution_count > 30 {
        ShadowForm::Harbinger
    } else if memory.repeated_attack_count > 80 || memory.most_used_item_hash != 0 {
        ShadowForm::Blighted
    } else {
        ShadowForm::Stalker
    }
}

/// Whether the Shadow can mirror the player at all, verbatim
/// (`08_shadow_counterpart_system.md:153-155`).
pub fn shadow_can_mirror(memory: &CounterpartMemory) -> bool {
    memory.refused_execution_count < 3
}

/// One row of the design bible's Behavioral Evolution table, verbatim
/// (`001-the-shadow-nemesis.md`'s counterpart, `08_shadow_counterpart_
/// system.md:139-148`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehavioralEvolution {
    /// The player habit the Shadow reads.
    pub player_habit: &'static str,
    /// How the Shadow answers it.
    pub shadow_response: &'static str,
}

/// The six cited behavioral-evolution rows, verbatim
/// (`08_shadow_counterpart_system.md:141-148`).
pub fn behavioral_evolution_table() -> [BehavioralEvolution; 6] {
    [
        BehavioralEvolution { player_habit: "repeated parries", shadow_response: "feints and delayed strikes" },
        BehavioralEvolution { player_habit: "repeated fire builds", shadow_response: "Albedo counters" },
        BehavioralEvolution { player_habit: "repeated execution prompts", shadow_response: "lethal execution mirror" },
        BehavioralEvolution { player_habit: "repeated refusal", shadow_response: "hesitation and incomplete mirroring" },
        BehavioralEvolution { player_habit: "overused route", shadow_response: "ambushes on that route" },
        BehavioralEvolution { player_habit: "speedrun behavior", shadow_response: "route-blocking counterpart" },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_shadow_is_a_stalker() {
        let memory = CounterpartMemory::new();
        assert_eq!(classify_shadow(&memory), ShadowForm::Stalker);
    }

    #[test]
    fn high_execution_count_promotes_to_harbinger() {
        let memory = CounterpartMemory { execution_count: 31, ..Default::default() };
        assert_eq!(classify_shadow(&memory), ShadowForm::Harbinger);
    }

    #[test]
    fn repeated_attacks_or_gear_reliance_promotes_to_blighted() {
        let by_repetition = CounterpartMemory { repeated_attack_count: 81, ..Default::default() };
        assert_eq!(classify_shadow(&by_repetition), ShadowForm::Blighted);

        let by_gear = CounterpartMemory { most_used_item_hash: 0xC0FFEE, ..Default::default() };
        assert_eq!(classify_shadow(&by_gear), ShadowForm::Blighted);
    }

    #[test]
    fn harbinger_outranks_blighted_when_both_conditions_hold() {
        let memory = CounterpartMemory { execution_count: 40, repeated_attack_count: 200, ..Default::default() };
        assert_eq!(classify_shadow(&memory), ShadowForm::Harbinger);
    }

    #[test]
    fn grudge_accrues_and_clamps_at_the_ceiling() {
        let mut memory = CounterpartMemory::new();
        memory.record_entity_win();
        assert_eq!(memory.grudge_q, 1_000);
        memory.record_player_win();
        assert_eq!(memory.grudge_q, 1_500);
        for _ in 0..20 {
            memory.record_entity_win();
        }
        assert_eq!(memory.grudge_q, GRUDGE_MAX, "grudge must clamp at 10000");
    }

    #[test]
    fn grudge_decays_and_floors_at_zero() {
        let mut memory = CounterpartMemory { grudge_q: 500, ..Default::default() };
        memory.decay_grudge(300);
        assert_eq!(memory.grudge_q, 200);
        memory.decay_grudge(9_999);
        assert_eq!(memory.grudge_q, 0, "grudge must floor at 0, never negative");
    }

    #[test]
    fn promotion_tier_climbs_the_cited_ladder() {
        let memory = CounterpartMemory::new();
        assert_eq!(memory.promotion_tier(0), 0);
        assert_eq!(memory.promotion_tier(1), 1);
        assert_eq!(memory.promotion_tier(2), 1);
        assert_eq!(memory.promotion_tier(3), 2);
        assert_eq!(memory.promotion_tier(5), 3);
        assert_eq!(memory.promotion_tier(8), 4);
        assert_eq!(memory.promotion_tier(12), 5);
        assert_eq!(memory.promotion_tier(999), 5, "the ladder tops out at 5");
    }

    #[test]
    fn three_or_more_refusals_stop_mirroring() {
        let mut memory = CounterpartMemory::new();
        assert!(shadow_can_mirror(&memory));
        memory.refused_execution_count = 3;
        assert!(!shadow_can_mirror(&memory));
    }

    #[test]
    fn behavioral_table_has_the_cited_six_rows_all_populated() {
        let rows = behavioral_evolution_table();
        assert_eq!(rows.len(), 6);
        for r in rows {
            assert!(!r.player_habit.is_empty());
            assert!(!r.shadow_response.is_empty());
        }
    }

    #[test]
    fn shadow_form_names_and_punishes_match_the_design_bible_table() {
        assert_eq!(ShadowForm::Stalker.name(), "Stalker");
        assert_eq!(ShadowForm::Stalker.punishes(), "habit");
        assert_eq!(ShadowForm::Blighted.name(), "Blighted");
        assert_eq!(ShadowForm::Blighted.punishes(), "optimization");
        assert_eq!(ShadowForm::Harbinger.name(), "Harbinger");
        assert_eq!(ShadowForm::Harbinger.punishes(), "identity");
    }
}
