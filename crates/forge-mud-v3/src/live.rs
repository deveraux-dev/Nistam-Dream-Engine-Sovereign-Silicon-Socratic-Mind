//! The mud.live beacon: one tiny signal file rewritten atomically each step
//! (nde_chat.live donor pattern) — the colour/harmonic feed the vixi glass
//! reads later. Skeleton pre-created by the conductor; Weld D fills it.

use std::path::{Path, PathBuf};

use crate::consequence;
use crate::hermetics::{self, ConnectionRoll, Planet, Stat};
use crate::operator::Operator;
use crate::weather::Weather;
use crate::world;

/// The sky's weight, spoken as a bucket word — same thresholds as
/// `game.rs`'s `weather_line` (0-2499 light / 2500-4999 weight / 5000-7499
/// close / else fist), reused here as the machine-parseable field.
fn air_word(intensity_pmy: u32) -> &'static str {
    match intensity_pmy {
        0..=2499 => "light",
        2500..=4999 => "weight",
        5000..=7499 => "close",
        _ => "fist",
    }
}

/// The register whose dealt value (from [`ConnectionRoll::deal`]) is
/// highest — Tarnish and Guilt never rise above 0 in a fresh deal, so this
/// always lands on one of the five rolled registers.
fn dominant_stat(seed: u64) -> Stat {
    let s = ConnectionRoll::deal(seed).stats;
    let pairs = [
        (s.vigor, Stat::Vigor),
        (s.shadow_weight, Stat::ShadowWeight),
        (s.logic_depth, Stat::LogicDepth),
        (s.momentum, Stat::Momentum),
        (s.tarnish, Stat::Tarnish),
        (s.resonance, Stat::Resonance),
        (s.guilt, Stat::Guilt),
    ];
    pairs.into_iter().max_by_key(|(v, _)| *v).map(|(_, stat)| stat).unwrap_or(Stat::Vigor)
}

/// A planet's name, exactly as [`hermetics::SEVENFOLD`] rules it.
fn planet_word(planet: Planet) -> &'static str {
    match planet {
        Planet::Mars => "Mars",
        Planet::Saturn => "Saturn",
        Planet::Mercury => "Mercury",
        Planet::Luna => "Luna",
        Planet::Venus => "Venus",
        Planet::Sol => "Sol",
        Planet::Jupiter => "Jupiter",
    }
}

/// The node's vibe word — the world's mood, spoken plainly.
pub fn vibe_word(seed: u64) -> &'static str {
    world::theme(seed).vibe.0
}

/// The node's hue word — the SEVENFOLD planet ruling the dominant stat of
/// this node's connection roll.
pub fn hue_word(seed: u64) -> &'static str {
    let stat = dominant_stat(seed);
    let planet = stat.correspondence().map(|c| c.planet).unwrap_or(Planet::Sol);
    planet_word(planet)
}

/// The node's hue as a machine RGB word — one of
/// [`hermetics::CORE_PALETTE`]'s seven values, the same register that named
/// [`hue_word`].
pub fn hue_rgb(seed: u64) -> u32 {
    hermetics::CORE_PALETTE[dominant_stat(seed).index()]
}

/// The word for a standing tier — reuses `consequence::REP_TIERS`, the
/// engine's own ladder, rather than authoring a second one (L05).
pub fn standing_word(tier: usize) -> &'static str {
    consequence::REP_TIERS[tier.min(consequence::REP_TIERS.len() - 1)].1
}

/// The shadow's awareness word, bucketed off `heat` — the same six-tier
/// ladder `game::Game::status` reads off `content::shadow::SHADOW_TIERS`.
/// Heat is the engine's real law-escalation register; no separate
/// fail-streak counter exists to bucket instead (see shadow.rs's own note).
pub fn shadow_word(heat: u16) -> &'static str {
    let idx = match heat {
        0 => 0,
        1..=2 => 1,
        3..=5 => 2,
        6..=10 => 3,
        11..=20 => 4,
        _ => 5,
    };
    crate::content::shadow::SHADOW_TIERS[idx].0
}

/// Strip ANSI SGR escapes (`\x1b[...m`) from a line — the beacon's `word`
/// field carries plain text only, never a raw escape sequence.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The exact `mud.live` body: `mud UP`, then one `key value+` line per
/// field, integers and words only. Deterministic per (op, weather, tick,
/// last). `last` is the mud's last spoken line — ANSI-stripped here so the
/// glass never renders a raw escape (game.rs passes the reply's first line).
pub fn live_body(op: &Operator, w: &Weather, tick: u64, last: &str) -> String {
    let seed = op.node_seed;
    let fac = consequence::town_faction(seed);
    let faction = consequence::FACTIONS[fac].name;
    let tier = consequence::standing_tier(op.standings[fac]);
    let mut out = String::from("mud UP\n");
    out.push_str(&format!("seed 0x{seed:016x}\n"));
    out.push_str(&format!("tick {tick}\n"));
    out.push_str(&format!("era {}\n", w.era.name()));
    out.push_str(&format!("sky {}\n", w.sky.name()));
    out.push_str(&format!("air {}\n", air_word(w.intensity_pmy)));
    out.push_str(&format!("faction {faction}\n"));
    out.push_str(&format!("standing {}\n", standing_word(tier)));
    out.push_str(&format!("vibe {}\n", vibe_word(seed)));
    out.push_str(&format!("hue {}\n", hue_word(seed)));
    out.push_str(&format!("level {}\n", crate::game::Game::level(op.xp)));
    out.push_str(&format!("shadow {}\n", shadow_word(op.heat)));
    out.push_str(&format!("word {}\n", strip_ansi(last)));
    out
}

/// Write `body` to `path` atomically: write to `<path>.tmp`, then rename
/// over the destination (one step, never a torn read).
pub fn write_live(path: &Path, body: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)
}

/// `mud.live`'s path, sitting beside the save (`.forge/mud/operator.mud3`'s
/// sibling).
pub fn live_path_beside(save_path: &Path) -> PathBuf {
    save_path.parent().unwrap_or_else(|| Path::new(".")).join("mud.live")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather::Era;

    fn op() -> Operator {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.standings[0] = 900;
        op
    }

    /// The full local wire, end to end: a `dm` resolution folds onto the
    /// walker and the `shadow` line on the glass changes word. This is the
    /// pipeline `dm.rs -> operator.rs -> live.rs -> mud.live` in one assert.
    #[test]
    fn a_resolution_changes_the_shadow_word_on_the_glass() {
        let shadow_line = |body: &str| {
            body.lines()
                .find(|l| l.starts_with("shadow "))
                .expect("the body always carries a shadow line")
                .to_string()
        };
        let mut o = op();
        let w = Weather::of(Era::Golden);
        let before = shadow_line(&live_body(&o, &w, 42, "quiet."));
        assert_eq!(before, format!("shadow {}", shadow_word(0)));

        o.apply_resolution(&crate::dm::resolution_effects(crate::dm::ResolutionMode::Kill), None);
        let after = shadow_line(&live_body(&o, &w, 42, "quiet."));

        assert_eq!(o.heat, 6, "Kill's shadow_pressure reached the walker");
        assert_eq!(after, format!("shadow {}", shadow_word(6)));
        assert_ne!(before, after, "the glass must show the resolution");
    }

    /// First line is exactly "mud UP"; every other line splits into a key
    /// plus one-or-more value tokens by plain whitespace split; the body is
    /// deterministic for the same inputs.
    #[test]
    fn body_shape_is_dumb_split_parseable() {
        let o = op();
        let w = Weather::of(Era::Golden);
        let body = live_body(&o, &w, 42, "\x1b[1mthe line sings.\x1b[0m");
        let mut lines = body.lines();
        assert_eq!(lines.next(), Some("mud UP"));
        let mut n = 0;
        for line in lines {
            let mut parts = line.split_whitespace();
            assert!(parts.next().is_some(), "line {line:?} has no key");
            assert!(parts.next().is_some(), "line {line:?} has no value");
            n += 1;
        }
        assert_eq!(n, 12, "expected 12 key-value lines after the UP line");
        assert!(!body.contains('\x1b'), "the word line leaked an ANSI escape: {body:?}");
        assert!(body.contains("word the line sings."), "the last words are missing: {body:?}");
        assert!(body.contains("level "), "the witness row needs a level: {body:?}");
        assert!(body.contains("shadow "), "the witness row needs a shadow tier: {body:?}");
        assert_eq!(
            body,
            live_body(&o, &w, 42, "\x1b[1mthe line sings.\x1b[0m"),
            "same inputs, same body"
        );
    }

    /// The cart words never surface in the beacon — the player-facing file
    /// stays honest game terms, never the WCE machinery's own vocabulary.
    #[test]
    fn no_cart_words_in_body() {
        let o = op();
        let w = Weather::of(Era::Decay);
        let body = live_body(&o, &w, 7, "the sky holds clear").to_lowercase();
        for banned in ["deed", "bias", "wce", "consequence", "cart"] {
            assert!(!body.contains(banned), "banned word {banned:?} leaked into {body:?}");
        }
    }

    /// Two writes to the same path: the file exists, the second body wins,
    /// and no `.tmp` sibling survives.
    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let dir = std::env::temp_dir().join(format!("forge-mud-live-test-{:x}", 0xC0FFEE_u64));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mud.live");
        write_live(&path, "mud UP\nseed 0x1\n").unwrap();
        write_live(&path, "mud UP\nseed 0x2\n").unwrap();
        let read = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read, "mud UP\nseed 0x2\n");
        let tmp = dir.join("mud.live.tmp");
        assert!(!tmp.exists(), ".tmp residue left behind");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Hue and vibe words are pure functions of the seed; `hue_rgb` always
    /// names one of the seven core palette values.
    #[test]
    fn hue_and_vibe_are_deterministic_and_hue_rgb_is_core() {
        for seed in [1u64, 0xDEAD_BEEF, u64::MAX] {
            assert_eq!(vibe_word(seed), vibe_word(seed));
            assert_eq!(hue_word(seed), hue_word(seed));
            let rgb = hue_rgb(seed);
            assert!(hermetics::CORE_PALETTE.contains(&rgb), "0x{rgb:06x} not a core hue");
        }
    }

    /// `shadow_word` climbs the same six-tier ladder `game::Game::status`
    /// reads, and never panics past the ladder's own top.
    #[test]
    fn shadow_word_climbs_the_ladder_and_saturates() {
        assert_eq!(shadow_word(0), "Unseen");
        assert_eq!(shadow_word(4), "Pattern");
        assert_eq!(shadow_word(15), "Witness");
        assert_eq!(shadow_word(u16::MAX), "Harbinger");
    }

    /// `standing_word` is total over every tier the ladder defines.
    #[test]
    fn standing_word_is_total_over_all_tiers() {
        for tier in 0..=8usize {
            assert!(!standing_word(tier).is_empty());
        }
    }

    /// `live_path_beside` lands `mud.live` next to the save file.
    #[test]
    fn live_path_sits_beside_the_save() {
        let save = Path::new(".forge/mud/operator.mud3");
        assert_eq!(live_path_beside(save), Path::new(".forge/mud").join("mud.live"));
    }
}
