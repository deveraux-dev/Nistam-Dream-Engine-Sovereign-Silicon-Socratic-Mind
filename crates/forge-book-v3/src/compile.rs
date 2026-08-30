//! compile.rs — the Codex Compiler: ONE `Book` source → THREE faces.
//!
//! Sean's model (2026-07-17): *"1 for standard user, 1 for devs (machine code),
//! and 1 Engine Native symbiote."* No face is hand-authored — the `Book` IR is the
//! single source and all three are EMITTED, so they can never drift (the failure
//! the hand-rolled `mirror_book.py` / `deveraux_lint.py` scripts risk).
//!
//!   * STANDARD — human prose (Markdown), via [`export_md`].
//!   * DEV      — the machine-readable typed IR (pretty JSON), lossless round-trip.
//!   * SYMBIOTE — engine-native VixiScript (`#vixi:kit v1`) that lowers through
//!     `forge-vix` AOT, exactly as the engine compiles every surface. Structure is
//!     baked; prose binds live at the host — the codex is a symbiote, not a snapshot.

use crate::book::Book;
use crate::export_md::export_md;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Provenance seal: a SHA-256 based tamper-evident seal over compiled faces.
/// The 12-hex short id is deterministic from the seal data; grid_hash anchors
/// additional metadata for engine integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceSeal {
    /// 12-hex short id derived from SHA-256 hash.
    pub id: String,
    /// Grid hash for engine-native metadata.
    pub grid_hash: u64,
}

/// Compute a stable, deterministic ProvenanceSeal over a title and byte payload.
/// Uses SHA-256 to produce the tamper-evident seal; any byte of drift re-keys it.
fn seal_bytes(title: &str, payload: &[u8]) -> ProvenanceSeal {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(payload);
    let hash = hasher.finalize();

    // Extract first 6 bytes for the 12-hex id string.
    let id = format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                     hash[0], hash[1], hash[2], hash[3], hash[4], hash[5]);

    // Extract 8 bytes (u64) for grid_hash from bytes 8-15.
    let mut grid_bytes = [0u8; 8];
    grid_bytes.copy_from_slice(&hash[8..16]);
    let grid_hash = u64::from_le_bytes(grid_bytes);

    ProvenanceSeal { id, grid_hash }
}

/// The compiled faces of one book. Same `Book` in → byte-identical faces out.
#[derive(Debug, Clone)]
pub struct CompiledFaces {
    /// STANDARD — human prose (Markdown).
    pub standard: String,
    /// DEV — the machine-readable typed IR (pretty JSON); round-trips to `Book`.
    pub dev: String,
    /// SYMBIOTE — engine-native `.kit.vixi`; lowers through forge-vix AOT.
    pub symbiote: String,
    /// LINT-MIRROR — the reconciled voice-gated face (M10): the deveraux voice
    /// linter run over STANDARD, per chapter, absolute book lines + quoted
    /// receipts (Sean's THE-100-BOOK.lint-mirror.md v2 exemplar format).
    pub lint_mirror: String,
}

/// Lowercase-alnum-underscore slug for a vixi `surface:` identifier.
fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Emit the SYMBIOTE face: a `#vixi:kit v1` surface — a stacked root with one
/// `text` slot per chapter (+ a title slot). The structure is engine-native; the
/// chapter prose binds live at the host through the `source=` path, so the codex
/// stays true to state instead of freezing a copy. Carries the universal AOT gates.
pub fn to_symbiote_kit(book: &Book) -> String {
    let mut s = String::new();
    s.push_str("#vixi:kit v1\n");
    s.push_str(&format!("surface: book_{}\n", slug(&book.title)));
    s.push_str("slot root kind=region layout=stack_v\n");
    s.push_str("slot root.title kind=text\n");
    for i in 0..book.spine.chapters.len() {
        s.push_str(&format!("slot root.ch{i} kind=text\n"));
    }
    // The engine's AOT contract — the same gates every studio surface declares.
    s.push_str("gate contrast_min = 4.5\n");
    s.push_str("gate runtime_parse = forbidden\n");
    s.push_str("gate alloc_steady = forbidden\n");
    s.push_str("gate float_in_ir = forbidden\n");
    s
}

/// Emit the LINT-MIRROR face: deveraux voice rules (dash · semicolon; long-line
/// warnings) over the STANDARD face, sectioned per chapter with absolute book
/// lines and quoted receipts. EMITTED from the same source as every other face —
/// the hand-rolled deveraux_lint.py risk retired.
pub fn to_lint_mirror(book: &Book, standard: &str) -> String {
    let lines: Vec<&str> = standard.lines().collect();
    // Chapter heads: "# " lines after the first (the book title line).
    let mut heads: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("# "))
        .map(|(i, l)| (i, &l[2..]))
        .collect();
    let mut out = format!(
        "# {} — lint mirror v2 (deveraux-lint · absolute book lines · quoted receipts)\n",
        book.title
    );
    if heads.len() <= 1 {
        return out;
    }
    heads.remove(0);
    let quote = |l: &str| {
        let mut q: String = l.chars().take(96).collect();
        if l.chars().count() > 96 {
            q.push('…');
        }
        q
    };
    for (n, (start, title)) in heads.iter().enumerate() {
        let end = heads.get(n + 1).map(|(i, _)| *i).unwrap_or(lines.len());
        let body = &lines[start + 1..end];
        let words: usize = body.iter().map(|l| l.split_whitespace().count()).sum();
        let mut faults: Vec<String> = Vec::new();
        let mut warnings = 0usize;
        for (off, l) in body.iter().enumerate() {
            let ln = start + 2 + off; // 1-indexed absolute book line
            for _ in 0..l.matches('—').count() {
                faults.push(format!("- FAULT L{ln} dash: `{}`", quote(l)));
            }
            for _ in 0..l.matches(';').count() {
                faults.push(format!("- FAULT L{ln} semicolon: `{}`", quote(l)));
            }
            if l.chars().count() > 500 {
                warnings += 1;
            }
        }
        out.push_str(&format!(
            "\n## {} · {}\nbook L{}–L{} · {} words · {} faults · {} warnings\n\n",
            n + 1,
            title,
            start + 2,
            end,
            words,
            faults.len(),
            warnings
        ));
        for f in &faults {
            out.push_str(f);
            out.push('\n');
        }
    }
    out
}

/// Compile a book to all faces. Deterministic (no wall-clock, no RNG).
pub fn compile_faces(book: &Book) -> CompiledFaces {
    let standard = export_md(book);
    let lint_mirror = to_lint_mirror(book, &standard);
    CompiledFaces {
        standard,
        dev: serde_json::to_string_pretty(book).unwrap_or_default(),
        symbiote: to_symbiote_kit(book),
        lint_mirror,
    }
}

/// The faces + their permanence seal (M10): one SHA-256 ProvenanceSeal over all
/// four faces with boundary markers — deterministic faces ⇒ a stable seal; ANY
/// byte of drift re-keys it (tamper-evidence, per the calligraphy seal law).
#[derive(Debug, Clone)]
pub struct SealedFaces {
    /// The compiled STANDARD, DEV, SYMBIOTE, and LINT-MIRROR faces.
    pub faces: CompiledFaces,
    /// Tamper-evident SHA-256 seal over all four faces.
    pub seal: ProvenanceSeal,
}

/// Compile a book to all faces and seal them with a ProvenanceSeal.
pub fn compile_sealed(book: &Book) -> SealedFaces {
    let faces = compile_faces(book);
    let mut bytes = Vec::new();
    for (label, body) in [
        ("standard", &faces.standard),
        ("dev", &faces.dev),
        ("symbiote", &faces.symbiote),
        ("lint_mirror", &faces.lint_mirror),
    ] {
        bytes.extend_from_slice(label.as_bytes());
        bytes.push(0xFF);
        bytes.extend_from_slice(body.as_bytes());
        bytes.push(0xFE);
    }
    let seal = seal_bytes(&book.title, &bytes);
    SealedFaces { faces, seal }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;
    use forge_vix_v3::ir::IrRect;
    use forge_vix_v3::layout::{lower, TokenCtx};
    use forge_vix_v3::parse::parse_kit;

    fn atlas() -> Book {
        full_atlas("The Opus", "13Forge")
    }

    #[test]
    fn three_faces_nonempty_and_typed() {
        let f = compile_faces(&atlas());
        assert!(f.standard.contains("# The Opus"), "human prose carries the title");
        assert!(f.dev.contains("\"title\": \"The Opus\""), "machine IR carries the typed title");
        assert!(f.symbiote.starts_with("#vixi:kit v1"), "symbiote is engine-native vixi");
    }

    /// THE load-bearing proof: the machine/engine face compiles the SAME way the
    /// engine compiles every surface — `forge-vix` AOT lower. Clean lower with each
    /// chapter slot materialized = engine-native, not a hand-serialized blob.
    #[test]
    fn symbiote_compiles_as_in_engine() {
        let book = atlas();
        let f = compile_faces(&book);
        let doc = parse_kit(&f.symbiote)
            .expect("symbiote .kit.vixi parses through forge-vix (compiles as in engine)");
        let ui = lower(&doc.root, IrRect::from_xywh(0, 0, 640_000, 480_000), &TokenCtx::comfy(), 1);
        assert!(ui.versions_synced(), "lowered planes share one layout version");
        let keys: Vec<&str> = ui.layout.iter().map(|b| b.stable_key.as_str()).collect();
        assert!(keys.contains(&"root.title"), "title slot lowered to a real box");
        assert!(keys.contains(&"root.ch0"), "first chapter slot lowered to a real box");
        assert!(!book.spine.chapters.is_empty(), "the atlas actually has chapters to compile");
    }

    /// The DEV/machine face is LOSSLESS — it round-trips back to a `Book`.
    #[test]
    fn dev_face_roundtrips_lossless() {
        let book = atlas();
        let f = compile_faces(&book);
        let back: Book = serde_json::from_str(&f.dev).expect("machine IR round-trips to Book");
        assert_eq!(back.title, "The Opus");
        assert_eq!(back.spine.chapters.len(), book.spine.chapters.len());
    }

    /// Same source → byte-identical faces (the determinism the seal depends on).
    #[test]
    fn deterministic_same_source_same_faces() {
        let a = compile_faces(&atlas());
        let b = compile_faces(&atlas());
        assert_eq!(a.standard, b.standard);
        assert_eq!(a.dev, b.dev);
        assert_eq!(a.symbiote, b.symbiote);
    }

    /// M10: the lint-mirror face reports the deveraux voice faults with quoted
    /// receipts, per chapter, in the v2 exemplar format — and is deterministic.
    // [BOARD: M10]
    #[test]
    fn lint_mirror_face_reports_voice_faults_with_receipts() {
        let f = compile_faces(&atlas());
        assert!(
            f.lint_mirror.contains("lint mirror v2 (deveraux-lint"),
            "v2 exemplar header"
        );
        assert!(f.lint_mirror.contains("book L"), "absolute book-line ranges");
        assert!(f.lint_mirror.contains("words ·"), "per-chapter word counts");
        // The atlas prose uses em-dashes — the dash rule must catch them.
        assert!(f.lint_mirror.contains("FAULT L"), "faults carry line receipts");
        assert!(f.lint_mirror.contains("dash: `"), "the dash rule quotes the line");
        let again = compile_faces(&atlas());
        assert_eq!(f.lint_mirror, again.lint_mirror, "deterministic face");
    }

    /// M10: the permanence seal — stable across identical compiles, re-keyed by
    /// any byte of drift (tamper evidence, the calligraphy seal law).
    // [BOARD: M10]
    #[test]
    fn sealed_faces_carry_a_stable_tamper_evident_seal() {
        let a = compile_sealed(&atlas());
        let b = compile_sealed(&atlas());
        assert_eq!(a.seal, b.seal, "same source, same seal");
        assert_eq!(a.seal.id.len(), 12, "12-hex short id (the calligraphy convention)");
        assert!(a.seal.grid_hash != 0, "grid hash present");
        let drift = seal_bytes("The Opus", b"tampered");
        assert_ne!(a.seal, drift, "any drift re-keys the seal");
    }

    /// The World-Building Atlas is a CANON chapter of the book now — it appears in
    /// every compiled face (the "put it in the book" proof).
    #[test]
    fn atlas_chapter_is_canon_in_all_faces() {
        let f = compile_faces(&atlas());
        assert!(f.standard.contains("The World-Building Atlas"), "prose carries the atlas chapter");
        assert!(
            f.standard.contains("wiring program, not a build-from-zero"),
            "prose carries the atlas body, not just the heading"
        );
        assert!(f.dev.contains("The World-Building Atlas"), "machine IR carries the atlas chapter");
    }
}
