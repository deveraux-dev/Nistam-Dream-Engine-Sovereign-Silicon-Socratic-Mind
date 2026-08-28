//! Landing a sidecar draft: reply → staged crate → (on green) promotion.
//!
//! A draft is Speculative until the gate speaks (MIGRATION §LANE DELEGATION),
//! so it never lands inside the main workspace first. It lands in a staging
//! workspace of its own under `target/foreman/stage/<name>/`, is gated there,
//! and only a green draft is promoted into `crates/<name>/` and registered as
//! a workspace member. A red draft therefore cannot turn the tree red.
//!
//! The reply format contract (stated in the brief the foreman sends): each
//! file as a `// FILE: src/....rs` marker line followed by a fenced code
//! block. Only `src/**.rs` paths are accepted — the foreman scaffolds the
//! manifest itself; the sidecar writes no Cargo.toml, no build.rs, and
//! nothing outside `src/`.

use std::path::{Path, PathBuf};

/// One extracted draft file: crate-relative path plus contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftFile {
    /// Crate-relative path, always under `src/`, always `.rs`.
    pub rel_path: String,
    /// The file body as the sidecar drafted it.
    pub body: String,
}

/// Extract `// FILE:` + fenced-block pairs from a sidecar reply.
///
/// Refusals, all loud: a path outside `src/`, a non-`.rs` path, a `..`
/// segment, an absolute path, a marker with no block, or a reply with no
/// files at all. The sidecar has no fs surface; this parser is where its
/// output meets the filesystem, so it is the customs desk.
///
/// Two hardenings past the strict contract, both from live fire 2026-08-10
/// (forge-intent-v3 grind log). The model drops the fences in two ways: it
/// wraps the ENTIRE reply in one outer non-rust fence and treats the inside
/// as a single code block (attempt 3, first firing), or it echoes brief
/// prose and then emits marker + bare code with no fence at all (attempt 1,
/// re-fire). The routes run strictest-first: exact contract, then one outer
/// fence stripped, then bare marker-to-next-marker bodies. The fence was
/// only ever body delimitation — the path law binds on every route, an
/// empty bare body is refused, and the gate stays the verdict on content.
pub fn extract_files(reply: &str) -> Result<Vec<DraftFile>, String> {
    let strict = parse_files(reply, false);
    if strict.is_ok() {
        return strict;
    }
    let reply = split_glued_markers(reply);
    if let Some(inner) = strip_outer_fence(&reply) {
        if let Ok(files) = parse_files(&inner, true) {
            return Ok(files);
        }
    }
    parse_files(&reply, true).or(strict)
}

/// Live-fire 2026-08-10 (re-fire attempts 2–3): the model glues the fence to
/// the marker — `​```// FILE: src/lib.rs` on ONE line — which hides the marker
/// from every route. A fence-opening line that carries a marker becomes the
/// marker line; the backticks were delimiter, never content.
fn split_glued_markers(reply: &str) -> String {
    let lines: Vec<String> = reply
        .lines()
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with("```") {
                if let Some(at) = t.find("// FILE:") {
                    return t[at..].to_string();
                }
            }
            l.to_string()
        })
        .collect();
    lines.join("\n")
}

/// The parse itself. `bare_ok` is the unwrapped-outer-fence mode: a marker
/// followed by unfenced content takes everything up to the next marker (or
/// the end) as its body instead of refusing.
fn parse_files(reply: &str, bare_ok: bool) -> Result<Vec<DraftFile>, String> {
    let mut files = Vec::new();
    let mut lines = reply.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        let Some(path_part) = trimmed.strip_prefix("// FILE:") else { continue };
        let rel = path_part.trim().replace('\\', "/");

        if !rel.starts_with("src/") || !rel.ends_with(".rs") {
            return Err(format!("draft path {rel:?} is outside the src/*.rs contract"));
        }
        if rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
            return Err(format!("draft path {rel:?} carries an empty or `..` segment"));
        }

        // Skip blank lines, then demand an opening fence.
        while matches!(lines.peek(), Some(l) if l.trim().is_empty()) {
            lines.next();
        }
        let fenced = matches!(lines.peek(), Some(l) if l.trim_start().starts_with("```"));
        let mut body = String::new();
        if fenced {
            lines.next(); // consume the opening fence
            let mut closed = false;
            for l in lines.by_ref() {
                if l.trim_start().starts_with("```") {
                    closed = true;
                    break;
                }
                body.push_str(l);
                body.push('\n');
            }
            if !closed {
                return Err(format!("fenced block for {rel:?} never closes"));
            }
        } else if bare_ok {
            // A line that is nothing but a fence is delimiter, never content
            // (a Rust file cannot lex one — attempt 1's gate proved it, and
            // attempts 2–3 proved echoed brief prose rides BEHIND the closer).
            // So in bare mode a pure fence line ENDS the body, mirroring the
            // fenced route; whatever follows before the next marker is
            // inter-file prose and is skipped, exactly as the strict route
            // skips prose between blocks.
            let is_fence = |l: &str| {
                let t = l.trim();
                t.starts_with("```") && t.trim_start_matches('`').chars().all(|c| c.is_alphanumeric())
            };
            let mut kept: Vec<&str> = Vec::new();
            while let Some(l) = lines.peek() {
                if l.trim().starts_with("// FILE:") {
                    break;
                }
                if is_fence(l) {
                    lines.next(); // consume the closer; prose after it is skipped
                    break;
                }
                kept.push(l);
                lines.next();
            }
            while kept.first().is_some_and(|l| l.trim().is_empty()) {
                kept.remove(0);
            }
            while kept.last().is_some_and(|l| l.trim().is_empty()) {
                kept.pop();
            }
            for l in kept {
                body.push_str(l);
                body.push('\n');
            }
            if body.trim().is_empty() {
                return Err(format!("FILE marker {rel:?} has no body at all"));
            }
        } else {
            return Err(format!("FILE marker {rel:?} has no fenced code block"));
        }
        files.push(DraftFile { rel_path: rel, body });
    }

    if files.is_empty() {
        return Err("reply carries no `// FILE:` sections — nothing to land".into());
    }
    Ok(files)
}

/// If the reply's first non-blank line opens a NON-rust fence (```text,
/// ```markdown, bare ```) and its last non-blank line is a bare closing
/// fence, return the content between them. A ```rust opener is never
/// stripped: at the top of a reply it is far more likely a file's own
/// (misplaced) block, and stripping it would eat the last file's closer.
fn strip_outer_fence(reply: &str) -> Option<String> {
    let lines: Vec<&str> = reply.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty())?;
    let last = lines.iter().rposition(|l| !l.trim().is_empty())?;
    if last <= first {
        return None;
    }
    let info = lines[first].trim().strip_prefix("```")?.trim();
    if info.eq_ignore_ascii_case("rust") || lines[last].trim() != "```" {
        return None;
    }
    Some(lines[first + 1..last].join("\n"))
}

/// Dependencies a draft may import — ARCH000-ruled, one entry per ruling.
/// The standing policy is L19 (dep-grab): a candidate that is lightweight
/// (near-zero transitive deps), sovereign (no network/exec/fs surface), and
/// replaces hand-rolled unsafe-adjacent grind with compiler-checked safety
/// is an EASY approve — burning retry compute to force a hand-roll of such a
/// crate is the measured waste this list exists to prevent (2026-08-10:
/// bytemuck, three attempts burned before the "get it" ruling). Each entry
/// is (crate root as imported, manifest line). The scaffolded manifests
/// declare exactly this list and [`foreign_imports`] whitelists exactly this
/// list — one home, so a ruling lands in both places or neither.
pub const DRAFT_DEPS: &[(&str, &str)] = &[
    ("bytemuck", "bytemuck = { version = \"1\", features = [\"derive\"] }"),
];

/// The `[dependencies]` block both manifests share, from [`DRAFT_DEPS`].
fn deps_block() -> String {
    let mut out = String::from("\n[dependencies]\n");
    for (_, line) in DRAFT_DEPS {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Crate roots a draft imports from outside itself and outside the approved
/// set — the deterministic customs lint (2026-08-10): the gate's E0432 names
/// the symptom but not the action, so an illegal import is refused BEFORE any
/// gate run, with a retry message that says what to do instead.
pub fn foreign_imports(files: &[DraftFile]) -> Vec<String> {
    const LOCAL: [&str; 6] = ["core", "alloc", "std", "self", "crate", "super"];
    // The draft's own top-level names are local too: edition-2021 uniform
    // paths make `use cell::X` legal at the crate root when `cell` is a
    // sibling module — measured live 2026-08-10 (haiku's forge-tui draft,
    // first delegate landing): the compiler accepted what this lint refused.
    // A file stem or a `mod` declaration anywhere in the draft is local.
    let mut own: Vec<String> = Vec::new();
    for f in files {
        for seg in f.rel_path.trim_start_matches("src/").trim_end_matches(".rs").split('/') {
            if !seg.is_empty() && seg != "lib" && seg != "mod" {
                own.push(seg.to_string());
            }
        }
        for line in f.body.lines() {
            let t = line.trim();
            let t = t.strip_prefix("pub ").unwrap_or(t);
            if let Some(rest) = t.strip_prefix("mod ") {
                let name: String =
                    rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() {
                    own.push(name);
                }
            }
        }
    }
    let mut found: Vec<String> = Vec::new();
    for f in files {
        for line in f.body.lines() {
            let t = line.trim();
            let t = t.strip_prefix("pub ").unwrap_or(t);
            let rest = match (t.strip_prefix("use "), t.strip_prefix("extern crate ")) {
                (Some(r), _) | (None, Some(r)) => r,
                (None, None) => continue,
            };
            let root: String = rest
                .trim_start_matches("::")
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if root.is_empty()
                || LOCAL.contains(&root.as_str())
                || own.contains(&root)
                || DRAFT_DEPS.iter().any(|(name, _)| *name == root)
                || found.contains(&root)
            {
                continue;
            }
            found.push(root);
        }
    }
    found
}

/// The staging directory for a named draft: its own tiny workspace under
/// `target/` so `cargo` treats it as unrelated to the main tree, and so a
/// `target`-skipping tape sync never records a speculative draft.
pub fn stage_dir(root: &Path, name: &str) -> PathBuf {
    root.join("target").join("foreman").join("stage").join(name)
}

/// Write a draft into its staging workspace: scaffolded manifest (the lints
/// mirror the root workspace so stage-green predicts root-green) plus the
/// drafted `src/` files. Any previous stage for the name is replaced whole —
/// stale files from attempt N-1 must not leak into attempt N's gate run.
pub fn stage(root: &Path, name: &str, files: &[DraftFile]) -> Result<PathBuf, String> {
    let dir = stage_dir(root, name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|e| format!("cannot clear stage {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir.join("src"))
        .map_err(|e| format!("cannot create stage {}: {e}", dir.display()))?;

    std::fs::write(dir.join("Cargo.toml"), stage_manifest(name))
        .map_err(|e| format!("cannot write stage manifest: {e}"))?;
    for f in files {
        let p = dir.join(&f.rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &f.body).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
    }
    Ok(dir)
}

/// The staged manifest. `[workspace]` makes it standalone (cargo stops
/// walking up), and the lint block restates the root workspace's lints so the
/// stage gate measures the same law the promotion gate will.
fn stage_manifest(name: &str) -> String {
    format!(
        "# STAGED DRAFT — speculative until the gate speaks. Foreman-scaffolded.\n\
         [package]\n\
         name = \"{name}\"\n\
         version = \"0.3.0\"\n\
         edition = \"2021\"\n\
         \n\
         [workspace]\n\
         {}\n\
         [lints.rust]\n\
         unsafe_code = \"deny\"\n\
         missing_docs = \"deny\"\n\
         \n\
         [lints.clippy]\n\
         undocumented_unsafe_blocks = \"deny\"\n",
        deps_block()
    )
}

/// The promoted manifest: identical law, but inherited from the workspace the
/// crate now joins instead of restated.
fn promoted_manifest(name: &str) -> String {
    format!(
        "# Foreman-promoted from a gated sidecar draft (MIGRATION §M2).\n\
         [package]\n\
         name = \"{name}\"\n\
         version.workspace = true\n\
         edition.workspace = true\n\
         license.workspace = true\n\
         authors.workspace = true\n\
         {}\n\
         [lints]\n\
         workspace = true\n",
        deps_block()
    )
}

/// Promote a green stage into `crates/<name>/` and register it as a workspace
/// member. Returns the repo-relative paths of every file written, for the
/// stamped commit that follows.
pub fn promote(root: &Path, name: &str, files: &[DraftFile]) -> Result<Vec<String>, String> {
    let crate_dir = root.join("crates").join(name);
    std::fs::create_dir_all(crate_dir.join("src")).map_err(|e| e.to_string())?;

    let mut written = Vec::new();
    let manifest_rel = format!("crates/{name}/Cargo.toml");
    std::fs::write(root.join(&manifest_rel), promoted_manifest(name))
        .map_err(|e| e.to_string())?;
    written.push(manifest_rel);

    for f in files {
        let rel = format!("crates/{name}/{}", f.rel_path);
        let p = root.join(&rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &f.body).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
        written.push(rel);
    }

    register_member(root, name)?;
    written.push("Cargo.toml".to_string());
    Ok(written)
}

/// Undo a promotion whose root gate went red: remove the member line and the
/// promoted `crates/<name>/` directory. Only files the foreman itself wrote in
/// this promotion are touched — the draft still exists in staging and will
/// travel to the brief queue, so nothing is lost, only un-landed.
pub fn demote(root: &Path, name: &str) -> Result<(), String> {
    unregister_member(root, name)?;
    let dir = root.join("crates").join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|e| format!("cannot remove demoted {}: {e}", dir.display()))?;
    }
    Ok(())
}

/// Remove the exact block [`register_member`] inserted. Idempotent; refuses
/// nothing — an absent entry is already the desired state.
fn unregister_member(root: &Path, name: &str) -> Result<(), String> {
    let manifest_path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("cannot read root manifest: {e}"))?;
    let block = format!(
        "\n    # Foreman-promoted from a gated sidecar draft (MIGRATION §M2).\n    \"crates/{name}\",\n",
    );
    if !text.contains(&block) {
        return Ok(());
    }
    std::fs::write(&manifest_path, text.replace(&block, "\n"))
        .map_err(|e| format!("cannot write root manifest: {e}"))
}

/// Add `"crates/<name>"` to the root manifest's `members` list, idempotently.
/// The insertion point is the line before the list's closing bracket, so the
/// hand-written commentary above each existing member is never touched.
pub fn register_member(root: &Path, name: &str) -> Result<(), String> {
    let manifest_path = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("cannot read root manifest: {e}"))?;
    let entry = format!("\"crates/{name}\"");
    if text.contains(&entry) {
        return Ok(());
    }

    let open = text.find("members = [").ok_or("root manifest has no members list")?;
    // The closer is the first LINE after the opener that starts with `]` —
    // never the first `]` byte, which can sit inside a comment (measured live
    // 2026-08-10: "[dependencies] enforces." split at its `]`, breaking the
    // root manifest mid-word).
    let mut at = open + text[open..].find('\n').ok_or("members list never closes")? + 1;
    let insert_at = loop {
        let line_end = text[at..].find('\n').map(|n| at + n).unwrap_or(text.len());
        if text[at..line_end].trim_start().starts_with(']') {
            break at;
        }
        if line_end >= text.len() {
            return Err("members list never closes".into());
        }
        at = line_end + 1;
    };
    let mut out = String::with_capacity(text.len() + 64);
    out.push_str(&text[..insert_at]);
    out.push_str(&format!(
        "    # Foreman-promoted from a gated sidecar draft (MIGRATION §M2).\n    {entry},\n",
    ));
    out.push_str(&text[insert_at..]);
    std::fs::write(&manifest_path, out).map_err(|e| format!("cannot write root manifest: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = "Here is the port.\n\
        // FILE: src/lib.rs\n\
        ```rust\n\
        //! Docs.\n\
        pub fn one() -> u8 { 1 }\n\
        ```\n\
        // FILE: src/extra.rs\n\
        ```rust\n\
        //! More.\n\
        ```\n";

    #[test]
    fn a_reply_yields_its_files_with_fences_stripped() {
        let files = extract_files(REPLY).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].rel_path, "src/lib.rs");
        assert!(files[0].body.contains("pub fn one()"));
        assert!(!files[0].body.contains("```"), "fences are wrapper, not content");
    }

    #[test]
    fn paths_outside_the_src_rs_contract_are_refused() {
        for bad in [
            "// FILE: Cargo.toml\n```\nx\n```\n",
            "// FILE: src/../../evil.rs\n```\nx\n```\n",
            "// FILE: /abs/path.rs\n```\nx\n```\n",
            "// FILE: src/lib.txt\n```\nx\n```\n",
        ] {
            assert!(extract_files(bad).is_err(), "should refuse: {bad:?}");
        }
        assert!(extract_files("no files here at all").is_err());
        assert!(extract_files("// FILE: src/a.rs\n").is_err(), "a marker with no body lands nothing");
    }

    /// The re-fire live shape (forge-intent-v3 attempt 1, 2026-08-10, second
    /// firing): echoed brief prose, then marker + bare code, no fence at all.
    /// The bare route carries it to the gate; the prose before the first
    /// marker never enters a body.
    #[test]
    fn a_bare_unfenced_reply_still_reaches_the_gate() {
        let reply = "Task note: proposed: M2 mechanism proof.\n\
            // FILE: src/lib.rs\n\
            #![no_std]\n\
            pub fn one() -> u8 { 1 }\n";
        let files = extract_files(reply).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].body.contains("#![no_std]"));
        assert!(!files[0].body.contains("Task note"), "preamble prose is not body");
    }

    /// The exact live-fire shape (forge-intent-v3 attempt 3, 2026-08-10): the
    /// whole reply is one outer ```text fence, markers inside it, bodies bare.
    #[test]
    fn an_outer_text_fence_with_bare_bodies_is_unwrapped() {
        let reply = "```text\n\
            // FILE: src/lib.rs\n\
            #![no_std]\n\
            pub fn one() -> u8 { 1 }\n\
            \n\
            // FILE: src/extra.rs\n\
            //! More.\n\
            ```\n";
        let files = extract_files(reply).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].rel_path, "src/lib.rs");
        assert!(files[0].body.contains("#![no_std]"));
        assert!(!files[0].body.contains("```"), "the wrapper never reaches a body");
        assert_eq!(files[1].body, "//! More.\n");
    }

    /// The other wrapper shape: outer non-rust fence, but the inner blocks
    /// are properly fenced — unwrapping must not break the fenced route.
    #[test]
    fn an_outer_fence_around_proper_inner_fences_still_parses() {
        let reply = "```markdown\n\
            // FILE: src/lib.rs\n\
            ```rust\n\
            pub fn one() -> u8 { 1 }\n\
            ```\n\
            ```\n";
        let files = extract_files(reply).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].body.contains("pub fn one()"));
        assert!(!files[0].body.contains("```"));
    }

    /// The hardening's own boundaries: an outer ```rust fence is never
    /// stripped as a wrapper — the bare route carries that shape (re-fire
    /// attempt 1's live form), with the edge fence lines dropped as delimiter
    /// residue and mid-body content untouched. The path law binds every route.
    #[test]
    fn the_unwrap_refuses_rust_openers_and_keeps_the_path_law() {
        let rust_outer = "```rust\n// FILE: src/a.rs\npub fn a() {}\n```\n";
        let files = extract_files(rust_outer).unwrap();
        assert!(files[0].body.contains("pub fn a()"));
        assert!(!files[0].body.contains("```"), "edge fences are delimiter, not content");

        let bad_path = "```text\n// FILE: Cargo.toml\n[package]\n```\n";
        assert!(extract_files(bad_path).is_err(), "path law holds on the bare route");
    }

    /// Re-fire attempts 2–3 live shape (2026-08-10): the fence GLUED to the
    /// marker on one line, body bare, closing fence at the end. The marker
    /// must be recovered and the edge fence dropped; a fence line in the
    /// MIDDLE of a bare body stays, and travels loud to the gate.
    #[test]
    fn a_fence_glued_to_the_marker_is_split_and_edges_are_clean() {
        let reply = "```// FILE: src/lib.rs\n#![no_std]\npub fn one() -> u8 { 1 }\n```\n";
        let files = extract_files(reply).unwrap();
        assert_eq!(files[0].rel_path, "src/lib.rs");
        assert!(files[0].body.contains("#![no_std]"));
        assert!(!files[0].body.contains("```"), "edge fence dropped");

        // A fence inside a bare body ENDS it (it is the closer); echoed
        // prose between the closer and the next marker is skipped exactly
        // as the strict route skips prose between blocks — the live shape
        // of re-fire attempts 2–3, where "END OF V2 REFERENCE…" echoes rode
        // behind the closer and reached the compiler as `found OF`.
        let mid = "```// FILE: src/lib.rs\nfn a() {}\n```\nEND OF V2 REFERENCE echo.\n// FILE: src/c.rs\nfn c() {}\n";
        let files = extract_files(mid).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].body, "fn a() {}\n", "the fence closed the body");
        assert!(!files[0].body.contains("OF"), "echoed prose never reaches a body");
        assert_eq!(files[1].body, "fn c() {}\n");
    }

    /// The customs lint: approved and local roots pass, anything else is
    /// named once, and both `use` forms are seen. bytemuck sits in the
    /// approved set (ARCH000 2026-08-10), so it must NOT flag.
    #[test]
    fn the_customs_lint_names_foreign_imports_and_passes_the_approved_set() {
        let draft = |body: &str| {
            vec![DraftFile { rel_path: "src/lib.rs".into(), body: body.into() }]
        };
        assert!(foreign_imports(&draft(
            "use core::mem;\nuse std::fmt;\nuse crate::x;\nuse self::y;\nuse alloc::vec;\n\
             use bytemuck::{Pod, Zeroable};\nmod t { use super::*; }"
        ))
        .is_empty());
        assert_eq!(
            foreign_imports(&draft("use serde::Serialize;\npub use rand::Rng;\nuse serde::de;")),
            vec!["serde".to_string(), "rand".to_string()],
            "each foreign root named exactly once"
        );
        assert_eq!(foreign_imports(&draft("extern crate libc;")), vec!["libc".to_string()]);
        // The draft's own modules are local (edition-2021 uniform paths):
        // both the sibling-file form and the inline `mod` form.
        let multi = vec![
            DraftFile {
                rel_path: "src/lib.rs".into(),
                body: "pub mod cell;\nuse cell::GridCell;\nmod util;\nuse util::clamp;\n".into(),
            },
            DraftFile { rel_path: "src/cell.rs".into(), body: "pub struct GridCell;\n".into() },
        ];
        assert!(foreign_imports(&multi).is_empty(), "own modules are not foreign");
        assert!(
            foreign_imports(&draft("/// use serde in docs is prose\nlet x = 1;")).is_empty(),
            "doc prose is not an import"
        );
    }

    /// The scaffolded manifests carry the approved deps — a ruling that
    /// whitelists an import but never declares it would gate-red on E0432.
    #[test]
    fn both_manifests_declare_the_approved_deps() {
        for manifest in [stage_manifest("forge-x-v3"), promoted_manifest("forge-x-v3")] {
            assert!(manifest.contains("[dependencies]"));
            for (name, line) in DRAFT_DEPS {
                assert!(manifest.contains(line), "{name} must be declared:\n{manifest}");
            }
        }
    }

    #[test]
    fn register_member_is_idempotent_and_preserves_the_manifest() {
        let dir = std::env::temp_dir().join(format!("foreman-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\n    # kept comment\n    \"crates/a\",\n]\n",
        )
        .unwrap();

        register_member(&dir, "forge-new-v3").unwrap();
        register_member(&dir, "forge-new-v3").unwrap();
        let text = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert_eq!(text.matches("crates/forge-new-v3").count(), 1, "registered exactly once");
        assert!(text.contains("# kept comment"), "hand commentary untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The live-fire regression (2026-08-10, first delegate landing): a `]`
    /// inside a members-list COMMENT is prose, not the closer — insertion
    /// must land before the `]` line, and the round trip must restore the
    /// manifest byte-identically.
    #[test]
    fn a_bracket_inside_a_comment_never_splits_the_manifest() {
        let dir = std::env::temp_dir().join(format!("foreman-brk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let original = "[workspace]\nmembers = [\n    # empty [dependencies] enforces.\n    \"crates/a\",\n]\n";
        std::fs::write(dir.join("Cargo.toml"), original).unwrap();

        register_member(&dir, "forge-new-v3").unwrap();
        let text = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(
            text.contains("# empty [dependencies] enforces."),
            "the comment must survive whole:\n{text}"
        );
        assert!(text.contains("\"crates/forge-new-v3\",\n]"), "entry lands before the closer");

        unregister_member(&dir, "forge-new-v3").unwrap();
        let back = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert_eq!(back, original, "demote restores the manifest exactly");
        std::fs::remove_dir_all(&dir).ok();
    }
}
