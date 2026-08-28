//! Two-lane door-verb doctrine (2026-08-22): Lane RON is a declarative spec
//! (`src/verbs/*.ron` + the append-only `src/verbs/ORDER.tsv` ledger) that
//! drives codegen instead of hand-written `wire.rs`/`protocol.rs`/`door.rs`
//! edits; Lane WELD is the per-verb `src/verbs/<name>.rs` handler body, the
//! only genuinely new-judgment piece. This module is the one home (L05) for
//! both the spec parser and the shared runtime encode/decode helpers the
//! generated code calls — `xtask`'s `gen-verbs`/`check-lanes`/`weld`/
//! `diff-wire` subcommands only drive it, never duplicate its logic.
//!
//! Scope, stated plainly: this landed as the proof-of-concept pass. Verbs
//! 1-54 (everything hand-written before and during 2026-08-21) stay in
//! `wire.rs`/`protocol.rs`/`door.rs`, untouched, not migrated. The generated
//! namespace (`EXT_TOOL_TABLE`/`GeneratedMsg`, ids continuing from 55) is not
//! yet spliced into the live `TOOL_TABLE`/`DaemonMsg`/dispatch — that splice
//! is the natural next step the first time a real NEW verb (one with no
//! existing hand-written twin) needs to go live; this pass proves the
//! generator is wire-identical to hand-written code first (`diff-wire`),
//! deliberately not wiring untested codegen straight into production traffic.

use std::path::Path;

use serde::Deserialize;

/// One typed key's value kind in a `Keyed`/`KeyedTail` payload.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum KeyType {
    /// `u32`, parsed with `.parse().unwrap_or(0)` (same lossy-default idiom
    /// every hand-written decode arm this session already uses).
    U32,
    /// Plain string, taken verbatim.
    String,
}

/// The four payload shapes every door verb this session actually needed —
/// confirmed against the 13 verbs hand-written 2026-08-21/22, not invented.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum PayloadKind {
    /// No payload (`ping`, `hook_drift`).
    Empty,
    /// N typed `key:value` lines, order-fixed, nothing else (`river_set_head`,
    /// `asp_solve`).
    Keyed {
        /// Key name -> value type, in wire order.
        keys: Vec<(String, KeyType)>,
    },
    /// The whole payload is one raw string field (`cst_check`,
    /// `lsp_diagnostics`).
    Verbatim,
    /// N typed `key:value` lines, then a verbatim tail field named `source`
    /// (`ast_parse`, `lsp_hover`) — the same `splitn(N+1, '\n')` idiom every
    /// hand-written verb of this shape already uses.
    KeyedTail {
        /// Key name -> value type, in wire order, before the tail.
        keys: Vec<(String, KeyType)>,
    },
}

/// One verb's Lane RON spec — `src/verbs/<name>.ron`. No numeric id lives
/// here on purpose (see module doc: id collision is exactly what putting one
/// here would invite); id assignment is `ORDER.tsv`'s append order.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct VerbSpec {
    /// Wire op name — must match the `.ron` file's own stem.
    pub name: String,
    /// Payload shape.
    pub payload: PayloadKind,
    /// Whether the handler mutates anything (documentation only at this
    /// layer — the door's own `Whitelist` is still the enforced gate, this
    /// field exists so `check-lanes` can flag an obviously-wrong claim later
    /// without yet wiring an enforcement path for it).
    pub mutating: bool,
}

/// One `check-lanes` violation, named — never a bare bool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneViolation(pub String);

impl std::fmt::Display for LaneViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Every `src/verbs/*.ron` spec, parsed. `Err` names the file and the parse
/// failure — never silently skips a malformed spec.
pub fn load_specs(verbs_dir: &Path) -> Result<Vec<VerbSpec>, String> {
    let mut specs = Vec::new();
    let entries = std::fs::read_dir(verbs_dir).map_err(|e| format!("read_dir {}: {e}", verbs_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let spec: VerbSpec = ron::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        specs.push(spec);
    }
    specs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(specs)
}

/// `src/verbs/ORDER.tsv` — one verb name per line, `#`-comments and blank
/// lines skipped, append-only. Id = position in this list + `legacy_len`.
pub fn load_order(verbs_dir: &Path) -> Result<Vec<String>, String> {
    let path = verbs_dir.join("ORDER.tsv");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new()); // no ledger yet = no generated verbs yet, not an error
    };
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

/// Pre-flight validation, run before `generate`. Every named violation is
/// returned (never just the first) so an agent sees the whole picture in one
/// pass. Empty = clean.
pub fn check_lanes(verbs_dir: &Path) -> Result<(), Vec<LaneViolation>> {
    let mut violations = Vec::new();

    let specs = match load_specs(verbs_dir) {
        Ok(s) => s,
        Err(e) => return Err(vec![LaneViolation(e)]),
    };
    let order = match load_order(verbs_dir) {
        Ok(o) => o,
        Err(e) => return Err(vec![LaneViolation(e)]),
    };

    let spec_names: std::collections::BTreeSet<&str> = specs.iter().map(|s| s.name.as_str()).collect();

    // Duplicate ORDER.tsv lines: two agents appending the same name.
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for name in &order {
        *seen.entry(name.as_str()).or_insert(0) += 1;
    }
    for (name, count) in &seen {
        if *count > 1 {
            violations.push(LaneViolation(format!(
                "ORDER.tsv lists '{name}' {count} times — duplicate append, id collision"
            )));
        }
    }

    // Every spec must be registered in ORDER.tsv exactly once.
    for name in &spec_names {
        let count = order.iter().filter(|o| o.as_str() == *name).count();
        if count == 0 {
            violations.push(LaneViolation(format!(
                "{name}.ron exists but is not registered in ORDER.tsv — run `cargo xtask weld {name}` after appending it"
            )));
        }
    }

    // Every ORDER.tsv entry must have a matching .ron file.
    let order_set: std::collections::BTreeSet<&str> = order.iter().map(String::as_str).collect();
    for name in &order_set {
        if !spec_names.contains(name) {
            violations.push(LaneViolation(format!(
                "ORDER.tsv lists '{name}' but {name}.ron does not exist"
            )));
        }
    }

    // Malformed payload: a Keyed/KeyedTail spec with zero keys is a spec
    // that should have been Empty/Verbatim instead — named, not silently let through.
    for spec in &specs {
        let empty_keys = matches!(
            &spec.payload,
            PayloadKind::Keyed { keys } | PayloadKind::KeyedTail { keys } if keys.is_empty()
        );
        if empty_keys {
            violations.push(LaneViolation(format!(
                "{}.ron: Keyed/KeyedTail with zero keys — use Empty or Verbatim instead",
                spec.name
            )));
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

// ── Shared runtime encode/decode — called by BOTH generated code and
// `diff-wire`'s comparison path, so there is exactly one implementation to
// drift, never a codegen'd copy of hand-derived logic. ─────────────────────

/// `Keyed`: `key1:val1\nkey2:val2\n...`, no trailing content.
pub fn encode_keyed(pairs: &[(&str, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join("\n")
}

/// `Keyed` decode: values in `key_names` order via per-line `split_once(':')`
/// (order-independent — matches `river_set_head`/`asp_solve`'s hand-written idiom).
pub fn decode_keyed(text: &str, key_names: &[&str]) -> Vec<String> {
    let mut out = vec![String::new(); key_names.len()];
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if let Some(i) = key_names.iter().position(|n| *n == k) {
                out[i] = v.to_string();
            }
        }
    }
    out
}

/// `Verbatim`: the payload is the field, unchanged either direction.
pub fn encode_verbatim(source: &str) -> String {
    source.to_string()
}

/// `Verbatim` decode — identical to encode (both are pass-through), named
/// separately so generated call sites read as decode, not a copy-paste of
/// the encode arm.
pub fn decode_verbatim(text: &str) -> String {
    text.to_string()
}

/// `KeyedTail`: `key1:val1\nkey2:val2\n<tail, verbatim, may embed anything>`.
pub fn encode_keyed_tail(pairs: &[(&str, String)], tail: &str) -> String {
    let mut s = pairs.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join("\n");
    s.push('\n');
    s.push_str(tail);
    s
}

/// `KeyedTail` decode: order-fixed (matches `ast_parse`/`lsp_hover`'s
/// hand-written `splitn(N+1, '\n')` idiom exactly) — the Nth line MUST be the
/// Nth key or that key comes back empty, same lossy-default contract as the
/// hand-written verbs.
pub fn decode_keyed_tail(text: &str, key_names: &[&str]) -> (Vec<String>, String) {
    let mut parts = text.splitn(key_names.len() + 1, '\n');
    let mut vals = Vec::with_capacity(key_names.len());
    for name in key_names {
        let line = parts.next().unwrap_or("");
        vals.push(line.strip_prefix(&format!("{name}:")).unwrap_or("").to_string());
    }
    (vals, parts.next().unwrap_or("").to_string())
}

/// `cargo xtask weld <verb_name>` — the exact `pub fn handle(...)` signature
/// `gen-verbs`'s dispatch arm will call, body a placeholder. A welder's job
/// becomes 100% the body; it cannot get the signature wrong because it never
/// writes one.
pub fn weld_stub(spec: &VerbSpec) -> String {
    let params = match &spec.payload {
        PayloadKind::Empty => String::new(),
        PayloadKind::Keyed { keys } => keys.iter().map(|(k, t)| format!("{k}: {}", rust_type(*t))).collect::<Vec<_>>().join(", "),
        PayloadKind::Verbatim => "source: &str".to_string(),
        PayloadKind::KeyedTail { keys } => {
            let mut parts: Vec<String> = keys.iter().map(|(k, t)| format!("{k}: {}", rust_type(*t))).collect();
            parts.push("source: &str".to_string());
            parts.join(", ")
        }
    };
    format!(
        "//! Lane WELD — `{name}`'s handler. Auto-stubbed by `cargo xtask weld {name}`;\n\
         //! fill in the body only, never the signature (it's generated from\n\
         //! `{name}.ron` and must match `generated.rs`'s dispatch arm exactly).\n\n\
         use crate::protocol::DaemonReply;\n\n\
         pub fn handle({params}) -> DaemonReply {{\n    \
         todo!(\"wire this to the real function this verb wraps\")\n}}\n",
        name = spec.name,
    )
}

fn rust_type(t: KeyType) -> &'static str {
    match t {
        KeyType::U32 => "u32",
        KeyType::String => "&str",
    }
}

/// `cargo xtask gen-verbs` — the generated `src/verbs/generated.rs` text.
/// `legacy_len` is `wire::TOOL_TABLE.len()` (the hand-written 1-54 range);
/// generated ids continue from there, in `ORDER.tsv`'s append order.
pub fn generate(verbs_dir: &Path, legacy_len: usize) -> Result<String, String> {
    let specs = load_specs(verbs_dir)?;
    let order = load_order(verbs_dir)?;
    let by_name: std::collections::HashMap<&str, &VerbSpec> = specs.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut out = String::new();
    out.push_str("//! AUTO-GENERATED by `cargo xtask gen-verbs` — do not hand-edit.\n");
    out.push_str("//! Source: `src/verbs/*.ron` + `src/verbs/ORDER.tsv`. Re-run the\n");
    out.push_str("//! generator after editing either; a stale file here is a defect.\n\n");

    out.push_str("/// Extension table — ids continue from the legacy `wire::TOOL_TABLE`.\n");
    out.push_str("pub const EXT_TOOL_TABLE: &[&str] = &[\n");
    for name in &order {
        out.push_str(&format!("    \"{name}\",\n"));
    }
    out.push_str("];\n\n");

    out.push_str(&format!("/// First id in the generated range (legacy table has {legacy_len} entries).\n"));
    out.push_str(&format!("pub const EXT_ID_BASE: u16 = {};\n\n", legacy_len + 1));

    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str("#[allow(missing_docs)]\n");
    out.push_str("pub enum GeneratedMsg {\n");
    for name in &order {
        let Some(spec) = by_name.get(name.as_str()) else { continue };
        let variant = to_pascal_case(name);
        match &spec.payload {
            PayloadKind::Empty => out.push_str(&format!("    {variant},\n")),
            PayloadKind::Keyed { keys } => {
                let fields = keys.iter().map(|(k, t)| format!("{k}: {}", owned_type(*t))).collect::<Vec<_>>().join(", ");
                out.push_str(&format!("    {variant} {{ {fields} }},\n"));
            }
            PayloadKind::Verbatim => out.push_str(&format!("    {variant} {{ source: String }},\n")),
            PayloadKind::KeyedTail { keys } => {
                let mut fields: Vec<String> = keys.iter().map(|(k, t)| format!("{k}: {}", owned_type(*t))).collect();
                fields.push("source: String".to_string());
                out.push_str(&format!("    {variant} {{ {} }},\n", fields.join(", ")));
            }
        }
    }
    out.push_str("}\n\n");

    out.push_str("impl GeneratedMsg {\n");
    out.push_str("    /// Encode to wire text via the shared `codegen::encode_*` helpers.\n");
    out.push_str("    pub fn encode(&self) -> String {\n        match self {\n");
    for name in &order {
        let Some(spec) = by_name.get(name.as_str()) else { continue };
        let variant = to_pascal_case(name);
        match &spec.payload {
            PayloadKind::Empty => out.push_str(&format!("            GeneratedMsg::{variant} => String::new(),\n")),
            PayloadKind::Keyed { keys } => {
                let pat = keys.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(", ");
                let pairs = keys.iter().map(|(k, _)| format!("(\"{k}\", {k}.to_string())")).collect::<Vec<_>>().join(", ");
                out.push_str(&format!(
                    "            GeneratedMsg::{variant} {{ {pat} }} => crate::codegen::encode_keyed(&[{pairs}]),\n"
                ));
            }
            PayloadKind::Verbatim => out.push_str(&format!(
                "            GeneratedMsg::{variant} {{ source }} => crate::codegen::encode_verbatim(source),\n"
            )),
            PayloadKind::KeyedTail { keys } => {
                let mut pat: Vec<&str> = keys.iter().map(|(k, _)| k.as_str()).collect();
                pat.push("source");
                let pairs = keys.iter().map(|(k, _)| format!("(\"{k}\", {k}.to_string())")).collect::<Vec<_>>().join(", ");
                out.push_str(&format!(
                    "            GeneratedMsg::{variant} {{ {} }} => crate::codegen::encode_keyed_tail(&[{pairs}], source),\n",
                    pat.join(", ")
                ));
            }
        }
    }
    out.push_str("        }\n    }\n\n");

    out.push_str("    /// Decode by op name via the shared `codegen::decode_*` helpers.\n");
    out.push_str("    pub fn decode(op: &str, text: &str) -> Option<Self> {\n        match op {\n");
    for name in &order {
        let Some(spec) = by_name.get(name.as_str()) else { continue };
        let variant = to_pascal_case(name);
        match &spec.payload {
            PayloadKind::Empty => out.push_str(&format!("            \"{name}\" => Some(GeneratedMsg::{variant}),\n")),
            PayloadKind::Keyed { keys } => {
                let key_names = keys.iter().map(|(k, _)| format!("\"{k}\"")).collect::<Vec<_>>().join(", ");
                out.push_str(&format!("            \"{name}\" => {{\n"));
                out.push_str(&format!("                let v = crate::codegen::decode_keyed(text, &[{key_names}]);\n"));
                out.push_str(&format!("                Some(GeneratedMsg::{variant} {{ {} }})\n", keyed_field_init(keys)));
                out.push_str("            }\n");
            }
            PayloadKind::Verbatim => out.push_str(&format!(
                "            \"{name}\" => Some(GeneratedMsg::{variant} {{ source: crate::codegen::decode_verbatim(text) }}),\n"
            )),
            PayloadKind::KeyedTail { keys } => {
                let key_names = keys.iter().map(|(k, _)| format!("\"{k}\"")).collect::<Vec<_>>().join(", ");
                out.push_str(&format!("            \"{name}\" => {{\n"));
                out.push_str(&format!(
                    "                let (v, tail) = crate::codegen::decode_keyed_tail(text, &[{key_names}]);\n"
                ));
                out.push_str(&format!(
                    "                Some(GeneratedMsg::{variant} {{ {} source: tail }})\n",
                    keyed_field_init(keys)
                ));
                out.push_str("            }\n");
            }
        }
    }
    out.push_str("            _ => None,\n        }\n    }\n");
    out.push_str("}\n");

    Ok(out)
}

fn keyed_field_init(keys: &[(String, KeyType)]) -> String {
    keys.iter()
        .enumerate()
        .map(|(i, (k, t))| match t {
            KeyType::U32 => format!("{k}: v[{i}].parse().unwrap_or(0), "),
            KeyType::String => format!("{k}: v[{i}].clone(), "),
        })
        .collect()
}

fn owned_type(t: KeyType) -> &'static str {
    match t {
        KeyType::U32 => "u32",
        KeyType::String => "String",
    }
}

fn to_pascal_case(snake: &str) -> String {
    snake.split('_').map(|part| {
        let mut c = part.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_tail_roundtrip_matches_hand_written_idiom() {
        let encoded = encode_keyed_tail(&[("line", "3".to_string()), ("character", "12".to_string())], "hello\nworld");
        assert_eq!(encoded, "line:3\ncharacter:12\nhello\nworld");
        let (vals, tail) = decode_keyed_tail(&encoded, &["line", "character"]);
        assert_eq!(vals, vec!["3".to_string(), "12".to_string()]);
        assert_eq!(tail, "hello\nworld");
    }

    #[test]
    fn keyed_roundtrip_is_order_independent_on_decode() {
        let encoded = encode_keyed(&[("domain", "gems".to_string()), ("sieve_upper_bound", "10000".to_string())]);
        assert_eq!(encoded, "domain:gems\nsieve_upper_bound:10000");
        let vals = decode_keyed(&encoded, &["sieve_upper_bound", "domain"]);
        assert_eq!(vals, vec!["10000".to_string(), "gems".to_string()]);
    }

    #[test]
    fn to_pascal_case_matches_daemon_msg_variant_naming() {
        assert_eq!(to_pascal_case("lsp_hover"), "LspHover");
        assert_eq!(to_pascal_case("hook_drift"), "HookDrift");
    }
}
