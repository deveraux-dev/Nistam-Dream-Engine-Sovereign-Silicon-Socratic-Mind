//! The Rosetta chain gate — bible 003: one word, deterministic projections, no drift between crates.

use forge_mud_v3::cdk::word_world_line;
use forge_harmonics::{word_note, PENTATONIC_C};
use forge_sieve_v3::prime_seed::prime_seed;

/// Test that word_note, prime_seed, and word_world_line are all deterministic:
/// calling them twice with the same input yields identical results.
#[test]
fn the_chain_is_deterministic() {
    let words = ["thorn", "bell", "ash"];

    for word in &words {
        // word_note is deterministic
        let note1 = word_note(word.as_bytes());
        let note2 = word_note(word.as_bytes());
        assert_eq!(note1, note2, "word_note must be deterministic for word '{}'", word);

        // prime_seed is deterministic
        let seed1 = prime_seed(word, 64);
        let seed2 = prime_seed(word, 64);
        assert_eq!(seed1, seed2, "prime_seed must be deterministic for word '{}'", word);

        // word_world_line is deterministic
        let line1 = word_world_line(word);
        let line2 = word_world_line(word);
        assert_eq!(line1, line2, "word_world_line must be deterministic for word '{}'", word);
    }
}

/// Test that word_note always projects into the PENTATONIC_C scale.
/// This holds for both the reference words and a generated set of 100 words.
#[test]
fn the_note_is_always_in_scale() {
    let mut words: Vec<&str> = vec!["thorn", "bell", "ash"];

    // Extend with 100 generated words
    let generated: Vec<String> = (0..100)
        .map(|i| format!("w{}", i))
        .collect();
    let generated_refs: Vec<&str> = generated.iter().map(|s| s.as_str()).collect();
    words.extend(generated_refs);

    for word in &words {
        let note = word_note(word.as_bytes());
        assert!(
            PENTATONIC_C.contains(&note),
            "word_note({}) = {} must be in PENTATONIC_C {:?}",
            word,
            note,
            PENTATONIC_C
        );
    }
}

/// Test that word_world_line output contains the word_note result formatted as a note line.
/// The line must carry the note value so the word's colour halo and pitch lock are visible.
#[test]
fn the_line_carries_its_own_note() {
    let words = ["thorn", "bell", "ash"];

    for word in &words {
        let note = word_note(word.as_bytes());
        let line = word_world_line(word);

        // The line must contain "note {note}" somewhere in its text
        let expected_note_text = format!("note {}", note);
        assert!(
            line.contains(&expected_note_text),
            "word_world_line for '{}' must contain '{}', but got: {}",
            word,
            expected_note_text,
            line
        );
    }
}
