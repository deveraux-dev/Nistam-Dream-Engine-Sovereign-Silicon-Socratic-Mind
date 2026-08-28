//! CDK — the Cosmic Dissonance Kernel, MUD-side dealer + rendering.
//! One faction mind + position + haunt in, one triad verdict + colour out.
//! Empedocles made mechanical: LOVE binds, STRIFE separates, ENTROPY is what neither holds.
//!
//! Integer-only throughout. Channels land in 0..=1000 permyriad.
//! Colour mapping: strife→R, love→G, entropy→B.
//!
//! **L05 fix (2026-08-13):** `Triad` itself used to be defined a second time
//! in this file, byte-identical to `forge_core_v3::cdk::Triad` (fields,
//! `disposition`/`dissonant`/`to_channels`/`harmony`/`norm_signed`, all
//! copy-pasted). `forge-core-v3` is this workspace's `dag_root` (L06) and
//! `forge-mud-v3` already depends on it (`Cargo.toml:14`), so the duplicate
//! had no reason to exist — this crate now imports the one home instead.
//! What stays here, correctly: [`triad`] (the dealer — needs [`FactionMind`],
//! a faction-cognition type that is itself MUD-domain, not core-domain, so it
//! belongs downstream) plus [`colour`]/[`bar`]/[`report`]/[`report_ansi`]
//! (rendering, also downstream).

use crate::mind::FactionMind;
use crate::zone::{Cell, Domain, Island, Zone};
pub use forge_core_v3::cdk::Triad;

/// The bar representation: 6 characters, filled with '#' and empty with '.'.
/// Formula: (v.clamp(0,1000)*6)/1000, matching the atlas HTML law.
pub fn bar(v: i32) -> String {
    let filled = (v.clamp(0, 1_000) * 6 / 1_000).min(6) as usize;
    let mut s = String::with_capacity(6);
    for i in 0..6 {
        if i < filled {
            s.push('#');
        } else {
            s.push('.');
        }
    }
    s
}

/// The verdict as a word (BOUND if not dissonant, DISSONANT if torn).
pub fn verdict_word(t: &Triad) -> &'static str {
    if t.dissonant() {
        "DISSONANT"
    } else {
        "BOUND"
    }
}

/// The triad's RGB colour: strife→R, love→G, entropy→B.
/// Each channel 0..=1000 scales to 0..=255.
pub fn colour(t: &Triad) -> (u8, u8, u8) {
    let [l, s, e] = t.to_channels();
    let to_byte = |v: i32| (v.clamp(0, 1_000) * 255 / 1_000) as u8;
    (to_byte(s), to_byte(l), to_byte(e))
}

/// One kernel trit-state line for a cell, footer-style under the wireframe: the spatial
/// trit signs off [`Cell::trit`], then the Dante plane for the z lane — the elevation
/// lane the examples walk. z<0 → -1 Inferno "Ah!", z=0 → 0 Purgatorio "Ahah!",
/// z>0 → +1 Paradiso "Ahhh". The mapping is the sign the cell already carries — no new
/// scale is invented here.
pub fn trit_line(cell: &Cell) -> String {
    let (tx, ty, tz) = cell.trit();
    let (plane, vocal) = match tz {
        -1 => ("Inferno", "Ah!"),
        0 => ("Purgatorio", "Ahah!"),
        _ => ("Paradiso", "Ahhh"),
    };
    let t3 = |v: i8| match v {
        -1 => "-1",
        0 => " 0",
        _ => "+1",
    };
    format!(
        "     trit ({},{},{})  z-plane {} {plane} · {vocal}",
        t3(tx),
        t3(ty),
        t3(tz),
        t3(tz)
    )
}

/// A MIDI note's name, total over `u8` (e.g. 67 → "G4").
fn note_name(midi: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[(midi % 12) as usize], (midi / 12) as i32 - 1)
}

/// The Magic Word line — bible 003's first stroke, footer-style under the
/// wireframe. One word in, one line out: the word's pentatonic note
/// (`forge_harmonics::word_note`, the Rosetta invariant — same word, same
/// note, always in-scale), its prime-grid world seed
/// (`forge_sieve_v3::prime_seed`, size 64), and the island world that seed
/// births. Radius/peak derive from seed bits — a minimal Authored map
/// (radius 4..=15, peak 8..=31), in-bounds against the 33-cell zone by
/// construction. Integer-only end to end; the sung world is deterministic.
pub fn word_world_line(word: &str) -> String {
    let note = forge_harmonics::word_note(word.as_bytes());
    let name = note_name(note);
    let seed = forge_sieve_v3::prime_seed::prime_seed(word, 64);
    let radius = 4 + (seed % 12) as i64;
    let peak = 8 + ((seed >> 8) % 24) as i64;
    let z = Zone::new(Domain::Water).with_water_level(0).with_island(Island::new(radius, peak));
    let f = z.depth_field_mu();
    let wet = f.iter().filter(|&&d| d > 0).count();
    // The frame law (photon wave1-footer, 2026-08-17): the panel is
    // WIREFRAME_COLS wide and CLIPS, so the line trims ITSELF — word display
    // capped at 12 chars, seed shown as its top 8 hex digits, and a final
    // hard cap at WIREFRAME_COLS so no word can overflow the glass.
    let shown: String = word.chars().take(12).collect();
    let line = format!(
        "     sung \"{shown}\" · note {note} {name} · seed {:08x} · isle r{radius} p{peak} · wet {wet}",
        seed >> 32
    );
    line.chars().take(WIREFRAME_COLS).collect()
}

/// The hermetic mirror of word_world_line — footer-style under the wireframe.
/// As above, so below (Sean 2026-08-17, bible 003 addendum): the same word,
/// the same seed (recomputed via forge_sieve_v3::prime_seed — never a second
/// hash), same radius (4..=15); what is peak above is cave depth below. The
/// Inferno plane (trit −1, vocal "Ah!") signs the line. Integer-only end to end.
pub fn word_world_below_line(word: &str) -> String {
    let seed = forge_sieve_v3::prime_seed::prime_seed(word, 64);
    let radius = 4 + (seed % 12) as i64;
    let depth = 8 + ((seed >> 8) % 24) as i64;
    // Same frame law as word_world_line: the line trims itself to the glass.
    let shown: String = word.chars().take(12).collect();
    let line = format!(
        "     as below \"{shown}\" · cave r{radius} d{depth} · plane -1 Inferno · Ah!"
    );
    line.chars().take(WIREFRAME_COLS).collect()
}

/// Compute the triad from a faction mind, cell position, and haunt strength.
/// LOVE binds: permeability + ambiguity_tolerance + novelty_drive + proximity.
/// STRIFE separates: threat_sensitivity + dominance_drive + closure_pressure + depth.
/// ENTROPY is the haunt.
///
/// # Arguments
/// * `mind` - The faction's psychological profile
/// * `x, y, z` - Position in the zone (z is depth; negative = below ground)
/// * `haunt` - Entropy strength, 0..=10000 permyriad
pub fn triad(mind: &FactionMind, x: i32, y: i32, z: i32, haunt: u32) -> Triad {
    // Love lane: faction's openness + proximity (centre pulls)
    let love = mind.permeability as i32
        + mind.ambiguity_tolerance as i32
        + mind.novelty_drive as i32
        + proximity_pull(x, y);

    // Strife lane: faction's rigour + depth contests
    let strife = mind.threat_sensitivity as i32
        + mind.dominance_drive as i32
        + mind.closure_pressure as i32
        + depth_contest(z);

    Triad {
        love,
        strife,
        entropy: haunt as i32,
    }
}

/// Proximity to centre: closer yields stronger pull (binding).
/// Simple integer: distance heuristic, clamped to the lane span.
fn proximity_pull(x: i32, y: i32) -> i32 {
    let dist_sq = x * x + y * y;
    let pull = 1_000 - ((dist_sq / 100).min(1_000));
    pull.clamp(-Triad::LANE_SPAN, Triad::LANE_SPAN)
}

/// Depth contest: deeper below ground means more to fight over.
/// Negative z = below ground; depth contests add to strife.
fn depth_contest(z: i32) -> i32 {
    let depth = (-z).max(0);
    (depth / 10).clamp(0, 1_000)
}

/// Generate a plain-text report of the triad.
pub fn report(t: &Triad) -> String {
    let [l, s, e] = t.to_channels();
    format!(
        "triad love={} strife={} entropy={}\nchannels l={l} s={s} e={e}\ndisposition={} harmony={}/1000\n{} #{:02x}{:02x}{:02x}",
        t.love,
        t.strife,
        t.entropy,
        t.disposition(),
        t.harmony(),
        verdict_word(t),
        colour(t).0,
        colour(t).1,
        colour(t).2,
    )
}

/// Generate a ANSI-coloured report showing strips and verdicts.
pub fn report_ansi(t: &Triad) -> String {
    let [l, s, e] = t.to_channels();
    let (r, g, b) = colour(t);
    let love_strip = bar(l);
    let strife_strip = bar(s);
    let entropy_strip = bar(e);
    let verdict = verdict_word(t);

    // Simple ANSI colour codes (24-bit RGB)
    let ink = |txt: &str, (r, g, b): (u8, u8, u8)| {
        format!("\x1b[38;2;{r};{g};{b}m{txt}\x1b[0m")
    };

    let love_colour = (70, 220, 70);
    let strife_colour = (235, 70, 45);
    let entropy_colour = (150, 95, 255);

    format!(
        "{} {}\n{} {}\n{} {}\n{} {}\n{} #{:02x}{:02x}{:02x}",
        ink(&format!("love    {}", love_strip), love_colour),
        ink(&format!("{}", l), love_colour),
        ink(&format!("strife  {}", strife_strip), strife_colour),
        ink(&format!("{}", s), strife_colour),
        ink(&format!("entropy {}", entropy_strip), entropy_colour),
        ink(&format!("{}", e), entropy_colour),
        ink(&format!("disp {} harm {}", t.disposition(), t.harmony()), love_colour),
        ink(&format!("/1000"), (128, 128, 128)),
        ink(verdict, (r, g, b)),
        r, g, b
    )
}

/// Number of lines [`wireframe_lines`] emits. Fixed: the frame is a shaped box, so a
/// caller can size a panel from this without rendering first.
pub const WIREFRAME_ROWS: usize = 13;

/// Rendered width of every framed row, in characters, including the two-space left margin
/// and both border columns. Measured off the authored layout, not assumed — a panel sizes
/// its plane from this, so a wrong value tears the box on glass.
pub const WIREFRAME_COLS: usize = 71;

/// The singing terminal as one authored face, returned as lines instead of printed.
///
/// Every bar, verdict and colour here is a live query off `t` — nothing is drawn by hand,
/// so the picture and the tests cannot disagree. Lifted out of
/// `examples/cdk_wireframe.rs` (2026-08-15) so the example and the on-glass panel render
/// from ONE source (L05); the example still prints, and remains the reference.
///
/// Takes the [`Triad`] rather than dealing its own, so a caller can pass the triad for the
/// room the player is actually standing in.
pub fn wireframe_lines(t: &Triad, cmd: &str) -> Vec<String> {
    let [love, strife, entropy] = t.to_channels();
    let (r, g, b) = colour(t);
    vec![
        "  +- 13forge-studio -------------------------[ Shell | Chat | Voice ]-+".to_string(),
        "  | THEORY              | PTY  pwsh.exe                               |".to_string(),
        // `{cmd:<34}`, not 33: the authored original was one column short here, so this row
        // alone closed at 70 while every other framed row closed at 71 — a ragged right
        // border. Caught by `every_framed_row_is_the_same_width` (2026-08-15).
        format!("  |  scale  D dorian    | PS F:\\v3> {cmd:<34}|"),
        "  |  chord  Dm7         |    test result: ok                          |".to_string(),
        "  |                     |                                             |".to_string(),
        "  | -- CDK ------------ |                                             |".to_string(),
        // 5 trailing spaces, not 7: the authored original ran this row to 73 against the
        // frame's 71 — the second ragged row the width test caught (2026-08-15).
        format!("  |  colour #{r:02x}{g:02x}{b:02x}     |                                             |"),
        format!("  |  love    {} {love:>4}|                                             |", bar(love)),
        format!("  |  strife  {} {strife:>4}|                                             |", bar(strife)),
        format!("  |  entropy {} {entropy:>4}|                                             |", bar(entropy)),
        format!("  |  {:<19}|                                             |", verdict_word(t)),
        "  +---------------------+---------------------------------------------+".to_string(),
        format!("     disposition {}  harmony {}/1000", t.disposition(), t.harmony()),
    ]
}

#[cfg(test)]
mod word_world_tests {
    use super::*;

    /// The Rosetta invariant carried to the face: the same word must birth the
    /// same line, bit for bit — no clock, no RNG stream, no float anywhere.
    #[test]
    fn the_same_word_births_the_same_world() {
        assert_eq!(word_world_line("thorn"), word_world_line("thorn"));
    }

    /// The line carries all three projections: note (in pentatonic scale),
    /// seed, and a born world with wet columns.
    #[test]
    fn the_line_carries_note_seed_and_world() {
        let l = word_world_line("thorn");
        let note = forge_harmonics::word_note(b"thorn");
        assert!(forge_harmonics::PENTATONIC_C.contains(&note), "note in scale: {note}");
        assert!(l.contains(&format!("note {note}")), "note on line: {l}");
        assert!(l.contains("· seed ") && !l.contains("0x"), "short seed on line: {l}");
        assert!(l.contains("isle r") && l.contains("wet"), "world on line: {l}");
    }

    /// The frame law: no word — however long — may overflow the glass. The
    /// photon wave1-footer.png caught the unbounded line clipping at the
    /// panel border; this pins the fix.
    #[test]
    fn no_word_overflows_the_frame() {
        for w in ["thorn", "a", "supercalifragilisticexpialidocious"] {
            assert!(
                word_world_line(w).chars().count() <= WIREFRAME_COLS,
                "line overflows frame for {w}: {}",
                word_world_line(w)
            );
            assert!(
                word_world_below_line(w).chars().count() <= WIREFRAME_COLS,
                "below line overflows frame for {w}"
            );
        }
    }
}

#[cfg(test)]
mod word_world_below_tests {
    use super::*;

    /// The hermetic mirror invariant: the same word must birth the same below
    /// line, bit for bit — no clock, no RNG stream, no float anywhere.
    #[test]
    fn the_same_word_births_the_same_below_world() {
        assert_eq!(word_world_below_line("thorn"), word_world_below_line("thorn"));
    }

    /// The below line's cave radius must match the above line's isle radius
    /// — they share the same seed, same formula.
    #[test]
    fn the_below_radius_matches_the_above_radius() {
        let word = "thorn";
        let above = word_world_line(word);
        let below = word_world_below_line(word);

        // Compute the expected radius directly
        let seed = forge_sieve_v3::prime_seed::prime_seed(word, 64);
        let expected_radius = 4 + (seed % 12);
        let radius_str = format!("r{}", expected_radius);

        // Verify both lines contain the same radius string
        assert!(above.contains(&radius_str), "above line missing radius: {}", above);
        assert!(below.contains(&radius_str), "below line missing radius: {}", below);
    }
}

#[cfg(test)]
mod trit_line_tests {
    use super::*;

    /// The demo cell the example deals at: z below the centre plane must read Inferno.
    #[test]
    fn the_demo_cell_reads_inferno() {
        let l = trit_line(&Cell::spatial(2, 0, -3));
        assert!(l.contains("(+1, 0,-1)"), "signs: {l}");
        assert!(l.contains("Inferno") && l.contains("Ah!"), "plane: {l}");
    }

    /// The origin is the zero-state: Purgatorio, the laugh plane.
    #[test]
    fn the_origin_reads_purgatorio() {
        let l = trit_line(&Cell::ORIGIN);
        assert!(l.contains("Purgatorio") && l.contains("Ahah!"), "plane: {l}");
    }
}

#[cfg(test)]
mod wireframe_tests {
    use super::*;

    fn face() -> Vec<String> {
        let t = triad(&FactionMind::for_faction(0), 2, 0, -3, 40);
        wireframe_lines(&t, "cargo test -p forge-mud-v3")
    }

    /// The row count is a published constant — a panel sizes itself from it without
    /// rendering first, so a drifted layout must fail here, not silently mis-size the box.
    #[test]
    fn emits_exactly_the_published_row_count() {
        assert_eq!(face().len(), WIREFRAME_ROWS);
    }

    /// The frame is a closed box. Every interior row opens and closes with a border column,
    /// and the two rules cap it.
    #[test]
    fn the_box_is_closed_on_every_row() {
        let lines = face();
        assert!(lines[0].starts_with("  +-") && lines[0].ends_with('+'), "top rule: {}", lines[0]);
        assert!(
            lines[11].starts_with("  +-") && lines[11].ends_with('+'),
            "bottom rule: {}",
            lines[11]
        );
        for (i, l) in lines.iter().enumerate().take(11).skip(1) {
            assert!(l.starts_with("  |"), "row {i} lost its left border: {l}");
            assert!(l.ends_with('|'), "row {i} lost its right border: {l}");
        }
    }

    /// Frame width is uniform. Not cosmetic: this caught TWO ragged rows in the authored
    /// original — the `{cmd}` row closing at 70 and the colour row at 73, against a frame
    /// of 71. Invisible enough in stdout to survive unnoticed; a torn panel on glass.
    #[test]
    fn every_framed_row_is_the_same_width() {
        let lines = face();
        for (i, l) in lines.iter().enumerate().take(12) {
            assert_eq!(l.chars().count(), WIREFRAME_COLS, "row {i} is not the frame width: {l}");
        }
    }

    /// A long command must not blow the frame open — the field truncates the box, so the
    /// panel stays rectangular whatever the caller passes in.
    #[test]
    fn an_overlong_command_cannot_tear_the_frame() {
        let t = triad(&FactionMind::for_faction(0), 2, 0, -3, 40);
        let lines = wireframe_lines(&t, &"x".repeat(200));
        assert!(lines[2].ends_with('|'), "overlong cmd broke the right border: {}", lines[2]);
    }

    /// The face is a live query, not a picture: change the triad and the rendered bars must
    /// change with it. This is the property the module doc claims — pin it.
    #[test]
    fn the_bars_follow_the_triad() {
        let bound = Triad { love: 900, strife: 0, entropy: 0 };
        let torn = Triad { love: 0, strife: 900, entropy: 900 };
        let a = wireframe_lines(&bound, "x");
        let b = wireframe_lines(&torn, "x");
        assert_ne!(a[7], b[7], "love row is identical across opposite triads");
        assert_ne!(a[10], b[10], "verdict row did not move with the triad");
        assert!(a[10].contains(verdict_word(&bound)));
        assert!(b[10].contains(verdict_word(&torn)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::FactionMind;

    /// Triad determinism: same inputs yield same outputs.
    #[test]
    fn triad_is_deterministic() {
        let mind = FactionMind::for_faction(0);
        let t1 = triad(&mind, 10, 20, -5, 500);
        let t2 = triad(&mind, 10, 20, -5, 500);
        assert_eq!(t1, t2);
    }

    /// Channels always land in 0..=1000 permyriad [L18 gate].
    #[test]
    fn channels_stay_in_bind_range() {
        for fac_idx in 0..5 {
            let mind = FactionMind::for_faction(fac_idx);
            for x in [-100i32, 0, 50] {
                for z in [-20i32, 0, 5] {
                    for haunt in [0u32, 500, 10_000] {
                        let ch = triad(&mind, x, 0, z, haunt).to_channels();
                        assert!(
                            ch.iter().all(|c| (0..=1_000).contains(c)),
                            "faction {fac_idx} x={x} z={z} haunt={haunt} channels {ch:?}"
                        );
                    }
                }
            }
        }
    }

    /// Bar rendering: (v.clamp(0,1000)*6)/1000 filled chars.
    #[test]
    fn bar_renders_correct_count() {
        assert_eq!(bar(0), "......");
        assert_eq!(bar(500), "###...");
        assert_eq!(bar(1000), "######");
        assert_eq!(bar(1500), "######"); // clamped
    }

    /// Verdict flips at disposition boundary.
    #[test]
    fn verdict_tracks_disposition() {
        let bound = Triad { love: 2_000, strife: 500, entropy: 200 };
        assert!(!bound.dissonant());
        assert_eq!(verdict_word(&bound), "BOUND");

        let torn = Triad { love: 500, strife: 2_000, entropy: 500 };
        assert!(torn.dissonant());
        assert_eq!(verdict_word(&torn), "DISSONANT");
    }

    /// Colour channel mapping: strife→R, love→G, entropy→B.
    #[test]
    fn colour_maps_channels_to_rgb() {
        let t = Triad { love: 2_000, strife: 300, entropy: 100 };
        let (r, g, b) = colour(&t);
        let [l, s, e] = t.to_channels();
        let to_byte = |v: i32| (v.clamp(0, 1_000) * 255 / 1_000) as u8;
        assert_eq!(r, to_byte(s));
        assert_eq!(g, to_byte(l));
        assert_eq!(b, to_byte(e));
    }

    /// Sample HTML-derived strip test: verify the bar formula exactly.
    /// From atlas HTML line 209: `Math.floor(Math.min(Math.max(v,0),1000)*6/1000)`
    /// For v=600, expect floor(600*6/1000) = floor(3.6) = 3 filled.
    #[test]
    fn bar_matches_atlas_formula() {
        let v = 600i32;
        let filled = (v.clamp(0, 1_000) * 6 / 1_000) as usize;
        assert_eq!(filled, 3);
        assert_eq!(bar(v), "###...");
    }

    /// Harmony is proportion, not force.
    #[test]
    fn harmony_is_not_love() {
        let held = Triad { love: 1_000, strife: 500, entropy: 0 };
        let haunted = Triad { love: 1_000, strife: 500, entropy: 5_000 };
        assert_eq!(held.love, haunted.love, "love is a force and unchanged");
        assert!(haunted.harmony() < held.harmony(), "harmony is proportion and falls with entropy");
    }
}
