//! The seed console: the operator's own debug bench — `seed`, `reseed <hex>`
//! (accepts the vixi birth interview's 0x-hex output), `worlds <n>` or `worlds <starname>`.
//! Numbers are allowed HERE only. Skeleton pre-created by the conductor; Weld C fills it.

use forge_core_v3::ramus_prime::MortonKey5D;
use forge_core_v3::sky;
use forge_core_v3::sprite_blob::u64_to_nistam;

use crate::consequence::{self, FACTIONS};
use crate::operator::{seed_hash, Operator};
use crate::weather::Era;
use crate::world;

/// Find a star by name (case-insensitive). Returns Some(index) if found, None otherwise.
pub fn find_star_by_name(name: &str) -> Option<u8> {
    let name_lower = name.to_lowercase();
    for (idx, star) in sky::CATALOG.iter().enumerate() {
        if star.name.to_lowercase() == name_lower {
            return Some(idx as u8);
        }
    }
    None
}

/// Deal the era the same way [`crate::game::Game::weather_for`] does — the
/// formula lives twice on purpose (L06: console.rs is a pure-fn bench with
/// no `Game` dependency; the conductor owns the single weather re-deal).
fn era_of(seed: u64) -> Era {
    Era::all()[(seed_hash(&[&u64_to_nistam(seed), b"era"]) % 4) as usize]
}

/// The town's watch, spoken as sensation (matches `Game::look`'s ladder) —
/// the debug console still names the number beside it (the one exemption).
fn watch_word(law: u8) -> &'static str {
    match law {
        0..=9 => "no law walks these lanes",
        10..=39 => "the watch is thin here",
        40..=79 => "the watch keeps its rounds",
        _ => "the watch stands like a drawn blade",
    }
}

/// Parse a `reseed` argument into a seed. Total — never panics.
///
/// * `0x`/`0X` + 1..=16 hex digits parses as hex (the vixi birth interview's
///   own contract: `0x` + 8 lowercase hex from its FNV-1a strike).
/// * A bare 1..=16 hex digits string parses as hex.
/// * Anything else hashes the lowercased word — `reseed thornhaven` deals
///   thornhaven's world, deterministically, forever.
pub fn parse_seed(arg: &str) -> u64 {
    let s = arg.trim();
    let hex_body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"));
    let candidate = hex_body.unwrap_or(s);
    let is_hex_len = !candidate.is_empty() && candidate.len() <= 16;
    if is_hex_len && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(v) = u64::from_str_radix(candidate, 16) {
            return v;
        }
    }
    seed_hash(&[s.to_lowercase().as_bytes()])
}

/// Derive a new seed from the old one and a salt — the argless `reseed`'s
/// dealer (the conductor passes xp as salt, so the roll is still
/// deterministic from earned state, never from the clock).
pub fn derive_seed(old_seed: u64, salt: u64) -> u64 {
    seed_hash(&[&u64_to_nistam(old_seed), &u64_to_nistam(salt), b"reseed"])
}

/// The `seed` verb's multi-line deal summary: everything the node wears,
/// dealt from the seed the same way every other face of the node is.
pub fn seed_summary(seed: u64) -> String {
    let town = world::town_lore(seed);
    let lore_first = town.1.split_whitespace().next().unwrap_or("");
    let law = consequence::law_level(seed);
    let fac = consequence::town_faction(seed);
    let era = era_of(seed);
    let theme = world::theme(seed);
    format!(
        "node {:016x}\r\ntown {} — {}\r\nlaw {} ({})\r\nfaction {}\r\nera {}\r\nsky {} · vibe {} · hue {}",
        seed,
        town.0,
        lore_first,
        law,
        watch_word(law),
        FACTIONS[fac].name,
        era.name(),
        theme.skybox.0,
        theme.vibe.0,
        theme.hue,
    )
}

/// List all 16 star names, one per line — no digits in the transcript.
pub fn worlds_list_stars() -> String {
    let mut out = String::new();
    for star in &sky::CATALOG {
        out.push_str(&format!("{}\r\n", star.name));
    }
    out
}

/// The `worlds <n>` verb: n clamped 1..=13 preview lines, each a derived
/// seed one-liner — a peek at the nodes a reseed chain would walk.
pub fn worlds_preview(seed: u64, n: usize) -> String {
    let n = n.clamp(1, 13);
    let mut out = String::new();
    for i in 0..n {
        let s = derive_seed(seed, i as u64);
        let town = world::town_lore(s);
        let law = consequence::law_level(s);
        let fac = consequence::town_faction(s);
        let era = era_of(s);
        out.push_str(&format!(
            "#{i} <{s:016x}> town {} law {law} faction {} era {}\r\n",
            town.0,
            FACTIONS[fac].name,
            era.name(),
        ));
    }
    out
}

/// Reseed via star name — returns the new seed if name matches, None otherwise.
pub fn try_reseed_star(name: &str) -> Option<u64> {
    find_star_by_name(name).map(|idx| sky::natal_seed(idx))
}

/// Apply a reseed to a live operator: new node, rehomed to the new town
/// square (z/t/s zeroed), heat cleared (a new node holds no warrant —
/// matches [`Operator::die`]'s semantics). XP, deaths, name, birthday,
/// deeds, bias and standings ride through untouched: this is the debug
/// verb, distinct from death, and it does not count as one.
pub fn apply_reseed(op: &mut Operator, new_seed: u64) {
    op.node_seed = new_seed;
    let (tx, ty) = world::town_square(new_seed);
    op.pos = MortonKey5D::encode([tx, ty, 0, 0, 0]);
    op.heat = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser's three lanes: prefixed hex, bare hex, and the word-hash
    /// fallback — case-folded, never a hex parse, total on the empty string.
    #[test]
    fn parse_seed_handles_hex_prefixes_and_words() {
        assert_eq!(parse_seed("0xdeadbeef"), 0xdeadbeef_u64);
        assert_eq!(parse_seed("0x00c0ffee"), 0x00c0ffee_u64, "leading zeros must survive");
        assert_eq!(parse_seed("DEADBEEF"), 0xDEADBEEF_u64, "bare hex works");
        assert_eq!(
            parse_seed("thornhaven"),
            parse_seed("THORNHAVEN"),
            "the word hash is case-folded"
        );
        assert_eq!(
            parse_seed("thornhaven"),
            seed_hash(&[b"thornhaven"]),
            "a non-hex word hashes, never parses"
        );
        assert_ne!(parse_seed("thornhaven"), 0xDEADBEEF_u64);
        let _ = parse_seed(""); // must not panic — total over the empty string
    }

    /// The vixi contract: every `0x` + 8 lowercase hex the birth interview
    /// can strike round-trips through `parse_seed` to the identical u32,
    /// sampled across 32 spread values (deterministic, no RNG dependency).
    #[test]
    fn vixi_contract_round_trips_32_sampled_u32s() {
        for i in 0u32..32 {
            let v = i.wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9);
            let s = format!("0x{v:08x}");
            assert_eq!(parse_seed(&s), v as u64, "vixi seed {s} did not round-trip");
        }
    }

    /// Reseeding twice from the same start with the same hex lands identical
    /// operator state, and the same seed's summary is the same string.
    #[test]
    fn reseed_determinism_and_summary_repeat() {
        let base = Operator::birth("Operator", 3, 12).unwrap();
        let mut a = base.clone();
        let mut b = base;
        let seed = parse_seed("0xdeadbeef");
        apply_reseed(&mut a, seed);
        apply_reseed(&mut b, seed);
        assert_eq!(a, b);
        assert_eq!(seed_summary(seed), seed_summary(seed));
    }

    /// XP and deaths survive a reseed, heat clears (a new node holds no
    /// warrant), and the operator lands on the new node's own town square.
    #[test]
    fn reseed_keeps_earned_state_clears_heat_and_rehomes() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.xp = 500;
        op.deaths = 2;
        op.heat = 40;
        let name = op.name.clone();
        let seed = parse_seed("thornhaven");
        apply_reseed(&mut op, seed);
        assert_eq!(op.xp, 500, "xp must survive a reseed");
        assert_eq!(op.deaths, 2, "deaths must survive a reseed");
        assert_eq!(op.heat, 0, "a new node holds no warrant");
        assert_eq!(op.name, name);
        assert_eq!(
            (op.pos.axes()[0], op.pos.axes()[1]),
            world::town_square(seed),
            "the operator must rehome to the new town"
        );
    }

    /// `worlds <n>` clamps 1..=13 and deals the same lines for the same
    /// input every time.
    #[test]
    fn worlds_preview_clamps_and_is_deterministic() {
        let seed = 0xDEAD_BEEF_u64;
        assert_eq!(worlds_preview(seed, 0).lines().count(), 1, "n=0 must clamp to 1");
        assert_eq!(worlds_preview(seed, 99).lines().count(), 13, "n=99 must clamp to 13");
        assert_eq!(worlds_preview(seed, 5), worlds_preview(seed, 5));
    }

    /// Reseed via star name twice yields identical seed and identical summaries.
    #[test]
    fn reseed_via_star_name_is_deterministic() {
        let base = Operator::birth("Operator", 3, 12).unwrap();
        let mut a = base.clone();
        let mut b = base;

        // Reseed via star name "Sirius" twice
        let sirius_seed = try_reseed_star("Sirius").expect("Sirius must be in catalog");
        apply_reseed(&mut a, sirius_seed);
        apply_reseed(&mut b, sirius_seed);

        // Both operators must be identical after reseeding to the same star
        assert_eq!(a, b, "Reseeding twice to the same star must yield identical operators");

        // The seed summary must also be identical
        let summary_a = seed_summary(sirius_seed);
        let summary_b = seed_summary(sirius_seed);
        assert_eq!(summary_a, summary_b, "Seed summary must be deterministic");

        // Case-insensitive: "SIRIUS" should also work
        let sirius_upper = try_reseed_star("SIRIUS");
        assert_eq!(
            sirius_upper, Some(sirius_seed),
            "Star name matching must be case-insensitive"
        );
    }

    /// Star names list contains all 16 stars with no numeric digits in the output.
    #[test]
    fn worlds_list_stars_has_no_digits() {
        let list = worlds_list_stars();
        let lines: Vec<&str> = list.lines().collect();
        assert_eq!(lines.len(), 16, "worlds_list_stars must output 16 star names");

        // No line should contain a digit (L20: no digits in transcript prose)
        for line in &lines {
            for c in line.chars() {
                assert!(
                    !c.is_ascii_digit(),
                    "Star name '{}' must not contain digits",
                    line
                );
            }
        }
    }
}
