//! TCP client for Gemma sidecar inference on port 13017.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Errors returned by the Gemma client.
#[derive(Debug, Clone)]
pub enum GemmaError {
    /// Sidecar daemon unreachable or connection failed.
    SidecarUnreachable(String),
    /// Failed to write frame to sidecar.
    FrameWrite(String),
    /// Failed to read frame from sidecar.
    FrameRead(String),
    /// Response body is not valid UTF-8.
    InvalidUtf8(String),
    /// Sidecar returned an error response.
    InvalidResponse(String),
}

impl std::fmt::Display for GemmaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GemmaError::SidecarUnreachable(e) => write!(f, "sidecar unreachable: {}", e),
            GemmaError::FrameWrite(e) => write!(f, "frame write failed: {}", e),
            GemmaError::FrameRead(e) => write!(f, "frame read failed: {}", e),
            GemmaError::InvalidUtf8(e) => write!(f, "response not utf8: {}", e),
            GemmaError::InvalidResponse(e) => write!(f, "invalid response: {}", e),
        }
    }
}

/// TCP client for Gemma 9B/4B model inference via sidecar on port 13017.
pub struct GemmaClient {
    addr: std::net::SocketAddr,
}

impl GemmaClient {
    /// Create a new client targeting the given address.
    pub fn new(addr: std::net::SocketAddr) -> Self {
        Self { addr }
    }

    /// Create a new client pointing to the default loopback sidecar (127.0.0.1:13017).
    pub fn localhost_13017() -> Self {
        Self {
            addr: "127.0.0.1:13017"
                .parse()
                .expect("hardcoded addr parses"),
        }
    }

    /// Infer with default parameters (temp=100, top_p=95).
    pub fn infer(&self, prompt: &str) -> Result<String, GemmaError> {
        self.infer_with_temp(prompt, 100, 95)
    }

    /// Infer with custom temperature and top_p percentiles.
    pub fn infer_with_temp(
        &self,
        prompt: &str,
        temp_pmy: u32,
        top_p_pmy: u32,
    ) -> Result<String, GemmaError> {
        let mut stream = TcpStream::connect_timeout(&self.addr, Duration::from_secs(5))
            .map_err(|e| GemmaError::SidecarUnreachable(e.to_string()))?;

        stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

        let req = if temp_pmy == 100 && top_p_pmy == 95 {
            format!("INFER {}", prompt)
        } else {
            format!("INFER_T {} {} {}", temp_pmy, top_p_pmy, prompt)
        };

        write_frame(&mut stream, req.as_bytes())
            .map_err(|e| GemmaError::FrameWrite(e.to_string()))?;

        let reply = read_frame(&mut stream)
            .map_err(|e| GemmaError::FrameRead(e.to_string()))?;

        let text = String::from_utf8(reply)
            .map_err(|e| GemmaError::InvalidUtf8(e.to_string()))?;

        if let Some(err) = text.strip_prefix("ERR") {
            return Err(GemmaError::InvalidResponse(err.to_string()));
        }

        Ok(text)
    }
}

fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    if len > 1_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame size exceeds 1MB limit",
        ));
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len() as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(payload)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn gemma_client_localhost_default() {
        let client = GemmaClient::localhost_13017();
        assert_eq!(client.addr.to_string(), "127.0.0.1:13017");
    }

    #[test]
    fn frame_roundtrip_echo() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                if let Ok(data) = read_frame(&mut stream) {
                    let _ = write_frame(&mut stream, &data);
                }
            }
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        write_frame(&mut client_stream, b"test payload").unwrap();
        let reply = read_frame(&mut client_stream).unwrap();
        assert_eq!(reply, b"test payload");

        server.join().ok();
    }

    #[test]
    fn frame_refuses_oversized() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(&(2_000_000u32).to_be_bytes());
            }
        });

        if let Ok(mut stream) = TcpStream::connect(addr) {
            let result = read_frame(&mut stream);
            assert!(result.is_err());
        }
    }
}
