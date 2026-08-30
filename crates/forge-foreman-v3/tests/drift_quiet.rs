//! The drift hook is silent on a green turn. The danger of that is a RED turn
//! going silent too, so this pins the one invariant that separates the two:
//! an unarmed tree must never report green.

/// A tree with no `.claude` at all is NOT green: L25 is unarmed there, so the
/// report must say so rather than going quiet.
#[test]
fn an_unarmed_tree_is_never_green() {
    let tmp = std::env::temp_dir().join("forge-drift-quiet-unarmed");
    std::fs::create_dir_all(&tmp).expect("mkdir");
    let r = forge_foreman_v3::drift::run_report_full(&tmp);
    assert!(!r.green, "an unarmed tree must not be green: {}", r.text);
    assert!(r.text.contains("ARM .loop-active:absent"), "{}", r.text);
    assert!(r.text.contains("ARM phase0:current.json MISSING"), "{}", r.text);
    assert!(r.text.contains("DRIFT verdict:"), "{}", r.text);
}
