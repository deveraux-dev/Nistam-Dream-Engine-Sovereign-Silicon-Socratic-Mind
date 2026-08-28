//! Scene Studio — terminal wireframe (concept, S3 scene-convergence).
//!
//! ASCII-only + basic 16-color ANSI (renders in every Windows console —
//! truecolor + unicode glyphs proved unreadable on the operator's terminal,
//! 2026-08-17). Run: `cargo run -p forge-mud-v3 --example scene_studio_wireframe`.

const R: &str = "\x1b[0m";
const DIM: &str = "\x1b[90m"; // bright black
const WHT: &str = "\x1b[97m";
const MAG: &str = "\x1b[95m"; // playhead
const CYN: &str = "\x1b[96m"; // cuts
const YEL: &str = "\x1b[93m"; // 3-click ribbon
const VIO: &str = "\x1b[35m"; // captions
const GRN: &str = "\x1b[92m"; // droplaw ok

fn lane(name: &str, glyph: char, cuts: [usize; 3]) {
    let mut s = String::new();
    for i in 0..80usize {
        if i == 31 {
            s.push_str(MAG);
            s.push('|');
        } else if cuts.contains(&i) {
            s.push_str(CYN);
            s.push(glyph);
        } else {
            s.push_str(DIM);
            s.push('.');
        }
    }
    println!("  {WHT}{name}{R} {s}{R}");
}

fn main() {
    println!();
    println!("  {WHT}SCENE STUDIO :: ironroot - scene convergence{R}   {DIM}fps 20 | ReelClock KEPT 500ms | tick 120Hz{R}");
    println!();
    println!("  {YEL}[1 PICK ATOM]{R} {DIM}--{R} {YEL}[2 DROP ON BEAT]{R} {DIM}--{R} {YEL}[3 PLAY]{R}     {DIM}atoms: branded_manifestation | biome_transition{R}");
    println!();
    println!("  {DIM}beat     .1        .2        .3        .4        .5        .6        .7        .8{R}");
    println!("  {DIM}         +---------+---------+---------+---------+---------+---------+---------+{R}");
    lane("G0 bass ", '#', [2, 22, 52]);
    lane("G1 drums", '*', [12, 32, 62]);
    lane("G2 synth", '+', [7, 47, 71]);
    lane("G3 voice", 'o', [17, 41, 66]);
    println!("  {WHT}caption {R} {DIM}....{VIO}\"IRONROOT - the closet sings worlds\"{DIM}.......{MAG}|{DIM}.............{VIO}\"...\"{DIM}.........{R}");
    println!("  {WHT}shader  {R} {DIM}....{CYN}bloom^{DIM}..........{CYN}fog^{DIM}......{MAG}|{DIM}....{CYN}grade^{DIM}.....................................{R}");
    println!();
    println!("  {WHT}[> play] [square stop] [+ add] [dup] [onion]{R}  {DIM}ghost <--o---->   scrub{R} {MAG}<====o=========>{R}");
    println!("  {DIM}droplaw{R} {GRN}OK trit -1 subcritical{R} {DIM}| dwell OK | blink OK | McCloud 65/20/15 OK   playhead{R} {MAG}00:12.400{R}{DIM} / 00:32.000{R}");
    println!();
    println!("  {DIM}concept face for F04 scrub-glass -- kit: crates\\scc\\golden\\vixi\\kits\\animation_timeline.kit.vixi{R}");
    println!();
}
