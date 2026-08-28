//! `foreman oracle` — the governed Gemini lane: free tier orients, paid tier
//! only inside a DECLARED workload, every attempt ledgered, caps fail-closed.
//!
//! Port receipt (2026-08-16, plan `composed-brewing-hanrahan.md`, Sean: "plan,
//! set a limit, stay within it"): transport is v2's proven CLI delegation —
//! `F:\NewRepo\crates\forge-daemon\src\oracle.rs` shells the auth-holding
//! `gemini.cmd` with the brief on stdin; this process holds NO cloud
//! credentials and adds NO http dependency. Doctrine is v2's
//! `forge-book/src/oracle1_governor.rs` ("gemini flash orients free, welders
//! mutate paid"), enforced here mechanically: the free tier never consults a
//! workload; the paid tier REFUSES without an unexpired
//! `.forge/oracle-workloads/<id>.ron` declaration carrying its own caps.
//!
//! Token columns are ESTIMATES (bytes/4 of brief and reply) — the CLI does not
//! reliably report real counts; an estimate labeled est_* is honest, a parsed
//! guess presented as exact would not be (L12: proof is typed, not toned).
//! Refusals are typed, logged rows, never silent: ORACLE_CLI_NOT_FOUND,
//! ORACLE_OVER_CAP, ORACLE_WORKLOAD_NOT_DECLARED, ORACLE_WORKLOAD_MALFORMED,
//! ORACLE_AUTH_FAILED.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge_ml_bqrouter::{embed_prompt, BqRouter};
use forge_vcs_v3::{spine::BrutalHash, BrutalHashExt};

/// Oracle lane configuration, read from `.forge/v3-directives.ron` by flat
/// line-prefix scan (same idiom directives.rs uses for `gemma_endpoint:`).
/// Keys are oracle_-prefixed so they can never collide with another section.
/// Fail-closed: every key required, no defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleConfig {
    /// Launcher that holds the Google auth (e.g. "gemini.cmd"). Never argv-passed keys.
    pub cli_path: String,
    /// Free-tier model id (orientation work).
    pub model_free: String,
    /// Paid-tier model id (declared workloads only).
    pub model_paid: String,
    /// Free-tier calls allowed per UTC day.
    pub free_calls_day: u64,
    /// Free-tier estimated tokens allowed per UTC day.
    pub free_tokens_day: u64,
}

/// A declared paid workload — `.forge/oracle-workloads/<id>.ron`. Absent or
/// expired = the paid step is refused. The declaration IS the "plan, set a
/// limit" half; this module is the "stay within it" half.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadDecl {
    /// Workload id (file stem must match).
    pub id: String,
    /// Calls this workload may spend, total.
    pub cap_calls: u64,
    /// Estimated tokens this workload may spend, total.
    pub cap_tokens: u64,
    /// Expiry as days-since-epoch; a call on or after this day is refused.
    pub expires_epoch_day: i64,
    /// Optional JSON-schema file (root-relative or absolute). When declared,
    /// the shim forwards it as responseSchema (remote enforcement) and the
    /// reply must parse as JSON here or the call is SCHEMA_REFUSED (stage 3
    /// light; the deterministic arbiter is a later weld).
    pub schema: Option<String>,
}

/// First value for `key:` in flat RON-ish text — exact prefix match after
/// trim, no regex (forbidden_ops). Strips trailing comma and wrapping quotes.
pub fn ron_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix(key) else { continue };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(':') else { continue };
        let v = rest.trim().trim_end_matches(',').trim();
        let v = v.strip_prefix('"').unwrap_or(v);
        let v = v.strip_suffix('"').unwrap_or(v);
        return Some(v.to_string());
    }
    None
}

fn ron_u64(text: &str, key: &str) -> Result<u64, String> {
    ron_value(text, key)
        .ok_or_else(|| format!("missing {key} in oracle config (fail-closed, no default)"))?
        .parse()
        .map_err(|e| format!("{key}: not a u64: {e}"))
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's days_from_civil,
/// integer-only, valid across the Gregorian range this repo will ever see).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Parse "YYYY-MM-DD" to days-since-epoch. Fail-closed: malformed = Err.
pub fn parse_date_days(s: &str) -> Result<i64, String> {
    let mut it = s.split('-');
    let y: i64 = it.next().and_then(|p| p.parse().ok()).ok_or("bad year")?;
    let m: i64 = it.next().and_then(|p| p.parse().ok()).ok_or("bad month")?;
    let d: i64 = it.next().and_then(|p| p.parse().ok()).ok_or("bad day")?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(format!("date out of range: {s}"));
    }
    Ok(days_from_civil(y, m, d))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Today as days-since-epoch, UTC.
pub fn today_epoch_day() -> i64 {
    (now_secs() / 86_400) as i64
}

/// bytes/4 token estimate — labeled est_ everywhere it lands.
pub fn est_tokens(bytes: usize) -> u64 {
    (bytes as u64) / 4
}

/// The Oracle-B collision brief, drained 2026-08-16 from v2
/// `forge-daemon/src/oracle.rs::build_brief` — same header, same `\n---\n`
/// separator, so v2-era collide transcripts stay comparable.
pub fn collide_brief(toward: &str, skeleton: &str) -> String {
    format!("ORACLE_B collide toward: {toward}\n---\n{skeleton}")
}

/// Structural-question markers, drained verbatim 2026-08-16 from v2
/// `forge-book/src/oracle1_governor.rs:45` — chosen there so a per-file
/// GREEN/ABSENT sweep never trips them.
pub const STRUCTURAL_MARKERS: &[&str] = &[
    "caller", "callers", "consumer", "consumers", "reachab", "wired", "wire",
    "dispatch", "architect", "cross-file", "call graph", "call-graph", "trait",
    "impl of", "who calls", "orphan", "downstream", "upstream", "invariant",
    "contract", "lifecycle", "ownership", "seam", "isomorph", "surface",
];

/// Does this brief ask something one file cannot answer? Case-insensitive
/// substring match. Pure — a tested seam (v2 doctrine: "gemini flash orients
/// free, welders mutate paid" — a structural ask on the free tier is
/// misrouted, and the gate below makes that mechanical).
pub fn question_is_structural(rules: &str) -> bool {
    let r = rules.to_ascii_lowercase();
    STRUCTURAL_MARKERS.iter().any(|m| r.contains(m))
}

fn config_path(root: &Path) -> PathBuf {
    root.join(".forge").join("v3-directives.ron")
}

/// Load the oracle config section. Every key required (fail-closed).
pub fn load_config(root: &Path) -> Result<OracleConfig, String> {
    let p = config_path(root);
    let text = std::fs::read_to_string(&p)
        .map_err(|e| format!("cannot read {}: {e}", p.display()))?;
    Ok(OracleConfig {
        cli_path: ron_value(&text, "oracle_cli_path")
            .ok_or("missing oracle_cli_path in v3-directives.ron (fail-closed)")?,
        model_free: ron_value(&text, "oracle_model_free")
            .ok_or("missing oracle_model_free in v3-directives.ron (fail-closed)")?,
        model_paid: ron_value(&text, "oracle_model_paid")
            .ok_or("missing oracle_model_paid in v3-directives.ron (fail-closed)")?,
        free_calls_day: ron_u64(&text, "oracle_free_calls_day")?,
        free_tokens_day: ron_u64(&text, "oracle_free_tokens_day")?,
    })
}

fn workloads_dir(root: &Path) -> PathBuf {
    root.join(".forge").join("oracle-workloads")
}

/// Load and validate a workload declaration. ORACLE_WORKLOAD_NOT_DECLARED if
/// the file is absent; ORACLE_WORKLOAD_MALFORMED on any parse failure.
pub fn load_workload(root: &Path, id: &str) -> Result<WorkloadDecl, String> {
    let p = workloads_dir(root).join(format!("{id}.ron"));
    let text = std::fs::read_to_string(&p).map_err(|_| {
        format!(
            "ORACLE_WORKLOAD_NOT_DECLARED: paid tier requires {} — declare it first: foreman oracle --root <root> --declare {id}",
            p.display()
        )
    })?;
    let parse = |k: &str| {
        ron_value(&text, k).ok_or(format!("ORACLE_WORKLOAD_MALFORMED: {id}.ron missing {k}"))
    };
    let cap_calls: u64 = parse("cap_calls")?
        .parse()
        .map_err(|_| format!("ORACLE_WORKLOAD_MALFORMED: {id}.ron cap_calls not a u64"))?;
    let cap_tokens: u64 = parse("cap_tokens")?
        .parse()
        .map_err(|_| format!("ORACLE_WORKLOAD_MALFORMED: {id}.ron cap_tokens not a u64"))?;
    let expires_epoch_day = parse_date_days(&parse("expires")?)
        .map_err(|e| format!("ORACLE_WORKLOAD_MALFORMED: {id}.ron expires: {e}"))?;
    // Optional schema key — absent is fine (prose reply), declared-but-missing
    // file is malformed (fail-closed, same posture as every other key).
    let schema = match ron_value(&text, "schema") {
        Some(s) => {
            let sp = if Path::new(&s).is_absolute() { PathBuf::from(&s) } else { root.join(&s) };
            if !sp.exists() {
                return Err(format!(
                    "ORACLE_WORKLOAD_MALFORMED: {id}.ron declares schema but file not found: {}",
                    sp.display()
                ));
            }
            Some(sp.to_string_lossy().into_owned())
        }
        None => None,
    };
    Ok(WorkloadDecl { id: id.to_string(), cap_calls, cap_tokens, expires_epoch_day, schema })
}

fn ledger_path(root: &Path) -> PathBuf {
    root.join(".forge").join("oracle-ledger.tsv")
}

const LEDGER_HEADER: &str = "day\tsecs\tworkload\ttier\tcalls\test_in\test_out\tstatus\tbrief_hash\tsoul\tbq_specialist\tbq_margin";

/// Sum (calls, est tokens) of OK rows matching `tier`; `day` of Some(d)
/// filters to that UTC day (free tier), None sums all days (paid workload
/// lifetime caps). Filter by workload id when given.
pub fn ledger_usage(
    ledger_text: &str,
    tier: &str,
    day: Option<i64>,
    workload: Option<&str>,
) -> (u64, u64) {
    let mut calls = 0u64;
    let mut toks = 0u64;
    for line in ledger_text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 || f[7] != "OK" || f[3] != tier {
            continue;
        }
        if let Some(d) = day {
            if f[0].parse::<i64>().ok() != Some(d) {
                continue;
            }
        }
        if let Some(w) = workload {
            if f[2] != w {
                continue;
            }
        }
        calls += f[4].parse::<u64>().unwrap_or(0);
        toks += f[5].parse::<u64>().unwrap_or(0);
        toks += f[6].parse::<u64>().unwrap_or(0);
    }
    (calls, toks)
}

/// Call SoulId (Sean 2026-08-16): 8-byte BrutalHash lineage id, 16-hex in the
/// ledger. Preimage = parent-label (workload id) ‖ brief ‖ model ‖ epoch-day —
/// identical preimage ⇒ identical soul, so the ledger itself surfaces repeat
/// briefs before the paid tier re-spends. Order-sensitive combine is the same
/// `hash.rs::combine` the vcs tape uses for lineage roll-ups (wire-first).
fn soul_hex(parent_label: &str, brief: &str, model: &str) -> String {
    let soul = BrutalHash::combine(&[
        BrutalHash::of(parent_label.as_bytes()),
        BrutalHash::of(brief.as_bytes()),
        BrutalHash::of(model.as_bytes()),
        BrutalHash::of(&today_epoch_day().to_le_bytes()),
    ]);
    format!("{:016x}", soul.as_u64())
}

/// The BQ routing verdict for a brief, or `None` (no router / no active
/// centroid / unloadable). Rendered `-\t-` in the ledger — absent, not zero.
type BqVerdict = Option<(u8, u32)>;

/// Route a brief through a baked `.bqr` router file. `None` is a valid, quiet
/// answer — the caller says it aloud once in its own output; it never fails
/// the oracle call (annotate, not veto: a veto threshold on the raw hamming
/// margin would be a guessed constant — the margin-calibration gap named in
/// CONDENSED-GPU-CPU-FLYWHEEL-S13-METAROUTER.md, deliberately not solved here).
pub fn route_brief_at(bqr: &Path, brief: &str) -> BqVerdict {
    let r = BqRouter::load(bqr, 512).ok()?;
    r.route(&embed_prompt(brief)).map(|(sid, margin)| (sid as u8, margin))
}

/// Resolve `flywheel.bqr_path` (root-relative, directives.rs:41) and route the
/// brief. Missing directives or router file resolve to `None`, never `Err` —
/// the BQ verdict annotates the oracle lane; it is not a gate on it.
pub fn route_brief(root: &Path, brief: &str) -> BqVerdict {
    let d = crate::directives::load(root).ok()?;
    let p = PathBuf::from(&d.bqr_path);
    let p = if p.is_absolute() { p } else { root.join(p) };
    route_brief_at(&p, brief)
}

fn append_ledger(root: &Path, workload: &str, tier: &str, est_in: u64, est_out: u64, status: &str, brief_hash: &str, soul: &str, bq: BqVerdict) -> Result<(), String> {
    let p = ledger_path(root);
    let mut body = std::fs::read_to_string(&p).unwrap_or_default();
    if body.is_empty() {
        body.push_str(LEDGER_HEADER);
        body.push('\n');
    }
    let (bq_sid, bq_margin) = match bq {
        Some((s, m)) => (s.to_string(), m.to_string()),
        None => ("-".to_string(), "-".to_string()),
    };
    body.push_str(&format!(
        "{}\t{}\t{workload}\t{tier}\t1\t{est_in}\t{est_out}\t{status}\t{brief_hash}\t{soul}\t{bq_sid}\t{bq_margin}\n",
        today_epoch_day(),
        now_secs()
    ));
    let tmp = p.with_extension("tsv.tmp");
    std::fs::write(&tmp, &body).map_err(|e| format!("ledger tmp write: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("ledger rename: {e}"))?;
    Ok(())
}

fn brief_hash(brief: &str) -> String {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(brief.as_bytes());
    format!("{:08x}", (h.finish() & 0xffff_ffff))
}

/// Write a paid-workload declaration skeleton for Sean to fill in.
pub fn init_workload(root: &Path, id: &str) -> Result<(), String> {
    let dir = workloads_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let p = dir.join(format!("{id}.ron"));
    if p.exists() {
        return Err(format!("workload {id} already declared at {}", p.display()));
    }
    let skeleton = format!(
        "(\n    id: \"{id}\",\n    cap_calls: 100,\n    cap_tokens: 1000000,\n    expires: \"2026-08-31\",\n    description: \"edit me: what is this workload for, and why paid?\",\n)\n"
    );
    std::fs::write(&p, skeleton).map_err(|e| format!("cannot write skeleton: {e}"))?;
    println!("WORKLOAD SKELETON: {} — fill in caps/expiry before first paid call", p.display());
    Ok(())
}

/// `foreman oracle --root <r> --brief <file> [--paid --workload <id>]`
/// `foreman oracle --root <r> --declare <id>`
/// `foreman oracle --root <r> --judge <json-file>` — offline deterministic
/// arbiter pass (stage 4), zero tokens spent: prints ARBITER_OK or the
/// CRITICAL REJECTION block and exits nonzero.
pub fn verb(root: &Path, args: &[String]) -> Result<(), String> {
    if let Some(at) = args.iter().position(|a| a == "--declare") {
        let id = args.get(at + 1).ok_or("--declare takes a workload id")?;
        return init_workload(root, id);
    }
    if let Some(at) = args.iter().position(|a| a == "--classify") {
        let file = args.get(at + 1).ok_or("--classify takes a brief file")?;
        let text = std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?;
        println!("{}", if question_is_structural(&text) { "STRUCTURAL" } else { "PROSE" });
        return Ok(());
    }
    if let Some(at) = args.iter().position(|a| a == "--collide") {
        let file = args.get(at + 1).ok_or("--collide takes a source file")?;
        let tw = args.iter().position(|a| a == "--toward").ok_or("--collide requires --toward <question>")?;
        let toward = args.get(tw + 1).ok_or("--toward takes a question")?;
        let src = std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?;
        // v2 collide squished via forge_ast::rust_squish — not in F:\v3 (quarry
        // organ in v2 forge-ast; its port is a named later weld). Until then the
        // skeleton is clipped raw source, admitted aloud, never dressed as squish.
        let cap = 6 * 1024;
        let mut end = src.len().min(cap);
        while end > 0 && !src.is_char_boundary(end) {
            end -= 1;
        }
        let brief = collide_brief(toward, &src[..end]);
        if args.iter().any(|a| a == "--dry") {
            println!("COLLIDE_DRY bytes={} file={file}", brief.len());
            return Ok(());
        }
        // Wet path re-enters the normal --brief flow, so the whole governed
        // lane (structural gate, caps, souls, schema, arbiter) applies as one.
        let tmp = root.join(".forge").join("oracle-collide-brief.tmp");
        std::fs::write(&tmp, &brief).map_err(|e| format!("collide brief write: {e}"))?;
        let mut fwd: Vec<String> = vec!["--brief".into(), tmp.display().to_string()];
        if let Some(w) = args.iter().position(|a| a == "--workload") {
            fwd.push("--paid".into());
            fwd.push("--workload".into());
            fwd.push(args.get(w + 1).ok_or("--workload takes an id")?.clone());
        }
        return verb(root, &fwd);
    }
    if let Some(at) = args.iter().position(|a| a == "--judge") {
        let file = args.get(at + 1).ok_or("--judge takes a json file")?;
        let text = std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?;
        let v: serde_json::Value = serde_json::from_str(text.trim())
            .map_err(|e| format!("ORACLE_ARBITER_REJECTED: not JSON at all: {e}"))?;
        return match crate::arbiter::rejection(&v) {
            None => {
                println!("ARBITER_OK {file}");
                Ok(())
            }
            Some(log) => Err(format!("ORACLE_ARBITER_REJECTED\n{log}")),
        };
    }

    let brief_at = args.iter().position(|a| a == "--brief").ok_or(
        "usage: foreman oracle --root <r> --brief <file> [--paid --workload <id>] | --declare <id>",
    )?;
    let brief_file = args.get(brief_at + 1).ok_or("--brief takes a file")?;
    let brief = std::fs::read_to_string(brief_file)
        .map_err(|e| format!("cannot read brief {brief_file}: {e}"))?;
    if brief.trim().is_empty() {
        return Err("brief is empty — nothing to ask".into());
    }

    let cfg = load_config(root)?;
    let paid = args.iter().any(|a| a == "--paid");
    let hash = brief_hash(&brief);
    // The BqRouter A/B verdict — computed once, rides every ledger row this
    // attempt writes. Local routing sees ALL oracle traffic; when the flywheel
    // harvests this ledger, each row is already a (query, specialist, outcome)
    // training candidate.
    let bq = route_brief(root, &brief);

    let mut schema_path: Option<String> = None;
    let (tier, model, workload_id) = if paid {
        let wat = args.iter().position(|a| a == "--workload").ok_or(
            "ORACLE_WORKLOAD_NOT_DECLARED: --paid requires --workload <id> (the declaration gate is the point)",
        )?;
        let id = args.get(wat + 1).ok_or("--workload takes an id")?.clone();
        let w = load_workload(root, &id)?;
        let soul = soul_hex(&id, &brief, &cfg.model_paid);
        if today_epoch_day() >= w.expires_epoch_day {
            let _ = append_ledger(root, &id, "paid", 0, 0, "DENIED_EXPIRED", &hash, &soul, bq);
            return Err(format!("ORACLE_WORKLOAD_NOT_DECLARED: workload {id} is expired"));
        }
        let ledger = std::fs::read_to_string(ledger_path(root)).unwrap_or_default();
        let (calls, toks) = ledger_usage(&ledger, "paid", None, Some(&id));
        let need = est_tokens(brief.len());
        if calls >= w.cap_calls || toks + need > w.cap_tokens {
            let _ = append_ledger(root, &id, "paid", 0, 0, "OVER_CAP", &hash, &soul, bq);
            return Err(format!(
                "ORACLE_OVER_CAP: workload {id} at {calls}/{} calls, {toks}/{} est tokens — raise the declared cap or stop",
                w.cap_calls, w.cap_tokens
            ));
        }
        schema_path = w.schema.clone();
        ("paid", cfg.model_paid.clone(), id)
    } else {
        // Governor doctrine, enforced not narrated: the free tier orients,
        // it never carries structural/mutation work — that belongs to a
        // declared paid workload with caps. For a collide brief only the
        // QUESTION is judged (v2: collide IS the free read lane; a source
        // skeleton always contains marker words like `trait`/`surface`, and
        // gating on content would ban free collide outright — observed live
        // 2026-08-16, first wet collide).
        let gate_text = brief
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("ORACLE_B collide toward: "))
            .unwrap_or(&brief);
        if question_is_structural(gate_text) {
            let _ = append_ledger(root, "free", "free", 0, 0, "DENIED_STRUCTURAL", &hash, &soul_hex("free", &brief, &cfg.model_free), bq);
            return Err(format!(
                "ORACLE_STRUCTURAL_NEEDS_WORKLOAD: brief reads structural (marker hit) — declare a paid workload: foreman oracle --root {} --declare <id>",
                root.display()
            ));
        }
        let ledger = std::fs::read_to_string(ledger_path(root)).unwrap_or_default();
        let (calls, toks) = ledger_usage(&ledger, "free", Some(today_epoch_day()), None);
        let need = est_tokens(brief.len());
        if calls >= cfg.free_calls_day || toks + need > cfg.free_tokens_day {
            let _ = append_ledger(root, "free", "free", 0, 0, "OVER_CAP", &hash, &soul_hex("free", &brief, &cfg.model_free), bq);
            return Err(format!(
                "ORACLE_OVER_CAP: free tier at {calls}/{} calls, {toks}/{} est tokens today — wait for the UTC day to roll or declare a paid workload",
                cfg.free_calls_day, cfg.free_tokens_day
            ));
        }
        ("free", cfg.model_free.clone(), "free".to_string())
    };

    // Brief travels as a one-shot `-p` argument: the interactive stdin path
    // hangs forever on the CLI's own consent/auth prompts when run headless
    // (observed 2026-08-16, first live receipt — a >180s hang). Windows arg
    // limits cap this; a bigger brief is refused, sip-cap style, not split.
    if brief.len() > 8 * 1024 {
        return Err(format!(
            "ORACLE_BRIEF_TOO_BIG: {} bytes (cap 8192) — sip the brief down; a smaller question is a better question",
            brief.len()
        ));
    }

    // The CLI holds the auth; this process passes a model id and a brief,
    // never a credential (v2 doctrine, forge-daemon/oracle.rs). One soul per
    // call from here down — same id on every row this attempt writes.
    let soul = soul_hex(&workload_id, &brief, &model);
    // Multi-line briefs cannot ride a .cmd argument (cmd.exe batch-arg rules;
    // rust std refuses them post-CVE-2024-24576), so the brief travels as a
    // soul-named temp file the shim reads with -pfile. Soul-named = two
    // concurrent calls never collide on the same path.
    let brief_tmp = root.join(".forge").join(format!("oracle-brief-{soul}.tmp"));
    std::fs::write(&brief_tmp, brief.trim())
        .map_err(|e| format!("cannot write brief tmp {}: {e}", brief_tmp.display()))?;
    let mut cli_args: Vec<String> = vec![
        "-m".into(),
        model.clone(),
        "--skip-trust".into(),
        "-pfile".into(),
        brief_tmp.display().to_string(),
    ];
    if let Some(sp) = &schema_path {
        cli_args.push("--schema".into());
        cli_args.push(sp.clone());
    }
    let mut child = Command::new(&cfg.cli_path)
        .args(&cli_args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = append_ledger(root, &workload_id, tier, 0, 0, "DENIED_CLI_NOT_FOUND", &hash, &soul, bq);
            format!("ORACLE_CLI_NOT_FOUND: cannot spawn {:?}: {e} — install/login the gemini CLI; auth lives there, never here", cfg.cli_path)
        })?;

    // Hard timeout: poll try_wait, kill on expiry. A hung CLI must become a
    // typed refusal, never a wedged foreman (receipt: the 180s hang above).
    const ORACLE_TIMEOUT_S: u64 = 120;
    let started = std::time::Instant::now();
    let finished = loop {
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) => {
                if started.elapsed().as_secs() >= ORACLE_TIMEOUT_S {
                    let _ = child.kill();
                    break false;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(format!("oracle: try_wait: {e}")),
        }
    };
    let out = child.wait_with_output().map_err(|e| format!("oracle: wait: {e}"))?;
    let _ = std::fs::remove_file(&brief_tmp); // best-effort cleanup, never fatal
    if !finished {
        append_ledger(root, &workload_id, tier, est_tokens(brief.len()), 0, "TIMEOUT", &hash, &soul, bq)?;
        return Err(format!(
            "ORACLE_TIMEOUT: CLI gave no reply in {ORACLE_TIMEOUT_S}s and was killed — check `{} -m {model} -p hi` by hand (auth/consent prompts hang headless runs)",
            cfg.cli_path
        ));
    }
    let reply = String::from_utf8_lossy(&out.stdout).into_owned();

    if !out.status.success() {
        let err_tail: String = String::from_utf8_lossy(&out.stderr).chars().take(200).collect();
        append_ledger(root, &workload_id, tier, est_tokens(brief.len()), 0, "FAILED_AUTH", &hash, &soul, bq)?;
        return Err(format!("ORACLE_AUTH_FAILED: CLI exited {:?} — {err_tail}", out.status.code()));
    }

    // Stage-3 light: a schema-declared workload's reply must at least be JSON.
    // Remote responseSchema enforcement is one oracle; this parse is the local
    // check; the deterministic arbiter (weaver_arbiter port) is the second and
    // lands as its own weld. Never trust remote conformance alone.
    if schema_path.is_some() {
        match serde_json::from_str::<serde_json::Value>(reply.trim()) {
            Err(e) => {
                append_ledger(root, &workload_id, tier, est_tokens(brief.len()), est_tokens(reply.len()), "SCHEMA_REFUSED", &hash, &soul, bq)?;
                return Err(format!(
                    "ORACLE_SCHEMA_REFUSED: workload {workload_id} declared a schema but the reply is not JSON: {e}"
                ));
            }
            // Stage 4: the deterministic arbiter. Remote responseSchema is one
            // oracle, the parse above is the second, this judge is the third —
            // and the only one whose law is ours.
            Ok(v) => {
                if let Some(log) = crate::arbiter::rejection(&v) {
                    append_ledger(root, &workload_id, tier, est_tokens(brief.len()), est_tokens(reply.len()), "ARBITER_REJECTED", &hash, &soul, bq)?;
                    return Err(format!("ORACLE_ARBITER_REJECTED: workload {workload_id}\n{log}"));
                }
            }
        }
    }

    append_ledger(root, &workload_id, tier, est_tokens(brief.len()), est_tokens(reply.len()), "OK", &hash, &soul, bq)?;
    // The BQ verdict said aloud once (refusal-first: an absent router is
    // `bq=none`, never faked and never fatal — sing.rs's own convention).
    let bq_face = match bq {
        Some((s, m)) => format!(
            "{s}/{m}/{:+}",
            forge_ml_bqrouter::margin_trit(Some((s as usize, m)))
        ),
        None => "none".to_string(), // the -1 void arm, spelled as absence
    };
    println!(
        "ORACLE {tier}/{workload_id} model={model} soul={soul} bq={bq_face} est_in={} est_out={}",
        est_tokens(brief.len()),
        est_tokens(reply.len())
    );
    println!("{reply}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_value_extracts_and_strips() {
        let t = "(\n  oracle_cli_path: \"gemini.cmd\",\n  oracle_free_calls_day: 20,\n)";
        assert_eq!(ron_value(t, "oracle_cli_path").as_deref(), Some("gemini.cmd"));
        assert_eq!(ron_value(t, "oracle_free_calls_day").as_deref(), Some("20"));
        assert_eq!(ron_value(t, "absent_key"), None);
    }

    #[test]
    fn civil_date_math_is_correct() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 8, 16), 20681);
        assert_eq!(parse_date_days("2026-08-31").unwrap(), 20696);
        assert!(parse_date_days("not-a-date").is_err());
        assert!(parse_date_days("2026-13-01").is_err());
    }

    #[test]
    fn ledger_usage_sums_only_matching_ok_rows() {
        let l = format!(
            "{LEDGER_HEADER}\n\
             100\t1\tfree\tfree\t1\t50\t70\tOK\tabc\n\
             100\t2\tfree\tfree\t1\t10\t10\tOVER_CAP\tdef\n\
             101\t3\tfree\tfree\t1\t5\t5\tOK\tghi\n\
             100\t4\tgoogleathon\tpaid\t1\t100\t200\tOK\tjkl\n"
        );
        assert_eq!(ledger_usage(&l, "free", Some(100), None), (1, 120));
        assert_eq!(ledger_usage(&l, "free", None, None), (2, 130));
        assert_eq!(ledger_usage(&l, "paid", None, Some("googleathon")), (1, 300));
        assert_eq!(ledger_usage(&l, "paid", None, Some("other")), (0, 0));
    }

    #[test]
    fn structural_classifier_matches_v2_governor() {
        assert!(question_is_structural("who calls MetaRouter::route and is it wired?"));
        assert!(question_is_structural("map the SEAM between intent and render"));
        assert!(!question_is_structural("summarize this changelog in two sentences"));
        // v2's own aperture: a per-file GREEN/ABSENT sweep never trips.
        assert!(!question_is_structural("is this file green or absent"));
    }

    #[test]
    fn soul_is_deterministic_and_input_sensitive() {
        let a = soul_hex("w1", "brief", "model-x");
        assert_eq!(a, soul_hex("w1", "brief", "model-x"));
        assert_ne!(a, soul_hex("w1", "brief CHANGED", "model-x"));
        assert_ne!(a, soul_hex("w2", "brief", "model-x"));
        assert_ne!(a, soul_hex("w1", "brief", "model-y"));
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn ledger_usage_accepts_ten_column_soul_rows() {
        let l = format!(
            "{LEDGER_HEADER}\n\
             100\t1\tfree\tfree\t1\t50\t70\tOK\tabc\tdeadbeefdeadbeef\n"
        );
        assert_eq!(ledger_usage(&l, "free", Some(100), None), (1, 120));
    }

    #[test]
    fn token_estimate_is_quarter_bytes() {
        assert_eq!(est_tokens(400), 100);
        assert_eq!(est_tokens(3), 0);
    }

    #[test]
    fn ledger_header_carries_bq_columns() {
        assert!(LEDGER_HEADER.ends_with("\tbq_specialist\tbq_margin"));
    }

    #[test]
    fn ledger_usage_accepts_twelve_column_bq_rows() {
        // Both an annotated row (bq cols numeric) and an unrouted one (`-`)
        // must sum exactly as before — usage parses by fixed position 0..=6.
        let l = format!(
            "{LEDGER_HEADER}\n\
             100\t1\tfree\tfree\t1\t50\t70\tOK\tabc\tdeadbeefdeadbeef\t4\t120\n\
             100\t2\tfree\tfree\t1\t5\t5\tOK\tdef\tdeadbeefdeadbeef\t-\t-\n"
        );
        assert_eq!(ledger_usage(&l, "free", Some(100), None), (2, 130));
    }

    #[test]
    fn route_brief_at_absent_router_is_none() {
        let p = std::env::temp_dir().join("oracle_bq_no_such_router.bqr");
        let _ = std::fs::remove_file(&p);
        assert_eq!(route_brief_at(&p, "any brief at all"), None);
    }

    #[test]
    fn route_brief_at_untrained_router_is_none() {
        // A saved router with zero active centroids routes nothing — `None`
        // is the honest verdict, not specialist 0 with margin 0.
        let dir = std::env::temp_dir().join("oracle_bq_untrained");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("router.bqr");
        forge_ml_bqrouter::BqRouter::new(512).save(&p).unwrap();
        assert_eq!(route_brief_at(&p, "hello oracle"), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn route_brief_at_routes_a_trained_router() {
        use forge_ml_bqrouter::TrainingPair;
        let brief = "hello oracle, orient me on the changelog";
        // Train specialist 3 on this exact brief's embedding (5 high-outcome
        // pairs clears the MIN_RECORDS activation gate), so routing the same
        // brief must come back 3 — the full embed→binarize→hamming path, no
        // mocks.
        let mut r = forge_ml_bqrouter::BqRouter::new(512);
        let pairs: Vec<TrainingPair> = (0..5)
            .map(|_| TrainingPair {
                specialist_id: 3,
                outcome_permyriad: 9_000,
                query_i8: embed_prompt(brief).to_vec(),
            })
            .collect();
        r.train_from_pairs(&pairs);
        let dir = std::env::temp_dir().join("oracle_bq_trained");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("router.bqr");
        r.save(&p).unwrap();

        let (sid, _margin) = route_brief_at(&p, brief).expect("trained router must route");
        assert_eq!(sid, 3);
        std::fs::remove_dir_all(&dir).ok();
    }
}
