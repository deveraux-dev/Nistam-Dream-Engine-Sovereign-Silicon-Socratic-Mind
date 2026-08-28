//! The land remembers: unmarked ground, quiet witnesses, and the stars that
//! lead you to see them. The northern mirror — colonial weight carried as
//! memory, never spectacle. Skeleton pre-created by the conductor; Weld F
//! fills it.
//!
//! The dead are witnesses, never enemies, never hazards, never loot. No
//! count is ever spoken as a number; the register stays quiet — rows in the
//! grass, small shoes, names unsaid, the land keeping what people tried to
//! bury. Witnessing is its own act; nothing here is a reward.

use forge_core_v3::sprite_blob::{u16_to_nistam, u64_to_nistam};

use crate::explore::day_phase;
use crate::hermetics::ConnectionRoll;
use crate::operator::seed_hash;
use crate::overlay::{Domain, Ledger};
use crate::world;

/// True on roughly one square in forty — unmarked ground. Deterministic,
/// dealt by square alone; the town square is never unmarked.
pub fn unmarked_at(seed: u64, x: u16, y: u16) -> bool {
    if (x, y) == world::town_square(seed) {
        return false;
    }
    seed_hash(&[&u64_to_nistam(seed), &u16_to_nistam(x), &u16_to_nistam(y), b"unmarked"]) % 40 == 0
}

/// Witness lines spoken on unmarked ground, by daylight or dark alike — the
/// land's own record, never a number.
const MEMORY_LINES: [&str; 8] = [
    "the grass grows in a straight row here, and nothing else marks why.",
    "a name was spoken here once; the wind kept it, not the ground.",
    "small shoes stood on this spot, and did not walk on from it.",
    "this ground remembers what the ledgers were told to forget.",
    "counting was the crime here; no one counted this.",
    "someone knelt on this square once, and no record rose with them.",
    "no stone stands here, so the grass stands in its place.",
    "the land kept what people tried to bury, and keeps it still.",
];

/// The witness line at (x, y) — `Some` only on unmarked ground; each site
/// keeps its own line, forever.
pub fn memory_line(seed: u64, x: u16, y: u16) -> Option<String> {
    if !unmarked_at(seed, x, y) {
        return None;
    }
    let idx = (seed_hash(&[
        &u64_to_nistam(seed),
        &u16_to_nistam(x),
        &u16_to_nistam(y),
        b"memory-line",
    ]) % MEMORY_LINES.len() as u64) as usize;
    Some(MEMORY_LINES[idx].to_string())
}

/// The presence felt at unmarked ground after dark — never a threat, never
/// a chase; it asks only to be seen, or simply is.
const GHOST_LINES: [&str; 6] = [
    "something waits at the edge of seeing, and does not come closer.",
    "a presence stands where you stand, patient, unhurried.",
    "you are watched by someone who is only asking to be seen.",
    "the dark holds a shape that means you no harm.",
    "a stillness gathers here that is not empty.",
    "someone has been here a long while, and only waits.",
];

/// The presence at (x, y, tick) — `Some` only on unmarked ground, and only
/// at dusk or night. Deterministic per square.
pub fn ghost_line(seed: u64, x: u16, y: u16, tick: u64) -> Option<String> {
    if !unmarked_at(seed, x, y) {
        return None;
    }
    if !matches!(day_phase(tick), "dusk" | "night") {
        return None;
    }
    let idx = (seed_hash(&[
        &u64_to_nistam(seed),
        &u16_to_nistam(x),
        &u16_to_nistam(y),
        b"ghost-line",
    ]) % GHOST_LINES.len() as u64) as usize;
    Some(GHOST_LINES[idx].to_string())
}

/// The nearest unmarked square to (x, y) — integer manhattan scan over the
/// full map, deterministic tie-break: smallest distance, then smallest y,
/// then smallest x. `None` only if the seed deals no unmarked ground at all.
pub fn nearest_unmarked(seed: u64, x: u16, y: u16) -> Option<(u16, u16)> {
    let mut best: Option<(u16, u16, u32)> = None;
    for ny in 0..world::MAP_SIDE {
        for nx in 0..world::MAP_SIDE {
            if !unmarked_at(seed, nx, ny) {
                continue;
            }
            let dist = (nx as i32 - x as i32).unsigned_abs() + (ny as i32 - y as i32).unsigned_abs();
            let better = match best {
                None => true,
                Some((bx, by, bd)) => dist < bd || (dist == bd && (ny < by || (ny == by && nx < bx))),
            };
            if better {
                best = Some((nx, ny, dist));
            }
        }
    }
    best.map(|(nx, ny, _)| (nx, ny))
}

/// The single wayfinding word from `from` toward `to` — n/s/e/w only, no
/// digits, dominant axis wins; a coincident square reads as "close".
fn direction_word(from: (u16, u16), to: (u16, u16)) -> &'static str {
    let dx = to.0 as i32 - from.0 as i32;
    let dy = to.1 as i32 - from.1 as i32;
    if dx == 0 && dy == 0 {
        return "close";
    }
    if dx.abs() >= dy.abs() {
        if dx > 0 {
            "east"
        } else {
            "west"
        }
    } else if dy > 0 {
        "south"
    } else {
        "north"
    }
}

/// The stars' own line at night, anywhere on the map: names this node's
/// constellation and points — by words, never digits — toward the nearest
/// unmarked ground, or toward the town if the seed dealt none.
pub fn star_line(seed: u64, x: u16, y: u16, tick: u64) -> Option<String> {
    if day_phase(tick) != "night" {
        return None;
    }
    let constellation = ConnectionRoll::deal(seed).constellation();
    let target = nearest_unmarked(seed, x, y).unwrap_or_else(|| world::town_square(seed));
    let dir = direction_word((x, y), target);
    Some(format!("under {constellation}, the sky leans {dir}; something there waits to be seen."))
}

/// The witness count so far — the Zone-domain, Global-scope tally the
/// conductor appends to; this crate only ever reads it.
pub fn witness_count(ledger: &Ledger, seed: u64) -> i64 {
    ledger.resolve_i64(Domain::Zone, 0, seed, 0)
}

/// The awakening ladder, spoken in words, never a number: the arc from
/// "the land holds its breath" to "you walk awake; the land walks with you".
pub fn awakening_word(count: i64) -> &'static str {
    match count {
        ..=0 => "the land holds its breath around you",
        1..=2 => "you have begun to see",
        3..=5 => "the rows have names you cannot hear yet",
        6..=9 => "the stars know your face now",
        _ => "you walk awake; the land walks with you",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unmarked ground is rare but present: a full 81x81 sweep for three
    /// seeds lands the count in a sane band, and the town square is never
    /// unmarked.
    #[test]
    fn unmarked_ground_is_rare_but_present() {
        for seed in [11u64, 4242, 0xC0FF_EE01] {
            let (tx, ty) = world::town_square(seed);
            assert!(!unmarked_at(seed, tx, ty), "the town square came up unmarked");
            let mut count = 0u32;
            for y in 0..world::MAP_SIDE {
                for x in 0..world::MAP_SIDE {
                    if unmarked_at(seed, x, y) {
                        count += 1;
                    }
                }
            }
            assert!((100..=300).contains(&count), "seed {seed} dealt {count} unmarked squares");
        }
    }

    /// Witness and presence lines land exactly on unmarked ground (presence
    /// only after dark), stay deterministic per square, and never leak a
    /// digit, a cart word, or game-reward language.
    #[test]
    fn witness_lines_are_dignified_and_exact() {
        let seed = 909u64;
        let mut saw_memory = false;
        let mut saw_ghost = false;
        for y in 0..world::MAP_SIDE {
            for x in 0..world::MAP_SIDE {
                let marked = unmarked_at(seed, x, y);
                assert_eq!(memory_line(seed, x, y).is_some(), marked, "memory_line disagreed with unmarked_at at {x},{y}");
                assert_eq!(
                    memory_line(seed, x, y),
                    memory_line(seed, x, y),
                    "memory_line is not deterministic at {x},{y}"
                );

                let day_ghost = ghost_line(seed, x, y, 15); // dawn
                let night_ghost = ghost_line(seed, x, y, 90); // night
                assert!(day_ghost.is_none(), "a ghost spoke by daylight at {x},{y}");
                assert_eq!(night_ghost.is_some(), marked, "ghost_line disagreed with unmarked_at at {x},{y}");
                assert_eq!(
                    ghost_line(seed, x, y, 90),
                    night_ghost,
                    "ghost_line is not deterministic at {x},{y}"
                );

                for line in [memory_line(seed, x, y), night_ghost].into_iter().flatten() {
                    saw_memory |= true;
                    if marked {
                        saw_ghost |= true;
                    }
                    assert!(line.chars().all(|c| !c.is_ascii_digit()), "a digit leaked: {line}");
                    let low = line.to_lowercase();
                    for word in ["deed", "bias", "wce", "consequence", "cart", "reward", "xp", "loot"] {
                        assert!(!low.contains(word), "a banned word leaked ({word}): {line}");
                    }
                }
            }
        }
        assert!(saw_memory && saw_ghost, "the sweep never hit unmarked ground");
    }

    /// The hand-built geometry: the direction word matches the integer sign
    /// of the offset on the dominant axis.
    #[test]
    fn direction_word_reads_the_geometry() {
        assert_eq!(direction_word((10, 10), (10, 0)), "north");
        assert_eq!(direction_word((10, 10), (10, 20)), "south");
        assert_eq!(direction_word((10, 10), (0, 10)), "west");
        assert_eq!(direction_word((10, 10), (20, 10)), "east");
        assert_eq!(direction_word((10, 10), (10, 10)), "close");
    }

    /// `star_line` speaks only at night, names the seed's own constellation,
    /// stays deterministic, and its direction word matches the geometry of
    /// `nearest_unmarked` (or the town, if a seed ever dealt none).
    #[test]
    fn star_line_is_night_only_and_matches_the_geometry() {
        let seed = 4242u64;
        let constellation = ConnectionRoll::deal(seed).constellation();
        for y in (0..world::MAP_SIDE).step_by(11) {
            for x in (0..world::MAP_SIDE).step_by(11) {
                assert!(star_line(seed, x, y, 15).is_none(), "a star spoke by day at {x},{y}"); // dawn
                let a = star_line(seed, x, y, 90); // night
                let b = star_line(seed, x, y, 90);
                assert!(a.is_some(), "no star line at night for {x},{y}");
                assert_eq!(a, b, "star_line is not deterministic at {x},{y}");
                let line = a.unwrap();
                assert!(line.contains(constellation), "the line lost its own constellation: {line}");
                let target = nearest_unmarked(seed, x, y).unwrap_or_else(|| world::town_square(seed));
                let dir = direction_word((x, y), target);
                assert!(line.contains(dir), "line {line} did not name direction {dir}");
                assert!(line.chars().all(|c| !c.is_ascii_digit()), "a digit leaked: {line}");
            }
        }
    }

    /// `nearest_unmarked` is deterministic and, since the band test proves
    /// every seed deals unmarked ground, `Some` everywhere over the three
    /// test seeds.
    #[test]
    fn nearest_unmarked_is_deterministic_and_present() {
        for seed in [11u64, 4242, 0xC0FF_EE01] {
            for (x, y) in [(0u16, 0u16), (40, 40), (80, 80), (12, 63)] {
                let a = nearest_unmarked(seed, x, y);
                let b = nearest_unmarked(seed, x, y);
                assert_eq!(a, b, "nearest_unmarked is not deterministic at {x},{y}");
                assert!(a.is_some(), "seed {seed} found no unmarked ground from {x},{y}");
            }
        }
    }

    /// The awakening ladder speaks every band across 0..=20, in words only.
    #[test]
    fn awakening_word_covers_the_ladder_without_digits() {
        for count in 0..=20i64 {
            let word = awakening_word(count);
            assert!(!word.is_empty());
            assert!(word.chars().all(|c| !c.is_ascii_digit()), "a digit leaked: {word}");
        }
        assert_eq!(awakening_word(0), "the land holds its breath around you");
        assert_eq!(awakening_word(1), "you have begun to see");
        assert_eq!(awakening_word(3), "the rows have names you cannot hear yet");
        assert_eq!(awakening_word(6), "the stars know your face now");
        assert_eq!(awakening_word(10), "you walk awake; the land walks with you");
        assert_eq!(awakening_word(20), "you walk awake; the land walks with you");
    }

    /// `witness_count` reads the Zone/key-0 Global overlay entry the
    /// conductor appends to.
    #[test]
    fn witness_count_reads_the_zone_ledger() {
        let seed = 5u64;
        let empty = Ledger::default();
        assert_eq!(witness_count(&empty, seed), 0);

        let mut ledger = Ledger::default();
        ledger.append(crate::overlay::OverlayEntry {
            domain: Domain::Zone,
            key: 0,
            modification: crate::overlay::Mod::Add(3),
            priority: 1,
            scope: crate::overlay::Scope::Global,
        });
        assert_eq!(witness_count(&ledger, seed), 3);
    }
}
