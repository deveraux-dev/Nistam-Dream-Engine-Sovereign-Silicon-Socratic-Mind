//! Just-intonation pitch siblings for the 16 astrolabe stars.
//!
//! Each star carries a 12-TET (equal-tempered) `milli_hz` frequency and a
//! just-intonation `Monzo11` counterpart (5-limit / 11-limit prime lattice: 2, 3, 5, 7, 11)
//! for harmonic-consonance queries and microtonal rendering.

use crate::mersenne_lattice::{Monzo, Monzo11};

/// A just-intonation pitch companion for one astrolabe star.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarMonzo {
    /// Star name.
    pub name: &'static str,
    /// 12-TET frequency in millihertz.
    pub milli_hz_12tet: u32,
    /// Just-intonation Monzo11 in the prime lattice.
    pub monzo: Monzo11,
}

/// The 16 astrolabe stars with their just-intonation companion pitch ratios.
pub const STAR_MONZOS: [StarMonzo; 16] = [
    StarMonzo { name: "Sirius", milli_hz_12tet: 440_000, monzo: Monzo::UNISON },
    StarMonzo { name: "Canopus", milli_hz_12tet: 415_305, monzo: Monzo([-1, 0, 0, 1, 0]) }, // 7/2
    StarMonzo { name: "Arcturus", milli_hz_12tet: 391_995, monzo: Monzo([-1, 1, 0, 0, 0]) }, // 3/2 (Fifth)
    StarMonzo { name: "Vega", milli_hz_12tet: 369_994, monzo: Monzo([-2, 0, 1, 0, 0]) },    // 5/4 (Major Third)
    StarMonzo { name: "Capella", milli_hz_12tet: 349_228, monzo: Monzo([2, -1, 0, 0, 0]) },  // 4/3 (Fourth)
    StarMonzo { name: "Rigel", milli_hz_12tet: 329_628, monzo: Monzo([-1, 0, 1, 0, 0]) },    // 5/2
    StarMonzo { name: "Procyon", milli_hz_12tet: 311_127, monzo: Monzo([0, -1, 1, 0, 0]) },  // 5/3 (Major Sixth)
    StarMonzo { name: "Betelgeuse", milli_hz_12tet: 293_665, monzo: Monzo([-3, 2, 0, 0, 0]) },// 9/8 (Major Second)
    StarMonzo { name: "Achernar", milli_hz_12tet: 277_183, monzo: Monzo([1, 0, 0, 0, -1]) }, // 2/11
    StarMonzo { name: "Hadar", milli_hz_12tet: 261_626, monzo: Monzo([-1, 0, 0, 0, 1]) },   // 11/2
    StarMonzo { name: "Altair", milli_hz_12tet: 246_942, monzo: Monzo([-4, 1, 1, 0, 0]) },  // 15/16
    StarMonzo { name: "Acrux", milli_hz_12tet: 233_082, monzo: Monzo([0, 0, 0, -1, 1]) },   // 11/7
    StarMonzo { name: "Aldebaran", milli_hz_12tet: 220_000, monzo: Monzo([-1, 0, 0, 0, 0]) }, // Octave down (1/2)
    StarMonzo { name: "Antares", milli_hz_12tet: 207_652, monzo: Monzo([-2, 0, 0, 1, 0]) },
    StarMonzo { name: "Spica", milli_hz_12tet: 195_998, monzo: Monzo([-2, 1, 0, 0, 0]) },
    StarMonzo { name: "Pollux", milli_hz_12tet: 184_997, monzo: Monzo([-3, 0, 1, 0, 0]) },
];

/// Query a star's just-intonation pitch companion by index (0..16).
pub fn star_monzo(idx: usize) -> Option<StarMonzo> {
    STAR_MONZOS.get(idx).copied()
}

/// Find the nearest star monzo to a target millihertz frequency.
pub fn nearest_star_monzo(milli_hz: u32) -> StarMonzo {
    *STAR_MONZOS
        .iter()
        .min_by_key(|s| (s.milli_hz_12tet as i64 - milli_hz as i64).abs())
        .unwrap_or(&STAR_MONZOS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_16_stars_queryable() {
        for i in 0..16 {
            let star = star_monzo(i).expect("all 16 stars must be queryable");
            assert!(!star.name.is_empty());
            assert!(star.milli_hz_12tet > 0);
        }
        assert_eq!(star_monzo(16), None);
    }

    #[test]
    fn test_nearest_star_monzo_anchors() {
        assert_eq!(nearest_star_monzo(440_000).name, "Sirius");
        assert_eq!(nearest_star_monzo(220_000).name, "Aldebaran");
    }
}
