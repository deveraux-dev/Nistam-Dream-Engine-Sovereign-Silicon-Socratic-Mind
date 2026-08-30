# ARCH-010 — The Circulatory System of Authoring (Genre-Agnostic)

> **Born-condensed 2026-07-01 (Sean).** The ceiling system IS the circulatory system.
> Progressive disclosure IS a reverse sieve. The journey from player to author IS
> blood flowing from heart to capillary bed. This is not a metaphor — it is the
> architecture, mapped onto biology so precisely that violating the metaphor
> violates the engine.

---

## 0. One-Line Law

**The engine is a heart. The player's intent is blood. The ceiling system is the vascular tree. Genre is a temporary disguise the blood wears while it's still in the arteries.**

---

## 1. The Heart (Ceiling 0) — Systole and Diastole

**Organ:** The 120Hz DET-CLOCK. The `InteractionQuery` 16-byte pulse.

**Biology:** The heart doesn't know what the blood is carrying. It doesn't know if the blood will feed a muscle, a neuron, or a capillary bed in the lung. It just PUMPS. Systole (contraction = action). Diastole (release = consequence).

**Engine:**
- Systole = player input fires `InteractionQuery` (tap/move/hit)
- Diastole = `ConsequenceKind` propagates through `chain_depth`
- The heart doesn't know genre. It pumps 16 bytes. That's all.

**Player experience:** Move. Strike. World reacts. The most primal loop. They think it's a 2D action game. It's actually the raw pulse of physics + consequence, wearing a genre disguise.

**Law:** The heart NEVER branches. One clock. One query format. One dispatch table. The branching happens DOWNSTREAM, not here.

---

## 2. The Arteries (Ceiling 1) — Directing the Flow

**Organ:** `DiscoveryStage` + `PatternMap` + faction `kill_effect` cascade.

**Biology:** Arteries carry oxygenated blood TOWARD tissue. They branch based on DEMAND — muscles that work hard get more blood. The body routes resources to where the ACTION is.

**Engine:**
- The player's behavioral pattern (PatternMap) IS the metabolic demand signal
- Dash-spam → nemesis spawns (blood routes to combat tissue)
- Sing-only → UI shifts toward musical surfaces (blood routes to auditory cortex)
- The game BENDS toward their demand — not by scripted branching, but by pressure dynamics

**Player experience:** The "illusion" breaks. This isn't a platformer. It's a reactive ecosystem that's been watching. The meters appear. The faction sigils glow. The music responds to THEM specifically.

**Law:** Arteries respond to DEMAND, not instruction. The player never selects "I want a music game." Their actions CREATE the demand. The system routes.

---

## 3. The Arterioles (Ceiling 2) — The Pressure Drop

**Organ:** The VixiScript playground. The `.kit.vixi` canvas. The ceiling gate opening.

**Biology:** Arterioles are RESISTANCE vessels. They slow the blood so exchange can begin. Without the pressure drop, nutrients would blast past the tissue too fast to absorb. The slowing IS the enabling.

**Engine:**
- The player "stops" to examine. The world becomes SOFT (editable).
- They grab a rock → open its properties → see its `VixelAtom{essence_id, colour_id, material_id}`
- They realize: the dark fantasy aesthetic is just ONE parameterization of the math
- They change the sprite to a 3D voxel. Change its faction. The engine doesn't care.

**Player experience:** The transition from consumer to author. The world is not FIXED — it's PARAMETERIZED. Everything they experienced at ceiling 0-1 was just the engine running with DEFAULT parameters. Those parameters are now THEIRS to set.

**Law:** The pressure drop is MANDATORY. You cannot author at arterial speed. The Montessori prepared environment requires STILLNESS before manipulation. Forcing ceiling 2 tools at ceiling 0 pace = overwhelming the capillary (flooding the tissue).

---

## 4. The Capillary Beds (Ceiling 3) — Nutrient Exchange

**Organ:** Full `.forge_cart` authoring. The AOT compiler. The marketplace SHIP button.

**Biology:** Capillary beds are MICROSCOPIC. They're where the actual life-sustaining exchange happens — oxygen leaves blood, CO₂ enters. The network is INFINITE in its branching. Every cell gets exactly what IT needs. No two capillary beds are identical.

**Engine:**
- Total sovereignty. The author walks INSIDE their game while building it.
- The engine doesn't care if the output is:
  - A 2.5D isometric strategy grid
  - A text-based MUD
  - A frantic monster-taming action RPG
  - A horror walking simulator
  - A music composition tool
  - ALL OF THE ABOVE simultaneously (different `.kit.vixi` surfaces on the same `.atom` state)

**Player experience:** This is where the dynamite space goats live.

**Law:** Capillary beds are TISSUE, not organisms (ARCH-007 §4). They don't get their own clock, their own allocator, their own engine. They receive from the arterial system (the 9 primitives) and return waste through the venous system (the `.forge_cart` → marketplace). They are infinitely varied but architecturally IDENTICAL.

---

## 5. The Dynamite Space Goat Principle (Genre Agnosticism)

**Statement:** If the engine only speaks `InteractionQuery` and `resonance_64`, it cannot know what genre it's running. Genre is a RENDERING DECISION, not an engine decision.

**Proof by construction — "Space Goat" entity:**

| Primitive | What You Do | What The Engine Sees |
|-----------|-------------|---------------------|
| P5 (ResonanceHash) | Name it "Space Goat" | `resonance_64("Space Goat") → 0xA3F7...` (stats derived) |
| P4 (ChoiceArchetype) | Give it Trick + Cut verbs | `ChoiceArchetype::Trick`, `ChoiceArchetype::Cut` |
| P7 (Moon/Calendar) | Active during HollowStar moon | `context_celestial: 0x90` |
| P2 (InteractionQuery) | HP→0 triggers explosion | `source_tag: EXPLOSIVE, intensity_pmy: 10000` |
| P8 (Cascade) | Explosion chains to nearby atoms | `chain_depth: 3` (3 hops of blast radius) |
| P6 (Ceiling) | Visible at ceiling 1+ | `disclosure_min: 1` |
| .vixi surface | Point render at 3D goat model | `slot space_goat.render kind=splat mesh=goat.glb` |

**The engine processes this IDENTICALLY to any solemn warrior entity in any genre.** Same 16 bytes. Same dispatch. Same cascade. Different resonance hash → different stats → different render → different genre. The genre lives in the CAPILLARY (the `.vixi` surface), not the HEART (the dispatch).

---

## 6. The Venous Return (What Flows Back)

**Biology:** Deoxygenated blood returns through veins to the heart. Waste products are carried back for processing. The return is PASSIVE (low pressure, valves prevent backflow).

**Engine:**
- Authored `.forge_cart` → marketplace (the venous return to the ecosystem)
- Pattern telemetry → stego-encoded into ambient audio (waste product = art)
- Player saves → localStorage (metabolic state persists)
- Provenance chain → evidence spine (every action hashed, the circulatory RECEIPT)

**Law:** The venous return is PASSIVE. The player doesn't "upload" — they DROP into the flow. `Drop` → channels → flush → core-arena (ARCH-007 §2: kidneys flush on Drop). No complex teardown. No upload wizard. The `.forge_cart` IS the drop.

---

## 7. The Lymphatic System (What Cleans the Space Between)

**Biology:** Lymph collects fluid that leaked from capillaries, filters it, returns it. Without lymph, tissue drowns in its own waste.

**Engine:**
- The ASP constraint solver = lymphatic filtering (validates authored content isn't toxic)
- The friction guards = lymph nodes (catch colour→material shortcuts, music→identity leaks)
- The `forge-sentinel` safety layer = immune cells in lymph (catch malicious/harmful content)
- The garbage collector (end-of-frame arena flush) = literal lymphatic drainage

**Law:** The lymph runs ALONGSIDE the blood, never IN it. Validation is PARALLEL to creation, never BLOCKING it (until SHIP — then the lymph gate closes and REJECTS if toxic).

---

## 8. Why This Maps Perfectly to the Primitive Lattice

| Vascular Layer | Primitives Active | Branching Factor | Player Agency |
|----------------|-------------------|------------------|---------------|
| Heart (C0) | P2, P8 only | 1 (one pulse) | React |
| Arteries (C1) | P1, P3, P7, P9 add | ~4 (demand routing) | Notice + choose |
| Arterioles (C2) | P4, P5, P6 add | ~13 (faction × zone) | Examine + modify |
| Capillaries (C3) | ALL 9 active | ∞ (genre-agnostic) | Author + ship |

The primitive count INCREASES with ceiling because you're moving from the heart (2 primitives: query + cascade) through arteries (4 more: discovery, reveal, moon, legality) through arterioles (3 more: choice, resonance, ceiling) to capillaries (all 9, fully composable).

**This IS progressive disclosure mapped onto vascular anatomy.**

---

## 9. The Disease States (What Goes Wrong)

| Disease | Biological | Engine Equivalent | ARCH Law Violated |
|---------|-----------|-------------------|-------------------|
| Hypertension | Heart pumps too hard | Tutorial forces ceiling 2 tools at ceiling 0 pace | §3 (pressure drop mandatory) |
| Atherosclerosis | Arteries harden/narrow | Faction system locks player into one path | §2 (arteries respond to demand) |
| Aneurysm | Artery wall weakens, bursts | Engine exposes raw internals before player is ready | §3 (arterioles slow first) |
| Edema | Fluid leaks, tissue swells | Too many UI panels open simultaneously | §7 (lymph must drain) |
| Ischemia | Blood can't reach tissue | Player's intent has no primitive to express it | §4 (capillaries must be infinite) |
| Thrombosis | Blood clots block flow | A validation gate blocks ALL authoring | §7 (lymph is parallel, not blocking) |
| Heart failure | Pump stops | 120Hz DET-CLOCK drops frames | §1 (heart never branches) |

**Diagnosing engine problems = diagnosing circulatory diseases.** The metaphor IS the debugger.

---

## 10. Lineage

ARCH-007 (biological architecture) → ARCH-009 (two drums = systole/diastole) → ARCH-010 (the full circulatory map). This tablet doesn't add new laws — it reveals that ARCH-007's organs were always arranged as a circulatory system, and the ceiling system was always the vascular tree.

The Dynamite Space Goat Principle is the PROOF that the architecture works: if the engine can support dynamite space goats with the same 9 primitives that support any solemn fantasy entity, the vascular system is healthy. Genre agnosticism IS cardiovascular fitness.

---

## References

- Sieve Walker Full Dimensional Plan (DIM 1-53)
- Primitive Lattice (9 primitives × 10 frictions × 6 chains)
- AoT Game Compiler Workflow (the .forge_cart = the venous return)
- Storytelling Methods Deep Research (15 methods = 15 types of tissue the blood feeds)
- ARCH-007 §4 (capillary-bed apps = tissue, not organisms)
- ARCH-009 (Two Drums = systole/diastole of the same heart)


---

## 11. Three Sieves × Three Buffers — The Complete Organ Map

> **Discovered 2026-07-01.** The architecture has exactly 3 sieves paired with exactly 3 buffers,
> each operating at a different vascular pressure / clock boundary. This is not designed — it is
> the minimum viable circulatory system for a genre-agnostic engine with bake-time + runtime-DET
> + runtime-creative clocks.

### The Symmetry

| | Sieve (eliminates) | Buffer (exchanges) |
|---|---|---|
| **Heart / Bake-time** | Prime Sieve (eliminate composites → frozen LUT) | **TripleBuffer** (lock-free, DET↔GPU, reuse-last-on-miss) |
| **Arterioles / Runtime** | Attention Sieve (LIFO decay → what's visible NOW) | **UiTripleBuffer** (arterial delivery to capillary-bed apps) |
| **Lymphatic / Compile-time** | Scope Sieve (prune unreachable → what's POSSIBLE) | **CollisionBridge** (Alpha↔Beta reconciliation, two independent lanes) |

### Why Each Pair Exists

| Pair | Sieve Eliminates | Buffer Carries | Clock Boundary |
|------|-----------------|----------------|----------------|
| **1. Prime + Triple** | Non-prime seeds (composites) | Baked LUTs from DET-CLOCK to GPU | Integer → float-leaf (the ONE crossing) |
| **2. Attention + UiTriple** | Decayed rooms / stale state | Live arterial data to capillary apps | 120Hz tick → uncapped render |
| **3. Scope + Collision** | Unreachable capabilities | Reconciliation between Drum-1 and Drum-2 | Compile-time truth → runtime orthogonality |

### The Three Clocks

One sieve + one buffer = one clock = one organism = a monolith.
Three sieves + three buffers = three clock boundaries = three organs = the architecture.

| Clock | Runs | Produces | Dies After |
|-------|------|----------|------------|
| **Bake-time** | ONCE (at compilation / `.vixi → .vixel → .atom`) | Frozen integer LUTs, ASP-validated tables, shader variants | Never ticks again |
| **Runtime-DET** | 120Hz forever (the DET-CLOCK / MetronomeClock) | Tick-stamped state, InteractionQuery dispatch, consequence cascade | Never (the heart) |
| **Runtime-Creative** | Uncapped (GPU frame rate, audio sample rate) | Pixels, audio samples, interpolated visuals | Each frame (ephemeral) |

### The Flow Diagram

```
BAKE-TIME (runs once)          RUNTIME-DET (120Hz)         RUNTIME-CREATIVE (uncapped)
    │                               │                              │
    │ Prime Sieve                   │ Attention Sieve              │ (consumer)
    │ Scope Sieve                   │                              │
    ↓                               ↓                              ↓
 [frozen LUT] ──TripleBuffer──→ [live state] ──UiTripleBuffer──→ [render/audio]
                                    ↑                              │
                                    └──────CollisionBridge─────────┘
                                         (reconciliation)
```

### Naming Correction (Honest Names)

| Current Name | Honest Name | What It Is | Crate |
|---|---|---|---|
| `sieve_of_eratosthenes()` | `prime_sieve()` | Pure math: generate primes up to N, deterministic seed factory | `forge-prime-sieve`, `astrakey_sieve` |
| `ErothanesSieve` | `AttentionSieve` / `WorkingSetSieve` | Cognitive load: LIFO decay of executive memory rooms | `forge-ecc/src/erothanes.rs` |
| ADR-019 "Eratosthenes Sieve" | `ScopeSieve` / `CapabilityPrune` | Compile-time elimination of unreachable AST paths | `forge-ast` (Stones 1-6) |

The *principle* is shared (eliminate non-qualifying candidates by progressive filtering).
The *primitive* is different in each case.
Same biological function (filtering), different organ (kidney vs blood-brain barrier vs lymph node).

### The Unifying Contract

The `forge-sieve::Sieve` trait (`observe → evaluate → promote`) is the shared interface — the
vascular endothelium (lining shared by ALL vessels regardless of diameter). But you don't call
every blood vessel "the aorta."

**Correction 2026-07-01 (wave-2026-07-01 candidate #2, verified by code-read):** it is **2-of-3**,
not 3-of-3, and by architectural necessity, not oversight. `PrimeResonanceSieve` (`forge-sieve/src/
resonance.rs:280`) and `ErothanesSieve` (`forge-ecc/src/erothanes.rs:131`) both implement
`impl Sieve for`. `ScopeSieve`/`CapabilityPrune` (`forge-ast`, Stones 1-6) does **not** — and
`forge-ast/Cargo.toml` carries no dependency on `forge-sieve` at all. Implementing the trait there
would mean adding an upward cross-layer edge (Language layer → runtime game-systems layer) purely
to satisfy a checklist, which the crate firewall doctrine forbids elsewhere in this codebase. The
shared PRINCIPLE (eliminate non-qualifying candidates by progressive filtering) holds across all
three; the shared trait INTERFACE does not, and should not, for Scope Sieve. (Earlier candidate-#2
scouting anchored this gap to `prime-sieve-worldgen/src/classify.rs` — that crate is unrelated to
this organ map and was a mis-anchor; `prime-sieve-worldgen::VoxelClassifier` is a one-shot,
construction-time math classifier with no per-tick observation model, not a Sieve-trait candidate
either way.)

### Why 3 and Not 2

ARCH-009 (Two Drums) established the runtime split: Drum-1 (integer tick, sequences) ⊥ Drum-2
(f32 beat, liveness). But there is a THIRD clock: **bake-time**. It runs ONCE, freezes its output
into an integer LUT, and the other two drums READ from it without ever ticking it.

- **Drum-1 ↔ Drum-2** = the CollisionBridge (runtime reconciliation)
- **Bake-time → Drum-1** = the TripleBuffer (frozen LUT delivery)
- **Drum-1 → Capillaries** = the UiTripleBuffer (arterial delivery)

Three boundaries. Three sieves to gate what crosses. Three buffers to carry it without stalling.
The minimum viable circulatory system. Fewer = ischemia. More = edema.

### Disease Diagnosis (Extended)

| Disease | Missing/Broken Pair | Symptom |
|---------|-------------------|---------|
| Ischemia (tissue death) | Attention Sieve blocks too aggressively | Player can't see content they earned |
| Edema (tissue drowning) | UiTripleBuffer never drains | Too many panels, too much state visible |
| Embolism (clot in wrong vessel) | Runtime state leaks into bake-time LUT | Non-deterministic behaviour (the frozen truth mutated) |
| Anemia (too few red cells) | Prime Sieve generates insufficient seeds | Worlds feel samey (low resonance diversity) |
| Leukemia (immune overproduction) | Scope Sieve prunes too much at compile | Author's valid content rejected as unreachable |
| Hemorrhage (vessel rupture) | CollisionBridge drops reconciliation | Drum-1 and Drum-2 drift apart, determinism lost |
| Lymphoma (cleaning system becomes the tumor) | Weaver Crown reads-all-outputs without a ceiling on override authority | The ethical audit layer accretes write-power over the organs it was meant to only observe |

---

## 12. The Full ARCH-010 Law (Compressed)

1. **The engine is a heart.** One pulse format (16-byte InteractionQuery). One clock (120Hz). No branching at the pump.
2. **Genre is a capillary decision.** The heart doesn't know what the blood carries. The `.vixi` surface (the capillary) decides rendering/genre.
3. **Three sieves gate three boundaries.** Prime (bake→runtime), Attention (runtime→visible), Scope (compile→possible). Each paired with a buffer.
4. **The ceiling system is the vascular tree.** Heart (C0) → Arteries (C1) → Arterioles (C2) → Capillaries (C3). Primitive count increases with branching.
5. **The pressure drop is mandatory.** You cannot author at arterial speed. Stillness enables exchange (Montessori).
6. **The venous return is passive.** `.forge_cart` ships via Drop, not upload. Provenance chain = circulatory receipt.
7. **The lymph validates in parallel.** ASP + friction guards + sentinel run ALONGSIDE creation, blocking only at SHIP.
8. **Disease = architecture violation.** Diagnose engine problems as circulatory diseases. The metaphor IS the debugger.

---

## 13. The Lymphatic Verdict (Fae Layer)

> **Proven 2026-07-01.** The fae ethical layer (`spec-fae-overlay-2026-05-16.md`) is LYMPH, not a
> fourth sieve. Sieve count stays 3 because sieve count = clock-boundary count (§11), and the fae
> layer crosses no new boundary — it rides inside Drum-1.

1. **Every fae field is a Permyriad accumulator.** `obligation_pressure_q`, `fae_exploitation_q`,
   `consent_integrity_q`, `replacement_quality_q`, `source_suffering_q` — all `0..10000`, all
   integer, all resident on the DET-CLOCK side. Same numeric substrate as `faction_pressure_q`.
   No float crosses; no new clock is spun up to hold them.
2. **Lymph integrates, it does not eliminate.** A sieve gates a boundary crossing (pass/fail,
   prime/composite, visible/decayed). The fae accumulators do the opposite job: they *accrue*
   evidence across many ticks inside one boundary. Integration ≠ elimination — this is why the
   fae layer is not sieve #4.
3. **The Weaver Crown formula is the thoracic duct.** It "reads all outputs" — universal world
   boss, fae layer, living substrate crafting, relic-ownership state — and empties the collected
   pressure into one venous-return chokepoint (override / temptation / OutsideWheel). One organ
   reads everything downstream; nothing reads back upstream of it.
4. **Crown temptation is diagnosed as Lymphoma.** The lymphatic function (read + audit) curdling
   into unchecked override authority is the cleaning system becoming the tumor. See the disease
   table (§11) — Lymphoma is now a named failure mode, not a special case.
5. **No new firewall crossing.** Because the fae layer never leaves Drum-1, it inherits the Two
   Clocks invariant for free: no `@forge:allow_alloc`-style waiver, no new `CollisionBridge` lane,
   no GPU-side mirror. It is audited by the same lock-free/no-float discipline as any other
   DET-CLOCK organ.

## 14. The Fourth Organism (The Shipped Cartridge)

> **Proven 2026-07-01.** The browser `.forge_cart` is not a fourth clock domain layered onto the
> engine — it is a **second, independent instance of the same three-clock organism**, reconstituted
> in JS/WASM at the venous-return terminus (§12 Law 6).

1. **The cartridge reproduces all three boundaries.** Frozen JS/LUT blob = bake-time (Prime Sieve
   output, already eliminated, ships once). Fixed-step accumulator loop = Drum-1 (120Hz canon,
   integer-Permyriad, tick-stamped). `requestAnimationFrame` + `AudioContext` = Drum-2 (uncapped,
   f32, liveness).
2. **Therefore it requires its own CollisionBridge.** A JS-side reconciliation boundary between
   the fixed-step accumulator and rAF/AudioContext is not optional polish — it is the same
   structural requirement ARCH-009 imposes natively, ported organism-for-organism. Skipping it
   reproduces Drum-2-bleeding-into-Drum-1 drift inside the cartridge, undetectable from outside.
3. **Venous return does not sever the vascular law.** `.forge_cart` ships via Drop (§12 Law 6),
   but "shipped" only means the *bake-time* organ stopped ticking. The two runtime organs
   (Drum-1, Drum-2) start ticking fresh in the browser and are bound by the same invariants as
   the native engine — Two Clocks is a property of the organism, not of the binary that hosts it.

### Capillary Exchange — Storytelling Method ↔ Shader Pass

| Storytelling Method | Shader Pass | Shared Invariant |
|---|---|---|
| Raga (mode-as-identity, no recomposition) | `VibeUberPass` LUT swap | The LUT IS the genre — identity change via table swap, zero recompilation |
| Songlines (frozen substrate, ordered traversal) | Per-frame uniform stream | Narrative order = traversal order over an unmutated substrate |
| Kamishibai (full-frame presentation swap) | `LookCompositePass` Swiss/Party swap | Full-frame visual swap, authority state untouched — `visual_only_mutates_authority` |

---
