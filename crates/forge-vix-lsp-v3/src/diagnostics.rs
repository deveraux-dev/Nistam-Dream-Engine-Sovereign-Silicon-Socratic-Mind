//! Custom VixiScript diagnostics engine inside the language server.

use crate::grammar;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

impl Diagnostic {
    pub fn error(line: usize, code: &'static str, message: String) -> Self {
        Self {
            line,
            severity: Severity::Error,
            code,
            message,
        }
    }
}

/// The `#vixi:<dialect> v<n>` header dialect, if the first non-empty line is a header.
pub fn header_dialect(src: &str) -> Option<String> {
    let line = src.lines().map(str::trim).find(|l| !l.is_empty())?;
    let rest = line.strip_prefix("#vixi:")?;
    rest.split_whitespace().next().map(|s| s.to_string())
}

/// Validate a `.vixi` source. Empty result = clean.
pub fn check(src: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let Some(_) = header_dialect(src) else {
        // `.vixel` (Family C, forge-ast) legitimately carries no `#vixi:` header — don't
        // false-flag it.
        if grammar::is_headerless_vixel(src) {
            return out;
        }
        out.push(Diagnostic::error(
            1,
            "no-header",
            "missing `#vixi:<dialect> v<n>` header — every .vixi must start with it".to_string(),
        ));
        return out;
    };

    out
}
