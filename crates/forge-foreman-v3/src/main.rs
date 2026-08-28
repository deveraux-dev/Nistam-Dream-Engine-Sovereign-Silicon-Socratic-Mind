//! `foreman` — the M2 loop's process. Two verbs:
//!
//! ```text
//! foreman run  --root F:\v3           # loop until the census drains
//! foreman run  --root F:\v3 --once    # move exactly one census row
//! foreman chat --root F:\v3           # ARCH000's direct line to the sidecar
//! ```
//!
//! `chat` exists because the sidecar has no other human door (ARCH000
//! 2026-08-09: "no direct line to gemma"). It is a REPL over the same
//! loopback frames the loop uses: type a prompt, get the reply, `STATUS`
//! passes through, `exit` leaves the sidecar running. `F:\v3\gemma.cmd`
//! wraps it in one memorable word.
//!
//! `--root` is required for the same reason the `tape` driver requires it: a
//! default would run the loop in whatever tree the process happened to start
//! in, and an orchestrator in the wrong airframe orchestrates the wrong build.
//!
//! Exit code 0 covers green, queued, and drained — a queued red is the fail
//! path *working* (the brief survives the night). Nonzero means the loop
//! itself is blocked: no census, sidecar down, tape refused.

use std::path::PathBuf;
use std::process::ExitCode;

use forge_foreman_v3::directives;
use forge_foreman_v3::gate;
use forge_foreman_v3::run::{run_all, run_once, Outcome};
mod distill;
mod witness;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("foreman: {e}");
            ExitCode::FAILURE
        }
    }
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verb = args.first().map(String::as_str);
    if verb != Some("run") && verb != Some("chat") && verb != Some("weld") && verb != Some("land")
        && verb != Some("distill") && verb != Some("route") && verb != Some("gate")
        && verb != Some("beat") && verb != Some("rolls") && verb != Some("dauer") && verb != Some("drift")
        && verb != Some("sidecar") && verb != Some("nde") && verb != Some("velocity") && verb != Some("witness")
        && verb != Some("hook") && verb != Some("oracle")
    {
        return Err("usage: foreman <run|chat|weld|land|distill|route|gate|beat|rolls|dauer|drift|sidecar|nde|velocity|witness|hook|oracle> --root <workspace> [--once] \
                    [weld: --dir <tree> --crate <name> | --resolve <name> --weld-file <path>] \
                    [land: --crate <name> --draft <file>] \
                    [gate: --crate <name> | --workspace] \
                    [beat: <PASS|FAIL> --green N --red N --unwired N] \
                    [rolls: [--day YYYY-MM-DD] [--all]] \
                    [dauer] \
                    [drift] \
                    [velocity: check|yield|unlock] \
                    [sidecar: up|down|status --root <workspace>] \
                    [nde: up|down|status --root <workspace>] \
                    [witness: <scenario>|--all [--tolerance N] [--bless]]"
            .into());
    }

    // These verbs don't require --root, handle them early
    if verb == Some("beat") {
        return beat_record(&args).map_err(|e| format!("[beat] {e}"));
    }
    if verb == Some("rolls") {
        return beat_rolls(&args).map_err(|e| format!("[rolls] {e}"));
    }
    if verb == Some("dauer") {
        beat_dauer(); // Never returns; exits directly via std::process::exit
    }
    if verb == Some("drift") {
        return beat_drift(&args).map_err(|e| format!("[drift] {e}"));
    }
    if verb == Some("velocity") {
        velocity_verb(&args); // Never returns; exits directly via std::process::exit
    }

    // All other verbs require --root
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let root = PathBuf::from(args.get(at + 1).ok_or("--root takes a directory")?);
    let once = args.iter().any(|a| a == "--once");

    if verb == Some("chat") {
        return chat(&root);
    }
    if verb == Some("sidecar") {
        return sidecar_verb(&root, &args);
    }
    if verb == Some("nde") {
        return nde_verb(&root, &args);
    }
    if verb == Some("weld") {
        return weld_verb(&root, &args);
    }
    if verb == Some("land") {
        return land_verb(&root, &args);
    }
    if verb == Some("distill") {
        return distill::run(&root, &args);
    }
    if verb == Some("route") {
        return distill::route(&root, &args);
    }
    if verb == Some("gate") {
        return gate::verb(&root, &args);
    }
    if verb == Some("hook") {
        return forge_foreman_v3::hook::verb(&root, &args);
    }
    if verb == Some("oracle") {
        return forge_foreman_v3::oracle::verb(&root, &args);
    }
    if verb == Some("witness") {
        return witness::verb(&root, &args);
    }

    let d = directives::load(&root)?;
    eprintln!("[foreman] root={} gate={:?} sidecar={}", root.display(), d.gate, d.endpoint);

    let outcomes =
        if once { vec![run_once(&root, &d)?] } else { run_all(&root, &d)? };
    for o in &outcomes {
        match o {
            Outcome::Green { name, committed } => {
                println!("GREEN   {name} — {} path(s) on the tape", committed.len());
                for p in committed {
                    println!("        {p}");
                }
            }
            Outcome::Queued { name } => {
                println!("QUEUED  {name} — brief at .forge/brief-queue/{name}/BRIEF.md");
            }
            Outcome::Drained => println!("DRAINED — census has no actionable row"),
        }
    }
    Ok(())
}

/// `foreman weld --root <workspace> --dir <tree> --crate <name>` — the
/// constrained repair lane (WAVE-WELD, live-fire form), now a thin arg-parse
/// over [`forge_foreman_v3::weld_lane::run_ladder`], which journals every
/// attempt as a W5 distill pair. Exit 0 only when the tree ends green.
///
/// `foreman weld --root <workspace> --resolve <name> --weld-file <path>` —
/// stamp a hand fix as the resolution of the crate's most recent journaled
/// session, retroactively labeling that session's red attempts.
fn weld_verb(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let d = directives::load(root)?;

    if let Some(at) = args.iter().position(|a| a == "--resolve") {
        let krate = args.get(at + 1).ok_or("--resolve takes a crate name")?;
        let wf_at = args
            .iter()
            .position(|a| a == "--weld-file")
            .ok_or("weld --resolve: --weld-file <path> is required (the landed weld's text)")?;
        let wf = args.get(wf_at + 1).ok_or("--weld-file takes a path")?;
        let text = std::fs::read_to_string(wf)
            .map_err(|e| format!("weld --resolve: cannot read {wf}: {e}"))?;
        let labeled = forge_foreman_v3::weld_lane::resolve(root, &d, krate, text.trim())?;
        println!("RESOLVED {krate} — resolution row stamped; {labeled} red attempt(s) now labeled");
        return Ok(());
    }

    let dir_at = args.iter().position(|a| a == "--dir").ok_or("weld: --dir <tree> is required")?;
    let dir = PathBuf::from(args.get(dir_at + 1).ok_or("--dir takes a directory")?);
    let crate_at =
        args.iter().position(|a| a == "--crate").ok_or("weld: --crate <name> is required")?;
    let krate = args.get(crate_at + 1).ok_or("--crate takes a name")?.clone();

    eprintln!("[foreman] weld lane: dir={} crate={krate} gate={:?}", dir.display(), d.gate);
    forge_foreman_v3::weld_lane::run_ladder(root, &dir, &krate, &d)
}

/// `foreman land --root <workspace> --crate <name> --draft <file>` — the
/// delegate lane (L11): land an externally produced draft through the exact
/// sidecar travel (parse → lint → stage → gate → promote → root gate →
/// stamped tape → census flip), truthfully stamped as LLM work.
fn land_verb(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let d = directives::load(root)?;
    let crate_at =
        args.iter().position(|a| a == "--crate").ok_or("land: --crate <name> is required")?;
    let name = args.get(crate_at + 1).ok_or("--crate takes a name")?;
    let draft_at =
        args.iter().position(|a| a == "--draft").ok_or("land: --draft <file> is required")?;
    let draft = PathBuf::from(args.get(draft_at + 1).ok_or("--draft takes a path")?);

    match forge_foreman_v3::run::land_external(root, &d, name, &draft)? {
        Outcome::Green { name, committed } => {
            println!("GREEN   {name} — {} path(s) on the tape (delegate lane)", committed.len());
            for p in committed {
                println!("        {p}");
            }
            Ok(())
        }
        other => Err(format!("unexpected delegate outcome: {other:?}")),
    }
}

/// The human REPL over the sidecar's loopback frames. One prompt per line;
/// `STATUS` passes through; `exit` (or EOF) leaves without touching the
/// sidecar; `SHUTDOWN` is deliberately NOT forwarded — standing the sidecar
/// down is the operator's call at the process level, not a chat typo.
fn chat(root: &PathBuf) -> Result<(), String> {
    use std::io::{BufRead, Write};
    let d = directives::load(root)?;
    let sidecar = forge_foreman_v3::client::Sidecar::at(&d.endpoint)?;
    let status = sidecar.status().map_err(|e| format!("sidecar is not up: {e}"))?;
    println!("[gemma] {status}");
    println!("[gemma] type a prompt and press enter; 'exit' to leave (sidecar stays up)");

    let stdin = std::io::stdin();
    loop {
        print!("gemma> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Ok(()); // EOF
        }
        let line = line.trim();
        match line {
            "" => continue,
            "exit" | "quit" => return Ok(()),
            "SHUTDOWN" => {
                println!("[gemma] not forwarded — stop the sidecar process deliberately, not from chat");
            }
            "STATUS" => println!("{}", sidecar.status()?),
            prompt => match sidecar.infer(prompt) {
                Ok(reply) => println!("{reply}"),
                Err(e) => eprintln!("[gemma] {e}"),
            },
        }
    }
}

/// `foreman sidecar up|down|status --root <workspace>` — the lifecycle leg
/// named in `HANDOFF-2026-08-12-GEMMA-SIDECAR-FIX.md` Step 5: `up` launches
/// the sidecar in a visible console and writes a PID beacon, `down` stops
/// the beaconed process, `status` reports the beacon plus a live probe.
fn sidecar_verb(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let sub = args.get(1).map(String::as_str);
    if sub == Some("down") {
        return forge_foreman_v3::sidecar_launch::down(root);
    }
    let d = directives::load(root)?;
    match sub {
        Some("up") => forge_foreman_v3::sidecar_launch::up(root, &d),
        Some("status") => forge_foreman_v3::sidecar_launch::status(root, &d),
        _ => Err("usage: foreman sidecar <up|down|status> --root <workspace>".into()),
    }
}

/// `foreman nde up|down|status --root <workspace>` — the NDE sidecar lifecycle
/// leg (BACKLOG STEP 3): `up` launches the nde-sidecar hidden and writes a
/// PID beacon, `down` stops the beaconed process, `status` reports the beacon
/// plus a live probe.
fn nde_verb(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let sub = args.get(1).map(String::as_str);
    if sub == Some("down") {
        return forge_foreman_v3::sidecar_launch::nde_down(root);
    }
    let d = directives::load(root)?;
    match sub {
        Some("up") => forge_foreman_v3::sidecar_launch::nde_up(root, &d),
        Some("status") => forge_foreman_v3::sidecar_launch::nde_status(root, &d),
        _ => Err("usage: foreman nde <up|down|status> --root <workspace>".into()),
    }
}

/// `foreman beat <PASS|FAIL> --green N --red N --unwired N` — record a beat,
/// calculate quality, and print rank/streak.
fn beat_record(args: &[String]) -> Result<(), String> {
    use forge_foreman_v3::beat_status;

    let verdict = args.get(1).map(|s| s.as_str()).unwrap_or("");
    if verdict != "PASS" && verdict != "FAIL" {
        return Err("beat: first arg must be PASS or FAIL".into());
    }

    let mut green = 0i64;
    let mut red = 0i64;
    let mut unwired = 0i64;

    let mut it = args.iter().skip(2);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--green" => {
                green = it.next().and_then(|s| s.parse().ok()).ok_or("--green requires a number")?;
            }
            "--red" => {
                red = it.next().and_then(|s| s.parse().ok()).ok_or("--red requires a number")?;
            }
            "--unwired" => {
                unwired = it.next().and_then(|s| s.parse().ok()).ok_or("--unwired requires a number")?;
            }
            _ => {}
        }
    }

    let quality = forge_foreman_v3::flywheel_beat::beat_quality(verdict, green, red, unwired);

    // Read current status (or default if absent)
    let status_path = PathBuf::from(format!("{}/.forge/foreman/beat-status.ron", "."));
    let mut status = beat_status::read_status(&status_path);

    // Update streak and verdict
    if verdict == "PASS" {
        status.streak = 0;
        status.last_verdict_is_pass = true;
    } else {
        status.streak += 1;
        status.last_verdict_is_pass = false;
    }

    // Update beats and quality
    status.beats_total += 1;
    status.quality_last = quality;

    // Write status back (refuse whole on error loud)
    beat_status::write_status(&status_path, status)?;

    // Append to progression ledger (simple JSONL format)
    let prog_path = PathBuf::from(format!("{}/.forge/foreman/progression.jsonl", "."));
    if let Some(parent) = prog_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = serde_json::json!({
        "tick": now_unix,
        "verdict": verdict,
        "quality": quality,
        "green": green,
        "red": red,
        "unwired": unwired,
    });
    let line = format!("{}\n", event.to_string());
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&prog_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
        .map_err(|e| format!("cannot append to progression.jsonl: {}", e))?;

    // Print output with rank word and streak
    let rank = status.rank_word();
    println!(
        "BEAT(verdict:{},quality:{}/10000,rank:{},streak:{})",
        verdict, quality, rank, status.streak
    );
    Ok(())
}

/// `foreman rolls [--day YYYY-MM-DD] [--all] [--verbose]` — report TokenRoll + OrientRoll
/// over transcripts. Looks for transcripts in FORGE_TRANSCRIPT_ROOT env var,
/// defaulting to ~/.claude/projects/. Walks project slug directories for .jsonl files.
fn beat_rolls(args: &[String]) -> Result<(), String> {
    let mut day = String::new();
    let mut all = false;
    let mut verbose = false;

    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--day" => {
                day = it.next().cloned().unwrap_or_default();
            }
            "--all" => all = true,
            "--verbose" => verbose = true,
            _ => {}
        }
    }

    // Get transcript root from env or use default
    let transcript_root = match std::env::var("FORGE_TRANSCRIPT_ROOT") {
        Ok(path) => {
            if verbose {
                eprintln!("[rolls] FORGE_TRANSCRIPT_ROOT={}", path);
            }
            PathBuf::from(path)
        }
        Err(_) => {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .map_err(|_| "cannot locate home directory")?;
            let default_root = PathBuf::from(home).join(".claude/projects");
            if verbose {
                eprintln!(
                    "[rolls] FORGE_TRANSCRIPT_ROOT not set, using default: {}",
                    default_root.display()
                );
            }
            default_root
        }
    };

    if verbose {
        eprintln!("[rolls] scanning: {}", transcript_root.display());
    }

    let Ok(entries) = std::fs::read_dir(&transcript_root) else {
        if verbose {
            eprintln!("[rolls] directory not found: {}", transcript_root.display());
        }
        // Directory absent — print zeroed and exit 0
        println!("{}", forge_foreman_v3::rolls::TokenRoll::default().render("absent"));
        println!("{}", forge_foreman_v3::rolls::OrientRoll::default().render("absent"));
        return Ok(());
    };

    // Walk entries and collect .jsonl files from project directories
    let mut files: Vec<(String, String)> = Vec::new(); // (path_display, content)

    for entry_result in entries {
        let Ok(entry) = entry_result else {
            continue;
        };
        let path = entry.path();

        // Check if this is a .jsonl file directly in transcript root
        if path.extension().is_some_and(|x| x == "jsonl") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if verbose {
                    eprintln!("[rolls] read: {}", path.display());
                }
                files.push((path.display().to_string(), content));
            }
            continue;
        }

        // Check if this is a directory (project slug), look for .jsonl files inside
        if path.is_dir() {
            if let Ok(dir_entries) = std::fs::read_dir(&path) {
                for dir_entry_result in dir_entries {
                    let Ok(dir_entry) = dir_entry_result else {
                        continue;
                    };
                    let file_path = dir_entry.path();
                    if file_path.extension().is_some_and(|x| x == "jsonl") {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            if verbose {
                                eprintln!("[rolls] read: {}", file_path.display());
                            }
                            files.push((file_path.display().to_string(), content));
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        if verbose {
            eprintln!("[rolls] no .jsonl files found");
        }
        // No transcripts — print zeroed and exit 0
        println!("{}", forge_foreman_v3::rolls::TokenRoll::default().render("absent"));
        println!("{}", forge_foreman_v3::rolls::OrientRoll::default().render("absent"));
        return Ok(());
    }

    if verbose {
        eprintln!("[rolls] found {} transcript file(s)", files.len());
    }

    if all {
        day.clear();
    } else if day.is_empty() {
        day = files
            .iter()
            .filter_map(|(_, content)| forge_foreman_v3::rolls::newest_day(content))
            .max()
            .unwrap_or_default();
        if verbose {
            eprintln!("[rolls] auto-selected day: {}", if day.is_empty() { "all" } else { &day });
        }
    }

    let mut token_roll = forge_foreman_v3::rolls::TokenRoll::default();
    let mut orient_roll = forge_foreman_v3::rolls::OrientRoll::default();

    for (path_display, content) in &files {
        token_roll.fold(&forge_foreman_v3::rolls::roll_jsonl(content, &day));
        orient_roll.fold(&forge_foreman_v3::rolls::roll_orient(content, &day));
        if verbose {
            let tokens = forge_foreman_v3::rolls::roll_jsonl(content, &day);
            eprintln!(
                "[rolls] {} — {} calls, {} billable units",
                path_display,
                tokens.calls,
                tokens.billable_units()
            );
        }
    }

    println!("{}", token_roll.render(if day.is_empty() { "all" } else { &day }));
    println!("{}", orient_roll.render(if day.is_empty() { "all" } else { &day }));

    Ok(())
}

/// `foreman dauer` — exit 0 if Active, 2 if Dauer survival mode.
/// Reads the most recent beat status (assumed in a standard location).
fn beat_dauer() -> ! {
    use forge_foreman_v3::beat_status;
    use forge_foreman_v3::dauer::{dauer_state, DauerState, DAUER_THRESHOLD};

    let status_path = PathBuf::from(format!("{}/.forge/foreman/beat-status.ron", "."));
    let status = beat_status::read_status(&status_path);

    match dauer_state(status.streak) {
        DauerState::Active => {
            eprintln!("[dauer] ACTIVE (streak={})", status.streak);
            std::process::exit(0);
        }
        DauerState::Dauer { streak } => {
            eprintln!(
                "[dauer] DAUER (streak={}) — survival mode active (threshold={})",
                streak, DAUER_THRESHOLD
            );
            std::process::exit(2);
        }
    }
}

/// `foreman velocity <check|yield|unlock>` — L21 diff-floor circuit breaker.
/// `check` is the `PreToolUse` hook body (exit 0 allow / 1 deny); `yield` is
/// model-issued (locks); `unlock` is `UserPromptSubmit`-issued ONLY (clears the
/// lock). See `forge_foreman_v3::velocity` module doc for the full contract.
fn velocity_verb(args: &[String]) -> ! {
    let sub = args.get(1).map(String::as_str);
    match sub {
        Some("check") => std::process::exit(forge_foreman_v3::velocity::check(".")),
        Some("yield") => match forge_foreman_v3::velocity::yield_now(".") {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("[velocity] {e}");
                std::process::exit(1);
            }
        },
        Some("unlock") => match forge_foreman_v3::velocity::unlock(".") {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("[velocity] {e}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("usage: foreman velocity <check|yield|unlock>");
            std::process::exit(1);
        }
    }
}

/// `foreman drift` — audit hook wiring against expected configuration.
/// Reports MISSING (expected but not wired) and EXTRA (wired but unexpected).
fn beat_drift(args: &[String]) -> Result<(), String> {
    // --root is optional here (unlike every other verb) only because this
    // runs on every UserPromptSubmit and must never hard-fail a turn on a
    // missing flag; honor it when given instead of always reading cwd, since
    // cwd drifts under a persistent shell session (receipt: FAIL fired after
    // `cd F:\v3\web`, PASS from F:\v3, same binary, 2026-08-20).
    let root = args
        .iter()
        .position(|a| a == "--root")
        .and_then(|at| args.get(at + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Report assembly (disk-wiring read, expected-hooks load, drift detect,
    // staleness precommit, verdict) lives in `drift::run_report` — shared
    // with `forge-daemon-door`'s `hook_drift` verb (2026-08-21) so the daemon
    // path and this direct-binary path can never silently diverge.
    println!("{}", forge_foreman_v3::drift::run_report(&root));

    Ok(())
}
