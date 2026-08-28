//! Weld RON → anchored file edits. Parse, plan, apply — the bounded-mutator
//! lane (WAVE-WELD W2).
//!
//! Ported from `F:\NewRepo\crates\forge-daemon\src\weld.rs` (~700 lines).
//! Kept whole: the `Weld`/`FileEdits`/`Edit`/`Op` types, the exactly-once
//! anchor rule ("0 or 2+ is a typed refusal", v2 line 3), typed [`WeldErr`],
//! plan-before-write, gate-judged commit with rollback. Stripped at the
//! customs gate, each with its reason:
//! - `forge_core::line_diff` — the unified-diff pretty print; the grind log
//!   journals old/new whole, and a diff renderer is a visual concern with no
//!   consumer in this tree yet;
//! - `record_weld_scar` / `backtick` / `WeldTape` — v2 taped through
//!   `forge_vcs` directly; in v3 the foreman already owns stamped tape commits
//!   (`commit_many_stamped`), and a second tape door would be an L05 defect;
//! - the CLI dry-run driver — the foreman's `weld` verb is the process.
//!
//! Sidecar-side generation of these payloads is grammar-clamped
//! (`sidecar/src/constrain.rs`); this parser re-validates on receive —
//! defense in depth, exactly as v2 ran it (daemon/protocol.rs:88).

use std::path::{Path, PathBuf};

/// What an edit does at its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Replace the anchor text with the payload.
    Replace,
    /// Insert the payload immediately before the anchor.
    Before,
    /// Insert the payload immediately after the anchor.
    After,
    /// Delete the anchor text (payload ignored).
    Delete,
}

impl Op {
    /// Parse the closed op vocabulary; anything else is [`WeldErr::UnknownOp`].
    pub fn parse(s: &str) -> Option<Op> {
        match s {
            "replace" => Some(Op::Replace),
            "before" => Some(Op::Before),
            "after" => Some(Op::After),
            "delete" => Some(Op::Delete),
            _ => None,
        }
    }
}

/// One anchored edit: find `anchor` exactly once, do `op` with `payload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The exact text to locate — must occur exactly once in the file.
    pub anchor: String,
    /// What to do at the anchor.
    pub op: Op,
    /// The replacement/insertion text (empty for `delete`).
    pub payload: String,
}

/// All edits against one file, applied in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdits {
    /// Path relative to the weld root.
    pub path: String,
    /// The edits, applied sequentially to the same buffer.
    pub edits: Vec<Edit>,
}

/// One parsed weld: `Weld(lane,files:[F(p,edits:[E(anchor,op,payload)])],gate,receipt)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weld {
    /// The lane label the payload carries (provenance, not mechanism).
    pub lane: String,
    /// Files to edit.
    pub files: Vec<FileEdits>,
    /// The gate command the weld claims to satisfy (informational; the foreman
    /// runs its OWN gate, never the payload's).
    pub gate: String,
    /// The receipt label the payload carries.
    pub receipt: String,
}

/// Typed refusals — every way a weld fails to parse or apply, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeldErr {
    /// The RON shape broke at a byte offset.
    Syntax {
        /// Byte offset of the failure.
        at: usize,
        /// What the parser wanted there.
        want: &'static str,
    },
    /// An op outside `replace|before|after|delete`.
    UnknownOp(String),
    /// A bad string escape at a byte offset.
    Escape(usize),
    /// The anchor was not found — the file drifted or the model guessed.
    AnchorMissing {
        /// File the anchor was sought in.
        path: String,
        /// The anchor text.
        anchor: String,
    },
    /// The anchor matched more than once — ambiguous, refused, never guessed.
    AnchorAmbiguous {
        /// File the anchor was sought in.
        path: String,
        /// The anchor text.
        anchor: String,
        /// How many times it matched.
        hits: usize,
    },
    /// Filesystem trouble reading or writing a planned file.
    Io {
        /// File involved.
        path: String,
        /// The OS's words.
        msg: String,
    },
}

impl std::fmt::Display for WeldErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeldErr::Syntax { at, want } => write!(f, "syntax at byte {at}: expected {want}"),
            WeldErr::UnknownOp(o) => write!(f, "unknown op `{o}` (replace|before|after|delete)"),
            WeldErr::Escape(at) => write!(f, "bad escape at byte {at}"),
            WeldErr::AnchorMissing { path, anchor } => {
                write!(f, "{path}: anchor not found: {anchor:?}")
            }
            WeldErr::AnchorAmbiguous { path, anchor, hits } => {
                write!(f, "{path}: anchor matched {hits}x, need exactly 1: {anchor:?}")
            }
            WeldErr::Io { path, msg } => write!(f, "{path}: {msg}"),
        }
    }
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.ws();
        *self.s.get(self.i).unwrap_or(&0)
    }

    fn eat(&mut self, c: u8, want: &'static str) -> Result<(), WeldErr> {
        if self.peek() == c {
            self.i += 1;
            Ok(())
        } else {
            Err(WeldErr::Syntax { at: self.i, want })
        }
    }

    fn sniff(&mut self, c: u8) -> bool {
        if self.peek() == c {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn ident(&mut self) -> &'a str {
        self.ws();
        let a = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_')
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[a..self.i]).unwrap_or("")
    }

    /// Skip an optional `name:` label so positional and named fields both parse.
    fn label(&mut self) {
        let save = self.i;
        if !self.ident().is_empty() && self.peek() == b':' {
            self.i += 1;
        } else {
            self.i = save;
        }
    }

    fn string(&mut self) -> Result<String, WeldErr> {
        self.eat(b'"', "\"")?;
        let mut out = String::new();
        loop {
            let Some(&c) = self.s.get(self.i) else {
                return Err(WeldErr::Syntax { at: self.i, want: "closing \"" });
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return Err(WeldErr::Escape(self.i));
                    };
                    self.i += 1;
                    out.push(match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'\\' => '\\',
                        b'"' => '"',
                        _ => return Err(WeldErr::Escape(self.i - 1)),
                    });
                }
                _ => {
                    let a = self.i - 1;
                    while self.i < self.s.len() && self.s[self.i] & 0xC0 == 0x80 {
                        self.i += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.s[a..self.i]).unwrap_or("\u{fffd}"));
                }
            }
        }
    }
}

/// Parse one weld. Named and positional field labels both accepted.
pub fn parse(src: &str) -> Result<Weld, WeldErr> {
    let mut p = P { s: src.as_bytes(), i: 0 };
    if p.ident() != "Weld" {
        return Err(WeldErr::Syntax { at: 0, want: "Weld(" });
    }
    p.eat(b'(', "(")?;

    p.label();
    let lane = p.string()?;
    p.eat(b',', ",")?;

    p.label();
    p.eat(b'[', "[")?;
    let mut files = Vec::new();
    while !p.sniff(b']') {
        if p.ident() != "F" {
            return Err(WeldErr::Syntax { at: p.i, want: "F(" });
        }
        p.eat(b'(', "(")?;
        p.label();
        let path = p.string()?;
        p.eat(b',', ",")?;
        p.label();
        p.eat(b'[', "[")?;
        let mut edits = Vec::new();
        while !p.sniff(b']') {
            if p.ident() != "E" {
                return Err(WeldErr::Syntax { at: p.i, want: "E(" });
            }
            p.eat(b'(', "(")?;
            p.label();
            let anchor = p.string()?;
            p.eat(b',', ",")?;
            p.label();
            let raw = p.string()?;
            let op = Op::parse(&raw).ok_or(WeldErr::UnknownOp(raw))?;
            let payload = if p.sniff(b',') {
                p.label();
                p.string()?
            } else {
                String::new()
            };
            p.eat(b')', ")")?;
            p.sniff(b',');
            edits.push(Edit { anchor, op, payload });
        }
        p.eat(b')', ")")?;
        p.sniff(b',');
        files.push(FileEdits { path, edits });
    }
    p.eat(b',', ",")?;

    p.label();
    let gate = p.string()?;
    p.eat(b',', ",")?;
    p.label();
    let receipt = p.string()?;
    p.sniff(b',');
    p.eat(b')', ")")?;
    Ok(Weld { lane, files, gate, receipt })
}

/// Apply one edit to a buffer. Anchor must occur exactly once — 0 or 2+ is a
/// typed refusal, never a guess.
pub fn splice(buf: &str, e: &Edit, path: &str) -> Result<String, WeldErr> {
    let hits = buf.matches(e.anchor.as_str()).count();
    match hits {
        0 => {
            return Err(WeldErr::AnchorMissing { path: path.into(), anchor: e.anchor.clone() });
        }
        1 => {}
        n => {
            return Err(WeldErr::AnchorAmbiguous {
                path: path.into(),
                anchor: e.anchor.clone(),
                hits: n,
            });
        }
    }
    let at = buf.find(e.anchor.as_str()).expect("hits==1");
    let end = at + e.anchor.len();
    Ok(match e.op {
        Op::Replace => format!("{}{}{}", &buf[..at], e.payload, &buf[end..]),
        Op::Before => format!("{}{}{}", &buf[..at], e.payload, &buf[at..]),
        Op::After => format!("{}{}{}", &buf[..end], e.payload, &buf[end..]),
        Op::Delete => format!("{}{}", &buf[..at], &buf[end..]),
    })
}

/// One file's resolved edit: the pre-weld bytes and the post-weld bytes.
pub struct Planned {
    /// Path relative to the weld root.
    pub path: String,
    /// Pre-weld file contents — the rollback bytes.
    pub old: String,
    /// Post-weld file contents.
    pub new: String,
}

/// Resolve every file's edits against disk. Pure decision — no write.
pub fn plan(w: &Weld, root: &Path) -> Result<Vec<Planned>, WeldErr> {
    let mut out = Vec::new();
    for f in &w.files {
        let abs = root.join(&f.path);
        let old = std::fs::read_to_string(&abs)
            .map_err(|e| WeldErr::Io { path: f.path.clone(), msg: e.to_string() })?;
        let mut cur = old.clone();
        for e in &f.edits {
            cur = splice(&cur, e, &f.path)?;
        }
        out.push(Planned { path: f.path.clone(), old, new: cur });
    }
    Ok(out)
}

fn write_atomic(abs: &Path, body: &str, label: &str) -> Result<(), WeldErr> {
    let tmp: PathBuf = abs.with_extension("weld.tmp");
    std::fs::write(&tmp, body)
        .map_err(|e| WeldErr::Io { path: label.into(), msg: e.to_string() })?;
    std::fs::rename(&tmp, abs)
        .map_err(|e| WeldErr::Io { path: label.into(), msg: e.to_string() })
}

/// Land a plan. Every file is resolved before the first byte is written.
pub fn commit(plan: &[Planned], root: &Path) -> Result<(), WeldErr> {
    for p in plan {
        write_atomic(&root.join(&p.path), &p.new, &p.path)?;
    }
    Ok(())
}

/// Restore every planned file to the bytes `plan` read.
pub fn rollback(plan: &[Planned], root: &Path) -> Result<(), WeldErr> {
    for p in plan {
        write_atomic(&root.join(&p.path), &p.old, &p.path)?;
    }
    Ok(())
}

/// Commit, then judge. A false verdict restores the pre-weld bytes. Returns
/// whether the weld stuck. `gate` is injected so the ratchet is testable
/// without a subprocess.
pub fn commit_gated(
    plan: &[Planned],
    root: &Path,
    gate: impl FnOnce() -> bool,
) -> Result<bool, WeldErr> {
    commit(plan, root)?;
    if gate() {
        Ok(true)
    } else {
        rollback(plan, root)?;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WELD: &str = r#"Weld(lane:"W1",files:[F(p:"src/lib.rs",edits:[E(anchor:"fn old()",op:"replace",payload:"fn new()")])],gate:"cargo check",receipt:"r1")"#;

    #[test]
    fn a_weld_parses_into_its_typed_shape() {
        let w = parse(WELD).unwrap();
        assert_eq!(w.lane, "W1");
        assert_eq!(w.files.len(), 1);
        assert_eq!(w.files[0].path, "src/lib.rs");
        assert_eq!(w.files[0].edits[0].anchor, "fn old()");
        assert_eq!(w.files[0].edits[0].op, Op::Replace);
        assert_eq!(w.files[0].edits[0].payload, "fn new()");
        assert_eq!(w.gate, "cargo check");
        assert_eq!(w.receipt, "r1");
    }

    #[test]
    fn the_constrained_grammar_and_this_parser_agree() {
        // The exact shape the sidecar's PDA emits must be the shape this
        // parser accepts — the L07 bijection across the wire, spot-checked on
        // the byte-exact weld the PDA's own test walks.
        let sidecar_shape = r#"Weld(lane:"W1",files:[F(p:"crates/x/src/lib.rs",edits:[E(anchor:"fn old()",op:"replace",payload:"fn new()")])],gate:"cargo check -p x",receipt:"read-W1.json")"#;
        let w = parse(sidecar_shape).expect("the PDA's output shape must parse");
        assert_eq!(w.files[0].edits[0].op, Op::Replace);
    }

    #[test]
    fn an_unknown_op_is_a_typed_refusal() {
        let bad = WELD.replace("replace", "rewrite");
        assert!(matches!(parse(&bad), Err(WeldErr::UnknownOp(o)) if o == "rewrite"));
    }

    #[test]
    fn a_missing_anchor_is_refused_not_guessed() {
        let e = Edit { anchor: "nowhere".into(), op: Op::Replace, payload: "x".into() };
        assert!(matches!(
            splice("fn main() {}", &e, "a.rs"),
            Err(WeldErr::AnchorMissing { .. })
        ));
    }

    #[test]
    fn an_ambiguous_anchor_is_refused_not_guessed() {
        let e = Edit { anchor: "fn ".into(), op: Op::Replace, payload: "x".into() };
        let r = splice("fn a() {}\nfn b() {}", &e, "a.rs");
        assert!(matches!(r, Err(WeldErr::AnchorAmbiguous { hits: 2, .. })));
    }

    #[test]
    fn all_four_ops_splice_exactly() {
        let buf = "alpha MARK omega";
        let mk = |op, payload: &str| Edit { anchor: "MARK".into(), op, payload: payload.into() };
        assert_eq!(splice(buf, &mk(Op::Replace, "X"), "t").unwrap(), "alpha X omega");
        assert_eq!(splice(buf, &mk(Op::Before, "X"), "t").unwrap(), "alpha XMARK omega");
        assert_eq!(splice(buf, &mk(Op::After, "X"), "t").unwrap(), "alpha MARKX omega");
        assert_eq!(splice(buf, &mk(Op::Delete, ""), "t").unwrap(), "alpha  omega");
    }

    #[test]
    fn a_red_gate_rolls_the_bytes_back() {
        let dir = std::env::temp_dir().join(format!("weld-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("lib.rs");
        std::fs::write(&f, "fn old() {}").unwrap();
        let w = Weld {
            lane: "t".into(),
            files: vec![FileEdits {
                path: "lib.rs".into(),
                edits: vec![Edit {
                    anchor: "fn old()".into(),
                    op: Op::Replace,
                    payload: "fn new()".into(),
                }],
            }],
            gate: "g".into(),
            receipt: "r".into(),
        };
        let plan = plan(&w, &dir).unwrap();

        // Red gate: the write must be undone.
        let stuck = commit_gated(&plan, &dir, || false).unwrap();
        assert!(!stuck);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "fn old() {}");

        // Green gate: the write must persist.
        let stuck = commit_gated(&plan, &dir, || true).unwrap();
        assert!(stuck);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "fn new() {}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escaped_payload_content_round_trips() {
        let src = r#"Weld(lane:"L",files:[F(p:"a.rs",edits:[E(anchor:"x",op:"replace",payload:"line1\nline2\t\"quoted\"")])],gate:"g",receipt:"r")"#;
        let w = parse(src).unwrap();
        assert_eq!(w.files[0].edits[0].payload, "line1\nline2\t\"quoted\"");
    }
}
