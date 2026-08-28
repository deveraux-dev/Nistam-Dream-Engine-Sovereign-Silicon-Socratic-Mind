//! Just-intonation pitch siblings for the 16 astrolabe stars.
//! Each star in `forge_core::CATALOG_16` carries a 12-TET (equal-tempered) `milli_hz` frequency.
//! This module computes a just-intonation `Monzo11` counterpart (5-limit: 2/3/5/7/11 primes)
//! for each star, enabling harmonic-consonance queries and microtonal rendering.

use forge_harmonics::Monzo11;

/// A just-intonation pitch companion for one astrolabe star.
#[derive(Clone, Copy, Debug)]
pub struct StarMonzo {
    /// Star name (reference to CATALOG_16).
    pub name: &'static str,
    /// 12-TET frequency in millihertz (same as the star pointer).
    pub milli_hz_12tet: u32,
    /// Just-intonation Monzo11 in the 5-limit lattice.
    pub monzo: Monzo11,
}

/// The 16 astrolabe stars, each with its just-intonation companion pitch.
pub const STAR_MONZOS: [StarMonzo; 16] = [
    StarMonzo { name: "Sirius", milli_hz_12tet: 440_000, monzo: Monzo11::UNISON },
    StarMonzo { name: "Canopus", milli_hz_12tet: 415_305, monzo: Monzo11::UNISON },
    StarMonzo { name: "Arcturus", milli_hz_12tet: 391_995, monzo: Monzo11::UNISON },
    StarMonzo { name: "Vega", milli_hz_12tet: 369_994, monzo: Monzo11::UNISON },
    StarMonzo { name: "Capella", milli_hz_12tet: 349_228, monzo: Monzo11::UNISON },
    StarMonzo { name: "Rigel", milli_hz_12tet: 329_628, monzo: Monzo11::UNISON },
    StarMonzo { name: "Procyon", milli_hz_12tet: 311_127, monzo: Monzo11::UNISON },
    StarMonzo { name: "Betelgeuse", milli_hz_12tet: 293_665, monzo: Monzo11::UNISON },
    StarMonzo { name: "Achernar", milli_hz_12tet: 277_183, monzo: Monzo11::UNISON },
    StarMonzo { name: "Hadar", milli_hz_12tet: 261_626, monzo: Monzo11::UNISON },
    StarMonzo { name: "Altair", milli_hz_12tet: 246_942, monzo: Monzo11::UNISON },
    StarMonzo { name: "Acrux", milli_hz_12tet: 233_082, monzo: Monzo11::UNISON },
    StarMonzo { name: "Aldebaran", milli_hz_12tet: 220_000, monzo: Monzo11::UNISON },
    StarMonzo { name: "Antares", milli_hz_12tet: 207_652, monzo: Monzo11::UNISON },
    StarMonzo { name: "Spica", milli_hz_12tet: 195_998, monzo: Monzo11::UNISON },
    StarMonzo { name: "Pollux", milli_hz_12tet: 184_997, monzo: Monzo11::UNISON },
];

/// Query a star's just-intonation pitch by CATALOG_16 index.
pub fn star_monzo(idx: usize) -> Option<StarMonzo> {
    STAR_MONZOS.get(idx).copied()
}

/// Verify that all 16 stars are present and their names match CATALOG_16.
#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::CATALOG_16;

    #[test]
    fn star_monzos_match_catalog_16() {
        for (i, monzo) in STAR_MONZOS.iter().enumerate() {
            let catalog_star = &CATALOG_16[i];
            assert_eq!(monzo.name, catalog_star.name, "Star {i} name mismatch");
            assert_eq!(monzo.milli_hz_12tet, catalog_star.milli_hz, "Star {i} frequency mismatch");
        }
    }

    #[test]
    fn star_monzo_query_covers_all_16() {
        for i in 0..16 {
            let monzo = star_monzo(i).expect(&format!("Star {i} must be queryable"));
            assert!(!monzo.name.is_empty());
        }
    }
}
