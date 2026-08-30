//! Ported verbatim from v2 `forge-evidence` (append-only chained JSONL evidence ledger, Ed25519 receipts, SHA-256 chain binding), 2026-08-15.
pub mod provenance;
pub mod nistam;
pub mod aggregate;
pub mod asset_type; // unified asset-type taxonomy (ex forge-asset-types, 2026-07-04)
pub mod stoppath; // QAQC stop-path gate receipts (RULED brief 2026-07-05 §2)

pub use aggregate::{aggregate_claim_bytes, compile_aggregate, verify_members, AggregateMember, AggregateOutcome};
pub use asset_type::AssetType;
pub use stoppath::{append_gate_to_chain, bundle_asset_audit, seal_gate_verdict, sign_gate, verify_gate, write_cart_manifest, GateKind, GateReceipt, GateReceiptBody, GateVerdict};
// One key-type home for gate/receipt callers — no per-crate dalek version skew.
pub use ed25519_dalek::{SigningKey, VerifyingKey};

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Evidence entry record.
pub struct EvidenceEntry {
    /// Tool.
    pub tool: String,
    /// Action.
    pub action: String,
    /// Detail.
    pub detail: String,
    /// Prev hash.
    pub prev_hash: String,
    /// Content hash.
    pub content_hash: String,
    /// Timestamp utc.
    pub timestamp_utc: String,
}

/// SHA-256 of raw bytes — THE one hashing home for gate/receipt callers, so
/// asset hashes are computed one way everywhere (no per-crate sha2 copies).
pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

/// Canonical JSON: sorted keys, compact separators, no NaN/Infinity.
/// Matches Python `json.dumps(sort_keys=True, separators=(',',':'), ensure_ascii=False, allow_nan=False)`
pub fn canonical_json(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    reject_nan(value)?;
    let sorted = sort_value(value);
    let s = compact_json(&sorted);
    Ok(s.into_bytes())
}

fn reject_nan(v: &serde_json::Value) -> Result<(), String> {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.is_nan() || f.is_infinite() {
                    return Err("NaN/Infinity not allowed in evidence payload".into());
                }
            }
            Ok(())
        }
        serde_json::Value::Array(arr) => { for item in arr { reject_nan(item)?; } Ok(()) }
        serde_json::Value::Object(map) => { for (_, val) in map { reject_nan(val)?; } Ok(()) }
        _ => Ok(()),
    }
}

fn sort_value(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(k, _)| k.to_string());
            serde_json::Value::Object(sorted.into_iter().map(|(k, v)| (k.clone(), sort_value(v))).collect())
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

fn compact_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        serde_json::Value::Number(n) => format!("{}", n),
        serde_json::Value::String(s) => serde_json::to_string(s).unwrap(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(compact_json).collect();
            format!("[{}]", items.join(","))
        }
        serde_json::Value::Object(map) => {
            let pairs: Vec<String> = map.iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), compact_json(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
    }
}

/// Evidence chain record.
pub struct EvidenceChain {
    path: PathBuf,
    last_hash: String,
    /// Entry count.
    pub entry_count: u64,
}

impl EvidenceChain {
    /// New.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), last_hash: "0".repeat(64), entry_count: 0 }
    }

    /// Load.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if !path.exists() { return Ok(Self::new(path)); }
        let file = fs::File::open(&path).map_err(|e| e.to_string())?;
        let mut last_hash = "0".repeat(64);
        let mut count = 0u64;
        let mut prev_expected = last_hash.clone();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() { continue; }
            let entry: EvidenceEntry = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if entry.prev_hash != prev_expected {
                return Err(format!("Chain broken at entry {}: expected prev_hash {} got {}", count, prev_expected, entry.prev_hash));
            }
            prev_expected = entry.content_hash.clone();
            last_hash = entry.content_hash.clone();
            count += 1;
        }
        Ok(Self { path, last_hash, entry_count: count })
    }

    /// Append.
    pub fn append(&mut self, tool: &str, action: &str, detail: &str) -> Result<EvidenceEntry, String> {
        let hash_payload = serde_json::json!({
            "tool": tool,
            "action": action,
            "detail": detail,
            "prev_hash": self.last_hash,
        });
        let canonical = canonical_json(&hash_payload)?;
        let content_hash = format!("{:x}", Sha256::digest(&canonical));
        let timestamp_utc = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let entry = EvidenceEntry {
            tool: tool.into(), action: action.into(), detail: detail.into(),
            prev_hash: self.last_hash.clone(), content_hash: content_hash.clone(), timestamp_utc,
        };

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| e.to_string())?;
        let json_line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        writeln!(file, "{}", json_line).map_err(|e| e.to_string())?;

        self.last_hash = content_hash;
        self.entry_count += 1;
        Ok(entry)
    }

    /// Verify.
    pub fn verify(&self) -> Result<bool, String> {
        if !self.path.exists() { return Ok(true); }
        let file = fs::File::open(&self.path).map_err(|e| e.to_string())?;
        let mut prev = "0".repeat(64);
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() { continue; }
            let entry: EvidenceEntry = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if entry.prev_hash != prev {
                return Err(format!("Chain broken at entry {}", i));
            }
            // Recompute hash
            let hash_payload = serde_json::json!({
                "tool": entry.tool,
                "action": entry.action,
                "detail": entry.detail,
                "prev_hash": entry.prev_hash,
            });
            let canonical = canonical_json(&hash_payload)?;
            let expected = format!("{:x}", Sha256::digest(&canonical));
            if entry.content_hash != expected {
                return Err(format!("Hash mismatch at entry {}: expected {} got {}", i, expected, entry.content_hash));
            }
            prev = entry.content_hash;
        }
        Ok(true)
    }

    /// Entries.
    pub fn entries(&self) -> Result<Vec<EvidenceEntry>, String> {
        if !self.path.exists() { return Ok(vec![]); }
        let file = fs::File::open(&self.path).map_err(|e| e.to_string())?;
        BufReader::new(file).lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(true))
            .map(|l| {
                let line = l.map_err(|e| e.to_string())?;
                serde_json::from_str(&line).map_err(|e| e.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    fn tmp_path() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("evidence_test_{}_{}.jsonl", std::process::id(), n));
        let _ = fs::remove_file(&p);
        p
    }

    #[test]
    fn append_3_entries_verify_chain() {
        let p = tmp_path();
        let mut chain = EvidenceChain::new(&p);
        let e1 = chain.append("wright", "inspect", "checked module A").unwrap();
        let e2 = chain.append("sieve", "filter", "removed dead code").unwrap();
        let e3 = chain.append("dream", "generate", "created handler").unwrap();
        assert_eq!(e1.prev_hash, "0".repeat(64));
        assert_eq!(e2.prev_hash, e1.content_hash);
        assert_eq!(e3.prev_hash, e2.content_hash);
        assert!(chain.verify().unwrap());
        fs::remove_file(&p).ok();
    }

    #[test]
    fn load_and_verify_existing() {
        let p = tmp_path();
        {
            let mut chain = EvidenceChain::new(&p);
            chain.append("tool1", "act1", "det1").unwrap();
            chain.append("tool2", "act2", "det2").unwrap();
        }
        let loaded = EvidenceChain::load(&p).unwrap();
        assert_eq!(loaded.entry_count, 2);
        assert!(loaded.verify().unwrap());
        fs::remove_file(&p).ok();
    }

    #[test]
    fn nan_causes_error() {
        // serde_json::Value can't represent NaN directly, so we test via
        // a raw f64 that we manually inject
        let mut map = serde_json::Map::new();
        map.insert("x".into(), serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()));
        let val = serde_json::Value::Object(map);
        // Normal float works
        assert!(canonical_json(&val).is_ok());
        // NaN can't be constructed via serde_json::Number::from_f64 (returns None)
        assert!(serde_json::Number::from_f64(f64::NAN).is_none());
        assert!(serde_json::Number::from_f64(f64::INFINITY).is_none());
    }

    #[test]
    fn canonical_json_matches_python() {
        // Python: json.dumps({"z":"hello","a":[1,2,3],"m":true}, sort_keys=True, separators=(',',':'))
        // Output: {"a":[1,2,3],"m":true,"z":"hello"}
        let val = serde_json::json!({"z": "hello", "a": [1,2,3], "m": true});
        let out = String::from_utf8(canonical_json(&val).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":[1,2,3],"m":true,"z":"hello"}"#);
    }

    #[test]
    fn tampered_file_fails_verify() {
        let p = tmp_path();
        let mut chain = EvidenceChain::new(&p);
        chain.append("tool", "act", "detail").unwrap();
        chain.append("tool2", "act2", "detail2").unwrap();
        let content = fs::read_to_string(&p).unwrap();
        let tampered = content.replace("detail2", "TAMPERED");
        fs::write(&p, tampered).unwrap();
        let chain2 = EvidenceChain::new(&p);
        assert!(chain2.verify().is_err());
        fs::remove_file(&p).ok();
    }

    #[test]
    fn entries_returns_all() {
        let p = tmp_path();
        let mut chain = EvidenceChain::new(&p);
        chain.append("a", "b", "c").unwrap();
        chain.append("d", "e", "f").unwrap();
        let entries = chain.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tool, "a");
        assert_eq!(entries[1].tool, "d");
        fs::remove_file(&p).ok();
    }
}
