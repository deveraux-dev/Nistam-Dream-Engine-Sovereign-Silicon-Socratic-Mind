use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
const FRAME_MAGIC: u32 = 0x4630_5243;
const WIRE_VERSION: u8 = 1;
const KIND_CALL: u8 = 0;
const KIND_RESULT: u8 = 1;
const KIND_FAULT: u8 = 4;
const MAX_FRAME_LEN: u32 = 4 * 1024 * 1024;
const TOOL_INFER: u16 = 9;

fn write_frame(w: &mut impl Write, kind: u8, tool_id: u16, payload: &[u8]) -> std::io::Result<()> {
    let mut hdr = [0u8; 12];
    hdr[..4].copy_from_slice(&FRAME_MAGIC.to_be_bytes());
    hdr[4] = WIRE_VERSION;
    hdr[5] = kind;
    hdr[6..8].copy_from_slice(&tool_id.to_be_bytes());
    hdr[8..12].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    w.write_all(&hdr)?;
    w.write_all(payload)?;
    w.flush()
}

fn read_header(r: &mut impl Read) -> std::io::Result<Option<(u8, u32)>> {
    let mut buf = [0u8; 12];
    match r.read(&mut buf[..1])? {
        0 => return Ok(None),
        _ => {}
    }
    r.read_exact(&mut buf[1..])?;

    let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if magic != FRAME_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("bad magic {magic:#010x}"),
        ));
    }

    let len = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
    if len > MAX_FRAME_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("payload len {len} exceeds MAX {MAX_FRAME_LEN}"),
        ));
    }

    Ok(Some((buf[5], len)))
}

pub fn infer(query: &str, budget_ms: u32, timeout_ms: u64) -> Result<String, String> {
    let payload = format!("query:{}\nbudget_ms:{}", query, budget_ms);
    let addr_str = forge_daemon_door::protocol::daemon_addr();
    let addr = addr_str
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .map_err(|e| format!("daemon down {} — {}", addr_str, e))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
        .map_err(|e| e.to_string())?;

    write_frame(&mut stream, KIND_CALL, TOOL_INFER, payload.as_bytes())
        .map_err(|e| format!("write: {}", e))?;

    let (kind, len) = read_header(&mut stream)
        .map_err(|e| format!("read header: {}", e))?
        .ok_or_else(|| "daemon closed before replying".to_string())?;

    let mut reply_body = vec![0u8; len as usize];
    stream
        .read_exact(&mut reply_body)
        .map_err(|e| format!("read body: {}", e))?;

    if kind == KIND_FAULT {
        let text = String::from_utf8_lossy(&reply_body);
        return Err(format!("fault: {}", text));
    }

    if kind != KIND_RESULT {
        return Err(format!("unexpected frame kind {}", kind));
    }

    Ok(String::from_utf8_lossy(&reply_body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_payload_format() {
        let payload = format!("query:{}\nbudget_ms:{}", "hello", 100);
        assert_eq!(payload, "query:hello\nbudget_ms:100");
    }

    #[test]
    fn frame_magic_and_constants() {
        assert_eq!(FRAME_MAGIC, 0x4630_5243);
        assert_eq!(WIRE_VERSION, 1);
        assert_eq!(KIND_CALL, 0);
        assert_eq!(KIND_RESULT, 1);
        assert_eq!(KIND_FAULT, 4);
        assert_eq!(TOOL_INFER, 9);
    }

    #[test]
    fn write_frame_builds_header() {
        let mut buf = Vec::new();
        let payload = b"test";
        write_frame(&mut buf, KIND_CALL, TOOL_INFER, payload).unwrap();
        assert_eq!(buf.len(), 12 + 4);
        assert_eq!(&buf[0..4], b"F0RC");
        assert_eq!(buf[4], WIRE_VERSION);
        assert_eq!(buf[5], KIND_CALL);
        assert_eq!(&buf[6..8], &9u16.to_be_bytes());
        assert_eq!(&buf[8..12], &4u32.to_be_bytes());
    }
}
