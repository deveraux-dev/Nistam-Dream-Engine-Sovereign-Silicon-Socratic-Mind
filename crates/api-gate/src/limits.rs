/// Request body size enforcement: read up to cap, hard-reject oversized.
use std::io::{Read, ErrorKind};
use std::net::TcpStream;

/// Errors from request reading: oversized, malformed, or I/O failure.
#[derive(Debug)]
pub enum RequestError {
    /// Body exceeds 4KB cap.
    TooLarge,
    /// I/O or other read failure.
    Io(std::io::Error),
}

impl From<std::io::Error> for RequestError {
    fn from(e: std::io::Error) -> Self {
        RequestError::Io(e)
    }
}

/// Read HTTP request (line + headers + body) capped at `cap` bytes total.
/// Returns `Err(TooLarge)` the moment cumulative bytes exceed cap; otherwise the request body.
pub fn read_capped(stream: &mut TcpStream, cap: usize) -> Result<Vec<u8>, RequestError> {
    let mut buf = [0u8; 512];
    let mut total = Vec::new();
    let mut bytes_read = 0;

    loop {
        let n = stream.read(&mut buf).map_err(|e| {
            if e.kind() == ErrorKind::WouldBlock {
                RequestError::Io(std::io::Error::new(ErrorKind::TimedOut, "read timeout"))
            } else {
                RequestError::Io(e)
            }
        })?;

        if n == 0 {
            break;
        }

        bytes_read += n;
        if bytes_read > cap {
            return Err(RequestError::TooLarge);
        }

        total.extend_from_slice(&buf[..n]);

        // Simple heuristic: if we see \r\n\r\n (end of headers), we've read enough to know
        // there's a body coming. For a real HTTP parser this needs proper Content-Length handling.
        // For now, if we detect double-CRLF and have some content, assume request is complete.
        if total.len() > 4 && total.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    if bytes_read > cap {
        Err(RequestError::TooLarge)
    } else {
        Ok(total)
    }
}

/// Minimal JSON shape validation: balanced braces/quotes, non-empty.
/// Returns true if body looks like valid JSON (not a detailed schema check).
pub fn json_shape_ok(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }

    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for &b in body {
        if escape_next {
            escape_next = false;
            continue;
        }

        match b {
            b'\\' if in_string => escape_next = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => brace_depth += 1,
            b'}' if !in_string => brace_depth = brace_depth.saturating_sub(1),
            b'[' if !in_string => bracket_depth += 1,
            b']' if !in_string => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
    }

    // Valid JSON shape: all braces/brackets balanced, no unterminated string.
    brace_depth == 0 && bracket_depth == 0 && !in_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_shape_ok_detects_balanced_braces() {
        assert!(json_shape_ok(b"{}"));
        assert!(json_shape_ok(b"{\"key\": \"value\"}"));
        assert!(json_shape_ok(b"[1, 2, 3]"));
    }

    #[test]
    fn json_shape_ok_rejects_unbalanced_braces() {
        assert!(!json_shape_ok(b"{"));
        assert!(!json_shape_ok(b"{}]"));
        assert!(!json_shape_ok(b"{\"key\": \"val\""));
    }

    #[test]
    fn json_shape_ok_rejects_unterminated_string() {
        assert!(!json_shape_ok(b"{\"key\": \"unterminated"));
    }

    #[test]
    fn json_shape_ok_rejects_empty() {
        assert!(!json_shape_ok(b""));
    }
}
