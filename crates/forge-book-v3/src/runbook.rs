//! The Runbook section — operator guides for the studio's live systems, authored
//! as BOOK chapters (not source-doc). First guide: the semantic codebook — the
//! GhostMoon 5D embedding (banks lead, water refines). Facts mirror
//! `forge_ml::nearest_neighbor`; the proof column is honest, never false-stamped.
//! Second guide: the river sweep — the 5D health+consolidation loop that keeps the
//! four river sources (spine/bed/exhaust/ledger) honest. Mirrors the machine-code
//! source of truth at `.claude/skills/riversweep/SKILL.md`.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// Build the "Runbook Guide" chapter: the semantic-codebook runbook as a book
/// page — the map, the law, the three rungs, how to invoke, and proof status.
pub fn runbook_guide() -> Chapter {
    let mut chapter = Chapter::new("Runbook Guide", AtlasSection::Runbook);
    chapter.add_lore(
        "The semantic codebook maps a line of text into the GhostMoon 5D box so a ray \
         retrieves by MEANING, not merely by literal token identity.",
    );
    chapter.add_lore(
        "The law: the discrete Cree family and role are the BANKS and lead — the language \
         cuts the channel. The continuous projection is the WATER and only orders WITHIN a \
         cell; it can never breach a bank. The machine surveys; the language is the source.",
    );

    let mut map = Page::new(1);
    map.add(Block::text("THE MAP — 5 lanes [x, y, z, theta, w] = 0..4:"));
    map.add(Block::text("  lane 2 (z)     = FAMILY  - coarse meaning, linear, dominant (FAMILY_STEP)."));
    map.add(Block::text("  lane 3 (theta) = ROLE    - rotation, wrap-aware (ROLE_STEP)."));
    map.add(Block::text("  lanes 0,1,4    = REFINE  - projection/identity, clamped to +/-IDENTITY_BAND."));
    map.add(Block::text("Invariant: one family step^2 > the whole identity+role band, so meaning wins."));
    chapter.add_page(map);

    let mut rungs = Page::new(2);
    rungs.add(Block::text("THE RUNGS (offline, deterministic, integer; all built on map5_refine):"));
    rungs.add(Block::text("  R1 embed_river_semantic_lexical(line)          - refine = tag/payload/token-set hash."));
    rungs.add(Block::text("  R2 embed_river_semantic_distributional(l, idf) - refine = TF x IDF signed random projection."));
    rungs.add(Block::text("  R3 embed_river_semantic_model(l, coder, idf)   - refine = learned 512-bit BQ code;"));
    rungs.add(Block::text("     coder = None falls back to R2 (Student -> Oracle ladder). Family/role always"));
    rungs.add(Block::text("     come from the line's own words, never from the model."));
    chapter.add_page(rungs);

    let mut run = Page::new(3);
    run.add(Block::text("INVOKE: door tool  raycast { from, toward, embedding: \"semantic\" }  (R1, live)."));
    run.add(Block::text("Codebook builders: load_river_codebook_semantic[_from_disk] (forge_ml::nearest_neighbor)."));
    run.add(Block::text("PROOF: R1/R2/R3 recall oracles green + live raycast readback"));
    run.add(Block::text("       (forge-daemon repo_query::semantic_raycast_tests)."));
    run.add(Block::text("UNVERIFIED: R3 real model forward (text -> code) - SovereignCoder is the seam,"));
    run.add(Block::text("            weights not wired, so R3 falls back to R2. Never false-stamped."));
    chapter.add_page(run);

    chapter
}

/// Build the "Wave Runbook" chapter — the long-running board-clearing loop as a book
/// page (Sean 08-02, "whats the runbook"). Law bodies stay compiled — session_cadence,
/// oracle1_governor, massweld — this page points at them, never restates.
pub fn wave_runbook() -> Chapter {
    let mut chapter = Chapter::new("Wave Runbook — Board Clearing", AtlasSection::Runbook);
    chapter.add_lore(crate::session_cadence::DONE_INVARIANT);
    let mut rungs = Page::new(1);
    rungs.add(Block::text("PRIME  - blast, fly PARK verbatim, sidecar status quoted; BUILD != stamped exe -> redeploy before anything."));
    rungs.add(Block::text("ROUTE  - route --take 10; SATURATED = Sean names the block; Tier-1 keystones first; rows carry [lane:][loc:][roi:] or they route paid; TAGGED-ON-DISK -> harvest, never rebuild."));
    rungs.add(Block::text("WAVE   - 1 Floor + 1 Circuit + 1 Surface (session_cadence, never 3 Floors); lane manifest TSV, two Xs one row = massweld refuses; orient quoted -> massread module sweep -> verify/dry -> land --row-gate; [BOARD: id] tags ride the welds."));
    rungs.add(Block::text("GATES  - iterate = check + filtered tests (debug-lane); land = full crate gate; background cargo builds in target/weld; a held lock is a live sibling - Wait-Process, never kill."));
    rungs.add(Block::text("CLOSE  - declare BoardTask rows BEFORE their tags harvest; harvest --lane gemma --timeout 0 per touched crate; flip --why for proven-untagged keystones; bin stamp; reseal; one SURFACE receipt."));
    rungs.add(Block::text("REDS   - anchor miss = concurrent writer, fresh-read + re-fire; GATE RED = the loop working; board exit 77 = queue + YIELD; ABSENT without pull_gate = silence."));
    chapter.add_page(rungs);
    chapter
}

/// Build the "River Sweep Runbook" chapter: the 5D health+consolidation loop over
/// the four river sources, as a book page — the law, the five axes, act-vs-recommend,
/// how to invoke, and the proof. Mirrors `.claude/skills/riversweep/SKILL.md`.
pub fn river_sweep_guide() -> Chapter {
    let mut chapter = Chapter::new("River Sweep Runbook", AtlasSection::Runbook);
    chapter.add_lore(
        "The river sweep keeps the four sources honest: river.idx (the plan spine), \
         riverbed.idx (the word bed), river.evt (forensic exhaust), and the rivercanon \
         ledger. It gauges all five axes, acts only on the preserve-safe wins, and names \
         the gated verb for everything else.",
    );
    chapter.add_lore(
        "The law: live state is the only authority — gauge every axis, parrot none. Prose, \
         memory, and the BUILD row are not truth. A live door over a flooded spine still \
         LIES, so purity (D2) gates liveness (D1). Every mutation is preserve-first: archive \
         before prune, never blind-delete (delete is Sean's one HARD gate).",
    );
    chapter.add_lore(
        "Born of the 2026-07-11 regression: query-result grains flooded river.idx to 79% \
         exhaust and the orient ray drowned in grain-noise. Law-B routed SPILL to river.evt, \
         the spine went plan-only, and this loop is the mechanical catch so it is never \
         unseen again.",
    );

    let mut axes = Page::new(1);
    axes.add(Block::text("THE 5 AXES [D1..D5]:"));
    axes.add(Block::text("  D1 LIVE   - door ping->pong, daemon_health ok, raycast self-check; down = disk-oracle."));
    axes.add(Block::text("  D2 PURITY - river.idx plan-only, SPILL == 0 (law-B); exhaust% = SPILL/total. D2 gates D1."));
    axes.add(Block::text("  D3 FLOOD  - idx <= 8192B, spill/ <= 1MB, evt > 50KB graduates; ledger + _plans prose bloat."));
    axes.add(Block::text("  D4 TWIN   - MAP anchors resolve via query kind=arch (NOT Test-Path); no-twin dedupe."));
    axes.add(Block::text("  D5 DRIFT  - BUILD row vs exe mtime; bed FLAG / CONTRADICTION-OPEN; SEAM-LAW."));
    chapter.add_page(axes);

    let mut acts = Page::new(2);
    acts.add(Block::text("ACT (preserve-first) vs RECOMMEND (gated verb):"));
    acts.add(Block::text("  ACT: ledger FORMAT-LAW - pipe |...| rows archive VERBATIM -> quarantine, strip to TSV."));
    acts.add(Block::text("  ACT: twin dedupe - drain to the live home THEN airgap (_attic + RESTORE); RETIRED != DELETED."));
    acts.add(Block::text("  RECOMMEND: spine SPILL > 0 -> build-door (rebuild + bounce the pre-law-B daemon)."));
    acts.add(Block::text("  RECOMMEND: evt > 50KB / contradictions / long TSV rows -> /rivercanon."));
    acts.add(Block::text("  RECOMMEND: garbage -> Sean-delete. Delete is never the sweep's move."));
    chapter.add_page(acts);

    let mut run = Page::new(3);
    run.add(Block::text("INVOKE: /riversweep  (\"sweep the river\" / \"is the spine clean\" / \"clean the ledger\")."));
    run.add(Block::text("RECEIPT: RIVERSWEEP D1<ok|down> D2<rows@pur% SPILLn> D3<idxB/8192 spillB evtB ledger plans>"));
    run.add(Block::text("         D4<anchors ok|n-stale> D5<door ok|drift · bed F/C@tail>; clean = receipt-line alone."));
    run.add(Block::text("PROOF (dogfood 2026-07-11): spine 79% exhaust -> 100% plan-only (SPILL 78 -> evt);"));
    run.add(Block::text("       ledger 88KB -> 21.7KB (116 pipe rows archived, viol 56 -> 11); door + bed gauged live."));
    run.add(Block::text("SOURCE OF TRUTH: .claude/skills/riversweep/SKILL.md (the 5D machine-code form)."));
    chapter.add_page(run);

    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runbook_guide_is_the_runbook_section() {
        let ch = runbook_guide();
        assert_eq!(ch.title(), "Runbook Guide");
        assert_eq!(ch.section, AtlasSection::Runbook);
        assert!(ch.lore_count() >= 2);
        assert_eq!(ch.page_count(), 3);
    }

    #[test]
    fn runbook_guide_names_the_three_rungs_and_the_law() {
        let ch = runbook_guide();
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in ["R1", "R2", "R3", "map5_refine", "FAMILY", "REFINE", "embedding: \"semantic\""] {
            assert!(text.contains(needle), "runbook guide missing '{needle}'");
        }
    }

    #[test]
    fn river_sweep_guide_is_the_runbook_section() {
        let ch = river_sweep_guide();
        assert_eq!(ch.title(), "River Sweep Runbook");
        assert_eq!(ch.section, AtlasSection::Runbook);
        assert!(ch.lore_count() >= 3);
        assert_eq!(ch.page_count(), 3);
    }

    #[test]
    fn river_sweep_guide_names_the_five_axes_and_the_law() {
        let ch = river_sweep_guide();
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in ["D1", "D2", "D3", "D4", "D5", "SPILL == 0", "D2 gates D1", "/riversweep", "preserve-first"] {
            assert!(text.contains(needle), "river sweep guide missing '{needle}'");
        }
    }
}
