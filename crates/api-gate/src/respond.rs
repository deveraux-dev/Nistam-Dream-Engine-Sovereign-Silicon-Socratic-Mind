/// Minimal HTTP response writer: status codes with blank bodies.
use std::io::Write;
use std::net::TcpStream;

/// Write an HTTP status response with no body (blank response line + CRLF + CRLF).
pub fn write_status(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {} {}\r\n\r\n", code, reason)?;
    stream.flush()
}
