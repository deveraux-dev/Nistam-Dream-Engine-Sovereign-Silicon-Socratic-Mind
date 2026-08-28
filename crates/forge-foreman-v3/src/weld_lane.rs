//! The constrained repair ladder (WAVE-WELD, live-fire form), journaled by
//! the W5 flywheel.
//!
//! Moved here from `main.rs` when W5 landed: the ladder must be drivable by a
//! fixture (fake sidecar, scripted gate) to prove its journal rows, and a
//! binary's `fn` cannot be. `foreman weld` is now a thin arg-parse over
//! [`run_ladder`] and [`resolve`].
//!
//! Every attempt journals one [`crate::flywheel::WeldPair`] row BEFORE the
//! ladder moves on; a row that cannot be written stops the ladder loudly
//! (W5: an unjournaled attempt is a wasted failure). A green attempt also
//! stamps the session's resolution row, which retroactively labels the reds.

use std::path::Path;

use crate::directives::Directives;
use crate::flywheel::{self, Verdict};
use crate::{gate, sip, weld};

/// How much of a red gate's output rides into a journal row's `gate_tail` —
/// the same evidence budget as the retry prompt (`run.rs`).
const GATE_TAIL_CAP: usize = 6_000;

/// The last `cap` bytes of gate output, on a char boundary.
fn tail(s: &str, cap: usize) -> String {
    if s.len() <= cap {
        return s.to_string();
    }
    let mut at = s.len() - cap;
    while !s.is_char_boundary(at) {
        at += 1;
    }
    s[at..].to_string()
}

/// Run the weld ladder against `dir` for `krate`. Gate first — a green tree
/// is reported and left alone. On red: sip, INFER_WELD, re-parse, apply,
/// re-gate; a still-red weld rolls back. `Ok(())` only when the tree ends
/// green; every attempt is journaled either way.
pub fn run_ladder(root: &Path, dir: &Path, krate: &str, d: &Directives) -> Result<(), String> {
    if !d.journal_enabled {
        return Err("weld: flywheel.journal_enabled is false — refusing to grind dark; \
                    every attempt is a training row and W5 exists so none is wasted"
            .into());
    }
    let jpath = flywheel::journal_path(root, d);

    let before = gate::run(&d.gate, krate, dir)?;
    if before.green {
        println!("GREEN   {krate} — nothing to weld");
        return Ok(());
    }

    let base_prompt =
        sip::build_weld_prompt(dir, &before.output, d.sip_cap_bytes, d.sip_anchor_context_lines)?;
    eprintln!("[foreman] sipped prompt: {} bytes (cap {})", base_prompt.len(), d.sip_cap_bytes);
    let session = flywheel::session_of(&base_prompt);

    let sc = crate::client::Sidecar::at(&d.endpoint)?.with_timeout_s(d.reply_timeout_s);

    // The retry ladder (pipeline.on_red's budget, applied to welds): a failed
    // attempt's evidence is APPENDED to the prompt — greedy decoding is
    // deterministic, so an unchanged prompt would fail identically forever.
    let mut feedback = String::new();
    for attempt in 1..=d.retry_max {
        let prompt = format!("{base_prompt}{feedback}");
        if prompt.len() > d.sip_cap_bytes {
            return Err(format!(
                "weld: prompt with attempt-{attempt} feedback is {} bytes, cap {} — refused",
                prompt.len(),
                d.sip_cap_bytes
            ));
        }
        // A sidecar refusal (ERR frame, dead socket) joins the ladder as
        // feedback like any other failed attempt — only the ladder's end is
        // fatal.
        let reply = match sc.infer_weld(&prompt) {
            Ok(r) => r,
            Err(e) => {
                flywheel::append(
                    &jpath,
                    &flywheel::attempt_row(
                        session,
                        attempt,
                        krate,
                        &prompt,
                        "",
                        Verdict::EngineRefused,
                        &e,
                    ),
                )?;
                eprintln!("[foreman] attempt {attempt} refused: {e}");
                feedback = format!(
                    "\nATTEMPT {attempt} FAILED: the engine refused ({e}). Reply with a \
                     simpler, shorter weld.\n"
                );
                continue;
            }
        };
        eprintln!("[foreman] attempt {attempt} weld reply ({} bytes): {reply}", reply.len());

        let outcome: Result<Vec<weld::Planned>, String> = weld::parse(&reply)
            .map_err(|e| format!("did not parse: {e}"))
            .and_then(|w| {
                // A no-op weld (payload == anchor on replace) can never turn a
                // red green — refuse it before spending an apply+gate cycle.
                for f in &w.files {
                    for e in &f.edits {
                        if e.op == weld::Op::Replace && e.payload == e.anchor {
                            return Err(format!(
                                "no-op: payload equals anchor {:?} — nothing would change",
                                e.anchor
                            ));
                        }
                    }
                }
                weld::plan(&w, dir).map_err(|e| format!("did not plan: {e}"))
            });
        match outcome {
            Ok(plan) => {
                let mut gate_out = String::new();
                let stuck = weld::commit_gated(&plan, dir, || match gate::run(&d.gate, krate, dir)
                {
                    Ok(v) => {
                        gate_out = tail(&v.output, GATE_TAIL_CAP);
                        v.green
                    }
                    Err(e) => {
                        gate_out = e;
                        false
                    }
                })
                .map_err(|e| format!("weld apply failed: {e}"))?;
                if stuck {
                    flywheel::append(
                        &jpath,
                        &flywheel::attempt_row(
                            session,
                            attempt,
                            krate,
                            &prompt,
                            &reply,
                            Verdict::Green,
                            "",
                        ),
                    )?;
                    flywheel::append(&jpath, &flywheel::resolution(session, krate, &reply))?;
                    for p in &plan {
                        println!("WELDED  {}", p.path);
                    }
                    println!("GREEN   {krate} — weld landed on attempt {attempt}, gate green");
                    return Ok(());
                }
                flywheel::append(
                    &jpath,
                    &flywheel::attempt_row(
                        session,
                        attempt,
                        krate,
                        &prompt,
                        &reply,
                        Verdict::GateRed,
                        &gate_out,
                    ),
                )?;
                // REPLACE the feedback (never accumulate): live-fire
                // 2026-08-09, cumulative feedback overflowed the sip cap on
                // attempt 3 — the ladder must stay inside the same window.
                feedback = format!(
                    "\nATTEMPT {attempt} FAILED: your weld `{reply}` was applied and the gate \
                     STILL failed; the bytes were rolled back. That payload is WRONG — reply \
                     with a DIFFERENT weld.\n"
                );
            }
            Err(e) => {
                flywheel::append(
                    &jpath,
                    &flywheel::attempt_row(
                        session,
                        attempt,
                        krate,
                        &prompt,
                        &reply,
                        Verdict::ParseRefused,
                        &e,
                    ),
                )?;
                feedback = format!(
                    "\nATTEMPT {attempt} FAILED: your weld `{reply}` was refused ({e}). Reply \
                     with a corrected weld.\n"
                );
            }
        }
    }
    Err(format!(
        "weld: {} attempt(s) spent, gate still red — tree left at pre-weld bytes",
        d.retry_max
    ))
}

/// Stamp a hand fix as the resolution of a crate's most recent journaled
/// session (`foreman weld --resolve`). The weld text is re-validated through
/// [`weld::parse`] — defense in depth, same as the receive side — and the
/// return value is how many red attempts the resolution just labeled.
pub fn resolve(root: &Path, d: &Directives, krate: &str, weld_text: &str) -> Result<usize, String> {
    if !d.journal_enabled {
        return Err("resolve: flywheel.journal_enabled is false — the journal is the whole \
                    point of a resolution; enable it or state the ruling aloud"
            .into());
    }
    weld::parse(weld_text).map_err(|e| format!("resolve: weld did not parse ({e}) — a \
        resolution row must carry a valid weld, it is the training target"))?;
    let jpath = flywheel::journal_path(root, d);
    let mut rows =
        flywheel::load(&jpath).map_err(|e| format!("resolve: {e} — no journal, nothing to resolve"))?;
    let session = flywheel::latest_session_for(&rows, krate)
        .ok_or_else(|| format!("resolve: journal has no session for {krate:?} — nothing to resolve"))?;
    flywheel::append(&jpath, &flywheel::resolution(session, krate, weld_text))?;
    rows.push(flywheel::resolution(session, krate, weld_text));
    Ok(flywheel::derived_pairs(&rows, session).len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;

    /// The weld the fake sidecar answers with — parses, plans against the
    /// fixture file, and its anchor occurs exactly once.
    const WELD_REPLY: &str = "Weld(lane:\"repair\",files:[F(p:\"src/lib.rs\",edits:[E(anchor:\"speling(\",op:\"replace\",payload:\"spelling(\")])],gate:\"\",receipt:\"\")";

    /// A fake sidecar answering every framed request with `reply`, forever —
    /// the run.rs test pattern (one frame per connection).
    fn fake_sidecar(reply: &'static str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { continue };
                let mut len = [0u8; 4];
                if s.read_exact(&mut len).is_err() {
                    continue;
                }
                let mut buf = vec![0u8; u32::from_be_bytes(len) as usize];
                if s.read_exact(&mut buf).is_err() {
                    continue;
                }
                let _ = s.write_all(&(reply.len() as u32).to_be_bytes());
                let _ = s.write_all(reply.as_bytes());
                let _ = s.flush();
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        });
        addr
    }

    fn directives(endpoint: String, gate: &str) -> Directives {
        Directives {
            endpoint,
            nde_endpoint: "127.0.0.1:13018".into(),
            gate: gate.into(),
            retry_max: 3,
            reply_timeout_s: 600,
            sip_cap_bytes: 4096,
            sip_anchor_context_lines: 2,
            retry_temp_pmy: 1500,
            retry_top_p_pmy: 9000,
            weld_pairs_path: ".forge/distill/weld-pairs.ndjson".into(),
            bqr_path: ".forge/distill/router.bqr".into(),
            journal_enabled: true,
        }
    }

    /// A scratch tree: the broken source and a counting gate script that goes
    /// red for `reds` runs (each red naming a real `--> file:line` site so
    /// the sip has something to slice), then green forever.
    fn fixture(tag: &str, reds: usize) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("weld-lane-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("lib.rs"), "fn a() { speling(); }\n").unwrap();

        let mut script = String::from("@echo off\n");
        script.push_str(&format!("if exist tick{reds} exit /b 0\n"));
        script.push_str("echo error[E0425]: cannot find value\necho  --^> src/lib.rs:1:10\n");
        for n in (2..=reds).rev() {
            script.push_str(&format!("if exist tick{} goto t{n}\n", n - 1));
        }
        script.push_str("echo.>tick1\nexit /b 1\n");
        for n in 2..=reds {
            script.push_str(&format!(":t{n}\necho.>tick{n}\nexit /b 1\n"));
        }
        std::fs::write(dir.join("gate.cmd"), script).unwrap();
        dir
    }

    fn journal(dir: &Path, d: &Directives) -> Vec<flywheel::WeldPair> {
        flywheel::load(&flywheel::journal_path(dir, d)).unwrap()
    }

    /// The W5 proof-plan fixture: a ladder that goes red-red-green writes
    /// exactly 3 attempt rows + 1 resolution row, and the derived pairs are
    /// exactly 2, both targeting the green weld.
    #[test]
    fn a_red_red_green_ladder_journals_every_attempt_and_its_resolution() {
        let dir = fixture("rrg", 3); // initial gate + 2 red attempts, then green
        let d = directives(fake_sidecar(WELD_REPLY), "cmd /c .\\gate.cmd");

        run_ladder(&dir, &dir, "weldtest", &d).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap(),
            "fn a() { spelling(); }\n",
            "the green attempt's bytes stuck"
        );

        let rows = journal(&dir, &d);
        assert_eq!(rows.len(), 4, "3 attempt rows + 1 resolution row: {rows:#?}");
        assert_eq!(
            rows.iter().map(|r| (r.attempt, r.verdict)).collect::<Vec<_>>(),
            vec![
                (1, Verdict::GateRed),
                (2, Verdict::GateRed),
                (3, Verdict::Green),
                (0, Verdict::Green),
            ]
        );
        let session = rows[0].session;
        assert!(rows.iter().all(|r| r.session == session), "one ladder, one session");
        assert!(rows.iter().all(|r| r.crate_name == "weldtest"));
        assert!(rows[0].gate_tail.contains("E0425"), "the red's evidence rides the row");
        assert!(rows[1].prompt.contains("ATTEMPT 1 FAILED"), "feedback is in the journaled prompt");
        assert_eq!(rows[3].weld, WELD_REPLY, "the resolution carries the green weld");

        let pairs = flywheel::derived_pairs(&rows, session);
        assert_eq!(pairs.len(), 2, "both reds labeled, the green attempt is not double-counted");
        assert!(pairs.iter().all(|(_, w)| w == WELD_REPLY));
        assert_ne!(pairs[0].0, pairs[1].0, "the two reds saw different prompts");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The kill switch refuses loudly BEFORE any gate or INFER runs — a dark
    /// flywheel is never a silent skip.
    #[test]
    fn a_disabled_journal_refuses_the_ladder_before_any_work() {
        let dir = std::env::temp_dir().join(format!("weld-lane-off-{}", std::process::id()));
        let mut d = directives("127.0.0.1:1".into(), "no-such-gate");
        d.journal_enabled = false;
        // No fixture, no sidecar, an unspawnable gate: refusing first means
        // none of those are ever touched.
        let e = run_ladder(&dir, &dir, "weldtest", &d).unwrap_err();
        assert!(e.contains("journal_enabled"), "the refusal names the key: {e}");
        let e = resolve(&dir, &d, "weldtest", WELD_REPLY).unwrap_err();
        assert!(e.contains("journal_enabled"), "resolve refuses too: {e}");
    }

    /// An unwritable journal stops the ladder loudly at the first row — it
    /// does not proceed dark (W5 proof plan, sabotage row (b) as a test).
    #[test]
    fn an_unwritable_journal_halts_the_ladder_loudly() {
        let dir = fixture("blocked", 9);
        std::fs::write(dir.join("blocker"), "a file where a dir must go").unwrap();
        let mut d = directives(fake_sidecar("ERR busy"), "cmd /c .\\gate.cmd");
        d.weld_pairs_path = "blocker/weld-pairs.ndjson".into();

        let e = run_ladder(&dir, &dir, "weldtest", &d).unwrap_err();
        assert!(e.contains("flywheel"), "the halt names the flywheel, not a vague io: {e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A hand fix enters through `--resolve`: it stamps the latest session's
    /// resolution row and reports how many reds it just labeled.
    #[test]
    fn a_hand_fix_resolves_the_latest_session_and_labels_its_reds() {
        let dir = std::env::temp_dir().join(format!("weld-lane-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = directives("127.0.0.1:1".into(), "unused");
        let jpath = flywheel::journal_path(&dir, &d);
        let s = flywheel::session_of("the queued red");
        flywheel::append(&jpath, &flywheel::attempt_row(s, 1, "weldtest", "p1", "w1", Verdict::GateRed, "red")).unwrap();
        flywheel::append(&jpath, &flywheel::attempt_row(s, 2, "weldtest", "p2", "w2", Verdict::ParseRefused, "no")).unwrap();
        flywheel::append(&jpath, &flywheel::attempt_row(77, 1, "other-crate", "p", "w", Verdict::GateRed, "red")).unwrap();

        let labeled = resolve(&dir, &d, "weldtest", WELD_REPLY).unwrap();
        assert_eq!(labeled, 2, "both reds of weldtest's session are now labeled");
        let rows = journal(&dir, &d);
        let res = rows.last().unwrap();
        assert_eq!((res.attempt, res.session, res.weld.as_str()), (0, s, WELD_REPLY));

        // A crate the journal has never seen has nothing to resolve.
        let e = resolve(&dir, &d, "phantom", WELD_REPLY).unwrap_err();
        assert!(e.contains("no session"), "{e}");
        // A resolution must carry a valid weld — it is the training target.
        let e = resolve(&dir, &d, "weldtest", "not a weld").unwrap_err();
        assert!(e.contains("did not parse"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The moved ladder still refuses a non-loopback endpoint via the client —
    /// pinning that the move changed homes, not behavior.
    #[test]
    fn the_ladder_still_speaks_only_loopback() {
        let dir = fixture("loop", 9);
        let d = directives("10.0.0.7:13017".into(), "cmd /c .\\gate.cmd");
        let e = run_ladder(&dir, &dir, "weldtest", &d).unwrap_err();
        assert!(e.contains("loopback"), "{e}");
        let _ = client::Sidecar::at("127.0.0.1:1");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
