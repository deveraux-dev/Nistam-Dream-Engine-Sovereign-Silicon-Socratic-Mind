//! ForgeWire client to forgedaemon :13013 — codec from forge_daemon_door::wire,
//! call idiom from xtask/src/daemon.rs (connect, 5s timeout, key:value payload).

use forge_daemon_door::wire;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

pub const SIDECAR_ADDR: &str = "127.0.0.1:13017";

pub fn call(op: &str, payload: &str) -> Result<(bool, String), String> {
    let tool_id = wire::tool_id_of(op).ok_or_else(|| format!("unknown op `{op}`"))?;
    let addr_str = forge_daemon_door::protocol::daemon_addr();
    let addr = addr_str.parse().map_err(|e| format!("addr: {e}"))?;
    let mut stream = TcpStream::connect_timeout(
        &addr,
        Duration::from_millis(800),
    )
    .map_err(|e| format!("connect {}: {e}", addr_str))?;
    stream.set_read_timeout(Some(Duration::from_secs(20))).ok();
    wire::write_frame(&mut stream, wire::KIND_CALL, tool_id, payload.as_bytes())
        .map_err(|e| format!("write frame: {e}"))?;
    let hdr = wire::read_header(&mut stream)
        .map_err(|e| format!("read header: {e}"))?
        .ok_or_else(|| "daemon closed before replying".to_string())?;
    let mut body = vec![0u8; hdr.len as usize];
    stream.read_exact(&mut body).map_err(|e| format!("read body: {e}"))?;
    let ok = hdr.kind == wire::KIND_RESULT;
    Ok((ok, String::from_utf8_lossy(&body).into_owned()))
}

pub fn sidecar_up() -> bool {
    let addr_str = forge_daemon_door::protocol::daemon_addr();
    if let Ok(addr) = addr_str.parse() {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok() {
            return true;
        }
    }
    SIDECAR_ADDR
        .parse()
        .ok()
        .and_then(|a| TcpStream::connect_timeout(&a, Duration::from_millis(300)).ok())
        .is_some()
}

/// Pull `key:value` lines out of a reply body.
pub fn field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    body.lines().find_map(|l| l.strip_prefix(&format!("{key}:")).map(str::trim))
}
