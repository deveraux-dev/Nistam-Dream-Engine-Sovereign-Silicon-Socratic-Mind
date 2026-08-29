//! Oracle escalation: when local Gemma fails or gives Ambiguous verdict,
//! escalate to remote Gemini via `gemini-rest.cmd` (F:\v3\.forge\tools\).
//! Caps from directives.ron honored; over-cap refusal typed and loud.

use std::path::{Path, PathBuf};

/// Typed refusal reason when escalation cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationRefusal {
    /// gemini-rest.cmd not found at the configured path.
    CliNotFound(String),
    /// Free-tier call cap reached for today.
    OverCallCap,
    /// Free-tier token cap reached for today.
    OverTokenCap,
    /// Command execution or output parsing failed (includes auth failures).
    CommandFailed(String),
}

impl std::fmt::Display for EscalationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CliNotFound(why) => write!(f, "ESCALATION_CLI_NOT_FOUND: {why}"),
            Self::OverCallCap => write!(f, "ESCALATION_OVER_CALL_CAP: free tier call limit reached today"),
            Self::OverTokenCap => write!(f, "ESCALATION_OVER_TOKEN_CAP: free tier token budget exhausted today"),
            Self::CommandFailed(why) => write!(f, "ESCALATION_COMMAND_FAILED: {why}"),
        }
    }
}

#[derive(Debug, Clone)]
struct OracleConfig {
    cli_path: String,
    model_free: String,
    free_calls_day: u64,
    free_tokens_day: u64,
}

fn ron_value(text: &str, key: &str) -> Option<String> {
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

fn load_oracle_config(root: &Path) -> Result<OracleConfig, String> {
    let p = root.join(".forge").join("v3-directives.ron");
    let text =
        std::fs::read_to_string(&p).map_err(|e| format!("cannot read directives: {e}"))?;

    Ok(OracleConfig {
        cli_path: ron_value(&text, "oracle_cli_path")
            .ok_or("missing oracle_cli_path in directives")?,
        model_free: ron_value(&text, "oracle_model_free")
            .ok_or("missing oracle_model_free in directives")?,
        free_calls_day: ron_value(&text, "oracle_free_calls_day")
            .ok_or("missing oracle_free_calls_day in directives")?
            .parse()
            .map_err(|_| "oracle_free_calls_day not a u64")?,
        free_tokens_day: ron_value(&text, "oracle_free_tokens_day")
            .ok_or("missing oracle_free_tokens_day in directives")?
            .parse()
            .map_err(|_| "oracle_free_tokens_day not a u64")?,
    })
}

fn today_epoch_day() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86_400) as i64
}

fn est_tokens(bytes: usize) -> u64 {
    (bytes as u64) / 4
}

fn ledger_free_usage_today(ledger_text: &str) -> (u64, u64) {
    let today = today_epoch_day();
    let mut calls = 0u64;
    let mut toks = 0u64;

    for line in ledger_text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 {
            continue;
        }
        if f[7] != "OK" || f[3] != "free" {
            continue;
        }
        if f[0].parse::<i64>().ok() != Some(today) {
            continue;
        }
        calls += f[4].parse::<u64>().unwrap_or(0);
        toks += f[5].parse::<u64>().unwrap_or(0);
        toks += f[6].parse::<u64>().unwrap_or(0);
    }
    (calls, toks)
}

/// Check if escalation would exceed caps. Returns Err if over-cap, Ok if
/// within budget. Does NOT make a network call; purely reads ledger from disk.
///
/// This is the gate that prevents silent over-budget escalation. Call before
/// `try_escalate` when you want to validate caps independently.
pub fn check_caps(root: &Path, question_bytes: usize) -> Result<(), EscalationRefusal> {
    let cfg = load_oracle_config(root).map_err(|e| {
        EscalationRefusal::CommandFailed(format!("failed to load oracle config: {e}"))
    })?;

    let ledger_path = root.join(".forge").join("oracle-ledger.tsv");
    let ledger = std::fs::read_to_string(&ledger_path).unwrap_or_default();
    let (calls, toks) = ledger_free_usage_today(&ledger);

    if calls >= cfg.free_calls_day {
        return Err(EscalationRefusal::OverCallCap);
    }

    let need = est_tokens(question_bytes);
    if toks + need > cfg.free_tokens_day {
        return Err(EscalationRefusal::OverTokenCap);
    }

    Ok(())
}

/// Try escalating a failed local verdict to remote Gemini. Checks caps first
/// (without network access). Returns the remote reply text or a typed refusal.
///
/// Cap check is always performed before any network call. If caps are exhausted,
/// returns the appropriate typed refusal without touching the CLI. This ensures
/// cost control even if the function is called repeatedly.
pub fn try_escalate(
    root: &Path,
    question: &str,
) -> Result<String, EscalationRefusal> {
    check_caps(root, question.len())?;

    let cfg = load_oracle_config(root).map_err(|e| {
        EscalationRefusal::CommandFailed(format!("failed to load oracle config: {e}"))
    })?;

    if !PathBuf::from(&cfg.cli_path).exists() {
        return Err(EscalationRefusal::CliNotFound(format!(
            "gemini-rest.cmd not found at {}",
            cfg.cli_path
        )));
    }

    let output = std::process::Command::new(&cfg.cli_path)
        .args(&["-m", &cfg.model_free, "-p", question])
        .output()
        .map_err(|e| {
            EscalationRefusal::CommandFailed(format!("failed to spawn gemini-rest.cmd: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EscalationRefusal::CommandFailed(format!(
            "gemini-rest.cmd exited with code {:?}: {}",
            output.status.code(),
            stderr.chars().take(100).collect::<String>()
        )));
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| EscalationRefusal::CommandFailed(format!("reply not UTF-8: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_caps_refuses_when_free_calls_exhausted() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_calls");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "dummy.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 2,
oracle_free_tokens_day: 1000000,
"#,
        )
        .unwrap();

        let today = today_epoch_day();
        std::fs::write(
            tmp.join(".forge/oracle-ledger.tsv"),
            format!(
                "day\tsecs\tworkload\ttier\tcalls\test_in\test_out\tstatus\tbrief_hash\tsoul\tbq_specialist\tbq_margin\n\
                 {today}\t1\tfree\tfree\t1\t100\t100\tOK\tabc\tdef\t-\t-\n\
                 {today}\t2\tfree\tfree\t1\t100\t100\tOK\tghi\tjkl\t-\t-\n"
            ),
        )
        .unwrap();

        let result = check_caps(&tmp, 1000);
        assert_eq!(result, Err(EscalationRefusal::OverCallCap));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn check_caps_refuses_when_free_tokens_exhausted() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_tokens");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "dummy.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 100,
oracle_free_tokens_day: 1000,
"#,
        )
        .unwrap();

        let today = today_epoch_day();
        let in_tokens = 400;
        let out_tokens = 400;
        std::fs::write(
            tmp.join(".forge/oracle-ledger.tsv"),
            format!(
                "day\tsecs\tworkload\ttier\tcalls\test_in\test_out\tstatus\tbrief_hash\tsoul\tbq_specialist\tbq_margin\n\
                 {today}\t1\tfree\tfree\t1\t{in_tokens}\t{out_tokens}\tOK\tabc\tdef\t-\t-\n"
            ),
        )
        .unwrap();

        let question_bytes = 2400;
        let result = check_caps(&tmp, question_bytes);
        assert_eq!(result, Err(EscalationRefusal::OverTokenCap));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn check_caps_allows_when_within_budget() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_ok");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "dummy.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 100,
oracle_free_tokens_day: 1000000,
"#,
        )
        .unwrap();

        let result = check_caps(&tmp, 100);
        assert!(result.is_ok(), "should allow when within budget");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn try_escalate_refuses_when_cli_missing() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_cli_missing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "F:\nonexistent\gemini-rest.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 100,
oracle_free_tokens_day: 1000000,
"#,
        )
        .unwrap();

        std::env::set_var("GEMINI_API_KEY", "dummy");
        let result = try_escalate(&tmp, "hello");
        assert!(
            matches!(result, Err(EscalationRefusal::CliNotFound(_))),
            "got: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn try_escalate_checks_cli_existence_before_running() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_cli_exists");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "/nonexistent/does/not/exist.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 100,
oracle_free_tokens_day: 1000000,
"#,
        )
        .unwrap();

        let result = try_escalate(&tmp, "hello");
        assert!(matches!(result, Err(EscalationRefusal::CliNotFound(_))), "should refuse if CLI missing");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn try_escalate_respects_over_cap() {
        let tmp = std::env::temp_dir().join("oracle_escalate_test_over_cap");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".forge")).unwrap();

        std::fs::write(
            tmp.join(".forge/v3-directives.ron"),
            r#"
oracle_cli_path: "F:\v3\.forge\tools\gemini-rest.cmd",
oracle_model_free: "gemini-2.5-flash-lite",
oracle_free_calls_day: 2,
oracle_free_tokens_day: 1000000,
"#,
        )
        .unwrap();

        let today = today_epoch_day();
        std::fs::write(
            tmp.join(".forge/oracle-ledger.tsv"),
            format!(
                "day\tsecs\tworkload\ttier\tcalls\test_in\test_out\tstatus\tbrief_hash\tsoul\tbq_specialist\tbq_margin\n\
                 {today}\t1\tfree\tfree\t1\t100\t100\tOK\tabc\tdef\t-\t-\n\
                 {today}\t2\tfree\tfree\t1\t100\t100\tOK\tghi\tjkl\t-\t-\n"
            ),
        )
        .unwrap();

        std::env::set_var("GEMINI_API_KEY", "dummy");
        let result = try_escalate(&tmp, "hello");
        assert!(
            matches!(result, Err(EscalationRefusal::OverCallCap)),
            "should refuse over-cap before network call"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn escalation_refusal_display() {
        assert_eq!(
            format!("{}", EscalationRefusal::CliNotFound("not found".into())),
            "ESCALATION_CLI_NOT_FOUND: not found"
        );
        assert_eq!(
            format!("{}", EscalationRefusal::OverCallCap),
            "ESCALATION_OVER_CALL_CAP: free tier call limit reached today"
        );
        assert_eq!(
            format!("{}", EscalationRefusal::OverTokenCap),
            "ESCALATION_OVER_TOKEN_CAP: free tier token budget exhausted today"
        );
        assert_eq!(
            format!("{}", EscalationRefusal::CommandFailed("auth failed".into())),
            "ESCALATION_COMMAND_FAILED: auth failed"
        );
    }
}
