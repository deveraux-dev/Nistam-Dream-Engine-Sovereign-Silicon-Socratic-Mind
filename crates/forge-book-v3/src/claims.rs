//! DOCTRINE CLAIM RESOLUTION (Sean 2026-08-04: "never ever porting prose to code verbatim
//! again") — a compiled doctrine string that names code is checked against disk.
//!
//! The 08-04 defect that earned this module: `.claude/skills/massloop/SKILL.md` was folded
//! into [`crate::oracle1_governor`] LOSSLESSLY, because `agent-code` says a fold condenses
//! and never removes. Lossless is right for the prose and wrong for the CLAIMS inside it —
//! the skill said milestone 7 fails with exit 77, and `massread --selftest` returns 77 for
//! a closed wire (`forge_firewall::egress::REFUSED_EXIT`) while 77-as-wrong-lane is
//! `forge_studio::board_harvest::EXIT_WRONG_LANE`. Two modules, same number, different
//! fault. A reader following the ladder would have re-fired at a shut door.
//!
//! Nothing here walks the tree or finds the repo root on its own. [`crate::seams::root`],
//! [`crate::type_homes::crate_sources`] and [`crate::type_homes::defines`] already do
//! that, and each of them was already the tested answer to its own question.

use crate::type_homes::{crate_sources, crates_dir, defines};
use std::collections::BTreeMap;

/// A checkable assertion found inside a doctrine string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// `board_sync.rs:429` — a file and a line number inside it.
    FileLine {
        /// The claimed file path.
        file: String,
        /// The claimed line number inside it.
        line: usize,
    },
    /// `forge_studio::massread::API_FILE_CEILING` — a path whose last segment is a symbol.
    Symbol {
        /// The dotted/colon path whose last segment names the symbol.
        path: String,
    },
    /// A bare process exit code quoted as a fact, e.g. `77 =` or `exit 2`.
    ExitCode {
        /// The claimed exit code.
        code: i32,
    },
}

/// What disk said about a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    /// The claim was verified against disk.
    Resolved,
    /// The file the claim references does not exist.
    FileAbsent,
    /// The file exists but stops short of the line the claim cites.
    LineAbsent,
    /// The symbol the claim references was not found in any source.
    SymbolAbsent,
    /// An exit code quoted with no constant named beside it. Not a disk fact — a
    /// READABILITY fault, and the exact one that produced this module.
    ExitCodeUnowned,
}

impl ClaimState {
    /// Returns true if the claim resolved successfully on disk.
    pub fn ok(self) -> bool {
        self == ClaimState::Resolved
    }
}

/// Files whose names are too common to resolve by basename alone. A claim naming one of
/// these resolves against ANY match, which is weak — but a false RED on `mod.rs` teaches
/// people to switch the gate off, and a gate nobody runs proves nothing.
const AMBIGUOUS_BASENAMES: &[&str] = &["mod.rs", "lib.rs", "main.rs", "build.rs"];

/// `tok` as a crate path, or None if it is any other `::` shape.
///
/// Requires two things a variant path never has: a snake_case (or all-lowercase) FIRST
/// segment, which is what a crate or module is named, and at least two `::` separators.
/// `Type::Variant` fails the first, `mod::Thing` fails the second.
fn crate_path(tok: &str) -> Option<String> {
    // TRUNCATE at the first non-ident char, never filter it out: filtering turned
    // `SYNTHESES.len()` into the symbol `SYNTHESESlen`, which disk correctly refused and
    // which no human ever wrote. A path ends where the identifier ends.
    let end = tok
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_' || *c == ':'))
        .map_or(tok.len(), |(i, _)| i);
    let clean = tok[..end].trim_end_matches(':');
    if clean.matches("::").count() < 2 || clean.starts_with("::") {
        return None;
    }
    let head = clean.split("::").next()?;
    // The standard library is not in `crates/`, so a `std::` path is unresolvable by
    // construction — reporting it as absent is a lie about this repo, not a finding.
    if matches!(head, "std" | "core" | "alloc") {
        return None;
    }
    let snake = !head.is_empty()
        && head.chars().next().is_some_and(|c| c.is_lowercase())
        && !head.chars().any(|c| c.is_uppercase());
    snake.then(|| clean.to_string())
}

/// Pull every claim out of one doctrine string.
///
/// Deliberately conservative: it under-matches rather than over-matches, because the
/// failure mode of a noisy claim scanner is a red build nobody believes.
pub fn scan(text: &str) -> Vec<Claim> {
    let mut out = Vec::new();
    for raw in text.split(|c: char| c.is_whitespace() || "()[]{}\"'`,;".contains(c)) {
        let tok = raw.trim_matches(|c: char| c == '.' || c == '·');
        if tok.is_empty() {
            continue;
        }
        // `<file>.rs:<line>` — the 164-hit shape.
        if let Some((file, rest)) = tok.split_once(".rs:") {
            if let Ok(line) = rest.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<usize>() {
                let leaf = file.rsplit(['/', '\\', '→', '-', ':']).next().unwrap_or(file);
                // A real module stem is snake_case. Anything else is a prose artifact —
                // `…density_pmy-as-uniform→unified.rs` split mid-phrase, not a file.
                let stem_ok = !leaf.is_empty()
                    && leaf.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
                if line > 0 && stem_ok {
                    out.push(Claim::FileLine { file: format!("{leaf}.rs"), line });
                    continue;
                }
            }
        }
        // `forge_studio::massread::API_FILE_CEILING` — a CRATE path, and only that.
        //
        // The first cut matched any `a::b`, which made the report lane useless: ordinary
        // Rust is full of `AchievementTier::Bronze` and `Rung::Proven`, and an enum
        // variant is not a declared symbol `defines` can find. A doctrine claim names a
        // crate, so the first segment must be snake_case and the path must have depth.
        if let Some(clean) = crate_path(tok) {
            out.push(Claim::Symbol { path: clean });
        }
    }
    // Exit codes are matched on the WHOLE string: they appear as `77 = wrong lane`, and a
    // token scan cannot see that the number is being quoted as a verdict.
    for code in [77i32, 64, 2, 1] {
        let quoted = [format!("{code} ="), format!("exit {code}"), format!("= {code}")];
        if quoted.iter().any(|q| text.contains(q.as_str())) {
            out.push(Claim::ExitCode { code });
        }
    }
    out
}

/// Every `.rs` source under `crates/`, read once and reused across a whole audit.
pub struct Sources(BTreeMap<String, String>);

impl Sources {
    /// Walk `crates/` once. Reuses [`crate_sources`] — no second walker.
    pub fn load() -> Self {
        Sources(crate_sources(&crates_dir()))
    }

    fn by_basename(&self, leaf: &str) -> Vec<&String> {
        self.0
            .iter()
            .filter(|(p, _)| p.rsplit('/').next().is_some_and(|b| b == leaf))
            .map(|(_, src)| src)
            .collect()
    }
}

/// Does `src` DECLARE `ident` — as a type-ish item ([`defines`]) or as a `const`/`static`?
///
/// [`defines`] covers seven decl kinds and `const` is not among them, because `type_homes`
/// asks about TYPES. Doctrine claims mostly name constants (`API_FILE_CEILING`,
/// `REFUSED_EXIT`), so this widens the question without editing that module's answer.
///
/// A bare `contains` was the first cut and it was WRONG in the way law 5 names: the test
/// asserting `NoSuchSymbolZZZ` is absent contains that string, so this file proved the
/// symbol it was written to disprove. A mention is not a declaration.
fn declares(src: &str, ident: &str) -> bool {
    if defines(src, ident) {
        return true;
    }
    ["const ", "static "].iter().any(|kind| {
        src.match_indices(kind).any(|(at, _)| {
            src[at + kind.len()..]
                .strip_prefix(ident)
                .is_some_and(|tail| tail.starts_with([':', ' ']))
        })
    })
}

/// The exit codes a doctrine row must name a constant for.
///
/// NOT every code. 0/1/2 are conventional across every verb in the tree (ok / lane failed
/// / usage), and demanding a constant beside each one is the noise that gets a gate
/// switched off. The 08-04 fault was an OVERLOADED code in the sysexits range: 77 means
/// `egress::REFUSED_EXIT` in one module and `board_harvest::EXIT_WRONG_LANE` in another,
/// and the number alone cannot say which. That range is where collisions live.
const OWNED_EXIT_FLOOR: i32 = 64;

/// Resolve one claim. `owner_text` is the doctrine string the claim came from — an exit
/// code is only judged against the sentence that quotes it.
pub fn resolve(c: &Claim, src: &Sources, owner_text: &str) -> ClaimState {
    match c {
        Claim::FileLine { file, line } => {
            let hits = src.by_basename(file);
            if hits.is_empty() {
                return ClaimState::FileAbsent;
            }
            // Ambiguous basenames resolve on existence alone — see AMBIGUOUS_BASENAMES.
            if AMBIGUOUS_BASENAMES.contains(&file.as_str()) {
                return ClaimState::Resolved;
            }
            match hits.iter().any(|s| s.lines().count() >= *line) {
                true => ClaimState::Resolved,
                false => ClaimState::LineAbsent,
            }
        }
        Claim::Symbol { path } => {
            let segs: Vec<&str> = path.split("::").filter(|s| !s.is_empty()).collect();
            let Some(last) = segs.last().copied() else { return ClaimState::SymbolAbsent };
            // `crate::atlas::CapabilityStatus::Proven` — when the segment BEFORE the tail
            // is CamelCase, the tail is one of its variants. `defines` finds the enum, not
            // the variant, so resolve the type that owns it and the claim is answered.
            let ident = match segs.len() >= 2 {
                true if segs[segs.len() - 2].chars().next().is_some_and(char::is_uppercase) => {
                    segs[segs.len() - 2]
                }
                _ => last,
            };
            // A lowercase tail is a module or fn path segment, not a named item to prove.
            // Only SCREAMING_CASE and CamelCase tails are claims about a declared symbol.
            if ident.chars().next().is_some_and(|c| c.is_lowercase()) {
                return ClaimState::Resolved;
            }
            match src.0.values().any(|s| declares(s, ident)) {
                true => ClaimState::Resolved,
                false => ClaimState::SymbolAbsent,
            }
        }
        // THE 08-04 RULE, GENERALIZED. An exit code quoted as a verdict must name the
        // constant that owns it, because 77 alone is a coin flip between two modules.
        Claim::ExitCode { code } => {
            if *code < OWNED_EXIT_FLOOR {
                return ClaimState::Resolved; // conventional; see OWNED_EXIT_FLOOR
            }
            match owner_text.contains("_EXIT") || owner_text.contains("EXIT_") {
                true => ClaimState::Resolved,
                false => ClaimState::ExitCodeUnowned,
            }
        }
    }
}

/// Audit one doctrine table's strings. Returns only the FAILURES, so an empty vec is the
/// green verdict and the caller never has to filter.
pub fn audit_rows(rows: &[String], src: &Sources) -> Vec<(Claim, ClaimState)> {
    let mut bad = Vec::new();
    for row in rows {
        for c in scan(row) {
            let st = resolve(&c, src, row);
            if !st.ok() {
                bad.push((c, st));
            }
        }
    }
    bad
}

/// Tables under the HARD gate — every claim in these resolves, or the build is red.
///
/// APPEND-ONLY, AND THE RATCHET IS ITS LENGTH. A table joins when its claims resolve and
/// no table ever leaves; `the_ratchet_only_turns_one_way` pins the floor so a red build
/// can never be made green by shortening this list. Growing it is the work: run the
/// report lane, clear one module's claims, append its name, watch the count rise.
pub const VERIFIED_TABLES: &[&str] = &["MASSLOOP_MILESTONES", "MASSLOOP_EXECUTION_LAW"];

/// The floor the ratchet may never fall below.
pub const RATCHET_FLOOR: usize = 2;

/// Every doctrine row currently under the hard gate, as flat strings.
pub fn verified_rows() -> Vec<String> {
    use crate::oracle1_governor as og;
    let mut rows = Vec::new();
    for (m, verb, receipt, fail) in og::MASSLOOP_MILESTONES {
        rows.push(format!("{m} {verb} {receipt} {fail}"));
    }
    for (id, law, receipt) in og::MASSLOOP_EXECUTION_LAW {
        rows.push(format!("{id} {law} {receipt}"));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: DOCTRINE-CLAIM-GATE]
    /// THE NEGATIVE CONTROL, copied from `seams::tests::a_drained_anchor_goes_red`. A
    /// resolver that cannot fail is a green light nobody has tested — and a claim gate
    /// that silently passes everything is worse than none, because it certifies the
    /// verbatim ports it was built to stop.
    #[test]
    fn a_false_claim_goes_red() {
        let src = Sources::load();
        let bad = |c: Claim, text: &str| resolve(&c, &src, text);

        // A file nobody has ever written.
        assert_eq!(
            bad(Claim::FileLine { file: "no_such_file_zzz.rs".into(), line: 1 }, ""),
            ClaimState::FileAbsent
        );
        // A real file, a line far past its end.
        assert_eq!(
            bad(Claim::FileLine { file: "seams.rs".into(), line: 900_000 }, ""),
            ClaimState::LineAbsent
        );
        // A symbol nothing declares.
        assert_eq!(
            bad(Claim::Symbol { path: "forge_book::claims::NoSuchSymbolZZZ".into() }, ""),
            ClaimState::SymbolAbsent
        );
        // The 08-04 fault itself: an exit code quoted with no owning constant.
        assert_eq!(bad(Claim::ExitCode { code: 77 }, "77 = wrong lane"), ClaimState::ExitCodeUnowned);
        // …and the fixed form, which is what the ladder says today.
        assert!(bad(
            Claim::ExitCode { code: 77 },
            "77 = wrong lane (forge_studio::board_harvest::EXIT_WRONG_LANE)"
        )
        .ok());
    }

    // [BOARD: DOCTRINE-CLAIM-GATE]
    /// The scanner finds the three shapes and does not invent a fourth.
    #[test]
    fn the_scanner_reads_claims_and_not_prose() {
        let c = scan("see board_sync.rs:429 and forge_studio::massread::API_FILE_CEILING");
        assert!(c.contains(&Claim::FileLine { file: "board_sync.rs".into(), line: 429 }));
        assert!(c.iter().any(|x| matches!(x, Claim::Symbol { path } if path.ends_with("API_FILE_CEILING"))));

        // A path spelled with a directory still resolves on its leaf.
        let c = scan("crates/forge-book/src/seams.rs:28 owns it");
        assert!(c.contains(&Claim::FileLine { file: "seams.rs".into(), line: 28 }));

        // Ordinary prose carries no claim — a scanner that fires on English is noise.
        assert!(scan("a wave that produces no lowered bytes did not happen").is_empty());
        assert!(scan("reads are the free tier and must not spend paid capacity").is_empty());

        // AND NEITHER DOES ORDINARY RUST. The first cut matched any `a::b`, so the report
        // lane drowned in `AchievementTier::Bronze` — an enum variant is not a symbol
        // `defines` can find, and a gate whose report is noise is a gate nobody reads.
        assert!(scan("match t { AchievementTier::Bronze => 1, Rung::Proven => 2 }").is_empty());
        assert!(scan("let v = items.iter().collect::<Vec<_>>();").is_empty());
        assert!(scan("use super::*;").is_empty());
        // std is not in `crates/` — calling it absent is a lie about this repo.
        assert!(scan("use std::collections::BTreeMap; use core::fmt::Display;").is_empty());
        // A method call is not part of the symbol: `SYNTHESES.len()` names SYNTHESES.
        let c = scan("crate::latent_synthesis::SYNTHESES.len() rows");
        assert_eq!(c, vec![Claim::Symbol { path: "crate::latent_synthesis::SYNTHESES".into() }]);
    }

    // [BOARD: DOCTRINE-CLAIM-GATE]
    /// THE HARD GATE. Every claim in every table named by [`VERIFIED_TABLES`] resolves.
    #[test]
    fn every_verified_table_claim_resolves_on_disk() {
        let src = Sources::load();
        let bad = audit_rows(&verified_rows(), &src);
        assert!(
            bad.is_empty(),
            "doctrine claims that disk refuses ({} tables under gate):\n  {}",
            VERIFIED_TABLES.len(),
            bad.iter().map(|(c, s)| format!("{c:?} -> {s:?}")).collect::<Vec<_>>().join("\n  ")
        );
    }

    // [BOARD: DOCTRINE-CLAIM-GATE]
    /// The ratchet turns one way. Shortening `VERIFIED_TABLES` is how a red build gets
    /// quietly made green, so the floor is compiled and only ever raised by hand.
    #[test]
    fn the_ratchet_only_turns_one_way() {
        assert!(
            VERIFIED_TABLES.len() >= RATCHET_FLOOR,
            "the ratchet was unwound: {} tables < floor {RATCHET_FLOOR}",
            VERIFIED_TABLES.len()
        );
        let mut sorted = VERIFIED_TABLES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), VERIFIED_TABLES.len(), "a table is listed twice");
    }

    /// THE REPORT LANE — the 164-claim truth, on demand, blocking nothing.
    /// `cargo test -p forge-book -- --ignored --nocapture claim_report`
    #[test]
    #[ignore = "reporting lane; run with --ignored --nocapture for the full drift count"]
    fn claim_report() {
        let src = Sources::load();
        let sources = crate_sources(&crates_dir());
        let mut total = 0usize;
        let mut drifted = 0usize;
        for (path, text) in &sources {
            if !path.starts_with("forge-book/src/") {
                continue;
            }
            for c in scan(text) {
                total += 1;
                let st = resolve(&c, &src, text);
                if !st.ok() {
                    drifted += 1;
                    println!("{path}\t{c:?}\t{st:?}");
                }
            }
        }
        println!("CLAIMS {total} scanned · {drifted} drifted · {} tables under hard gate", VERIFIED_TABLES.len());
    }
}
