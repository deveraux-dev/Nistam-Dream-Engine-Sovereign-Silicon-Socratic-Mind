# The Law Is a Verb — the forge-law session

`2026-07-20 · a plugin refused, a law compiled, two skills made one · register-honest: Live is proven, Ghost is only decided`

## What it was  ·  `F:\output\forgemarketplace\…\forge-law`

A desktop-claude plugin: a `python3` hook, a bundled `uiux-doctrine` skill, an `implement-first` agent. Portable-marketplace shape. It does not fit this repo, and the misfit is the lesson.

> Three defects, all fatal here: `${CLAUDE_PLUGIN_ROOT}` is undefined outside the marketplace loader; bare `python3` on this box is an extensionless bash shim (`exec python`) that no PowerShell hook can run; and the checker BLOCKS with exit 2, while this repo's gate contract is decision-JSON on stdout, exit 0 — exit 2 here means "could not evaluate → fail OPEN" (gate.rs:5-7). A block model inverted.

## The port — register 4, Live  ·  `F:\NewRepo\crates\forge-daemon\src\gate.rs` · `crates/forge-cli/src/main.rs`

The register borrowed its name from paint inspection. Every beam in service passes through five states: bare steel → primed → inspected → sealed → in-service. A NACE Level 2 inspector reads those five off a single surface to know what happened and what it cost. The substrate speaks; no narrative attached. See *NARRATIVE-BIBLE* Section II—this is not theoretical. Every defect visible on the coating is a defect stamped into the record; you cannot paint over a fact and call it finished. In code, the same epistemology binds register 4: a claim is `Ghost` (unproven), `Proven` (tested), or `Live` (witnessed on the board). The register stamps each claim with its proof-state because substrate *determines* finish. The binary is the beam.

The enforcement was not installed as a plugin. It was compiled into the binary as a verb: `forge.exe gate implement-first`. Patterns are `const` slices in gate.rs — one artifact, no external config, nothing to drift. A `--scan` face walks the tree for CI and the agent. The agent lives at `.claude/agents/implement-first.md`, pointed at the verb.

> Proof, not claim: `cargo test -p forge-daemon --release` = 27/27 gate tests green; the live binary denies a `todo!()` write (deny JSON, exit 0), passes clean content, honors `[SEAN-OK]`; `cargo xtask board` = 39 GREEN / 0 RED, seal 8402a20d25ec. That is register 4.

The binary-verb law made flesh: a gate exists only as a compiled, tested verb on the board — never as an ambient script that exempts its own lane.

## One skill, not three — register 4, Live  ·  `.claude/skills/vixi-uiux/`

The repo carried two UI/UX skills plus the plugin's third. `ui-ux-doctrine` (universal HCI: Nielsen 10, the three pillars, WCAG) was folded into `vixi-uiux`, kept universal — the same laws bind an exported slide / webpage / chart as bind an in-repo `.kit.vixi` panel. `research.md` moved with it.

> `forge-canvas.md` was NOT merged — it is stale pre-Tauri doctrine. Its Law 1 ("No HTML. No CSS. One wgpu pipeline") contradicts the live Tauri app, which renders `.kit.vixi` → HTML/DOM through kit_pane.rs + magic-canvas.html. Settled to `_vault/_quarry/skills-consolidation-2026-07-20/`, preserved not destroyed (delete stays Sean's gate).

## The taxonomy — register 2, Proven  ·  how the harness actually loads

Four things keep getting confused as one:

- **CLAUDE.md** — static always-on TEXT, full body every turn. The only true "preload." (≈ Kiro "always" steering.)
- **Skill** — description always in context (the trigger), body loaded only on the Skill call. Dynamic. (≈ Kiro "conditional / manual".) A plugin does not change this: 60 skills = 60 descriptions always, 60 bodies still lazy. "Preloading a skill body" just means pasting it into CLAUDE.md — which the 4.5KB cap law bans.
- **Hook** — not context the model reads; code the HARNESS runs per event, then the process exits. `forge.exe gate` spawns ~19ms and dies. Always-wired ≠ ambient. Most enforce; a UserPromptSubmit hook can also INJECT (that is how `gate prompt` pushes the vixi law).
- **Plugin** — a shipping container for the above. Packaging, orthogonal to loading. Wrapping forge-law in one adds zero always-on-ness and re-drags a C: plugin dir back in.

The one resident / ambient process is the MCP door (:13013 / :13016). `AMBIENT_PS=0` killed the rest.

## The aspiration — register 1 Design / 0 Ghost, NOT built

Named so a later session can build it, and so no one mistakes it for done:

- **Never-drift lock** (Design) — a `[BOARD]` test that pulls every `gate <kind>` from settings.json and asserts each `GateKind::parse`s; rename the verb → board RED. Stronger (Ghost): `forge.exe gate --emit-hooks` — the binary declares its own wiring, the board diffs it against settings.json. Binary = authority, board = referee: sync-or-RED, never hand-edited twice.
- **Alpha / Beta symbiote** (Ghost) — the door (Beta) push-injects intent-scoped raycast / live-indexer `file:line` context into the prompt via `gate prompt`, so the agent (Alpha) skips the first search instead of calling raycast. Fail-open (door down → no inject → fall back to tools), clamp-bounded so it cannot flood. The seam exists; the intent→context query is not wired.

## The line

The plugin was the wrong container. The law wanted to be a verb — one binary, built in one shot, sealed by the board, chained on the tape. Everything outside it (the settings wiring, the agent doc) is a reference to be VALIDATED against the binary, never a second copy kept in sync by hand.
