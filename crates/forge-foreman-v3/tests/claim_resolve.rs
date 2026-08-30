//! Gate for Gate 2's resolver: the scanner that finds a path anchor, and the
//! predicate that says it points outside this tree. Pins the exact live
//! violations found 2026-08-29 so a fix cannot silently un-fix.

use forge_foreman_v3::claim::{is_foreign_root, root_correction, scan_line, CandidateKind};

#[test]
fn the_eight_live_foreign_root_violations_are_all_caught() {
    // Every one receipted 2026-08-29. If any goes false, Gate 2 has a hole.
    let live = [
        "F:/NewRepo/crates/ffi-ui-assimilator-001/corpora/astro/astro_concepts.json",
        "F:/NewRepo/crates/forge-audio",
        "F:\\NewRepo\\crates\\forge-book\\src\\tablets",
        "F:\\13forge-super\\_merged\\reposold\\_plans",
        "E:\\snapshots\\newrepo-2026-06-20\\_plans",
        "G:\\E DRIVE\\v3\\_attic\\2026-08-19\\scc-golden-vixi-kits-stubs",
        "C:\\Users\\seanm\\Desktop",
        "F:\\NewRepo\\.forge\\river.idx",
    ];
    for p in live {
        assert!(is_foreign_root(p), "must be flagged foreign: {p}");
    }
}

#[test]
fn nothing_inside_this_tree_is_ever_flagged() {
    let ours = [
        "F:/v3/crates/forge-core-v3/src/lib.rs",
        "F:\\v3\\shell\\src\\main.rs",
        "F:/v3/.forge/grind-log/forge-wright.md",
        "crates/forge-daemon-door/src/door.rs",
        ".forge/handoffs/HANDOFF-2026-08-29.md",
        "shell/src/tabs.rs",
        "xtask/src/main.rs",
    ];
    for p in ours {
        assert!(!is_foreign_root(p), "must NOT be flagged: {p}");
    }
}

#[test]
fn a_v3_path_that_merely_mentions_a_foreign_name_is_not_foreign() {
    // The root must be the PREFIX. A file about NewRepo is not in NewRepo.
    assert!(!is_foreign_root("F:/v3/docs/porting-from-NewRepo.md"));
    assert!(!is_foreign_root("crates/forge-book-v3/src/newrepo_notes.rs"));
}

#[test]
fn the_astro_harvest_line_is_scanned_and_flagged_end_to_end() {
    // The literal from forge-astro-harvest-v3/src/main.rs:31.
    let line = r#"    let astro_concepts_path = "F:/NewRepo/crates/ffi-ui-assimilator-001/corpora/astro/astro_concepts.json";"#;
    let found = scan_line(line);
    let drive: Vec<_> = found.iter().filter(|c| c.kind == CandidateKind::Drive).collect();
    assert_eq!(drive.len(), 1, "one drive anchor expected, got {found:?}");
    assert!(
        is_foreign_root(&drive[0].text),
        "the astro-harvest literal must trip the gate: {}",
        drive[0].text
    );
}

#[test]
fn the_doctor_fallback_line_is_caught_too() {
    // forge-audio-v3/src/doctor.rs:76 — the silent fallback into v2.
    let line = r#"if Path::new(rel).exists() { rel.to_string() } else { "F:/NewRepo/crates/forge-audio".to_string() }"#;
    let found = scan_line(line);
    assert!(
        found.iter().any(|c| is_foreign_root(&c.text)),
        "doctor.rs fallback must be caught: {found:?}"
    );
}

#[test]
fn a_dead_root_still_offers_its_receipted_correction() {
    let (fixed, receipt) = root_correction("E:\\airgap\\x").expect("C-1 applies");
    assert_eq!(fixed, "E:\\.airgap\\x");
    assert!(receipt.contains("2026-08-17"));
}

#[test]
fn scanning_is_pure_and_touches_no_disk() {
    // A path that cannot exist anywhere still scans and classifies.
    let c = scan_line("cite F:\\v3\\does\\not\\exist\\anywhere.rs here");
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].kind, CandidateKind::Drive);
    assert_eq!(c[0].text, "F:\\v3\\does\\not\\exist\\anywhere.rs");
}
