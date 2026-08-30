//! Interval — musical interval math over semitones (toward the MIDI page). Names
//! and inversion; integer only.

/// The interval name for `semitones` (12 = octave; larger wraps by octave).
pub fn name(semitones: u8) -> &'static str {
    if semitones == 12 {
        return "octave";
    }
    match semitones % 12 {
        0 => "unison",
        1 => "minor 2nd",
        2 => "major 2nd",
        3 => "minor 3rd",
        4 => "major 3rd",
        5 => "perfect 4th",
        6 => "tritone",
        7 => "perfect 5th",
        8 => "minor 6th",
        9 => "major 6th",
        10 => "minor 7th",
        _ => "major 7th",
    }
}

/// The inversion of an interval within the octave.
pub fn invert(semitones: u8) -> u8 {
    (12 - semitones % 12) % 12
}

/// Is the interval consonant (unison/3rds/4th/5th/6ths/octave)?
pub fn is_consonant(semitones: u8) -> bool {
    matches!(semitones % 12, 0 | 3 | 4 | 5 | 7 | 8 | 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_octave() {
        assert_eq!(name(0), "unison");
        assert_eq!(name(7), "perfect 5th");
        assert_eq!(name(12), "octave");
        assert_eq!(name(14), "major 2nd"); // wraps
    }

    #[test]
    fn inversion_is_complementary() {
        assert_eq!(invert(5), 7); // 4th inverts to 5th
        assert_eq!(invert(0), 0);
    }

    #[test]
    fn consonance_classifies() {
        assert!(is_consonant(7)); // 5th
        assert!(!is_consonant(1)); // minor 2nd
        assert!(!is_consonant(6)); // tritone
    }
}
