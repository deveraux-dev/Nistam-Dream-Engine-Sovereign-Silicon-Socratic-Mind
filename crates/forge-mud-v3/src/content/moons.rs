//! The thirteen moons, drained from forge-insights `rpg/moon.rs:10-70`
//! (`Moon` enum, `.name()`/`.short()`) — wired in `game::Game::status` as
//! the CURRENT overhead moon, riding `Operator::xp` as the beat: xp is
//! documented (operator.rs:88-89) as monotonic "terminal bytes earned",
//! the real beat counter `Moon::on_beat(beat)` wants, not a stand-in.

/// Beats one moon holds before handing over — verbatim from
/// `rpg/moon.rs:36` (`BEATS_PER_MOON`), so the calendar turns at the same
/// cadence the drain source used.
pub const BEATS_PER_MOON: u64 = 2_048;

/// The thirteen moons: (name, short form), in the source's own `ALL` order.
pub const MOONS: &[(&str, &str)] = &[
    ("The Moon that Held its Breath", "Held Breath"),
    ("The Moon of Swallowed Stones", "Swallowed Stones"),
    ("The Keepmoon", "Keepmoon"),
    ("Yesterday's New Moon", "Yesterday's New"),
    ("The Empty Moon", "Empty"),
    ("The Moon that Had No More", "No More"),
    ("The Backwards New Moon", "Backwards New"),
    ("The Hearthmoon", "Hearthmoon"),
    ("The Tidemoon", "Tidemoon"),
    ("The Hungry Moon of Self", "Hungry Self"),
    ("The Ambiguous Dark Moon", "Ambiguous Dark"),
    ("The Cairnmoon", "Cairnmoon"),
    ("The Ghost Moon", "Ghost"),
];

#[cfg(test)]
mod tests {
    #[test]
    fn moons_are_exactly_thirteen_and_word_only() {
        assert_eq!(super::MOONS.len(), 13);
        for (name, short) in super::MOONS {
            assert!(!name.is_empty() && name.is_ascii());
            assert!(!short.is_empty() && short.is_ascii());
            assert!(!name.chars().any(|c| c.is_ascii_digit()));
            assert!(!short.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
