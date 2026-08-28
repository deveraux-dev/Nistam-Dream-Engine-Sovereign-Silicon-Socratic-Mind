//! Lane WELD — `lsp_hover`'s handler. Ported from `door.rs::handle_lsp_hover`
//! (the still-live hand-written verb, tool_id 53) as the proof-of-concept
//! pair for `lsp_hover.ron`. Not yet called by live dispatch — see
//! `codegen.rs`'s module doc.

use crate::protocol::DaemonReply;

/// `forge_vix_lsp_v3::handlers::hover`, called straight — identical body to
/// `door.rs::handle_lsp_hover`, the byte-for-byte match `diff-wire` proves.
pub fn handle(line: u32, character: u32, source: &str) -> DaemonReply {
    match forge_vix_lsp_v3::handlers::hover(source, line, character) {
        Some(v) => match serde_json::to_string(&v) {
            Ok(json) => DaemonReply::with_data(json),
            Err(e) => DaemonReply::err(format!("lsp_hover: serialize: {e}")),
        },
        None => DaemonReply::ok(),
    }
}
