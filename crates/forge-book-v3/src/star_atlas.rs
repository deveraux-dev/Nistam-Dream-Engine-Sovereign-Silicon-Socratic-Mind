//! Star Atlas — real, locked star microcanon (13moons.stars_microcanon.v1,
//! 2026-03-24), drained from the E:\13forge-super quarry. Not a stub.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

// ── Local celestial_alignment types and functions (no forge_game_systems v3 crate yet) ──

/// One star entry with name, constellation, magnitude, spectral class, and optional seasonal window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StarEntry {
    /// Unique identifier (e.g., HIP catalog number or cluster name).
    pub star_id: String,
    /// Display name of the star.
    pub name: String,
    /// Constellation this star belongs to.
    pub constellation: String,
    /// Apparent magnitude (brightness).
    pub mag: f64,
    /// Stellar spectral classification (e.g., A1V, M2Iab).
    pub spectral_class: String,
    /// Moons this star is worth looking for, 1..=13.
    /// Empty = always visible (absence of a seasonal claim is not a claim of absence).
    #[serde(default)]
    pub best_moons: Vec<u8>,
}

impl StarEntry {
    /// True if this star is in season for `moon` (1..=13).
    /// An EMPTY `best_moons` reads as always-visible, not never-visible.
    pub fn visible_in_moon(&self, moon: u8) -> bool {
        self.best_moons.is_empty() || self.best_moons.contains(&moon)
    }
}

/// Brightness classification for a star magnitude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessBucket {
    /// Lore name for this brightness level.
    pub name: String,
    /// Maximum magnitude (brightness) to include in this bucket.
    pub max_mag: f64,
}

/// Spectral class classification for a star.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralBucket {
    /// First character of spectral classification (O, B, A, F, G, K, M).
    pub prefix: char,
    /// Lore name for this spectral class bucket.
    pub bucket: String,
}

/// Rules for bucketing stars by brightness and spectral class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreRules {
    /// Brightness buckets ordered by magnitude threshold.
    pub brightness_buckets: Vec<BrightnessBucket>,
    /// Spectral class buckets mapped by first character.
    pub spectral_buckets: Vec<SpectralBucket>,
}

/// Classify a magnitude into a brightness bucket.
pub fn get_brightness_bucket(mag: f64, rules: &LoreRules) -> String {
    let mut sorted = rules.brightness_buckets.clone();
    sorted.sort_by(|a, b| a.max_mag.partial_cmp(&b.max_mag).unwrap());
    sorted
        .iter()
        .find(|b| mag <= b.max_mag)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| sorted.last().map(|b| b.name.clone()).unwrap_or("UNKNOWN".into()))
}

/// Classify a spectral class into a spectral bucket (first character match).
pub fn get_spectral_bucket(spectral_class: &str, rules: &LoreRules) -> String {
    let first = spectral_class.chars().next().unwrap_or('?');
    rules
        .spectral_buckets
        .iter()
        .find(|b| b.prefix == first)
        .map(|b| b.bucket.clone())
        .unwrap_or("UNKNOWN".into())
}

/// Real, locked microcanon (13moons.stars_microcanon.v1, 2026-03-24) — 16 real
/// HIP/cluster/planet entries, Prairie-visible at ~52N.
pub fn ironroot_microcanon() -> Vec<StarEntry> {
    let s = |id: &str, name: &str, con: &str, mag: f64, spec: &str, moons: &[u8]| StarEntry {
        star_id: id.into(),
        name: name.into(),
        constellation: con.into(),
        mag,
        spectral_class: spec.into(),
        best_moons: moons.to_vec(),
    };
    const ALL_YEAR: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    vec![
        s("HIP_32349", "Sirius", "Canis Major", -1.46, "A1V", &[1, 2, 12, 13]),
        s("HIP_11767", "Polaris", "Ursa Minor", 1.98, "F8Ib", ALL_YEAR),
        s("HIP_91262", "Vega", "Lyra", 0.03, "A0V", &[5, 6, 7, 8, 9]),
        s("HIP_102098", "Deneb", "Cygnus", 1.25, "A2Ia", &[5, 6, 7, 8, 9, 10]),
        s("HIP_97649", "Altair", "Aquila", 0.77, "A7V", &[6, 7, 8, 9]),
        s("HIP_24436", "Capella", "Auriga", 0.08, "G5III", ALL_YEAR),
        s("HIP_27989", "Betelgeuse", "Orion", 0.42, "M2Iab", &[1, 2, 3, 12, 13]),
        s("HIP_24608", "Rigel", "Orion", 0.13, "B8Ia", &[1, 2, 3, 12, 13]),
        s("HIP_69673", "Arcturus", "Bootes", -0.05, "K1.5III", &[3, 4, 5, 6, 7, 8, 9]),
        s("HIP_65474", "Spica", "Virgo", 0.97, "B1III", &[4, 5, 6, 7, 8]),
        s("HIP_37826", "Procyon", "Canis Minor", 0.34, "F5IV", &[1, 2, 3, 12, 13]),
        s("CLUSTER_M45", "Pleiades", "Taurus", 1.6, "B6III", &[1, 2, 3, 10, 11, 12, 13]),
        s("ASTERISM_BIG_DIPPER", "Big Dipper", "Ursa Major", 1.8, "A1V", ALL_YEAR),
        s("PLANET_VENUS", "Morning Star", "ecliptic", -4.6, "PLANET", ALL_YEAR),
        s("GALAXY_M31", "Andromeda Galaxy", "Andromeda", 3.44, "GALAXY", &[8, 9, 10, 11, 12, 1]),
        s("BAND_MILKY_WAY", "Milky Way", "galactic", -6.5, "BAND", &[5, 6, 7, 8, 9]),
    ]
}

/// Lore rules matching ironroot_microcanon (13moons.stars_lore_rules.v1, 2026-03-24).
pub fn ironroot_lore_rules() -> LoreRules {
    LoreRules {
        brightness_buckets: vec![
            BrightnessBucket { name: "SPIRIT_FIRE".into(), max_mag: 0.0 },
            BrightnessBucket { name: "GUIDE_STAR".into(), max_mag: 2.0 },
            BrightnessBucket { name: "ANCESTOR_LIGHT".into(), max_mag: 4.0 },
            BrightnessBucket { name: "THE_FORGOTTEN".into(), max_mag: 99.0 },
        ],
        spectral_buckets: vec![
            SpectralBucket { prefix: 'O', bucket: "DEEP_WINTER".into() },
            SpectralBucket { prefix: 'B', bucket: "BONE_STAR".into() },
            SpectralBucket { prefix: 'A', bucket: "FROST".into() },
            SpectralBucket { prefix: 'F', bucket: "ASKIY_GOLD".into() },
            SpectralBucket { prefix: 'G', bucket: "ASKIY_GOLD".into() },
            SpectralBucket { prefix: 'K', bucket: "THE_FORGE".into() },
            SpectralBucket { prefix: 'M', bucket: "WISAKEDJAK".into() },
        ],
    }
}

/// One lore line per star: name, constellation, magnitude, brightness/spectral bucket.
pub fn star_atlas_chapter(title: impl Into<String>) -> Chapter {
    let rules = ironroot_lore_rules();
    let mut ch = Chapter::new(title, AtlasSection::Custom("Sky".into()));
    for s in ironroot_microcanon() {
        let brightness = get_brightness_bucket(s.mag, &rules);
        let spectral = get_spectral_bucket(&s.spectral_class, &rules);
        ch.add_lore(format!("{} [{}] mag {} — {}/{}", s.name, s.constellation, s.mag, brightness, spectral));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_atlas_chapter_has_sixteen_stars() {
        let ch = star_atlas_chapter("Star Atlas");
        assert_eq!(ch.lore_count(), 16);
    }

    #[test]
    fn star_visible_in_moon_empty_is_always() {
        let s = StarEntry {
            star_id: "OLD".into(),
            name: "Legacy".into(),
            constellation: "none".into(),
            mag: 1.0,
            spectral_class: "A0V".into(),
            best_moons: Vec::new(),
        };
        for moon in 1..=13 {
            assert!(s.visible_in_moon(moon));
        }
    }

    #[test]
    fn brightness_bucketing() {
        let rules = LoreRules {
            brightness_buckets: vec![
                BrightnessBucket { name: "BLAZE".into(), max_mag: 1.0 },
                BrightnessBucket { name: "LAMP".into(), max_mag: 3.0 },
                BrightnessBucket { name: "ASH".into(), max_mag: 6.0 },
            ],
            spectral_buckets: vec![],
        };
        assert_eq!(get_brightness_bucket(0.5, &rules), "BLAZE");
        assert_eq!(get_brightness_bucket(5.0, &rules), "ASH");
    }

    #[test]
    fn spectral_bucketing() {
        let rules = LoreRules {
            brightness_buckets: vec![],
            spectral_buckets: vec![
                SpectralBucket { prefix: 'O', bucket: "FROST".into() },
                SpectralBucket { prefix: 'G', bucket: "GOLD".into() },
            ],
        };
        assert_eq!(get_spectral_bucket("O5V", &rules), "FROST");
        assert_eq!(get_spectral_bucket("G2V", &rules), "GOLD");
    }

    #[test]
    fn ironroot_microcanon_has_sixteen_real_stars() {
        let stars = ironroot_microcanon();
        assert_eq!(stars.len(), 16);
        assert!(stars.iter().any(|s| s.name == "Polaris" && s.star_id == "HIP_11767"));
    }

    #[test]
    fn ironroot_lore_rules_bucket_a_real_star() {
        let rules = ironroot_lore_rules();
        assert_eq!(get_brightness_bucket(-1.46, &rules), "SPIRIT_FIRE"); // Sirius
        assert_eq!(get_spectral_bucket("A1V", &rules), "FROST"); // Sirius
    }
}
