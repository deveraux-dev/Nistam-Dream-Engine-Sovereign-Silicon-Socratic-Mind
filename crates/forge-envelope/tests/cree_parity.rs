//! The 3-wave sovereign lexicon has two enforcement points — this crate, and the Python
//! hub that is the only path actually dispatching to Vertex. Editing one alone reds here.

use forge_envelope::CreeLinguisticFilter;

const HUB: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/scripts/vertex_flash_cache.py"
);

/// Every double-quoted string in the `NAME = [ ... ]` literal, in source order.
fn python_list(source: &str, name: &str) -> Vec<String> {
    let head = source
        .find(&format!("\n{name} = ["))
        .unwrap_or_else(|| panic!("{name} is not declared in {HUB}"));
    let open = source[head..].find('[').unwrap() + head;
    let close = source[open..]
        .find(']')
        .unwrap_or_else(|| panic!("{name} literal is unterminated"))
        + open;

    let mut out = Vec::new();
    let mut rest = &source[open + 1..close];
    while let Some(start) = rest.find('"') {
        let tail = &rest[start + 1..];
        let end = tail
            .find('"')
            .unwrap_or_else(|| panic!("unterminated string in {name}"));
        out.push(tail[..end].to_string());
        rest = &tail[end + 1..];
    }
    out
}

fn assert_same_lexicon(wave: &str, rust: &[&str], python: &[String]) {
    let mut rust_sorted: Vec<&str> = rust.to_vec();
    rust_sorted.sort_unstable();
    let dupes = rust_sorted.len();
    rust_sorted.dedup();
    assert_eq!(dupes, rust_sorted.len(), "{wave}: Rust side repeats a token");

    let mut py_sorted: Vec<&str> = python.iter().map(String::as_str).collect();
    py_sorted.sort_unstable();
    let dupes = py_sorted.len();
    py_sorted.dedup();
    assert_eq!(dupes, py_sorted.len(), "{wave}: Python side repeats a token");

    let only_rust: Vec<&&str> = rust_sorted.iter().filter(|t| !py_sorted.contains(t)).collect();
    let only_py: Vec<&&str> = py_sorted.iter().filter(|t| !rust_sorted.contains(t)).collect();
    assert!(
        only_rust.is_empty() && only_py.is_empty(),
        "{wave} drifted — the in-process gate and the dispatching gate disagree.\n  \
         only in cree_validator.rs: {only_rust:?}\n  \
         only in vertex_flash_cache.py: {only_py:?}"
    );
}

#[test]
fn the_three_waves_are_identical_on_both_sides_of_the_airgap() {
    let source = std::fs::read_to_string(HUB).expect("the Vertex hub must be readable");

    assert_same_lexicon(
        "Wave 1 (syllabic & phonemic)",
        CreeLinguisticFilter::WAVE_1_PHONEMIC_MARKERS,
        &python_list(&source, "WAVE_1_PHONEMIC_MARKERS"),
    );
    assert_same_lexicon(
        "Wave 2 (ghost words)",
        CreeLinguisticFilter::WAVE_2_GHOST_WORDS,
        &python_list(&source, "WAVE_2_GHOST_WORDS"),
    );
    assert_same_lexicon(
        "Wave 3 (sacred sentinels & OCAP)",
        CreeLinguisticFilter::WAVE_3_SACRED_SENTINELS,
        &python_list(&source, "WAVE_3_SACRED_SENTINELS"),
    );
}

#[test]
fn the_hub_refuses_what_the_validator_refuses() {
    let source = std::fs::read_to_string(HUB).expect("the Vertex hub must be readable");
    let filter = CreeLinguisticFilter::new();

    for wave in [
        "WAVE_1_PHONEMIC_MARKERS",
        "WAVE_2_GHOST_WORDS",
        "WAVE_3_SACRED_SENTINELS",
    ] {
        for token in python_list(&source, wave) {
            assert!(
                filter.validate_text(&token).is_refused(),
                "{wave} token {token:?} passes the Rust gate but the hub blocks it"
            );
        }
    }
}

#[test]
fn the_validator_itself_is_barred_from_cloud_transit() {
    let source = std::fs::read_to_string(HUB).expect("the Vertex hub must be readable");
    let blocked = python_list(&source, "SOVEREIGN_BLOCKED_PATTERNS");
    for path in ["cree_validator.rs", "cree_grammar.rs", "gemma-s13"] {
        assert!(
            blocked.iter().any(|p| p == path),
            "{path} must stay on the sovereign side of the airgap"
        );
    }
}
