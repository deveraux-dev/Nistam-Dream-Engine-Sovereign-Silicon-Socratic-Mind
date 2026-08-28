//! Camelot wheel harmonic mixing — key compatibility for DJ track selection.
//!
//! Camelot notation: 1A–12A (minor), 1B–12B (major).
//! Compatible keys: same key, ±1 on wheel, A↔B at same number.
//! Invention #47 Audio-Genre-Driven Compositing.

/// Parse a Camelot key string into (number 1-12, is_major).
pub fn parse_camelot(key: &str) -> Option<(u8, bool)> {
    let key = key.trim();
    if key.len() < 2 || key.len() > 3 { return None; }
    let mode = key.as_bytes()[key.len() - 1];
    let is_major = match mode {
        b'B' | b'b' => true,
        b'A' | b'a' => false,
        _ => return None,
    };
    let num: u8 = key[..key.len() - 1].parse().ok()?;
    if !(1..=12).contains(&num) { return None; }
    Some((num, is_major))
}

/// Format a Camelot key back to string.
pub fn format_camelot(num: u8, is_major: bool) -> String {
    format!("{}{}", num, if is_major { "B" } else { "A" })
}

/// Dimensionless wheel index (0..11) for a Camelot key — the neutral integer the
/// cam_ele1 weld hands to gameplay (NO game types here; no-leak). `1A`→0 … `12A`→11.
pub fn key_index(key: &str) -> Option<u8> {
    parse_camelot(key).map(|(num, _major)| num - 1)
}

/// Get all compatible keys for harmonic mixing.
/// Returns keys that sound good mixed with the given key:
/// - Same key
/// - ±1 on the Camelot wheel (adjacent numbers, same mode)
/// - Mode switch (A↔B at same number)
pub fn compatible_keys(key: &str) -> Vec<String> {
    let Some((num, is_major)) = parse_camelot(key) else { return vec![] };
    let prev = if num == 1 { 12 } else { num - 1 };
    let next = if num == 12 { 1 } else { num + 1 };

    vec![
        format_camelot(num, is_major),
        format_camelot(prev, is_major),
        format_camelot(next, is_major),
        format_camelot(num, !is_major),
    ]
}

/// Distance on the Camelot wheel (0 = same, 1 = adjacent, 7 = opposite).
/// Returns None if either key is unparseable.
pub fn key_distance(a: &str, b: &str) -> Option<u8> {
    let (na, ma) = parse_camelot(a)?;
    let (nb, mb) = parse_camelot(b)?;
    let num_dist = {
        let d1 = (na as i8 - nb as i8).unsigned_abs();
        let d2 = 12 - d1;
        d1.min(d2)
    };
    let mode_dist = if ma != mb { 1u8 } else { 0 };
    Some(num_dist + mode_dist)
}

/// Check if two keys are harmonically compatible (distance ≤ 1).
pub fn is_compatible(a: &str, b: &str) -> bool {
    key_distance(a, b).map(|d| d <= 1).unwrap_or(false)
}

/// Wheel adjacency as a permyriad affinity — [`key_distance`] read as a weight.
///
/// Same key `10_000`, distance 1 (±1 on the wheel, or A↔B at the same number)
/// `5_000`, anything further `0`. Not a chosen curve: it is the compatibility rule
/// already stated above, expressed as the coupling term a spectral read needs.
pub fn affinity_pmy(a: &str, b: &str) -> i64 {
    match key_distance(a, b) {
        Some(0) => 10_000,
        Some(1) => 5_000,
        _ => 0,
    }
}

/// Read the harmonic spectrum of a track set: which tracks form a coherent mixable
/// cluster, and which are loud strangers.
///
/// The conductor/DAW read (`pp_math::spectral::spectrum`). Diagonal = each track's
/// own energy; off-diagonal = wheel affinity. `Spectrum::Uncoupled` therefore means
/// loud and in a clashing key — visible to the operator, never scored as structure.
/// Integer throughout, so the verdict replays bit-identically off the same set.
pub fn harmonic_spectrum(tracks: &[(String, i64)], floor: i32) -> pp_math::spectral::Spectrum {
    let n = tracks.len();
    let mut k = vec![0i64; n * n]; // @forge:allow_alloc load-time set analysis, not the realtime callback
    for i in 0..n {
        for j in 0..n {
            k[i * n + j] = if i == j {
                tracks[i].1
            } else {
                affinity_pmy(&tracks[i].0, &tracks[j].0) * tracks[i].1.min(tracks[j].1) / 10_000
            };
        }
    }
    pp_math::spectral::spectrum(&k, n, floor)
}

#[cfg(test)]
mod spectral_tests {
    use super::*;
    use pp_math::spectral::{Spectrum, COUPLING_FLOOR_PMY};

    #[test]
    fn affinity_follows_the_wheel_and_refuses_junk() {
        assert_eq!(affinity_pmy("8B", "8B"), 10_000, "same key");
        assert_eq!(affinity_pmy("8B", "8A"), 5_000, "relative major/minor");
        assert_eq!(affinity_pmy("8B", "9B"), 5_000, "+1 same mode");
        assert_eq!(affinity_pmy("12B", "1B"), 5_000, "the wheel wraps 12 -> 1");
        assert_eq!(affinity_pmy("8B", "2A"), 0, "a key clash couples at zero");
        assert_eq!(affinity_pmy("8B", "13B"), 0, "13 is off the wheel");
        assert_eq!(affinity_pmy("", "8B"), 0, "an empty code is not position 0");
    }

    #[test]
    fn at_equal_energy_wheel_coupling_decides_the_mode() {
        // Equal energy, so coupling is the only thing that can decide. A LOUDER
        // clashing track would legitimately carry the mode — energy is real signal —
        // so what this seam buys is a verdict over comparably loud tracks.
        let mut tracks: Vec<(String, i64)> = Vec::with_capacity(3); // @forge:allow_alloc test fixture, no callback
        for (c, e) in [("8B", 5_000i64), ("9B", 5_000), ("2A", 5_000)] {
            tracks.push((String::from(c), e)); // @forge:allow_alloc test fixture, no callback
        }
        let s = harmonic_spectrum(&tracks, COUPLING_FLOOR_PMY);
        let Spectrum::Coupled(p) = s else { panic!("a mixable pair is a mode, got {s:?}") };
        assert!(p.x[0].0 > p.x[2].0 && p.x[1].0 > p.x[2].0, "pair over the clash: {:?}", p.x);
        let lone = harmonic_spectrum(&tracks[2..], COUPLING_FLOOR_PMY);
        assert!(matches!(lone, Spectrum::Uncoupled { .. }), "a lone track is noise: {lone:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_keys() {
        assert_eq!(parse_camelot("8A"), Some((8, false)));
        assert_eq!(parse_camelot("12B"), Some((12, true)));
        assert_eq!(parse_camelot("1a"), Some((1, false)));
    }

    #[test]
    fn parse_invalid_keys() {
        assert_eq!(parse_camelot(""), None);
        assert_eq!(parse_camelot("13A"), None);
        assert_eq!(parse_camelot("0B"), None);
        assert_eq!(parse_camelot("XY"), None);
    }

    #[test]
    fn compatible_8a() {
        let compat = compatible_keys("8A");
        assert!(compat.contains(&"8A".to_string()));
        assert!(compat.contains(&"7A".to_string()));
        assert!(compat.contains(&"9A".to_string()));
        assert!(compat.contains(&"8B".to_string()));
        assert_eq!(compat.len(), 4);
    }

    #[test]
    fn compatible_wraps_12() {
        let compat = compatible_keys("12A");
        assert!(compat.contains(&"11A".to_string()));
        assert!(compat.contains(&"1A".to_string()));
    }

    #[test]
    fn compatible_wraps_1() {
        let compat = compatible_keys("1B");
        assert!(compat.contains(&"12B".to_string()));
        assert!(compat.contains(&"2B".to_string()));
    }

    #[test]
    fn distance_same_key() {
        assert_eq!(key_distance("8A", "8A"), Some(0));
    }

    #[test]
    fn distance_adjacent() {
        assert_eq!(key_distance("8A", "9A"), Some(1));
        assert_eq!(key_distance("8A", "7A"), Some(1));
    }

    #[test]
    fn distance_mode_switch() {
        assert_eq!(key_distance("8A", "8B"), Some(1));
    }

    #[test]
    fn distance_opposite() {
        assert_eq!(key_distance("1A", "7A"), Some(6));
    }

    #[test]
    fn is_compatible_true() {
        assert!(is_compatible("8A", "8A"));
        assert!(is_compatible("8A", "9A"));
        assert!(is_compatible("8A", "8B"));
    }

    #[test]
    fn is_compatible_false() {
        assert!(!is_compatible("8A", "10A"));
        assert!(!is_compatible("1A", "6A"));
    }
}
