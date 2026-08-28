//! `foreman hook <event>` — the harness hook lane, in compiled Rust.
//!
//! Port receipt (2026-08-16, Sean: "yes, build it, the faster we can get rid
//! of all ambient ps1 scripts the better"): replaces the eight
//! `.claude/hooks/*.ps1` scripts with one verb reading the harness's hook
//! JSON on stdin. Behavior changes on purpose, not by accident:
//!
//!  - NO auto-revert, ever. The per-edit revert destroyed three crates and
//!    zeroed 64KB of aspire.rs under 3-5 concurrent sessions
//!    (foreman-gate.ps1's own 2026-08-14/15 receipts — path-keyed snapshots
//!    have no session identity and cannot be made safe). Verification moves
//!    to turn granularity: `post-edit` records what was touched into a
//!    per-session ledger, `stop` gates it once per turn and reports RED
//!    honestly instead of mutating files it does not own.
//!  - The snapshot/backup/.gate-skip machinery dies with the revert.
//!  - Everything else ports 1:1: phase-zero arming (L25, `pre-edit`), the
//!    L04/L22b grep reminder with its 15-minute debounce (`pre-grep`), the
//!    PowerShell source-write block (L18, `pre-shell`), and the SessionEnd
//!    measured beat (`session-end`).
//!
//! Stdin field extraction is a bounded string scan, not a JSON tree parse
//! (C03/L19: no serde dep for four flat string fields). Aperture: it takes
//! the FIRST occurrence of a key, which is correct only because the harness
//! serializes `file_path` before `content` inside `tool_input`; if that
//! ordering ever flips, a written file body containing the literal text
//! `"file_path"` could shadow the real key. Stated, not silently assumed.
//!
//! Every event exits Ok on its ordinary path — a hook process that dies loud
//! on malformed stdin would block every tool call in the harness; refusal is
//! expressed as a `{"decision":"block"}` JSON line, never a nonzero exit.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// shell/src files whose edit must be witnessed (visual regression) — the
/// same whitelist foreman-witness.ps1 carried: the files a witness
/// scenario's pixels actually depend on, deliberately narrow.
const RENDER_INPUT_FILES: &[&str] = &[
    "gpu.rs",
    "compose.rs",
    "organs.rs",
    "main.rs",
    "hud.rs",
    "pen_input.rs",
    "pen_canvas.rs",
    "camera_lens.rs",
    "canvas_layer.rs",
    "effects.rs",
    "keyboard_hook.rs",
    "input_route.rs",
    "sand_gpu.rs",
];

/// PowerShell verbs/sinks that write bytes — a command holding one of these
/// AND a gated source path walks around the Edit/Write tool layer (L18).
const SHELL_WRITE_TOKENS: &[&str] = &[
    "Set-Content",
    "Add-Content",
    "Out-File",
    "Tee-Object",
    "[System.IO.File]::",
    "[IO.File]::",
    "Copy-Item",
    "Move-Item",
    "Rename-Item",
];

/// PowerShell verbs that DELETE a file or directory outright. Checked against
/// a gated path with no `.rs` requirement (directory deletion has no
/// extension) — this is what a whole-file/whole-module removal actually looks
/// like on the wire, distinct from the write-tokens above. Receipt: 2026-08-20,
/// a general-purpose wave welder deleted `realtime.rs`/`alloc_tracer.rs`/
/// `broadcast.rs` and all 7 files of `src/fauna/` this way — a destructive,
/// hard-to-reverse diff that should never bypass the Edit/Write tool layer.
const SHELL_DELETE_TOKENS: &[&str] = &[
    "Remove-Item",
    "ri -Recurse",
    "rd /s",
    "rmdir /s",
];

/// Dispatch `foreman hook <event>`: read the harness hook JSON from stdin
/// and run the named event's gate. Ordinary paths always exit 0 — refusal is
/// a `{"decision":"block"}` line, never a nonzero exit (see module docs).
pub fn verb(root: &Path, args: &[String]) -> Result<(), String> {
    let event = args
        .iter()
        .skip_while(|a| a.as_str() != "hook")
        .nth(1)
        .map(String::as_str)
        .ok_or("hook: usage: foreman hook <pre-edit|post-edit|pre-grep|pre-shell|stop|session-end> --root <workspace>")?;

    let mut stdin_text = String::new();
    // A hook with no stdin (manual invocation) still runs; empty is fine.
    let _ = std::io::stdin().read_to_string(&mut stdin_text);

    let out = match event {
        "pre-edit" => pre_edit(root, &stdin_text),
        "post-edit" => post_edit(root, &stdin_text),
        "pre-grep" => pre_grep(root, &stdin_text),
        "pre-shell" => pre_shell(&stdin_text),
        "stop" => stop(root, &stdin_text),
        "session-end" => session_end(root),
        other => return Err(format!("hook: unknown event {other:?}")),
    }?;
    if let Some(line) = out {
        println!("{line}");
    }
    Ok(())
}

// ── stdin field extraction ──────────────────────────────────────────────────

/// First string value for `key` anywhere in `json`, unescaped. Bounded scan,
/// no tree parse — see module aperture note on first-occurrence semantics.
pub fn json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let at = json.find(&needle)?;
    let rest = &json[at + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;

    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'u' => {
                    // Consume exactly 4 hex digits; decode BMP scalars,
                    // substitute '?' for anything unpaired/invalid.
                    let hex: String = chars.by_ref().take(4).collect();
                    let decoded = u32::from_str_radix(&hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .unwrap_or('?');
                    out.push(decoded);
                }
                esc => out.push(esc), // covers \" \\ \/ and passthrough
            },
            _ => out.push(c),
        }
    }
    None // unterminated string — treat as absent, never panic
}

/// Escape a string for embedding in a JSON output line.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn block_line(reason: &str) -> String {
    format!("{{\"decision\":\"block\",\"reason\":\"{}\"}}", json_escape(reason))
}

fn block_with_sys_line(reason: &str, sys: &str) -> String {
    format!(
        "{{\"decision\":\"block\",\"reason\":\"{}\",\"systemMessage\":\"{}\"}}",
        json_escape(reason),
        json_escape(sys)
    )
}

fn sys_line(msg: &str) -> String {
    format!("{{\"systemMessage\":\"{}\"}}", json_escape(msg))
}

fn sys_and_context_line(sys: &str, ctx: &str) -> String {
    format!(
        "{{\"systemMessage\":\"{}\",\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\"additionalContext\":\"{}\"}}}}",
        json_escape(sys),
        json_escape(ctx)
    )
}

// ── pre-edit: L25 phase-zero gate (port of phase0-gate.ps1) ─────────────────

/// Where the loop-arm flag lives — one home, cited by both the exemption
/// check and the armed-metadata read below.
fn armed_flag_path(root: &Path) -> PathBuf {
    root.join(".claude").join("hooks").join(".loop-active")
}

/// >60-min arm age -> the loud disarm notice; fresh arm -> None.
fn stale_notice(age_secs: u64) -> Option<String> {
    if age_secs <= 60 * 60 {
        return None;
    }
    Some(sys_line(&format!(
        "[foreman hook pre-edit] .loop-active is STALE ({}min > 60min) — L25 gate DISARMED for this edit. \
         Re-arm .claude/hooks/.loop-active + .phase0/current.json, or delete the flag.",
        age_secs / 60
    )))
}

/// Rewrite the arm flag byte-for-byte to bump its mtime (sliding TTL:
/// an actively-gated session never crosses the stale threshold).
fn refresh_arm(root: &Path) {
    let p = armed_flag_path(root);
    if let Ok(body) = std::fs::read(&p) {
        let _ = std::fs::write(&p, body);
    }
}

/// L25 phase-zero gate. `Ok(Some(json))` carries the hook-protocol decision
/// line to emit; `Ok(None)` means the edit proceeds with nothing to say.
pub fn pre_edit(root: &Path, stdin_text: &str) -> Result<Option<String>, String> {
    let phase0_dir = root.join(".claude").join("hooks").join(".phase0");
    // The gate's own control files are exempt — otherwise arming it (writing
    // .loop-active) can brick the one write (current.json) required to
    // satisfy it, a self-referential deadlock no amount of retrying escapes.
    let file_path = json_str(stdin_text, "file_path").or_else(|| json_str(stdin_text, "filePath"));
    if let Some(fp) = &file_path {
        let target = PathBuf::from(fp);
        if target == phase0_dir.join("current.json") || target == phase0_dir.join("verified.hash") || target == armed_flag_path(root) {
            return Ok(None);
        }
    }

    let Ok(meta) = std::fs::metadata(&armed_flag_path(root)) else {
        return Ok(None); // not a loop iteration — gate does not apply
    };
    // Stale arm (crashed/abandoned loop, >60 min) must not gate forever —
    // it disarms for this edit, loudly, never silent.
    if let Ok(modified) = meta.modified() {
        if let Ok(age) = modified.elapsed() {
            if let Some(notice) = stale_notice(age.as_secs()) {
                return Ok(Some(notice));
            }
        }
    }

    let current = phase0_dir.join("current.json");
    let Ok(body) = std::fs::read_to_string(&current) else {
        return Ok(Some(block_line(
            "[foreman hook pre-edit] L25 phase-zero: loop is ARMED but no .claude\\hooks\\.phase0\\current.json exists. \
             Before editing, write it with {\"outcome\": \"...\", \"proof_command\": \"...\", \"fail_state\": \"...\"} — \
             the single user-observable outcome this iteration produces, the exact command that proves it, \
             and that command's CURRENT failing state.",
        )));
    };

    let outcome = json_str(&body, "outcome").unwrap_or_default();
    let proof_cmd = json_str(&body, "proof_command").unwrap_or_default();
    let fail_state = json_str(&body, "fail_state").unwrap_or_default();
    if outcome.trim().is_empty() || proof_cmd.trim().is_empty() || fail_state.trim().is_empty() {
        return Ok(Some(block_line(
            "[foreman hook pre-edit] current.json is missing outcome/proof_command/fail_state — \
             all three are required, none may be blank.",
        )));
    }

    // A verified statement re-runs only when its content changes — many edits
    // under ONE phase0 statement must not re-pay a slow proof_command each.
    let mut hasher = DefaultHasher::new();
    hasher.write(body.as_bytes());
    let hash = format!("{:016x}", hasher.finish());
    let verified = phase0_dir.join("verified.hash");
    if std::fs::read_to_string(&verified).map(|h| h.trim() == hash).unwrap_or(false) {
        refresh_arm(root);
        return Ok(None);
    }

    // THE mechanical check: run the claimed proof command for real.
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &proof_cmd])
        .current_dir(root)
        .output();
    match status {
        Ok(out) if out.status.success() => Ok(Some(block_line(&format!(
            "[foreman hook pre-edit] L25 phase-zero: proof_command '{proof_cmd}' EXITS 0 (passes) right now — \
             it does not match the claimed fail_state ('{fail_state}'). Either the outcome is already achieved \
             (nothing to edit for) or the wrong command was named. Fix current.json or pick a real target."
        )))),
        Ok(_) => {
            let _ = std::fs::write(&verified, &hash);
            refresh_arm(root);
            Ok(None)
        }
        Err(e) => {
            // Cannot spawn the prover: say so, do not silently wave the edit through as verified.
            Ok(Some(sys_line(&format!(
                "[foreman hook pre-edit] could not run proof_command ({e}) — edit proceeds UNGATED, phase0 unverified."
            ))))
        }
    }
}

// ── post-edit: record touched work into the per-session turn ledger ─────────

/// Nearest enclosing Cargo.toml package name, walking up from `file` but
/// never above `root`. Folder name != package name (shell/ -> studio-shell).
fn crate_name_above(root: &Path, file: &Path) -> Option<String> {
    let mut dir = file.parent()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if let Ok(body) = std::fs::read_to_string(&manifest) {
            if let Some(name) = parse_package_name(&body) {
                return Some(name);
            }
        }
        if !dir.starts_with(root) {
            return None;
        }
        dir = dir.parent()?;
    }
}

/// `name = "x"` from Cargo.toml text — exact string matching, no regex (L04).
pub fn parse_package_name(toml: &str) -> Option<String> {
    for line in toml.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("name") else { continue };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else { continue };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('"') else { continue };
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn ledger_path(root: &Path, stdin_text: &str) -> PathBuf {
    let session = json_str(stdin_text, "session_id").unwrap_or_else(|| "default".into());
    // Durable-ish process state belongs under .forge, never _scratch (T1).
    root.join(".forge").join("hook-state").join(format!("{session}.touched"))
}

/// Is `file` a witness-scoped render/input file under shell/src?
pub fn is_witness_scoped(root: &Path, file: &Path) -> bool {
    if !file.starts_with(root.join("shell").join("src")) {
        return false;
    }
    match file.file_name().and_then(|n| n.to_str()) {
        Some(name) => RENDER_INPUT_FILES.contains(&name),
        None => false,
    }
}

// ── post_edit: L22b/T1 receipt gate + L05 one-home gate ─────────────────────

/// Files where an absence claim owes a `RECEIPT(...)`/`[ASSUMED]`/`[INFERRED]`
/// marker — narrow and named, not "every .rs file" (avoids false positives on
/// legitimate code/test text that happens to contain "does not exist").
const CLAIM_BEARING_FILES: &[&str] = &["crates/forge-book-v3/src/seams.rs", "crates/forge-core-v3/src/aspire.rs"];

fn is_claim_bearing(root: &Path, file: &Path) -> bool {
    let in_grind_log = file.starts_with(root.join(".forge").join("grind-log"))
        && file.extension().and_then(|e| e.to_str()) == Some("md");
    in_grind_log || CLAIM_BEARING_FILES.iter().any(|rel| file == root.join(rel))
}

const ABSENCE_PHRASES: &[&str] = &["ABSENT", "does not exist", "not found", "never existed"];

/// L22b/T1: an absence claim just written to a claim-bearing file needs a
/// receipt marker nearby. `content` is the write's own new text (`content`
/// for Write, `new_string` for Edit) — only what's newly landing is checked,
/// not the file's pre-existing, possibly-already-receipted body.
fn receipt_gate(root: &Path, file: &Path, content: &str) -> Option<String> {
    if content.is_empty() || !is_claim_bearing(root, file) {
        return None;
    }
    let has_absence_claim = ABSENCE_PHRASES.iter().any(|p| content.contains(p));
    if has_absence_claim && !crate::receipt::has_receipt_or_tag(content) {
        return Some(format!(
            "[foreman hook post-edit] L22b/T1: {} was just written with an absence claim \
             (ABSENT / \"does not exist\" / \"not found\") but no RECEIPT(...) row and no \
             [ASSUMED]/[INFERRED] tag anywhere in the new text. Add one — \
             RECEIPT(claim:\"...\",verdict:ABSENT,roots:[...],anchor:\"file:line\") — naming which \
             roots were actually checked.",
            file.display()
        ));
    }
    None
}

fn read_ident(s: &str) -> &str {
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    &s[..end]
}

/// Conservative `#[cfg(test)]` detector via brace-depth tracking from the
/// nearest preceding attribute to `pos`. Approximate — doesn't account for
/// braces inside string/comment literals — but the failure mode is a missed
/// detection (skips a real check), never a wrong block.
fn is_inside_cfg_test(content: &str, pos: usize) -> bool {
    let bytes = content.as_bytes();
    let marker = b"#[cfg(test)]";
    let mut i = 0;
    let mut test_depth: i32 = -1;
    let mut depth: i32 = 0;
    while i < pos && i < bytes.len() {
        // Byte-slice comparison, never a `&str` re-slice: `i` walks every raw
        // byte (including mid-multibyte-char offsets when content has any
        // non-ASCII, e.g. this very file's own em-dashes) and `content[i..]`
        // panics on a non-char-boundary index — `bytes[i..]` never does.
        if test_depth == -1 && bytes[i..].starts_with(marker) {
            test_depth = depth;
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if test_depth != -1 && depth <= test_depth {
                    test_depth = -1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    test_depth != -1
}

/// Every `pub struct <Name>` / `pub enum <Name>` newly declared in `content`,
/// outside `#[cfg(test)]`.
fn new_type_decls(content: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for marker in ["pub struct ", "pub enum "] {
        let mut start = 0;
        while let Some(rel) = content[start..].find(marker) {
            let marker_pos = start + rel;
            let ident_start = marker_pos + marker.len();
            if !is_inside_cfg_test(content, marker_pos) {
                let ident = read_ident(&content[ident_start..]);
                if !ident.is_empty() {
                    names.push(ident);
                }
            }
            start = ident_start;
        }
    }
    names
}

/// Does `name` already exist as a `pub struct`/`pub enum` in a file other
/// than `editing_file`, anywhere under `crates/`? Bounded walk via
/// `forge_index_v3::walker::walk_bounded_skipping` (T1 unbound_io: never a
/// raw recursive walk) — returns the colliding file's path if found.
fn find_existing_decl(root: &Path, editing_file: &Path, name: &str) -> Option<String> {
    let crates_dir = root.join("crates");
    let report = forge_index_v3::walker::walk_bounded_skipping(&crates_dir, 200_000, 10, &[]);
    for entry in &report.entries {
        if entry.is_dir || entry.path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if entry.path == editing_file {
            continue; // the file being edited right now, not a collision with itself
        }
        let Ok(text) = std::fs::read_to_string(&entry.path) else { continue };
        for marker in ["pub struct ", "pub enum "] {
            let mut start = 0;
            while let Some(rel) = text[start..].find(marker) {
                let ident_start = start + rel + marker.len();
                let found = read_ident(&text[ident_start..]);
                if found == name && !is_inside_cfg_test(&text, start + rel) {
                    return Some(entry.path.display().to_string());
                }
                start = ident_start;
            }
        }
    }
    None
}

/// L05 one-home: does `content` introduce a `pub struct`/`pub enum` name that
/// already lives, under the same name, in a different file under `crates/`?
fn one_home_gate(root: &Path, file: &Path, content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    for name in new_type_decls(content) {
        if let Some(hit) = find_existing_decl(root, file, name) {
            return Some(format!(
                "[foreman hook post-edit] L05 one-home: '{name}' was just declared in {} but already \
                 exists as a pub struct/enum in {hit} — a second live home for the same name is a \
                 defect (CLAUDE.md L05). Rename, or if this really is the same type moving homes, \
                 remove the old declaration in the same turn.",
                file.display()
            ));
        }
    }
    None
}

// ── post_edit: forbidden_ops gate ──────────────────────────────────────────

/// Detects usage of `regex` crate — forbidden for intent scans (L04: always
/// index-first, never grep-then-build-a-regex; regex-based intent extraction
/// makes a crate unshippable in a codebase this large).
fn check_regex_forbidden(content: &str) -> bool {
    content.contains("use regex") || content.contains("regex::") || content.contains("Regex::new(")
}

/// Detects usage of `glob` crate — forbidden for unbounded traversal (L04/T1:
/// bounded walks via forge_index_v3::walker, never recursive-glob). Catches
/// the import surface; generic unbounded custom recursion is a code-review
/// concern this gate cannot catch (deliberate scope limit, not oversight).
fn check_glob_forbidden(content: &str) -> bool {
    content.contains("use glob") || content.contains("glob::glob(") || content.contains("glob!(")
}

/// Within a `tick()` or `metarouter.rs` scope, detects heap allocations
/// (Vec/String/to_string/format) outside `#[cfg(test)]` blocks (L04: RT budget
/// requires zero-copy/zero-alloc hot paths, test-only allocations acceptable).
fn check_alloc_forbidden(file: &Path, content: &str) -> bool {
    let is_metarouter = file
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.contains("metarouter"))
        .unwrap_or(false);
    let has_tick_fn = content.contains("fn tick(");

    if !is_metarouter && !has_tick_fn {
        return false;
    }

    // Conservative scan: exclude `#[cfg(test)]` blocks via brace-depth tracking,
    // borrowing `is_inside_cfg_test`. A false negative (missing a real allocation
    // in test code) is acceptable; a false positive (blocking test-only code) is not.
    let forbidden_allocs = ["Vec::new(", "String::new(", ".to_string()", "format!("];
    for alloc in &forbidden_allocs {
        let mut start = 0;
        while let Some(rel) = content[start..].find(alloc) {
            let alloc_pos = start + rel;
            if !is_inside_cfg_test(content, alloc_pos) {
                return true;
            }
            start = alloc_pos + 1;
        }
    }
    false
}

/// L04/T1 forbidden_ops gate: checks three distinct rules covering regex,
/// glob, and hot-path allocations. Returns the first violation found (one
/// error message per gate call).
fn forbidden_ops_gate(_root: &Path, file: &Path, content: &str, file_path: &str) -> Option<String> {
    if content.is_empty() || !file_path.ends_with(".rs") {
        return None;
    }

    if check_regex_forbidden(content) {
        return Some(format!(
            "[foreman hook post-edit] L04 forbidden_ops: {} uses the 'regex' crate. \
             L04 index-first (goldminer, .forge/*.tsv, .idx roots) ALWAYS comes before grep. \
             An intent scan built on regex makes this crate unshippable (CLAUDE.md forbidden_ops). \
             Remove the regex import and use forge-index-v3's bounded search instead.",
            file.display()
        ));
    }

    if check_glob_forbidden(content) {
        return Some(format!(
            "[foreman hook post-edit] L04/T1 forbidden_ops: {} uses the 'glob' crate. \
             Unbounded recursive walks are forbidden (CLAUDE.md forbidden_ops). \
             Replace with forge_index_v3::walker::walk_bounded_skipping or a bounded direct walk. \
             (Note: this gate catches crate imports; generic custom recursion is a code-review concern.)",
            file.display()
        ));
    }

    if check_alloc_forbidden(file, content) {
        return Some(format!(
            "[foreman hook post-edit] L04 forbidden_ops: {} is a hot-path function \
             (metarouter.rs or fn tick()) but allocates on the heap (Vec::new, String::new, \
             .to_string(), format!). RT budget demands zero-copy/zero-alloc hot paths. \
             Move allocations outside the critical path or use stack/borrowed references. \
             (Note: allocations inside #[cfg(test)] blocks are permitted.)",
            file.display()
        ));
    }

    None
}

/// L22b/T1 receipt gate + L05 one-home gate + L04/T1 forbidden_ops gate,
/// then records the touched crate/witness scope into the per-session turn
/// ledger. `Ok(Some(json))` carries a block decision when any gate fires (all
/// are checked and reported — the last block, if any, is the one returned,
/// matching the prior print-both/last-wins-on-stdout behavior).
pub fn post_edit(root: &Path, stdin_text: &str) -> Result<Option<String>, String> {
    let file_path = json_str(stdin_text, "file_path")
        .or_else(|| json_str(stdin_text, "filePath"))
        .unwrap_or_default();
    if file_path.is_empty() {
        return Ok(None);
    }
    let file = PathBuf::from(&file_path);
    let content = json_str(stdin_text, "content").or_else(|| json_str(stdin_text, "new_string")).unwrap_or_default();

    let mut block: Option<String> = None;
    if let Some(reason) = receipt_gate(root, &file, &content) {
        block = Some(block_line(&reason));
    }
    if file_path.ends_with(".rs") {
        if let Some(reason) = one_home_gate(root, &file, &content) {
            block = Some(block_line(&reason));
        }
        if let Some(reason) = forbidden_ops_gate(root, &file, &content, &file_path) {
            block = Some(block_line(&reason));
        }
    }

    if !file_path.ends_with(".rs") {
        return Ok(block);
    }

    let entry = if file.starts_with(root.join("crates")) {
        match crate_name_above(root, &file) {
            Some(name) => format!("crate\t{name}"),
            None => return Ok(block),
        }
    } else if is_witness_scoped(root, &file) {
        "witness\t--all".to_string()
    } else {
        return Ok(block);
    };

    let ledger = ledger_path(root, stdin_text);
    if let Some(dir) = ledger.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let existing = std::fs::read_to_string(&ledger).unwrap_or_default();
    if existing.lines().any(|l| l == entry) {
        return Ok(block); // already queued for this turn's stop-gate
    }
    let mut body = existing;
    body.push_str(&entry);
    body.push('\n');
    std::fs::write(&ledger, body).map_err(|e| format!("hook post-edit: ledger write: {e}"))?;
    Ok(block)
}

// ── pre-grep: L04/L22b reminder, 15-minute debounce ─────────────────────────

/// Does the path look like a single file (short alphanumeric extension)?
pub fn looks_single_file(path: &str) -> bool {
    let last = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match last.rsplit_once('.') {
        Some((stem, ext)) => {
            !stem.is_empty()
                && (1..=6).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        }
        None => false,
    }
}

/// Actually run the L04 index-first chain (`cargo xtask search`) instead of
/// just reminding the caller to — goldminer + the three `.idx` sources +
/// PKM, in one already-built binary invocation. Bounded wait, never a hang:
/// a hard 5s deadline, polled via `try_wait` (zero-dep — no timeout crate),
/// killed and abandoned past it. `None` covers every non-happy path
/// (binary not built yet, spawn failure, timeout, unreadable stdout) —
/// the caller falls back to the advisory-only text rather than block or
/// silently pretend zero hits were found.
fn run_index_search(root: &Path, pattern: &str) -> Option<String> {
    if pattern.trim().is_empty() {
        return None;
    }
    let bin = [root.join("target").join("debug").join("xtask.exe"), root.join("target").join("release").join("xtask.exe")]
        .into_iter()
        .find(|p| p.exists())?;

    let mut child = Command::new(&bin)
        .args(["search", pattern, "--top", "5"])
        .current_dir(root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }

    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    // The summary line is the search verb's own last-printed row
    // ("search: N goldminer, N adr, N test, N river, N pkm hit(s) for ...");
    // a format change downgrades this to "no summary line found", not a crash.
    out.lines().rev().find(|l| l.starts_with("search:")).map(str::to_string)
}

/// Run `cmd`, killing the whole process tree if it outlives `deadline`.
/// Same spawn+`try_wait`+deadline shape as [`run_index_search`] above, sized
/// up to return captured output instead of a search summary. `Child::kill()`
/// alone only signals the immediate process — on Windows a hung `cargo`/
/// `rustc` child would survive that and keep holding the gate's target-dir
/// lock, which is exactly the failure this exists to prevent — so timeout
/// escalates to `taskkill /PID <pid> /T /F` (Receipt 2026-08-22: a Stop hook
/// stalled ~2 hours on an unbounded `Command::output()` here).
fn run_bounded(mut cmd: Command, deadline: Duration) -> Result<std::process::Output, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("could not spawn: {e}"))?;
    let pid = child.id();
    let end = Instant::now() + deadline;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output();
                let _ = child.wait();
                return Err(format!("TIMED OUT after {}s — process tree killed", deadline.as_secs()));
            }
            Err(e) => return Err(format!("wait failed: {e}")),
        }
    }
    child.wait_with_output().map_err(|e| format!("could not collect output: {e}"))
}

/// L04/L22b index-first reminder, advisory only — never a block decision.
pub fn pre_grep(root: &Path, stdin_text: &str) -> Result<Option<String>, String> {
    // Debounce (Sean 2026-08-16, "context poisoning"): at most one reminder
    // per 15 minutes; the law holds identically when stated once per interval.
    let stamp = root.join(".claude").join("hooks").join(".root-check-last");
    if let Ok(meta) = std::fs::metadata(&stamp) {
        if let Ok(modified) = meta.modified() {
            if let Ok(age) = modified.elapsed() {
                if age.as_secs() < 15 * 60 {
                    return Ok(None);
                }
            }
        }
    }
    let _ = std::fs::write(&stamp, "stamp");

    let pat = json_str(stdin_text, "pattern").unwrap_or_default();
    let path = json_str(stdin_text, "path").unwrap_or_else(|| "(default cwd)".into());

    // L04 index-first, actually run instead of only reminded: goldminer +
    // .idx + PKM via `cargo xtask search` (see `run_index_search` above).
    // `None` (tool unbuilt/timed out/etc.) falls back to the prior text
    // verbatim — the reminder never disappears, it's only upgraded when a
    // real answer is available in time.
    let (mut full, mut terse) = match run_index_search(root, &pat) {
        Some(summary) => (
            format!("L04 index-first for \"{pat}\" under {path}: {summary}. If this doesn't answer it, grep is the legitimate next tier."),
            format!("L04 ({summary})."),
        ),
        None => (
            format!(
                "L22 root-search / L04 index-first: before this grep for \"{pat}\" under {path}, run \
                 .forge/tools/goldminer.exe <dir> --toward \"{pat}\" and check .forge/census.tsv / \
                 .forge/domains.tsv / .forge/criticality.tsv FIRST. Summaries and Cargo.toml blurbs \
                 don't satisfy this gate. (index search unavailable this turn: xtask.exe not built \
                 or timed out — this is the fallback text, not a real result.)"
            ),
            String::from("L04: check goldminer/.forge/*.tsv before this grep. (index search unavailable this turn)"),
        ),
    };

    let single = looks_single_file(&path);
    let v3_only = !single
        && (path == "(default cwd)" || path.to_ascii_lowercase().starts_with("f:\\v3"));
    if v3_only {
        full.push_str(
            " L22b ABSENCE GATE (forced 2026-08-15): a grep scoped to F:\\v3 alone can NEVER support \
             a bare ABSENT claim, only an ABSENT-V3 one. Before writing 'does not exist' / 'ABSENT' \
             anywhere, this same pattern must ALSO be checked against F:\\NewRepo, E:\\.airgap, \
             F:\\_quarry, and E:\\13forge-super (bounded Test-Path or targeted grep, never a recursive \
             walk), OR the claim must be stated as 'not in F:\\v3, other roots unchecked'.",
        );
        terse.push_str(
            " L22b: v3-only != ABSENT. Also check NewRepo/.airgap/_quarry/13forge-super, or say 'not in v3, unchecked'.",
        );
    }
    Ok(Some(sys_and_context_line(&full, &terse)))
}

// ── pre-shell: refuse writes that walk around the Edit/Write tool (L18) ─────

/// Does this PowerShell command hold both a write verb/sink and a path into
/// the gated source trees? Plain substring matching, no regex (forbidden_ops).
pub fn shell_write_hits_source(cmd: &str) -> Option<&'static str> {
    let gated = (cmd.contains("crates\\") || cmd.contains("shell\\src\\")) && cmd.contains(".rs");
    if !gated {
        return None;
    }
    for tok in SHELL_WRITE_TOKENS {
        if cmd.contains(tok) {
            return Some(tok);
        }
    }
    // Redirection into a .rs file: a '>' with a .rs mention after it.
    if let Some(at) = cmd.find('>') {
        if cmd[at..].contains(".rs") {
            return Some(">");
        }
    }
    None
}

/// Does this PowerShell command DELETE something under a gated source tree?
/// No `.rs` requirement — deleting `crates\forge-audio-v3\src\fauna` (a whole
/// directory) has no file extension to match, and that is exactly the shape
/// of deletion this gate exists to catch.
pub fn shell_delete_hits_source(cmd: &str) -> Option<&'static str> {
    let gated = cmd.contains("crates\\") || cmd.contains("shell\\src\\") || cmd.contains("xtask\\src\\");
    if !gated {
        return None;
    }
    SHELL_DELETE_TOKENS.iter().find(|tok| cmd.contains(*tok)).copied()
}

/// Does this PowerShell command LAUNCH forgedaemon from a shell? Foreground
/// launch never returns (it's a server); any shell-spawned daemon inherits and
/// pins the caller's stdout pipe (POSTMORTEM-2026-08-23-DAEMON-PIPE-INHERIT).
/// Kill/query/build commands pass — the sanctioned cycle is build + Stop-Process,
/// then the next hook's `door_hook::spawn_daemon` redeploys and respawns clean.
pub fn shell_launches_daemon(cmd: &str) -> Option<&'static str> {
    if !cmd.contains("forgedaemon") {
        return None;
    }
    const LAUNCH_TOKENS: &[&str] = &["Start-Process", "Start-Job", "cargo run", "cmd /c start", "Invoke-Expression"];
    if let Some(tok) = LAUNCH_TOKENS.iter().find(|t| cmd.contains(*t)) {
        return Some(tok);
    }
    if cmd.contains("forgedaemon.exe") && (cmd.contains("& ") || cmd.trim_start().starts_with(".\\")) {
        return Some("&");
    }
    None
}

/// L18 shell-write/delete gate: refuses PowerShell commands that write or
/// delete inside a gated source tree, bypassing the Edit/Write tool layer.
pub fn pre_shell(stdin_text: &str) -> Result<Option<String>, String> {
    let cmd = json_str(stdin_text, "command").unwrap_or_default();
    if cmd.is_empty() {
        return Ok(None);
    }
    if let Some(tok) = shell_launches_daemon(&cmd) {
        return Ok(Some(block_line(&format!(
            "[foreman hook pre-shell] BLOCKED: this command launches forgedaemon from a shell (via '{tok}'). \
             A shell-launched daemon either never returns (foreground server) or inherits and pins this \
             call's stdout pipe until the daemon dies — the 3-day rebuild stall \
             (.forge/evidence/POSTMORTEM-2026-08-23-DAEMON-PIPE-INHERIT.md). The whole restart cycle is: \
             `cargo build -p forge-daemon-door --bin forgedaemon` then bounce it with \
             `F:\\v3\\target\\debug\\xtask.exe daemon shutdown` (graceful; `Stop-Process -Name forgedaemon \
             -Force` also works) and STOP — the next hook event auto-deploys .forge\\bin and respawns it detached."
        ))));
    }
    if let Some(tok) = shell_delete_hits_source(&cmd) {
        return Ok(Some(block_line(&format!(
            "[foreman hook pre-shell] BLOCKED: this PowerShell command DELETES a file or directory \
             under a gated source tree (crates\\, shell\\src\\, or xtask\\src\\) via '{tok}'. Whole-file \
             or whole-module removal is a destructive, hard-to-reverse diff (L17) and must never happen \
             as a side effect of a 'clean up dead code' task — surface the specific file(s) you believe \
             are dead and their zero-caller evidence, then get an explicit human go-ahead before any \
             delete. If you are restoring/reverting a file from a tape backup (E:\\v3), use Read+Write, \
             not a shell copy/delete pair."
        ))));
    }
    if let Some(tok) = shell_write_hits_source(&cmd) {
        return Ok(Some(block_line(&format!(
            "[foreman hook pre-shell] BLOCKED: this PowerShell command writes to a gated source file \
             (crates\\*.rs or shell\\src\\*.rs) via '{tok}', which bypasses the Edit/Write tool layer \
             and the turn-level `foreman hook stop` gate. Use the Edit or Write tool instead — \
             multi-edit welds are fine now; verification happens once per turn at Stop, not per edit."
        ))));
    }
    Ok(None)
}

// ── stop: gate the turn's touched work ONCE (replaces per-edit build+revert) ─

fn self_exe(root: &Path) -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| root.join(".forge").join("bin").join("foreman.exe"))
}

fn tail_of(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}

/// Bare `true`/`false` for `key` — `json_str` needs a `"`, booleans have none.
fn json_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let at = json.find(&needle)?;
    let rest = &json[at + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Harness-set on a stop-hook-forced retry. Skip re-gating — one report per red, not per retry.
fn is_stop_hook_active(stdin_text: &str) -> bool {
    json_bool(stdin_text, "stop_hook_active").unwrap_or(false)
}

/// Turn gate: runs `foreman gate --crate` on every crate touched this turn
/// (recorded by [`post_edit`]), reports RED honestly, never auto-reverts.
pub fn stop(root: &Path, stdin_text: &str) -> Result<Option<String>, String> {
    if is_stop_hook_active(stdin_text) {
        return Ok(None);
    }
    sweep_scratch(root, stdin_text);
    sweep_hook_snapshots(root, stdin_text);

    let ledger = ledger_path(root, stdin_text);
    let body = std::fs::read_to_string(&ledger).unwrap_or_default();

    let mut crates: Vec<&str> = Vec::new();
    let mut witness = false;
    for line in body.lines() {
        if let Some(name) = line.strip_prefix("crate\t") {
            if !crates.contains(&name) {
                crates.push(name);
            }
        } else if line.starts_with("witness\t") {
            witness = true;
        }
    }
    if crates.is_empty() && !witness {
        return Ok(None); // nothing gated this turn — stay silent, zero noise
    }

    let exe = self_exe(root);
    // Parallel, not sequential (2026-08-23): each crate's gate ran one after
    // another under its own 300s deadline, so an N-crate turn could wait up
    // to N*300s wall-clock even though every gate is an independent
    // subprocess. std::thread::scope runs them concurrently against the SAME
    // 300s ceiling — worst case stays ~300s regardless of crate count.
    let mut reds: Vec<String> = Vec::new();
    let mut greens: Vec<String> = Vec::new();
    let results: Vec<(String, Result<std::process::Output, String>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = crates
            .iter()
            .map(|krate| {
                let exe = &exe;
                let krate = (*krate).to_string();
                scope.spawn(move || {
                    let mut cmd = Command::new(exe);
                    cmd.args(["gate", "--crate", &krate, "--root"]).arg(root).current_dir(root);
                    let out = run_bounded(cmd, Duration::from_secs(300));
                    (krate, out)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| ("<panicked>".to_string(), Err("gate thread panicked".to_string())))).collect()
    });
    for (krate, out) in results {
        match out {
            Ok(o) if o.status.success() => greens.push(krate),
            Ok(o) => {
                let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&o.stderr));
                reds.push(format!("crate {krate} RED:\n{}", tail_of(&text, 30)));
            }
            Err(e) => reds.push(format!("crate {krate}: gate {e}")),
        }
    }
    // End-of-turn staleness check ("hardhook" — this Stop gate, once, for
    // files actually touched this turn): if forge-foreman-v3's OWN source
    // was edited and gated green above, the deployed `.forge/bin/
    // foreman.exe` still hasn't picked it up until a human rebuilds and
    // redeploys it (this agent cannot copy a `.exe` itself — a confirmed
    // harness restriction, not a missing step). A green crate gate proves
    // the SOURCE compiles; it says nothing about what's actually deployed.
    if crates.contains(&"forge-foreman-v3") {
        if let Some(msg) = crate::staleness::check(root) {
            reds.push(msg);
        }
    }

    // Sean 2026-08-24 ("Stop hook is launching a debug studio shell. Very
    // disruptive"): the pixel leg no longer EXECS `cargo xtask photon` here —
    // that built shell/ and mounted a live studio-shell.exe window at every
    // turn end, the same uncontrolled-window class the 2026-08-17 ruling
    // demoted `witness --all` for (.claude/hooks/attic-stop-hook-2026-08-17.md).
    // The photon is operator-invoked (`cargo xtask photon turn-gate`, at a
    // moment a human chose); this gate only READS its receipt: the capture
    // PNG's mtime must postdate the turn ledger's last witness-scoped edit.
    // WAVE_CLOSE=PHOTON holds — a pending photon keeps the witness row alive
    // across stops (never silently expires); it just never mounts glass.
    let mut photon_pending = false;
    if witness {
        let png = root.join(".forge").join("photons").join("turn-gate.png");
        let fresh = match (
            png.metadata().and_then(|m| m.modified()),
            std::fs::metadata(&ledger).and_then(|m| m.modified()),
        ) {
            (Ok(p), Ok(l)) => p > l,
            _ => false,
        };
        if fresh {
            greens.push("photon turn-gate (PNG mtime postdates turn ledger)".to_string());
        } else {
            photon_pending = true;
        }
    }

    if reds.is_empty() {
        if photon_pending {
            let kept: String = body.lines().filter(|l| l.starts_with("witness\t")).fold(String::new(), |mut s, l| {
                s.push_str(l);
                s.push('\n');
                s
            });
            let _ = std::fs::write(&ledger, kept);
            let crates_part = if greens.is_empty() {
                String::new()
            } else {
                format!(" Crate gates green: {}.", greens.join(", "))
            };
            return Ok(Some(sys_line(&format!(
                "[foreman hook stop] PHOTON PENDING — witness-scoped edits await their pixel receipt; \
                 wave stays open.{crates_part} Close it at a moment of your choosing: cargo xtask photon turn-gate"
            ))));
        }
        let _ = std::fs::remove_file(&ledger); // resolved — next turn starts clean
        Ok(Some(sys_line(&format!(
            "[foreman hook stop] turn gate GREEN — verified: {}",
            greens.join(", ")
        ))))
    } else {
        // Ledger is KEPT on red: the next stop re-gates the same work, so a
        // red can never silently expire. No file is ever reverted here.
        Ok(Some(block_with_sys_line(
            &format!(
                "[foreman hook stop] TURN GATE RED — the edits this turn did not verify. Nothing was \
                 reverted; fix the breaks and end the turn again. Green: [{}]. Red:\n{}",
                greens.join(", "),
                reds.join("\n---\n")
            ),
            &format!("foreman turn gate RED: {} red item(s), nothing auto-reverted", reds.len()),
        )))
    }
}

// ── stop: scratch sweep (Sean 2026-08-19: "scratch should last no longer
// than the agent session... cleaned via process consistently, not a ps1
// script") ───────────────────────────────────────────────────────────────

/// Session-scoped scratch TTL. Stop fires every turn end of THIS session, so
/// the sweep runs constantly and self-heals rather than depending on
/// SessionEnd firing reliably — a dir this old that isn't the CURRENT
/// session is dead.
const SCRATCH_TTL_SECS: u64 = 2 * 60 * 60;

/// Remove stale `.forge/_scratch/claude/<repo>/<session>` directories.
/// Bounded: two flat `fs::read_dir` levels, never a recursive walk (T1
/// unbound_io). Never touches the CURRENT session's dir, and never touches
/// anything outside `_scratch/claude/*` — the rest of `_scratch` is the
/// machine's general TEMP (rustc/node/WinGet/…), out of scope here.
fn sweep_scratch(root: &Path, stdin_text: &str) {
    sweep_scratch_at(root, stdin_text, std::time::SystemTime::now());
}

/// `now` is injected so TTL arithmetic is testable without modifying filesystem mtimes.
fn sweep_scratch_at(root: &Path, stdin_text: &str, now: std::time::SystemTime) {
    let current = json_str(stdin_text, "session_id");
    let claude_root = root.join(".forge").join("_scratch").join("claude");
    let Ok(repo_dirs) = std::fs::read_dir(&claude_root) else { return };

    for repo_entry in repo_dirs.flatten() {
        let repo_path = repo_entry.path();
        if !repo_path.is_dir() {
            // A loose file sitting directly at `.forge/_scratch/claude/*` (not
            // inside a `<repo>/<session>/` dir) is never a legitimate durable
            // artifact — the contract here is two levels deep, same reasoning
            // as `sweep_ephemeral_at`'s no-exemption floor. Age alone decides;
            // no session-ID exemption applies to a file that isn't a session.
            if let Ok(metadata) = std::fs::metadata(&repo_path) {
                if let Ok(modified) = metadata.modified() {
                    let age = now.duration_since(modified).unwrap_or_default();
                    if age.as_secs() >= SCRATCH_TTL_SECS {
                        let _ = std::fs::remove_file(&repo_path);
                    }
                }
            }
            continue;
        }

        let Ok(session_dirs) = std::fs::read_dir(&repo_path) else { continue };
        for session_entry in session_dirs.flatten() {
            let session_path = session_entry.path();
            if !session_path.is_dir() { continue; }

            let session_name = session_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if current.as_deref() == Some(session_name) {
                continue; // Never delete the active session
            }

            let Ok(metadata) = std::fs::metadata(&session_path) else { continue };
            let Ok(modified) = metadata.modified() else { continue };

            // A session dir carrying its own nested `.forge/` looks like it's
            // mimicking the real repo's durable-state tree (Sean 2026-08-19,
            // pointing at a stray `_scratch/spine5d-11608/.forge/river.idx`)
            // — that content may not have been pulled into F:\v3 proper yet.
            // Never guess; leave it for a human to look at instead of sweeping.
            if session_path.join(".forge").is_dir() {
                continue;
            }

            let age = now.duration_since(modified).unwrap_or_default();
            if age.as_secs() >= SCRATCH_TTL_SECS {
                let _ = std::fs::remove_dir_all(&session_path);
            }
        }
    }

    sweep_ephemeral_at(root, now);
}

/// Remove stale entries under `.forge/_scratch/ephemeral/*` — the sole
/// sanctioned OS-TEMP target (Sean 2026-08-19, closing the `%TEMP%`-into-
/// `_scratch` leak). One flat `fs::read_dir` level, no session-ID exemption:
/// nothing durable is allowed to land here by design (diagnostic evidence
/// goes to `.forge/evidence/` instead), so age alone is a safe delete
/// signal — unlike `_scratch/claude/*` above, which must protect the live
/// session. This is what "ban heuristic sweeps" means in practice: the
/// sweep stays dumb (age-only) because the FOLDER'S CONTRACT is narrow, not
/// because the sweep logic got smarter about file types.
fn sweep_ephemeral_at(root: &Path, now: std::time::SystemTime) {
    let ephemeral_root = root.join(".forge").join("_scratch").join("ephemeral");
    let Ok(entries) = std::fs::read_dir(&ephemeral_root) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = std::fs::metadata(&path) else { continue };
        let Ok(modified) = metadata.modified() else { continue };

        let age = now.duration_since(modified).unwrap_or_default();
        if age.as_secs() >= SCRATCH_TTL_SECS {
            if metadata.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

// ── stop: hook-snapshot sweep (2026-08-22: `.forge/hook-snapshots/` grows one
// `.bak`+`.path` pair per edited file per session, forever, with no eviction —
// confirmed live with leftover backups from an unrelated past project still
// on disk) ────────────────────────────────────────────────────────────────

/// Pre-edit backups are a safety net, not throwaway scratch — longer
/// retention than [`SCRATCH_TTL_SECS`] on purpose, its own named constant
/// rather than reusing that one, since the two data classes have different
/// retention intent.
const HOOK_SNAPSHOT_TTL_SECS: u64 = 24 * 60 * 60;

/// Grace period before an unreferenced `objects/<hash>` (or a stray
/// `objects/.tmp.*`) is eligible for collection. Closes the TOCTOU gap
/// between `handle_hook_snapshot` (forge-daemon-door) landing an object and
/// committing the `.path` pointer that references it: without this, a sweep
/// racing that exact window would see a real, just-written, momentarily
/// unreferenced object and delete it out from under an in-flight write.
const HOOK_SNAPSHOT_OBJECT_GRACE_SECS: u64 = 10 * 60;

/// Remove stale `.forge/hook-snapshots/<session_id>/` directories, purge any
/// leftover `.bak` file from the pre-CAS snapshot format (2026-08-23: full
/// byte-exact copies per edit per session, no dedup — this purge reclaims
/// what's already on disk immediately rather than waiting on the TTL), and
/// garbage-collect `objects/` entries no surviving `.path` pointer names.
/// One flat `fs::read_dir` level (T1 unbound_io), never the CURRENT
/// session's dir regardless of age — same shape as [`sweep_scratch_at`],
/// applied to the session-keyed snapshot layout `handle_hook_snapshot`
/// (forge-daemon-door) writes.
fn sweep_hook_snapshots(root: &Path, stdin_text: &str) {
    sweep_hook_snapshots_at(root, stdin_text, std::time::SystemTime::now());
}

/// `now` is injected so TTL arithmetic is testable without modifying filesystem mtimes.
fn sweep_hook_snapshots_at(root: &Path, stdin_text: &str, now: std::time::SystemTime) {
    let current = json_str(stdin_text, "session_id");
    let snapshots_root = root.join(".forge").join("hook-snapshots");
    let Ok(session_dirs) = std::fs::read_dir(&snapshots_root) else { return };

    let mut referenced_hashes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for session_entry in session_dirs.flatten() {
        let session_path = session_entry.path();
        if !session_path.is_dir() {
            continue;
        }
        let session_name = session_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let is_current = current.as_deref() == Some(session_name);

        // Legacy `.bak` purge: unconditional, in every session dir visited
        // (current session included — it can still carry pre-migration
        // `.bak` files written before this fix landed). A `.bak` is never
        // an in-flight write target (that's `objects/.tmp.*`, grace-period
        // guarded below), so it is safe to delete the moment it's seen.
        if let Ok(files) = std::fs::read_dir(&session_path) {
            for f in files.flatten() {
                if f.path().extension().and_then(|e| e.to_str()) == Some("bak") {
                    let _ = std::fs::remove_file(f.path());
                }
            }
        }

        if is_current {
            collect_referenced_hashes(&session_path, &mut referenced_hashes);
            continue; // Never delete the active session's own directory
        }

        let Ok(metadata) = std::fs::metadata(&session_path) else { continue };
        let Ok(modified) = metadata.modified() else { continue };
        let age = now.duration_since(modified).unwrap_or_default();
        if age.as_secs() >= HOOK_SNAPSHOT_TTL_SECS {
            let _ = std::fs::remove_dir_all(&session_path);
        } else {
            collect_referenced_hashes(&session_path, &mut referenced_hashes);
        }
    }

    sweep_orphaned_objects(&snapshots_root.join("objects"), &referenced_hashes, now);
}

/// Read every `v1`-schema `.path` pointer in a session dir and add the
/// `hash16hex` it names to `out`. Malformed/legacy pointers are skipped
/// (nothing to protect their object with — they'll fall out via the grace
/// period like any other orphan).
fn collect_referenced_hashes(session_path: &Path, out: &mut std::collections::HashSet<String>) {
    let Ok(files) = std::fs::read_dir(session_path) else { return };
    for f in files.flatten() {
        if f.path().extension().and_then(|e| e.to_str()) != Some("path") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(f.path()) else { continue };
        let mut lines = contents.lines();
        if lines.next() != Some("v1") {
            continue;
        }
        if let Some(hash) = lines.next() {
            out.insert(hash.to_string());
        }
    }
}

/// Delete any `objects/<hash>` not named by a surviving pointer, and any
/// stray `objects/.tmp.*` promote-in-progress leftover — both gated by
/// [`HOOK_SNAPSHOT_OBJECT_GRACE_SECS`] so a write still in flight is never
/// collected mid-write.
fn sweep_orphaned_objects(
    objects_dir: &Path,
    referenced_hashes: &std::collections::HashSet<String>,
    now: std::time::SystemTime,
) {
    let Ok(entries) = std::fs::read_dir(objects_dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let is_tmp = name.starts_with(".tmp.");
        let is_orphaned = is_tmp || !referenced_hashes.contains(&name);
        if !is_orphaned {
            continue;
        }
        let Ok(metadata) = std::fs::metadata(&path) else { continue };
        let Ok(modified) = metadata.modified() else { continue };
        let is_stale = now.duration_since(modified).unwrap_or_default().as_secs() >= HOOK_SNAPSHOT_OBJECT_GRACE_SECS;
        if is_stale {
            let _ = std::fs::remove_file(&path);
        }
    }
}

// ── session-end: the measured beat (port of foreman-beat.ps1) ───────────────

/// SessionEnd measured beat: runs the workspace gate and records PASS/FAIL
/// via `foreman beat`.
pub fn session_end(root: &Path) -> Result<Option<String>, String> {
    let exe = self_exe(root);
    // The verdict IS the workspace gate — a hardcoded PASS would blind DAUER.
    let gate = Command::new(&exe)
        .args(["gate", "--root"])
        .arg(root)
        .arg("--workspace")
        .current_dir(root)
        .output();
    let verdict = match &gate {
        Ok(o) if o.status.success() => "PASS",
        Ok(_) => "FAIL",
        Err(_) => "FAIL",
    };
    let _ = Command::new(&exe).arg("beat").arg(verdict).current_dir(root).output();
    Ok(Some(format!("[foreman hook session-end] recorded {verdict} (workspace gate)")))
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn stale_arm_notice() {
        assert!(super::stale_notice(3601).is_some_and(|s| s.contains("STALE")));
        assert!(super::stale_notice(300).is_none());
    }

    use super::*;

    #[test]
    fn json_str_extracts_flat_and_nested_first_occurrence() {
        let j = r#"{"session_id":"abc123","tool_input":{"file_path":"F:\\v3\\crates\\x\\src\\lib.rs","content":"\"file_path\" fake"}}"#;
        assert_eq!(json_str(j, "session_id").as_deref(), Some("abc123"));
        assert_eq!(json_str(j, "file_path").as_deref(), Some(r"F:\v3\crates\x\src\lib.rs"));
    }

    #[test]
    fn run_bounded_kills_process_tree_past_deadline() {
        // Sabotage receipt (L18): `ping -n 1000` (~1000s) would hang the old
        // unbounded `.output()` call for that long — the shape of the 2h
        // stall this fix exists for. Assert `run_bounded` returns well
        // before that AND that the spawned process is actually gone, not
        // just orphaned. `ping` (unlike `timeout`) needs no console/stdin.
        let mut cmd = Command::new("ping");
        cmd.args(["-n", "1000", "127.0.0.1"]);
        let start = Instant::now();
        let result = run_bounded(cmd, Duration::from_secs(2));
        let elapsed = start.elapsed();

        assert!(result.is_err(), "expected a timeout error, got {result:?}");
        assert!(result.unwrap_err().contains("TIMED OUT"));
        assert!(elapsed < Duration::from_secs(10), "took {elapsed:?}, should bail near the 2s deadline");

        // Confirm the tree was actually killed, not merely abandoned: no
        // `PING.EXE` should still be alive shortly after.
        std::thread::sleep(Duration::from_millis(300));
        let still_running = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq PING.EXE"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("ping.exe"))
            .unwrap_or(false);
        assert!(!still_running, "PING.EXE survived run_bounded's kill");
    }

    #[test]
    fn sweep_scratch_removes_old_sessions_keeps_current_and_fresh() {
        let root = std::env::temp_dir().join(format!("foreman-sweep-test-{}", std::process::id()));
        let claude_dir = root.join(".forge").join("_scratch").join("claude").join("F--v3");

        let old_session = claude_dir.join("old-session-uuid");
        let current_session = claude_dir.join("current-session-uuid");
        let fresh_session = claude_dir.join("fresh-session-uuid");

        for d in [&old_session, &current_session, &fresh_session] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("marker.txt"), b"x").unwrap();
        }

        // 1. Immediate sweep at real `now`: all dirs are fresh, none should be removed
        sweep_scratch_at(&root, r#"{"session_id":"current-session-uuid"}"#, std::time::SystemTime::now());
        assert!(old_session.exists());
        assert!(current_session.exists());
        assert!(fresh_session.exists());

        // 2. Simulated future time past TTL: unreferenced dirs are removed, active session survives
        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(SCRATCH_TTL_SECS + 60);
        sweep_scratch_at(&root, r#"{"session_id":"current-session-uuid"}"#, future_time);

        assert!(!old_session.exists(), "stale session must be swept");
        assert!(!fresh_session.exists(), "stale session must be swept");
        assert!(current_session.exists(), "active session_id must survive even if past TTL");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stop_witness_pending_keeps_row_and_never_blocks() {
        let root = std::env::temp_dir().join(format!("foreman-stop-witness-{}", std::process::id()));
        let hs = root.join(".forge").join("hook-state");
        std::fs::create_dir_all(&hs).unwrap();
        let ledger = hs.join("s1.touched");
        std::fs::write(&ledger, "witness\t--all\n").unwrap();

        let out = stop(&root, r#"{"session_id":"s1"}"#).unwrap().unwrap();
        assert!(out.contains("PHOTON PENDING"), "{out}");
        assert!(!out.contains("\"decision\":\"block\""), "pending photon must not block the stop: {out}");
        assert_eq!(
            std::fs::read_to_string(&ledger).unwrap(),
            "witness\t--all\n",
            "witness row must survive a pending stop — a wave never silently expires"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stop_witness_greens_on_fresh_photon_mtime_and_clears_ledger() {
        let root = std::env::temp_dir().join(format!("foreman-stop-photon-{}", std::process::id()));
        let hs = root.join(".forge").join("hook-state");
        std::fs::create_dir_all(&hs).unwrap();
        let ledger = hs.join("s2.touched");
        std::fs::write(&ledger, "witness\t--all\n").unwrap();
        let photons = root.join(".forge").join("photons");
        std::fs::create_dir_all(&photons).unwrap();
        // PNG mtime must be STRICTLY newer than the ledger's for the receipt to read fresh.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(photons.join("turn-gate.png"), b"png-bytes").unwrap();

        let out = stop(&root, r#"{"session_id":"s2"}"#).unwrap().unwrap();
        assert!(out.contains("GREEN"), "{out}");
        assert!(!ledger.exists(), "a fresh photon receipt closes the wave and clears the ledger");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_hook_snapshots_removes_old_sessions_keeps_current_and_fresh() {
        let root = std::env::temp_dir().join(format!("foreman-snap-sweep-test-{}", std::process::id()));
        let snapshots_dir = root.join(".forge").join("hook-snapshots");

        let old_session = snapshots_dir.join("old-session-uuid");
        let current_session = snapshots_dir.join("current-session-uuid");
        let fresh_session = snapshots_dir.join("fresh-session-uuid");

        for d in [&old_session, &current_session, &fresh_session] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("x.rs.bak"), b"x").unwrap();
        }

        // 1. Immediate sweep at real `now`: all dirs are fresh, none removed.
        sweep_hook_snapshots_at(&root, r#"{"session_id":"current-session-uuid"}"#, std::time::SystemTime::now());
        assert!(old_session.exists());
        assert!(current_session.exists());
        assert!(fresh_session.exists());

        // 2. Simulated future time past TTL: unreferenced dirs are removed, active session survives.
        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(HOOK_SNAPSHOT_TTL_SECS + 60);
        sweep_hook_snapshots_at(&root, r#"{"session_id":"current-session-uuid"}"#, future_time);

        assert!(!old_session.exists(), "stale snapshot session must be swept");
        assert!(!fresh_session.exists(), "stale snapshot session must be swept");
        assert!(current_session.exists(), "active session_id must survive even if past TTL");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_hook_snapshots_purges_legacy_bak_files_immediately() {
        let root = std::env::temp_dir().join(format!("foreman-snap-bak-purge-test-{}", std::process::id()));
        let snapshots_dir = root.join(".forge").join("hook-snapshots");
        let current_session = snapshots_dir.join("current-session-uuid");
        std::fs::create_dir_all(&current_session).unwrap();
        let legacy_bak = current_session.join("F__v3_CLAUDE.md.bak");
        std::fs::write(&legacy_bak, b"a whole file's worth of bytes").unwrap();

        // No TTL wait needed: a `.bak` is purged on sight, even in the
        // active session's own dir, even on an otherwise-fresh sweep.
        sweep_hook_snapshots_at(&root, r#"{"session_id":"current-session-uuid"}"#, std::time::SystemTime::now());

        assert!(!legacy_bak.exists(), "a legacy .bak must be reclaimed immediately, not wait on the 24h TTL");
        assert!(current_session.exists(), "the session dir itself must survive — only the .bak is purged");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Backdate a file's mtime by `secs_ago`, so grace-period arithmetic can
    /// be exercised without advancing the sweep's own `now` (which would
    /// also age the session-dir TTL check — a different clock, deliberately
    /// tested separately below).
    fn backdate(path: &Path, secs_ago: u64) {
        let past = std::time::SystemTime::now() - Duration::from_secs(secs_ago);
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(past).unwrap();
    }

    #[test]
    fn sweep_orphaned_objects_respects_the_toctou_grace_period() {
        let root = std::env::temp_dir().join(format!("foreman-snap-orphan-gc-test-{}", std::process::id()));
        let objects_dir = root.join(".forge").join("hook-snapshots").join("objects");
        std::fs::create_dir_all(&objects_dir).unwrap();

        // Unreferenced AND past the grace period: must be collected.
        let orphaned_hash = "1111111111111111";
        let orphaned_path = objects_dir.join(orphaned_hash);
        std::fs::write(&orphaned_path, b"stale").unwrap();
        backdate(&orphaned_path, HOOK_SNAPSHOT_OBJECT_GRACE_SECS + 60);

        // Unreferenced but freshly written (simulating the exact race:
        // object landed, .path not yet committed) — must survive.
        let racing_hash = "2222222222222222";
        std::fs::write(objects_dir.join(racing_hash), b"in flight").unwrap();

        // A stray leftover temp file from a promote that never completed,
        // also fresh — must likewise survive the grace period.
        std::fs::write(objects_dir.join(".tmp.9999-12345"), b"torn").unwrap();

        sweep_orphaned_objects(&objects_dir, &std::collections::HashSet::new(), std::time::SystemTime::now());

        assert!(!orphaned_path.exists(), "orphaned past its grace period must be collected");
        assert!(
            objects_dir.join(racing_hash).exists(),
            "unreferenced but under the grace period must survive — the TOCTOU guard"
        );
        assert!(
            objects_dir.join(".tmp.9999-12345").exists(),
            "a fresh stray temp file must also respect the grace period"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_orphaned_objects_keeps_a_referenced_object_regardless_of_age() {
        let root = std::env::temp_dir().join(format!("foreman-snap-orphan-gc-stale-test-{}", std::process::id()));
        let objects_dir = root.join(".forge").join("hook-snapshots").join("objects");
        std::fs::create_dir_all(&objects_dir).unwrap();

        let referenced_hash = "3333333333333333";
        let referenced_path = objects_dir.join(referenced_hash);
        std::fs::write(&referenced_path, b"kept").unwrap();
        backdate(&referenced_path, HOOK_SNAPSHOT_OBJECT_GRACE_SECS + 60);

        let orphan_hash = "4444444444444444";
        let orphan_path = objects_dir.join(orphan_hash);
        std::fs::write(&orphan_path, b"gone").unwrap();
        backdate(&orphan_path, HOOK_SNAPSHOT_OBJECT_GRACE_SECS + 60);

        let mut referenced = std::collections::HashSet::new();
        referenced.insert(referenced_hash.to_string());
        sweep_orphaned_objects(&objects_dir, &referenced, std::time::SystemTime::now());

        assert!(!orphan_path.exists(), "past the grace period, an unreferenced object is collected");
        assert!(referenced_path.exists(), "a referenced object survives regardless of age");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_hook_snapshots_at_gcs_an_object_left_by_a_ttl_swept_session() {
        // End-to-end through the public sweep: a session dir past the 24h
        // TTL is removed, and the object it alone referenced (also past its
        // own, much shorter, grace period) goes with it.
        let root = std::env::temp_dir().join(format!("foreman-snap-orphan-e2e-test-{}", std::process::id()));
        let snapshots_dir = root.join(".forge").join("hook-snapshots");
        let objects_dir = snapshots_dir.join("objects");
        std::fs::create_dir_all(&objects_dir).unwrap();
        let current_session = snapshots_dir.join("current-session-uuid");
        std::fs::create_dir_all(&current_session).unwrap();

        let orphaned_hash = "5555555555555555";
        let orphaned_path = objects_dir.join(orphaned_hash);
        std::fs::write(&orphaned_path, b"stale").unwrap();

        let old_session = snapshots_dir.join("old-session-uuid");
        std::fs::create_dir_all(&old_session).unwrap();
        std::fs::write(old_session.join("x.rs.path"), format!("v1\n{orphaned_hash}\nF:/v3/x.rs")).unwrap();

        // A single future `now`, same technique as
        // `sweep_hook_snapshots_removes_old_sessions_keeps_current_and_fresh`:
        // both the object and the session dir were created moments ago in
        // real time, so this one virtual clock jump pushes both the
        // session's 24h TTL and the object's 10min grace period into the
        // past at once — no mtime manipulation needed.
        let future = std::time::SystemTime::now() + Duration::from_secs(HOOK_SNAPSHOT_TTL_SECS + 180);
        sweep_hook_snapshots_at(&root, r#"{"session_id":"current-session-uuid"}"#, future);

        assert!(!old_session.exists(), "the referencing session dir was past TTL and must be swept");
        assert!(!orphaned_path.exists(), "the object it alone referenced, also past grace, must be collected");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The real bug found 2026-08-19: a loose file dropped directly at
    /// `.forge/_scratch/claude/` (not inside a `<repo>/<session>/` dir) was
    /// never touched by the sweep, so it persisted forever regardless of TTL.
    #[test]
    fn sweep_scratch_removes_a_stale_loose_file_at_the_claude_root() {
        let root = std::env::temp_dir().join(format!("foreman-sweep-loose-test-{}", std::process::id()));
        let claude_dir = root.join(".forge").join("_scratch").join("claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let stray = claude_dir.join("CLAUDE.md");
        std::fs::write(&stray, b"stray note").unwrap();

        // Fresh: survives.
        sweep_scratch_at(&root, r#"{"session_id":"unrelated"}"#, std::time::SystemTime::now());
        assert!(stray.exists(), "a fresh loose file must not be swept");

        // Past TTL: removed, no session-ID exemption (it isn't a session dir).
        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(SCRATCH_TTL_SECS + 60);
        sweep_scratch_at(&root, r#"{"session_id":"unrelated"}"#, future_time);
        assert!(!stray.exists(), "a stale loose file at the claude root must be swept");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_scratch_never_removes_a_session_carrying_a_nested_forge_tree() {
        let root = std::env::temp_dir().join(format!("foreman-sweep-forge-test-{}", std::process::id()));
        let claude_dir = root.join(".forge").join("_scratch").join("claude").join("F--v3");

        // Mirrors the real `spine5d-11608/.forge/river.idx` shape Sean flagged:
        // a stale-looking scratch session that nonetheless carries its own
        // `.forge/` subtree — content that may not be pulled into F:\v3 yet.
        let river_session = claude_dir.join("spine5d-11608");
        std::fs::create_dir_all(river_session.join(".forge")).unwrap();
        std::fs::write(river_session.join(".forge").join("river.idx"), b"row\n").unwrap();

        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(SCRATCH_TTL_SECS + 60);
        sweep_scratch_at(&root, r#"{"session_id":"unrelated-session"}"#, future_time);

        assert!(river_session.exists(), "a session dir with a nested .forge/ must never be auto-swept");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_ephemeral_removes_stale_entries_with_no_exemption() {
        let root = std::env::temp_dir().join(format!("foreman-sweep-ephemeral-test-{}", std::process::id()));
        let ephemeral_dir = root.join(".forge").join("_scratch").join("ephemeral");

        let old_dir = ephemeral_dir.join("forge-lora-io-test-9999");
        let fresh_dir = ephemeral_dir.join("forge-lora-io-test-9998");
        let old_file = ephemeral_dir.join("stray-tempfile.tmp");

        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::create_dir_all(&fresh_dir).unwrap();
        std::fs::write(old_dir.join("marker.txt"), b"x").unwrap();
        std::fs::write(&old_file, b"x").unwrap();

        // Immediate sweep at real `now`: everything is fresh, nothing removed.
        sweep_ephemeral_at(&root, std::time::SystemTime::now());
        assert!(old_dir.exists());
        assert!(fresh_dir.exists());
        assert!(old_file.exists());

        // Simulated future time past TTL: both the dir and the bare file go,
        // unconditionally — ephemeral/ has no session-ID exemption because
        // nothing durable is allowed to land there by contract.
        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(SCRATCH_TTL_SECS + 60);
        sweep_ephemeral_at(&root, future_time);

        assert!(!old_dir.exists(), "stale ephemeral dir must be swept");
        assert!(!old_file.exists(), "stale ephemeral file must be swept");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn json_str_unescapes() {
        let j = r#"{"command":"echo \"hi\"\n\tdone \u0041"}"#;
        assert_eq!(json_str(j, "command").as_deref(), Some("echo \"hi\"\n\tdone A"));
    }

    #[test]
    fn json_str_absent_or_unterminated_is_none() {
        assert_eq!(json_str(r#"{"a":1}"#, "b"), None);
        assert_eq!(json_str(r#"{"a":"unterminated"#, "a"), None);
    }

    #[test]
    fn json_escape_round_trips_through_extractor() {
        let raw = "line1\nline2 \"quoted\" back\\slash";
        let wrapped = format!("{{\"k\":\"{}\"}}", json_escape(raw));
        assert_eq!(json_str(&wrapped, "k").as_deref(), Some(raw));
    }

    #[test]
    fn package_name_parses_and_rejects() {
        assert_eq!(
            parse_package_name("[package]\nname = \"forge-foreman-v3\"\nversion = \"0.1.0\""),
            Some("forge-foreman-v3".to_string())
        );
        assert_eq!(parse_package_name("[package]\nversion = \"0.1.0\""), None);
    }

    #[test]
    fn witness_scope_is_the_whitelist_not_all_of_shell() {
        let root = Path::new("F:\\v3");
        assert!(is_witness_scoped(root, Path::new("F:\\v3\\shell\\src\\gpu.rs")));
        assert!(!is_witness_scoped(root, Path::new("F:\\v3\\shell\\src\\pty.rs")));
        assert!(!is_witness_scoped(root, Path::new("F:\\v3\\crates\\x\\src\\gpu.rs")));
    }

    #[test]
    fn single_file_detection() {
        assert!(looks_single_file("F:\\v3\\shell\\src\\gpu.rs"));
        assert!(looks_single_file("Cargo.toml"));
        assert!(!looks_single_file("F:\\v3\\shell\\src"));
        assert!(!looks_single_file("F:\\v3"));
    }

    #[test]
    fn shell_write_gate_hits_writes_and_passes_reads() {
        assert!(shell_write_hits_source("Set-Content crates\\x\\src\\lib.rs -Value hi").is_some());
        assert!(shell_write_hits_source("echo hi > shell\\src\\gpu.rs").is_some());
        assert!(shell_write_hits_source("Get-Content crates\\x\\src\\lib.rs").is_none());
        assert!(shell_write_hits_source("cargo test -p forge-core-v3").is_none());
        // Writes OUTSIDE the gated trees stay allowed (arming .loop-active etc.)
        assert!(shell_write_hits_source("Set-Content .claude\\hooks\\.loop-active -Value x").is_none());
    }

    #[test]
    fn shell_launch_gate_blocks_daemon_launches_and_passes_the_recipe() {
        // The stall shapes: foreground call, Start-Process, cargo run, cmd-start.
        assert!(shell_launches_daemon("& F:\\v3\\.forge\\bin\\forgedaemon.exe").is_some());
        assert!(shell_launches_daemon("Start-Process F:\\v3\\.forge\\bin\\forgedaemon.exe -WindowStyle Hidden").is_some());
        assert!(shell_launches_daemon("cargo run -p forge-daemon-door --bin forgedaemon").is_some());
        assert!(shell_launches_daemon("cmd /c start \"\" F:\\v3\\.forge\\bin\\forgedaemon.exe").is_some());
        assert!(shell_launches_daemon(".\\forgedaemon.exe").is_some());
        // The sanctioned cycle and queries pass.
        assert!(shell_launches_daemon("cargo build -p forge-daemon-door --bin forgedaemon").is_none());
        assert!(shell_launches_daemon("Stop-Process -Name forgedaemon -Force").is_none());
        assert!(shell_launches_daemon("Get-Process forgedaemon | Select-Object Id,Path").is_none());
        assert!(shell_launches_daemon("taskkill /IM forgedaemon.exe /F").is_none());
        // Unrelated commands never trip it.
        assert!(shell_launches_daemon("Start-Process notepad.exe").is_none());
    }

    #[test]
    fn shell_delete_gate_blocks_deletes_and_passes_reads() {
        // The exact incident: a whole directory deleted, no `.rs` extension in the path.
        assert!(shell_delete_hits_source("Remove-Item -Recurse -Force crates\\forge-audio-v3\\src\\fauna").is_some());
        // A single gated file via Remove-Item.
        assert!(shell_delete_hits_source("Remove-Item crates\\forge-audio-v3\\src\\realtime.rs").is_some());
        // xtask source is gated too.
        assert!(shell_delete_hits_source("Remove-Item xtask\\src\\revasc.rs").is_some());
        // Reads and non-delete commands stay allowed.
        assert!(shell_delete_hits_source("Get-ChildItem crates\\forge-audio-v3\\src\\fauna").is_none());
        assert!(shell_delete_hits_source("cargo test -p forge-audio-v3").is_none());
        // Deletes OUTSIDE the gated trees stay allowed (scratch/temp cleanup).
        assert!(shell_delete_hits_source("Remove-Item .forge\\_scratch\\tmp.txt").is_none());
    }

    // ── L22b/T1 receipt gate ────────────────────────────────────────────────

    #[test]
    fn claim_bearing_scopes_to_named_files_and_grind_log_only() {
        let root = Path::new("F:\\v3");
        assert!(is_claim_bearing(root, &root.join(".forge").join("grind-log").join("x.md")));
        assert!(is_claim_bearing(root, &root.join("crates").join("forge-book-v3").join("src").join("seams.rs")));
        assert!(is_claim_bearing(root, &root.join("crates").join("forge-core-v3").join("src").join("aspire.rs")));
        assert!(!is_claim_bearing(root, &root.join("crates").join("forge-core-v3").join("src").join("lib.rs")));
        assert!(!is_claim_bearing(root, &root.join(".forge").join("grind-log").join("x.txt")), "wrong extension");
    }

    #[test]
    fn receipt_gate_blocks_unreceipted_absence_passes_receipted_and_passes_unrelated_text() {
        let root = Path::new("F:\\v3");
        let seams = root.join("crates").join("forge-book-v3").join("src").join("seams.rs");

        // True-positive: absence claim, nothing backing it.
        assert!(receipt_gate(root, &seams, "// RouteExpert does not exist in v3").is_some());

        // True-negative: same claim, but carries a RECEIPT row.
        let receipted = "// RouteExpert does not exist in v3\nRECEIPT(claim:\"x\",verdict:ABSENT,roots:[\"F:/v3\"],anchor:\"a.rs:1\")";
        assert!(receipt_gate(root, &seams, receipted).is_none());

        // True-negative: [ASSUMED] tag also satisfies it.
        assert!(receipt_gate(root, &seams, "[ASSUMED] RouteExpert does not exist").is_none());

        // Out of scope: same unreceipted claim in a non-claim-bearing file.
        let other = root.join("crates").join("forge-core-v3").join("src").join("lib.rs");
        assert!(receipt_gate(root, &other, "// RouteExpert does not exist in v3").is_none());

        // No absence phrase at all: passes regardless of receipts.
        assert!(receipt_gate(root, &seams, "// landed and tested").is_none());
    }

    // ── L05 one-home gate ───────────────────────────────────────────────────

    #[test]
    fn is_inside_cfg_test_never_panics_on_non_ascii_content() {
        // Regression: an em-dash (3-byte UTF-8) before `pos` used to make the
        // byte-by-byte scan re-slice mid-character and panic. Any non-ASCII
        // prose anywhere before `pos` must not crash the scan.
        let src = "//! doc comment with an em-dash — right here\npub struct Live;\n";
        let pos = src.find("pub struct Live").unwrap();
        let _ = is_inside_cfg_test(src, pos); // must not panic
    }

    #[test]
    fn new_type_decls_skips_cfg_test_but_finds_live_items() {
        let src = "pub struct Live { pub x: u32 }\n#[cfg(test)]\nmod tests {\n    pub struct TestOnly;\n}\n";
        let names = new_type_decls(src);
        assert!(names.contains(&"Live"));
        assert!(!names.contains(&"TestOnly"), "cfg(test) items must not count as new live declarations");
    }

    fn one_home_scratch() -> PathBuf {
        std::env::temp_dir().join(format!(
            "one-home-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ))
    }

    #[test]
    fn one_home_gate_blocks_a_real_collision() {
        let root = one_home_scratch();
        let existing_dir = root.join("crates").join("forge-core-v3").join("src");
        std::fs::create_dir_all(&existing_dir).unwrap();
        std::fs::write(existing_dir.join("ghostmoon.rs"), "pub struct Ghostmoon { pub x0: i64 }").unwrap();

        let new_dir = root.join("crates").join("pp-math-v3").join("src");
        std::fs::create_dir_all(&new_dir).unwrap();
        let new_file = new_dir.join("ghostmoon.rs");

        let reason = one_home_gate(&root, &new_file, "pub struct Ghostmoon { pub x0: i64 }");
        assert!(reason.is_some(), "a second live Ghostmoon must be blocked");
        assert!(reason.unwrap().contains("ghostmoon.rs"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn one_home_gate_passes_a_genuinely_new_name_and_a_self_edit() {
        let root = one_home_scratch();
        let existing_dir = root.join("crates").join("forge-core-v3").join("src");
        std::fs::create_dir_all(&existing_dir).unwrap();
        let existing_file = existing_dir.join("ghostmoon.rs");
        std::fs::write(&existing_file, "pub struct Ghostmoon { pub x0: i64 }").unwrap();

        // A brand-new name has no collision anywhere.
        assert!(one_home_gate(&root, &existing_dir.join("other.rs"), "pub struct TotallyNewName;").is_none());

        // Editing the SAME file that already holds the declaration is not a
        // collision with itself.
        assert!(one_home_gate(&root, &existing_file, "pub struct Ghostmoon { pub x0: i64 }").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    // ── L04/T1 forbidden_ops gate ──────────────────────────────────────────

    #[test]
    fn forbidden_ops_gate_blocks_regex_import() {
        let root = Path::new("F:\\v3");
        let file = root.join("crates").join("forge-core-v3").join("src").join("lib.rs");

        // Positive: use regex
        assert!(forbidden_ops_gate(root, &file, "use regex;", "crates/forge-core-v3/src/lib.rs").is_some());

        // Positive: regex:: namespace
        assert!(forbidden_ops_gate(root, &file, "let r = regex::Regex::new(\"x\");", "crates/forge-core-v3/src/lib.rs").is_some());

        // Positive: Regex::new( call
        assert!(forbidden_ops_gate(root, &file, "Regex::new(\"pattern\")", "crates/forge-core-v3/src/lib.rs").is_some());

        // Negative: legitimate code without regex
        assert!(forbidden_ops_gate(root, &file, "fn process(s: &str) { println!(\"{s}\"); }", "crates/forge-core-v3/src/lib.rs").is_none());

        // Negative: regex as a substring in a comment/string (not actually imported)
        assert!(forbidden_ops_gate(root, &file, "// this regex pattern works great", "crates/forge-core-v3/src/lib.rs").is_none());
    }

    #[test]
    fn forbidden_ops_gate_blocks_glob_import() {
        let root = Path::new("F:\\v3");
        let file = root.join("crates").join("forge-core-v3").join("src").join("lib.rs");

        // Positive: use glob
        assert!(forbidden_ops_gate(root, &file, "use glob;", "crates/forge-core-v3/src/lib.rs").is_some());

        // Positive: glob::glob( call
        assert!(forbidden_ops_gate(root, &file, "for entry in glob::glob(\"*.rs\") {", "crates/forge-core-v3/src/lib.rs").is_some());

        // Positive: glob!( macro
        assert!(forbidden_ops_gate(root, &file, "let files = glob!(\"src/**/*.rs\");", "crates/forge-core-v3/src/lib.rs").is_some());

        // Negative: legitimate code without glob
        assert!(forbidden_ops_gate(root, &file, "let files = vec![\"a\", \"b\"];", "crates/forge-core-v3/src/lib.rs").is_none());

        // Negative: glob as substring (not the crate)
        assert!(forbidden_ops_gate(root, &file, "// glob pattern: **/*.txt", "crates/forge-core-v3/src/lib.rs").is_none());
    }

    #[test]
    fn forbidden_ops_gate_blocks_alloc_in_metarouter() {
        let root = Path::new("F:\\v3");
        let file = root.join("crates").join("forge-core-v3").join("src").join("metarouter.rs");

        // Positive: Vec::new() in metarouter
        assert!(forbidden_ops_gate(root, &file, "fn route() { let v = Vec::new(); }", "crates/forge-core-v3/src/metarouter.rs").is_some());

        // Positive: String::new() in metarouter
        assert!(forbidden_ops_gate(root, &file, "fn route() { let s = String::new(); }", "crates/forge-core-v3/src/metarouter.rs").is_some());

        // Positive: .to_string() in metarouter
        assert!(forbidden_ops_gate(root, &file, "fn route() { let s = \"hi\".to_string(); }", "crates/forge-core-v3/src/metarouter.rs").is_some());

        // Positive: format!() in metarouter
        assert!(forbidden_ops_gate(root, &file, "fn route() { let s = format!(\"x\"); }", "crates/forge-core-v3/src/metarouter.rs").is_some());

        // Negative: allocation inside #[cfg(test)]
        let test_code = "#[cfg(test)]\nmod tests {\n    fn test() { let v = Vec::new(); }\n}";
        assert!(forbidden_ops_gate(root, &file, test_code, "crates/forge-core-v3/src/metarouter.rs").is_none());

        // Negative: no allocation in metarouter
        assert!(forbidden_ops_gate(root, &file, "fn route(s: &str) -> &str { s }", "crates/forge-core-v3/src/metarouter.rs").is_none());
    }

    #[test]
    fn forbidden_ops_gate_blocks_alloc_in_tick_function() {
        let root = Path::new("F:\\v3");
        let file = root.join("crates").join("forge-audio-v3").join("src").join("lib.rs");

        // Positive: Vec::new() in a file with fn tick()
        let tick_src = "fn tick() { let v = Vec::new(); }";
        assert!(forbidden_ops_gate(root, &file, tick_src, "crates/forge-audio-v3/src/lib.rs").is_some());

        // Positive: String::new() in a file with fn tick()
        let tick_src = "fn tick() { let s = String::new(); }";
        assert!(forbidden_ops_gate(root, &file, tick_src, "crates/forge-audio-v3/src/lib.rs").is_some());

        // Negative: allocation outside fn tick() scope
        let no_tick = "fn process() { let v = Vec::new(); }\nfn other() { let s = \"hi\".to_string(); }";
        assert!(forbidden_ops_gate(root, &file, no_tick, "crates/forge-audio-v3/src/lib.rs").is_none());

        // Negative: fn tick() with no allocations
        let clean_tick = "fn tick() { for x in 0..10 { process(x); } }";
        assert!(forbidden_ops_gate(root, &file, clean_tick, "crates/forge-audio-v3/src/lib.rs").is_none());
    }

    #[test]
    fn forbidden_ops_gate_ignores_non_rs_files() {
        let root = Path::new("F:\\v3");
        let file = root.join("README.md");

        // Even with forbidden patterns, non-.rs files pass (out of scope)
        assert!(forbidden_ops_gate(root, &file, "use regex;", "README.md").is_none());
        assert!(forbidden_ops_gate(root, &file, "use glob;", "README.md").is_none());
    }

    #[test]
    fn forbidden_ops_gate_passes_clean_code() {
        let root = Path::new("F:\\v3");
        let file = root.join("crates").join("forge-core-v3").join("src").join("lib.rs");
        let clean = "
            pub fn process(input: &[u8]) -> Option<&str> {
                if input.is_empty() {
                    return None;
                }
                std::str::from_utf8(input).ok()
            }
        ";
        assert!(forbidden_ops_gate(root, &file, clean, "crates/forge-core-v3/src/lib.rs").is_none());
    }
}
