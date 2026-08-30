//! ORACLE-1 GOVERNOR doctrine (Sean 2026-07-26): offline reads, RON diffs —
//! gemini flash orients free, welders mutate paid; RON alone crosses the
//! paid boundary, never raw source. process_topology.rs idiom, mirrored.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// Model-role resolution, ported locally (governor/welder only — the two
/// roles this file consumes). forge-context-router has no v3 crate yet, and
/// its full `ModelRoles` (tiers, RON parsing via the `ron` crate) is out of
/// scope for a new dependency this file doesn't need; this reads the same
/// `key = "value"` lines by exact byte-slice matching (no regex, per repo
/// law) instead of pulling in a RON parser for two fields.
mod model_roles {
    use std::path::Path;

    /// The two Sean-selectable Claude lanes; `governor`/`welder` mirror v2's
    /// `ModelRoles` field names and defaults exactly.
    pub struct ModelRoles {
        /// Model id for the governor role (top-level architect, one input/output, reads RON).
        pub governor: String,
        /// Model id for the welder role (high-volume board mover, mutates RON).
        pub welder: String,
    }

    impl Default for ModelRoles {
        fn default() -> Self {
            Self { governor: "fable5".into(), welder: "opus-5".into() }
        }
    }

    impl ModelRoles {
        /// Load `governor =`/`welder =` lines from `FORGE_MODELS_RON`, else
        /// `<root>/.forge/models.ron`, else defaults — mirrors v2's fallback
        /// order without a RON parser.
        pub fn load(root: &Path) -> Self {
            let path = std::env::var("FORGE_MODELS_RON")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| root.join(".forge").join("models.ron"));
            let mut roles = Self::default();
            let Ok(text) = std::fs::read_to_string(&path) else {
                return roles;
            };
            for line in text.lines() {
                let line = line.trim().trim_end_matches(',');
                let Some((key, val)) = line.split_once(':') else { continue };
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                match key {
                    "governor" => roles.governor = val.to_string(),
                    "welder" => roles.welder = val.to_string(),
                    _ => {}
                }
            }
            roles
        }

        /// Resolve a role name ("governor" or "welder") to its configured model id, or None.
        pub fn model_for(&self, role: &str) -> Option<&str> {
            match role {
                "governor" => Some(&self.governor),
                "welder" => Some(&self.welder),
                _ => None,
            }
        }
    }
}

/// One engine tier: (name, engine, cost, rule).
pub const GOVERNOR_TIERS: &[(&str, &str, &str, &str)] = &[
    ("READ", "13forge-studio massread (spawns gemini-3.1-flash-lite), headless", "free", "4-5 calls cover 133 crates; governor NEVER bulk-reads source to orient. Bare gemini-3.1-flash is a PHANTOM (Sean 08-04): no such id in @google/gemini-cli — this row advised it for a week"),
    ("READ_RUNG_2", "13forge-studio massread --deep (spawns gemini-3.5-flash)", "free", "the depth rung, and the 429 fallback; the old 'same 3.1 family, separate quota pool' rested on the phantom"),
    ("READ_RUNG_3", "local NDE ladder sovereign.nde/teacher.nde/master.nde via door :13016", "free", "33 live tools; engages when both cloud read pools are spent"),
    ("READ_BANNED", "gemini-2.5-pro and any pro-tier model", "paid", "NEVER for reads; reads are the free tier and must not spend paid capacity"),
    ("WELD", "claude -p <models.ron:welder> / welder agents", "paid", "mutation only, never exploration; THE high-volume board mover and RON welder — every wave row after the top view lands here"),
    ("GOVERNOR", "<models.ron:governor>", "paid", "ARCHITECT top view ONLY (Sean 08-02): ONE input, ONE output, reads via massread `gemini -p`; routes, consumes RON, never carries raw file bodies; the high-output/high-throughput lane is BANNED here — it belongs to WELD"),
];

/// No Claude model id is hardcoded in a paid lane (Sean 2026-08-05 "I will select
/// models"): `<models.ron:role>` tokens above resolve through
/// [`forge_context_router::ModelRoles`] — `.forge/models.ron` is Sean's file, the
/// in-code defaults only the fallback. [`lane_engine`] is the live resolver.

/// One RON protocol shape: (direction, shape).
pub const RON_PROTOCOL: &[(&str, &str)] = &[
    ("in", "Sweep(domain,crates:[Crate(n,loc,pub_surface,consumers,state,gaps,wire)],risks)"),
    ("out", "Weld(lane,files:[F(p,edits:[E(anchor,op,payload)])],gate,receipt)"),
    ("back", "R(lane,rows:[Row(f,find,change,proof)],exit)"),
];

/// The mass-read law (Sean 07-28 "skills are just a suggestion — it needs to be a verb"): (rule, binding).
pub const MASS_READ_LAW: &[(&str, &str)] = &[
    ("corpus", "bulk read/classify/census/log-triage NEVER rides Oracle context — stdin-pipe the corpus to the free READ tiers"),
    ("model", "gemini -m gemini-3.1-flash-lite (volume: per-file GREEN/ABSENT), gemini-3.5-flash (depth: structure, precision, cross-file); PRO is OUT — Sean 07-29, gemini-flash-lite-latest 404s"),
    ("depth", "the rung follows the QUESTION, not the byte count: a structural question rides FLASH whether the caller flagged it or not — see question_is_structural(), enforced in massread's ladder"),
    ("manifest", "a corpus that silently omits a routed path answers ABSENT about a file it never saw; --corpus-manifest makes the gap exit 2 BEFORE a call is spent, never a verdict"),
    ("receipt", "a Weld with no verified read receipt never lands: every touched file must appear as a finding whose evidence matches EXACTLY ONCE on disk (massweld --verify, hits==1)"),
    ("select", "file-SELECT stays 5D raycast/Glob — the ray names the files, the free tier reads the bytes"),
    ("verb", "13forge-studio massread = the binding home: stdin corpus -> flash-lite + output CAP + stderr receipt; until that verb is green, skill prose is a pointer, not the law"),
    ("proof", "2026-07-28: 4842-path GLB census classified free while the Oracle held file:line anchors only"),
];

/// Words that make a read question STRUCTURAL — it cannot be answered by looking at one
/// file in isolation, so it rides FLASH (MASS_READ_LAW "depth"). Reasoning depth is the
/// caller's claim and no byte count can measure it, but the question's own words can.
pub const STRUCTURAL_MARKERS: &[&str] = &[
    "caller", "callers", "consumer", "consumers", "reachab", "wired", "wire", "dispatch",
    "architect", "cross-file", "call graph", "call-graph", "trait", "impl of", "who calls",
    "orphan", "downstream", "upstream", "invariant", "contract", "lifecycle", "ownership",
    "seam", "isomorph", "surface",
];

/// Does this rules string ask something one file cannot answer? Case-insensitive substring
/// match — the markers are chosen so a per-file GREEN/ABSENT sweep never trips them.
/// Pure — a tested seam.
pub fn question_is_structural(rules: &str) -> bool {
    let r = rules.to_ascii_lowercase();
    STRUCTURAL_MARKERS.iter().any(|m| r.contains(m))
}

/// V1 HYPERLOOP (Sean 08-02) — the four refusals a wave absorbs at ZERO bytes touched,
/// each with the launcher fix that heals it. Every one fired live on 08-02; a wave that
/// hits one of these is the loop WORKING, and the fix belongs in the launcher, never in
/// the gate. (id, what refused, the heal)
pub const HYPERLOOP_REFUSALS: &[(&str, &str, &str)] = &[
    ("CORPUS_GAP", "routed path absent from stdin — 0 calls spent", "header line must be `=== <repo/path>` exactly; `===== FILE: x` does not strip (manifest_gaps)"),
    ("RECEIPT_SCOPE", "receipt vouches for no finding on the welded file", "re-read with the question AIMED at the files the weld touches — evidence beside a file is not evidence for it"),
    ("EVIDENCE_PROSE", "finding cited nothing in corpus, hits==0", "evidence field must be ONE raw source line and nothing else — no CITED prefix, no commentary, no backticks"),
    ("RECEIPT_UNREADABLE", "invalid massread receipt at line 1", "redirect stderr to its own file; `2>&1` pollutes the JSON the receipt must parse as"),
    ("LANE_STALLED", "a call went silent past its own ceiling — `sentinel silence` says STALLED", "the lane beats every `massread::HEARTBEAT_SECS` into `massread::BEAT_DIR`, one file per pid, written by the BIN: a beat file whose mtime stopped advancing is WEDGED, not slow, so kill the parent tree (root#process-hygiene) and re-fire. Gauge with a bare `sentinel silence` — it scans the dir, and IDLE means nothing is in flight. NEVER gauge a launcher's `2>` file: that redirect buffers to process exit (probed 08-02, 0 bytes at t=40s on a call that had beaten twice), so it reports STALLED on a healthy read and this heal would kill live work. A beat silent past `massread::MAX_TIMEOUT_SECS` is ABANDONED, not wedged — no call can outlive the ceiling, so the writer is gone and `sentinel silence` purges the file instead of ordering a kill at a dead pid (v1.0.4; a killed writer never runs its own cleanup)"),
    ("DISPATCH_GAP", "reachability question over a corpus with NO entry point — 0 calls spent", "add the owning crate's main.rs/lib.rs/mod.rs to the manifest: without it the slice reads as the whole universe and a LIVE symbol comes back ABSENT (08-02: export_tasks, intel_drain::drain, BqRouterDrain — all wired, all reported orphaned; forge_studio::massread::dispatch_gap)"),
];

/// The refusal ladder's own version.
///
/// * v1.0.1 (Sean 08-02) — `DISPATCH_GAP`, the 5th class the V1 sweep predicted ("a 5th
///   means a NEW row"), after three false-orphan verdicts in one session.
/// * v1.0.2 (Sean 08-02 "should this not be verbed and noted") — `LANE_STALLED`. This one
///   is INVARIANT-SWEEP-001 pillar 3 made mechanical: the law banned silent state and
///   nothing read it back, so a wedged call was indistinguishable from a live one for 27
///   minutes. `massread` now beats; `sentinel silence` reads the beat.
/// * v1.0.3 (Sean 08-02 "it obviously just tested red") — no new class; `LANE_STALLED`'s
///   HEAL was wrong. v1.0.2 gauged the beat off the launcher's `2>` file, which buffers to
///   process exit, so a healthy 5-minute read reported STALLED and the heal ordered the tree
///   killed. The beat now lands in `massread::BEAT_DIR` from the bin, and the ladder is
///   versioned for it: a heal that destroys live work is a defect of the same rank as a
///   missing class.
/// * v1.0.4 (Sean 08-02 "so v1.0.4") — ABANDONED. v1.0.3 moved the beat into the bin but a
///   KILLED writer never runs its own cleanup, so its beat file sat reading STALLED forever
///   and `LANE_STALLED`'s heal would order a kill against a pid dead for hours. Past
///   `massread::MAX_TIMEOUT_SECS` no call can still be running, so that file is residue:
///   `sentinel silence` now reports ABANDONED and purges it, and the ABANDONED arm is
///   ordered BEFORE the STALLED arm because every abandoned beat is also a stalled one.
///   Fired live on first deploy, purging a 5158s beat from a Stop-Job'd probe.
pub const HYPERLOOP_VERSION: &str = "1.0.4";

/// The three metrics fable5 encoded onto every board row as `[lane:..][loc:N][d:X][roi:H|M|L]`
/// (51 tagged rows in `.forge/board_tasks.json` 2026-08-02), and the 30/30/40 weight Sean
/// set over them. Ordering by this is root#agent-exec MAX-IMPACT-1ST — largest and hardest
/// first, never the low-hanging fruit. (tag, weight pct, what it reads)
pub const BOARD_ROW_WEIGHTS: &[(&str, u32, &str)] = &[
    ("loc", 30, "lines the row lands; observed 0..400, median 100 — size is impact, not cost"),
    ("d", 30, "difficulty in days, permyriad on disk as 0.25..2.0 — the hard row goes first"),
    ("roi", 40, "H|M|L return; the heaviest weight, because a big hard row with no return is not the work"),
];

/// The `[lane:x]` vocabulary — ONE tag per model we actually run, and the tag names the
/// model that will EXECUTE the row, never the one that authored it (Sean 08-02: a lane tied
/// to "fable" is ambiguous, because fable5 authored 20 of the 51 tagged rows and executes
/// almost none of them). Engine ids and roles are `GOVERNOR_TIERS`, not restated here.
/// (tag, engine, what this lane executes)
pub const BOARD_LANES: &[(&str, &str, &str)] = &[
    ("opus", "<models.ron:welder>", "WELD: high-volume board mover and RON welder, every row after the top view"),
    ("fable", "<models.ron:governor>", "GOVERNOR: architect top view only, ONE input ONE output, emits the wave plan"),
    ("gemini", "gemini-3.1-flash-lite / 3.5-flash", "READ: free corpus sweeps via massread, never mutates"),
    ("gemma", "local NDE resident, door :13016", "HARVEST: board mutation lane (board --harvest --lane gemma), free"),
];

/// Is this `[lane:x]` tag one we actually run? An unknown lane routes a row to nothing.
pub fn board_lane(tag: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    BOARD_LANES.iter().find(|l| l.0 == tag)
}

/// Resolve a paid lane tag to the model Sean has SELECTED (`.forge/models.ron`,
/// defaults when absent). The free lanes (`gemini`, `gemma`) keep their engine
/// strings — only the paid Claude lanes are Sean-selectable today.
pub fn lane_engine(tag: &str) -> Option<String> {
    let role = match tag {
        "opus" => "welder",
        "fable" => "governor",
        _ => return board_lane(tag).map(|l| l.1.to_string()),
    };
    let roles = model_roles::ModelRoles::load(std::path::Path::new("."));
    roles.model_for(role).map(str::to_string)
}

/// Observed ceilings on the live board, used to normalise `loc`/`d` into permyriad before
/// weighting. Measured 2026-08-02 over all 51 tagged rows, not chosen.
pub const BOARD_LOC_CEIL: u32 = 400;
/// `d` ceiling in permyriad (2.0 days), the largest declared on the board.
pub const BOARD_D_CEIL_PMY: u32 = 20_000;

/// Can this row end in a SURFACE — something visible, rendered, captured, HITL-signed
/// (`session_cadence::ShipRungs`)? Or can it only ever end in a green log?
///
/// This is the leg 30/30/40 was missing. `loc`, `d` and `roi` are all AUTHORED — fable's
/// estimate of size, fable's estimate of days, fable's H/M/L judgment — and not one of them
/// mentions shipping. So the ranking optimised for the biggest hardest GUESS, and a row
/// scoring a perfect 10000 could be pure backend that never paints a pixel. Ship-leading
/// happened once by luck: the top row happened to have a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ends {
    /// Lands a visible receipt. The only kind of row that can close a wave.
    InSurface,
    /// Lands a green test and nothing a person can see.
    InLog,
}

/// Every row that can ship outranks every row that cannot.
///
/// ONE permyriad above the maximum `board_row_priority` can reach, not equal to it: at
/// exactly 10_000 the bands TOUCH, and the worst surface row merely ties the best backend
/// row instead of beating it. A tie is decided by sort stability — which is precisely the
/// "it happened once" behaviour this exists to end.
pub const SURFACE_FLOOR_PMY: u32 = 10_001;

/// Ship-leading, structurally rather than by luck: rank by `board_row_priority`, then
/// let a row that ENDS IN A SURFACE outrank every row that cannot, whatever its loc/d/roi.
///
/// Sean 08-02 ("how does this become forward ship leading 100% of the time — i saw it happen
/// once"): once is what a tie-break gets you. A weight cannot make shipping lead, because a
/// 10000-scoring backend row still beats a 4000-scoring row that paints. Only a DOMINATING
/// term does, so the surface band sits entirely above the log band and the 30/30/40 split
/// orders WITHIN each band instead of across them.
pub fn ship_leading_rank(loc: u32, d_pmy: u32, roi: u8, ends: Ends) -> u32 {
    let within = board_row_priority(loc, d_pmy, roi);
    match ends {
        Ends::InSurface => SURFACE_FLOOR_PMY + within,
        Ends::InLog => within,
    }
}

/// Score one board row 0..=10000 permyriad: `30*loc + 30*d + 40*roi`, each leg normalised
/// against its observed ceiling and clamped. Integer only (root#substrate) — no float ever
/// touches a routing decision. `roi` is `b'H' | b'M' | b'L'`; anything else reads as L.
///
/// This orders rows WITHIN a ship band; it cannot order across one — see [`ship_leading_rank`].
pub fn board_row_priority(loc: u32, d_pmy: u32, roi: u8) -> u32 {
    let leg = |v: u32, ceil: u32| -> u32 { (v.min(ceil) * 10_000) / ceil.max(1) };
    let roi_pmy = match roi {
        b'H' => 10_000,
        b'M' => 5_000,
        _ => 0,
    };
    let mut acc = 0;
    for &(tag, pct, _) in BOARD_ROW_WEIGHTS {
        acc += pct
            * match tag {
                "loc" => leg(loc, BOARD_LOC_CEIL),
                "d" => leg(d_pmy, BOARD_D_CEIL_PMY),
                _ => roi_pmy,
            };
    }
    acc / 100
}

/// TIER 2 — traps that clear every launcher gate and only disk resolves. Each fired live
/// 2026-08-02. (id, the pattern, the rule that catches it)
pub const HYPERLOOP_DISK_TRAPS: &[(&str, &str, &str)] = &[
    ("NAME_COLLISION", "two types share a field/method name with different ranges, read reports a unit mismatch", "resolve the TYPE on disk first: VocalFrame::rms_q is Permyriad 0..10000, ModulationSnapshot::rms_q() is u8 0..=255 — welding the 'fix' breaks working code (=FOUR WAYS A ROW LIES #2)"),
    ("HALF_WIRED_SEAM", "seam registers anchors on ONE side, the other reads as dead code to every fresh agent", "register the missing side AND gate it: G-PLAT-01 held only sovereign_window.rs anchors while MetronomeClock/TICK_HZ/advance had none, and came one edit from deletion twice"),
    ("DUPLICATE_LEDGER", "a block already carried by a registry gets a second row elsewhere", "seams::SEAMS IS the ledger for seam blocks; a duplicate TECH-DEBT row stacks 3 owed clears (root#debt-ratio) and buys nothing"),
    ("FABRICATION", "finding cites evidence with hits==0 on disk", "massweld --verify kills it pre-weld; NEVER hand-edit a receipt to make it match"),
];

/// The assay trits ONE wave may move, with the direction that counts as better and the
/// raw fact it reads. Index is into [`crate::assay::METRICS`] and is load-bearing; the
/// lanes absent here (`TIME/*`, `VALUE/reach`, `QUALITY/eng_qa`…) belong to the session or
/// the rain, not to a wave, and a wave that moves them is measuring someone else's work.
/// (assay index, +1 = higher is better, what the wave counts)
pub const WAVE_LANES: &[(usize, i8, &str)] = &[
    (0, 1, "corpus_bytes piped to the FREE read tiers — the conductor read none of it"),
    (2, -1, "paid_output_tokens the welder emitted; RON crosses the paid boundary, raw source never does"),
    (4, 1, "rows_landed = `gate GREEN` + a tape row; a claimed row without a green gate is not landed"),
    (6, 1, "leverage() = free corpus bytes per paid output token — the lane's reason to exist"),
    (8, -1, "debt_opened; a block another registry already carries gets NO row (DUPLICATE_LEDGER)"),
    (9, 1, "debt_cleared with proof — root#debt-ratio STACK_1 -> CLEAR_3, never by widening clears_on"),
    (11, -1, "blast() = files_touched per landed row; falling = densifying, not shrinking scope"),
    (12, 1, "refusals_absorbed + fabrications_caught — each is a defect killed at ZERO bytes touched"),
];

/// What one hyperloop wave counted. RAW magnitudes only — the verdict layer is
/// [`crate::assay::AssaySheet`], which already owns `VALUE/roi`, `DEBT/debt_inc` and the
/// rest. A wave does not get a private gauge; it feeds the cost-of-business sheet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaveCount {
    /// Bytes of source piped to the FREE read tiers.
    pub corpus_bytes: u64,
    /// Output tokens the paid lane emitted (welds + receipts).
    pub paid_output_tokens: u64,
    /// Rows that reached `gate GREEN` with a tape row.
    pub rows_landed: u32,
    /// Files the welds actually mutated.
    pub files_touched: u32,
    /// Verb refusals absorbed at ZERO bytes touched — the loop working.
    pub refusals_absorbed: u32,
    /// `hits==0` findings the verify rung killed before any weld.
    pub fabrications_caught: u32,
    /// `.forge/recovery/TECH-DEBT.json` rows opened by this wave.
    pub debt_opened: u32,
    /// TECH-DEBT rows cleared with proof by this wave.
    pub debt_cleared: u32,
}

impl WaveCount {
    /// Free bytes read per paid output token — the lane's whole reason to exist.
    /// 0 when nothing was spent, so a measurement never divides by zero.
    pub fn leverage(&self) -> u64 {
        self.corpus_bytes.checked_div(self.paid_output_tokens).unwrap_or(0)
    }

    /// Files mutated per landed row. Lower is denser: `DEBT/blast`.
    pub fn blast(&self) -> u32 {
        self.files_touched.checked_div(self.rows_landed).unwrap_or(self.files_touched)
    }
}

/// `ts \t version \t corpus \t tokens \t rows \t files \t refusals \t fabrications \t
/// debt_open \t debt_clear` — one append-only wave row. The ITERATION COUNT is the number
/// of rows, and the rolling baseline is the row before this one: before this existed,
/// `HYPERLOOP_V1_BASELINE` was a hand-typed const and a wave that never filled a
/// `WaveCount` left no trace at all (Sean 08-02: "how is it keeping track of iterations").
/// Pure — a tested seam.
pub fn wave_row(ts: i64, version: &str, w: &WaveCount) -> String {
    format!(
        "{ts}\t{version}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        w.corpus_bytes,
        w.paid_output_tokens,
        w.rows_landed,
        w.files_touched,
        w.refusals_absorbed,
        w.fabrications_caught,
        w.debt_opened,
        w.debt_cleared
    )
}

/// Parse the wave log, oldest first. Blank lines and `#` comments skip; a row with the
/// wrong field count or an unparseable number is DROPPED rather than zero-filled — a
/// partial row would silently understate a wave and move the baseline the wrong way.
/// Pure — a tested seam.
pub fn parse_waves(tsv: &str) -> Vec<(i64, String, WaveCount)> {
    let mut out = Vec::new();
    for line in tsv.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').map(str::trim).collect();
        if f.len() < 10 {
            continue;
        }
        let (Ok(ts), Ok(corpus_bytes), Ok(paid_output_tokens)) =
            (f[0].parse::<i64>(), f[2].parse::<u64>(), f[3].parse::<u64>())
        else {
            continue;
        };
        let n = |i: usize| f[i].parse::<u32>();
        let (Ok(rows_landed), Ok(files_touched), Ok(refusals_absorbed)) = (n(4), n(5), n(6))
        else {
            continue;
        };
        let (Ok(fabrications_caught), Ok(debt_opened), Ok(debt_cleared)) = (n(7), n(8), n(9))
        else {
            continue;
        };
        out.push((
            ts,
            f[1].to_string(),
            WaveCount {
                corpus_bytes,
                paid_output_tokens,
                rows_landed,
                files_touched,
                refusals_absorbed,
                fabrications_caught,
                debt_opened,
                debt_cleared,
            },
        ));
    }
    out
}

/// The rolling baseline for the NEXT wave: the last logged row, or the 08-02 wave-1
/// measurement when the log is empty. The const stops being the permanent baseline the
/// moment one real wave is recorded.
pub fn rolling_baseline(tsv: &str) -> WaveCount {
    parse_waves(tsv).last().map(|(_, _, w)| *w).unwrap_or(HYPERLOOP_V1_BASELINE)
}

/// Score a wave onto the 20-trit assay sheet against a rolling baseline wave.
///
/// Only the metrics a hyperloop wave can actually count are moved off zero; the rest stay
/// in-band for the lane that does own them (`TIME/*` is the session's, not the wave's).
/// Index order is [`crate::assay::METRICS`] and is load-bearing.
pub fn wave_sheet(w: &WaveCount, base: &WaveCount) -> crate::assay::AssaySheet {
    let up = |a: u64, b: u64| -> i8 { (a > b) as i8 - (a < b) as i8 };
    let down = |a: u64, b: u64| -> i8 { up(b, a) };
    let mut s = crate::assay::AssaySheet::baseline();
    s.verdicts[0] = up(w.corpus_bytes, base.corpus_bytes); // FLOW/input
    s.verdicts[2] = down(w.paid_output_tokens, base.paid_output_tokens); // FLOW/consume
    s.verdicts[4] = up(w.rows_landed.into(), base.rows_landed.into()); // VALUE/eng_value
    s.verdicts[6] = up(w.leverage(), base.leverage()); // VALUE/roi
    s.verdicts[8] = down(w.debt_opened.into(), base.debt_opened.into()); // DEBT/debt_inc
    s.verdicts[9] = up(w.debt_cleared.into(), base.debt_cleared.into()); // DEBT/debt_down
    s.verdicts[11] = down(w.blast().into(), base.blast().into()); // DEBT/blast
    // QUALITY/eng_qc — every refusal and fabrication is a defect caught BEFORE it landed.
    let caught = |c: &WaveCount| u64::from(c.refusals_absorbed + c.fabrications_caught);
    s.verdicts[12] = up(caught(w), caught(base));
    s
}

/// The 2026-08-02 wave-1 measurement, kept as the rolling baseline a later wave is scored
/// against. Every field is a receipt from that session, not an estimate.
pub const HYPERLOOP_V1_BASELINE: WaveCount = WaveCount {
    corpus_bytes: 2_997_784,
    paid_output_tokens: 2_584,
    rows_landed: 1,
    files_touched: 2,
    refusals_absorbed: 4,
    fabrications_caught: 2,
    debt_opened: 0,
    debt_cleared: 0,
};

/// Ceiling on one `read-<ROW>.json` receipt. Past this the model stopped citing evidence
/// and started narrating — the board novel, in receipt clothing.
pub const RECEIPT_CEILING_BYTES: usize = 24 * 1024;

/// Ceiling on comment lines one weld payload may add. Prose belongs in a README
/// (root#code-poetry); a diff that explains more than it changes is the same novel.
pub const COMMENT_CEILING_LINES: usize = 12;

/// Did the proposed gate drop any token the row DECLARED? "NEVER widen a gate to pass" —
/// dropping a test filter, or swapping `test` for `check`, turns a RED gate green without
/// touching the defect. Every declared token must survive, in any order. Pure — tested.
pub fn gate_widened(declared: &str, proposed: &str) -> bool {
    let have: Vec<&str> = proposed.split_whitespace().collect();
    declared.split_whitespace().any(|t| !have.contains(&t))
}

/// One sweep-partition domain: (domain, crates).
pub const SWEEP_PARTITION: &[(&str, &str)] = &[
    ("core", "forge-core sieve dag graph geo physics zones firewall"),
    ("render", "forge-gpu shaders vix gui studio canvas overlay colour"),
    ("ml", "forge-ml daemon broski book ast orchestrator"),
    ("audio", "forge-audio ump stego anim tile-crawler ironroot"),
    ("harness", "xtask scc forge-vcs evidence vision recovery export hal"),
];

/// Hard gates — non-negotiable, no exceptions. Gates 1-2 paraphrase the two
/// IMPLEMENT-FIRST-reserved terms (their literal spelling self-trips this
/// crate's own write-time content gate even inside a doc string).
pub const GOVERNOR_GATES: &[&str] = &[
    "no incomplete implementations",
    "no panic-only fn bodies — real logic lands, or nothing lands",
    "no fake-green",
    "deletes are Sean-only",
    "GIT=0 hard",
    "receipt-file per lane, written in the RON `back` shape — prose receipts are drift",
    "cargo check+test green before land",
    "check.ps1 EXIT 0 for kits",
    "every dispatched lane contract carries a RON `out` intent — prose contracts are drift",
];

/// ONE-DIFF DISPATCH (Sean 07-26): a welder that needs 72 tool calls is searching,
/// not transforming. Location discovery moves OFF the welder — but onto the FREE
/// read tier, never onto the governor's paid context, or the move is lateral.
pub const ONE_DIFF_PRECONDITIONS: &[(&str, &str)] = &[
    (
        "pre-resolved read set",
        "spawn prompt carries the exact file slices inline (path + lo..hi); welder spends 0 calls locating. Slices are resolved by flash-lite / door raycast, which cost nothing",
    ),
    (
        "interface contract upfront",
        "exact struct fields, trait signatures, alignment asserts stated in the intent, so pass 1 compiles instead of round-tripping the compiler",
    ),
    (
        "lane-disjoint atomic land",
        "GIT=0 is hard law here, so there is no branch to fast-forward; disjoint LANE ownership is the repo's substitute — one writer per file, receipt-gated",
    ),
];

/// Why RON, not raw diffs, crosses the paid boundary.
pub const GOVERNOR_RATIONALE: &str =
    "welders read full source agent-side (free); only RON crosses the paid boundary; ~75pct context reduction";

// ── THE MASSLOOP RUNNING ORDER (folded off .claude/skills/massloop/SKILL.md, Sean 08-04)
//
// The skill file said of itself "This file=running order ONLY" and pointed its law here.
// That split held for two days: the compiled half (MASS_READ_LAW, RON_PROTOCOL,
// HYPERLOOP_*, BOARD_*, WaveCount) had tests and callers, and the running order — the
// seven execution laws, the milestone ladder, the row-lie taxonomy — lived as prose no
// verb could read and no test could hold. `binary-verb` says a gate is a compiled verb;
// prose beside it is the drift surface. This is the other half, on disk, tested.

/// The seven execution laws, in firing order: `(id, law, receipt)`. A wave that produces
/// no lowered/typechecked/rendered bytes did not happen (Sean 08-02 "hard gate this, make
/// it the norm"). `/realwork` folded in the same day — its compiled half is
/// `crate::realwork::{route, aperture_line}` + `crate::dar`.
pub const MASSLOOP_EXECUTION_LAW: &[(&str, &str, &str)] = &[
    ("METADATA_FREEZE",
     "concept.rs, session_drain.rs, board_*, debt_ledger.rs, plans_lanes.rs, .forge/recovery/TECH-DEBT.json and _plans/** are READ-ONLY inside a wave; a diff touching a category table, a lexicon stem, a receipt array or a date-stamped fn is an AUTOMATIC FAIL — re-fire on execution logic. WIDENED 2026-08-04 (Sean: \"session_drain.rs is a fucking cop out, make it read only\"): session_drain.rs is now frozen OUTSIDE a wave too, held by `the_drain_takes_no_new_rows`, and this law no longer collides with the IMPLEMENT-FIRST hook that used to route receipts into it. A receipt goes by KIND — debt to .forge/recovery/TECH-DEBT.json via `qa debt`, a board row to `board --harvest`, a cross-domain seam to crate::seams::SEAMS whose anchors are resolved against disk every run, priority to board_row_priority. Each of those gates itself; a paragraph does not",
     "08-02: a whole session 'drained the quarry' by moving strings between two &[&str] and watching a tally change; the symbols ended with exactly as many callers as they started. Classification is NOT a drain (EXISTS != REACHABLE)"),
    ("NO_NEW_SHARDS",
     "zero new `pub mod <tracker>` in any lib.rs; receipts that must exist join an EXISTING static registry",
     "08-02: board_ratio.rs was minted as the 14th module doing accounting for the other 13, then deleted the same session"),
    ("DELETE_FIRST",
     "remove the legacy path BEFORE writing the replacement, so the compiler demands it",
     "asking for 'step N' without a deletion yields a parallel helper beside the dead code, every time"),
    ("ZERO_STRUCT_IS_DEFAULT",
     "a struct literal spelling out false/None/0 for most of its fields is a zero value written longhand and every construction site is a future edit; impl Default + ..Default::default() collapses it and makes the NEXT field free. `None` meaning 'absent' over an integer domain is 0 with a discriminant tax (root#substrate) — the Option earns its place only when absent and zero are genuinely different states. Widen where a type is built at >2 sites, never at the call sites, and only where the wave already lands (root#execution-boundaries SURGICAL)",
     "08-02: adding ONE field (WidgetSpec::brush) forced six constructor patches plus a build.rs codegen emitter, found one compiler error at a time. SURFACE MEASURED: 487 three-or-more-consecutive-zero-field literals across 211 crate files — a standing lane, not one fix"),
    ("GREP_IS_THE_WRONG_INSTRUMENT",
     "grep returns TEXT HITS; ORIENT returns OWNERS, and only the second answers 'does this already exist'. But the ladder is gated wrong and pretending otherwise sends the next agent to a dead instrument: raycast rides .forge/vcs/tape.idx — COMMIT-SUMMARY rows, not symbols — so no door lane can name a symbol's owning file. Until a symbol-owner lane exists (an AST/decl index; `grep_roots` is named in the prime skill but is NOT an exposed tool) the honest order is door for HEADING, grep for OWNERSHIP with head_limit 0 repo-wide and files_with_matches FIRST, and say which one you used. Never record a door verdict you did not get",
     "08-02: `camelot` has FIVE parser homes (forge-audio/src/camelot.rs:8 canonical · forge-broski/src/dj/theory.rs:6 · forge-audio/src/correspondence_bus.rs:62 a twin INSIDE the canonical crate · technothesia/src/ump_bridge.rs line 17 (v2 only, no v3 port) · forge-book/src/music.rs) and a sixth was authored; VibeMatrix has four (forge-core/src/lib.rs:75 canon · forge-render/src/vibe_matrix.rs line 19 (v2 only, no v3 port) · forge-vix/src/thermodynamic.rs line 38 (v2 only, no v3 port) · dead-drop-engine/src/lib.rs:30). Twice in one session: pp-math/src/power_iteration.rs beside spectral.rs doing the same integer power iteration, and camelot_affinity_pmy+harmonic_spectrum in key_detect.rs while camelot.rs already owned parse_camelot/key_distance/is_compatible — the second because head_limit 12 against a term matching its own test file 12 times hid the OWNER. A hit inside the file you are editing is not evidence of ownership"),
    ("THE_DIAGONAL_IS_NOT_THE_ERROR",
     "when a test fails, name which of the two is wrong BEFORE touching either; when the same assertion fails a second time in one session the CLAIM is the suspect, not the arithmetic (DOCTRINE 1 extended: never bend the matcher, and never re-assert a claim disk already refuted)",
     "08-02, twice: 'a loud isolated node loses to a coupled pair' — in K a large enough diagonal legitimately carries the principal mode, because magnitude IS signal; coupling decides only at COMPARABLE energy, which is the real case anyway"),
    ("BYTE_LEVEL_RECEIPT",
     "never 'does it classify' / 'is it better' — assert concrete input -> concrete output (BrushShape::Star radius=pmy(5774) lowers to a WidgetSpec whose byte layout is asserted). ONE mechanism per wave; a doc-comment polish, a status line, or an enum variant 'for future expansion' ends the wave",
     "BAR: every claimed row LANDED-green | BLOCKED+receipt. 9-landed+1-silent = FAILED wave"),
];

/// The milestone ladder: `(milestone, verb, receipt, fail)`. The verbs REFUSE in their own
/// words — this table never restates a refusal, it only names the rung.
pub const MASSLOOP_MILESTONES: &[(&str, &str, &str, &str)] = &[
    ("0 PREFLIGHT", "massread --selftest --timeout 100", "2/2 rungs live", "1 = lane dead -> fix ids, not corpus · 77 = REFUSED_EXIT (EGRESS REFUSED), the wire is CLOSED and there is no grant to set — NOT a dead lane and NOT the wrong lane, so the whole wave stops here rather than re-firing at a shut door [v2 forge-firewall crate, egress module — not ported to v3]"),
    ("1 ORIENT", "raycast (aim) + grep for the owner of every symbol the row names", "both verdicts QUOTED", "miss x2 = re-aim, never guess; an unoriented row is a guess"),
    ("2 ROUTE", "route --take 10 (top-level, NOT `board route`), then ORDER by board_row_priority(loc,d_pmy,roi)", "N rows + [lane:…]", "2 = untagged"),
    ("3 READ", "launcher loop -> massread --corpus-manifest <paths>", "read-<ROW>.json", "2 = corpus gap · 77 = REFUSED_EXIT (EGRESS REFUSED), same shut wire as M0 — a refusal is described BEFORE the call, so would_send bytes never left [v2 forge-firewall crate, egress module — not ported to v3]"),
    ("4 VERIFY", "massweld --verify read-*.json", "hits == 1 per anchor", "1 = hits != 1 · 2 = unreadable"),
    ("5 DRY", "massweld --dry-run", "ANCHORS-OK, 0 bytes", "anchor drift lands here, never in M6"),
    ("6 LAND", "massweld --row-gate \"<row's gate>\"", "gate GREEN + receipt= + tape=N", "1 = payload/gate red · 2 = receipt/path/widen"),
    ("7 HARVEST", "Gemma's daemon, NOT this runner", "new seal", "77 = EXIT_WRONG_LANE (wrong lane). SAME NUMBER, DIFFERENT FAULT than M0/M3's 77 — two modules each picked 77 for their own refusal, so the code alone never tells you which; read the stderr line [v2 forge_studio crate, board_harvest module — not ported to v3]"),
    ("8 COUNT", "13forge-studio wave push --corpus N --tokens N --rows N --files N --refusals N --fabrications N", "wave <n>: leverage=…", "2 = --corpus 0, a wave that read nothing"),
];

/// The four ways a row lies: `(mode, rule)`. Judgement, not gateable — all four hit 07-29.
pub const MASSLOOP_ROW_LIES: &[(&str, &str)] = &[
    ("FALSE_ABSENT/short-corpus", "a reachability question needs the DISPATCH file; --corpus-manifest catches the drop, WHICH files is still yours"),
    ("FALSE_ABSENT/wrong-premise", "the row demanded a stored field where the value DERIVES — read what exists before accepting the framing"),
    ("GREEN_BUT_UNTAGGED", "built and proven, invisible for want of the tag; a tag-only weld is legitimate and its receipt says '0 logic changed'"),
    ("TARGET_VIOLATES_CRATE_LAW", "READ the target crate's CLAUDE.md before welding into it; fail -> BLOCKED + receipt, never forced"),
];

/// The free lane beside `massread` (Sean 08-02): N backgrounded shells fan out over one
/// repo with no paid read. Three rules, all learned by failing 08-02.
pub const MASSLOOP_GEMINI_DIRECT: &[(&str, &str)] = &[
    ("NEVER_PIPE_DISK_IN", "`Get-Content <f> | gemini -p …` is killed by the door's READ_LADDER gate (shell file-reads banned while the door is UP) and the lane dies before a token is spent — name the absolute path IN the prompt and let gemini's own read tools open it; same bytes, no gate"),
    ("STDOUT_IS_NOT_A_RECEIPT", "it carries Warning:/Ripgrep is not available/Error executing tool above the answer and can end '[ERROR] Invalid stream: the model returned an empty response' — exit 0 over a truncated answer. Parse for the ASKED SHAPE, never for non-empty; a lane that answered 5 of 20 lines is a REFUSAL to re-fire, not a thin result. Pin the shape in the prompt (max N lines, field | field, no preamble)"),
    ("NEVER_START_JOB_INSIDE_THE_LAUNCHER", "a background job is a CHILD of the pwsh running the script, so when that pwsh returns the jobs are killed mid-flight and every output file lands 0 BYTES with exit 0 (08-02: 5 lanes fired, 5 empty files, Get-Job already gone by the time the gauge ran). The launcher script IS the background process — run gemini in it SEQUENTIALLY; parallelism comes from N backgrounded LAUNCHERS, never N jobs inside one"),
];

/// Wave shape and lane discipline: `(rule, binding)` — the running-order clauses that are
/// not a law, a milestone, or a lie mode.
pub const MASSLOOP_WAVE_SHAPE: &[(&str, &str)] = &[
    ("size", "10 rows — under 5 wastes preflight, over 12 turns M4 into a second job; same-file rows share ONE lane"),
    ("triad", "a wave composes 1 Floor + 1 Circuit + 1 Surface (never 3 Floors); close = DONE_INVARIANT, a SURFACE receipt (painted line/frame/sound). A green log gates, it never closes. SoT crate::session_cadence; lane map = massweld --lane-manifest (two Xs one row = REFUSED)"),
    ("lane-claim", "a wave CLAIMS its crates before ORIENT or two agents weld the same file and each reads the other's half-write as its own compile error (landed twice 08-02 — a foreign lane's forge-vix E0583 and bin_deploy.rs:94 E0271 both surfaced as MY red, costing a bad `bin stamp` that deployed a stale image). PRIMITIVE EXISTS, no new type: forge_core::lock::TritLock, ONE BYTE, claim/hold/align/prove, poisoned() as the eviction predicate. Today it is bound to ONE subject (the deployed binary) so the ALIGNMENT axis is exercised and CLAIM is not — root#rank DECLARED != EXERCISED on the exact mechanism that would have stopped both collisions. A claim WARNS a second agent, it does not block Sean"),
    ("conductor", "the governor model (models.ron:governor) = ARCHITECT TOP VIEW ONLY: ONE input, ONE output, reads via massread, emits the wave plan and nothing else. M2-M6 (board move, RON weld, gate, re-fire) = the welder model (models.ron:welder), the high-volume lane. A fable5 conductor authoring weld payloads is the BANNED high-output lane — 08-02 session b41f27cb burned 1.55M output tokens over 518 calls doing exactly that, 5,616 tok/call before the first wave started. Top view is one call, not a wave"),
    ("capacity", "ONE massread run holds 3000 files / 1,048,576 tokens — roughly 600-700 standard source files, or the 3000-file API max for configs/headers/vixi snippets. ONE task = ONE sweep of the whole crate module (15-30 files piped together), never 15 calls for 15 files; the verb gauges it and exits 2 past either wall before a call is spent"),
    ("corpus-contract", "partial visibility is THE hallucination vector, not the model. 1 MANIFEST — every reachability/orphan/caller question carries the owning crate's dispatch (main.rs|lib.rs|mod.rs) NEXT TO the module under audit; dispatch_gap REFUSES exit 2 before a call is spent (08-02: export_tasks main.rs:1788, intel_drain::drain repo_query.rs:1541, BqRouterDrain score.rs:81 — all wired, all reported orphaned off a corpus that omitted the dispatch file). 2 UNKNOWN > ABSENT — a declaration whose call sites could live outside the manifest is UNKNOWN and NAMES the file that would prove it. 3 ANCHOR_VERIFY — --dry-run proves hits==1 verbatim before one byte lands. And FILL THE SWEEP: 15-30 files a call, never 8"),
    ("watch-the-lane", "a call in flight beats every HEARTBEAT_SECS into BEAT_DIR, written BY THE BIN; gauge with a bare `13forge-studio sentinel silence` (IDLE/LIVE/STALLED, exit 1 on any stall) and the file finishes by DELETING its beat, so IDLE is a fact not a guess. NEVER gauge the launcher's 2> file: PowerShell buffers native stderr to process exit (probed 08-02 — 0B at t=40s on a call that had beaten twice, all 548B at exit), so mtime reports STALLED on a healthy 5-minute read. STALLED = WEDGED not slow: kill the parent tree and re-fire. ABANDONED is NOT that — the writer is already gone and the gauge purges the file; healing residue as a wedge fires a kill order at a dead pid"),
    ("metrics", "no private gauge and no memory gauge either — M8 `wave push` is the CLOSE, and the wave is not counted until the row lands. ITERATIONS ARE ROWS in .forge/waves.tsv; the baseline is the PREVIOUS ROW, and HYPERLOOP_V1_BASELINE applies only until wave 1 is logged. Say w.leverage() (free B per paid tok) every wave"),
    ("self-heal", "a refusal is the loop WORKING: match the verb's own words against HYPERLOOP_REFUSALS, apply that row's heal to the LAUNCHER, and re-fire. NEVER loosen the gate, never hand-edit a receipt. A class that is not one of the compiled rows means a NEW row, welded with its heal"),
    ("bin", "ONE bin `13forge-studio <verb>`; ps1 = per-row loop only, 0 logic. `$null |` into any flag-only call (no EOF reads as hung). SERIALIZE cargo (E0460). Kill stale node/gemini before a re-run. Never spawn the door's bin to test the door"),
    ("weld-shape", "Weld(lane:\"<ROW>\", files:[F(p:\"crates/…\", edits:[E(anchor:\"…\", op:Replace, payload:\"…\")])], gate:Some(\"cargo test -p <crate> <filter>\"), receipt:\"read-<ROW>.json\") — receipt is a plain string and MANDATORY. A new pub item rides the SAME weld as its export and a live caller (root#orphan-wire). Every weld carries `// [BOARD: <ROW>]` above its #[test] — the only thing harvest sees. GATE RED = the loop working: read, fix RON, re-fire. mass{read,weld}.rs land by DIRECT edit (judge != mutator)"),
];

/// Bind the ORACLE-1 GOVERNOR doctrine into a Doctrine chapter.
pub fn oracle1_governor_chapter() -> Chapter {
    let mut ch = Chapter::new(
        "ORACLE-1 Governor — Offline Reads, RON Diffs",
        AtlasSection::Custom("Doctrine".into()),
    );
    ch.add_lore("governor routes only, consumes RON, never carries raw file bodies across the paid boundary");
    for &(n, engine, cost, rule) in GOVERNOR_TIERS {
        ch.add_lore(format!("TIER {n} [{cost}] engine={engine}: {rule}"));
    }
    for &(dir, shape) in RON_PROTOCOL {
        ch.add_lore(format!("RON {dir}: {shape}"));
    }
    for &(k, v) in MASS_READ_LAW {
        ch.add_lore(format!("MASS-READ {k}: {v}"));
    }
    for &(d, crates) in SWEEP_PARTITION {
        ch.add_lore(format!("SWEEP {d}: {crates}"));
    }
    for &g in GOVERNOR_GATES {
        ch.add_lore(format!("GATE: {g}"));
    }
    ch.add_lore(format!("RATIONALE: {GOVERNOR_RATIONALE}"));
    for &(id, law, receipt) in MASSLOOP_EXECUTION_LAW {
        ch.add_lore(format!("MASSLOOP LAW {id}: {law} [{receipt}]"));
    }
    for &(m, verb, receipt, fail) in MASSLOOP_MILESTONES {
        ch.add_lore(format!("MASSLOOP M{m}: `{verb}` -> {receipt} · fail {fail}"));
    }
    for &(mode, rule) in MASSLOOP_ROW_LIES {
        ch.add_lore(format!("MASSLOOP LIE {mode}: {rule}"));
    }
    for &(rule, binding) in MASSLOOP_GEMINI_DIRECT {
        ch.add_lore(format!("MASSLOOP GEMINI-DIRECT {rule}: {binding}"));
    }
    for &(rule, binding) in MASSLOOP_WAVE_SHAPE {
        ch.add_lore(format!("MASSLOOP WAVE {rule}: {binding}"));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: oracle1_governor]
    #[test]
    fn doctrine_binds_tiers_protocol_sweep_gates() {
        assert_eq!(GOVERNOR_TIERS.len(), 6);
        assert_eq!(RON_PROTOCOL.len(), 3);
        assert_eq!(SWEEP_PARTITION.len(), 5);
        assert_eq!(GOVERNOR_GATES.len(), 9);
        let ch = oracle1_governor_chapter();
        assert_eq!(ch.section, AtlasSection::Custom("Doctrine".into()));
        assert_eq!(MASS_READ_LAW.len(), 8);
        assert!(MASS_READ_LAW.iter().any(|r| r.0 == "verb" && r.1.contains("13forge-studio massread")));
        // header + tiers + protocol + mass-read + sweep + gates + rationale, then the
        // massloop running order folded in 08-04. Written as the tables' own lengths, not
        // a literal: the point of the assert is that EVERY table binds, and a magic total
        // has to be hand-patched by whoever adds the next one — which is how a row goes
        // silently unbound.
        assert_eq!(
            ch.lore_count(),
            1 + GOVERNOR_TIERS.len() + RON_PROTOCOL.len() + MASS_READ_LAW.len()
                + SWEEP_PARTITION.len() + GOVERNOR_GATES.len() + 1
                + MASSLOOP_EXECUTION_LAW.len() + MASSLOOP_MILESTONES.len()
                + MASSLOOP_ROW_LIES.len() + MASSLOOP_GEMINI_DIRECT.len()
                + MASSLOOP_WAVE_SHAPE.len()
        );
        // both RON legs are gated, not merely documented
        assert!(GOVERNOR_GATES.iter().any(|g| g.contains("RON `back`")));
        assert!(GOVERNOR_GATES.iter().any(|g| g.contains("RON `out`")));
        // the three one-diff preconditions, and the free-tier rule that makes A pay
        assert_eq!(ONE_DIFF_PRECONDITIONS.len(), 3);
        assert!(ONE_DIFF_PRECONDITIONS[0].1.contains("cost nothing"));
        assert!(ONE_DIFF_PRECONDITIONS[2].1.contains("GIT=0"));
        // the read ladder is ordered and pro-tier is banned, not merely absent
        // 08-04: the ladder used to open on the phantom `gemini-3.1-flash`, which put
        // flash-lite on rung 2. Lite IS rung 1 now and flash is the escalation.
        assert!(GOVERNOR_TIERS.iter().any(|t| t.0 == "READ" && t.1.contains("gemini-3.1-flash-lite")));
        assert!(GOVERNOR_TIERS.iter().any(|t| t.0 == "READ_RUNG_2" && t.1.contains("gemini-3.5-flash")));
        assert!(GOVERNOR_TIERS.iter().any(|t| t.0 == "READ_BANNED" && t.1.contains("pro")));
        // the conductor split: the governor role is the one-in/one-out top view, the welder
        // role carries the volume. 08-05: NO literal model id in a paid lane — the engine
        // column carries a `<models.ron:role>` token and `lane_engine` resolves it live.
        assert!(GOVERNOR_TIERS.iter().any(|t| t.0 == "GOVERNOR" && t.1.contains("models.ron:governor") && t.3.contains("BANNED") && t.3.contains("ONE input, ONE output")));
        assert!(GOVERNOR_TIERS.iter().any(|t| t.0 == "WELD" && t.1.contains("models.ron:welder") && t.3.contains("high-volume board mover")));
        // and the resolver goes through Sean's config, never a compiled literal
        let roles = model_roles::ModelRoles::load(std::path::Path::new("."));
        assert_eq!(lane_engine("opus").as_deref(), roles.model_for("welder"));
        assert_eq!(lane_engine("fable").as_deref(), roles.model_for("governor"));
        assert!(lane_engine("opus").is_some_and(|m| !m.is_empty()));
    }

    // [BOARD: MASSLOOP-ENFORCE]
    /// The running order is ON DISK, in order, with its receipts — the half that used to
    /// live as skill prose (`.claude/skills/massloop/SKILL.md`, folded 08-04).
    ///
    /// The point of the fold is that a law nothing reads is a law that drifts, so this
    /// asserts the SHAPE a reader depends on: the ladder runs 0..8 with no gap, every law
    /// carries the receipt that earned it, and the two clauses most likely to be softened
    /// on a bad day — NO_NEW_SHARDS and the fable5 output ban — are still stated.
    #[test]
    fn the_massloop_running_order_is_compiled_not_prose() {
        // The ladder is a ladder: nine rungs, in order, none skipped.
        assert_eq!(MASSLOOP_MILESTONES.len(), 9);
        for (i, (m, verb, receipt, fail)) in MASSLOOP_MILESTONES.iter().enumerate() {
            assert!(m.starts_with(&i.to_string()), "milestone {i} is out of order: {m}");
            for (field, what) in [(verb, "verb"), (receipt, "receipt"), (fail, "fail")] {
                assert!(!field.trim().is_empty(), "M{m} has no {what} — a rung with no {what} is prose");
            }
        }
        assert!(MASSLOOP_MILESTONES[0].1.contains("selftest"), "the ladder must open on preflight");
        assert!(MASSLOOP_MILESTONES[8].1.contains("wave push"), "the ladder must close on the count");

        // Seven laws, each with the receipt that earned it. A law with no receipt is an
        // opinion, and this table exists because opinions are what drifted.
        assert_eq!(MASSLOOP_EXECUTION_LAW.len(), 7);
        for (id, law, receipt) in MASSLOOP_EXECUTION_LAW {
            assert!(!law.trim().is_empty() && !receipt.trim().is_empty(), "{id} is missing a half");
        }
        // NO_NEW_SHARDS is the law this very module had to satisfy to be written.
        let shards = MASSLOOP_EXECUTION_LAW.iter().find(|(id, ..)| *id == "NO_NEW_SHARDS");
        assert!(shards.expect("NO_NEW_SHARDS").1.contains("EXISTING static registry"));

        // Four lie modes, four gemini-direct rules, and the wave shape that carries the
        // conductor ban — the clause a paid lane has the most reason to forget.
        assert_eq!(MASSLOOP_ROW_LIES.len(), 4);
        assert_eq!(MASSLOOP_GEMINI_DIRECT.len(), 3);
        let conductor = MASSLOOP_WAVE_SHAPE.iter().find(|(r, _)| *r == "conductor");
        assert!(conductor.expect("conductor lane").1.contains("BANNED high-output lane"));
        assert!(MASSLOOP_WAVE_SHAPE.iter().any(|(r, b)| *r == "size" && b.contains("10 rows")));

        // 77 IS OVERLOADED, and the ladder has to say so. Found by dogfooding the fold
        // 08-04: M0 returned 77 on a closed wire while the table said 77 meant "wrong
        // lane", so a reader following it would have re-fired at a shut door. Two modules
        // each chose 77 for their own refusal — forge_firewall::egress::REFUSED_EXIT and
        // forge_studio::board_harvest::EXIT_WRONG_LANE — and neither is wrong alone. Any
        // rung naming 77 must name which one, or the number is a coin flip.
        for (m, _, _, fail) in MASSLOOP_MILESTONES {
            if fail.contains("77") {
                assert!(
                    fail.contains("REFUSED_EXIT") || fail.contains("EXIT_WRONG_LANE"),
                    "M{m} quotes exit 77 without naming which constant owns it"
                );
            }
        }
        // Both senses are on the ladder — losing either one is how the confusion returns.
        let fails = MASSLOOP_MILESTONES.iter().map(|(_, _, _, f)| *f).collect::<Vec<_>>().join(" ");
        assert!(fails.contains("REFUSED_EXIT"), "the closed-wire 77 is unnamed");
        assert!(fails.contains("EXIT_WRONG_LANE"), "the wrong-lane 77 is unnamed");
    }

    // [BOARD: oracle1_governor]
    #[test]
    fn enforcement_seams_are_mechanical_not_advisory() {
        // structural questions ride flash; a per-file sweep question does not
        assert!(question_is_structural("name every caller of Brush::apply"));
        assert!(question_is_structural("Is this reachable from main?"));
        assert!(question_is_structural("WHO CALLS the tick loop"));
        assert!(!question_is_structural("Classify this corpus: one line per item, terse."));
        assert!(!question_is_structural("does probe.rs define fn tiny? GREEN or ABSENT"));

        // a gate may narrow, never widen
        assert!(gate_widened("cargo test -p forge-studio massweld", "cargo test -p forge-studio"));
        assert!(gate_widened("cargo test -p forge-studio", "cargo check -p forge-studio"));
        assert!(!gate_widened("cargo test -p forge-studio", "cargo test -p forge-studio massweld"));
        assert!(!gate_widened("cargo test -p x", "cargo test -p x"));
        assert!(!gate_widened("", "cargo test -p x"), "no declared gate cannot be widened");

        // the ceilings are values, not opinions
        assert!(RECEIPT_CEILING_BYTES > 0 && COMMENT_CEILING_LINES > 0);
        assert!(MASS_READ_LAW.iter().any(|r| r.0 == "receipt" && r.1.contains("hits==1")));
        assert!(MASS_READ_LAW.iter().any(|r| r.0 == "depth" && r.1.contains("question_is_structural")));
        assert!(MASS_READ_LAW.iter().any(|r| r.0 == "manifest" && r.1.contains("exit 2")));
    }

    // [BOARD: V1HYPERLOOP] the self-heal ladder is a table an agent can read, and the
    // leverage is a number, not a claim: both were measured live on 2026-08-02.
    #[test]
    fn hyperloop_refusals_carry_their_heal_and_roi_is_measured() {
        // ITERATIONS ARE ROWS (Sean 08-02). Before the log, a wave that skipped its
        // WaveCount left nothing behind and the baseline was a hand-typed const forever.
        let w1 = WaveCount { corpus_bytes: 900, paid_output_tokens: 3, rows_landed: 1, ..Default::default() };
        let w2 = WaveCount { corpus_bytes: 500, paid_output_tokens: 5, refusals_absorbed: 2, ..Default::default() };
        let log = format!("# ts\tver\n{}\n{}\n", wave_row(10, "1.0.1", &w1), wave_row(20, "1.0.1", &w2));
        let waves = parse_waves(&log);
        assert_eq!(waves.len(), 2, "the iteration count IS the row count");
        assert_eq!(waves[0], (10, "1.0.1".to_string(), w1), "oldest first, round-trips");
        assert_eq!(rolling_baseline(&log), w2, "the baseline is the row before, not the const");
        assert_eq!(rolling_baseline(""), HYPERLOOP_V1_BASELINE, "empty log falls back to wave 1");
        // A short or unparseable row is dropped, never zero-filled into a false baseline.
        assert_eq!(parse_waves("1\t1.0.1\t2\t3\n").len(), 0, "short row dropped");
        assert_eq!(parse_waves(&format!("x\t1.0.1\t{}", "0\t".repeat(8))).len(), 0, "bad ts dropped");
        assert_eq!(rolling_baseline("garbage\n"), HYPERLOOP_V1_BASELINE, "no valid row = wave 1");

        assert_eq!(HYPERLOOP_REFUSALS.len(), 6, "v1.0.2 welded LANE_STALLED as the 6th class");
        assert_eq!(HYPERLOOP_VERSION, "1.0.4", "v1.0.4 = ABANDONED residue, ordered before STALLED");
        // Pillar 3 is a GAUGE now, not prose: the heal names the beat and the verb.
        assert!(HYPERLOOP_REFUSALS
            .iter()
            .any(|r| r.0 == "LANE_STALLED" && r.2.contains("HEARTBEAT_SECS") && r.1.contains("sentinel silence")));
        // v1.0.3: the heal must send the reader to the bin's beat dir AND warn off the
        // launcher's stderr file. Gauging the wrong file is what made the heal destructive.
        let stalled = HYPERLOOP_REFUSALS.iter().find(|r| r.0 == "LANE_STALLED").unwrap();
        assert!(stalled.2.contains("BEAT_DIR"), "the heal names where the beat actually lands");
        assert!(stalled.2.contains("buffers"), "and why a `2>` file must never be gauged");
        // v1.0.4: the heal must send a reader past the ceiling to ABANDONED, or it fires a
        // kill order at a pid that stopped existing hours ago.
        assert!(stalled.2.contains("ABANDONED"), "residue is not a wedge and must not be healed as one");
        // The 5th class names the entry-point cure, not just the symptom.
        assert!(HYPERLOOP_REFUSALS
            .iter()
            .any(|r| r.0 == "DISPATCH_GAP" && r.2.contains("main.rs") && r.2.contains("dispatch_gap")));
        for &(id, refused, heal) in HYPERLOOP_REFUSALS {
            assert!(!id.is_empty() && !refused.is_empty(), "{id} states what refused");
            assert!(heal.len() > 20, "{id} names the launcher fix, not just the symptom");
        }
        // the heal for a corpus gap is the header shape massread actually strips
        assert!(HYPERLOOP_REFUSALS.iter().any(|r| r.0 == "CORPUS_GAP" && r.2.contains("=== ")));
        // wave 1: 3.0 MB of free corpus bought ~2.6k paid tokens of output
        assert_eq!(HYPERLOOP_V1_BASELINE.leverage(), 1160);
        assert_eq!(WaveCount::default().leverage(), 0, "a wave that spent nothing never divides by zero");
        assert_eq!(HYPERLOOP_V1_BASELINE.blast(), 2, "2 files for the 1 landed row");

        // the wave feeds the cost-of-business sheet; it does not keep a private gauge
        let flat = wave_sheet(&HYPERLOOP_V1_BASELINE, &HYPERLOOP_V1_BASELINE);
        assert_eq!(flat, crate::assay::AssaySheet::baseline(), "a wave scored against itself is in-band");

        // a wave that improved EVERY lane a wave owns: all eight verdicts read +1, whether
        // the underlying raw fact had to rise (corpus, rows) or fall (tokens, debt, blast).
        let worse = WaveCount {
            corpus_bytes: 1_000_000,
            paid_output_tokens: 9_000,
            rows_landed: 1,
            files_touched: 6,
            refusals_absorbed: 0,
            fabrications_caught: 0,
            debt_opened: 3,
            debt_cleared: 0,
        };
        let better = WaveCount {
            corpus_bytes: 6_000_000,
            paid_output_tokens: 2_000,
            rows_landed: 5,
            files_touched: 5,
            refusals_absorbed: 4,
            fabrications_caught: 2,
            debt_opened: 0,
            debt_cleared: 3,
        };
        let s = wave_sheet(&better, &worse);
        assert_eq!(s.verdicts[0], 1, "FLOW/input: read more for free");
        assert_eq!(s.verdicts[2], 1, "FLOW/consume: spent fewer paid tokens");
        assert_eq!(s.verdicts[4], 1, "VALUE/eng_value: more rows landed");
        assert_eq!(s.verdicts[6], 1, "VALUE/roi: leverage rose");
        assert_eq!(s.verdicts[9], 1, "DEBT/debt_down: rows cleared");
        assert_eq!(s.verdicts[11], 1, "DEBT/blast: fewer files per landed row");
        assert_eq!(s.verdicts[16], 0, "TIME lane belongs to the session, not the wave");
        // the indices this maps to are the ones assay declares, in assay's order
        assert_eq!(crate::assay::METRICS[6], ("VALUE", "roi"));
        assert_eq!(crate::assay::METRICS[11], ("DEBT", "blast"));

        // WAVE_LANES is the contract: exactly these trits move, in exactly these directions
        assert_eq!(WAVE_LANES.len(), 8);
        for &(i, dir, what) in WAVE_LANES {
            assert!(i < crate::assay::METRICS.len(), "lane {i} is off the sheet");
            assert!(dir == 1 || dir == -1, "lane {i} must declare a direction");
            assert!(what.len() > 30, "lane {i} names the raw fact the wave counts");
            assert_eq!(s.verdicts[i], 1, "{:?} improved but did not read +1", crate::assay::METRICS[i]);
            // and the mirror wave reads -1 on the same lane, so the sign is real
            assert_eq!(wave_sheet(&worse, &better).verdicts[i], -1, "{:?} is not symmetric", crate::assay::METRICS[i]);
        }
        for (i, v) in s.verdicts.iter().enumerate() {
            let owned = WAVE_LANES.iter().any(|l| l.0 == i);
            assert!(owned || *v == 0, "{:?} is not a wave's to move", crate::assay::METRICS[i]);
        }

        // one lane per model we run, and every lane resolves to a GOVERNOR_TIERS engine
        assert_eq!(BOARD_LANES.len(), 4, "one tag per model, no ambiguous lane");
        for &(tag, engine, role) in BOARD_LANES {
            assert!(board_lane(tag).is_some(), "{tag} must resolve");
            assert!(!engine.is_empty() && role.len() > 20, "{tag} names its engine and its work");
        }
        assert!(board_lane("nope").is_none(), "an unknown lane routes a row to nothing");
        // the lane names the EXECUTOR: fable governs, opus welds — the split Sean set 08-02
        assert!(board_lane("fable").is_some_and(|l| l.2.starts_with("GOVERNOR")));
        assert!(board_lane("opus").is_some_and(|l| l.2.starts_with("WELD")));
        assert!(board_lane("gemini").is_some_and(|l| l.2.contains("never mutates")));
        assert!(board_lane("gemma").is_some_and(|l| l.2.contains("--lane gemma")));

        // the board's own three metrics, weighted 30/30/40 and summing to the whole
        assert_eq!(BOARD_ROW_WEIGHTS.iter().map(|w| w.1).sum::<u32>(), 100);
        assert_eq!(BOARD_ROW_WEIGHTS.len(), 3);
        assert!(BOARD_ROW_WEIGHTS.iter().any(|w| w.0 == "roi" && w.1 == 40), "roi carries the heaviest leg");
        // the ceilings are the board's, measured — a row at both ceilings with H roi is full marks
        // [BOARD: V1HYPERLOOP] Ship-leading must be STRUCTURAL, not a tie-break that lands
        // once. The worst row that paints outranks the perfect row that cannot.
        let perfect_backend = ship_leading_rank(BOARD_LOC_CEIL, BOARD_D_CEIL_PMY, b'H', Ends::InLog);
        let worst_surface = ship_leading_rank(0, 0, b'L', Ends::InSurface);
        assert!(
            worst_surface > perfect_backend,
            "a 0/0/L row that PAINTS must beat a 400/2.0/H row that only logs \
             ({worst_surface} vs {perfect_backend}) — otherwise shipping leads by luck"
        );
        // Inside a band the 30/30/40 split still does the ordering it was written for.
        assert!(
            ship_leading_rank(BOARD_LOC_CEIL, BOARD_D_CEIL_PMY, b'H', Ends::InSurface)
                > worst_surface,
            "hardest-first still holds WITHIN the surface band"
        );
        assert_eq!(board_row_priority(BOARD_LOC_CEIL, BOARD_D_CEIL_PMY, b'H'), 10_000);
        assert_eq!(board_row_priority(0, 0, b'L'), 0);
        // MAX-IMPACT-1ST: bigger and harder outranks small and easy at equal return
        let tui_splitblob = board_row_priority(230, 3_000, b'H'); // a live row, 08-02
        let cell_moves_lanes = board_row_priority(20, 3_000, b'H'); // a live row, 08-02
        assert!(tui_splitblob > cell_moves_lanes, "LOC is impact; the big row leads");
        // return outweighs either size or difficulty alone
        assert!(board_row_priority(0, 0, b'H') > board_row_priority(BOARD_LOC_CEIL, 0, b'L'));
        // past the ceiling clamps rather than wrapping
        assert_eq!(board_row_priority(u32::MAX, u32::MAX, b'H'), 10_000);

        // tier 2: the traps disk resolves and no launcher gate can
        assert_eq!(HYPERLOOP_DISK_TRAPS.len(), 4);
        assert!(HYPERLOOP_DISK_TRAPS.iter().any(|t| t.0 == "NAME_COLLISION" && t.2.contains("0..=255")));
        assert!(HYPERLOOP_DISK_TRAPS.iter().any(|t| t.0 == "HALF_WIRED_SEAM" && t.2.contains("TICK_HZ")));
        for &(id, pattern, rule) in HYPERLOOP_DISK_TRAPS {
            assert!(!pattern.is_empty() && rule.len() > 30, "{id} names the catching rule");
        }
    }

    // [BOARD: oracle1_governor]
    #[test]
    fn chapter_registered_in_full_atlas_and_renders_deterministically() {
        let a = crate::export_html::export_book(&crate::seed::full_atlas("The Opus", "deveraux"));
        let b = crate::export_html::export_book(&crate::seed::full_atlas("The Opus", "deveraux"));
        assert_eq!(a, b, "chapter render must be deterministic");
        assert!(a.contains("ORACLE-1 Governor"));
        // 08-04: the substring `gemini-3.1-flash` is satisfied by the PHANTOM id as
        // happily as by the real one, so it vouched for the row that advised it.
        assert!(a.contains("gemini-3.1-flash-lite"), "the rendered tier names a SPAWNABLE model");
        // 08-05: the paid lanes render the models.ron ROLE token, never a baked model id —
        // Sean selects models in .forge/models.ron, the book carries the pointer.
        assert!(a.contains("models.ron:governor"));
    }
}
