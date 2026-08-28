//! The JSON-RPC-over-stdio server loop, lib-callable (no `stdio-transport` feature
//! required) so `13forge-studio` can dispatch it as a `vix-lsp` subcommand. The
//! `stdio-transport`-gated bin (`main.rs`) is a one-line wrapper over `run_stdio`.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use crate::handlers;
use crate::telemetry::{EditorEvent, TelemetryTracker};
use crate::cognitive::{Adaptation, CognitiveCeiling, CognitiveState};
use serde_json::{json, Value};

/// Run the stdio LSP server loop to completion (until `exit` or EOF). Returns the
/// process exit code (`0`).
pub fn run_stdio() -> i32 {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut docs: HashMap<String, String> = HashMap::new();
    // Cognitive telemetry: editor events → CognitiveSignal → a live AdhdLens. The
    // client streams `forge/telemetry`; we reply `forge/cognitiveState` so the studio
    // can react (IDE adaptations + the heal synth). Detection lives in `telemetry.rs`.
    let mut tracker = TelemetryTracker::new();

    while let Some(msg) = read_message(&mut reader) {
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => respond(&mut writer, id, json!({ "capabilities": capabilities() })),
            "initialized" => {}
            "shutdown" => respond(&mut writer, id, Value::Null),
            "exit" => break,

            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (doc_uri(&params), open_text(&params)) {
                    docs.insert(uri.clone(), text.clone());
                    publish_diagnostics(&mut writer, &uri, &text);
                }
            }
            "textDocument/didChange" => {
                if let (Some(uri), Some(text)) = (doc_uri(&params), change_text(&params)) {
                    docs.insert(uri.clone(), text.clone());
                    publish_diagnostics(&mut writer, &uri, &text);
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = doc_uri(&params) {
                    docs.remove(&uri);
                }
            }

            // Cognitive telemetry intake (FORGE-COGNITIVE-IDE Phase-2). A notification
            // (no id): feed the editor event to the lens, push the new state back so the
            // studio can adapt the UI + drive the heal audio.
            "forge/telemetry" => {
                if let Some(event) = parse_editor_event(&params) {
                    let state = tracker.push(event);
                    let adaptations = tracker.adaptations();
                    publish_cognitive_state(&mut writer, &state, &adaptations);
                    // C5: also mirror the reading to the on-disk cognitive bus the studio
                    // neuro-hud reads (best-effort; a dead disk never blocks the editor).
                    let _ = tracker.publish_bus();
                }
            }

            // The rung source (Sean 07-20): the operator moves the ceiling dial —
            // {"rung":"Child|Curious|Maker|Master"}. Set, never inferred; publishes
            // straight to the bus so the glass gates flip on the next read.
            "forge/ceiling" => {
                if let Some(rung) = params.get("rung").cloned() {
                    if let Ok(c) = serde_json::from_value::<CognitiveCeiling>(rung) {
                        tracker.set_ceiling(c);
                        let _ = tracker.publish_bus();
                    }
                }
            }

            "textDocument/hover" => {
                let result = position_request(&docs, &params)
                    .and_then(|(src, l, c)| handlers::hover(src, l, c))
                    .unwrap_or(Value::Null);
                respond(&mut writer, id, result);
            }
            "textDocument/completion" => {
                let ctx = position_request(&docs, &params);
                let mut items = ctx
                    .map(|(src, l, c)| handlers::completion(src, l, c))
                    .unwrap_or_default();
                // Door-side ray completion (:13013 vixi lane, best-effort): reorder
                // the closed set by proximity on .forge/domains/vixi.idx — a dead or
                // slow door just leaves `items` in its pre-ray order (never blocks,
                // never invents; see ray_complete's own doc comment).
                if let Some((src, l, c)) = ctx {
                    let (prefix, value_attr) = handlers::cursor_context(src, l, c);
                    if !prefix.is_empty() || value_attr.is_some() {
                        let from = value_attr.clone().unwrap_or_else(|| "vixiscript kit".to_string());
                        let toward = if prefix.is_empty() { value_attr.unwrap_or_default() } else { prefix };
                        if let Ok(hits) = crate::ray_complete::fetch_ray_hits(&from, &toward, 20) {
                            items = crate::ray_complete::boost_by_ray(items, &hits);
                        }
                    }
                }
                // Cognitive adapt: under fatigue/overload the lens caps the list to
                // kill option-scan friction — but ONLY when the user has armed the
                // guidance slider. Off (default) => no cap, ever (C1 never-force gate).
                if let Some(cap) =
                    crate::telemetry::completion_cap(&tracker.adaptations(), tracker.guidance())
                {
                    items.truncate(cap);
                }
                respond(&mut writer, id, Value::Array(items));
            }
            "textDocument/documentSymbol" => {
                let result = doc_uri(&params)
                    .and_then(|u| docs.get(&u))
                    .map(|src| Value::Array(handlers::document_symbol(src)))
                    .unwrap_or_else(|| Value::Array(vec![]));
                respond(&mut writer, id, result);
            }
            "textDocument/definition" => {
                let uri = doc_uri(&params);
                let mut result = position_request(&docs, &params)
                    .and_then(|(src, l, c)| handlers::definition(src, l, c))
                    .unwrap_or(Value::Null);
                // The pure handler emits an empty `uri`; fill it with the request's.
                if let (Some(obj), Some(uri)) = (result.as_object_mut(), uri.as_ref()) {
                    obj.insert("uri".to_string(), Value::String(uri.clone()));
                }
                respond(&mut writer, id, result);
            }
            "textDocument/references" => {
                let uri = doc_uri(&params);
                let result = position_request(&docs, &params)
                    .map(|(src, l, c)| handlers::references(src, l, c))
                    .unwrap_or_default();
                let lsp_refs: Vec<Value> = result
                    .into_iter()
                    .map(|range| {
                        json!({
                            "uri": uri.clone().unwrap_or_default(),
                            "range": range,
                        })
                    })
                    .collect();
                respond(&mut writer, id, Value::Array(lsp_refs));
            }
            "textDocument/rename" => {
                let uri = doc_uri(&params);
                let new_name = params.get("newName").and_then(Value::as_str).unwrap_or("");
                let result = position_request(&docs, &params)
                    .and_then(|(src, l, c)| handlers::rename(src, l, c, new_name))
                    .map(|edits| {
                        let mut changes = HashMap::new();
                        if let Some(u) = uri {
                            changes.insert(u, Value::Array(edits));
                        }
                        json!({ "changes": changes })
                    })
                    .unwrap_or(Value::Null);
                respond(&mut writer, id, result);
            }
            "textDocument/documentHighlight" => {
                let result = position_request(&docs, &params)
                    .map(|(src, l, c)| handlers::document_highlight(src, l, c))
                    .unwrap_or_default();
                respond(&mut writer, id, Value::Array(result));
            }

            // Unknown request → null result (notifications are ignored).
            _ => {
                if id.is_some() {
                    respond(&mut writer, id, Value::Null);
                }
            }
        }
    }
    0
}

fn capabilities() -> Value {
    json!({
        "textDocumentSync": 1, // Full
        "hoverProvider": true,
        "documentSymbolProvider": true,
        "definitionProvider": true,
        "referencesProvider": true,
        "renameProvider": true,
        "documentHighlightProvider": true,
        "completionProvider": { "triggerCharacters": ["=", " "] }
    })
}

// ── message framing ──────────────────────────────────────────────────────────

fn read_message<R: BufRead>(r: &mut R) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None; // EOF
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some(v) = trimmed.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn respond<W: Write>(w: &mut W, id: Option<Value>, result: Value) {
    let resp = json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result });
    write_framed(w, &resp);
}

fn publish_diagnostics<W: Write>(w: &mut W, uri: &str, text: &str) {
    let note = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": handlers::diagnostics(text) }
    });
    write_framed(w, &note);
}

fn publish_cognitive_state<W: Write>(w: &mut W, state: &CognitiveState, adaptations: &[Adaptation]) {
    let note = json!({
        "jsonrpc": "2.0",
        "method": "forge/cognitiveState",
        "params": {
            "state": serde_json::to_value(state).unwrap_or(Value::Null),
            "adaptations": serde_json::to_value(adaptations).unwrap_or(Value::Null),
        }
    });
    write_framed(w, &note);

    // Cross-process bus: persist for the neuro-hud + heal synth to READ
    // (telemetry -> AdhdLens -> bus -> studio). `guidance:"Off"` = the reader
    // never forces past the user's slider (Sean 2026-07-18). Best-effort, never
    // blocks the LSP wire.
    let bus = json!({
        "state": serde_json::to_value(state).unwrap_or(Value::Null),
        "adaptation": adaptations.first().and_then(|a| serde_json::to_value(a).ok()).unwrap_or(Value::Null),
        "guidance": "Off",
    });
    if let Ok(s) = serde_json::to_string(&bus) {
        let _ = std::fs::create_dir_all(".forge");
        let _ = std::fs::write(".forge/cognitive.json", s);
    }
}

/// Parse one `forge/telemetry` notification's params into an `EditorEvent`.
fn parse_editor_event(params: &Value) -> Option<EditorEvent> {
    match params.get("event")?.as_str()? {
        "keystroke" => Some(EditorEvent::Keystroke),
        "fileSwitch" => Some(EditorEvent::FileSwitch),
        "astOk" => Some(EditorEvent::AstOk),
        "diagnosticsCleared" => Some(EditorEvent::DiagnosticsCleared),
        "minute" => Some(EditorEvent::Minute),
        "astError" => {
            let ms = params.get("sinceLastRunMs").and_then(Value::as_u64).unwrap_or(0) as u32;
            Some(EditorEvent::AstError { since_last_run_ms: ms })
        }
        _ => None,
    }
}

fn write_framed<W: Write>(w: &mut W, msg: &Value) {
    let body = serde_json::to_vec(msg).unwrap_or_default();
    let _ = write!(w, "Content-Length: {}\r\n\r\n", body.len());
    let _ = w.write_all(&body);
    let _ = w.flush();
}

// ── params extraction ────────────────────────────────────────────────────────

fn doc_uri(params: &Value) -> Option<String> {
    params
        .get("textDocument")?
        .get("uri")?
        .as_str()
        .map(str::to_string)
}

fn open_text(params: &Value) -> Option<String> {
    params
        .get("textDocument")?
        .get("text")?
        .as_str()
        .map(str::to_string)
}

/// `didChange` with `textDocumentSync = Full`: the last content change holds the
/// whole document text.
fn change_text(params: &Value) -> Option<String> {
    params
        .get("contentChanges")?
        .as_array()?
        .last()?
        .get("text")?
        .as_str()
        .map(str::to_string)
}

/// Resolve a `(uri, position)` request against the document store, returning
/// `(text, line, character)`. `None` if the document isn't open.
fn position_request<'a>(
    docs: &'a HashMap<String, String>,
    params: &Value,
) -> Option<(&'a str, u32, u32)> {
    let uri = doc_uri(params)?;
    let src = docs.get(&uri)?;
    let pos = params.get("position")?;
    let line = pos.get("line")?.as_u64()? as u32;
    let character = pos.get("character")?.as_u64()? as u32;
    Some((src.as_str(), line, character))
}
