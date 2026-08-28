//! Fae item ethics — pressure deltas for the 5 fae reward outcomes.

/// How a player interacts with a fae reward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FaeItemOutcome {
    /// The reward was taken outright.
    Claimed = 0,
    /// The reward was negotiated for.
    Bargained = 1,
    /// The reward was freely given away.
    Gifted = 2,
    /// The reward was taken without consent.
    Stolen = 3,
    /// The reward was declined.
    Refused = 4,
}

/// Pressure deltas produced by one fae reward outcome, permyriad-quantized.
#[derive(Clone, Copy, Debug, Default)]
pub struct FaeItemPressure {
    /// Change in ownership pressure.
    pub ownership_pressure_q: i16,
    /// Change in outstanding obligation pressure.
    pub obligation_pressure_q: i16,
    /// Change in temptation toward crown-claiming behavior.
    pub crown_temptation_q: i16,
    /// Change in fae hostility toward the player.
    pub fae_hostility_q: i16,
    /// Change in ecological pressure.
    pub ecology_pressure_q: i16,
    /// True if this outcome leaves detectable shimmer residue.
    pub shimmer_detectable: bool,
}

/// Look up the pressure deltas produced by a given fae reward outcome.
pub const fn fae_item_pressure(outcome: FaeItemOutcome) -> FaeItemPressure {
    match outcome {
        FaeItemOutcome::Claimed => FaeItemPressure { ownership_pressure_q: 2500, obligation_pressure_q: 0, crown_temptation_q: 2000, fae_hostility_q: 2000, ecology_pressure_q: 500, shimmer_detectable: false },
        FaeItemOutcome::Bargained => FaeItemPressure { ownership_pressure_q: 500, obligation_pressure_q: 2500, crown_temptation_q: 500, fae_hostility_q: -500, ecology_pressure_q: 0, shimmer_detectable: false },
        FaeItemOutcome::Gifted => FaeItemPressure { ownership_pressure_q: -1000, obligation_pressure_q: 500, crown_temptation_q: -2000, fae_hostility_q: -2000, ecology_pressure_q: -1000, shimmer_detectable: false },
        FaeItemOutcome::Stolen => FaeItemPressure { ownership_pressure_q: 3000, obligation_pressure_q: 0, crown_temptation_q: 1500, fae_hostility_q: 3500, ecology_pressure_q: 1000, shimmer_detectable: true },
        FaeItemOutcome::Refused => FaeItemPressure { ownership_pressure_q: -1500, obligation_pressure_q: -500, crown_temptation_q: -2500, fae_hostility_q: -3000, ecology_pressure_q: -2000, shimmer_detectable: false },
    }
}
