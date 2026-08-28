//! The walk itself: tick-dealt exploration options (go / climb / camp) spoken
//! as sensation, dealt from seed + square + sky. Skeleton pre-created by the
//! conductor; Weld B fills it.
//!
//! Pure functions only — no [`crate::game::Game`] dependency; the conductor
//! wires dispatch. Every line speaks words, never the digits or the cart
//! words (deed/bias/wce/consequence/cart) that built it.

use forge_core_v3::sprite_blob::{u16_to_nistam, u64_to_nistam};

use crate::operator::{seed_hash, BIAS_NONE};
use crate::weather::{Era, Sky, Weather, WeatherModel};
use crate::world;

/// Standing height: flat ground.
pub const GROUND: u16 = 0;
/// Standing height: a ridge.
pub const RIDGE: u16 = 1;
/// Standing height: a peak.
pub const PEAK: u16 = 2;

/// The land's standing height at a square — what `climb` can reach here.
/// Deterministic, integer, dealt from the seed alone (never the operator's
/// current `z`).
pub fn height_at(seed: u64, x: u16, y: u16) -> u16 {
    (seed_hash(&[&u64_to_nistam(seed), &u16_to_nistam(x), &u16_to_nistam(y), b"height"]) % 3)
        as u16
}

/// One offer on the walk's menu: the exact verb that takes it, and the
/// sensation line that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The exact input that takes this offer (`"n"`, `"climb"`, `"camp"`...).
    pub verb: &'static str,
    /// The sensation line shown on the menu — words only, never digits.
    pub line: String,
}

/// The invitation a neighbouring biome speaks, named but never numbered.
fn biome_invite(name: &str) -> &'static str {
    match name {
        "prairie" => "grass runs open and pale toward the horizon",
        "forest" => "the pines thin toward open light",
        "swamp" => "the reeds thicken and the water rises",
        "dungeon" => "cut stone breathes cold air upward",
        "lake" => "still water reaches wide and blue",
        "bonefield" => "bleached ground rattles underfoot",
        "frostfen" => "frost rimes the reeds white",
        "forgeheart" => "the air tastes of hot iron",
        _ => "the land ahead holds its shape close",
    }
}

/// The sensation of a climb or descent — the peak's line differs from the
/// ridge's, and the era colours the light.
pub fn climb_line(era: Era, up: bool, z_after: u16) -> String {
    let light = match era {
        Era::Ancient => "faded light",
        Era::Golden => "golden light",
        Era::Decay => "failing light",
        Era::Void => "lightless dark",
    };
    if up {
        match z_after {
            PEAK => format!("the last stone gives way to sky; the peak stands bare under {light}."),
            RIDGE => format!("the ground steepens; the ridge opens under {light}."),
            _ => format!("you climb, and the land does not rise further; {light} holds the ground."),
        }
    } else {
        match z_after {
            GROUND => format!("the ground levels out beneath you, {light} settling with it."),
            RIDGE => format!("you drop from the peak to the ridge's shoulder, {light} following."),
            _ => format!("you climb down; {light} sinks with you."),
        }
    }
}

/// One day = 120 ticks, 30 per phase (one pulse bar per phase): the
/// forge-worms donor tod cycle ported to integers.
pub fn day_phase(tick: u64) -> &'static str {
    match (tick % 120) / 30 {
        0 => "dawn",
        1 => "day",
        2 => "dusk",
        _ => "night",
    }
}

/// Deal 2-4 offers for this square, deterministic over (seed, x, y, z,
/// weather, xp, tick). Order: camp (if urgent) first, then climb/descend,
/// then compass.
pub fn offers(seed: u64, x: u16, y: u16, z: u16, w: &Weather, xp: u64, tick: u64) -> Vec<Offer> {
    let mut out = Vec::new();

    let (tx, ty) = world::town_square(seed);
    let phase = day_phase(tick);
    let urgent = (x, y) != (tx, ty)
        && (matches!(phase, "dusk" | "night")
            || matches!(w.sky, Sky::Storm | Sky::Ashfall)
            || w.intensity_pmy >= 5000);
    if urgent {
        out.push(Offer {
            verb: "camp",
            line: String::from("Camp Overnight — the sky argues for shelter."),
        });
    }

    let height = height_at(seed, x, y);
    if z < height {
        out.push(Offer { verb: "climb", line: format!("Climb — {}", climb_line(w.era, true, z + 1)) });
    }
    if z > 0 {
        out.push(Offer {
            verb: "descend",
            line: format!("Descend — {}", climb_line(w.era, false, z - 1)),
        });
    }

    // Compass: every in-bounds direction is a candidate; at most 2 are dealt
    // onto the menu, chosen by a hash over (square, xp) so the menu breathes
    // as xp grows.
    let mut dirs: Vec<(&'static str, &'static str, u16, u16)> = Vec::new();
    if y > 0 {
        dirs.push(("n", "North", x, y - 1));
    }
    if y < world::MAP_SIDE - 1 {
        dirs.push(("s", "South", x, y + 1));
    }
    if x > 0 {
        dirs.push(("w", "West", x - 1, y));
    }
    if x < world::MAP_SIDE - 1 {
        dirs.push(("e", "East", x + 1, y));
    }
    dirs.sort_by_key(|(verb, _, _, _)| {
        seed_hash(&[
            &u64_to_nistam(seed),
            &u16_to_nistam(x),
            &u16_to_nistam(y),
            &u64_to_nistam(xp),
            verb.as_bytes(),
            b"offer",
        ])
    });
    let take = dirs.len().min(2);
    for (verb, name, nx, ny) in dirs.into_iter().take(take) {
        let b = world::biome_at(seed, nx, ny, BIAS_NONE);
        out.push(Offer { verb, line: format!("Go {name} — {}", biome_invite(b.name)) });
    }

    out.truncate(4);
    out
}

/// The dawn's sensation after a camp — sky word plus weight word, no digits;
/// the same word choices `weather_line` uses, replicated (this module is
/// pure and does not depend on `game.rs`).
fn dawn_line(w: Weather) -> String {
    let sky = match w.sky {
        Sky::Clear => "clear skies",
        Sky::Overcast => "a lidded sky",
        Sky::Storm => "a storm-worked sky",
        Sky::Ashfall => "a sky sifting ash",
        Sky::Hardfrost => "a hard frost sitting on everything",
    };
    let weight = match w.intensity_pmy {
        0..=2499 => "the air is light",
        2500..=4999 => "the air carries weight",
        5000..=7499 => "the air presses close",
        _ => "the air is a held fist",
    };
    format!("you wake under {sky}; {weight}.")
}

/// The overnight: tick the model 30 steps (one pulse bar — the night
/// passes), then speak the dawn from the NEW sky.
pub fn camp(w: &mut WeatherModel) -> String {
    for _ in 0..30 {
        w.tick();
    }
    dawn_line(w.current)
}

/// The menu's face: one option per line, dimmed.
pub fn render_offers(offers: &[Offer]) -> String {
    offers
        .iter()
        .map(|o| format!("  \x1b[2m>\x1b[0m {}", o.line))
        .collect::<Vec<_>>()
        .join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same inputs deal the identical menu, every time.
    #[test]
    fn offers_are_deterministic() {
        let w = Weather::of(Era::Golden);
        let a = offers(42, 10, 10, 0, &w, 100, 45);
        let b = offers(42, 10, 10, 0, &w, 100, 45);
        assert_eq!(render_offers(&a), render_offers(&b), "the same square dealt a different menu");
    }

    /// The menu holds 2..=4 offers everywhere, and never leaks a digit or a
    /// cart word — an 81x81 sweep sampled every 9th square, at z=0 and z=2.
    #[test]
    fn offers_are_bounded_and_word_only() {
        let seed = 777u64;
        let w = Weather::of(Era::Decay);
        for &z in &[0u16, 2u16] {
            for y in (0..world::MAP_SIDE).step_by(9) {
                for x in (0..world::MAP_SIDE).step_by(9) {
                    let list = offers(seed, x, y, z, &w, 50, 60);
                    assert!(
                        (2..=4).contains(&list.len()),
                        "square {x},{y} z{z} dealt {} offers",
                        list.len()
                    );
                    for o in &list {
                        assert!(
                            o.line.chars().all(|c| !c.is_ascii_digit()),
                            "a digit leaked: {}",
                            o.line
                        );
                        let low = o.line.to_lowercase();
                        for word in ["deed", "bias", "wce", "consequence", "cart"] {
                            assert!(!low.contains(word), "the cart leaked ({word}): {}", o.line);
                        }
                    }
                }
            }
        }
    }

    /// The climb ladder matches `height_at` exactly: `z < height` offers
    /// climb, `z > 0` offers descend, and `z == height == 0` offers neither.
    #[test]
    fn the_climb_ladder_matches_height() {
        let seed = 321u64;
        let w = Weather::of(Era::Void);
        for y in (0..world::MAP_SIDE).step_by(9) {
            for x in (0..world::MAP_SIDE).step_by(9) {
                let h = height_at(seed, x, y);
                assert!(h <= PEAK);
                let ground = offers(seed, x, y, GROUND, &w, 0, 0);
                assert_eq!(
                    ground.iter().any(|o| o.verb == "climb"),
                    GROUND < h,
                    "square {x},{y} climb mismatch (height {h})"
                );
                assert!(
                    !ground.iter().any(|o| o.verb == "descend"),
                    "z=0 offered descend at {x},{y}"
                );
                let peak = offers(seed, x, y, PEAK, &w, 0, 0);
                assert!(
                    peak.iter().any(|o| o.verb == "descend"),
                    "z=PEAK never offers descend at {x},{y}"
                );
                assert!(
                    !peak.iter().any(|o| o.verb == "climb"),
                    "z=PEAK still offered climb at {x},{y}"
                );
            }
        }
    }

    /// `camp` moves the sky (30 ticks pass), speaks a digit-free dawn line,
    /// and is itself deterministic.
    #[test]
    fn camp_advances_the_sky_and_is_deterministic() {
        let before = WeatherModel::new(Era::Ancient, 5);
        let mut a = before.clone();
        let mut b = before.clone();
        let la = camp(&mut a);
        let lb = camp(&mut b);
        assert_ne!(a, before, "camp did not move the sky");
        assert!(la.chars().all(|c| !c.is_ascii_digit()), "the dawn line leaked a digit: {la}");
        assert_eq!(la, lb, "camp is not deterministic");
        assert_eq!(a, b, "two camps from the same sky landed on different skies");
    }

    /// The corners of the map never offer a compass step off the edge.
    #[test]
    fn edge_squares_never_offer_out_of_bounds_compass() {
        let seed = 55u64;
        let w = Weather::of(Era::Golden);
        let last = world::MAP_SIDE - 1;
        let a = offers(seed, 0, 0, 0, &w, 0, 0);
        assert!(!a.iter().any(|o| o.verb == "w"), "(0,0) offered West");
        assert!(!a.iter().any(|o| o.verb == "n"), "(0,0) offered North");
        let b = offers(seed, last, last, 0, &w, 0, 0);
        assert!(!b.iter().any(|o| o.verb == "e"), "the far corner offered East");
        assert!(!b.iter().any(|o| o.verb == "s"), "the far corner offered South");
    }

    /// `day_phase` cycles dawn/day/dusk/night on 30-tick boundaries and wraps
    /// at 120.
    #[test]
    fn day_phase_cycles_and_wraps() {
        assert_eq!(day_phase(0), "dawn");
        assert_eq!(day_phase(30), "day");
        assert_eq!(day_phase(60), "dusk");
        assert_eq!(day_phase(90), "night");
        assert_eq!(day_phase(120), "dawn");
    }
}
