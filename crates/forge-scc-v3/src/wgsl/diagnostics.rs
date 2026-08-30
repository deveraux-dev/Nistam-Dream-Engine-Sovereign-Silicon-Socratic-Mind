//! Stable, code-carrying compile errors with optional source spans, so a failure
//! always traces back to the input.

/// A compile failure: a stable machine-readable code plus a human message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// Stable, machine-readable error code (e.g. `E_PARSE_SCAFFOLD`).
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Where in the source this error traces back to, if known.
    pub span: Option<SourceSpan>,
}

/// A byte range into the original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// Start offset, inclusive.
    pub start: usize,
    /// End offset, exclusive.
    pub end: usize,
}

impl CompileError {
    /// Construct an error with no source span.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            span: None,
        }
    }
}
