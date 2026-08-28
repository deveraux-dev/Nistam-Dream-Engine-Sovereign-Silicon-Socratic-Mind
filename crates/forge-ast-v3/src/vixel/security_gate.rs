//! # security_gate.rs — Standalone Security Gate for VixiScript Source
//!
//! Rejects float literals and f32/f64 keywords at the source level,
//! independent of the full AST pipeline. Designed for `build.rs`
//! integration to abort Cargo builds on detection.
//!
//! **Security checks:**
//! - Float literal pattern: `[0-9]+\.[0-9]+`
//! - Forbidden type keywords: `f32`, `f64` as standalone tokens
//!
//! **Guarantees:**
//! - Never panics — always returns `Ok(())` or `Err(SecurityDeny)`
//! - Deterministic: same input always produces the same result
//! - No external dependencies — pure byte-level scanning

use std::fmt;

/// Error returned when the security gate rejects source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityDeny {
    /// Human-readable reason the source was rejected.
    pub reason: String, // alloc-ok: build-time only, never on hot path
}

impl fmt::Display for SecurityDeny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecurityDeny: {}", self.reason)
    }
}

impl std::error::Error for SecurityDeny {}

/// Scan VixiScript source for float literals and f32/f64 keywords.
///
/// Returns `Ok(())` if the source is clean (no float contamination).
/// Returns `Err(SecurityDeny)` if a float literal or forbidden keyword
/// is detected, with a descriptive reason string.
///
/// # Integration
///
/// In `build.rs` of the AOT Vixel Compiler pipeline:
/// ```ignore
/// if let Err(deny) = forge_ast::vixel::security_gate::gate_security(&source) {
///     panic!("Security gate DENY: {}", deny.reason);
/// }
/// ```
pub fn gate_security(source: &str) -> Result<(), SecurityDeny> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    // Scan for float literal pattern: digit(s) followed by '.' followed by digit(s)
    let mut i = 0;
    while i < len {
        if bytes[i].is_ascii_digit() {
            // Walk past consecutive digits
            let start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Check for '.' followed by digit
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                return Err(SecurityDeny {
                    reason: format!( // alloc-ok: build-time error path only
                        "float literal detected at byte offset {}",
                        start,
                    ),
                });
            }
        } else {
            i += 1;
        }
    }

    // Scan for f32/f64 keywords as standalone tokens
    for keyword in &["f32", "f64"] {
        let kw_bytes = keyword.as_bytes();
        let kw_len = kw_bytes.len();
        if len < kw_len {
            continue;
        }
        let mut j = 0;
        while j + kw_len <= len {
            if &bytes[j..j + kw_len] == kw_bytes {
                let before_ok = j == 0 || !is_ident_char(bytes[j - 1]);
                let after_ok = j + kw_len == len || !is_ident_char(bytes[j + kw_len]);
                if before_ok && after_ok {
                    return Err(SecurityDeny {
                        reason: format!( // alloc-ok: build-time error path only
                            "forbidden keyword '{}' at byte offset {}",
                            keyword, j,
                        ),
                    });
                }
            }
            j += 1;
        }
    }

    Ok(())
}

/// Returns true if `b` is an ASCII alphanumeric or underscore (identifier char).
#[inline]
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Float literal detection ────────────────────────────────────────

    #[test]
    fn deny_float_literal() {
        let result = gate_security("let x = 3.14;");
        assert!(result.is_err());
        let deny = result.unwrap_err();
        assert!(deny.reason.contains("float literal"), "reason: {}", deny.reason);
    }

    #[test]
    fn deny_float_in_expression() {
        let result = gate_security("result = value * 0.5 + offset");
        assert!(result.is_err());
    }

    #[test]
    fn allow_integer_with_dot_access() {
        // "obj.field" should NOT trigger (no digit after dot)
        let result = gate_security("let x = mat.mass + 100");
        assert!(result.is_ok());
    }

    #[test]
    fn allow_pure_integer() {
        let result = gate_security("let x = 42;");
        assert!(result.is_ok());
    }

    // ── f32/f64 keyword detection ──────────────────────────────────────

    #[test]
    fn deny_f32_keyword() {
        let result = gate_security("var speed: f32 = 10;");
        assert!(result.is_err());
        let deny = result.unwrap_err();
        assert!(deny.reason.contains("f32"), "reason: {}", deny.reason);
    }

    #[test]
    fn deny_f64_keyword() {
        let result = gate_security("let value: f64 = 0;");
        assert!(result.is_err());
        let deny = result.unwrap_err();
        assert!(deny.reason.contains("f64"), "reason: {}", deny.reason);
    }

    #[test]
    fn allow_f32_as_substring() {
        // "af32b" contains "f32" but not as a standalone token
        let result = gate_security("let af32b = 10;");
        assert!(result.is_ok());
    }

    #[test]
    fn allow_f320_not_standalone() {
        let result = gate_security("let f320 = 10;");
        assert!(result.is_ok());
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn allow_empty_string() {
        let result = gate_security("");
        assert!(result.is_ok());
    }

    #[test]
    fn allow_single_char() {
        let result = gate_security("x");
        assert!(result.is_ok());
    }

    #[test]
    fn allow_whitespace_only() {
        let result = gate_security("   \n\t  ");
        assert!(result.is_ok());
    }

    #[test]
    fn deny_reason_is_descriptive() {
        let result = gate_security("let x = 1.5");
        let deny = result.unwrap_err();
        assert!(deny.reason.contains("float literal"));
        assert!(deny.reason.contains("byte offset"));
    }

    #[test]
    fn display_impl_works() {
        let deny = SecurityDeny {
            reason: "test reason".into(), // alloc-ok: test only
        };
        let s = deny.to_string(); // alloc-ok: test only
        assert!(s.contains("SecurityDeny"));
        assert!(s.contains("test reason"));
    }
}
