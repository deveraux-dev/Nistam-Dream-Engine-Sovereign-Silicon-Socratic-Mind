//! Context sipping — the weld lane's prompt builder (WAVE-WELD W4).
//!
//! The seam-sip law: never dump a raw neighborhood into a prompt window. The
//! v2 receipt that motivates the cap: `outland/src/lib.rs`, 803 lines read
//! whole to check a 20-line fn (`forge-daemon/src/gate.rs:90`). The M2
//! whole-file BRIEF embeds entire crates and blew three reply windows in one
//! day (grind-log receipts, 2026-08-09); this module builds the OTHER kind of
//! prompt: the failing gate's error lines, the sliced neighborhoods those
//! errors name, and nothing else.
//!
//! The cap is a REFUSAL, not a style note: a prompt over
//! `foreman.sip_cap_bytes` is an error BEFORE any INFER is sent — the same
//! fail-before-spend shape as the weld applier's exactly-once anchor rule.

use std::path::Path;

/// A `file:line` coordinate pulled out of gate output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSite {
    /// Path exactly as the gate printed it (workspace-relative).
    pub path: String,
    /// 1-indexed line the gate pointed at.
    pub line: usize,
}

/// Pull every `--> path:line:col` coordinate out of cargo output, in order,
/// deduplicated. Cargo's arrow form is the anchor; anything else in the
/// output is prose and is not trusted to name a file.
pub fn error_sites(gate_output: &str) -> Vec<ErrorSite> {
    let mut out: Vec<ErrorSite> = Vec::new();
    for line in gate_output.lines() {
        let Some(rest) = line.trim_start().strip_prefix("--> ") else { continue };
        // rest = path:line:col — split from the right so Windows drive colons survive.
        let mut parts = rest.rsplitn(3, ':');
        let _col = parts.next();
        let Some(ln) = parts.next().and_then(|l| l.parse::<usize>().ok()) else { continue };
        let Some(path) = parts.next() else { continue };
        let site = ErrorSite { path: path.trim().to_string(), line: ln };
        if !out.contains(&site) {
            out.push(site);
        }
    }
    out
}

/// One sliced neighborhood: `context_lines` above and below the error line,
/// with 1-indexed line numbers, exactly the citation form the tree reads.
fn slice(root: &Path, site: &ErrorSite, context_lines: usize) -> Result<String, String> {
    let abs = root.join(&site.path);
    let body = std::fs::read_to_string(&abs)
        .map_err(|e| format!("sip: cannot read {}: {e}", abs.display()))?;
    let lines: Vec<&str> = body.lines().collect();
    let lo = site.line.saturating_sub(context_lines + 1); // 0-indexed inclusive
    let hi = (site.line + context_lines).min(lines.len()); // 0-indexed exclusive
    let mut out = String::new();
    out.push_str(&format!("// {}:{} ±{}\n", site.path, site.line, context_lines));
    for (i, l) in lines[lo..hi].iter().enumerate() {
        out.push_str(&format!("{:>5} | {l}\n", lo + i + 1));
    }
    Ok(out)
}

/// The error tail worth carrying: each `error` diagnostic BLOCK, verbatim —
/// the header, the arrows, the annotated snippet, and cargo's `help:`
/// suggestion lines. Live-fire 2026-08-09, twice: filtering to header+arrow
/// lines dropped the literal fix (rustc prints the corrected line as an
/// indented suggestion snippet) and the model guessed a recursion instead.
/// The compiler's diagnostic IS the salient slice; warnings and the summary
/// stay out.
fn error_lines(gate_output: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in gate_output.lines() {
        let t = line.trim_start();
        if t.starts_with("error") {
            in_block = true;
        } else if in_block {
            // A block continues while lines are indented, snippet-numbered, or
            // help/note; anything else (warning:, Compiling, blank) ends it.
            let first = line.chars().next();
            let continues = matches!(first, Some(c) if c.is_whitespace() || c.is_ascii_digit())
                || t.starts_with("help:")
                || t.starts_with("note:");
            if !continues {
                in_block = false;
            }
        }
        if in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Build the sipped weld prompt: instruction + error lines + sliced
/// neighborhoods, refused loudly if it exceeds `cap_bytes`.
///
/// The instruction states the exact reply grammar because the sidecar's PDA
/// enforces it anyway — a prompt that asks for what the clamp permits
/// converges in fewer masked candidates.
pub fn build_weld_prompt(
    root: &Path,
    gate_output: &str,
    cap_bytes: usize,
    context_lines: usize,
) -> Result<String, String> {
    let sites = error_sites(gate_output);
    if sites.is_empty() {
        return Err("sip: gate output names no `--> file:line` site — nothing to slice, \
                    the weld lane needs a compiler-shaped red"
            .into());
    }

    let file = sites[0].path.replace('\\', "/");
    let mut prompt = format!(
        "Fix the Rust compile error below with ONE minimal edit to the file `{file}`. \
         Reply with EXACTLY one weld, no prose, no markdown, this shape:\n\
         Weld(lane:\"repair\",files:[F(p:\"{file}\",edits:[E(anchor:\"<the broken text, copied verbatim>\",\
         op:\"replace\",payload:\"<the corrected text>\")])],gate:\"\",receipt:\"\")\n\
         The anchor is the text that is WRONG (it must occur exactly once in the file); the payload \
         is what it must BECOME. A weld whose payload equals its anchor is INVALID. Use forward \
         slashes in paths. Escape newlines as \\n, quotes as \\\".\n\
         Example: for `cannot find function \\`foobr\\`` where `foobar` exists, the edit is \
         E(anchor:\"foobr(\",op:\"replace\",payload:\"foobar(\").\n\nERRORS:\n",
    );
    prompt.push_str(&error_lines(gate_output));
    prompt.push_str("\nSLICES:\n");
    for site in &sites {
        prompt.push_str(&slice(root, site, context_lines)?);
    }

    if prompt.len() > cap_bytes {
        return Err(format!(
            "sip: weld prompt is {} bytes, cap is {cap_bytes} (foreman.sip_cap_bytes) — \
             REFUSED before spend; {} site(s) sliced, shrink the red or raise the cap aloud",
            prompt.len(),
            sites.len()
        ));
    }
    Ok(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO_RED: &str = "\
error[E0425]: cannot find value `speling` in this scope\n\
  --> src/lib.rs:4:13\n\
   |\n\
4  |     let x = speling;\n\
   |             ^^^^^^^ not found in this scope\n\
warning: unused variable `y`\n\
  --> src/lib.rs:9:9\n\
error: aborting due to 1 previous error\n";

    #[test]
    fn error_sites_come_from_arrows_in_order_deduped() {
        let sites = error_sites(CARGO_RED);
        assert_eq!(
            sites,
            vec![
                ErrorSite { path: "src/lib.rs".into(), line: 4 },
                ErrorSite { path: "src/lib.rs".into(), line: 9 },
            ]
        );
    }

    #[test]
    fn windows_drive_colons_survive_the_split() {
        let sites = error_sites(" --> F:\\v3\\src\\lib.rs:12:5\n");
        assert_eq!(sites, vec![ErrorSite { path: "F:\\v3\\src\\lib.rs".into(), line: 12 }]);
    }

    #[test]
    fn a_prompt_over_the_cap_is_refused_before_spend() {
        let dir = std::env::temp_dir().join(format!("sip-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("lib.rs"), "fn a() {}\n".repeat(50)).unwrap();
        let red = "error[E0308]: types\n --> src/lib.rs:3:1\n";
        let e = build_weld_prompt(&dir, red, 64, 2).unwrap_err();
        assert!(e.contains("REFUSED before spend"), "the cap is a refusal: {e}");
        // The same prompt under a sane cap builds.
        assert!(build_weld_prompt(&dir, red, 4096, 2).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_red_with_no_sites_is_refused_not_padded() {
        let dir = std::env::temp_dir();
        let e = build_weld_prompt(&dir, "everything is on fire", 4096, 2).unwrap_err();
        assert!(e.contains("names no"), "{e}");
    }

    #[test]
    fn slices_carry_numbered_lines_around_the_site() {
        let dir = std::env::temp_dir().join(format!("sip-slice-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let body: String = (1..=20).map(|i| format!("line{i}\n")).collect();
        std::fs::write(dir.join("src").join("lib.rs"), body).unwrap();
        let red = "error: x\n --> src/lib.rs:10:1\n";
        let p = build_weld_prompt(&dir, red, 4096, 2).unwrap();
        assert!(p.contains("    8 | line8"));
        assert!(p.contains("   12 | line12"));
        assert!(!p.contains("line7\n"), "outside the window stays home");
        assert!(!p.contains("line13"), "outside the window stays home");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
