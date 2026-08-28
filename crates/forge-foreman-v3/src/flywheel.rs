//! W5 — the weld flywheel journal (WAVE-WELD-PROPOSAL §W5, approved
//! ARCH000 2026-08-10 "W5 approved as drafted").
//!
//! Every weld-ladder attempt lands here as one ndjson row: the sipped prompt
//! as sent, the emitted weld verbatim, and a TYPED verdict (L12 — proof is
//! typed, not toned). When a green weld eventually lands for a session — a
//! later attempt, a later rung, or a hand fix via `foreman weld --resolve` —
//! one `attempt: 0` resolution row records it, and training pairs are DERIVED,
//! never stored: every red row in the session pairs its prompt with the
//! resolution's weld as target. One queued red hand-fixed tomorrow
//! retroactively labels every wrong weld the ladder emitted tonight.
//!
//! The stream mechanics are v2's flywheel, kept whole (L13): ndjson,
//! append-only, one JSON object per line (`flywheel_distill.rs:79-141`).
//! Only the pair's content is new. The codec is hand-rolled — no serde, no
//! deps — and L07-tested: `parse_line(encode(row)) == row` over interior,
//! empty-weld, sip-cap-sized, and session-0 rows.
//!
//! An unwritable journal is a LOUD error, never a skip: the ladder must not
//! grind dark (same stance as the grind log, `run.rs`).

use std::path::{Path, PathBuf};

use crate::directives::Directives;

/// What one ladder attempt measurably did. Typed, closed, refused on unknown
/// words — the same posture as [`crate::census::Disposition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The sidecar refused the INFER_WELD (ERR frame, dead socket, cliff).
    EngineRefused,
    /// The reply did not survive re-parse/plan (defense in depth said no).
    ParseRefused,
    /// The weld applied but the gate stayed red; bytes were rolled back.
    GateRed,
    /// The weld applied and the gate went green.
    Green,
}

impl Verdict {
    /// Parse the journal column, refusing unknown words — a typo must not
    /// silently mislabel a training row.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "engine_refused" => Ok(Self::EngineRefused),
            "parse_refused" => Ok(Self::ParseRefused),
            "gate_red" => Ok(Self::GateRed),
            "green" => Ok(Self::Green),
            other => Err(format!("unknown verdict {other:?}")),
        }
    }

    /// The journal column spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EngineRefused => "engine_refused",
            Self::ParseRefused => "parse_refused",
            Self::GateRed => "gate_red",
            Self::Green => "green",
        }
    }
}

/// One journal row. `attempt` 1.. is a ladder attempt; `attempt` 0 is the
/// session's resolution — the eventually-green weld that labels the reds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeldPair {
    /// fnv1a-64 of the session's FIRST sipped prompt — groups a ladder.
    pub session: u64,
    /// Unix milliseconds at write time, same clock as the grind log.
    pub ts_ms: u64,
    /// 1-based attempt number; 0 marks the resolution row.
    pub attempt: u32,
    /// The crate the ladder was repairing.
    pub crate_name: String,
    /// The sip as sent, feedback included; empty on a resolution row.
    pub prompt: String,
    /// The reply verbatim; empty when the engine refused.
    pub weld: String,
    /// What happened, typed.
    pub verdict: Verdict,
    /// Tail of the gate/refusal output that judged this attempt.
    pub gate_tail: String,
}

/// fnv1a-64 over the first sipped prompt — the session key. Deterministic by
/// construction (same red, same sip, same session), no platform RNG.
pub fn session_of(first_prompt: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in first_prompt.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Where the journal lives: `flywheel.weld_pairs_path` under the root.
pub fn journal_path(root: &Path, d: &Directives) -> PathBuf {
    root.join(&d.weld_pairs_path)
}

/// Unix milliseconds, the grind log's clock (`run.rs` journal).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Build one attempt row, stamped now.
pub fn attempt_row(
    session: u64,
    attempt: u32,
    crate_name: &str,
    prompt: &str,
    weld: &str,
    verdict: Verdict,
    gate_tail: &str,
) -> WeldPair {
    WeldPair {
        session,
        ts_ms: now_ms(),
        attempt,
        crate_name: crate_name.to_string(),
        prompt: prompt.to_string(),
        weld: weld.to_string(),
        verdict,
        gate_tail: gate_tail.to_string(),
    }
}

/// Build the session's resolution row (`attempt: 0`), stamped now.
pub fn resolution(session: u64, crate_name: &str, weld: &str) -> WeldPair {
    attempt_row(session, 0, crate_name, "", weld, Verdict::Green, "")
}

impl WeldPair {
    /// One ndjson line, fixed field order, no interior newline possible —
    /// every string is escaped before it reaches the buffer.
    pub fn encode(&self) -> String {
        let mut o = String::with_capacity(
            96 + self.crate_name.len() + self.prompt.len() + self.weld.len() + self.gate_tail.len(),
        );
        o.push_str(&format!(
            "{{\"session\":{},\"ts_ms\":{},\"attempt\":{},\"crate\":\"",
            self.session, self.ts_ms, self.attempt
        ));
        esc(&self.crate_name, &mut o);
        o.push_str("\",\"prompt\":\"");
        esc(&self.prompt, &mut o);
        o.push_str("\",\"weld\":\"");
        esc(&self.weld, &mut o);
        o.push_str("\",\"verdict\":\"");
        o.push_str(self.verdict.as_str());
        o.push_str("\",\"gate_tail\":\"");
        esc(&self.gate_tail, &mut o);
        o.push_str("\"}");
        o
    }

    /// Decode one journal line. Strict — this parser accepts exactly what
    /// [`WeldPair::encode`] emits (the journal has one writer), and refuses
    /// anything else loudly rather than harvesting a corrupt training row.
    pub fn parse_line(line: &str) -> Result<Self, String> {
        let mut j = J { s: line.as_bytes(), i: 0 };
        j.lit("{\"session\":")?;
        let session = j.num()?;
        j.lit(",\"ts_ms\":")?;
        let ts_ms = j.num()?;
        j.lit(",\"attempt\":")?;
        let attempt = u32::try_from(j.num()?).map_err(|_| "attempt exceeds u32".to_string())?;
        j.lit(",\"crate\":")?;
        let crate_name = j.string()?;
        j.lit(",\"prompt\":")?;
        let prompt = j.string()?;
        j.lit(",\"weld\":")?;
        let weld = j.string()?;
        j.lit(",\"verdict\":")?;
        let verdict = Verdict::parse(&j.string()?)?;
        j.lit(",\"gate_tail\":")?;
        let gate_tail = j.string()?;
        j.lit("}")?;
        if j.i != j.s.len() {
            return Err(format!("trailing bytes at {}", j.i));
        }
        Ok(WeldPair { session, ts_ms, attempt, crate_name, prompt, weld, verdict, gate_tail })
    }
}

/// JSON string escape: `"` `\` and every control byte; `\n` `\r` `\t` get
/// their short forms, the rest `\u00XX`. Non-ASCII rides raw — JSON permits
/// unescaped UTF-8 and the decode side copies multibyte sequences whole.
fn esc(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

/// The strict line scanner — same shape as the weld parser's `P`.
struct J<'a> {
    s: &'a [u8],
    i: usize,
}

impl J<'_> {
    /// Exact literal match; the encoder emits no optional whitespace, so the
    /// decoder tolerates none.
    fn lit(&mut self, t: &str) -> Result<(), String> {
        let b = t.as_bytes();
        if self.s.len() >= self.i + b.len() && &self.s[self.i..self.i + b.len()] == b {
            self.i += b.len();
            Ok(())
        } else {
            Err(format!("expected {t:?} at byte {}", self.i))
        }
    }

    /// An unsigned decimal integer.
    fn num(&mut self) -> Result<u64, String> {
        let a = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if a == self.i {
            return Err(format!("expected digits at byte {a}"));
        }
        std::str::from_utf8(&self.s[a..self.i])
            .ok()
            .and_then(|d| d.parse().ok())
            .ok_or_else(|| format!("number at byte {a} does not fit u64"))
    }

    /// A quoted JSON string with the escapes [`esc`] emits, plus generic
    /// `\uXXXX` for any BMP scalar.
    fn string(&mut self) -> Result<String, String> {
        self.lit("\"")?;
        let mut out = String::new();
        loop {
            let Some(&c) = self.s.get(self.i) else {
                return Err(format!("unterminated string at byte {}", self.i));
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return Err(format!("bad escape at byte {}", self.i));
                    };
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        b'u' => {
                            let hex = self
                                .s
                                .get(self.i..self.i + 4)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .ok_or_else(|| format!("bad \\u at byte {}", self.i))?;
                            self.i += 4;
                            out.push(
                                char::from_u32(hex)
                                    .ok_or_else(|| format!("\\u{hex:04x} is not a scalar"))?,
                            );
                        }
                        _ => return Err(format!("bad escape at byte {}", self.i - 1)),
                    }
                }
                _ => {
                    let a = self.i - 1;
                    while self.i < self.s.len() && self.s[self.i] & 0xC0 == 0x80 {
                        self.i += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.s[a..self.i]).unwrap_or("\u{fffd}"));
                }
            }
        }
    }
}

/// Append one row. Creates the journal's directory on first write; any
/// failure is a loud error carrying the path — the caller must NOT proceed
/// past it (a dark flywheel is the defect W5 exists to kill).
pub fn append(path: &Path, row: &WeldPair) -> Result<(), String> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("flywheel: cannot create {}: {e}", parent.display()))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("flywheel journal unwritable at {}: {e}", path.display()))?;
    writeln!(f, "{}", row.encode())
        .map_err(|e| format!("flywheel journal write failed at {}: {e}", path.display()))
}

/// Read the whole journal. A missing file or a malformed line is an error
/// with its line number — corrupt rows are refused, never skipped into a
/// training corpus.
pub fn load(path: &Path) -> Result<Vec<WeldPair>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("flywheel: cannot read {}: {e}", path.display()))?;
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(
            WeldPair::parse_line(line)
                .map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?,
        );
    }
    Ok(rows)
}

/// The most recent session journaled for a crate — what `--resolve` stamps.
pub fn latest_session_for(rows: &[WeldPair], crate_name: &str) -> Option<u64> {
    rows.iter().rev().find(|r| r.crate_name == crate_name).map(|r| r.session)
}

/// Derive a session's training pairs: every red attempt row's prompt, paired
/// with the resolution row's weld as target. No resolution row, no pairs —
/// derivation is pure and re-runnable, nothing is stored twice.
pub fn derived_pairs(rows: &[WeldPair], session: u64) -> Vec<(String, String)> {
    let Some(res) = rows.iter().rev().find(|r| r.session == session && r.attempt == 0) else {
        return Vec::new();
    };
    rows.iter()
        .filter(|r| r.session == session && r.attempt > 0 && r.verdict != Verdict::Green)
        .map(|r| (r.prompt.clone(), res.weld.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interior() -> WeldPair {
        WeldPair {
            session: 0x1234_5678_9abc_def0,
            ts_ms: 1_786_400_000_000,
            attempt: 2,
            crate_name: "weldtest".into(),
            prompt: "fix it:\nline \"two\"\ttabbed\\slashed\r\nsite --> src/lib.rs:4:13".into(),
            weld: "Weld(lane:\"repair\",files:[],gate:\"\",receipt:\"\")".into(),
            verdict: Verdict::GateRed,
            gate_tail: "error[E0425]: cannot find value `speling`".into(),
        }
    }

    /// L07 over interior ∪ empty-weld ∪ sip-cap-sized prompt ∪ session 0 —
    /// the proof plan's own corpus, plus control bytes and non-BMP unicode.
    #[test]
    fn a_row_survives_encode_then_parse_byte_exactly() {
        let mut cases = vec![interior()];
        let mut empty_weld = interior();
        empty_weld.weld = String::new();
        empty_weld.verdict = Verdict::EngineRefused;
        cases.push(empty_weld);
        let mut cap_sized = interior();
        cap_sized.prompt = "x".repeat(2048);
        cases.push(cap_sized);
        let mut zero = interior();
        zero.session = 0;
        zero.attempt = 0;
        cases.push(zero);
        let mut spicy = interior();
        spicy.prompt = "ctl\u{1} bell\u{7} é 🦀 \u{0}end".into();
        spicy.gate_tail = String::new();
        cases.push(spicy);

        for row in cases {
            let line = row.encode();
            assert!(!line.contains('\n'), "one row is one line: {line}");
            let back = WeldPair::parse_line(&line).unwrap();
            assert_eq!(back, row, "parse(encode(x)) == x");
        }
    }

    #[test]
    fn a_corrupt_line_is_refused_with_its_line_number() {
        assert!(WeldPair::parse_line("{\"session\":oops").is_err());
        assert!(WeldPair::parse_line("").is_err());
        let mut truncated = interior().encode();
        truncated.pop();
        assert!(WeldPair::parse_line(&truncated).is_err());
        let trailing = format!("{} ", interior().encode());
        assert!(WeldPair::parse_line(&trailing).is_err(), "trailing bytes are refused");
        assert!(Verdict::parse("greenish").is_err(), "verdict vocabulary is closed");
    }

    #[test]
    fn the_session_key_is_deterministic_and_content_bound() {
        assert_eq!(session_of("same sip"), session_of("same sip"));
        assert_ne!(session_of("same sip"), session_of("same sip "));
        // fnv1a-64 of the empty string is the offset basis — a fixed vector,
        // so the hash can never silently change shape.
        assert_eq!(session_of(""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn derived_pairs_label_every_red_with_the_resolution_weld() {
        let s = session_of("the red");
        let rows = vec![
            attempt_row(s, 1, "weldtest", "prompt one", "wrong-a", Verdict::GateRed, "red"),
            attempt_row(s, 2, "weldtest", "prompt two", "wrong-b", Verdict::ParseRefused, "no"),
            attempt_row(999, 1, "other", "other prompt", "x", Verdict::GateRed, "red"),
            resolution(s, "weldtest", "the-green-weld"),
        ];
        let pairs = derived_pairs(&rows, s);
        assert_eq!(pairs.len(), 2, "both reds, the other session excluded");
        assert!(pairs.iter().all(|(_, w)| w == "the-green-weld"));
        assert_eq!(pairs[0].0, "prompt one");
        assert_eq!(pairs[1].0, "prompt two");
        assert!(derived_pairs(&rows, 999).is_empty(), "no resolution, no pairs");
        assert_eq!(latest_session_for(&rows, "other"), Some(999));
        assert_eq!(latest_session_for(&rows, "phantom"), None);
    }

    #[test]
    fn append_creates_the_directory_and_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("flywheel-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("deep").join("weld-pairs.ndjson");
        let a = interior();
        let b = resolution(a.session, "weldtest", "green-weld");
        append(&path, &a).unwrap();
        append(&path, &b).unwrap();
        let rows = load(&path).unwrap();
        assert_eq!(rows, vec![a, b], "append order is load order");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unwritable_journal_is_a_loud_error() {
        let dir = std::env::temp_dir().join(format!("flywheel-block-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("blocker"), "a file, not a dir").unwrap();
        let path = dir.join("blocker").join("weld-pairs.ndjson");
        let e = append(&path, &interior()).unwrap_err();
        assert!(e.contains("flywheel"), "the error names the flywheel: {e}");
        assert!(load(&path).is_err(), "a missing journal reads as an error, not as empty");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
