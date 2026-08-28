//! Co-op one-shot policy and Crown touch policy.

// ── Co-op One-Shot Policy ────────────────────────────────────────────────────

/// Who owns the attempt in co-op.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AttemptOwner {
    /// Host's root cycle owns the attempt.
    HostRootCycle = 0,
}

/// What guests receive (never a true relic).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GuestReward {
    /// Echo proof of participation.
    EchoProof = 0,
    /// Cosmetic scar as a badge.
    CosmeticScar = 1,
    /// Faction memory or record.
    FactionMemory = 2,
    /// Recipe hint or knowledge.
    RecipeHint = 3,
    /// Public witness line or testament.
    PublicWitnessLine = 4,
}

/// How the host can distribute the true relic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RelicOwnerMode {
    /// Host claims the true relic.
    HostClaim = 0,
    /// Witness claims the true relic.
    WitnessClaim = 1,
    /// Relic is surrendered to the world.
    SurrenderedToWorld = 2,
    /// Relic is shattered into echoes for the party.
    ShatteredEchoesForParty = 3,
}

/// Co-op anti-exploit rules (all true by default).
#[derive(Clone, Copy, Debug)]
pub struct CoopAntiExploit {
    /// Whether guests cannot farm the true relic.
    pub guests_cannot_farm_true_relic: bool,
    /// Whether guest participation records the solution path.
    pub guest_participation_records_path: bool,
    /// Whether guest shadow can learn from their participation.
    pub guest_shadow_can_learn: bool,
    /// Whether host attempt is consumed on commit.
    pub host_attempt_consumed_on_commit: bool,
}

/// Default co-op anti-exploit rules with all protections enabled.
pub const COOP_ANTI_EXPLOIT: CoopAntiExploit = CoopAntiExploit {
    guests_cannot_farm_true_relic: true,
    guest_participation_records_path: true,
    guest_shadow_can_learn: true,
    host_attempt_consumed_on_commit: true,
};

// ── Crown Touch Policy ───────────────────────────────────────────────────────

/// Conditions required for Crown override.
#[derive(Clone, Copy, Debug)]
pub struct CrownOverrideConditions {
    /// Whether Outside the Wheel is exposed.
    pub outside_wheel_exposed: bool,
    /// Whether a crown fragment is currently active.
    pub crown_fragment_active: bool,
    /// Whether the ownership threshold has been met.
    pub ownership_threshold_met: bool,
    /// Whether the refusal threshold has been met.
    pub refusal_threshold_met: bool,
}

impl CrownOverrideConditions {
    /// Returns true if all conditions for a valid Crown override are met.
    pub const fn is_valid(&self) -> bool {
        self.outside_wheel_exposed
            && self.crown_fragment_active
            && (self.ownership_threshold_met || self.refusal_threshold_met)
    }
}

/// What Crown touch does to a relic or boss state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CrownTouchEffect {
    /// Temptation effect on the target.
    Temptation = 0,
    /// Ownership pressure on the target.
    OwnershipPressure = 1,
    /// Optimization pressure on the target.
    OptimizationPressure = 2,
    /// Shadow learning boost from the interaction.
    ShadowLearningBoost = 3,
    /// Shift in relic variant.
    RelicVariantShift = 4,
    /// Alteration of boss state.
    BossStateAlteration = 5,
}

// ── Failure & Prevention Rewards ─────────────────────────────────────────────

/// Failure output — world consequences from a failed attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FailureOutput {
    /// Faction hardens in response to failure.
    FactionHardening = 0,
    /// Vendor stock changes.
    AlternateVendorStock = 1,
    /// New rumor chain emerges.
    NewRumorChain = 2,
    /// Death scar variant appears.
    DeathScarVariant = 3,
    /// Corrupted echo relic spawns.
    CorruptedEchoRelic = 4,
    /// Route changes.
    ChangedRoute = 5,
    /// Spawn ecology alters.
    AlteredSpawnEcology = 6,
    /// Raid shortcut changes.
    ChangedRaidShortcut = 7,
    /// Shadow learning updates.
    ShadowLearningUpdate = 8,
    /// Crown temptation increases.
    CrownTemptationDelta = 9,
}

/// Prevention output — world consequences from preventing a boss spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PreventionOutput {
    /// Faction reforms positively.
    FactionReform = 0,
    /// No-shear route progress awarded.
    NoShearRouteProgress = 1,
    /// Public ledger proof recorded.
    PublicLedgerProof = 2,
    /// Unique recipe granted instead of relic.
    UniqueRecipeInsteadOfRelic = 3,
    /// A witness becomes an ally.
    WitnessAlly = 4,
    /// Crown hostility is reduced.
    CrownHostilityReduction = 5,
    /// Boss non-spawn record created.
    BossNonspawnRecord = 6,
}
