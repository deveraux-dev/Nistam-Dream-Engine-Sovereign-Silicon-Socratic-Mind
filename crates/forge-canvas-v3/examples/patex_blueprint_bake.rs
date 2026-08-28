//! VAULT_OF_SEVEN_METALS as a drafting sheet — floor plan, front elevation,
//! side elevation and a true axonometric of ONE authored 71-column pane.
//!
//! Source: `docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md` §2.2.
//!
//! The pane carries no third dimension; height is extruded from the lane each
//! cell already has. Plan/front/side are rect collapses through the landed
//! `ProjectionPlane`. The axonometric is not — a projected cuboid is a hexagon
//! of three parallelograms, so it scores each face on the ternary sub-lattice
//! (`penteract::coverage`) and blends in the 9-ring (`penteract::lerp_ring`).
//!
//! Run: `cargo run -p forge-canvas-v3 --example patex_blueprint_bake`
//! Writes: `.forge/photons/patex_blueprint.bmp`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::patex::{
    lower_patex_glyphs, lower_projection, project_patex, project_patex_cut, render_axon,
    AbsenceIndex5D, PatexExtrude, PatexGrid, PatexLegend, PatexPalette, PATEX_PANE_FACE,
};
use forge_canvas_v3::rasterizer::{rasterize_into, write_bmp, PixelBuffer};
use forge_canvas_v3::structural_box::ProjectionPlane;
use forge_canvas_v3::text::FontAtlas;

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

const SHEET_W: u32 = 1400;
const SHEET_H: u32 = 780;
const PAPER: u32 = 0x0B_10_18_FF;
const RULE: u32 = 0x2E_4A_5A_FF;
const LABEL: u32 = 0x7F_A8_B8_FF;

/// Elevation cell edge in px.
const ELEV_CELL: i64 = 8;
/// Axonometric source-cell tile width in px (even — the slope is 1:2).
const AXON_TILE: i64 = 14;
/// Axonometric pixels per unit of extruded height.
const AXON_ELEV: i64 = 7;

fn frame(dl: &mut DrawList, atlas: &mut FontAtlas, x: i64, y: i64, w: i64, h: i64, title: &str) {
    dl.rect_outline(UiRect::new(x * 1000, y * 1000, w * 1000, h * 1000), RULE, 1000);
    dl.push_text(title, UiRect::new(x * 1000, (y - 18) * 1000, w * 1000, 16_000), LABEL, atlas);
}

fn main() {
    let legend = PatexLegend::canonical();
    let (plan, raster) = PatexGrid::rasterize(VAULT, &legend);
    let ex = PatexExtrude::CANONICAL;
    let palette = PatexPalette::CANONICAL;
    println!(
        "PARSE  {}x{} cells · bound {} · unbound {} · max height {}",
        plan.cols(),
        plan.rows(),
        raster.cells_bound,
        raster.cells_unbound,
        ex.max_height()
    );

    // Cut the near courses away: uncut, an elevation of a closed vault is its
    // own front wall and shows nothing of the room.
    const CUT: usize = 3;
    let front = project_patex_cut(&plan, ProjectionPlane::Front, &ex, CUT).expect("Front section");
    let side = project_patex_cut(&plan, ProjectionPlane::Side, &ex, CUT).expect("Side section");
    println!(
        "PROJ   front {}x{} drawn {} collapsed {} depth {} · side {}x{} drawn {} collapsed {} depth {}",
        front.pane.cols(), front.pane.rows(), front.stats.cells_drawn, front.stats.collapsed, front.depth_max,
        side.pane.cols(), side.pane.rows(), side.stats.cells_drawn, side.stats.collapsed, side.depth_max
    );
    assert!(
        project_patex(&plan, ProjectionPlane::Iso, &ex).is_none(),
        "Iso must refuse the rect path"
    );

    let mut atlas = FontAtlas::init(PATEX_PANE_FACE.bytes(), 16.0);
    let pitch = atlas.cell_advance();
    let line = pitch * 21_000 / 10_000;
    let mut dl = DrawList::new_boxed();

    // ── Floor plan: the authored sheet, as glyphs ───────────────────────────
    let plan_at = (40i64, 60i64);
    frame(
        &mut dl,
        &mut atlas,
        plan_at.0 - 8,
        plan_at.1 - 8,
        (plan.cols() as i64 * pitch) / 1000 + 16,
        (plan.rows() as i64 * line) / 1000 + 16,
        "1  FLOOR PLAN  (TOP)  1:1 authored",
    );
    let pg = lower_patex_glyphs(
        &plan,
        &mut dl,
        &mut atlas,
        &legend,
        UiRect::new(plan_at.0 * 1000, plan_at.1 * 1000, 0, 0),
        pitch,
        line,
        AbsenceIndex5D::FULL,
        &palette,
    );
    assert!(pg.is_complete(), "plan lost cells: {pg:?}");

    // ── Elevations: rect collapses, drawn as cells ──────────────────────────
    let front_at = (40i64, 430i64);
    frame(
        &mut dl,
        &mut atlas,
        front_at.0 - 8,
        front_at.1 - 8,
        front.pane.cols() as i64 * ELEV_CELL + 16,
        front.pane.rows() as i64 * ELEV_CELL + 16,
        "2  FRONT SECTION  (x / height)  cut 3, depth-shaded",
    );
    let fe = lower_projection(
        &front,
        &mut dl,
        UiRect::new(front_at.0 * 1000, front_at.1 * 1000, 0, 0),
        ELEV_CELL * 1000,
        ELEV_CELL * 1000,
        &palette,
        2_500,
    );

    let side_at = (40i64, 560i64);
    frame(
        &mut dl,
        &mut atlas,
        side_at.0 - 8,
        side_at.1 - 8,
        side.pane.cols() as i64 * ELEV_CELL + 16,
        side.pane.rows() as i64 * ELEV_CELL + 16,
        "3  SIDE SECTION  (y / height)  cut 3, depth-shaded",
    );
    let se = lower_projection(
        &side,
        &mut dl,
        UiRect::new(side_at.0 * 1000, side_at.1 * 1000, 0, 0),
        ELEV_CELL * 1000,
        ELEV_CELL * 1000,
        &palette,
        2_500,
    );
    println!("LOWER  plan {} glyphs · front {} quads · side {} quads", pg.glyphs, fe.quads, se.quads);

    // Axon viewport frame goes in the same command stream; its PIXELS are
    // painted after rasterizing, straight into the buffer.
    // Extent is derived, never guessed: width spans (cols+rows) half-tiles,
    // height the same in half-rises plus whatever the tallest cell stands.
    let half_w = AXON_TILE / 2;
    let half_h = half_w / 2;
    let axon_box = (740i64, 60i64);
    let axon_w = (plan.cols() + plan.rows()) as i64 * half_w;
    let axon_h = (plan.cols() + plan.rows()) as i64 * half_h + ex.max_height() as i64 * AXON_ELEV;
    let axon_org = (axon_box.0 + plan.rows() as i64 * half_w, axon_box.1 + ex.max_height() as i64 * AXON_ELEV);
    frame(
        &mut dl,
        &mut atlas,
        axon_box.0 - 8,
        axon_box.1 - 8,
        axon_w + 16,
        axon_h + 16,
        "4  AXONOMETRIC  2:1 dimetric = 2 of 27 ternary steps (26.67 deg)",
    );

    assert_eq!(dl.dropped, 0, "sheet arena overflowed: {} commands refused", dl.dropped);

    let mut buf = PixelBuffer::new(SHEET_W, SHEET_H);
    buf.clear(PAPER);
    rasterize_into(&mut buf, &dl, &atlas);

    // ── Axonometric: coverage-scored faces, painted per pixel ───────────────
    let ax = render_axon(&plan, &ex, &palette, &mut buf, axon_org, AXON_TILE, AXON_ELEV);
    println!(
        "AXON   cells {} drawn {} faces {} subsamples {} off-sheet {}",
        ax.cells_in, ax.cells_drawn, ax.faces_painted, ax.subsamples_lit, ax.off_sheet
    );
    assert!(ax.cells_drawn > 0, "the axonometric painted nothing");
    assert_eq!(ax.off_sheet, 0, "the axonometric ran off the sheet");

    // ── Readback: four populated regions, each distinct from the paper ──────
    let paper = [
        (PAPER >> 24) as u8,
        (PAPER >> 16) as u8,
        (PAPER >> 8) as u8,
    ];
    let ink_in = |x0: u32, y0: u32, w: u32, h: u32| -> u32 {
        let mut n = 0;
        for y in y0..(y0 + h).min(SHEET_H) {
            for x in x0..(x0 + w).min(SHEET_W) {
                let at = ((y * SHEET_W + x) * 4) as usize;
                if [buf.data[at], buf.data[at + 1], buf.data[at + 2]] != paper {
                    n += 1;
                }
            }
        }
        n
    };
    let views = [
        ("plan", ink_in(40, 60, 680, 300)),
        ("front", ink_in(40, 430, 580, 40)),
        ("side", ink_in(40, 560, 130, 40)),
        ("axon", ink_in(axon_box.0 as u32, axon_box.1 as u32, axon_w as u32, axon_h as u32)),
    ];
    for (name, n) in views {
        println!("READ   {name:<6} {n} inked px");
        assert!(n > 0, "viewport {name} is empty");
    }

    let dir = std::path::Path::new(".forge/photons");
    std::fs::create_dir_all(dir).expect("photon dir");
    let path = dir.join("patex_blueprint.bmp");
    write_bmp(&buf, &path).expect("write bmp");
    println!("PHOTON {} ({}x{})", path.display(), SHEET_W, SHEET_H);
}
