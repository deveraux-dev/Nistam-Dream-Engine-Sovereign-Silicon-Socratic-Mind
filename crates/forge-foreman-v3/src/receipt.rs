//! `RECEIPT(...)` RON — the flat text shape T1/L22b's compiled gate looks
//! for. Same technique as `weld.rs`'s `P` scanner (byte-level, ident/string
//! primitives) and `beat_status.rs`'s render/parse pairing — not a shared
//! struct import (both are private to their own module and shaped for a
//! different grammar), a fresh small parser for a simpler shape.
//!
//! CORRECTED 2026-08-19: "not the `ron` crate" is true ONLY of this
//! hand-rolled flavor (`weld.rs`/`beat_status.rs`/`rolls.rs`/here, all in
//! `forge-foreman-v3`) — `crates/forge-massops-wire-v3/src/weld_wire.rs`
//! genuinely uses `ron`+`serde` for its own, unrelated "massweld" protocol.
//! Two real, coexisting "RON" surfaces under one name — not the same thing,
//! never merge them.
//!
//! ```text
//! RECEIPT(claim:"...",verdict:PROVEN,roots:["F:\v3","F:\NewRepo"],anchor:"file:line")
//! ```

/// A claim's verification state — T1's `observed|inferred|[ASSUMED]` made typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Confirmed on disk this turn.
    Proven,
    /// Confirmed absent across the checked roots.
    Absent,
    /// Checked but inconclusive, or a root couldn't be checked.
    Unverified,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Proven => "PROVEN",
            Verdict::Absent => "ABSENT",
            Verdict::Unverified => "UNVERIFIED",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "PROVEN" => Some(Verdict::Proven),
            "ABSENT" => Some(Verdict::Absent),
            "UNVERIFIED" => Some(Verdict::Unverified),
            _ => None,
        }
    }
}

/// One parsed `RECEIPT(...)` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// The claim being made, verbatim.
    pub claim: String,
    /// Its verification state.
    pub verdict: Verdict,
    /// Roots checked to reach `verdict` (e.g. `F:\v3`, `F:\NewRepo`).
    pub roots: Vec<String>,
    /// `file:line` this receipt anchors to.
    pub anchor: String,
}

/// Render a receipt to its RON row.
pub fn render(r: &Receipt) -> String {
    let roots = r.roots.iter().map(|s| format!("\"{}\"", escape(s))).collect::<Vec<_>>().join(",");
    format!(
        "RECEIPT(claim:\"{}\",verdict:{},roots:[{}],anchor:\"{}\")",
        escape(&r.claim),
        r.verdict.as_str(),
        roots,
        escape(&r.anchor)
    )
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
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

    fn eat(&mut self, c: u8) -> Option<()> {
        if self.peek() == c {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    fn ident(&mut self) -> &'a str {
        self.ws();
        let a = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_') {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[a..self.i]).unwrap_or("")
    }

    /// Skip an optional `name:` field label.
    fn label(&mut self) {
        let save = self.i;
        if !self.ident().is_empty() && self.peek() == b':' {
            self.i += 1;
        } else {
            self.i = save;
        }
    }

    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let &c = self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return Some(out),
                b'\\' => {
                    let &e = self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        // Windows path segment (`\v3`, `\NewRepo`, …) written with a
                        // single backslash instead of the escaped `\\` — tolerate it
                        // rather than fail the whole receipt over one under-escaped
                        // path char (the recurring hand-authoring mistake this exists
                        // to absorb).
                        other => {
                            out.push('\\');
                            out.push(other as char);
                        }
                    }
                }
                _ => out.push(c as char),
            }
        }
    }
}

/// Parse the FIRST `RECEIPT(...)` row found anywhere in `text`, or `None` if
/// none is present or it's malformed. A malformed receipt is treated the
/// same as no receipt — never a claimed-but-broken pass.
pub fn parse(text: &str) -> Option<Receipt> {
    let at = text.find("RECEIPT(")?;
    let mut p = P { s: text.as_bytes(), i: at + "RECEIPT(".len() };

    let mut claim = None;
    let mut verdict = None;
    let mut roots = None;
    let mut anchor = None;

    loop {
        if p.peek() == b')' {
            break;
        }
        p.label();
        p.ws();
        match p.peek() {
            b'"' => {
                let v = p.string()?;
                if claim.is_none() {
                    claim = Some(v);
                } else if anchor.is_none() {
                    anchor = Some(v);
                }
            }
            b'[' => {
                p.i += 1;
                let mut list = Vec::new();
                loop {
                    if p.peek() == b']' {
                        p.i += 1;
                        break;
                    }
                    list.push(p.string()?);
                    if p.peek() == b',' {
                        p.i += 1;
                    }
                }
                roots = Some(list);
            }
            _ => {
                let ident = p.ident();
                if let Some(v) = Verdict::parse(ident) {
                    verdict = Some(v);
                } else if ident.is_empty() {
                    return None; // stuck — not a well-formed row
                }
            }
        }
        if p.peek() == b',' {
            p.i += 1;
        }
    }

    Some(Receipt {
        claim: claim.unwrap_or_default(),
        verdict: verdict?,
        roots: roots.unwrap_or_default(),
        anchor: anchor.unwrap_or_default(),
    })
}

/// Does `text` carry a `RECEIPT(...)` row, or an `[ASSUMED]`/`[INFERRED]`
/// tag — the two forms T1 accepts as a claim's proof-state marker?
pub fn has_receipt_or_tag(text: &str) -> bool {
    text.contains("RECEIPT(") || text.contains("[ASSUMED]") || text.contains("[INFERRED]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_then_parse_round_trips() {
        let r = Receipt {
            claim: "RouteExpert exists in v3".to_string(),
            verdict: Verdict::Absent,
            roots: vec!["F:\\v3".to_string(), "F:\\NewRepo".to_string()],
            anchor: "hook.rs:42".to_string(),
        };
        let rendered = render(&r);
        let parsed = parse(&rendered).expect("parses");
        assert_eq!(parsed, r);
    }

    #[test]
    fn parse_finds_receipt_amid_other_text() {
        let text = "some prose before\nRECEIPT(claim:\"X\",verdict:PROVEN,roots:[\"F:\\\\v3\"],anchor:\"a.rs:1\")\nmore after";
        let r = parse(text).expect("parses");
        assert_eq!(r.claim, "X");
        assert_eq!(r.verdict, Verdict::Proven);
    }

    #[test]
    fn under_escaped_windows_path_tolerates_not_fails() {
        // Single backslash before 'v' (not a real escape) — the recurring
        // hand-authoring mistake. Must still parse, not silently drop the row.
        let text = "RECEIPT(claim:\"X\",verdict:PROVEN,roots:[\"F:\\v3\"],anchor:\"a.rs:1\")";
        let r = parse(text).expect("tolerates under-escaped backslash");
        assert_eq!(r.roots, vec!["F:\\v3".to_string()]);
    }

    #[test]
    fn malformed_receipt_parses_to_none() {
        assert!(parse("RECEIPT(claim:\"unterminated").is_none());
        assert!(parse("no receipt here at all").is_none());
    }

    #[test]
    fn has_receipt_or_tag_covers_all_three_forms() {
        assert!(has_receipt_or_tag("RECEIPT(claim:\"x\",verdict:ABSENT,roots:[],anchor:\"a:1\")"));
        assert!(has_receipt_or_tag("[ASSUMED] this might be true"));
        assert!(has_receipt_or_tag("[INFERRED] probably"));
        assert!(!has_receipt_or_tag("bare prose claim, nothing backing it"));
    }
}
