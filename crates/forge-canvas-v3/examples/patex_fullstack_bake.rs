//! FORGE-ENVELOPE / DJ DAW / 7-Domain Spectral MoE / SplitShader GPU Warden
//! full-stack PaTeX drafting sheet — floor plan, front/side sections, true axonometric.

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::patex::{
    lower_patex_glyphs, lower_projection, project_patex_cut, render_axon,
    AbsenceIndex5D, PatexExtrude, PatexGrid, PatexLegend, PatexPalette, PATEX_PANE_FACE,
};
use forge_canvas_v3::rasterizer::{rasterize_into, write_bmp, PixelBuffer};
use forge_canvas_v3::structural_box::ProjectionPlane;
use forge_canvas_v3::text::FontAtlas;

const FULLSTACK: &str = "\
╔═══════════════════════════════════════════════════════════════════╗
║ [13FORGE DUAL-DECK PHYSICAL DJ DAW & 7-DOMAIN SPECTRAL MoE]       ║
╠═══════════════════════════════════════════════════════════════════╣
║  ┌───────[PHYSICAL GESTURES & SOMATIC TOKENIZER 120Hz]─────────┐  ║
║  │ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ │  ║
║  │ │ MAGVEL  │ │ 8 RGB   │ │BASS-SWAP│ │ MODBUS  │ │ CAN BUS │ │  ║
║  │ │ PLATTER │ │ NEURAL  │ │CROSSFADE│ │ RS-485  │ │ISO 11898│ │  ║
║  │ └───┬─────┘ └───┬─────┘ └───┬─────┘ └───┬─────┘ └───┬─────┘ │  ║
║  │     │           │           │           │           │       │  ║
║  │ ════╪═══════════╪═══════════╪═══════════╪═══════════╪═════  │  ║
║  │     └───────────[ 16-BYTE UMPWORD SPSC BUS ]────────┘       │  ║
║  └─────────────────────────────────────────────────────────────┘  ║
║  ┌─────────[S13 7-DOMAIN SPECTRAL METAROUTER (128B)]───────────┐  ║
║  │ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────────┐ │  ║
║  │ │VOCAL│ │BASS │ │PERC │ │CAMEL│ │VOICE│ │CYMA │ │ LIMITER │ │  ║
║  │ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └───┬─────┘ │  ║
║  │    │       │       │       │       │       │         │       │  ║
║  │ ═══╪═══════╪═══════╪═══════╪═══════╪═══════╪═════════╪════   │  ║
║  │ ┌──┼───────┼───────┼───────┼───────┼───────┼─────────┼────┐ │  ║
║  │ │ SPLITSHADER GPU WARDEN & WGSL 256-PT FFT (IN-PLACE)     │ │  ║
║  │ └─────────────────────────────────────────────────────────┘ │  ║
║  └───────────[4-LANE AMBISONIC / LABAN EFFORT BUS]─────────────┘  ║
║  ┌───[SOVEREIGN CRUCIBLE]───┐  ┌──────────[CLOUD GOVERNOR]─────┐  ║
║  │ ┌─────────┐ ┌──────────┐ │  │ ┌───────────┐ ┌─────────────┐ │  ║
║  │ │ASP/FST  │ │ADR-0026  │ │  │ │GEMINI 3.7 │ │450K CONTEXT │ │  ║
║  │ │GBNF MASK│ │0-RETENTION│ │  │ │FLASH T=0  │ │$0.0004/CALL│ │  ║
║  │ └─────────┘ └──────────┘ │  │ └───────────┘ └─────────────┘ │  ║
║  └───────[TAPE E: FAILSAFE]─┘  └───────[WASIBOX SANDBOX]───────┘  ║
╚═══════════════════════════════════════════════════════════════════╝";

const SHEET_W: u32 = 1600;
const SHEET_H: u32 = 940;
const PAPER: u32 = 0x07_0B_12_FF;
const RULE: u32 = 0x22_44_55_FF;
const LABEL: u32 = 0x64_B5_CD_FF;
const INK_HI: u32 = 0x38_BD_F8_FF;
const INK_GOLD: u32 = 0xF5_9E_0B_FF;

/// Elevation cell edge in px.
const ELEV_CELL: i64 = 8;
/// Axonometric source-cell tile width in px (even — the slope is 1:2).
const AXON_TILE: i64 = 14;
/// Axonometric pixels per unit of extruded height.
const AXON_ELEV: i64 = 7;

fn frame(dl: &mut DrawList, atlas: &mut FontAtlas, x: i64, y: i64, w: i64, h: i64, title: &str) {
    dl.rect_outline(UiRect::new(x * 1000, y * 1000, w * 1000, h * 1000), RULE, 1000);
    dl.push_text(title, UiRect::new(x * 1000, (y - 18) * 1000, w.max(500) * 1000, 16_000), LABEL, atlas);
}

/// Char-aware (col, row) of a label inside the authored pane.
fn find_cell(pane: &str, row: usize, needle: &str) -> (i64, i64) {
    let cs: Vec<char> = pane.lines().nth(row).expect("pane row").chars().collect();
    let ns: Vec<char> = needle.chars().collect();
    let col = cs.windows(ns.len()).position(|w| w == ns).unwrap_or_else(|| {
        panic!("needle '{needle}' not found in row {row}: '{}'", pane.lines().nth(row).unwrap());
    });
    (col as i64, row as i64)
}

fn main() {
    let legend = PatexLegend::canonical();
    let (plan, raster) = PatexGrid::rasterize(FULLSTACK, &legend);
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

    const CUT: usize = 3;
    let front = project_patex_cut(&plan, ProjectionPlane::Front, &ex, CUT).expect("Front section");
    let side = project_patex_cut(&plan, ProjectionPlane::Side, &ex, CUT).expect("Side section");
    println!(
        "PROJ   front {}x{} drawn {} collapsed {} depth {} · side {}x{} drawn {} collapsed {} depth {}",
        front.pane.cols(), front.pane.rows(), front.stats.cells_drawn, front.stats.collapsed, front.depth_max,
        side.pane.cols(), side.pane.rows(), side.stats.cells_drawn, side.stats.collapsed, side.depth_max
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
        "1  FULL-STACK SOVEREIGN PLAN (TOP)  1:1 authored",
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

    // ── Callouts ────────────────────────────────────────────────────────────
    let callouts = [
        (1usize, "[13FORGE DUAL-DECK PHYSICAL DJ DAW & 7-DOMAIN SPECTRAL MoE]"),
        (3, "[PHYSICAL GESTURES & SOMATIC TOKENIZER 120Hz]"),
        (5, "MAGVEL"),
        (5, "8 RGB"),
        (5, "BASS-SWAP"),
        (5, "MODBUS"),
        (5, "CAN BUS"),
        (6, "PLATTER"),
        (6, "NEURAL"),
        (6, "CROSSFADE"),
        (6, "RS-485"),
        (6, "ISO 11898"),
        (10, "[ 16-BYTE UMPWORD SPSC BUS ]"),
        (12, "[S13 7-DOMAIN SPECTRAL METAROUTER (128B)]"),
        (14, "VOCAL"),
        (14, "BASS"),
        (14, "PERC"),
        (14, "CAMEL"),
        (14, "VOICE"),
        (14, "CYMA"),
        (14, "LIMITER"),
        (19, "SPLITSHADER GPU WARDEN & WGSL 256-PT FFT (IN-PLACE)"),
        (21, "[4-LANE AMBISONIC / LABAN EFFORT BUS]"),
        (22, "[SOVEREIGN CRUCIBLE]"),
        (22, "[CLOUD GOVERNOR]"),
        (24, "ASP/FST"),
        (24, "ADR-0026"),
        (24, "GEMINI 3.7"),
        (24, "450K CONTEXT"),
        (25, "GBNF MASK"),
        (25, "0-RETENTION"),
        (25, "FLASH T=0"),
        (25, "$0.0004/CALL"),
        (27, "[TAPE E: FAILSAFE]"),
        (27, "[WASIBOX SANDBOX]"),
    ];
    for (row, text) in callouts {
        let (c, r) = find_cell(FULLSTACK, row, text);
        dl.push_text(
            text,
            UiRect::new(
                plan_at.0 * 1000 + c * pitch,
                plan_at.1 * 1000 + r * line,
                text.chars().count() as i64 * pitch,
                line,
            ),
            INK_HI,
            &mut atlas,
        );
    }

    // ── Title block ─────────────────────────────────────────────────────────
    let tb = (780i64, 530i64);
    frame(&mut dl, &mut atlas, tb.0, tb.1, 760, 360, "5  TITLE BLOCK & PROVENANCE LEDGER");
    let block = [
        "13FORGE - SOVEREIGN CYBER-PHYSICAL STACK & 7-DOMAIN SPECTRAL MoE",
        "SOMATIC TOKENIZER (MODBUS/CAN/XINPUT 120Hz) -> 16B UMPWORD -> S13 ROUTER",
        "SPLITSHADER: WGSL 64/32 DUAL-U32, BIT-PERFECT, 256-PT IN-PLACE FFT",
        "4-LANE AMBISONIC: SCHAEFFER DSP & RUDOLF LABAN EFFORT VECTORS",
        "SOVEREIGN VAULT: ASP + FST + GBNF CRUCIBLE, ADR-0026 ZERO-RETENTION",
        "WASIBOX SANDBOX: FAIL-CLOSED SNAPSHOTS TO TAPE E:/.AIRGAP",
        "GEMINI VERTEX-AI: 450K CACHED CONTEXT, T=0.0, $0.0004/CALL GOVERNOR",
        "",
        "BAKED BY PATEX 5D GEOMETRIC TYPESETTING ENGINE",
        "zero heap hotpath · integer only ALU · 3^5 = 243 cell states · 13 moons",
    ];
    for (i, s) in block.iter().enumerate() {
        let color = if i < 7 { LABEL } else if i == 8 { INK_GOLD } else { INK_HI };
        dl.push_text(
            s,
            UiRect::new((tb.0 + 16) * 1000, (tb.1 + 24 + i as i64 * 32) * 1000, 720_000, 16_000),
            color,
            &mut atlas,
        );
    }

    // ── Sections ────────────────────────────────────────────────────────────
    let front_at = (40i64, 690i64);
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

    let side_at = (40i64, 800i64);
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

    // Axonometric Viewport
    let half_w = AXON_TILE / 2;
    let half_h = half_w / 2;
    let axon_box = (780i64, 60i64);
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
        "4  AXONOMETRIC  2:1 dimetric (26.67 deg) 5D Lattice Extrusion",
    );

    assert_eq!(dl.dropped, 0, "sheet arena overflowed: {} commands refused", dl.dropped);

    let mut buf = PixelBuffer::new(SHEET_W, SHEET_H);
    buf.clear(PAPER);
    rasterize_into(&mut buf, &dl, &atlas);

    // Axonometric Rendering
    let ax = render_axon(&plan, &ex, &palette, &mut buf, axon_org, AXON_TILE, AXON_ELEV);
    println!(
        "AXON   cells {} drawn {} faces {} subsamples {} off-sheet {}",
        ax.cells_in, ax.cells_drawn, ax.faces_painted, ax.subsamples_lit, ax.off_sheet
    );

    let dir = std::path::Path::new("F:/v3/.forge/photons");
    std::fs::create_dir_all(dir).expect("photon dir");
    let path_bmp = dir.join("patex_fullstack.bmp");
    write_bmp(&buf, &path_bmp).expect("write bmp");
    println!("PHOTON BMP {} ({}x{})", path_bmp.display(), SHEET_W, SHEET_H);
}
