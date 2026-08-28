//! Every authored `.shaderbind.vixi` in the v3 golden corpus must parse.
//! An unparseable authored file reds the gate instead of going silently inert.

use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scc")
        .join("golden")
        .join("vixi")
        .join("shaderbinds")
}

fn corpus_files() -> Vec<PathBuf> {
    let dir = corpus_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("corpus dir unreadable at {}: {e}", dir.display()));
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.to_string_lossy().ends_with(".shaderbind.vixi"))
        .collect();
    out.sort();
    out
}

#[test]
fn the_corpus_is_not_empty() {
    let files = corpus_files();
    assert!(
        !files.is_empty(),
        "no .shaderbind.vixi found in {}",
        corpus_dir().display()
    );
}

#[test]
fn every_authored_shaderbind_parses() {
    let mut failures: Vec<String> = Vec::new();
    for path in corpus_files() {
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{}: unreadable: {e}", path.display()));
                continue;
            }
        };
        if let Err(e) = forge_shaderbind::parse_shaderbind(&src) {
            failures.push(format!("{}: {e:?}", path.display()));
        }
    }
    assert!(failures.is_empty(), "unparseable authored shaderbinds:\n{}", failures.join("\n"));
}

#[test]
fn the_deck_panel_the_shell_includes_is_itself_authored_and_parses() {
    let deck = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("shell")
        .join("panels")
        .join("deck.shaderbind.vixi");
    let src = std::fs::read_to_string(&deck)
        .unwrap_or_else(|e| panic!("deck panel unreadable at {}: {e}", deck.display()));
    let bind = forge_shaderbind::parse_shaderbind(&src)
        .unwrap_or_else(|e| panic!("deck panel does not parse: {e:?}"));
    bind.verify_gates()
        .unwrap_or_else(|e| panic!("deck panel gates refuse: {e:?}"));
}
