//! The foreman's slice of `.forge/v3-directives.ron`, read fail-closed.
//!
//! Hand-rolled key scanning, same shape as the sidecar's reader
//! (`sidecar/src/directives.rs`) — no serde, no RON crate. Every key the
//! foreman consumes is required; a missing key is an error, never a default
//! (NOTEBOOK-BRIEF rule, applied tree-wide).

use std::path::{Path, PathBuf};

/// The pipeline keys the foreman runs on. All caller-facing mechanism values
/// live here; the loop itself holds no numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directives {
    /// `pipeline.gemma_endpoint` with its scheme stripped — a dialable
    /// `host:port`, loopback-enforced by [`crate::client`].
    pub endpoint: String,
    /// `nde.nde_endpoint` with its scheme stripped — a dialable `host:port`
    /// for the NDE inference sidecar, defaulting to `127.0.0.1:13018` if the
    /// key is absent (WAVE-WELD-PROPOSAL: optionality allows gradual wiring).
    pub nde_endpoint: String,
    /// `pipeline.gate` verbatim, `{crate}` still unsubstituted.
    pub gate: String,
    /// The retry budget parsed out of `pipeline.on_red`'s
    /// `retry_generator(max: N)` clause.
    pub retry_max: u32,
    /// `foreman.reply_timeout_s` — the sidecar reply window in seconds
    /// (WAVE-WELD: promoted from a client const after two live window-blows).
    pub reply_timeout_s: u64,
    /// `foreman.sip_cap_bytes` — the weld-lane prompt ceiling; over-cap is a
    /// refusal BEFORE any INFER is sent (the seam-sip law).
    pub sip_cap_bytes: usize,
    /// `foreman.sip_anchor_context_lines` — lines kept around each error site
    /// when slicing.
    pub sip_anchor_context_lines: usize,
    /// `foreman.retry_temp_pmy` — sampling temperature (permyriad) for run-lane
    /// RETRY attempts. Attempt 1 is always greedy; greedy retries were measured
    /// byte-identical across differing retry prompts (forge-intent-v3,
    /// 2026-08-10) — the same rut the weld lane already broke with seeded temp.
    pub retry_temp_pmy: u32,
    /// `foreman.retry_top_p_pmy` — nucleus mass (permyriad) for run-lane retries.
    pub retry_top_p_pmy: u32,
    /// `flywheel.weld_pairs_path` — the W5 journal, root-relative. Every weld
    /// ladder attempt lands here as a distill pair.
    pub weld_pairs_path: String,
    /// `flywheel.bqr_path` — the baked BQ router, root-relative. Produced by
    /// `foreman distill`, consumed by `foreman route`.
    pub bqr_path: String,
    /// `flywheel.journal_enabled` — the W5 kill switch. `false` REFUSES the
    /// weld lane loudly; it never lets the ladder grind dark.
    pub journal_enabled: bool,
}

/// Read and parse the directives that sit under `<root>/.forge/v3-directives.ron`.
pub fn load(root: &Path) -> Result<Directives, String> {
    let path: PathBuf = root.join(".forge").join("v3-directives.ron");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse(&content)
}

/// Parse the required keys and optional nde_endpoint out of the directives text.
pub fn parse(content: &str) -> Result<Directives, String> {
    let mut endpoint = None;
    let mut nde_endpoint = None;
    let mut gate = None;
    let mut on_red = None;
    let mut reply_timeout_s = None;
    let mut sip_cap_bytes = None;
    let mut sip_anchor_context_lines = None;
    let mut weld_pairs_path = None;
    let mut journal_enabled = None;
    let mut retry_temp_pmy = None;
    let mut retry_top_p_pmy = None;
    let mut bqr_path = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("//") || line.is_empty() {
            continue;
        }
        if line.starts_with("gemma_endpoint:") {
            endpoint = string_value(line);
        }
        if line.starts_with("nde_endpoint:") {
            nde_endpoint = string_value(line);
        }
        // `gate:` must not match `prove-gate` prose or `on_red:`; anchor on the
        // exact key at line start.
        if line.starts_with("gate:") {
            gate = string_value(line);
        }
        if line.starts_with("on_red:") {
            on_red = string_value(line);
        }
        if line.starts_with("reply_timeout_s:") {
            reply_timeout_s = number_value(line);
        }
        if line.starts_with("sip_cap_bytes:") {
            sip_cap_bytes = number_value(line);
        }
        if line.starts_with("sip_anchor_context_lines:") {
            sip_anchor_context_lines = number_value(line);
        }
        if line.starts_with("retry_temp_pmy:") {
            retry_temp_pmy = number_value(line);
        }
        if line.starts_with("retry_top_p_pmy:") {
            retry_top_p_pmy = number_value(line);
        }
        if line.starts_with("weld_pairs_path:") {
            weld_pairs_path = string_value(line);
        }
        if line.starts_with("bqr_path:") {
            bqr_path = string_value(line);
        }
        if line.starts_with("journal_enabled:") {
            journal_enabled = bool_value(line);
        }
    }

    let endpoint = endpoint.ok_or("missing pipeline.gemma_endpoint in directives")?;
    let endpoint = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("tcp://"))
        .unwrap_or(&endpoint)
        .trim_end_matches('/')
        .to_string();
    let nde_endpoint = nde_endpoint
        .unwrap_or_else(|| "http://127.0.0.1:13018".to_string());
    let nde_endpoint = nde_endpoint
        .strip_prefix("http://")
        .or_else(|| nde_endpoint.strip_prefix("tcp://"))
        .unwrap_or(&nde_endpoint)
        .trim_end_matches('/')
        .to_string();
    let gate = gate.ok_or("missing pipeline.gate in directives")?;
    let on_red = on_red.ok_or("missing pipeline.on_red in directives")?;
    let retry_max = retry_max_of(&on_red)
        .ok_or_else(|| format!("pipeline.on_red carries no `max: N` clause: {on_red:?}"))?;
    let reply_timeout_s =
        reply_timeout_s.ok_or("missing foreman.reply_timeout_s in directives")? as u64;
    let sip_cap_bytes =
        sip_cap_bytes.ok_or("missing foreman.sip_cap_bytes in directives")? as usize;
    let sip_anchor_context_lines = sip_anchor_context_lines
        .ok_or("missing foreman.sip_anchor_context_lines in directives")?
        as usize;
    let weld_pairs_path =
        weld_pairs_path.ok_or("missing flywheel.weld_pairs_path in directives")?;
    let bqr_path =
        bqr_path.ok_or("missing flywheel.bqr_path in directives")?;
    let journal_enabled =
        journal_enabled.ok_or("missing flywheel.journal_enabled in directives")?;
    let retry_temp_pmy =
        retry_temp_pmy.ok_or("missing foreman.retry_temp_pmy in directives")?;
    let retry_top_p_pmy =
        retry_top_p_pmy.ok_or("missing foreman.retry_top_p_pmy in directives")?;

    Ok(Directives {
        endpoint,
        nde_endpoint,
        gate,
        retry_max,
        reply_timeout_s,
        sip_cap_bytes,
        sip_anchor_context_lines,
        retry_temp_pmy,
        retry_top_p_pmy,
        weld_pairs_path,
        bqr_path,
        journal_enabled,
    })
}

/// `true`/`false` after the colon on the line; anything else is no value —
/// a typo'd bool must read as a missing key, never as a default.
fn bool_value(line: &str) -> Option<bool> {
    let after = line[line.find(':')? + 1..].trim_start();
    let word: String = after.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    match word.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// First integer after the colon on the line.
fn number_value(line: &str) -> Option<u32> {
    let after = &line[line.find(':')? + 1..];
    let digits: String = after.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Pull `N` out of `retry_generator(max: N)`. The rest of the `on_red` string
/// is policy the loop implements directly (retries, then the brief queue).
fn retry_max_of(on_red: &str) -> Option<u32> {
    let at = on_red.find("max:")?;
    let digits: String =
        on_red[at + 4..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// First double-quoted value on the line.
fn string_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
    pipeline: (
        gemma_endpoint:       "http://127.0.0.1:13017",
        gate:      "cargo test -p {crate}",
        on_red:    "retry_generator(max: 3) || escalate_to_claude",
    ),
    nde: (
        nde_endpoint:                "http://127.0.0.1:13018",
    ),
    foreman: (
        reply_timeout_s:          600,
        sip_cap_bytes:            2048,
        sip_anchor_context_lines: 8,
        retry_temp_pmy:           1500,
        retry_top_p_pmy:          9000,
    ),
    flywheel: (
        weld_pairs_path: ".forge/distill/weld-pairs.ndjson",
        bqr_path: ".forge/distill/router.bqr",
        journal_enabled: true,
    ),
    "#;

    #[test]
    fn the_three_pipeline_keys_parse() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.endpoint, "127.0.0.1:13017", "scheme stripped, dialable");
        assert_eq!(d.nde_endpoint, "127.0.0.1:13018", "nde endpoint parsed, scheme stripped");
        assert_eq!(d.gate, "cargo test -p {crate}");
        assert_eq!(d.retry_max, 3);
    }

    #[test]
    fn the_weld_lane_keys_parse_and_are_required() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.reply_timeout_s, 600);
        assert_eq!(d.sip_cap_bytes, 2048);
        assert_eq!(d.sip_anchor_context_lines, 8);
        // Dropping any one of the three is an error, never a default.
        let no_timeout = SAMPLE.replace("reply_timeout_s:          600,", "");
        assert!(parse(&no_timeout).is_err());
        let no_cap = SAMPLE.replace("sip_cap_bytes:            2048,", "");
        assert!(parse(&no_cap).is_err());
    }

    /// The run-lane retry knobs parse and are required — a missing knob is a
    /// missing key, never a silent greedy fallback.
    #[test]
    fn the_retry_sampling_keys_parse_and_are_required() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.retry_temp_pmy, 1500);
        assert_eq!(d.retry_top_p_pmy, 9000);
        let no_temp = SAMPLE.replace("retry_temp_pmy:           1500,", "");
        assert!(parse(&no_temp).is_err());
        let no_p = SAMPLE.replace("retry_top_p_pmy:          9000,", "");
        assert!(parse(&no_p).is_err());
    }

    /// W5's keys are required and typed: a journal without a home or a
    /// half-stated kill switch is a missing key, never a default.
    #[test]
    fn the_flywheel_keys_parse_and_are_required() {
        let d = parse(SAMPLE).unwrap();
        assert_eq!(d.weld_pairs_path, ".forge/distill/weld-pairs.ndjson");
        assert!(d.journal_enabled);
        let no_path = SAMPLE.replace("weld_pairs_path: \".forge/distill/weld-pairs.ndjson\",", "");
        assert!(parse(&no_path).is_err());
        let no_switch = SAMPLE.replace("journal_enabled: true,", "");
        assert!(parse(&no_switch).is_err());
        let typo = SAMPLE.replace("journal_enabled: true,", "journal_enabled: yes,");
        assert!(parse(&typo).is_err(), "a typo'd bool is missing, not defaulted");
        let off = SAMPLE.replace("journal_enabled: true,", "journal_enabled: false,");
        assert!(!parse(&off).unwrap().journal_enabled);
    }

    #[test]
    fn nde_endpoint_defaults_to_127_0_0_1_13018_when_absent() {
        let sample_no_nde = SAMPLE.replace(
            "nde: (\n        nde_endpoint:                \"http://127.0.0.1:13018\",\n    ),\n    ",
            ""
        );
        let d = parse(&sample_no_nde).unwrap();
        assert_eq!(d.nde_endpoint, "127.0.0.1:13018", "nde_endpoint defaults to standard port");
    }

    #[test]
    fn a_missing_key_is_an_error_never_a_default() {
        assert!(parse("pipeline: ( gate: \"cargo test\" )").is_err());
        assert!(parse("gemma_endpoint: \"http://127.0.0.1:1\"").is_err());
        let no_max = "gemma_endpoint: \"h\"\ngate: \"g\"\non_red: \"pray\"";
        assert!(parse(no_max).is_err(), "an on_red without a bound is not a policy");
    }

    /// The live directives file must satisfy the foreman's reader — this test
    /// pins the two files together so a key rename in one is red in the other.
    /// The root is found by walking up from the manifest dir, because this
    /// test also runs inside `cargo xtask sabotage`'s scratch copy, which
    /// carries no `.forge/` — the walk exits `target/` and finds the real one.
    #[test]
    fn the_real_directives_file_parses() {
        let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        while !root.join(".forge").join("v3-directives.ron").exists() {
            assert!(root.pop(), "no .forge/v3-directives.ron above {}", env!("CARGO_MANIFEST_DIR"));
        }
        let d = load(&root).expect(".forge/v3-directives.ron must carry the pipeline keys");
        assert!(d.endpoint.starts_with("127.0.0.1:"), "loopback endpoint, got {}", d.endpoint);
        assert!(d.gate.contains("{crate}"), "gate must be per-crate");
    }
}
