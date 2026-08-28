//! VAULT_OF_SEVEN_METALS, baked headless — the whitepaper's own 69-column
//! `\topology{...}` block lowered through the PaTeX 5D pipeline and this
//! crate's real DrawList/atlas/raster path, straight to a BMP.
//!
//! Source: `docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md` §2.2.
//!
//! Two renderers, composed the way the lanes want to be drawn: the material
//! lane fills quads (occlusion reads as mass), topology and marks shape real
//! monospace glyphs. Each pass is gated by an `AbsenceIndex5D` lane mask, so
//! the material pass early-outs on four ANDs — the block authors no fill glyph.
//!
//! Run: `cargo run -p forge-canvas-v3 --example patex_vault_bake`
//! Writes: `.forge/photons/patex_vault.bmp`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::patex::{
    ink_rgba, lane_mask, lower_patex, lower_patex_glyphs, write_ansi_fg, AbsenceIndex5D, PatexGrid,
    PatexLegend, PatexPalette, LANE_MARK, LANE_MATERIAL, LANE_TOPOLOGY, PATEX_PANE_FACE,
};
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::{FontAtlas, TypeFace};

const VAULT: &str = "\
╔═══════════════════════════════════════════════════════════════════╗
║ [STRATUM -3: LEADEN FOUNDRY]                  [PHASE: T0 PRESENT] ║
╠═══════════════════════════════════════════════════════════════════╣
║  ┌─────────────────────────[NORTH PORTAL]──────────────────────┐  ║
║  │ . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . │  ║
║  │ . . ┌─────────┐ . . . . . . . . . . . . . . . ┌─────────┐ . │  ║
║  │ . . │  NPE-1  │ . . . . . [ALTAR] . . . . . . │ SHADOW  │ . │  ║
║  │ . . │ (Meska) │ . . . . . 0x8F2C  . . . . . . │ AWARE=2 │ . │  ║
║  │ . . └────┬────┘ . . . . . . . . . . . . . . . └────┬────┘ . │  ║
║  │ . . . . .│. . . . . . . . . . . . . . . . . . . . .│. . . . │  ║
║  │ ═════════╪═════════════════════════════════════════╪═══════ │  ║
║  │          └──────────────< 120Hz VIBE >─────────────┘        │  ║
║  └─────────────────────────[ALIDADE SIGHT]─────────────────────┘  ║
╚═══════════════════════════════════════════════════════════════════╝";

/// Point size the pane's face is rasterized at.
const FONT_PX: f32 = 16.0;

/// Face by name, for side-by-side comparison bakes.
/// `cargo run --example patex_vault_bake -- jetbrains`
fn face_by_name(name: &str) -> Option<TypeFace> {
    Some(match name {
        "iosevka" => TypeFace::IosevkaFixed,
        "jetbrains" => TypeFace::JetBrainsMono,
        "rajdhani" => TypeFace::Rajdhani,
        "tektur" => TypeFace::Tektur,
        "cinzel" => TypeFace::Cinzel,
        "reemkufi" => TypeFace::ReemKufi,
        "garamond" => TypeFace::CormorantGaramond,
        _ => return None,
    })
}
/// Line pitch as a permyriad of the cell advance — terminal proportion.
const LINE_PITCH_PMY: i64 = 21_000;
/// Pane inset from the framebuffer edge, in pixels.
const MARGIN: i64 = 20;

fn main() {
    let legend = PatexLegend::canonical();
    let (grid, raster) = PatexGrid::rasterize(VAULT, &legend);
    println!(
        "PARSE  {}x{} cells · bound {} · held-blank {} · unbound {} · overflow {}c/{}r",
        grid.cols(),
        grid.rows(),
        raster.cells_bound,
        raster.cells_held_blank,
        raster.cells_unbound,
        raster.cols_overflowed,
        raster.rows_overflowed
    );
    println!("INDEX  {} distinct lattice states present", grid.index().population());

    // The atlas's own advance IS the grid pitch. Anything else drifts the run
    // off the columns (text.rs cell_advance law).
    // No argument = the pane's declared face. An argument overrides it, for
    // side-by-side comparison bakes only.
    let arg = std::env::args().nth(1);
    let face = match arg.as_deref() {
        None => PATEX_PANE_FACE,
        Some(name) => face_by_name(name).unwrap_or_else(|| panic!("unknown face {name:?}")),
    };
    let slug = arg.as_deref().unwrap_or("pane");
    let mut atlas = FontAtlas::init(face.bytes(), FONT_PX);
    let cell_w = atlas.cell_advance();
    let cell_h = cell_w * LINE_PITCH_PMY / 10_000;
    println!(
        "PITCH  advance {cell_w} mu · line {cell_h} mu · face {} (mono={}) @{FONT_PX}px",
        face.label(),
        face.is_mono()
    );

    let screen_w = ((grid.cols() as i64 * cell_w) / 1_000 + 2 * MARGIN) as u32;
    let screen_h = ((grid.rows() as i64 * cell_h) / 1_000 + 2 * MARGIN) as u32;
    let origin = UiRect::new(MARGIN * 1_000, MARGIN * 1_000, 0, 0);

    let palette = PatexPalette::CANONICAL;
    let mut draw = DrawList::new_boxed();

    // Material lane: fill quads. The block authors no fill glyph, so this must
    // early-out on the index gate without touching the arena.
    let mat = lower_patex(
        &grid,
        &mut draw,
        origin,
        cell_w,
        cell_h,
        lane_mask(LANE_MATERIAL),
        &palette,
    );
    println!("LANE   material  quads early_out={} quads={}", mat.early_out, mat.quads);
    assert!(mat.early_out, "the vault authors no material glyph — the gate must reject the pane");
    assert_eq!(draw.cmd_count, 0, "an early-out must not touch the arena");

    // Topology and marks: real glyphs.
    let topo = lower_patex_glyphs(
        &grid,
        &mut draw,
        &mut atlas,
        &legend,
        origin,
        cell_w,
        cell_h,
        lane_mask(LANE_TOPOLOGY),
        &palette,
    );
    let mark = lower_patex_glyphs(
        &grid,
        &mut draw,
        &mut atlas,
        &legend,
        origin,
        cell_w,
        cell_h,
        lane_mask(LANE_MARK),
        &palette,
    );
    println!("LANE   topology  glyphs={} filtered={} complete={}", topo.glyphs, topo.filtered, topo.is_complete());
    println!("LANE   mark      glyphs={} filtered={} complete={}", mark.glyphs, mark.filtered, mark.is_complete());

    // Every way a cell can vanish is refused here, not discovered on glass.
    assert!(topo.is_complete(), "topology lane lost cells: {topo:?}");
    assert!(mark.is_complete(), "mark lane lost cells: {mark:?}");
    assert_eq!(
        draw.dropped, 0,
        "DrawList arena overflowed: {} commands refused — the pane would render incomplete",
        draw.dropped
    );

    let full = lower_patex(
        &grid,
        &mut DrawList::new_boxed(),
        origin,
        cell_w,
        cell_h,
        AbsenceIndex5D::FULL,
        &palette,
    );
    assert_eq!(
        topo.glyphs + mark.glyphs,
        full.quads,
        "the two glyph lanes must cover exactly the cells the quad path draws"
    );

    let buf = rasterize(&draw, &atlas, screen_w, screen_h);

    let px_at = |x: u32, y: u32| -> [u8; 4] {
        let at = ((y * buf.width + x) * 4) as usize;
        [buf.data[at], buf.data[at + 1], buf.data[at + 2], buf.data[at + 3]]
    };
    // A glyph does not fill its cell, so a readback samples the cell BOX and
    // asks whether any ink landed in it — the honest test for shaped text.
    let cell_inked = |col: i64, row: i64| -> (u32, [u8; 4]) {
        let x0 = (MARGIN * 1_000 + col * cell_w) / 1_000;
        let y0 = (MARGIN * 1_000 + row * cell_h) / 1_000;
        let bg = px_at(2, 2);
        let mut lit = 0u32;
        let mut brightest = bg;
        for y in y0..(y0 + cell_h / 1_000) {
            for x in x0..(x0 + cell_w / 1_000) {
                let p = px_at(x as u32, y as u32);
                if p != bg {
                    lit += 1;
                    if p[0] as u32 + p[1] as u32 + p[2] as u32
                        > brightest[0] as u32 + brightest[1] as u32 + brightest[2] as u32
                    {
                        brightest = p;
                    }
                }
            }
        }
        (lit, brightest)
    };

    let (topo_lit, topo_px) = cell_inked(0, 0);
    let (mark_lit, mark_px) = cell_inked(5, 4);
    let (empty_lit, _) = cell_inked(1, 1);
    println!("READ   cell 0.0 '╔'  inked_px={topo_lit} brightest={topo_px:?}");
    println!("READ   cell 5.4 '.'  inked_px={mark_lit} brightest={mark_px:?}");
    println!("READ   cell 1.1 ' '  inked_px={empty_lit} (held-blank, must stay dark)");
    assert!(topo_lit > 0, "the topology glyph did not paint");
    assert!(mark_lit > 0, "the mark glyph did not paint");
    assert_ne!(topo_px, mark_px, "the two lanes rendered the same ink");
    if face.is_mono() {
        // Only a fixed advance guarantees a glyph stays inside its own cell.
        assert_eq!(empty_lit, 0, "a held-blank cell must draw nothing at all");
        assert!(topo_lit > mark_lit, "a box corner must cover more of its cell than a floor dot");
    } else if empty_lit > 0 {
        println!("WARN   {} is proportional — ink bled into a held-blank cell", face.label());
    }

    let mut ansi = [0u8; 24];
    for (name, ink) in [
        ("topology", palette.topology),
        ("mark", palette.mark),
        ("material", palette.material),
    ] {
        let n = write_ansi_fg(ink, &mut ansi).expect("escape fits 24 bytes");
        println!(
            "INK    {name:<9} rgba=0x{:08X} ansi={:?}",
            ink_rgba(ink),
            core::str::from_utf8(&ansi[1..n]).expect("ascii")
        );
    }

    let dir = std::path::Path::new(".forge/photons");
    std::fs::create_dir_all(dir).expect("photon dir");
    let path = dir.join(format!("patex_vault_{slug}.bmp"));
    write_bmp(&buf, &path).expect("write bmp");
    println!(
        "PHOTON {} ({}x{}, {} glyphs, {} cmds)",
        path.display(),
        screen_w,
        screen_h,
        draw.glyph_count,
        draw.cmd_count
    );
}
