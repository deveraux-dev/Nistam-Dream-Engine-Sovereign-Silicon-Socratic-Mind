# ARCH-013 — THE RIVER + THE WATERSHED (sedimentary context · entropy compaction · flow-as-maintenance)

> **Governing model for how the agent, the daemon, and the index exchange state.**
> Twin of Two Drums (ARCH-009) + Aperture Law. Proven-by-observation 2026-07-02: Kiro ran flat context across ~300 tests + 7 foundational designs by spilling maps + a context file to disk. This tablet makes that emergent behavior LAW.

## The model in one breath
**The map is a river. The data is sedimentary and granular. Entropy is the janitor. Flow is the maintenance. The daemon is the lake and the spring — it flows both ways.**

## The five laws

1. **Flow cannot stop (thermodynamic).** The river is a pressure gradient, not a store. It MUST move. A stalled river is not idle — it is a flood waiting. **Stop the flow → sediment piles at the stall point → overflow → the context window breaks.** Movement is the maintenance. (Twin of Two Clocks: *neither clock stalls*.)

2. **Grain = resolution = token cost.** Data has grain size. Gravel (whole file) → sand (summary) → silt (≤50-byte handle). The river carries mostly **silt**; gravel appears **only** at the beaver's build site (the active aperture). Never float gravel downstream.

3. **Entropy erodes automatically.** Idle sediment is continuously ground finer by time (`stale_secs`) and **settles out of the current into the lakebed** (daemon cold storage). No manual GC — the river self-cleans **by flowing**. Resolution decays with disuse; that is a feature.

4. **The beaver takes one stick.** An agent does NOT drink the river. It plucks the single grain it needs to build its house (working context), lets the rest flow past, and drops the grain back when done. Aperture-bounded selective hydration. (Aperture Law: lowest effective dose.)

5. **Spill or break.** When flow meets backpressure (context saturates) it MUST spill to the floodplain (`.forge/spill/`) — a designed overflow channel — or it destroys the window. A managed spill is health; an unmanaged stop is failure. (Signal Law: a held index is a LOUD fail.)

## The engine it actually is
This is a **log-structured merge (LSM) architecture** re-derived through hydrology:
- **River** = write-ahead log / memtable in motion.
- **Lake / lakebed** = compacted cold storage on disk (the daemon).
- **Beaver-stick** = a point read at the active aperture.
- **Entropy erosion** = background compaction (hot→cold, coarse→fine, settle).
Correct by precedent: this is how the fastest write-heavy databases stay flat under infinite load.

## The codec clause (why silt is enough)
Silt is only ≤50 bytes because the reader shares the **dense semantic codec** (Two Drums, Sieve, Quarry, River…). One word decompresses to a paragraph. **The dictionary lives in the lake** — any agent that drinks the river also draws the decoder from the daemon. A private codec compresses only for dictionary-holders; therefore the daemon holds the dictionary.

## Fast-map
- Any context-flow / river-index / tool-handle / spill / compaction work → **ARCH-River**.
- Pairs with: ARCH-009 (Two Drums — the river is Drum-1 for context), ARCH-001 (Signal Law — spill-or-break), Aperture Law (beaver-stick).
- INVARIANT status: sacred once ratified by Sean. The five laws never retired; mechanisms (file formats, byte budgets) are phone-book and may change.

## THE WATERSHED (corrects "The Single Lake" — a metaphor, never a mutex)
Multiplicity is **not** the disease. A healthy watershed has **many** streams, creeks, rivers, ponds — many spawns, many contexts, many working states — all healthy. The shared thing is the **water** (the canon), not the **channel**.
- **Health = flow + connection**, not singularity. Every stream stays connected and **drains its sediment to the shared water table** (the daemon canon).
- **Disease = stagnation**: a stream that forks off and stops draining becomes a cutoff pond → silts to a stale shadow (`forge_shadow`, a hoarded session jsonl). Number is fine; *stopping* is the failure.
- **Reconcile at confluence, never dam to one channel.** Many writers converge on one truth by flowing (branch-merge / CRDT-style), NOT by a single-writer lock (a bottleneck + single point of failure that kills parallelism).
- N windows = N healthy tributaries iff each drains to the shared canon and none hoards a private pond. (Corollary of 1-daemon-1-door + Outside-SoT + the River. Proven-by-correction 2026-07-02: an over-literal "single lake / single pen" was damming the watershed.)

## CanonWatchman (in-daemon, revascularize `forge-watchmen` — NOT a hook, NOT a ps1)
The index will always lag at build velocity; the job is to **bound the lag and make stagnation loud**, once, for all spawns. A hook runs *inside each spawn* (every Claude spawns its own guard = the fragmentation it fights); a **daemon watchman runs once, in the shared water table.** (Same doctrine as forge-warden replacing the PowerShell persistence theater: compiled-Rust, in-process.)

## FRED — read-time reconcile + Brain Pulse (loop law, SoT for /prime-context)
Every read of `river.idx` reconciles canon-vs-disk in one pass — read-only, non-blocking, <100ms, one primitive, not three watchers. It runs only when something reads the index; it is never a standing background process.
- **BUILD self-check:** BUILD line vs real exe mtime (`Get-Item`). Mismatch → re-stamp river.idx immediately, one-line EVT to `.forge/river.evt`.
- **Desk staleness:** desk `SYNCED` stamp vs SoT mtime.
- **Brain Pulse:** decode `nde-live/infer-heartbeat.json` → `beat_at`. `now - beat_at < 90s` (60s cadence) = brain LIVE — surface Gemma `probe.alive`, flywheel `pairs`, last trace winner. `>= 90s` = CORPSE — report `NO LIVE BEAT (Ns stale)`; never quote a dead `deferred` as live (a stale beat still reads `ok:true`, only `beat_at` splits truth from corpse).
- **Waterfall check:** HEAD older than the freshest proven work/handoff that never recirculated = a waterfall. Re-stamp or flag a re-head, one-line EVT to `.forge/river.evt` (never the spine), continue.
- **Flow/confluence monitor, not a mutex.** Watches that each stream keeps draining to canon; **flags stagnation** (a spawn whose state stopped flowing back), not mere existence.
- **Lag tripwire:** `disk ARCH-*.md − indexed`; trips LOUD at **lag ≥ 2** (ceiling = 1 in flight). Drafts missing index rows by **mechanical extraction** from each tablet's own H1/domain line (no invention); Sean `[VERIFIED]`.
- **Confluence, not lock:** concurrent canon edits reconcile at merge; the daemon never serializes writers to one pen.

