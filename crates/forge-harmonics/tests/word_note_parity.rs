//! The gate the tree never had: one word, one degree, twenty-four keys.
//! Pins the DEFAULT_8B shim as an exact identity so every landed
//! `word_note` assert site keeps its meaning after the key was threaded through.

use forge_harmonics::{
    answer_melody, answer_melody_in_key, word_degree, word_note, word_note_in_key, CamelotKey,
    PENTATONIC_C,
};

/// Every Camelot key on the wheel, 1A..12B.
fn all_keys() -> Vec<CamelotKey> {
    let mut keys = Vec::with_capacity(24);
    for number in 1..=12u8 {
        keys.push(CamelotKey::new(number, true));
        keys.push(CamelotKey::new(number, false));
    }
    keys
}

/// The deck the landed asserts already use, plus the canon's own shapes.
const DECK: [&[u8]; 9] = [
    b"cargo",
    b"test",
    b"RED",
    b"x",
    b"forge_core_v3",
    b"0",
    b"hello",
    b"harvest",
    b"sovereignty",
];

/// `PENTATONIC_C` was never a musical fact — it is 8B's own scale, frozen.
#[test]
fn pentatonic_c_is_just_8b_spelled_out() {
    assert_eq!(
        CamelotKey::DEFAULT_8B.pentatonic_span_7(0),
        PENTATONIC_C,
        "the C-major literal must be recoverable from the key type alone"
    );
}

/// The shim is an exact identity, so the eight landed `word_note` assert sites
/// across forge-mud-v3 and xtask keep passing unchanged.
#[test]
fn the_default_8b_shim_is_an_identity() {
    for w in DECK {
        assert_eq!(
            word_note(w),
            word_note_in_key(w, CamelotKey::DEFAULT_8B),
            "word_note must be exactly its own DEFAULT_8B case: {:?}",
            core::str::from_utf8(w).unwrap_or("?")
        );
    }
    assert_eq!(
        answer_melody("the sky remembers"),
        answer_melody_in_key("the sky remembers", CamelotKey::DEFAULT_8B),
    );
}

/// No wrong note is possible in ANY key — the old law, widened from one to 24.
#[test]
fn no_wrong_note_is_possible_in_any_key() {
    for key in all_keys() {
        let span = key.pentatonic_span_7(0);
        for w in DECK {
            let note = word_note_in_key(w, key);
            assert!(
                span.contains(&note),
                "{:?} sang {note} outside {}{}'s span {span:?}",
                core::str::from_utf8(w).unwrap_or("?"),
                key.number,
                if key.is_minor { "A" } else { "B" },
            );
        }
    }
}

/// THE NEW LAW: same word, same degree, whatever key you were born into.
/// Only the pitch moves.
#[test]
fn a_word_picks_the_same_degree_in_every_key() {
    for w in DECK {
        let degree = word_degree(w);
        for key in all_keys() {
            let note = word_note_in_key(w, key);
            let span = key.pentatonic_span_7(0);
            assert_eq!(
                note, span[degree],
                "{:?} must hold degree {degree} in {}{}",
                core::str::from_utf8(w).unwrap_or("?"),
                key.number,
                if key.is_minor { "A" } else { "B" },
            );
        }
    }
}

/// The pitch really does move — a keyed singer is not a decorated one.
#[test]
fn different_keys_actually_sound_different() {
    let sirius = CamelotKey::from_star_idx(0).expect("Sirius");
    let procyon = CamelotKey::from_star_idx(6).expect("Procyon");
    let differing = DECK
        .iter()
        .filter(|w| word_note_in_key(w, sirius) != word_note_in_key(w, procyon))
        .count();
    assert!(
        differing > 0,
        "two different natal stars must not sing the same pitches"
    );
}
