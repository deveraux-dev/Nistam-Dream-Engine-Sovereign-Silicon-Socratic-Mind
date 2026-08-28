//! The oracle's deterministic stage-4 judge — the weaver-arbiter port.
//!
//! Ported 2026-08-16 from `TODO/weaver-arbiter-roundup/weaver_arbiter.rs.quarry`
//! (v2 `sf-wasm/src/weaver_arbiter.rs`): the SHAPE is the drain — `violation()`
//! returns the FIRST rule broken (`Option<&'static str>`, shape before enum
//! before content), `rejection()` wraps it in the exact `CRITICAL REJECTION`
//! header the generator's self-correction loop obeys. The v2 entity laws
//! (powercurve, hermetic principles) stay in v2 with their deps; the laws here
//! are the roundup reply schema's own.
//!
//! BOUNDARY (v2 doctrine, kept verbatim): the generator is out-of-process and
//! may be non-deterministic; the judge is not, and the judge is the half that
//! ships. Remote responseSchema enforcement is one oracle; this judge is the
//! second. Never trust remote conformance alone.

use serde_json::Value;

/// The six surfaces a roundup verdict may claim as home.
pub const SURFACES: [&str; 6] = ["TODO", "ASPIRE", "PIN", "QUARRY", "GRIND", "CENSUS"];

/// The three verdicts the schema admits.
pub const VERDICTS: [&str; 3] = ["DRAIN-NOW", "LATER", "NOISE"];

/// Every key the roundup reply schema requires
/// (`.forge/oracle-workloads/todo-roundup.schema.json`).
pub const REQUIRED_KEYS: [&str; 4] = ["file", "surface", "card", "verdict"];

/// Judge one oracle reply. `None` means it may enter the ledger.
///
/// Order is deliberate (v2 law): shape before enum before content, so the rule
/// handed back is the FIRST thing wrong rather than the last thing checked.
pub fn violation(reply: &Value) -> Option<&'static str> {
    let obj = match reply.as_object() {
        Some(o) => o,
        None => return Some("reply is not a JSON object"),
    };

    for key in REQUIRED_KEYS {
        if !obj.contains_key(key) {
            return Some("dropped a key the schema requires");
        }
    }

    if !obj
        .get("surface")
        .and_then(Value::as_str)
        .map(|s| SURFACES.contains(&s))
        .unwrap_or(false)
    {
        return Some("named a surface outside TODO/ASPIRE/PIN/QUARRY/GRIND/CENSUS");
    }

    if !obj
        .get("verdict")
        .and_then(Value::as_str)
        .map(|v| VERDICTS.contains(&v))
        .unwrap_or(false)
    {
        return Some("invented a verdict outside DRAIN-NOW/LATER/NOISE");
    }

    if obj.get("file").and_then(Value::as_str).unwrap_or("").is_empty() {
        return Some("left the file field empty");
    }

    if obj.get("card").and_then(Value::as_str).unwrap_or("").trim().len() < 20 {
        return Some("card too thin to be a capability card");
    }

    None
}

/// The `CRITICAL REJECTION` block the generator's own protocol obeys. `None`
/// when the reply passed and there is nothing to send back.
///
/// This is the whole symbiosis (v2 doc, kept): a non-deterministic generator,
/// a deterministic judge, and one string carrying the verdict between them.
pub fn rejection(reply: &Value) -> Option<String> {
    let rule = violation(reply)?;
    Some(format!(
        "CRITICAL REJECTION\n\
         Error Log: {rule}.\n\
         Prioritize this log over your creative intent. Re-emit ONLY the corrected \
         JSON object per the declared schema."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn good() -> Value {
        json!({
            "file": "F:\\v3\\TODO\\weaver-arbiter-roundup\\MANIFEST.md",
            "surface": "TODO",
            "card": "Provenance manifest for the weaver/arbiter quarry copies; names the fold targets.",
            "verdict": "DRAIN-NOW",
            "justification": "quarry originals confirmed at F:\\NewRepo"
        })
    }

    #[test]
    fn a_lawful_reply_passes() {
        assert_eq!(violation(&good()), None);
        assert!(rejection(&good()).is_none(), "nothing to send back when it obeyed");
    }

    #[test]
    fn a_missing_schema_key_is_caught() {
        let mut r = good();
        r.as_object_mut().unwrap().remove("card");
        assert_eq!(violation(&r), Some("dropped a key the schema requires"));
    }

    #[test]
    fn an_invented_surface_is_caught() {
        let mut r = good();
        r["surface"] = json!("VIBES");
        assert_eq!(violation(&r), Some("named a surface outside TODO/ASPIRE/PIN/QUARRY/GRIND/CENSUS"));
        for s in SURFACES {
            let mut ok = good();
            ok["surface"] = json!(s);
            assert_eq!(violation(&ok), None, "{s} is one of the six");
        }
    }

    #[test]
    fn an_invented_verdict_is_caught() {
        let mut r = good();
        r["verdict"] = json!("MAYBE");
        assert_eq!(violation(&r), Some("invented a verdict outside DRAIN-NOW/LATER/NOISE"));
    }

    #[test]
    fn a_non_object_reply_is_caught() {
        assert_eq!(violation(&json!(["not", "an", "object"])), Some("reply is not a JSON object"));
    }

    #[test]
    fn a_thin_card_is_caught() {
        let mut r = good();
        r["card"] = json!("ok");
        assert_eq!(violation(&r), Some("card too thin to be a capability card"));
    }

    /// The seam the protocol was written against: the verdict comes back in the
    /// exact header the generator is told to obey (v2 test, kept).
    #[test]
    fn the_rejection_speaks_the_generators_own_protocol() {
        let mut r = good();
        r["verdict"] = json!("MAYBE");
        let log = rejection(&r).expect("a broken reply owes a rejection");
        assert!(log.starts_with("CRITICAL REJECTION"), "{log}");
        assert!(log.contains("Error Log:"), "{log}");
    }
}
