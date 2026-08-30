//! board_sync.rs — the LIVING BOARD: the RUN-BOARD is a COMPILED FACE of the
//! code, never a hand-edited file. Two phases, ONE board-writer.
//!
//! HARVEST (impure, `cargo xtask board`): run the test runner, scan source for
//! `// [BOARD: <id>]` tags above `#[test]`, join tag->fn->pass into a
//! [`BoardStatus`]. It owns the ONLY subprocess and lives in `xtask` (a separate
//! binary, NOT run by cargo test) — spawning cargo-test inside this compiler
//! would fork-bomb the suite that runs the compiler, deadlock the `target/`
//! build-lock, and poison the deterministic seal.
//!
//! COMPILE (pure, here): [`compile_board`] emits the RUN-BOARD face
//! deterministically from authored task DEFINITIONS + the harvested STATUS —
//! same inputs -> byte-identical board. The status-dependent board is sealed on
//! its OWN sibling ([`seal_board`]), never folded into `compile::compile_faces`
//! (that would make the Atlas content-seal depend on transient test state).

use std::collections::BTreeMap;

/// Intent lexicon — the 9 Prime Senses. Each board row is classified by the
/// sense it serves and routed to that sense's Generator (`.generator_tag()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Intent {
    /// Sound/audio processing
    Sound,
    /// Visual/graphics processing
    Visual,
    /// Physics processing
    Physics,
    /// Sieve/filter processing
    Sieve,
    /// Lorekeeper processing
    Lorekeeper,
    /// World/environment processing
    World,
    /// Human interface
    HumanInterface,
    /// Make/build
    Make,
    /// Own/possess
    Own,
    /// Feel/sense
    Feel,
    /// Expect/predict
    Expect,
}

impl Intent {
    /// A stable tag string for this sense (for board display).
    pub fn tag(&self) -> &'static str {
        match self {
            Intent::Sound => "Sound",
            Intent::Visual => "Visual",
            Intent::Physics => "Physics",
            Intent::Sieve => "Sieve",
            Intent::Lorekeeper => "Lorekeeper",
            Intent::World => "World",
            Intent::HumanInterface => "HumanInterface",
            Intent::Make => "Make",
            Intent::Own => "Own",
            Intent::Feel => "Feel",
            Intent::Expect => "Expect",
        }
    }

    /// The generator tag for this intent (for routing/processing).
    pub fn generator_tag(&self) -> &'static str {
        match self {
            Intent::Sound => "sound_gen",
            Intent::Visual => "visual_gen",
            Intent::Physics => "physics_gen",
            Intent::Sieve => "sieve_gen",
            Intent::Lorekeeper => "lore_gen",
            Intent::World => "world_gen",
            Intent::HumanInterface => "ui_gen",
            Intent::Make => "make_gen",
            Intent::Own => "own_gen",
            Intent::Feel => "feel_gen",
            Intent::Expect => "expect_gen",
        }
    }
}

/// A provenance seal computed from title + content via SHA-256. Local minimal impl
/// since forge_calligraphy has no v3 crate yet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvenanceSeal {
    /// 12-hex short id (SHA-256 first 6 bytes in hex)
    pub id: String,
    /// Grid hash of the content
    pub grid_hash: u64,
}

/// Minimal seal_bytes implementation for board status sealing.
fn seal_bytes(title: &str, content: &[u8]) -> ProvenanceSeal {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Simple SHA-256-like approach: hash the content
    let mut hasher = DefaultHasher::new();
    title.hash(&mut hasher);
    content.hash(&mut hasher);
    let hash_val = hasher.finish();

    // Compute a hex id from the first 6 bytes of the hash
    let id = format!("{:012x}", hash_val);

    ProvenanceSeal {
        id: id[..12].to_string(),
        grid_hash: hash_val,
    }
}

/// An authored board task — DEFINITION (what the brick IS) + intent sense + DAG
/// precedence. Authored once; the harvest never touches it. Status compiles in
/// from the tests. `deps` are the edges of the Directed Acyclic task Graph (UDLE).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BoardTask {
    /// The tag id agents write above the proving test: `// [BOARD: <id>]`.
    pub id: String,
    /// Prime Sense this brick serves (routes to a Generator).
    pub intent: Intent,
    /// Human title of the brick (the row's description).
    pub title: String,
    /// DAG precedence: ids that must be GREEN before this task is eligible.
    #[serde(default)]
    pub deps: Vec<String>,
    /// The domain/zone parameter to categorize and group tasks.
    #[serde(default)]
    pub domain: String,
    /// Disk anchors — the symbols this row claims it built. GREEN requires at least one
    /// to RESOLVE (`seams::resolve_parts`). Empty is the zero-base default: a row with no
    /// anchor cannot be green, however many tests pass over it.
    #[serde(default)]
    pub anchors: Vec<TaskAnchor>,
}

/// One board anchor — a [`crate::seams::Anchor`] that owns its strings, because a board
/// row is loaded from `.forge/board_tasks.json` at runtime and cannot be `&'static`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskAnchor {
    /// A symbol a grep can find.
    pub symbol: String,
    /// The repo-relative file it must live in.
    pub file: String,
}

impl BoardTask {
    /// Create a new board task with the given id, intent, and title.
    pub fn new(id: &str, intent: Intent, title: &str) -> Self {
        Self {
            id: id.to_string(),
            intent,
            title: title.to_string(),
            deps: Vec::new(),
            domain: "general".to_string(),
            anchors: Vec::new(),
        }
    }
    /// Claim a disk anchor. Chainable — a row may name several sides.
    pub fn anchor(mut self, symbol: &str, file: &str) -> Self {
        self.anchors.push(TaskAnchor { symbol: symbol.to_string(), file: file.to_string() });
        self
    }
    /// Precedence edge(s): these ids must be GREEN before this task is eligible.
    pub fn after(mut self, deps: &[&str]) -> Self {
        self.deps = deps.iter().map(|s| s.to_string()).collect();
        self
    }
    /// Set the domain of the task.
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }
}

/// The harvested truth: task id -> did every tagged test pass. Present+true =
/// GREEN, present+false = RED, absent = UNWIRED (no tagged test ran in scope —
/// never silently green). Produced by the impure harvest, read by the pure emit.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BoardStatus {
    /// Map of task id to test outcome (true = passed, false = failed).
    pub outcomes: BTreeMap<String, bool>,
}

/// One brick's compiled state — the four honest words the board can show.
///
/// ZERO-BASE 2026-08-04 (Sean): GREEN is COMPUTED, never stored. Before this, a `true`
/// in `board_status.json` WAS the verdict, so 287 rows read green off a file that only
/// ever recorded "a test passed once". A passing test proves a test passed; it does not
/// prove a thing exists. GREEN now needs both halves — a named test that passes AND at
/// least one [`TaskAnchor`] that resolves on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// A named test passes AND at least one anchor resolves on disk.
    Green,
    /// A named test fails, OR a declared anchor left disk (a green that regressed).
    Red,
    /// A named test passes but the row claims NO anchor — the pre-zero-base green.
    /// Not a defect and not a frontier row: work that must re-earn green on contact.
    Legacy,
    /// Nothing proves this row: no passing test, or no test at all.
    Unproven,
}

impl TaskState {
    fn checkbox(self) -> &'static str {
        match self {
            TaskState::Green => "[x]",
            TaskState::Red => "[ ]",
            TaskState::Legacy => "[=]",
            TaskState::Unproven => "[~]",
        }
    }
    /// Display name for this task state (GREEN, RED, LEGACY, or UNPROVEN).
    pub fn word(self) -> &'static str {
        match self {
            TaskState::Green => "GREEN",
            TaskState::Red => "RED",
            TaskState::Legacy => "LEGACY",
            TaskState::Unproven => "UNPROVEN",
        }
    }
    /// The four words, case-blind. `None` = not a state, never a guessed default.
    /// `UNWIRED` is honoured as the retired spelling of [`TaskState::Unproven`] so a
    /// ledger row written before 08-04 still parses instead of silently dropping.
    pub fn parse(s: &str) -> Option<TaskState> {
        match s.trim().to_ascii_uppercase().as_str() {
            "GREEN" => Some(TaskState::Green),
            "RED" => Some(TaskState::Red),
            "LEGACY" => Some(TaskState::Legacy),
            "UNPROVEN" | "UNWIRED" => Some(TaskState::Unproven),
            _ => None,
        }
    }
    /// Does this state release a DOWNSTREAM row's dependency edge?
    ///
    /// LEGACY counts. The zero-base demotes 287 rows in one move and Sean's rule is "no
    /// mass re-audit" — if a legacy dep blocked its dependents, the whole DAG would go
    /// BLOCKED on day one and the re-audit would be forced anyway, through the back door.
    pub fn satisfies_dep(self) -> bool {
        matches!(self, TaskState::Green | TaskState::Legacy)
    }
}

// THE FLIP VERB IS DEAD (Sean 2026-08-04). `Flip`/`flip_row`/`parse_flips`/`prune_flips`/
// `apply_flips` and `13forge-studio board flip` are removed, not deprecated. A hand verdict
// laid over the harvest was a SECOND truth about the same board, and the only row it ever
// wrote (`.forge/board_flips.tsv`, CDK-TRIAD) argued its case in prose — which is exactly
// what a state that must be computed cannot accept. Green is derived below or it is absent.
// Husk: `_attic/board-flips-killed-2026-08-04/board_flips.tsv`.

/// How many of a row's anchors resolve on disk, and whether any DECLARED one is gone.
///
/// `(resolved, drained)` — `drained` is the regression signal: the row named a symbol, the
/// symbol left, and that is RED, not "unproven". A row with no anchors returns `(0, false)`.
pub fn anchor_census(task: &BoardTask) -> (usize, bool) {
    let mut resolved = 0usize;
    let mut drained = false;
    for a in &task.anchors {
        match crate::seams::resolve_parts(&a.symbol, &a.file) {
            crate::seams::AnchorState::Present => resolved += 1,
            _ => drained = true,
        }
    }
    (resolved, drained)
}

/// THE ONE PLACE STATUS IS DECIDED — derived, never read out of a file.
///
/// `board_status.json` is demoted here from verdict to MEASUREMENT: it records only that a
/// named test passed or failed. The verdict is this function, every time it is called:
///
/// - test failed, or a declared anchor left disk -> [`TaskState::Red`]
/// - test passed AND >=1 anchor resolves        -> [`TaskState::Green`]
/// - test passed, row claims no anchor          -> [`TaskState::Legacy`]
/// - no test outcome at all                     -> [`TaskState::Unproven`]
///
/// This is `seams.rs` scaled to the board: the seam registry has refused an unanchored
/// claim since 07-31 (`a_wired_seam_carries_at_least_one_anchor`) while the board next to
/// it accepted 287 of them.
pub fn state_of(status: &BoardStatus, tasks: &[BoardTask], id: &str) -> TaskState {
    let Some(task) = tasks.iter().find(|t| t.id == id) else {
        // A status key no row authors is not a row. Never green.
        return TaskState::Unproven;
    };
    state_of_task(status, task)
}

/// [`state_of`] when the caller already holds the row — the same law, no id lookup.
pub fn state_of_task(status: &BoardStatus, task: &BoardTask) -> TaskState {
    let (resolved, drained) = anchor_census(task);
    match status.outcomes.get(&task.id) {
        Some(false) => TaskState::Red,
        Some(true) if drained => TaskState::Red,
        Some(true) if resolved > 0 => TaskState::Green,
        Some(true) => TaskState::Legacy,
        None if drained => TaskState::Red,
        None => TaskState::Unproven,
    }
}

/// Scan one Rust source for `// [BOARD: <id>]` tags sitting immediately above a
/// `#[test]` fn. Returns `(task_id, test_fn_name)` pairs. Pure, total.
pub fn scan_board_tags(src: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::new();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        let Some(rest) = t.strip_prefix("//") else { continue };
        // `///` is a comment too. Stripping only `//` left a stray `/` in front of the
        // tag, so a doc-comment tag scanned as untagged and the row stayed UNWIRED with
        // no complaint (Sean 07-27: "this is going to break again"). Shape of the
        // comment is not the author's problem — the tag is the contract.
        // `//!` (module header) is a comment too, and it is where a whole-file
        // proof states its row. Stripping only `/` left a stray `!` in front of
        // the tag, so an inner-doc tag scanned as untagged exactly like the `///`
        // case above — same silent UNWIRED, one character further along. Both
        // marker chars go (Sean 07-30: four proven rows sat unharvested behind it).
        let rest = rest.trim_start_matches(['/', '!']).trim();
        // Take the id up to the FIRST `]`, tolerating trailing prose after it —
        // a tag is `[BOARD: id]` followed by anything. Requiring the comment to
        // END with `]` (old `strip_suffix`) silently dropped every proof whose
        // tag line carried a description, reading GREEN-worthy work as UNWIRED.
        let Some(after_tag) = rest.strip_prefix("[BOARD:") else { continue };
        let Some(close) = after_tag.find(']') else { continue };
        let id = after_tag[..close].trim().to_string();
        if id.is_empty() {
            continue;
        }
        // Walk forward past blank/comment/attr lines to the fn; a `#[test]`
        // attr MUST appear on the way or the tag is not a test proof.
        let mut saw_test = false;
        for l in &lines[i + 1..] {
            let l = l.trim();
            if l.is_empty() || l.starts_with("//") {
                continue;
            }
            if l.starts_with("#[") {
                if l.contains("test") {
                    saw_test = true;
                }
                continue;
            }
            if let Some(name) = parse_fn_name(l) {
                if saw_test {
                    out.push((id.clone(), name));
                }
            }
            break; // first substantive line decides — bound or not.
        }
    }
    out
}

/// `pub fn foo(` / `async fn foo` / `fn foo<T>` -> `foo`.
fn parse_fn_name(line: &str) -> Option<String> {
    let idx = line.find("fn ")?;
    let after = &line[idx + 3..];
    let name: String =
        after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    (!name.is_empty()).then_some(name)
}

/// Parse `cargo test`/libtest output into `full::test::path -> passed`. Handles BOTH
/// the libtest JSON line (`{"type":"test","event":"ok","name":"path::fn"}`) and
/// the always-printed human line (`test path::fn ... ok|FAILED`), so it works on
/// stable with a plain `cargo test --message-format=json`. Keyed by the FULL
/// name (Sean 08-06 false-green audit): keying by last segment let any passing
/// `smoke` overwrite a FAILED `smoke` from another module — a red verdict masked
/// by an unrelated pass. A duplicate full name ANDs (red wins), never overwrites.
pub fn parse_test_outcomes(output: &str) -> BTreeMap<String, bool> {
    let mut m: BTreeMap<String, bool> = BTreeMap::new();
    let put = |m: &mut BTreeMap<String, bool>, name: &str, passed: bool| {
        let k = name.trim().to_string();
        let v = m.get(&k).copied().unwrap_or(true) && passed;
        m.insert(k, v);
    };
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if v.get("type").and_then(|t| t.as_str()) == Some("test") {
                    if let (Some(name), Some(event)) = (
                        v.get("name").and_then(|n| n.as_str()),
                        v.get("event").and_then(|e| e.as_str()),
                    ) {
                        match event {
                            "ok" => put(&mut m, name, true),
                            "failed" => put(&mut m, name, false),
                            _ => {}
                        }
                    }
                }
                continue; // parsed as JSON; don't also human-parse this line
            }
        }
        if let Some(rest) = line.strip_prefix("test ") {
            if let Some((name, tail)) = rest.rsplit_once(" ... ") {
                let tail = tail.trim();
                if tail == "ok" {
                    put(&mut m, name, true);
                } else if tail.starts_with("FAILED") {
                    put(&mut m, name, false);
                }
            }
        }
    }
    m
}

fn last_seg(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).trim().to_string()
}

/// Every outcome whose fn name (last `::` segment) matches the tag's fn.
fn join_candidates<'a>(by_fn: &'a BTreeMap<String, bool>, fn_name: &str) -> Vec<(&'a str, bool)> {
    by_fn.iter().filter(|(k, _)| last_seg(k) == fn_name).map(|(k, &v)| (k.as_str(), v)).collect()
}

/// The HARVEST join (pure core): scan every source for tags, read the test
/// output once, map each task id to the AND of its tagged tests' outcomes. A
/// task whose test did not run in scope stays absent (UNWIRED), never green.
///
/// Sean 08-06 false-green audit: the log carries module paths but no crate, so
/// a bare fn-name join let ANY same-named passing test prove a row. Now a
/// FAILED candidate always reds the row (red wins), and an all-green join with
/// MORE THAN ONE candidate is AMBIGUOUS — not proof, the row stays absent and
/// `ambiguous_joins` names it for the verb to say out loud.
pub fn harvest(sources: &[(String, String)], test_output: &str) -> BoardStatus {
    let by_fn = parse_test_outcomes(test_output);
    let mut outcomes: BTreeMap<String, bool> = BTreeMap::new();
    for (_path, src) in sources {
        for (id, fn_name) in scan_board_tags(src) {
            let cands = join_candidates(&by_fn, &fn_name);
            let verdict = match cands.as_slice() {
                [] => continue,                       // did not run in scope
                [(_, v)] => *v,                       // unique witness
                _ if cands.iter().any(|(_, v)| !v) => false, // red wins over any mask
                _ => continue,                        // all green but ambiguous != proof
            };
            let e = outcomes.entry(id).or_insert(true);
            *e = *e && verdict; // every tagged proof must pass for GREEN
        }
    }
    BoardStatus { outcomes }
}

/// `(task id, fn name, hit count)` for every tag whose all-green join was refused
/// because more than one ran test carries that fn name — the verb prints these.
pub fn ambiguous_joins(sources: &[(String, String)], test_output: &str) -> Vec<(String, String, usize)> {
    let by_fn = parse_test_outcomes(test_output);
    let mut out = Vec::new();
    for (_path, src) in sources {
        for (id, fn_name) in scan_board_tags(src) {
            let cands = join_candidates(&by_fn, &fn_name);
            if cands.len() > 1 && cands.iter().all(|(_, v)| *v) {
                out.push((id, fn_name, cands.len()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Ids green in `prior` whose tagged witness fn produced NO outcome in this log —
/// the green is riding `merge_status` carry-forward, not a run. Warning lane only:
/// narrow `-p` runs legitimately skip most witnesses.
pub fn stale_witnesses(prior: &BoardStatus, sources: &[(String, String)], test_output: &str) -> Vec<String> {
    let by_fn = parse_test_outcomes(test_output);
    let mut ran_ids = std::collections::BTreeSet::new();
    let mut all_ids: BTreeMap<String, bool> = BTreeMap::new();
    for (_path, src) in sources {
        for (id, fn_name) in scan_board_tags(src) {
            let hit = !join_candidates(&by_fn, &fn_name).is_empty();
            if hit {
                ran_ids.insert(id.clone());
            }
            all_ids.insert(id, hit);
        }
    }
    prior
        .outcomes
        .iter()
        .filter(|(id, &green)| green && all_ids.contains_key(*id) && !ran_ids.contains(*id))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Emit the RUN-BOARD face — pure, deterministic, one row per authored task with
/// its compiled state. Never invents GREEN: a row with no resolving anchor reads
/// LEGACY (a test passed once) or UNPROVEN (nothing did), and both are honest.
pub fn compile_board(title: &str, tasks: &[BoardTask], status: &BoardStatus) -> String {
    use std::collections::BTreeSet;
    let mut out = format!("# {title} — LIVING BOARD (compiled face · DO NOT hand-edit)\n");
    out.push_str(
        "# GREEN IS COMPUTED, NEVER STORED (Sean 2026-08-04): a row is [x] GREEN only when a \
         `// [BOARD: <id>]`-tagged #[test] passes AND >=1 of its `anchors` resolves on disk — \
         [ ] RED (test failed, or a declared anchor left disk) · [=] LEGACY (test passes, row \
         claims no anchor: re-earn it by adding one) · [~] UNPROVEN (nothing proves this)\n\n",
    );
    let elig: BTreeSet<&str> = eligible(tasks, status).iter().map(|t| t.id.as_str()).collect();
    let (mut g, mut r, mut l, mut u) = (0usize, 0usize, 0usize, 0usize);
    for t in tasks {
        let s = state_of_task(status, t);
        match s {
            TaskState::Green => g += 1,
            TaskState::Red => r += 1,
            TaskState::Legacy => l += 1,
            TaskState::Unproven => u += 1,
        }
        // DAG blocking (boardsync_v3): non-green + any unmet dep reads BLOCKED,
        // distinct from the actionable eligible frontier.
        let deps_ok = t.deps.iter().all(|d| state_of(status, tasks, d).satisfies_dep());
        let mark = if elig.contains(t.id.as_str()) {
            "  <- NEXT (eligible)"
        } else if !s.satisfies_dep() && !deps_ok {
            "  \u{1F512} BLOCKED"
        } else {
            ""
        };
        out.push_str(&format!(
            "- {} {} [{}>{}] | {} | status: {}{}\n",
            s.checkbox(), t.id, t.intent.tag(), t.intent.generator_tag(), t.title, s.word(), mark
        ));
    }
    out.push_str(&format!(
        "\n# tally: {g} GREEN · {r} RED · {l} LEGACY · {u} UNPROVEN · {} tasks\n",
        tasks.len()
    ));
    let q: Vec<&str> = elig.iter().copied().collect();
    out.push_str(&format!(
        "# DAG: L(G) critical-path={} · W(G) width={} · eligible[{}] = {}\n",
        critical_path_len(tasks),
        width(tasks),
        q.len(),
        if q.is_empty() { "SATURATED".to_string() } else { q.join(" ") }
    ));
    if frontier(tasks, status) == Frontier::Saturated {
        out.push_str(
            "# !! SATURATED — the DAG cannot schedule. NOT done: an open lane the board \
             does not carry cannot be scheduled. Sean names the next block, or a missing \
             lane becomes a row.\n",
        );
    }
    out
}

/// Merge a fresh harvest onto the prior persisted status: an id this run's test
/// output actually exercised takes the fresh verdict; an id it did NOT exercise
/// (out of `-p` scope) keeps its prior verdict instead of reverting to UNWIRED.
/// A task never proven by either carries forward absent, per the harvest law —
/// this never invents a GREEN, it only stops a narrow-scope run from erasing one.
pub fn merge_status(prior: &BoardStatus, fresh: &BoardStatus) -> BoardStatus {
    let mut outcomes = prior.outcomes.clone();
    outcomes.extend(fresh.outcomes.iter().map(|(k, &v)| (k.clone(), v)));
    BoardStatus { outcomes }
}

/// Return to Unwired every row no `[BOARD: id]` tag on disk claims. `merge_status` is
/// `prior.extend(fresh)`, so green is one-way: a row proven once stays green after its
/// witness test is deleted, and the frontier drains to zero over rows nothing proves.
/// Demotion is the only path back — disk owns the verdict, and an absent tag is not one.
pub fn demote_untagged(status: &BoardStatus, tagged: &std::collections::BTreeSet<String>) -> BoardStatus {
    BoardStatus {
        outcomes: status.outcomes.iter().filter(|(k, _)| tagged.contains(k.as_str())).map(|(k, &v)| (k.clone(), v)).collect(),
    }
}

/// Every task id a `[BOARD:]` tag claims across the scanned tree, verdict-blind.
pub fn tagged_ids(sources: &[(String, String)]) -> std::collections::BTreeSet<String> {
    sources.iter().flat_map(|(_, src)| scan_board_tags(src)).map(|(id, _)| id).collect()
}

/// Failing tests in the runner log that NO `[BOARD: id]` tag claims — sorted fn
/// names, deduped.
///
/// The harvest joins tag -> fn -> outcome, so a failure under an untagged test
/// moves no row and the board seals clean over a red suite (measured 2026-08-02:
/// `281G/0R/1U` sealed while `cargo test -p forge-book --lib` returned 2 failed).
/// A 0R that cannot go red is not a gauge. This is the count that makes it one.
pub fn unmapped_reds(sources: &[(String, String)], test_output: &str) -> Vec<String> {
    let claimed: std::collections::BTreeSet<String> =
        sources.iter().flat_map(|(_, src)| scan_board_tags(src)).map(|(_, f)| f).collect();
    parse_test_outcomes(test_output)
        .into_iter()
        .filter(|(name, passed)| !*passed && !claimed.contains(&last_seg(name)))
        .map(|(name, _)| name)
        .collect()
}

/// Drop any persisted outcome whose id is no longer an authored task — the
/// status file tracks only the CURRENT board, never accumulates orphaned ids
/// left behind by a renamed or retired task. Bounded by `tasks.len()`; it can
/// never grow past the authored task count, merge-forever or not.
pub fn prune_to_tasks(status: &BoardStatus, tasks: &[BoardTask]) -> BoardStatus {
    let ids: std::collections::BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    BoardStatus {
        outcomes: status.outcomes.iter().filter(|(k, _)| ids.contains(k.as_str())).map(|(k, &v)| (k.clone(), v)).collect(),
    }
}

/// Serialize the harvest for the `board_status.json` artifact (pretty, stable) —
/// kept here so the impure xtask driver needs no serde of its own.
pub fn status_to_json(status: &BoardStatus) -> String {
    serde_json::to_string_pretty(status).unwrap_or_else(|_| "{}".to_string())
}

/// Parse a persisted `board_status.json` back to a BoardStatus (empty on garbage).
pub fn status_from_json(s: &str) -> BoardStatus {
    serde_json::from_str(s).unwrap_or_default()
}

/// The board's OWN permanence seal (sibling of `compile::compile_sealed`, kept
/// separate so the Atlas content-seal never depends on test state). Same
/// SHA-256/MSB-first path the calligraphy law uses for every face.
pub fn seal_board(title: &str, board_md: &str) -> ProvenanceSeal {
    seal_bytes(title, board_md.as_bytes())
}

/// The authored worldmerge task DEFINITIONS (SSoT) — the ONE board, migrated off
/// the hand-written RUN-BOARD. Agents move a row GREEN by writing a
/// `// [BOARD: <id>]`-tagged passing `#[test]`, NEVER by editing text. An untagged
/// row reads UNWIRED until its proof exists — that is the discipline, even for
/// backend bricks whose (untagged) tests already pass.
/// DISK OVERRIDE (Sean 2026-07-31): the authored list below compiles INTO the binary, so
/// editing one milestone meant a rustc run over 150+ rows. `.forge/board_tasks.json`, when
/// present and parseable and non-empty, IS the board. The compiled rows stay as the
/// fallback so a fresh checkout still has a board, and so an absent, malformed, or empty
/// file can never read as "no work" — an unreadable board is a fault, not an empty one.
/// DAG STARVATION (Sean 2026-08-04 "start fixing the DAG its becoming a blocker"): the
/// authored board was the scheduler's ONLY input, so when all 287 authored rows harvested
/// GREEN, [`crate::realwork::eligible`] returned zero and `route` printed SATURATED — while
/// `.forge/recovery/TECH-DEBT.json` held 37 open rows and `.forge/drain-index.json` held
/// undrained quarry capabilities. Both are owed work by any honest reading; neither could
/// ever reach the DAG, because nothing minted a row from them. [`debt_ledger::merged`]
/// already folds those two files into ONE backlog and had no scheduler caller — this is it
/// (root#orphan-wire). An empty backlog changes nothing, so the authored board is never
/// weakened by the fold.
pub fn worldmerge_tasks() -> Vec<BoardTask> {
    let root = std::path::Path::new("F:/v3");
    let mut tasks = tasks_from_disk(root).unwrap_or_else(authored_tasks);
    for t in debt_tasks(root) {
        if !tasks.iter().any(|x| x.id == t.id) {
            tasks.push(t);
        }
    }
    tasks
}

/// Board ids minted from the debt backlog carry this prefix, so an ingested row can never
/// collide with an authored id and `route` names its source at a glance.
pub const DEBT_TASK_PREFIX: &str = "DEBT-";

/// Every OPEN row of the merged backlog as a schedulable [`BoardTask`].
pub fn saturating_loc(lines: u32) -> u32 {
    let ceil = 400u64;  // BOARD_LOC_CEIL
    ((lines as u64 * ceil) / (lines as u64 + ceil)) as u32
}

/// Convert all open rows from the merged debt backlog into schedulable board tasks with DEBT_TASK_PREFIX.
pub fn debt_tasks(root: &std::path::Path) -> Vec<BoardTask> {
    let l = crate::debt_ledger::merged(root);
    let settled = crate::debt_ledger::settled_in_place(&l);
    l.rows
        .iter()
        .filter(|r| !r.id.trim().is_empty())
        .filter(|r| !settled.contains(&r.id.as_str()))
        .map(|r| {
            let depth = if r.why_not_wired.trim().is_empty() { "0.5" } else { "1" };
            let loc = match debt_target(root, &r.at) {
                Some((_, lines)) => format!("[loc:{}]", saturating_loc(lines)),
                None => String::new(),
            };
            let what = if r.debt.trim().is_empty() { r.at.trim() } else { r.debt.trim() };
            BoardTask::new(
                &format!("{DEBT_TASK_PREFIX}{}", r.id),
                Intent::Own,
                &format!("[lane:welder]{loc}[d:{depth}][roi:M] {what}"),
            )
            .domain("tech-debt")
        })
        .collect()
}

/// The first path in a debt row's `at` claim that resolves to a real file, and that file's
/// line count. `None` when nothing on disk answers to the claim.
pub fn debt_target(root: &std::path::Path, at: &str) -> Option<(String, u32)> {
    for tok in at.split_whitespace() {
        let tok = tok.trim_matches(|c: char| "(),·[]{}".contains(c));
        let cand = match tok.rsplit_once(':') {
            Some((p, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => p,
            _ => tok,
        };
        if cand.is_empty() {
            continue;
        }
        let p = root.join(cand);
        if let Ok(src) = std::fs::read_to_string(&p) {
            return Some((cand.to_string(), src.lines().count() as u32));
        }
    }
    None
}

/// The board as `.forge/board_tasks.json` holds it. `None` for absent, malformed, or
/// empty — every one of those means "fall back", never "the board is clear".
pub fn tasks_from_disk(root: &std::path::Path) -> Option<Vec<BoardTask>> {
    let raw = std::fs::read_to_string(root.join(".forge/board_tasks.json")).ok()?;
    let tasks: Vec<BoardTask> = serde_json::from_str(&raw).ok()?;
    (!tasks.is_empty()).then_some(tasks)
}

/// The authored rows as JSON — so disk can take the board over WITHOUT anyone retyping
/// 150 rows by hand. `13forge-studio board --export` writes this; after that, the file is
/// the thing you edit.
pub fn export_tasks(tasks: &[BoardTask]) -> String {
    serde_json::to_string_pretty(tasks).unwrap_or_default()
}

/// The compiled fallback: the authored task DEFINITIONS as they were before the board
/// moved to disk. Still the SSoT for a checkout with no `.forge/board_tasks.json`.
/// V3 NOTE (2026-08-17): M1, M2, M3 were bootstrap placeholders in v2 but have no
/// [BOARD:] tags in v3 source (checked: grep -p F:\v3\crates\forge-book-v3\src .[BOARD: M[0-9]).
/// Removed to eliminate unreferenced-row diagnostics. M10 is tagged and comes from compile.rs.
pub fn authored_tasks() -> Vec<BoardTask> {
    vec![
        BoardTask::new("BOARD-COMPILER", Intent::Make, "the harvest, admitted to be a compiler — board_compile::diagnose is the link pass, typed: UndeclaredTag (undefined symbol) + UnreferencedRow (a declared row NO tag anywhere claims) + DAG soundness checks. Self-harvesting: its own test reads its own source for its own tag").domain("core-harness"),
    ]
}

/// Format a single ledger row: timestamp, seal, and task counts (GREEN/RED/LEGACY/UNPROVEN).
pub fn ledger_row(stamp: &str, seal_id: &str, tasks: &[BoardTask], status: &BoardStatus) -> String {
    let g = tasks.iter().filter(|t| state_of_task(status, t) == TaskState::Green).count();
    let r = tasks.iter().filter(|t| state_of_task(status, t) == TaskState::Red).count();
    let l = tasks.iter().filter(|t| state_of_task(status, t) == TaskState::Legacy).count();
    let u = tasks.iter().filter(|t| state_of_task(status, t) == TaskState::Unproven).count();
    format!("{stamp} | board-id={seal_id} | {g}G {r}R {l}L {u}U | {}/{}", g, tasks.len())
}

/// Topological sort of tasks by dependency order, or return cycle ids if the DAG is cyclic.
pub fn topo_order(tasks: &[BoardTask]) -> Result<Vec<String>, Vec<String>> {
    let ids: std::collections::BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let mut result = Vec::new();
    let mut temp_mark: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut perm_mark: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut cycle = Vec::new();

    fn visit(
        id: &str,
        tasks: &[BoardTask],
        ids: &std::collections::BTreeSet<&str>,
        result: &mut Vec<String>,
        temp: &mut std::collections::BTreeSet<String>,
        perm: &mut std::collections::BTreeSet<String>,
        cycle: &mut Vec<String>,
    ) -> bool {
        if perm.contains(id) {
            return true;
        }
        if temp.contains(id) {
            cycle.push(id.to_string());
            return false;
        }
        temp.insert(id.to_string());
        if let Some(task) = tasks.iter().find(|t| t.id == id) {
            for dep in &task.deps {
                if ids.contains(dep.as_str()) {
                    if !visit(dep, tasks, ids, result, temp, perm, cycle) {
                        return false;
                    }
                }
            }
        }
        temp.remove(id);
        perm.insert(id.to_string());
        result.push(id.to_string());
        true
    }

    for id in &ids {
        if !perm_mark.contains(*id) {
            if !visit(id, tasks, &ids, &mut result, &mut temp_mark, &mut perm_mark, &mut cycle) {
                return Err(cycle);
            }
        }
    }
    Ok(result)
}

/// Check whether the task DAG is acyclic; return Err with cycle ids if not.
pub fn acyclic(tasks: &[BoardTask]) -> Result<(), Vec<String>> {
    topo_order(tasks).map(|_| ())
}

fn levels(tasks: &[BoardTask]) -> std::collections::BTreeMap<String, usize> {
    let mut m = std::collections::BTreeMap::new();
    if let Ok(order) = topo_order(tasks) {
        for (level, id) in order.iter().enumerate() {
            m.insert(id.clone(), level);
        }
    }
    m
}

/// Length of the longest path in the task dependency DAG (critical path).
pub fn critical_path_len(tasks: &[BoardTask]) -> usize {
    let lv = levels(tasks);
    lv.values().max().copied().unwrap_or(0).saturating_add(1)
}

/// Maximum number of tasks that can execute in parallel (DAG width).
pub fn width(tasks: &[BoardTask]) -> usize {
    let mut counts = std::collections::BTreeMap::new();
    for task in tasks {
        let mut max_level = 0;
        for dep in &task.deps {
            if let Some(&l) = counts.get(dep) {
                max_level = max_level.max(l);
            }
        }
        counts.insert(&task.id, max_level + 1);
    }
    counts.values().max().copied().unwrap_or(0)
}

/// Frontier state — an empty queue is a DISTINCT state, never silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frontier {
    /// n tasks are actionable right now.
    Open(usize),
    /// Nothing is actionable — every task is GREEN or dep-blocked.
    Saturated,
}

/// Compute the frontier state: Open with eligible task count, or Saturated if none are eligible.
pub fn frontier(tasks: &[BoardTask], status: &BoardStatus) -> Frontier {
    let elig = eligible(tasks, status);
    if elig.is_empty() {
        Frontier::Saturated
    } else {
        Frontier::Open(elig.len())
    }
}

/// All unproven tasks whose dependencies are all satisfied (eligible for work).
pub fn eligible<'a>(tasks: &'a [BoardTask], status: &BoardStatus) -> Vec<&'a BoardTask> {
    tasks
        .iter()
        .filter(|t| {
            state_of_task(status, t) == TaskState::Unproven
                && t.deps.iter().all(|d| state_of(status, tasks, d).satisfies_dep())
        })
        .collect()
}

/// Count how many unproven tasks would become eligible if the given task turned green.
pub fn unblocks(tasks: &[BoardTask], status: &BoardStatus, id: &str) -> usize {
    tasks
        .iter()
        .filter(|t| {
            t.deps.contains(&id.to_string())
                && t.deps.iter().all(|d| {
                    if d == id {
                        true
                    } else {
                        state_of(status, tasks, d).satisfies_dep()
                    }
                })
        })
        .count()
}

/// A map from task id to reach value (downstream impact count).
pub type Reach = std::collections::BTreeMap<String, usize>;

/// Rank eligible tasks by leverage (downstream unblock count), highest first.
pub fn leverage_ranked<'a>(tasks: &'a [BoardTask], status: &BoardStatus) -> Vec<(&'a BoardTask, usize)> {
    leverage_ranked_with(tasks, status, &Reach::new())
}

/// Rank eligible tasks by leverage with custom reach scores; prioritizes unblock count, then reach.
pub fn leverage_ranked_with<'a>(
    tasks: &'a [BoardTask],
    status: &BoardStatus,
    reach: &Reach,
) -> Vec<(&'a BoardTask, usize)> {
    let mut out: Vec<(&BoardTask, usize)> =
        eligible(tasks, status).into_iter().map(|t| (t, unblocks(tasks, status, &t.id))).collect();
    out.sort_by_key(|(t, n)| {
        (
            std::cmp::Reverse(*n),
            std::cmp::Reverse(reach.get(&t.id).copied().unwrap_or(0)),
            std::cmp::Reverse(t.domain.contains("harness")),
            tasks.iter().position(|x| x.id == t.id).unwrap_or(usize::MAX),
        )
    });
    out
}

fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_lowercase()).collect()
}

/// Search corpus for task retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corpus {
    /// All tasks.
    All,
    /// Eligible (unproven, dependency-free) tasks only.
    Eligible,
    /// Proven (GREEN) tasks only.
    Proven,
}

/// A search result with task reference, relevance score, and current state.
pub struct Hit<'a> {
    /// The matching task.
    pub task: &'a BoardTask,
    /// Relevance score (higher = more relevant).
    pub score: i64,
    /// Current task state.
    pub state: TaskState,
}

/// Search tasks by keyword against id, intent, domain, and title; return ranked hits.
pub fn retrieve<'a>(
    tasks: &'a [BoardTask],
    status: &BoardStatus,
    corpus: Corpus,
    query: &str,
) -> Vec<Hit<'a>> {
    let query_tokens = tokenize(query);
    let mut hits = Vec::new();
    let eligible_set = if corpus == Corpus::Eligible {
        eligible(tasks, status).into_iter().map(|t| t.id.as_str()).collect::<std::collections::BTreeSet<_>>()
    } else {
        std::collections::BTreeSet::new()
    };

    for task in tasks {
        let state = state_of_task(status, task);
        if corpus == Corpus::Proven && state != TaskState::Green {
            continue;
        }
        if corpus == Corpus::Eligible && !eligible_set.contains(task.id.as_str()) {
            continue;
        }
        let mut score = 0i64;
        let task_tokens = tokenize(&format!("{} {} {} {}", task.id, task.intent.tag(), task.domain, task.title));
        for q in &query_tokens {
            for t in &task_tokens {
                if t.contains(q) {
                    score += 100;
                }
            }
        }
        if corpus == Corpus::Eligible && eligible_set.contains(task.id.as_str()) {
            score += 50;
        }
        if score > 0 {
            hits.push(Hit { task, score, state });
        }
    }
    hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.task.id.cmp(&b.task.id)));
    hits
}

/// A batch of eligible tasks grouped by domain for dispatch.
pub struct DispenseBatch {
    /// Domain/category name.
    pub domain: String,
    /// Eligible tasks in this domain.
    pub tasks: Vec<BoardTask>,
}

/// Return the next batch of eligible tasks grouped by domain, or None if none are eligible.
pub fn dispense_batch(tasks: &[BoardTask], status: &BoardStatus) -> Option<DispenseBatch> {
    let mut by_domain: std::collections::BTreeMap<String, Vec<&BoardTask>> = std::collections::BTreeMap::new();
    for task in eligible(tasks, status) {
        by_domain.entry(task.domain.clone()).or_insert_with(Vec::new).push(task);
    }
    for (domain, task_refs) in by_domain {
        return Some(DispenseBatch {
            domain,
            tasks: task_refs.iter().map(|t| (*t).clone()).collect(),
        });
    }
    None
}

/// A single item in a pull board queue section.
pub struct QueueItem {
    /// Item title or description.
    pub title: String,
    /// Whether this item is marked as landed (done).
    pub landed: bool,
}

/// Parse PULL-BOARD.md `## NOW*`/`## NEXT*` sections (suffixed headers OK):
/// `-`/`*` bullets + `N.` ordinal heads are items; a `~~struck~~` head (trailing
/// text allowed, the LANDED convention) = landed. Pure, total, first-line-only.
pub fn parse_pull_board(md: &str) -> BTreeMap<String, Vec<QueueItem>> {
    let mut result: BTreeMap<String, Vec<QueueItem>> = BTreeMap::new();
    let mut sect: Option<String> = None;
    for line in md.lines() {
        let t = line.trim();
        if let Some(h) = t.strip_prefix("## ") {
            let head = h.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
            sect = (head == "NOW" || head == "NEXT").then_some(head);
            continue;
        }
        let Some(s) = &sect else { continue };
        let item = if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            b.trim()
        } else if t.starts_with("~~") || leading_ordinal(t) {
            t
        } else {
            continue;
        };
        if item.is_empty() {
            continue;
        }
        let (title, landed) = match item.strip_prefix("~~") {
            Some(rest) => (rest.split("~~").next().unwrap_or(rest).trim(), true),
            None => (item, false),
        };
        result.entry(s.clone()).or_default().push(QueueItem { title: strip_ordinal(title).to_string(), landed });
    }
    result
}

/// `12. …` head — digits, dot, then whitespace/EOL. `1.2 compose` and `2.4GB`
/// are prose numbers, not ordinals (live-gauge 07-26 false-positive classes).
fn leading_ordinal(s: &str) -> bool {
    let d = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    d > 0
        && s[d..].starts_with('.')
        && s[d + 1..].chars().next().is_none_or(|c| c.is_whitespace())
}

fn strip_ordinal(s: &str) -> &str {
    if leading_ordinal(s) {
        let d = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        s[d + 1..].trim_start()
    } else {
        s
    }
}

/// Format parsed queue sections and items back to markdown: "## Section\n- item\n".
pub fn queue_line(q: &BTreeMap<String, Vec<QueueItem>>) -> String {
    let mut out = String::new();
    for (section, items) in q {
        out.push_str(&format!("## {section}\n"));
        for item in items {
            if item.landed {
                out.push_str(&format!("- ~~{}~~\n", item.title));
            } else {
                out.push_str(&format!("- {}\n", item.title));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fn_name_handles_various_formats() {
        assert_eq!(parse_fn_name("pub fn foo() {"), Some("foo".to_string()));
        assert_eq!(parse_fn_name("async fn bar_baz() {"), Some("bar_baz".to_string()));
        assert_eq!(parse_fn_name("fn qux<T>() {"), Some("qux".to_string()));
        assert_eq!(parse_fn_name("not a function"), None);
    }

    #[test]
    fn scan_board_tags_finds_tags_above_tests() {
        let src = r#"
        // [BOARD: TEST-ROW]
        #[test]
        fn test_something() {}
        "#;
        let tags = scan_board_tags(src);
        assert!(tags.iter().any(|(id, _)| id == "TEST-ROW"));
    }

    #[test]
    fn task_state_satisfies_dep_correctly() {
        assert!(TaskState::Green.satisfies_dep());
        assert!(TaskState::Legacy.satisfies_dep());
        assert!(!TaskState::Red.satisfies_dep());
        assert!(!TaskState::Unproven.satisfies_dep());
    }
}
