//! Solution path kinds — the 17 canonical ways to resolve any interaction.

/// One of the 17 canonical ways a player can resolve any interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SolutionPathKind {
    /// Direct confrontation.
    Combat = 0,
    /// Making or building something.
    Crafting = 1,
    /// Pursuing and taking down prey.
    Hunting = 2,
    /// Working a body of water for catch.
    Fishing = 3,
    /// Performance or sound-based resolution.
    Music = 4,
    /// Enduring hostile conditions.
    Survival = 5,
    /// Extracting information from a source.
    DataMining = 6,
    /// Negotiated, social resolution.
    Diplomacy = 7,
    /// Exchange of goods or services.
    Trade = 8,
    /// Unseen, undetected resolution.
    Stealth = 9,
    /// Covert disruption of an opposing plan.
    Sabotage = 10,
    /// Establishing or verifying origin/history.
    Provenance = 11,
    /// Constructing a record others can witness.
    WitnessBuilding = 12,
    /// A ceremonial or rule-bound act.
    Ritual = 13,
    /// Declining to engage at all.
    Refusal = 14,
    /// Working with or through the natural system.
    Ecology = 15,
    /// Mastery of a route or path itself as the resolution.
    RouteMastery = 16,
}

impl SolutionPathKind {
    /// Total number of canonical solution path kinds.
    pub const COUNT: usize = 17;
}
