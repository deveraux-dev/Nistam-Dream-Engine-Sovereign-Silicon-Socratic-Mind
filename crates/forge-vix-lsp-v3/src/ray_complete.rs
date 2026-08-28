//! Door-side ray completion (Sean 3x: "nothing guessing my words" / ".vixi 5D
//! spatial" / "5D Raycast and lexicon semantic intent") — the next head after
//! row164's w-tier cut: a best-effort NDJSON call to the daemon's `raycast`
//! tool over `.forge/domains/vixi.idx` (R2 distributional), REORDERING the
//! closed-set `handlers::completion()` pool by ray proximity. `forge-ml` stays
//! out of this crate's deps (Cargo.toml:23 bar) — this rides the SAME
//! `DreamCall{tool:"dream_tool_call"}` bridge `repo_query::dispatch("raycast",
//! ..)` already answers on :13013 (a free fn, no `&Brain`/live-index needed,
//! per GOLDMINER-DAEMON-DOWN-2026-07-12.md's own finding), over a raw
//! `TcpStream` mirroring `forge-gui/src/vixi_autofill.rs`'s NDJSON pattern.
//!
//! Never invents: a dead/slow door falls back to the closed-set's own order
//! (see `boost_by_ray`'s empty-hits no-op) — completion never blocks or
//! errors on the ray, it only gets worse-ordered.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{json, Value};

/// One `raycast` hit off `.forge/domains/vixi.idx` — `line`'s first tab field
/// is the vixiscript identifier (row shape confirmed live:
/// `name\tfamily role t<tier> — doc`).
#[derive(Debug, Clone, PartialEq)]
pub struct RayHit {
    pub name: String,
    pub perp_sq: i64,
}

const VIXI_IDX: &str = ".forge/domains/vixi.idx";
/// Short enough that a dead/slow door never stalls an editor completion
/// request — this is a best-effort reorder, not a required round-trip.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const READ_TIMEOUT: Duration = Duration::from_millis(800);

/// Fire the ray over the daemon's :13013 control socket. `from`/`toward` are
/// embedded 5D-box endpoints per the tool's own convention (origin/context,
/// heading/task) — here, the cursor's `attr=` scope and typed prefix.
pub fn fetch_ray_hits(from: &str, toward: &str, limit: usize) -> Result<Vec<RayHit>, String> {
    let port: u16 = std::env::var("FORGE_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(13013);
    let addr = format!("127.0.0.1:{port}");
    let sock_addr: std::net::SocketAddr =
        addr.parse().map_err(|e| format!("daemon addr parse '{addr}': {e}"))?;

    let stream = TcpStream::connect_timeout(&sock_addr, CONNECT_TIMEOUT)
        .map_err(|e| format!("daemon unreachable at {addr}: {e}"))?;
    stream.set_read_timeout(Some(READ_TIMEOUT)).map_err(|e| e.to_string())?;

    // DaemonMsg::DreamCall{tool:"dream_tool_call", params:{name,args}} — the
    // existing dream_tool_call bridge (lib.rs) relays straight into
    // repo_query::dispatch(name, args), the same free fn the door's own
    // `raycast` MCP tool answers through.
    let req = json!({
        "op": "dream_call",
        "tool": "dream_tool_call",
        "params": {
            "name": "raycast",
            "args": {
                "from": from,
                "toward": toward,
                "limit": limit,
                "idx": VIXI_IDX,
                "embedding": "distributional",
            }
        }
    });
    {
        let mut w: &TcpStream = &stream;
        w.write_all(req.to_string().as_bytes()).map_err(|e| format!("send: {e}"))?;
        w.write_all(b"\n").map_err(|e| format!("send newline: {e}"))?;
    }

    let mut reply_line = String::new();
    BufReader::new(&stream)
        .read_line(&mut reply_line)
        .map_err(|e| format!("recv reply: {e}"))?;
    let reply: Value = serde_json::from_str(&reply_line).map_err(|e| format!("reply parse: {e}"))?;
    if reply.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(format!("daemon reply not ok: {reply_line}"));
    }
    let dream_json = reply
        .get("dream_json")
        .and_then(Value::as_str)
        .ok_or_else(|| "reply missing dream_json".to_string())?;
    let inner: Value =
        serde_json::from_str(dream_json).map_err(|e| format!("dream_json parse: {e}"))?;
    if inner.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(format!("raycast tool error: {dream_json}"));
    }
    // The tool reply wraps its payload under "data" (matches the MCP door's own
    // raycast tool shape: {data:{hits,...}, ok}) — confirmed live against :13013,
    // not guessed (a first draft here read top-level `hits` and silently got
    // nothing back).
    let hits = inner
        .get("data")
        .and_then(|d| d.get("hits"))
        .and_then(Value::as_array)
        .ok_or_else(|| "raycast reply missing data.hits".to_string())?;
    Ok(hits
        .iter()
        .filter_map(|h| {
            let line = h.get("line")?.as_str()?;
            let name = line.split('\t').next()?.to_string();
            let perp_sq = h.get("perp_sq")?.as_i64()?;
            Some(RayHit { name, perp_sq })
        })
        .collect())
}

/// Pure reorder (no I/O — directly unit-testable): completion items whose
/// `label` exact-matches a ray hit's identifier move to the FRONT, in the
/// ray's own closest-first order; everything else keeps its original
/// relative order (stable sort) as the fallback band. The closed set itself
/// never changes size or gains a label the ray invented — it only reorders.
pub fn boost_by_ray(items: Vec<Value>, hits: &[RayHit]) -> Vec<Value> {
    if hits.is_empty() {
        return items;
    }
    let rank_of = |label: &str| hits.iter().position(|h| h.name == label);
    let mut items = items;
    items.sort_by_key(|item| {
        let label = item.get("label").and_then(Value::as_str).unwrap_or("");
        match rank_of(label) {
            Some(r) => (0u8, r as u32),
            None => (1u8, 0u32),
        }
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(label: &str) -> Value {
        json!({ "label": label, "kind": 14, "detail": "", "sortText": format!("1_{label}") })
    }

    #[test]
    fn boost_by_ray_is_a_noop_on_empty_hits() {
        let items = vec![item("a"), item("b")];
        let out = boost_by_ray(items.clone(), &[]);
        assert_eq!(out, items, "no hits => order unchanged");
    }

    #[test]
    fn boost_by_ray_pulls_matched_labels_to_front_in_ray_order() {
        let items = vec![item("alpha"), item("beta"), item("gamma")];
        let hits = vec![
            RayHit { name: "gamma".into(), perp_sq: 10 },
            RayHit { name: "alpha".into(), perp_sq: 40 },
        ];
        let out = boost_by_ray(items, &hits);
        let labels: Vec<&str> = out.iter().map(|v| v["label"].as_str().unwrap()).collect();
        assert_eq!(labels, vec!["gamma", "alpha", "beta"], "ray-closest first, non-hit keeps original slot last");
    }

    #[test]
    fn boost_by_ray_never_invents_or_drops_a_label() {
        let items = vec![item("x"), item("y")];
        let hits = vec![RayHit { name: "z_not_in_pool".into(), perp_sq: 1 }];
        let out = boost_by_ray(items, &hits);
        let labels: Vec<&str> = out.iter().map(|v| v["label"].as_str().unwrap()).collect();
        assert_eq!(labels.len(), 2, "closed set size must not change");
        assert!(labels.contains(&"x") && labels.contains(&"y"));
    }

    // Live wire proof, not run by default (needs `forge daemon` UP on :13013) —
    // `cargo test -p forge-vix-lsp --lib -- --ignored ray_complete::tests::live`.
    // Caught a real bug on first probe: the daemon's dream_tool_call reply
    // wraps hits under "data" (matches the MCP door's own raycast shape), not
    // top-level — a first draft here silently returned zero hits.
    #[test]
    #[ignore]
    fn live_fetch_ray_hits_hits_the_real_daemon() {
        let hits = fetch_ray_hits("vibe", "audio bind attribute", 8).expect("daemon must be reachable");
        assert!(!hits.is_empty(), "live vixi.idx ray must return hits");
        assert!(
            hits.iter().any(|h| h.name.starts_with("vibe_")),
            "expected a vibe_* attr near this ray, got {hits:?}"
        );
    }

    #[test]
    fn boost_by_ray_preserves_relative_order_within_the_unmatched_band() {
        let items = vec![item("m"), item("n"), item("o")];
        let hits = vec![RayHit { name: "o".into(), perp_sq: 1 }];
        let out = boost_by_ray(items, &hits);
        let labels: Vec<&str> = out.iter().map(|v| v["label"].as_str().unwrap()).collect();
        assert_eq!(labels, vec!["o", "m", "n"], "m before n unchanged (stable sort)");
    }
}
