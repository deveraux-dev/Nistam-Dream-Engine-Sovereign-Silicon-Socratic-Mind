//! Fae overlay types — 12+1 fae bosses, courts, moral bands, selection rules.

use crate::faction::FactionId;
use crate::solution_path::SolutionPathKind;

// ── Moral Band ───────────────────────────────────────────────────────────────

/// Fae moral band — the alignment or moral category of a fae being.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FaeMoralBand {
    /// Genuinely good fae.
    Good = 0,
    /// Good but inherently dangerous fae.
    GoodButDangerous = 1,
    /// Stern, strict good alignment.
    SternGood = 2,
    /// Mischievous but ultimately good.
    MischievousGood = 3,
    /// Purely mischievous, neutral alignment.
    Mischievous = 4,
    /// Mischievous with inclinations toward good.
    MischievousToGood = 5,
    /// Mischievous with inclinations toward evil.
    MischievousToEvil = 6,
    /// Mischievous with malevolent tendencies.
    MischievousToMalevolent = 7,
    /// Tragic or sad evil.
    TragicEvil = 8,
    /// Tragic, ranging from good to evil.
    TragicGoodToEvil = 9,
    /// Alien or incomprehensible mischievousness.
    AlienMischievous = 10,
    /// Stern good, approaching lich-grade power.
    SternGoodToLichGrade = 11,
    /// Good to lich-grade depending on pollution levels.
    GoodToLichGradeDependingOnPollution = 12,
    /// Mischievous to evil but capable of redemption.
    MischievousToEvilButRedeemable = 13,
    /// Good if refused by the player, evil if owned.
    GoodIfRefusedEvilIfOwned = 14,
}

// ── Voice Tags ───────────────────────────────────────────────────────────────

/// Fae voice tag — a tag describing the type of fae communication or character archetype.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FaeVoiceTag {
    /// Luring or enticing fae voice.
    Lure = 0,
    /// Warning or cautionary fae voice.
    Warning = 1,
    /// Grief or sorrow-stricken fae voice.
    Grief = 2,
    /// Bargaining fae voice.
    Bargain = 3,
    /// Guest-right or hospitality fae voice.
    GuestRight = 4,
    /// Glamorous or deceptive fae voice.
    Glamour = 5,
    /// Debt song or obligation fae voice.
    DebtSong = 6,
    /// Route song or path-related fae voice.
    RouteSong = 7,
}

// ── Fae Boss Definition ──────────────────────────────────────────────────────

/// Fae boss definition — static metadata for a fae boss.
#[derive(Clone, Debug)]
pub struct FaeBossDef {
    /// Unique identifier for this fae boss.
    pub id: &'static str,
    /// Faction this fae is associated with.
    pub faction_id: FactionId,
    /// Display name for this fae boss.
    pub display_name: &'static str,
    /// Fae court or realm affiliation.
    pub court: &'static str,
    /// Type or species of fae.
    pub fae_type: &'static str,
    /// Moral alignment band for this fae.
    pub moral_band: FaeMoralBand,
    /// Zones where this fae has affinity.
    pub zone_bias: &'static [&'static str],
    /// Folklore or lore hint about this fae.
    pub folklore_hint: &'static str,
    /// Quest identifier associated with this fae.
    pub quest_id: &'static str,
    /// Solution paths available to interact with this fae.
    pub solution_paths: &'static [SolutionPathKind],
    /// Reward identifier for defeating or resolving this fae.
    pub reward_id: &'static str,
}

// ── Mutual Exclusion ─────────────────────────────────────────────────────────

/// Mutual exclusion group — fae that cannot all be present in the same playthrough.
#[derive(Clone, Debug)]
pub struct MutualExclusionGroup {
    /// Name or label for this exclusion group.
    pub group_name: &'static str,
    /// Fae IDs that belong to this mutual exclusion group.
    pub members: &'static [&'static str],
    /// Maximum number of members from this group allowed per playthrough.
    pub max_per_playthrough: u8,
}

/// Static array of mutual exclusion groups defining which fae cannot coexist in a playthrough.
pub const MUTUAL_EXCLUSIONS: &[MutualExclusionGroup] = &[
    MutualExclusionGroup {
        group_name: "water_fae",
        members: &["the_pearl_masked_selkie", "the_siren_who_forgot_hunger", "the_baptismal_hag"],
        max_per_playthrough: 1,
    },
    MutualExclusionGroup {
        group_name: "route_fae",
        members: &["the_walking_milestone", "the_hare_with_twelve_shadows", "the_cartwheel_king"],
        max_per_playthrough: 1,
    },
    MutualExclusionGroup {
        group_name: "grave_refusal_fae",
        members: &["the_mourning_briar", "the_child_who_unwove_crowns", "the_baptismal_hag"],
        max_per_playthrough: 2,
    },
    MutualExclusionGroup {
        group_name: "industrial_fire_fae",
        members: &["the_shift_whistle_dryad", "the_hearth_that_marched"],
        max_per_playthrough: 1,
    },
];

// ── Selection Weights ────────────────────────────────────────────────────────

/// Permyriad weight bonuses for fae selection based on player behavior.
#[derive(Clone, Copy, Debug, Default)]
pub struct FaeSelectionWeights {
    /// Bonus (permyriad) for high faction pressure — weight toward faction-aligned fae.
    pub high_faction_pressure_pmy: u16,       // +3000 (30%)
    /// Bonus (permyriad) for high ecology pressure — weight toward nature-aligned fae.
    pub high_ecology_pressure_pmy: u16,       // +2500 (25%)
    /// Bonus (permyriad) for overuse of trade profits — weight toward bargain-fae.
    pub overuses_trade_profit_pmy: u16,       // +2000 (20%) bargain-fae
    /// Bonus (permyriad) for overhunting — weight toward hunt-fae.
    pub overhunts_pmy: u16,                   // +3500 (35%) hunt-fae
    /// Bonus (permyriad) for overfishing — weight toward tide-fae.
    pub overfishes_pmy: u16,                  // +3500 (35%) tide-fae
    /// Bonus (permyriad) for using refusal — weight toward secret benign fae.
    pub uses_refusal_pmy: u16,               // +2000 (20%) secret benign
    /// Bonus (permyriad) for claiming many relics — weight toward hostile secret fae.
    pub claims_many_relics_pmy: u16,         // +3000 (30%) hostile secret
    /// Bonus (permyriad) for active void leak — weight toward glamour/code fae.
    pub void_leak_active_pmy: u16,           // +2500 (25%) glamour/code
}

/// Default fae selection weights for canonical playthrough behavior.
pub const DEFAULT_FAE_WEIGHTS: FaeSelectionWeights = FaeSelectionWeights {
    high_faction_pressure_pmy: 3000,
    high_ecology_pressure_pmy: 2500,
    overuses_trade_profit_pmy: 2000,
    overhunts_pmy: 3500,
    overfishes_pmy: 3500,
    uses_refusal_pmy: 2000,
    claims_many_relics_pmy: 3000,
    void_leak_active_pmy: 2500,
};
