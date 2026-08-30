//! Gate for the proof-tag hole found 2026-08-29: `book_drift.rs:132` matched
//! only the bare `[PROVEN]`, so 34 colon-form tags in one chapter were invisible
//! and their dead paths were downgraded from FATAL to WARN.

use forge_foreman_v3::claim::proof_tags;

#[test]
fn the_bare_form_is_still_recognised() {
    let t = proof_tags("the clock is real [PROVEN] and holds");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].inline_path, None, "bare form carries no inline path");
}

#[test]
fn the_colon_form_is_recognised_and_yields_its_path() {
    // Verbatim shape from 11-sovereign-routing-topology.md:96.
    let line = "instantiates the dual-clock architecture [PROVEN:crates/GEMINI.md:8].";
    let t = proof_tags(line);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].inline_path.as_deref(), Some("crates/GEMINI.md"));
}

#[test]
fn one_line_can_carry_several_tags() {
    let line = "[PROVEN:crates/GEMINI.md:8] drives physics [PROVEN:crates/GEMINI.md:10] renders";
    let t = proof_tags(line);
    assert_eq!(t.len(), 2, "got {t:?}");
    assert!(t.iter().all(|x| x.inline_path.as_deref() == Some("crates/GEMINI.md")));
}

#[test]
fn mixed_forms_on_one_line_both_land() {
    let t = proof_tags("[PROVEN] and also [PROVEN:crates/a/b.rs:3]");
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].inline_path, None);
    assert_eq!(t[1].inline_path.as_deref(), Some("crates/a/b.rs"));
}

#[test]
fn a_drive_colon_survives_the_line_suffix_strip() {
    let t = proof_tags("[PROVEN:F:\\v3\\crates\\x.rs:42]");
    assert_eq!(t[0].inline_path.as_deref(), Some("F:\\v3\\crates\\x.rs"));
}

#[test]
fn a_path_with_no_line_number_is_kept_whole() {
    let t = proof_tags("[PROVEN:.forge/waves.tsv]");
    assert_eq!(t[0].inline_path.as_deref(), Some(".forge/waves.tsv"));
}

#[test]
fn near_misses_are_not_tags() {
    assert!(proof_tags("[PROVENANCE] is a different word").is_empty());
    assert!(proof_tags("PROVEN without brackets").is_empty());
    assert!(proof_tags("").is_empty());
}

#[test]
fn an_unterminated_tag_reports_nothing_rather_than_guessing() {
    // The orphan pattern this tree already uses: report, never invent.
    assert!(proof_tags("[PROVEN:crates/a.rs — someone forgot the bracket").is_empty());
}

#[test]
fn an_empty_colon_body_yields_no_path_not_an_empty_one() {
    let t = proof_tags("[PROVEN:]");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].inline_path, None);
}
