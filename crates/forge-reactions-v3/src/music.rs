//! Music stat links — musical dimensions mapped to game stats.

/// Music dimension — musical attributes that map to game mechanics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MusicDimension {
    /// Pitch dimension.
    Pitch = 0,
    /// Rhythm dimension.
    Rhythm = 1,
    /// Harmony dimension.
    Harmony = 2,
    /// Dissonance dimension.
    Dissonance = 3,
    /// Timbre dimension.
    Timbre = 4,
    /// Volume pressure dimension.
    VolumePressure = 5,
    /// Color/chroma dimension.
    ColourChroma = 6,
}

/// Music stat — game stats that correspond to music dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MusicStat {
    /// Resonance stat (corresponds to Pitch).
    Resonance = 0,
    /// Momentum stat (corresponds to Rhythm).
    Momentum = 1,
    /// Clarity stat (corresponds to Harmony).
    Clarity = 2,
    /// Guilt stat (corresponds to Dissonance).
    Guilt = 3,
    /// Tarnish stat (corresponds to Timbre).
    Tarnish = 4,
    /// Vigor stat (corresponds to VolumePressure).
    Vigor = 5,
    /// Logic Depth stat (corresponds to ColourChroma).
    LogicDepth = 6,
}

/// Map a music dimension to its corresponding game stat.
pub const fn music_to_stat(dim: MusicDimension) -> MusicStat {
    match dim {
        MusicDimension::Pitch => MusicStat::Resonance,
        MusicDimension::Rhythm => MusicStat::Momentum,
        MusicDimension::Harmony => MusicStat::Clarity,
        MusicDimension::Dissonance => MusicStat::Guilt,
        MusicDimension::Timbre => MusicStat::Tarnish,
        MusicDimension::VolumePressure => MusicStat::Vigor,
        MusicDimension::ColourChroma => MusicStat::LogicDepth,
    }
}
