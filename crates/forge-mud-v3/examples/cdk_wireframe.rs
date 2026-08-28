//! Prints the singing terminal as an ASCII wireframe, straight off the real CDK API.
//! Nothing here is drawn by hand — every bar, verdict and channel is a query, so the
//! picture and the tests cannot disagree.
//! `cargo run -p forge-mud-v3 --example cdk_wireframe`
//!
//! Re-scoped from the v2 donor (`F:\NewRepo\crates\forge-game-systems\examples\
//! cdk_wireframe.rs`, moved verbatim to `forge-core-v3/examples/` 2026-08-15, found not to
//! compile there) onto what v3 actually landed, which turned out to be a real,
//! independently-derived equivalent under different names — not a gap:
//! - `forge_core_v3::cdk::Triad` (re-exported by `forge_mud_v3::cdk`) already carries
//!   `to_channels`/`harmony`/`disposition`/`dissonant`.
//! - `forge_mud_v3::cdk::{triad, bar, colour, verdict_word}` is the dealer + rendering the
//!   v2 file hand-rolled inline (`window`/`bar` here) — reused, not reimplemented (L05).
//! - `forge_mud_v3::mind::FactionMind::for_faction(idx)` replaces v2's
//!   `faction_mind_profile(faction_id: &str)` — index-based, not name-based, in this crate.
//! - `forge_mud_v3::zone::{Cell, Domain, Zone, Island}` is v2's `normalized_zone`, ported
//!   under this crate instead of a standalone `forge-zones-v3::normalized_zone` (which
//!   never got that module — only the unrelated Ulam spiral).
//!
//! **LIFTED (2026-08-15):** the studio frame itself is no longer authored here. It lives in
//! `forge_mud_v3::cdk::wireframe_lines`, so the on-glass panel in `shell/` renders from the
//! SAME source rather than a second copy of the layout (L05). This example remains the
//! reference: it prints those lines verbatim, and a test asserts the two agree.
//!
//! Named, not silently dropped (C09 aperture): `room_frame`/`tone_of_triad`/`RoomFrame`/
//! `Stance`/`stance_from_rep` and `Triad::to_vibe` (needs `vibe_matrix`, absent from v3)
//! and `sky`/`seed` flavour (needs `syzygy`, absent from v3) are cut. Pulling those in would
//! mean porting two more whole subsystems for cosmetic example output — a separate, larger
//! follow-on, not bundled into "make this build."

use forge_mud_v3::cdk::{triad, trit_line, verdict_word, wireframe_lines, word_world_line};
use forge_mud_v3::mind::FactionMind;
use forge_mud_v3::zone::{Cell, Domain, Island, Zone, CELL_MILLI, EDGE};

/// The window as the host paints it: theory rail left, live PTY right, CDK under the rail.
fn window(mind: &FactionMind, cmd: &str) {
    let t = triad(mind, 2, 0, -3, 40);
    for line in wireframe_lines(&t, cmd) {
        println!("{line}");
    }
}

/// The same room read at three depths — the triad is a function of the cell, so walking
/// down the z lane is the whole demo.
fn walk(mind: &FactionMind) {
    println!("\n== walking down z ==  one cell per row");
    println!("   cell(x,y,z)     love strife entr  verdict");
    for z in [4_i64, 0, -4, -12] {
        let t = triad(mind, 0, 0, z as i32, 0);
        let [l, s, e] = t.to_channels();
        println!("   ( 0, 0,{z:>3})     {l:>4} {s:>5} {e:>4}  {}", verdict_word(&t));
    }
}

/// The floor the room stands on. The triad walks down z above; the zone says what that z
/// IS physically — same integer lattice, same cell, read through the other organ.
fn zone() {
    let z = Zone::new(Domain::Water).with_water_level(0).with_island(Island::new(8, 24));
    let f = z.depth_field_mu();
    let wet = f.iter().filter(|&&d| d > 0).count();
    let deepest = f.iter().max().copied().unwrap_or(0);
    println!(
        "\n== the floor under it ==  zone {EDGE}x{EDGE}  cell={CELL_MILLI}mu  level={}",
        z.water_level_cells
    );
    println!("   columns {}  wet {wet}  dry {}  deepest {deepest}mu", f.len(), f.len() - wet);
    println!("   cell(x,y,z)     medium  density  depth");
    let m = CELL_MILLI as i32;
    for zc in [4_i64, 0, -4, -12] {
        let s = z.submersion([15 * m, 0, zc as i32 * m]);
        println!(
            "   ( 0, 0,{zc:>3})     {}   {:>5}pmy {:>6}mu",
            if zc < z.water_level_cells { "water" } else { "air  " },
            s.density_pmy,
            s.depth_mu,
        );
    }
}

fn main() {
    let mind = FactionMind::for_faction(0);
    let cell = Cell::spatial(2, 0, -3);

    println!("\n== TECHNOTHESIA · singing terminal ==  every number below is a live query");
    window(&mind, "cargo test -p forge-mud-v3");
    println!("{}", trit_line(&cell));
    println!("{}", word_world_line("thorn"));
    walk(&mind);
    zone();
    println!();
}
