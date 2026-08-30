//! Claim resolution — the byte scanner that finds path anchors in a line, and
//! the predicate that says whether one points outside this tree. Pure: no fs,
//! no alloc beyond the captured strings. Disk probing belongs to the caller.

/// Receipted root corrections, checked when a drive path is dead as written.
/// Each entry: (miscited prefix, corrected prefix, receipt).
///
/// Ported verbatim from `xtask/src/book_drift.rs:37-41` (2026-08-29 extraction);
/// that file keeps driving, this is now the one home (L05).
pub const ROOT_CORRECTIONS: &[(&str, &str, &str)] = &[(
    "E:\\airgap\\",
    "E:\\.airgap\\",
    "C-1 2026-08-17: Test-Path proved the dotted root live where the bare one is not",
)];

/// Repo roots that are NOT this tree. A runtime path into one of these is a
/// standing-law violation (Sean 2026-08-29: nothing hardcoded into a previous
/// repo). Matched case-insensitively, either slash direction.
pub const FOREIGN_ROOTS: &[&str] = &[
    "f:/newrepo",
    "f:/.reposold",
    "f:/13forge-super",
    "f:/_quarry",
    "f:/nistam-",
    "e:/",
    "g:/",
    "c:/users/seanm/desktop",
];

/// What a captured candidate is, so the caller knows how to resolve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    /// A drive-absolute path (`X:\...`).
    Drive,
    /// A repo-relative path (`.forge/...`, `crates/...`).
    Relative,
}

/// One path anchor found on a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The captured path text, trailing punctuation stripped.
    pub text: String,
    /// How to resolve it.
    pub kind: CandidateKind,
    /// Byte offset the capture started at.
    pub at: usize,
}

/// Bytes that may appear inside a path capture.
fn is_path_byte(b: u8) -> bool {
    !matches!(
        b,
        b' ' | b'\t'
            | b'"'
            | b'\''
            | b'`'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'<'
            | b'>'
            | b'|'
            | b'*'
            | b','
            | b';'
    )
}

/// True when position `i` starts a fresh token (line start, or a boundary byte
/// before it) — keeps `sub.forge/` or `mycrates/` from matching.
pub fn at_token_start(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    !matches!(bytes[i - 1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'\\' | b'/')
}

fn strip_trailing(mut s: String) -> String {
    while s.ends_with('.') || s.ends_with('\\') || s.ends_with('/') {
        s.pop();
    }
    s
}

/// Capture a drive-absolute path starting at `start` (pointing at the drive
/// letter). The drive colon is the only legal `:`; a later `:` ends the capture,
/// which is what strips `path.rs:43`-style line suffixes.
pub fn capture_drive_path(bytes: &[u8], start: usize) -> String {
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b':' && end != start + 1 {
            break;
        }
        if b != b':' && !is_path_byte(b) {
            break;
        }
        end += 1;
    }
    strip_trailing(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}

/// Capture a repo-relative path at `start`. Any `:` terminates — a relative
/// path carries no drive colon.
pub fn capture_relative_path(bytes: &[u8], start: usize) -> String {
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b':' || !is_path_byte(b) {
            break;
        }
        end += 1;
    }
    strip_trailing(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}

/// Every path anchor on one line, in order. Pure — nothing is probed.
pub fn scan_line(line: &str) -> Vec<Candidate> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();

    let mut i = 0usize;
    while i + 2 < bytes.len() {
        if bytes[i].is_ascii_alphabetic()
            && bytes[i + 1] == b':'
            && (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/')
            && at_token_start(bytes, i)
        {
            let text = capture_drive_path(bytes, i);
            if !text.is_empty() {
                i += text.len().max(1);
                out.push(Candidate { text, kind: CandidateKind::Drive, at: i });
                continue;
            }
        }
        i += 1;
    }

    for prefix in [".forge/", ".forge\\", "crates/", "crates\\", "shell/src/", "xtask/src/"] {
        let mut from = 0usize;
        while let Some(pos) = line[from..].find(prefix) {
            let abs = from + pos;
            if at_token_start(bytes, abs) {
                let text = capture_relative_path(bytes, abs);
                if !text.is_empty() {
                    out.push(Candidate { text, kind: CandidateKind::Relative, at: abs });
                }
            }
            from = abs + prefix.len();
        }
    }

    out.sort_by_key(|c| c.at);
    out
}

/// A proof tag found on a line, and the path it names inline (colon form only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofTag {
    /// The path carried inside the tag, e.g. `crates/GEMINI.md:8` -> `crates/GEMINI.md`.
    /// `None` for the bare `[PROVEN]` form, whose anchors live elsewhere on the line.
    pub inline_path: Option<String>,
}

/// Every proof tag on a line. Recognises BOTH authored forms:
/// bare `[PROVEN]`, and `[PROVEN:<path>[:<line>]]`.
///
/// `book_drift.rs:132` recognised only the bare literal, so the 34 colon-form
/// tags in `11-sovereign-routing-topology.md` were invisible and their dead
/// paths reported WARN instead of FATAL (receipted 2026-08-29).
pub fn proof_tags(line: &str) -> Vec<ProofTag> {
    const OPEN: &str = "[PROVEN";
    let mut out = Vec::new();
    let mut from = 0usize;

    while let Some(pos) = line[from..].find(OPEN) {
        let abs = from + pos;
        let after = abs + OPEN.len();
        from = after;

        let rest = &line[after..];
        match rest.as_bytes().first() {
            Some(b']') => out.push(ProofTag { inline_path: None }),
            Some(b':') => {
                let body = match rest[1..].find(']') {
                    Some(end) => &rest[1..1 + end],
                    None => continue, // unterminated tag: report nothing, never guess
                };
                out.push(ProofTag { inline_path: strip_line_suffix(body) });
            }
            _ => {} // `[PROVENANCE` and friends are not proof tags
        }
    }

    out
}

/// `crates/GEMINI.md:8` -> `crates/GEMINI.md`. Keeps a drive colon intact.
fn strip_line_suffix(body: &str) -> Option<String> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    let bytes = body.as_bytes();
    let mut cut = body.len();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b':' && i != 1 {
            cut = i;
            break;
        }
    }
    let s = strip_trailing(body[..cut].to_string());
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// True when `path` points into a repo that is not this tree.
pub fn is_foreign_root(path: &str) -> bool {
    let norm: String = path
        .chars()
        .map(|c| if c == '\\' { '/' } else { c.to_ascii_lowercase() })
        .collect();
    FOREIGN_ROOTS.iter().any(|r| norm.starts_with(r))
}

/// The receipted correction for a miscited root, if one applies.
pub fn root_correction(path: &str) -> Option<(String, &'static str)> {
    for (bad, good, receipt) in ROOT_CORRECTIONS {
        if let Some(rest) = path.strip_prefix(bad) {
            return Some((format!("{good}{rest}"), receipt));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_suffix_is_stripped_from_a_drive_path() {
        let line = "see F:\\v3\\crates\\forge-core-v3\\src\\lib.rs:43 for the anchor";
        let c = scan_line(line);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, "F:\\v3\\crates\\forge-core-v3\\src\\lib.rs");
        assert_eq!(c[0].kind, CandidateKind::Drive);
    }

    #[test]
    fn token_boundary_refuses_a_path_glued_to_a_word() {
        assert!(scan_line("mycrates/forge-core-v3").is_empty());
        assert!(scan_line("sub.forge/books").is_empty());
        assert!(!scan_line("in crates/forge-core-v3").is_empty());
    }

    #[test]
    fn foreign_roots_are_recognised_either_slash_and_any_case() {
        assert!(is_foreign_root("F:/NewRepo/crates/x.rs"));
        assert!(is_foreign_root("F:\\NewRepo\\crates\\x.rs"));
        assert!(is_foreign_root("f:/newrepo/x"));
        assert!(is_foreign_root("E:/airgap/thing"));
        assert!(is_foreign_root("G:\\E DRIVE\\v3"));
        assert!(is_foreign_root("C:/Users/seanm/Desktop/x"));
    }

    #[test]
    fn this_tree_is_not_foreign() {
        assert!(!is_foreign_root("F:/v3/crates/forge-core-v3/src/lib.rs"));
        assert!(!is_foreign_root("F:\\v3\\shell\\src\\main.rs"));
        assert!(!is_foreign_root("crates/forge-core-v3/src/lib.rs"));
        assert!(!is_foreign_root(".forge/grind-log/x.md"));
    }

    #[test]
    fn the_c1_root_correction_still_applies() {
        let (fixed, receipt) = root_correction("E:\\airgap\\snap\\x.rs").expect("C-1 correction");
        assert_eq!(fixed, "E:\\.airgap\\snap\\x.rs");
        assert!(receipt.contains("C-1"));
        assert!(root_correction("F:\\v3\\x.rs").is_none());
    }

    #[test]
    fn a_line_with_no_anchor_yields_nothing() {
        assert!(scan_line("this sentence cites no path at all").is_empty());
        assert!(scan_line("").is_empty());
    }

    #[test]
    fn several_anchors_come_back_in_line_order() {
        let line = "crates/forge-a and .forge/books then F:\\v3\\x.rs";
        let c = scan_line(line);
        assert!(c.len() >= 3, "got {c:?}");
        let mut prev = 0;
        for cand in &c {
            assert!(cand.at >= prev, "candidates must be ordered: {c:?}");
            prev = cand.at;
        }
    }

    #[test]
    fn quotes_and_backticks_bound_a_capture() {
        let c = scan_line("`crates/forge-core-v3/src/lib.rs` trailing");
        assert_eq!(c[0].text, "crates/forge-core-v3/src/lib.rs");
        let d = scan_line("\"F:\\v3\\a.rs\", next");
        assert_eq!(d[0].text, "F:\\v3\\a.rs");
    }
}
