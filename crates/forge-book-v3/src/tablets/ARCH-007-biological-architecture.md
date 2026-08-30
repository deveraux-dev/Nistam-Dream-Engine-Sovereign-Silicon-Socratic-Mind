# ARCH-007 — Biological Architecture (The Vascular Doctrine)

**Born-condensed 2026-06-30 (Sean).** Consolidates ARCH-001 (Two Clocks · Determinism · Lock-Free),
ARCH-002 (substrate), ARCH-006 (regenerative heuristics · ForgeVision) and the `CLAUDE.md <gates>` under
ONE biological map. **Introduces NO new invariant** — every § below names the *organ* a live law already
protects — **except §4 (capillary-bed apps) and §8 (foundation-up corollary)**, which operationalize the
standing *Primitives over Monoliths* mandate at the app + index layers.

> ONE organism: one heart, one pair of kidneys, one immune barrier — many organs, many capillary-bed apps.
> The workspace survives natural Rust unwinding *because* its organs are isolated: a lane can panic and be
> dismantled while the organism lives. Each § is **why that holds** + the **live gate** that keeps it holding.
> Refactor toward this map; **revascularize organs, never scaffold new ones.**

## §1 — The Heart · atomic, decoupled clocks
*Deepens ARCH-001 Two Clocks + Determinism.* The 120 Hz DET-CLOCK is an `AtomicU64` on its own thread —
hardware CPU instructions, not a mutex.
- **Biology:** when the GPU CREATIVE-LANE panics, its stack unwinds (drop textures / handles / channels)
  while the heartbeat continues; dead tissue is flushed without stopping the heart.
- **LIVE GATE:** `vixi universal_gates.float_in_ir: forbidden` — no `f32/f64` in compiled IR; `clip`/`sprite`
  ticks are the integer SoT. The heart never shares a lock with a panic-able lane.
- **Anchor:** `forge-studio/src/main.rs` DET-CLOCK thread ("separate atomic thread (unaffected)"); drift
  re-aligns at the `UiTripleBuffer` boundary, never a local patch.

## §2 — The Kidneys · channels flush on Drop
*Deepens the Lock-Free Gate + Signal Law.* Cross-lane data moves over channels (`try_send`) and
TripleBuffers — never shared locked memory.
- **Biology:** on panic, unwinding `Drop`s one channel end; the channel severs, pending toxins flush, and
  **no poisoned lock** survives for a sibling thread to trip on. Dead tissue is excreted cleanly.
- **LIVE GATE:** bridge / thread-swap code uses `try_lock` / `try_take` / `try_send`; a `Mutex::lock()` a
  panicking lane could poison is banned.
- **Anchor:** `main.rs:374` `event_tx.try_send`; `main.rs:385` `overlay_t3.try_take`.

## §3 — Rest without backlog · park below the gate
*The `wait_message` discipline — the busy-spin fix, generalized.* At rest an organ parks at the kernel
(`WaitMessage`); the producer `try_send` is gated **below** the rest check, so the pipe stays empty while idle.
- **Biology:** a resting organ never floods its channel; a crash-on-wake finds **no toxic backlog** to clean.
- **LIVE GATE:** `alloc_steady: forbidden` (zero heap in the steady hot path) — the Kidneys / Sound-Gate
  corollary. Park, do not busy-spin; DET-CLOCK unaffected; post-wake dt clamped so restore cannot spiral.
- **Anchor:** `main.rs:341` minimize park → `forge-gpu/src/sovereign_window.rs:526` `wait_message()`.

## §4 — The Capillary Beds · apps are tissue, not organisms   ← **NEW MANDATE**
*Operationalizes Primitives-over-Monoliths + Guard-the-Line-of-Action at the app layer.* The DAW, the 2D/3D
creators, the Photoshop-equivalent, the Vixiscript playground = specialized **tissue** (muscle, retina), not
organisms.
- **No app gets its own clock, allocator, event loop, or GC.**
- **Arterial delivery (in):** state / UI / input arrive via `UiTripleBuffer`, pumped by the 120 Hz heart.
- **Venous return (waste):** on delete/drop the app **drops the reference** → stale data flows through
  channels to the central core arenas for the end-of-frame flush. **No complex teardown.**
- **LAW:** build isolated **primitive** UI components that ingest arterial data + excrete venous waste.
  **Never a "Photoshop monolith"; never a redundant loop/allocator bolted into an organ.**
- **Canonical illustration:** the lymphatic-capillary diagram — arteriole (arterial in) · venule (venous out)
  · lymphatic vessels (the deep end-of-frame flush to core arenas) · tissue cells (the apps).

## §5 — Vixiscript is mRNA · local metabolism
*Deepens ARCH-002 substrate + Contracts-before-Logic.* Compiled Rust core = immutable **DNA** in the nucleus
(the core `.exe` binaries). Vixiscript = **messenger RNA**: it travels to the leaves to synthesize temporary
proteins (behaviors / macros / tool scripts) on demand.
- **LIVE GATE:** `runtime_parse: forbidden` (AOT only) — Vixiscript binds **only** to exposed receptors
  (contracts before logic); it cannot mutate the DNA, cannot bypass the kernel park, and runs strictly within
  `UiTripleBuffer` bounds.
- **Anchor:** `forge-vix` / `tree-sitter-vixel`; the 11 live dialects are the mRNA transcripts.

## §6 — The Blood-Brain Barrier · gates as immune enforcement
*Reframes the `<gates>` — structural rejection, never symptom-catch.* The gates are the endothelial cells
lining the vessels: they prevent toxins from reaching the organs **structurally**, not by catching an error
downstream.
- **Sound Gate:** the DAW is the brain — a heap alloc on a `forge-audio` thread is rejected at the barrier
  (no `@forge:allow_alloc` → it does not compile through), never caught at runtime.
- **Vision Gate:** a hardcoded hex is rejected for `rgba(CID_*)`; `shaderbind.gates.visual_only_mutates_authority:
  forbidden` keeps a visual signal from ever writing sim authority (the `float_leaf` membrane).
- **LAW:** a gate prevents the toxin from crossing. When a bug slips through, find the **gate that failed** —
  never patch the symptom past the barrier.

## §7 — Revascularize, don't scaffold · growing branches
*Deepens the Regenerative Heuristics (ARCH-006).* A new branch (e.g., video editing in the DAW) does **not**
scaffold new rendering logic.
- **LAW:** scan the proven quarries; take the healthy pipeline from the 3D asset tool and physically **reroute
  its data flow** (its vessels) into the new timeline. Clone healthy DNA; grow the systemic network. Never a
  prosthetic limb.

## §8 — Foundation-up corollary · aorta before capillary
*Why the daemon index executes bottom → top.* Every capillary-bed app is fed by the ONE heart through the ONE
arterial bus, so the organism grows **aorta → arteriole → capillary**, never the reverse.
- `/prime-context` is the **aorta** — the prime user's invariant entryway; it is aligned before any branch, and
  every session's cognition is loaded *through* it.
- **Primitives before Monoliths is vascular order.** A capillary (e.g., the studio resize/strobe render path)
  whose feeding artery is not yet aligned can only be symptom-patched — patching it first is **circumvention**
  (Signal Law).
- **LAW:** the ROADMAP index executes bottom-up; the active head sits at the **lowest un-aligned artery**, not
  the loudest symptom.

---

**Three lanes, one membrane** (bus-binding canon, `corpse-assimilator/SKILL.md:86`): `integer_sot` (120 Hz sim;
**only it writes sim**) · `render.*` (reads the integer sim buffer, zero sim authority) · `float_leaf`
(foreign / float sim-authority → **reject** at the membrane). Every primitive declares its lane.

**Crate → organ fast-map:** Heart/Kidneys → `pp-math` + `forge-hal` (TripleBuffer) + `forge-studio` DET-CLOCK ·
capillary beds → `forge-gui` kits · DAW (`forge-audio`) · 2D/3D creators · `termithesia` · the Vixiscript
playground · mRNA → `forge-vix` / `scc` · barrier → the `<gates>` (Sound / Vision / Lock-Free / Pass) ·
revascularize → `forge-furnace` / `forge-export` reuse.

**Loaded by the aorta:** `/prime-context` Phase-4 carries the compressed organ→gate lattice of this tablet, so
every session enters doctrine-aligned (see `.claude/skills/prime-context/SKILL.md` Part 0a).
