//! THE ASTROLABE — the Forge's star instrument, baked headless, with its effects.
//!
//! An astrolabe reads the sky and tells you where you stand. This one takes a clock, selects
//! a star, returns the register that star bends, and renders the whole plate in colour and
//! harmony — the reading Sean ran in v2 as `forge-studio/src/sky_verb.rs`, rebuilt on v3's
//! live catalog through this crate's monospace atlas and rasteriser.
//!
//! **Nothing here authors a colour.** Every ink is drained from what the tree declares:
//! - the instrument's brass, from `.forge/hud.html:4-5` — whose `:19` names the organ and
//!   whose `:195` states the law: *"astrolabe keeps its brass; only the hearth is vixi"*.
//! - star rows, from `Spectral::ink()` — what the star actually burns.
//! - register rows, from `hermetics::stat_ink()`, the SEVENFOLD metal spine.
//!
//! **Effects** (all integer, all deterministic — same `(frame, row)` ⇒ same ink, so the plate
//! replays bit-identically; no wall clock, no float, no `sin`):
//! shimmer · consonance-dimming · the pitch-class ring · per-star timbre.
//!
//! Run: `cargo run -p forge-canvas-v3 --example astrolabe_bake`
//! Writes: `.forge/astrolabe.bmp`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::{FontAtlas, TypeFace};
use forge_canvas_v3::widgets::{glow_dot, label, level_meter, progress_bar};
use forge_core_v3::decay::LeakyPermyriad;
use forge_core_v3::sky::{active_index, mag_fill, report_lines, Brightness, CATALOG};
use forge_harmonics::scale_mask::ScaleMask;
use forge_harmonics::scale_voice::{note_to_mhz, VoicePreset};
use forge_mud_v3::hermetics::{modifier_for, stat_ink, Stat};

/// MilliUnit per pixel (`widgets.rs:846` calls `UiRect::new(0,0,40_000,40_000)` a 40x40 socket).
const MU: i64 = 1_000;
const PT: f32 = 15.0;
const ROW_PX: i64 = 18;
const PAD: i64 = 14;
/// Iosevka's advance is ~0.6em; +1 keeps the right edge off the frame.
const CHAR_PX: i64 = (PT * 0.62) as i64 + 1;

// ── The Astrolabe's own palette, verbatim from `.forge/hud.html:4-5` ─────────

/// `--brasshi` — gold highlight. Headings and the active reading.
const BRASS_HI: u32 = 0xC3A256FF;
/// `--brassdim` — age. Tarnished bronze, for quiet structure.
const BRASS_DIM: u32 = 0x5F4A22FF;
/// `--verd` — verdigris. What copper becomes; the aligned register.
const VERD: u32 = 0x6D8A6BFF;
/// `--ink` — sand. The plate's default text.
const SAND: u32 = 0xC3B791FF;

/// The base every register starts from — v2's `BASE` (`sky_verb.rs:125`).
const BASE: i32 = 100;
/// The frame this plate is baked at. Stepping it animates the shimmer.
const FRAME: u32 = 5;

/// Raise an ink to a legibility floor on a dark plate, keeping its hue.
///
/// The SEVENFOLD metals are the true encoding (L08), and one is Saturn's lead `0x0F0C17`,
/// which rendered the ShadowWeight row **invisible** on this ground (measured, first bake).
/// Not a wrong colour — a correct colour on the wrong ground. This scales the channels until
/// the brightest reaches `FLOOR`, preserving their ratio: hue survives, only value moves.
const FLOOR: u32 = 0x66;
fn legible(ink: u32) -> u32 {
    let (r, g, b, a) = (ink >> 24 & 0xFF, ink >> 16 & 0xFF, ink >> 8 & 0xFF, ink & 0xFF);
    let peak = r.max(g).max(b);
    if peak >= FLOOR || peak == 0 {
        return ink;
    }
    let lift = |c: u32| (c * FLOOR / peak).min(0xFF);
    (lift(r) << 24) | (lift(g) << 16) | (lift(b) << 8) | a
}

// ── EFFECTS ─────────────────────────────────────────────────────────────────

/// Scale an ink's value by `pmy` permyriad, hue held. 10_000 = unchanged.
fn scale_value(ink: u32, pmy: u32) -> u32 {
    let ch = |c: u32| ((c * pmy) / 10_000).min(0xFF);
    (ch(ink >> 24 & 0xFF) << 24)
        | (ch(ink >> 16 & 0xFF) << 16)
        | (ch(ink >> 8 & 0xFF) << 8)
        | (ink & 0xFF)
}

/// **SHIMMER** — v2's `mag_ink` carried a `shimmer_pmy` lightness swing at 12fps
/// (`sky_verb.rs:39-69`). Here it is an integer triangle wave: each row sits at its own phase
/// so the plate breathes rather than pulsing in unison, and brighter stars swing wider — a
/// dim star twinkling as hard as Sirius reads as noise, not as sky.
/// A star's light as a LEAKY INTEGRATOR, not a wave.
///
/// An earlier cut of this file hand-rolled a triangle wave here — an invention sitting next
/// to the real primitive. `forge_core_v3::decay::LeakyPermyriad` IS the decay primitive,
/// drained from the v2 proof harness `decay_primitive.py` (Sanctuary decay spec §3,
/// `decay.rs:1-4`), with its flooring discipline already pinned by tests.
///
/// So a star injects light on its own beat and leaks it back: brighter stars inject more AND
/// leak slower, so they linger while faint ones snap dark. A twinkle with a physical model
/// behind it — and the model is the one this tree already proved.
///
/// Deterministic: same `(frame, row, fill)` ⇒ same value. Per-tick floor throughout, never a
/// floored base mixed with an unfloored sum (`decay.rs:12-16`).
const GLOW_PERIOD: u32 = 24;
fn shimmer_pmy(frame: u32, row: usize, fill_cells: usize) -> u32 {
    let fill = fill_cells.clamp(1, 10) as u64;
    // Leak is parts-per-myriad lost per tick: a full bar (10) leaks 200, a single cell 1_100.
    // Brighter light lingers. `new` rejects leak 0, so the floor keeps the value legal.
    let leak = (1_200u16).saturating_sub(fill as u16 * 100).max(200);
    let mut d = LeakyPermyriad::new(0, leak).expect("leak is 1..=PMY by construction");
    let phase = (row as u32 * 5) % GLOW_PERIOD; // row-offset so the plate breathes, not pulses
    for t in 0..=frame.min(96) {
        if (t + phase) % GLOW_PERIOD == 0 {
            d.inject(fill * 900); // the star's beat
        }
        d.tick();
    }
    // Ride the decayed light around unity so text stays legible at the trough.
    9_400 + (d.value.min(1_200) as u32 / 2)
}

/// **CONSONANCE** — each spectral class is dealt a pitch class, stepping by 5 (a fourth), so
/// neighbouring classes are NOT neighbouring pitches — the same decorrelation the star clock
/// gets from its stride of 7. Membership is then one bitwise op through `ScaleMask`.
fn star_pitch_class(spectral_index: usize) -> u8 {
    ((spectral_index * 5) % 12) as u8
}

/// The 12-cell pitch-class ring: the scale made visible.
fn scale_ring(mask: ScaleMask, root_pc: u8) -> String {
    let mut s = String::with_capacity(12 * 3);
    for pc in 0..12u8 {
        let member = mask.is_member(pc);
        s.push(match (pc == root_pc, member) {
            (true, true) => '◉',  // the root, sounding
            (true, false) => '○', // the root, outside its own scale
            (false, true) => '●',
            (false, false) => '·',
        });
    }
    s
}

/// **TIMBRE** — spiritual weight picks the voice a star would sing in. Glass is fast and
/// crystalline, Hearth warm and mid, Reed slow and breathy (`scale_voice.rs` attacks:
/// Glass 500us, Hearth 15_000us, Reed 80_000us).
fn star_voice(b: Brightness) -> VoicePreset {
    match b {
        Brightness::SpiritFire => VoicePreset::Glass,
        Brightness::GuideStar => VoicePreset::Hearth,
        Brightness::AncestorLight | Brightness::TheForgotten => VoicePreset::Reed,
    }
}

fn main() {
    let clock: u8 = 5;
    let active = active_index(clock);
    let star = &CATALOG[active];
    let reading = modifier_for(star.brightness, star.spectral);

    // The active star's spectral class sets the key; the scale is the ANCIENT prior
    // transposed onto it. Every other star is then consonant or not against THAT.
    let root_pc = star_pitch_class(active % 9);
    let mask = ScaleMask::ANCIENT.transpose(root_pc);
    let root_midi = 60 + root_pc; // middle-C octave
    let root_mhz = note_to_mhz(root_midi);
    let voice = star_voice(star.brightness);

    // ── assemble the plate: (text, ink) ──
    let mut lines: Vec<(String, u32)> = Vec::new();
    lines.push(("T H E   A S T R O L A B E".to_string(), BRASS_HI));
    lines.push(("the forge reads the sky".to_string(), BRASS_DIM));
    lines.push((String::new(), SAND));

    // Row indices are RECORDED as the plate is assembled, never hardcoded — the effects below
    // draw into the same rows as the text, and a hand-counted offset would silently drift the
    // moment a line is added.
    let star_row_0 = lines.len() as i64;

    // Star rows: own spectral ink, shimmered by brightness, dimmed when out of key.
    for (i, (text, ink)) in report_lines(clock).into_iter().enumerate() {
        let s = &CATALOG[i];
        let fill = mag_fill(s.mag_permyriad);
        let pc = star_pitch_class(i % 9);
        let consonant = mask.is_member(pc);
        // Harmony drives colour: a star outside the active key sits back on the plate.
        let harmony_pmy = if consonant { 10_000 } else { 5_200 };
        let lit = scale_value(ink, shimmer_pmy(FRAME, i, fill) * harmony_pmy / 10_000);
        lines.push((text, if i == active { legible(ink) } else { lit }));
    }
    lines.push((String::new(), SAND));

    // ── the harmony block ──
    let harmony_row = lines.len() as i64;
    lines.push((
        format!(
            "key   {} root pc{root_pc} midi{root_midi} {}.{:03} Hz",
            scale_ring(mask, root_pc),
            root_mhz / 1_000,
            root_mhz % 1_000
        ),
        VERD,
    ));
    lines.push((
        format!(
            "voice {voice:?}  attack {}us  decay {}us  partial {}",
            voice.attack_us(),
            voice.decay_us(),
            voice.emphasis_partial()
        ),
        VERD,
    ));
    lines.push((String::new(), SAND));

    // The active reading, in the v2 plate's own words.
    lines.push((
        match &reading {
            Some((stat, op)) => format!(
                "clock={clock}  active={}  {} {}  modifier={:?} {}",
                star.name,
                star.brightness.name(),
                star.spectral.name(),
                stat,
                op.sigil()
            ),
            None => format!(
                "clock={clock}  active={}  {} {}  modifier=None",
                star.name,
                star.brightness.name(),
                star.spectral.name()
            ),
        },
        BRASS_HI,
    ));

    // The register sheet — each stat in its own metal, the bent one in verdigris and lifted.
    let stat_row_0 = lines.len() as i64;
    for (i, stat) in Stat::ALL.into_iter().enumerate() {
        let bent = matches!(&reading, Some((b, _)) if *b == stat);
        let base_ink = if bent {
            VERD
        } else {
            legible(stat_ink(stat).map(|rgb| (rgb << 8) | 0xFF).unwrap_or(SAND))
        };
        // The bent register glows: it rides the shimmer at full brightness weight.
        let ink = if bent { scale_value(base_ink, shimmer_pmy(FRAME, i, 10)) } else { base_ink };
        let after = match &reading {
            Some((b, op)) if *b == stat => op.apply(BASE),
            _ => BASE,
        };
        let mark = if bent { '>' } else { ' ' };
        lines.push((format!("{mark} {stat:?} {BASE} -> {after}"), ink));
    }

    // ── render ──
    let mut atlas = FontAtlas::init(TypeFace::IosevkaFixed.bytes(), PT);
    let cols = lines.iter().map(|(t, _)| t.chars().count()).max().unwrap_or(1) as i64;
    let w = PAD * 2 + CHAR_PX * cols;
    let h = PAD * 2 + ROW_PX * lines.len() as i64;

    let mut draw = DrawList::new_boxed();
    for (i, (text, ink)) in lines.iter().enumerate() {
        if text.is_empty() {
            continue;
        }
        let y = PAD + ROW_PX * i as i64;
        let rect = UiRect::new(PAD * MU, y * MU, (w - PAD * 2) * MU, ROW_PX * MU);
        label(&mut draw, rect, text, *ink, &mut atlas);
    }
    // ── graphical effects, over the plate ────────────────────────────────────
    // `glow_dot`, `progress_bar`, `level_meter` are landed widgets
    // (`widgets.rs:804/735/748`) — no authored geometry here.
    let gutter_x = w - PAD - 132;

    // GLOW — one dot per star, intensity from its own magnitude fill, shimmered, and GATED
    // BY HARMONY: a star outside the active key gives less light, not just paler text.
    for i in 0..CATALOG.len() {
        let fill = mag_fill(CATALOG[i].mag_permyriad);
        let harmony = if mask.is_member(star_pitch_class(i % 9)) { 10_000u32 } else { 3_800 };
        let intensity = (fill as u32 * 1_000).min(10_000)
            * shimmer_pmy(FRAME, i, fill)
            / 10_000
            * harmony
            / 10_000;
        let y = PAD + ROW_PX * (star_row_0 + i as i64);
        glow_dot(&mut draw, UiRect::new(gutter_x * MU, (y + 3) * MU, 11 * MU, 11 * MU), intensity.min(10_000));
    }

    // METERS — each register's value as a bar in its own metal, against the 200 ceiling a
    // `<< 1` reaches. The doubled register visibly fills where the others sit at half.
    for (i, stat) in Stat::ALL.into_iter().enumerate() {
        let bent = matches!(&reading, Some((b, _)) if *b == stat);
        let value = match &reading {
            Some((b, op)) if *b == stat => op.apply(BASE),
            _ => BASE,
        };
        let ink = if bent {
            VERD
        } else {
            legible(stat_ink(stat).map(|rgb| (rgb << 8) | 0xFF).unwrap_or(SAND))
        };
        let y = PAD + ROW_PX * (stat_row_0 + i as i64);
        progress_bar(
            &mut draw,
            UiRect::new(gutter_x * MU, (y + 5) * MU, 118 * MU, 8 * MU),
            (value.max(0) as u32 * 10_000) / 200,
            ink,
        );
    }

    // HARMONY METER — how much of the 12-tone ring this key claims, against how many stars
    // fall inside it. The key's weight on the sky, as two lanes.
    let claimed = (0..12u8).filter(|pc| mask.is_member(*pc)).count() as u32;
    let inside =
        (0..CATALOG.len()).filter(|i| mask.is_member(star_pitch_class(i % 9))).count() as u32;
    let hy = PAD + ROW_PX * harmony_row;
    level_meter(
        &mut draw,
        UiRect::new(gutter_x * MU, (hy + 4) * MU, 118 * MU, 10 * MU),
        claimed * 10_000 / 12,
        inside * 10_000 / CATALOG.len() as u32,
        0,
    );

    assert_eq!(draw.dropped, 0, "DrawList arena overflowed — the plate would render incomplete");

    // NOTE for the shell wiring: use `rasterize_overlay` there, not `rasterize`.
    // `.forge/hud.html:6-11` states the law — "the shell's GPU sky rotates BEHIND this page…
    // the sky is the work; the glass must show it". `rasterize` clears to an OPAQUE ground,
    // right for a BMP viewed alone and wrong for a plate over the sky.
    let buf = rasterize(&draw, &atlas, w as u32, h as u32);

    // Readback (L09): sample the ground from a provably unpainted corner, count only what
    // differs. "alpha != 0" would count the opaque ground and prove nothing.
    let px_at = |x: u32, y: u32| -> [u8; 4] {
        let at = ((y * buf.width + x) * 4) as usize;
        [buf.data[at], buf.data[at + 1], buf.data[at + 2], buf.data[at + 3]]
    };
    let ground = px_at(0, 0);
    let lit = buf.data.chunks_exact(4).filter(|p| *p != ground).count();
    let mut seen: Vec<[u8; 4]> = Vec::new();
    for p in buf.data.chunks_exact(4) {
        let px = [p[0], p[1], p[2], p[3]];
        if px != ground && !seen.contains(&px) {
            seen.push(px);
        }
    }

    let out = std::path::Path::new(".forge/astrolabe.bmp");
    write_bmp(&buf, out).expect("write .forge/astrolabe.bmp");

    let consonant = (0..CATALOG.len()).filter(|i| mask.is_member(star_pitch_class(i % 9))).count();

    println!("THE ASTROLABE");
    println!("  clock      : {clock}  ->  {} [{}]", star.name, star.constellation);
    println!("  reading    : {}/{}", star.brightness.name(), star.spectral.name());
    match &reading {
        Some((stat, op)) => {
            println!("  modifier   : {stat:?} {}  ({BASE} -> {})", op.sigil(), op.apply(BASE))
        }
        None => println!("  modifier   : None"),
    }
    println!("  key        : {}  root pc{root_pc}  {}.{:03} Hz", scale_ring(mask, root_pc), root_mhz / 1_000, root_mhz % 1_000);
    println!("  voice      : {voice:?} (attack {}us)", voice.attack_us());
    println!("  consonance : {consonant}/{} stars sit inside the key", CATALOG.len());
    println!("  shimmer    : frame {FRAME}, LeakyPermyriad decay, brightness-weighted leak");
    println!("  palette    : brass .forge/hud.html:4-5 (drained, not authored)");
    println!("  surface    : {w} x {h} px RGBA8");
    println!("  rows       : {} ({} cmds, {} dropped)", lines.len(), draw.cmd_count, draw.dropped);
    println!("  readback   : {lit} inked texels, {} distinct colours", seen.len());
    println!("  written    : {}", out.display());
}
