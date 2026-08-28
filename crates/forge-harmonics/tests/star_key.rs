//! Integration test verifying CamelotKey derivation across all 16 CATALOG_16 stars.

use forge_harmonics::CamelotKey;

#[test]
fn test_all_16_stars_map_to_valid_camelot_keys() {
    let mut seen = std::collections::HashSet::new();
    for idx in 0..16 {
        let key = CamelotKey::from_star_idx(idx).expect("Star index 0..15 must produce CamelotKey");
        assert!((1..=12).contains(&key.number), "Camelot number must be in 1..=12");
        if idx < 12 {
            assert!(key.is_minor, "Stars 0..11 must be minor (A)");
        } else {
            assert!(!key.is_minor, "Stars 12..15 must be major (B)");
        }
        let mut buf = [0u8; 4];
        let tag = key.format_fixed(&mut buf).to_string();
        seen.insert(tag);

        // Verify tonic pitch class matches (9 - idx).rem_euclid(12)
        let expected_pc = (9 - (idx as i32)).rem_euclid(12) as u8;
        assert_eq!(
            key.tonic_pitch_class(),
            expected_pc,
            "Star {idx} key {key:?} tonic pc must match expected {expected_pc}"
        );
    }
    assert_eq!(seen.len(), 16, "All 16 stars must map to distinct Camelot keys");
    assert_eq!(CamelotKey::from_star_idx(16), None, "Index >= 16 must return None");
}

/// L07 bijection: `star_idx` is `from_star_idx`'s inverse over the whole
/// catalogue, and the eight major keys no star occupies report None.
#[test]
fn every_star_round_trips_through_its_key() {
    for idx in 0..16 {
        let key = CamelotKey::from_star_idx(idx).expect("star in range");
        assert_eq!(key.star_idx(), Some(idx), "star {idx} did not round-trip via {key:?}");
    }
    let unoccupied: Vec<u8> = (1..=12u8)
        .filter(|&n| CamelotKey::new(n, false).star_idx().is_none())
        .collect();
    assert_eq!(unoccupied.len(), 8, "only four major keys carry a star: {unoccupied:?}");
}

#[test]
fn test_sirius_is_8a_and_major_transition() {
    let sirius = CamelotKey::from_star_idx(0).unwrap();
    assert_eq!(sirius, CamelotKey { number: 8, is_minor: true });

    let aldebaran = CamelotKey::from_star_idx(12).unwrap();
    assert_eq!(aldebaran, CamelotKey { number: 11, is_minor: false });

    let pollux = CamelotKey::from_star_idx(15).unwrap();
    assert_eq!(pollux, CamelotKey { number: 2, is_minor: false });
}
