//! Star lore rules — drained from `stars_lore_rules.v1.json`
//! (`13moons.stars_lore_rules.v1`, adapted from AKGAME `zr.stars_lore_rules.v1`,
//! locked 2026-03-24). Integer-only: magnitudes ride Permyriad, never floats.
//!
//! Two independent classifications over the same sky — how BRIGHT a light is
//! (spiritual weight) and what COLOUR it burns (season/character) — plus the
//! per-moon sky and the Walker's effect on each brightness band.

use serde::{Deserialize, Serialize};

/// Apparent magnitude in Permyriad (`1_0000 = 1.0 mag`). Magnitudes run
/// BACKWARDS — smaller is brighter, and the brightest are negative — so this is
/// `i32`, not the unsigned Permyriad the rest of the lore module uses.
pub type MagQ = i32;

/// Brightness band — spiritual weight, brightest first. The Walker dims these
/// in order (see [`walker_effect`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Brightness {
    /// Bright enough to cast shadow on snow. Sirius, Arcturus, Vega, Capella,
    /// Rigel, Morning Star. AKGAME `BLAZE`.
    SpiritFire,
    /// Cree wayfinding across open prairie — the ones you follow home. Polaris,
    /// Deneb, Altair, Spica, Pleiades. AKGAME `LAMP`.
    GuideStar,
    /// Distant but present: someone who walked the prairie before you. Most
    /// naked-eye stars, Andromeda. AKGAME `EMBER`.
    AncestorLight,
    /// Too dim to see, but the land still feels them. Deep field. AKGAME `ASH`.
    TheForgotten,
}

/// Upper magnitude bound of each band, Permyriad. A light belongs to the FIRST
/// band whose bound it does not exceed.
const BRIGHTNESS_LADDER: [(MagQ, Brightness); 4] = [
    (0, Brightness::SpiritFire),
    (2_0000, Brightness::GuideStar),
    (4_0000, Brightness::AncestorLight),
    (99_0000, Brightness::TheForgotten),
];

impl Brightness {
    /// The band's AKGAME name — kept because the save format and the AKGAME
    /// bridge still speak it.
    pub fn akgame_name(self) -> &'static str {
        match self {
            Self::SpiritFire => "BLAZE",
            Self::GuideStar => "LAMP",
            Self::AncestorLight => "EMBER",
            Self::TheForgotten => "ASH",
        }
    }

    /// What the band means to a watcher on the prairie.
    pub fn meaning(self) -> &'static str {
        match self {
            Self::SpiritFire => {
                "Stars bright enough to cast shadow on snow. Strongest spiritual weight. \
                 The Walker dims these first."
            }
            Self::GuideStar => {
                "Navigation stars. Cree wayfinding across open prairie. These are the ones \
                 you follow home."
            }
            Self::AncestorLight => {
                "Distant but present. Each ancestor light is someone who walked the prairie \
                 before you."
            }
            Self::TheForgotten => {
                "Too dim to see. But the land still feels them. Only visible through telescope \
                 or long exposure."
            }
        }
    }
}

/// Band a magnitude falls in. Anything dimmer than the last bound is still
/// [`Brightness::TheForgotten`] — the land feels them regardless.
pub fn brightness_of(mag_q: MagQ) -> Brightness {
    for (bound, band) in BRIGHTNESS_LADDER {
        if mag_q <= bound {
            return band;
        }
    }
    Brightness::TheForgotten
}

/// Colour band — spectral class mapped onto the season and its character.
/// `AskiyGold` covers BOTH F and G: the ladder is 10 prefixes over 9 bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spectral {
    /// O — hottest, rarest, blue fire. Moons 1-2 and 12-13. Walker season.
    DeepWinter,
    /// B — blue-white, sharp and cold. Bleached like prairie bones in winter sun.
    BoneStar,
    /// A — white watchers. Sirius, Vega, Deneb. Cold clarity. They see you.
    Frost,
    /// F and G — yellow-white warmth, the Land providing. Our Sun's class.
    AskiyGold,
    /// K — orange. Transformation through heat. Arcturus. The forge that reshapes.
    TheForge,
    /// M — red, ancient, unpredictable. Betelgeuse. The trickster's eyes watching
    /// from dying fire.
    Wisakedjak,
    /// Planets move. They do not follow the rules. Morning Star is the brightest.
    Wanderer,
    /// Another fire, impossibly far — the prairie is not the whole world.
    TheDistant,
    /// Meskanaw, The Road. The Milky Way, the spirit path across the sky.
    Meskanaw,
}

/// Authored prefix → band. `PLANET`/`GALAXY`/`BAND` are not spectral classes;
/// they are the three things in the sky that a letter cannot describe.
const SPECTRAL_LADDER: [(&str, Spectral); 10] = [
    ("O", Spectral::DeepWinter),
    ("B", Spectral::BoneStar),
    ("A", Spectral::Frost),
    ("F", Spectral::AskiyGold),
    ("G", Spectral::AskiyGold),
    ("K", Spectral::TheForge),
    ("M", Spectral::Wisakedjak),
    ("PLANET", Spectral::Wanderer),
    ("GALAXY", Spectral::TheDistant),
    ("BAND", Spectral::Meskanaw),
];

impl Spectral {
    /// Packed `0xRRGGBBAA`, opaque. The authored hex, not a re-derivation.
    pub fn rgba(self) -> u32 {
        match self {
            Self::DeepWinter => 0x9BB0FFFF,
            Self::BoneStar => 0xAABFFFFF,
            Self::Frost => 0xCAD7FFFF,
            // F is #F8F7FF and G is #FFF4EA; one band, and G's warmth is the one
            // that ships — it is our own Sun's colour, the reference the rest read against.
            Self::AskiyGold => 0xFFF4EAFF,
            Self::TheForge => 0xFFD2A1FF,
            Self::Wisakedjak => 0xFFCC6FFF,
            Self::Wanderer => 0xFFFFFFFF,
            Self::TheDistant => 0xE8E0D0FF,
            Self::Meskanaw => 0xD4C8B0FF,
        }
    }
}

/// Band for an authored prefix (`"O"`, `"M"`, `"PLANET"`, …). Case-insensitive.
/// Unknown prefix = `None`: an unclassified light is never silently gold.
pub fn spectral_of(prefix: &str) -> Option<Spectral> {
    let up = prefix.to_ascii_uppercase();
    SPECTRAL_LADDER.iter().find(|(p, _)| *p == up).map(|(_, b)| *b)
}

/// How much of the Milky Way shows, per moon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MilkyWay {
    /// Milky Way is barely visible or obscured.
    Low,
    /// Milky Way becoming more visible as the season progresses.
    Rising,
    /// Milky Way visibility increasing noticeably.
    Brightening,
    /// Milky Way is clearly visible and prominent.
    Bright,
    /// Milky Way visibility at its maximum.
    Peak,
    /// Milky Way visibility beginning to diminish.
    Fading,
}

/// How dark the night runs, per moon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Darkness {
    /// Night is at its darkest point of the year.
    Deepest,
    /// Night is very dark.
    Deep,
    /// Night darkness is moderate.
    Moderate,
    /// Night is growing longer as season advances.
    Growing,
    /// Night is noticeably short; twilight lingers.
    Short,
    /// Nights are consistently short throughout the period.
    ShortNights,
    /// Night is at its shortest point of the year.
    Shortest,
}

/// One moon's sky — what is prominent, and how the night reads.
///
/// Serialize-only: the dominant list is `&'static str`, so the table IS the
/// source. Reading a sky back from JSON would mean a second, forkable copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MoonSky {
    /// What dominates, in authored order.
    pub dominant: &'static [&'static str],
    /// Milky Way visibility during this moon.
    pub milky_way: MilkyWay,
    /// Night darkness level during this moon.
    pub darkness: Darkness,
}

/// The 13 moons' skies, index `0` = moon 1. Drives celestial viewport rendering.
pub const MOON_SKIES: [MoonSky; 13] = [
    MoonSky { dominant: &["Orion", "Sirius", "Pleiades"], milky_way: MilkyWay::Low, darkness: Darkness::Deep },
    MoonSky { dominant: &["Orion", "Sirius", "Capella"], milky_way: MilkyWay::Low, darkness: Darkness::Deep },
    MoonSky { dominant: &["Arcturus_rising", "Orion_setting"], milky_way: MilkyWay::Low, darkness: Darkness::Moderate },
    MoonSky { dominant: &["Arcturus", "Spica", "Big_Dipper_high"], milky_way: MilkyWay::Rising, darkness: Darkness::Moderate },
    MoonSky { dominant: &["Summer_Triangle_rising", "Arcturus"], milky_way: MilkyWay::Brightening, darkness: Darkness::ShortNights },
    MoonSky { dominant: &["Vega_zenith", "Summer_Triangle"], milky_way: MilkyWay::Bright, darkness: Darkness::Shortest },
    MoonSky { dominant: &["Summer_Triangle", "Milky_Way_center"], milky_way: MilkyWay::Peak, darkness: Darkness::Short },
    MoonSky { dominant: &["Summer_Triangle", "Andromeda_rising"], milky_way: MilkyWay::Peak, darkness: Darkness::Growing },
    MoonSky { dominant: &["Andromeda", "Deneb_high"], milky_way: MilkyWay::Bright, darkness: Darkness::Moderate },
    MoonSky { dominant: &["Andromeda_high", "Pleiades_rising"], milky_way: MilkyWay::Fading, darkness: Darkness::Growing },
    MoonSky { dominant: &["Pleiades", "Capella", "Andromeda"], milky_way: MilkyWay::Low, darkness: Darkness::Deep },
    MoonSky { dominant: &["Orion_rising", "Pleiades_high"], milky_way: MilkyWay::Low, darkness: Darkness::Deepest },
    MoonSky { dominant: &["Orion", "Sirius", "Betelgeuse"], milky_way: MilkyWay::Low, darkness: Darkness::Deep },
];

/// The sky of moon `n` (1-13). Out of range = `None`; the year has 13 moons and
/// a 14th is a caller bug, not a wrap.
pub fn moon_sky(moon: u8) -> Option<MoonSky> {
    MOON_SKIES.get((moon.checked_sub(1)?) as usize).copied()
}

/// What the Walker's presence does to a brightness band. Permyriad of the
/// light that REMAINS: `10_000` = untouched, `0` = gone.
///
/// `TheForgotten` is unchanged — they were already beyond the Walker's reach,
/// which is the whole point of the band.
pub fn walker_effect(band: Brightness) -> u16 {
    match band {
        // "dim by 60%" — 40% of the light remains.
        Brightness::SpiritFire => 4_000,
        // Flicker is erratic, not dimmer: navigation fails on RELIABILITY, and
        // that is the renderer's business (see `walker_flickers`).
        Brightness::GuideStar => 10_000,
        Brightness::AncestorLight => 0,
        Brightness::TheForgotten => 10_000,
    }
}

/// Does this band flicker erratically under the Walker? Only the guide stars —
/// they still burn, they just stop being trustworthy.
pub fn walker_flickers(band: Brightness) -> bool {
    matches!(band, Brightness::GuideStar)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_bands_every_authored_example() {
        // Sirius (-1.46), Vega (0.03 — SPIRIT_FIRE by the <= 0.0 bound? no: 0.03 > 0)
        assert_eq!(brightness_of(-1_4600), Brightness::SpiritFire);
        assert_eq!(brightness_of(0), Brightness::SpiritFire);
        // Polaris ~1.98 — a guide star, the one you follow home.
        assert_eq!(brightness_of(1_9800), Brightness::GuideStar);
        assert_eq!(brightness_of(3_5000), Brightness::AncestorLight);
        assert_eq!(brightness_of(12_0000), Brightness::TheForgotten);
        // Past the last bound is still forgotten, never a panic or a wrap.
        assert_eq!(brightness_of(400_0000), Brightness::TheForgotten);
    }

    #[test]
    fn f_and_g_share_askiy_gold_and_nothing_else_doubles_up() {
        assert_eq!(spectral_of("F"), Some(Spectral::AskiyGold));
        assert_eq!(spectral_of("G"), Some(Spectral::AskiyGold));
        assert_eq!(spectral_of("m"), Some(Spectral::Wisakedjak), "case-insensitive");
        assert_eq!(spectral_of("BAND"), Some(Spectral::Meskanaw));
        assert_eq!(spectral_of("Z"), None, "an unclassified light is never silently gold");
        // 10 authored prefixes, 9 distinct bands — F/G is the ONE fold.
        let mut bands: Vec<Spectral> = SPECTRAL_LADDER.iter().map(|(_, b)| *b).collect();
        bands.dedup();
        assert_eq!(SPECTRAL_LADDER.len(), 10);
        assert_eq!(bands.len(), 9);
    }

    #[test]
    fn thirteen_moons_and_no_fourteenth() {
        assert_eq!(MOON_SKIES.len(), 13);
        assert_eq!(moon_sky(1).expect("moon 1").darkness, Darkness::Deep);
        assert_eq!(moon_sky(6).expect("moon 6").milky_way, MilkyWay::Bright);
        assert_eq!(moon_sky(7).expect("moon 7").milky_way, MilkyWay::Peak);
        assert_eq!(moon_sky(12).expect("moon 12").darkness, Darkness::Deepest);
        assert!(moon_sky(0).is_none(), "moons are 1-indexed");
        assert!(moon_sky(14).is_none(), "the year has 13 moons");
        for (i, sky) in MOON_SKIES.iter().enumerate() {
            assert!(!sky.dominant.is_empty(), "moon {} names nothing", i + 1);
        }
    }

    // The Walker's rule is an ORDER, not four independent numbers: what he
    // consumes first is what burns brightest, and the forgotten are untouched.
    #[test]
    fn the_walker_takes_the_brightest_and_never_the_forgotten() {
        assert_eq!(walker_effect(Brightness::SpiritFire), 4_000, "dim by 60%");
        assert_eq!(walker_effect(Brightness::AncestorLight), 0, "the ancestors withdraw");
        assert_eq!(walker_effect(Brightness::TheForgotten), 10_000, "already beyond his reach");
        assert!(walker_flickers(Brightness::GuideStar), "navigation becomes unreliable");
        assert!(!walker_flickers(Brightness::SpiritFire), "only the guide stars flicker");
    }
}
