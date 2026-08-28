//! Shadow hooks — observation, mutation, and learning weights.

/// Shadow observation — types of player behavior observed by the shadow system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShadowObservation {
    /// Player's chosen solution path.
    SolutionPath = 0,
    /// Player's greed versus refusal behavior.
    GreedVsRefusal = 1,
    /// Player killing witnesses.
    WitnessKills = 2,
    /// Player claiming relics.
    RelicClaims = 3,
    /// Player repeating the same tactics.
    RepeatedTactics = 4,
    /// Player's failed one-shot attempts.
    FailedOneShotAttempts = 5,
    /// Player preventing bosses.
    PreventedBosses = 6,
    /// Player surrendering relics.
    SurrenderedRelics = 7,
    /// Player's bias toward crafted solutions.
    CraftedSolutionBias = 8,
    /// Player's hunting or fishing ethics.
    HuntingOrFishingEthics = 9,
}

/// Shadow mutation — types of world/behavior mutations triggered by shadow learning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShadowMutation {
    /// Mutation of future boss tactics in response to player behavior.
    FutureBossTactics = 0,
    /// Mutation of the crown raid model.
    CrownRaidModel = 1,
    /// Echo of death scars appearing in the world.
    DeathScarEcho = 2,
    /// Development of anti-player routes.
    AntiPlayerRoute = 3,
    /// Temptation offers tailored to player behavior.
    TemptationOffers = 4,
    /// World boss warning signs surfacing.
    WorldBossWarningSurfaces = 5,
}

/// Shadow learning weights — weight deltas for different player behaviors observed by the shadow system.
#[derive(Clone, Copy, Debug)]
pub struct ShadowLearningWeights {
    /// Weight for a normal combat kill.
    pub combat_kill: i16,
    /// Weight for overkill (excessive damage).
    pub overkill: i16,
    /// Weight for killing a witness.
    pub witness_killed: i16,
    /// Weight for claiming a true relic.
    pub true_relic_claimed: i16,
    /// Weight for surrendering a relic.
    pub relic_surrendered: i16,
    /// Weight for preventing a boss.
    pub boss_prevented: i16,
    /// Weight for clearing via refusal.
    pub refusal_clear: i16,
    /// Weight for a failed attempt.
    pub failed_attempt: i16,
    /// Weight for using the same solution repeatedly.
    pub same_solution_repeated: i16,
}

/// Default shadow learning weights for canonical playthrough behavior.
pub const DEFAULT_SHADOW_WEIGHTS: ShadowLearningWeights = ShadowLearningWeights {
    combat_kill: 1000,
    overkill: 1750,
    witness_killed: 2500,
    true_relic_claimed: 1500,
    relic_surrendered: -1000,
    boss_prevented: -1500,
    refusal_clear: -2000,
    failed_attempt: 1250,
    same_solution_repeated: 2000,
};
