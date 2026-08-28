//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\theory.rs (132 LOC).
//!
//! Music theory for harmonic mixing — Camelot wheel compatibility.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionType { Smooth, BpmRide, Bridge, HardCut }

pub fn parse_camelot(camelot: &str) -> Option<(u8, bool)> {
    let s = camelot.trim();
    if s.len() < 2 { return None; }
    let (num_str, letter) = s.split_at(s.len() - 1);
    let num: u8 = num_str.parse().ok()?;
    if !(1..=12).contains(&num) { return None; }
    match letter {
        "A" | "a" => Some((num, true)),  // minor
        "B" | "b" => Some((num, false)), // major
        _ => None,
    }
}

pub fn keys_compatible(key_a: &str, key_b: &str) -> bool {
    let a = match parse_camelot(key_a) { Some(v) => v, None => return false };
    let b = match parse_camelot(key_b) { Some(v) => v, None => return false };
    // Same key
    if a == b { return true; }
    // Same number, different mode (relative major/minor)
    if a.0 == b.0 { return true; }
    // ±1 on the wheel (same mode)
    if a.1 == b.1 {
        let diff = (a.0 as i8 - b.0 as i8).unsigned_abs();
        if diff == 1 || diff == 11 { return true; } // wrap around 12→1
    }
    false
}

pub fn compatible_keys(camelot: &str) -> Vec<String> {
    let (num, is_minor) = match parse_camelot(camelot) { Some(v) => v, None => return vec![] };
    let letter = if is_minor { "A" } else { "B" };
    let other = if is_minor { "B" } else { "A" };
    let prev = if num == 1 { 12 } else { num - 1 };
    let next = if num == 12 { 1 } else { num + 1 };
    vec![
        format!("{}{}", num, letter),       // same
        format!("{}{}", prev, letter),      // -1
        format!("{}{}", next, letter),      // +1
        format!("{}{}", num, other),        // relative major/minor
    ]
}

pub fn transition_type(bpm_from: f64, bpm_to: f64) -> TransitionType {
    let diff = (bpm_from - bpm_to).abs();
    if diff <= 3.0 { TransitionType::Smooth }
    else if diff <= 8.0 { TransitionType::BpmRide }
    else { TransitionType::Bridge }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camelot_compatible_same_key() { assert!(keys_compatible("8A", "8A")); }
    #[test]
    fn test_camelot_compatible_plus_one() { assert!(keys_compatible("8A", "9A")); }
    #[test]
    fn test_camelot_compatible_relative() { assert!(keys_compatible("8A", "8B")); }
    #[test]
    fn test_camelot_incompatible() { assert!(!keys_compatible("8A", "3B")); }
    #[test]
    fn test_transition_smooth() { assert_eq!(transition_type(174.0, 175.0), TransitionType::Smooth); }
    #[test]
    fn test_transition_bridge() { assert_eq!(transition_type(174.0, 130.0), TransitionType::Bridge); }

    use proptest::prelude::*;

    fn arb_camelot_key() -> impl Strategy<Value = String> {
        (1u8..=12, prop::bool::ANY).prop_map(|(n, minor)| {
            format!("{}{}", n, if minor { "A" } else { "B" })
        })
    }

    // Validates: Requirements 13.1, 13.2, 13.3, 13.4, 13.5
    // Property 11: Camelot key compatibility
    proptest! {
        /// Reflexivity: keys_compatible(k, k) == true
        #[test]
        fn prop_camelot_reflexive(key in arb_camelot_key()) {
            prop_assert!(keys_compatible(&key, &key),
                "keys_compatible({0}, {0}) should be true", key);
        }

        /// Relative major/minor: keys_compatible(nA, nB) == true
        #[test]
        fn prop_camelot_relative_major_minor(n in 1u8..=12) {
            let a = format!("{}A", n);
            let b = format!("{}B", n);
            prop_assert!(keys_compatible(&a, &b),
                "keys_compatible({}, {}) should be true (relative major/minor)", a, b);
            prop_assert!(keys_compatible(&b, &a),
                "keys_compatible({}, {}) should be true (relative major/minor)", b, a);
        }

        /// Adjacent ±1 same mode: compatible (including 12→1 wrap)
        #[test]
        fn prop_camelot_adjacent_same_mode(n in 1u8..=12, minor in prop::bool::ANY) {
            let letter = if minor { "A" } else { "B" };
            let key = format!("{}{}", n, letter);
            let next_n = if n == 12 { 1 } else { n + 1 };
            let prev_n = if n == 1 { 12 } else { n - 1 };
            let next = format!("{}{}", next_n, letter);
            let prev = format!("{}{}", prev_n, letter);
            prop_assert!(keys_compatible(&key, &next),
                "keys_compatible({}, {}) should be true (adjacent +1)", key, next);
            prop_assert!(keys_compatible(&key, &prev),
                "keys_compatible({}, {}) should be true (adjacent -1)", key, prev);
        }

        /// Distant keys: incompatible (same mode, diff > 1 and not wrapping neighbor)
        #[test]
        fn prop_camelot_distant_incompatible(
            n1 in 1u8..=12,
            offset in 2u8..=5,
            minor in prop::bool::ANY,
        ) {
            let letter = if minor { "A" } else { "B" };
            let n2 = ((n1 as u16 - 1 + offset as u16) % 12 + 1) as u8;
            // Ensure the distance is truly > 1 in both directions on the wheel
            let diff_fwd = ((n2 as i16 - n1 as i16).rem_euclid(12)) as u8;
            let diff_bwd = ((n1 as i16 - n2 as i16).rem_euclid(12)) as u8;
            let min_dist = diff_fwd.min(diff_bwd);
            // Only assert incompatible when min circular distance > 1
            prop_assume!(min_dist > 1);
            let key_a = format!("{}{}", n1, letter);
            let key_b = format!("{}{}", n2, letter);
            prop_assert!(!keys_compatible(&key_a, &key_b),
                "keys_compatible({}, {}) should be false (distant, min_dist={})", key_a, key_b, min_dist);
        }

        /// Invalid strings: return false (ASCII-only; parse_camelot panics on multi-byte split_at)
        #[test]
        fn prop_camelot_invalid_returns_false(s in "[c-zC-Z!@#$%^&*() ]{0,10}") {
            prop_assert!(!keys_compatible(&s, "8A"),
                "keys_compatible('{}', '8A') should be false (invalid key)", s);
            prop_assert!(!keys_compatible("8A", &s),
                "keys_compatible('8A', '{}') should be false (invalid key)", s);
        }
    }
}
