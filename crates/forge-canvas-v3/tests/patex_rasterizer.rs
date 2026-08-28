#![deny(unsafe_code)]
//! PROOF: the 71-column PaTeX rasterizer binds Horner radix-3 cells, projects
//! back to source glyphs losslessly, and early-outs a whole pane on the
//! AbsenceIndex5D gate — all inside DrawList's fixed arena, zero drops.

use forge_canvas_v3::draw::{DrawList, MAX_CMDS};
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::patex::{
    lane_mask, lower_patex, lower_patex_glyphs, AbsenceIndex5D, Material, PatexGrid, PatexLegend,
    PatexPalette,
    project_patex, project_patex_cut, render_axon, sentinel_mask, AxonFace, AxonStats,
    PatexExtrude, AXON_CIRCLE_TRITS,
    AXON_RISE, AXON_RUN, AXON_STEPS, BOX_ALGEBRA, INTERIOR_MASK, LANE_MARK, LANE_MATERIAL,
    LANE_TOPOLOGY, MARKS, MATERIALS, PATEX_COLS, PATEX_HELD_BLANK,
};
use forge_canvas_v3::rasterizer::PixelBuffer;
use forge_canvas_v3::structural_box::ProjectionPlane;
use forge_canvas_v3::text::{FontAtlas, TypeFace, ATLAS_SIZE};
use forge_core_v3::atom::TritCell5D;

use forge_canvas_v3::patex::PATEX_PANE_FACE as PANE_FACE;

/// The VAULT_OF_SEVEN_METALS `\topology{...}` block, verbatim from
/// `docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md` lines 87-100.
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

/// A pane of nothing but box-drawing and floor marks — every glyph is bound.
const PURE: &str = "\
┌───┬───┐
│...│...│
├───┼───┤
│...│...│
└───┴───┘";

fn vault() -> (PatexGrid, PatexLegend) {
    let legend = PatexLegend::canonical();
    let (grid, _) = PatexGrid::rasterize(VAULT, &legend);
    (grid, legend)
}

#[test]
fn whitepaper_topology_fits_the_71_column_bound() {
    let legend = PatexLegend::canonical();
    let (grid, r) = PatexGrid::rasterize(VAULT, &legend);
    assert_eq!(grid.rows(), 14, "the vault block is 14 source rows");
    assert_eq!(grid.cols(), 69, "measured width of the authored block");
    assert!(grid.cols() <= PATEX_COLS, "the B-locked pane bound is 71 columns");
    assert_eq!(r.rows_read, 14);
    assert_eq!(r.rows_overflowed, 0);
    assert_eq!(r.cols_overflowed, 0);
    assert!(r.cells_bound > 0, "the block must bind real lattice cells");
    assert!(r.cells_unbound > 0, "prose labels are unbound by design, not silently geometry");
    assert_eq!(
        r.cells_bound + r.cells_held_blank,
        14 * 69,
        "every consumed character lands in exactly one bucket"
    );
}

#[test]
fn every_glyph_of_a_pure_pane_binds_and_projects_back() {
    let legend = PatexLegend::canonical();
    let (grid, r) = PatexGrid::rasterize(PURE, &legend);
    assert_eq!(r.cells_unbound, 0, "a pure geometry pane has no unbound glyph");
    assert_eq!(r.cells_held_blank, 0, "and no held-blank cell");

    // Lossless reverse projection: every cell renders back to its source char.
    for (row, line) in PURE.lines().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let cell = grid.cell(col, row);
            assert_eq!(
                legend.glyph_for(cell.0),
                Some(ch),
                "cell {col}.{row} did not project back to {ch:?}"
            );
        }
    }
}

#[test]
fn rasterize_is_deterministic() {
    let legend = PatexLegend::canonical();
    let a = PatexGrid::rasterize(VAULT, &legend);
    let b = PatexGrid::rasterize(VAULT, &legend);
    assert_eq!(a.0, b.0, "same source, same pane");
    assert_eq!(a.1, b.1, "same source, same receipt");
}

#[test]
fn overwide_and_overtall_source_is_refused_loudly() {
    let legend = PatexLegend::canonical();
    let wide: String = core::iter::repeat('─').take(90).collect();
    let (_, r) = PatexGrid::rasterize(&wide, &legend);
    assert_eq!(r.cols_overflowed, 90 - PATEX_COLS as u32, "19 columns past the bound");

    let tall: String = core::iter::repeat("│\n").take(60).collect();
    let (_, r) = PatexGrid::rasterize(&tall, &legend);
    assert_eq!(r.rows_overflowed, 60 - 48, "12 rows past the ceiling");
}

#[test]
fn absence_index_gates_the_whole_pane_before_any_sweep() {
    let (grid, _) = vault();
    let mut dl = DrawList::new_boxed();
    let origin = UiRect::new(0, 0, 71_000, 48_000);

    // The vault block carries topology and marks, but no material glyph.
    assert!(grid.any_of(lane_mask(LANE_TOPOLOGY)), "box drawing is present");
    assert!(grid.any_of(lane_mask(LANE_MARK)), "floor dots are present");
    assert!(!grid.any_of(lane_mask(LANE_MATERIAL)), "no fill glyph was authored");
    assert!(grid.index().is_absent(Material::Rock.cell()));

    let stats = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        lane_mask(LANE_MATERIAL),
        &PatexPalette::CANONICAL,
    );
    assert!(stats.early_out, "a pane holding none of the filtered states must early-out");
    assert_eq!(stats.quads, 0);
    assert_eq!(dl.cmd_count, 0, "the early-out must not touch the arena at all");
}

#[test]
fn full_lowering_fills_the_arena_without_dropping() {
    let (grid, _) = vault();
    let mut dl = DrawList::new_boxed();
    let origin = UiRect::new(0, 0, 71_000, 48_000);

    let stats = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        AbsenceIndex5D::FULL,
        &PatexPalette::CANONICAL,
    );
    assert!(!stats.early_out);
    assert!(stats.quads > 0, "the vault must lower to real quads");
    assert!(stats.blank > 0, "unbound prose cells are held-blank, never drawn");
    assert_eq!(stats.filtered, 0, "a FULL filter rejects nothing");
    assert_eq!(
        stats.quads + stats.blank,
        (grid.cols() * grid.rows()) as u32,
        "every cell is either lowered or skipped, none lost"
    );
    assert!(stats.quads as usize <= MAX_CMDS, "the pane must fit the fixed arena");
    assert_eq!(dl.cmd_count as u32, stats.quads);
    assert_eq!(dl.dropped, 0, "zero arena drops");

    // Steady state: a second identical pass over the same cleared arena.
    let first = dl.cmd_count;
    dl.clear();
    let again = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        AbsenceIndex5D::FULL,
        &PatexPalette::CANONICAL,
    );
    assert_eq!(again, stats, "lowering is frame-invariant");
    assert_eq!(dl.cmd_count, first);
    assert_eq!(dl.dropped, 0);
}

#[test]
fn a_single_lane_filter_draws_only_that_lane() {
    let (grid, _) = vault();
    let mut dl = DrawList::new_boxed();
    let origin = UiRect::new(0, 0, 71_000, 48_000);

    let topo = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        lane_mask(LANE_TOPOLOGY),
        &PatexPalette::CANONICAL,
    );
    dl.clear();
    let marks = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        lane_mask(LANE_MARK),
        &PatexPalette::CANONICAL,
    );
    dl.clear();
    let both = lower_patex(
        &grid,
        &mut dl,
        origin,
        1_000,
        1_000,
        lane_mask(LANE_TOPOLOGY).union(lane_mask(LANE_MARK)),
        &PatexPalette::CANONICAL,
    );

    assert!(topo.quads > 0 && marks.quads > 0);
    assert_eq!(
        topo.quads + marks.quads,
        both.quads,
        "the lanes partition the drawn cells — no cell drawn twice, none missed"
    );
}

#[test]
fn material_glyphs_carry_their_occlusion_into_the_pane() {
    let legend = PatexLegend::canonical();
    let (solid, _) = PatexGrid::rasterize("████\n████", &legend);
    let (mist, _) = PatexGrid::rasterize("░░░░\n░░░░", &legend);
    assert_eq!(solid.occlusion_pmy(), Material::Rock.density_pmy() as u32);
    assert_eq!(mist.occlusion_pmy(), Material::Mist.density_pmy() as u32);
    assert!(solid.any_of(lane_mask(LANE_MATERIAL)));
    assert!(!solid.any_of(lane_mask(LANE_TOPOLOGY)));
}

/// The rasterized coverage bitmap for `ch`, lifted out of the atlas texture.
fn glyph_ink(atlas: &mut FontAtlas, ch: char) -> Option<Vec<u8>> {
    let g = atlas.get_or_rasterize(ch)?;
    let x0 = (g.uv[0] * ATLAS_SIZE as f32).round() as usize;
    let y0 = (g.uv[1] * ATLAS_SIZE as f32).round() as usize;
    let (w, h) = (g.size[0] as usize, g.size[1] as usize);
    let mut ink = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            ink.push(atlas.texture_data[(y0 + y) * ATLAS_SIZE + (x0 + x)]);
        }
    }
    Some(ink)
}

/// Legend glyphs `face` cannot actually draw.
///
/// A missing codepoint does NOT rasterize empty — fontdue draws `.notdef`, the
/// tofu box, which has ink and a size like any other glyph. So the oracle is a
/// bitmap comparison against a codepoint guaranteed absent (U+E000, private
/// use): any legend glyph whose ink is byte-identical to tofu is not in the
/// face. Checking `is_none()` alone passes every font on disk and proves
/// nothing — Tektur and Rajdhani both "covered" all 54 that way, and both bake
/// to a wall of tofu.
fn uncovered_glyphs(face: TypeFace) -> Vec<char> {
    let mut atlas = FontAtlas::init(face.bytes(), 15.0);
    let tofu = glyph_ink(&mut atlas, '\u{E000}');
    let mut missing = Vec::new();
    let legend_chars = BOX_ALGEBRA
        .iter()
        .map(|(c, _)| *c)
        .chain(MATERIALS.iter().map(|m| m.glyph()))
        .chain(MARKS.iter().map(|k| k.glyph()))
        .collect::<Vec<_>>();
    for ch in legend_chars {
        let ink = glyph_ink(&mut atlas, ch);
        if ink.is_none() || (tofu.is_some() && ink == tofu) {
            missing.push(ch);
        }
    }
    missing
}

#[test]
fn every_canonical_glyph_rasterizes_in_the_pane_face() {
    assert!(PANE_FACE.is_mono(), "the pane face must be monospace");
    let missing = uncovered_glyphs(PANE_FACE);
    assert!(
        missing.is_empty(),
        "{} cannot draw these legend glyphs: {missing:?}",
        PANE_FACE.label()
    );
}

#[test]
fn face_coverage_is_recorded_for_every_mono_candidate() {
    // A face swap must be a decision with a receipt, not a surprise on glass.
    for face in [
        TypeFace::IosevkaFixed,
        TypeFace::JetBrainsMono,
        TypeFace::Rajdhani,
        TypeFace::Tektur,
        TypeFace::ReemKufi,
        TypeFace::Cinzel,
        TypeFace::CormorantGaramond,
        TypeFace::EbGaramond,
        TypeFace::Amiri,
    ] {
        let missing = uncovered_glyphs(face);
        println!(
            "{:<20} mono={:<5} missing {:>2}: {missing:?}",
            face.label(),
            face.is_mono(),
            missing.len()
        );
    }
}

#[test]
fn glyph_lowering_draws_the_authored_ascii_not_fill_quads() {
    let (grid, legend) = vault();
    let mut atlas = FontAtlas::init(PANE_FACE.bytes(), 15.0);
    let mut dl = DrawList::new_boxed();
    let pitch = atlas.cell_advance();
    let origin = UiRect::new(0, 0, 0, 0);

    let stats = lower_patex_glyphs(
        &grid,
        &mut dl,
        &mut atlas,
        &legend,
        origin,
        pitch,
        pitch * 2,
        AbsenceIndex5D::FULL,
        &PatexPalette::CANONICAL,
    );
    assert!(stats.is_complete(), "no cell may be lost silently: {stats:?}");
    assert_eq!(stats.unmapped, 0, "every bound byte projects back to a glyph");
    assert_eq!(stats.unrenderable, 0, "the face draws every legend glyph");
    assert_eq!(stats.refused, 0, "the arena must hold the whole pane");
    assert_eq!(dl.dropped, 0);
    assert_eq!(dl.glyph_count as u32, stats.glyphs, "one glyph per lowered cell");

    // The quad path and the glyph path must agree on WHICH cells survive —
    // they differ only in what they emit.
    let mut qdl = DrawList::new_boxed();
    let quads = lower_patex(
        &grid,
        &mut qdl,
        origin,
        pitch,
        pitch * 2,
        AbsenceIndex5D::FULL,
        &PatexPalette::CANONICAL,
    );
    assert_eq!(stats.glyphs, quads.quads, "the two renderers cover the same cells");
    assert_eq!(stats.blank, quads.blank);
    assert_eq!(stats.filtered, quads.filtered);
}

#[test]
fn glyph_lowering_honours_the_same_early_out() {
    let (grid, legend) = vault();
    let mut atlas = FontAtlas::init(PANE_FACE.bytes(), 15.0);
    let mut dl = DrawList::new_boxed();
    let stats = lower_patex_glyphs(
        &grid,
        &mut dl,
        &mut atlas,
        &legend,
        UiRect::new(0, 0, 0, 0),
        12_000,
        24_000,
        lane_mask(LANE_MATERIAL),
        &PatexPalette::CANONICAL,
    );
    assert!(stats.early_out);
    assert_eq!(stats.glyphs, 0);
    assert_eq!(dl.glyph_count, 0, "the early-out must not touch the glyph arena");
    assert_eq!(dl.cmd_count, 0);
}

#[test]
fn an_empty_legend_reports_unmapped_rather_than_drawing_nothing_quietly() {
    // A grid built through the canonical legend, then lowered through a legend
    // that cannot name any of its bytes: every cell must be COUNTED as unmapped,
    // not silently absent from the frame.
    let (grid, _) = vault();
    let empty = PatexLegend::new();
    let mut atlas = FontAtlas::init(PANE_FACE.bytes(), 15.0);
    let mut dl = DrawList::new_boxed();
    let stats = lower_patex_glyphs(
        &grid,
        &mut dl,
        &mut atlas,
        &empty,
        UiRect::new(0, 0, 0, 0),
        12_000,
        24_000,
        AbsenceIndex5D::FULL,
        &PatexPalette::CANONICAL,
    );
    assert_eq!(stats.glyphs, 0);
    assert!(stats.unmapped > 0, "an unnameable byte must be counted, never swallowed");
    assert!(!stats.is_complete(), "a pane that lost cells must not read as complete");
    assert_eq!(dl.glyph_count, 0);
}

/// Independent oracle: `pentaract_march_5d.wgsl::check_absence_5d`, transcribed
/// verbatim. Nothing here calls back into `AbsenceIndex5D` — that is the point.
fn wgsl_check_absence_5d(mask: [u32; 8], cell_idx: u32) -> bool {
    if cell_idx >= 243 {
        return false;
    }
    let word_idx = cell_idx >> 5;
    let bit_idx = cell_idx & 31;
    let vec_idx = word_idx >> 2;
    let comp_idx = word_idx & 3;
    (mask[(vec_idx * 4 + comp_idx) as usize] & (1u32 << bit_idx)) != 0
}

#[test]
fn gpu_word_projection_is_a_bijection() {
    // Every interior state, one at a time, survives the round trip.
    for b in 0u16..256 {
        let mut ix = AbsenceIndex5D::EMPTY;
        ix.set(TritCell5D(b as u8));
        let round = AbsenceIndex5D::from_gpu_words(ix.to_gpu_words());
        assert_eq!(round, ix, "state {b} did not survive the GPU word layout");
    }
    // And a populated real pane.
    let (grid, _) = vault();
    let ix = grid.index();
    assert_eq!(AbsenceIndex5D::from_gpu_words(ix.to_gpu_words()), ix);
    assert_eq!(AbsenceIndex5D::from_gpu_words([0; 8]), AbsenceIndex5D::EMPTY);
    assert_eq!(AbsenceIndex5D::from_gpu_words([u32::MAX; 8]), AbsenceIndex5D::FULL);
}

#[test]
fn the_shader_reads_the_same_bits_this_crate_writes() {
    // Singleton sweep: one state set at a time, so ANY word or bit permutation
    // in the projection shows up rather than hiding behind a dense mask.
    for b in 0u16..243 {
        let cell = TritCell5D(b as u8);
        let mut ix = AbsenceIndex5D::EMPTY;
        ix.set(cell);
        let gpu = ix.to_gpu_words();
        for probe in 0u16..243 {
            assert_eq!(
                probe == b,
                wgsl_check_absence_5d(gpu, probe as u32),
                "state {b} set on the CPU, kernel saw {probe} differently"
            );
        }
    }

    // And a real dense pane agrees state-for-state.
    let (grid, _) = vault();
    let gpu = grid.index().interior_only().to_gpu_words();
    for b in 0u16..243 {
        assert_eq!(
            grid.index().contains(TritCell5D(b as u8)),
            wgsl_check_absence_5d(gpu, b as u32),
            "CPU and the WGSL kernel disagree about interior state {b}"
        );
    }
}

#[test]
fn interior_only_strips_what_the_shader_would_never_honour() {
    let (grid, _) = vault();
    // The vault is full of held-blank cells, so the sentinel bit IS set.
    assert!(grid.index().contains(TritCell5D(PATEX_HELD_BLANK)));
    let interior = grid.index().interior_only();
    assert!(!interior.contains(TritCell5D(PATEX_HELD_BLANK)));
    assert_eq!(INTERIOR_MASK.population(), 243);
    assert!(!INTERIOR_MASK.intersects(sentinel_mask()));
    assert_eq!(INTERIOR_MASK.union(sentinel_mask()), AbsenceIndex5D::FULL);

    // The shader's domain guard and interior_only must agree at the boundary.
    let gpu = grid.index().to_gpu_words();
    for b in 243u32..256 {
        assert!(!wgsl_check_absence_5d(gpu, b), "the kernel must refuse state {b}");
    }
}

#[test]
fn the_gpu_mask_is_two_sixteen_byte_rows() {
    assert_eq!(core::mem::size_of::<[u32; 8]>(), 32, "two vec4<u32> rows");
    assert_eq!(core::mem::size_of::<[u32; 8]>() % 16, 0, "the 16B GPU-share property");
    assert_eq!(core::mem::size_of::<AbsenceIndex5D>(), 32, "same 256 bits either side");
}

// ── Projections ─────────────────────────────────────────────────────────────

#[test]
fn the_top_plane_round_trips_to_the_authored_pane() {
    let (grid, _) = vault();
    let ex = PatexExtrude::CANONICAL;
    let p = project_patex(&grid, ProjectionPlane::Top, &ex).expect("Top is a rect plane");
    let (top, st) = (p.pane, p.stats);
    assert_eq!(top.cols(), grid.cols());
    assert_eq!(top.rows(), grid.rows());
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            assert_eq!(
                top.cell(col, row),
                grid.cell(col, row),
                "the floor plan IS the authored sheet at {col}.{row}"
            );
        }
    }
    assert_eq!(st.collapsed, 0, "a plan view stacks nothing");
    assert_eq!(st.off_sheet, 0);
    assert_eq!(st.cells_in, st.cells_drawn, "one in, one out");
}

#[test]
fn elevations_drop_the_axis_they_should_and_stand_things_up() {
    let (grid, _) = vault();
    let ex = PatexExtrude::CANONICAL;

    let fp = project_patex(&grid, ProjectionPlane::Front, &ex).expect("Front");
    let (front, fs) = (fp.pane, fp.stats);
    assert_eq!(front.cols(), grid.cols(), "front elevation keeps columns");
    assert_eq!(front.rows(), ex.max_height() as usize + 1, "and is as tall as the tallest cell");

    let side = project_patex(&grid, ProjectionPlane::Side, &ex).expect("Side").pane;
    assert_eq!(side.cols(), grid.rows(), "side elevation swaps rows in for columns");

    // 966 source cells collapse onto 69 columns, so the flattening MUST be
    // counted — that is the whole reason the receipt exists.
    assert!(fs.collapsed > 0, "an elevation of 14 rows onto one MUST collapse");
    assert!(fs.cells_drawn < fs.cells_in, "and draw fewer cells than it consumed");
}

#[test]
fn a_wall_stands_taller_than_a_floor_mark_in_elevation() {
    let legend = PatexLegend::canonical();
    let ex = PatexExtrude::CANONICAL;
    let (walls, _) = PatexGrid::rasterize("│││", &legend);
    let (floor, _) = PatexGrid::rasterize("...", &legend);

    let we = project_patex(&walls, ProjectionPlane::Front, &ex).expect("Front").pane;
    let fe = project_patex(&floor, ProjectionPlane::Front, &ex).expect("Front").pane;

    let occupied = |g: &PatexGrid| -> usize {
        (0..g.rows()).filter(|r| !g.cell(0, *r).is_sentinel()).count()
    };
    assert_eq!(occupied(&we), ex.topology as usize + 1, "a wall fills its courses plus the ground");
    assert_eq!(occupied(&fe), 1, "a floor mark occupies the ground line only");
    assert!(occupied(&we) > occupied(&fe));
}

#[test]
fn depth_shading_makes_a_solid_elevation_readable_again() {
    // An elevation of a closed volume is a solid wall — correct, and unreadable.
    // Depth is what turns it back into a drawing, so it must actually vary.
    let (grid, _) = vault();

    // Uncut, the vault's front wall occludes the whole room — every winning
    // cell comes from depth 0. That is CORRECT for a closed box and useless on
    // a sheet, which is exactly why a drafter cuts a section.
    let flat = project_patex(&grid, ProjectionPlane::Front, &PatexExtrude::CANONICAL)
        .expect("Front");
    let flat_depths: std::collections::BTreeSet<u8> = (0..flat.pane.rows())
        .flat_map(|r| (0..flat.pane.cols()).map(move |c| (c, r)))
        .filter(|(c, r)| !flat.pane.cell(*c, *r).is_sentinel())
        .map(|(c, r)| flat.depth[r][c])
        .collect();
    assert_eq!(flat_depths.len(), 1, "a closed box shows one surface: its front wall");

    // Cut the near courses away and the interior comes onto the sheet.
    let p = project_patex_cut(&grid, ProjectionPlane::Front, &PatexExtrude::CANONICAL, 3)
        .expect("Front section");
    assert!(p.depth_max > 0, "a sectioned pane must project more than one depth");

    let mut inks = std::collections::BTreeSet::new();
    let mut depths = std::collections::BTreeSet::new();
    for row in 0..p.pane.rows() {
        for col in 0..p.pane.cols() {
            if p.pane.cell(col, row).is_sentinel() {
                continue;
            }
            depths.insert(p.depth[row][col]);
            inks.insert(p.shaded_ink(col, row, &PatexPalette::CANONICAL, 3_000));
        }
    }
    assert!(depths.len() > 1, "every cell came from the same depth — the sort is not running");
    assert!(inks.len() > 1, "depth shading produced one flat tone, so the slab stays a slab");

    // The nearest surface must never be dimmer than the deepest one.
    let near = p.shaded_ink(0, p.pane.rows() - 1, &PatexPalette::CANONICAL, 3_000);
    assert_eq!(near & 0xFF, 0xFF, "shading must not touch alpha");
}

#[test]
fn iso_is_refused_as_a_rect_plane() {
    let (grid, _) = vault();
    assert!(
        project_patex(&grid, ProjectionPlane::Iso, &PatexExtrude::CANONICAL).is_none(),
        "a projected cuboid is a hexagon of parallelograms, never a rect"
    );
}

#[test]
fn a_projected_pane_keeps_the_early_out() {
    let (grid, _) = vault();
    let front = project_patex(&grid, ProjectionPlane::Front, &PatexExtrude::CANONICAL)
        .expect("Front")
        .pane;
    assert!(front.any_of(lane_mask(LANE_TOPOLOGY)), "walls survive the projection");
    assert!(!front.any_of(lane_mask(LANE_MATERIAL)), "and no fill appears from nowhere");
}

#[test]
fn the_extrude_profile_reads_height_off_the_lane() {
    let ex = PatexExtrude::CANONICAL;
    let legend = PatexLegend::canonical();
    assert_eq!(ex.height_of(TritCell5D(legend.lookup('│').expect("wall"))), ex.topology);
    assert_eq!(ex.height_of(TritCell5D(legend.lookup('.').expect("floor"))), ex.mark);
    assert_eq!(ex.height_of(Material::Rock.cell()), ex.material, "full occlusion stands full");
    assert_eq!(ex.height_of(Material::Void.cell()), 0, "open air stands not at all");
    assert!(ex.height_of(Material::Mist.cell()) < ex.material, "mist stands lower than rock");
    assert_eq!(ex.height_of(TritCell5D(PATEX_HELD_BLANK)), 0, "a sentinel stands nothing");
}

// ── Axonometric ─────────────────────────────────────────────────────────────

#[test]
fn the_axon_angle_is_two_ternary_steps_of_a_27_division_circle() {
    assert_eq!(3u32.pow(AXON_CIRCLE_TRITS), 27, "3 trits quantize the circle into 27");
    // 2 of 27 steps is 26.667 degrees; a 1:2 slope is atan(1/2) = 26.565 degrees.
    // They agree to a tenth of a degree, which is why the slope is 1:2 and the
    // projection stays exact in integers.
    assert_eq!((AXON_RUN, AXON_RISE), (2, 1), "1:2 slope");
    let step_millideg = 360_000 / 27;
    assert_eq!(AXON_STEPS * step_millideg, 26_666, "2 steps = 26.666 deg");
}

#[test]
fn every_face_is_a_distinct_trit() {
    let trits: Vec<i8> = AxonFace::ALL.iter().map(|f| f.trit()).collect();
    assert_eq!(trits, vec![-1, 0, 1], "the faces ARE the balanced digit set");
    assert!(AxonFace::Top.shade_pmy() > AxonFace::Right.shade_pmy());
    assert!(AxonFace::Right.shade_pmy() > AxonFace::Left.shade_pmy());
}

fn axon_of(src: &str, w: u32, h: u32) -> (PixelBuffer, AxonStats) {
    let legend = PatexLegend::canonical();
    let (grid, _) = PatexGrid::rasterize(src, &legend);
    let mut buf = PixelBuffer::new(w, h);
    buf.clear(0x00000000);
    let st = render_axon(
        &grid,
        &PatexExtrude::CANONICAL,
        &PatexPalette::CANONICAL,
        &mut buf,
        (60, 20),
        16,
        8,
    );
    (buf, st)
}

#[test]
fn one_extruded_cell_shows_three_shaded_faces() {
    let (buf, st) = axon_of("│", 160, 120);
    assert_eq!(st.cells_in, 1);
    assert_eq!(st.cells_drawn, 1);
    assert_eq!(st.faces_painted, 3, "a standing cell shows exactly three faces");
    assert!(st.subsamples_lit > 0);

    let mut shades = std::collections::BTreeSet::new();
    for px in buf.data.chunks_exact(4) {
        if px[3] != 0 {
            shades.insert([px[0], px[1], px[2]]);
        }
    }
    assert!(shades.len() >= 3, "top/right/left must differ in shade, got {}", shades.len());
}

#[test]
fn coverage_gives_real_partial_pixels_not_a_hard_edge() {
    // The point of scoring on the ternary sub-lattice: an edge pixel lands on a
    // true third or two-thirds, so there MUST be tones between background and
    // the three flat face shades. A binary in/out rasterizer produces none.
    let (buf, _) = axon_of("│", 160, 120);
    let mut lit = std::collections::BTreeMap::new();
    for px in buf.data.chunks_exact(4) {
        if px[3] != 0 {
            *lit.entry([px[0], px[1], px[2]]).or_insert(0u32) += 1;
        }
    }
    assert!(
        lit.len() > 3,
        "only {} tones — the coverage blend is not running, edges are hard",
        lit.len()
    );
}

#[test]
fn a_flat_mark_shows_only_its_top_face() {
    let (_, st) = axon_of(".", 160, 120);
    assert_eq!(st.cells_in, 1);
    assert_eq!(st.faces_painted, 1, "nothing standing means no side walls");
}

#[test]
fn the_axon_is_deterministic() {
    let (a, sa) = axon_of("┌─┐\n│.│\n└─┘", 200, 160);
    let (b, sb) = axon_of("┌─┐\n│.│\n└─┘", 200, 160);
    assert_eq!(sa, sb, "same pane, same receipt");
    assert_eq!(a.data, b.data, "same pane, same pixels");
}

#[test]
fn an_off_sheet_pane_paints_nothing_and_says_so() {
    let legend = PatexLegend::canonical();
    let (grid, _) = PatexGrid::rasterize("│││", &legend);
    let mut buf = PixelBuffer::new(32, 32);
    buf.clear(0);
    let st = render_axon(
        &grid,
        &PatexExtrude::CANONICAL,
        &PatexPalette::CANONICAL,
        &mut buf,
        (9_000, 9_000),
        16,
        8,
    );
    assert_eq!(st.cells_drawn, 0);
    assert_eq!(st.off_sheet, st.cells_in, "every cell must be accounted off-sheet");
    assert!(buf.data.iter().all(|b| *b == 0), "and not a pixel touched");
}

#[test]
fn out_of_bounds_reads_are_held_blank_not_a_panic() {
    let (grid, _) = vault();
    assert_eq!(grid.cell(999, 0).0, PATEX_HELD_BLANK);
    assert_eq!(grid.cell(0, 999).0, PATEX_HELD_BLANK);
    assert!(grid.cell(999, 999).is_sentinel());
}
