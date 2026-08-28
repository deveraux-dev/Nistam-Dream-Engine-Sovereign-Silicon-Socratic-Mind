//! `mud` — the singing terminal's game door. Interactive at a keyboard AND
//! headless over a pipe (`echo look | mud`), which is how the conductor
//! witnesses the ASCII output without a window. Autosaves every step,
//! including the last.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process;

use forge_mud_v3::hermetics::{describe_boon, ConnectionRoll};
use forge_mud_v3::{Game, Operator};
use forge_cart_v3;

fn prompt(out: &mut impl Write, line: &str) {
    let _ = write!(out, "{line}");
    let _ = out.flush();
}

/// The operator's persistent home: `<repo-root>/.forge/mud/operator.mud3`,
/// found by walking up from the cwd to the nearest `.forge` — the same
/// operator answers from any directory (Sean 2026-08-11: a persistent save
/// in the forge config's own place).
fn default_save_path() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join(".forge").is_dir() {
            return dir.join(".forge/mud/operator.mud3");
        }
        if !dir.pop() {
            return PathBuf::from(".forge/mud/operator.mud3");
        }
    }
}

fn main() {
    let mut args = std::env::args();
    args.next(); // Skip program name.

    // Check for --cart flag.
    if let Some(arg) = args.next() {
        if arg == "--cart" {
            if let Some(cart_path) = args.next() {
                let cart_bytes = match std::fs::read(&cart_path) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("failed to read cart file: {}", e);
                        process::exit(1);
                    }
                };
                match forge_cart_v3::load(&cart_bytes) {
                    Ok(_) => {
                        println!("the cart is sealed and live.");
                        return;
                    }
                    Err(refusal) => {
                        eprintln!("cart refusal: {}", refusal);
                        process::exit(1);
                    }
                }
            } else {
                eprintln!("--cart requires a path argument");
                process::exit(1);
            }
        }
    }

    // Normal game flow: arg is the save path, or use default.
    let save_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_save_path);

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut out = std::io::stdout();

    // Load the operator or hold the door: name, then birthday.
    let op = match std::fs::read(&save_path).ok().and_then(|b| Operator::decode(&b)) {
        Some(op) => {
            let _ = writeln!(out, "\x1b[1mwelcome back, operator {}.\x1b[0m", op.name);
            op
        }
        // The birth-prompt bounds (moon 1-13, day 1-28) are NOT arbitrary.
        // They are governed by carts/base/npe.base.ron (birth.moon_count: 13,
        // birth.day_count: 28) and mirrored in operator.rs as MOON_COUNT and
        // MOON_DAYS (lines 48,50). This is currently a manual two-place
        // invariant, proven live by the test npe_base_cart_moon_and_day_counts_
        // match_operator_constants in operator.rs. Full cart-loading is future work.
        None => loop {
            prompt(&mut out, "operator name (yours, not Claude): ");
            let Some(Ok(name)) = lines.next() else { return };
            prompt(&mut out, "birth moon (1-13): ");
            let Some(Ok(moon)) = lines.next() else { return };
            prompt(&mut out, "birth day (1-28): ");
            let Some(Ok(day)) = lines.next() else { return };
            let moon = moon.trim().parse::<u8>().unwrap_or(1).saturating_sub(1);
            let day = day.trim().parse::<u8>().unwrap_or(1).saturating_sub(1);
            match Operator::birth(&name, moon, day) {
                Some(mut op) => {
                    // The sky moving the game has to be visible, not merely
                    // true (`hermetics.rs::apply_natal`'s own doc) — this is
                    // the one caller in the live game that redeems it.
                    let mut roll = ConnectionRoll::deal(op.node_seed);
                    let boon = roll.apply_natal(&mut op);
                    let _ = writeln!(
                        out,
                        "\x1b[1mborn under moon {} day {}. the node deals.\x1b[0m\r\nunder {}: {}",
                        op.moon + 1,
                        op.day + 1,
                        roll.constellation(),
                        describe_boon(boon)
                    );
                    break op;
                }
                None => {
                    let _ = writeln!(out, "a nameless operator cannot pass.");
                }
            }
        },
    };

    let mut game = Game::new(op, Some(save_path));
    game.light_dream_fire(Box::new(forge_mud_v3::dream::DoorFire::new()));
    let (hello, _) = game.process("look");
    let _ = writeln!(out, "{hello}\r\ntype 'help' for verbs, 'map' for the land.");

    loop {
        prompt(&mut out, "\x1b[1;36m» \x1b[0m");
        let Some(Ok(line)) = lines.next() else {
            // EOF (a pipe ran dry): one last save through the quit verb.
            let (bye, _) = game.process("quit");
            let _ = writeln!(out, "{bye}");
            return;
        };
        let trimmed = line.trim();
        if trimmed == "sing" {
            let _ = writeln!(out, "the word is missing — sing <word>");
            continue;
        } else if let Some(rest) = trimmed.strip_prefix("sing ") {
            let word = rest.split_whitespace().next().unwrap_or("");
            if word.is_empty() {
                let _ = writeln!(out, "the word is missing — sing <word>");
            } else {
                let result = forge_mud_v3::cdk::word_world_line(word);
                let _ = writeln!(out, "{}", result);
            }
            continue;
        }
        let (reply, keep_going) = game.process(&line);
        let _ = writeln!(out, "{reply}");
        if !keep_going {
            return;
        }
    }
}
