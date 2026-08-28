//! Spanned diagnostic for table lookups — precise location, never a generic
//! "unexpected token" (the friction guard: table-generation must not flatten
//! diagnostics; every message names the vocabulary and the accepted set).

/// A diagnostic pinned to an exact source location, carrying the vocabulary
/// name and the accepted set rather than a generic "unexpected token".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedError {
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub col: usize,
    /// The full diagnostic text.
    pub message: String,
}

impl std::fmt::Display for SpannedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for SpannedError {}
