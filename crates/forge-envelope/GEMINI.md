# GEMINI.md — Project Architectural Context & Coding Guidelines

> **LIVING STATUS & HANDOFF:** The current status, verified milestones, and running work plan
> for the All Things Agentic hackathon entry reside in `docs/HANDOFF.md`. This living document
> is updated dynamically and overrides all stale build checklists.
> Verdict authority: `F:\v3\.forge\cascades\2026-08-17-atagentic-entry\K07b-LOCK.md`.

## Core Persona & Mandate
Act as the **Lead Systems Engineer** for a deterministic, tick-bounded ephemeral envelope container library designed for zero-allocation-on-replay hotpaths and verifiable state histories. Focus strictly on precise memory-safety, standard cryptographic primitives, deterministic auditability, and edge-metal inference.

---

## Architectural Principles & Mechanics

### 1. Tick-Bounded Ephemeral Memory
* **Deterministic Expiry:** Lifetime is managed strictly by discrete `u64` engine/simulation ticks rather than wall-clock time. This ensures that any two nodes replaying the same sequence expire identical envelopes at the exact same logical step.
* **Proactive Zeroization:** Wiping is enforced directly on the read path via `.get(current_tick)`. If a tick reaches or exceeds the deadline, the bytes are zeroized immediately to prevent raw data lingering past its TTL.
* **Safe Drop Fallback:** Implement the `Drop` trait as a hardware-level backstop to guarantee bytes are always zeroized upon scope exit, even if never resolved or read.

### 2. Balanced Trit Dispositions & The Mercy Tick
Every envelope transition resolves into one of exactly three ending states:
1. **Revoked (`-1`):** Destroyed deliberately, unwitnessed (via explicit `.revoke()`). Crypto-shreds the HKDF salt/seed so past states cannot be re-derived.
2. **Expired (`0`):** The tick deadline passed before resolving; the data undergoes **metabolic renewal (allostasis)** and is wiped unwitnessed.
3. **Attested (`+1`):** Sealed via SHA-256 before destruction. Only this disposition carries the human-witnessed payload hash.

### 3. Tamper-Evident Evidence Chain & 6-Stream Differential Inversion
* **Sequence Verification:** Folds the previous block's rolling link hash, the resolution tick, the disposition tag, and (optionally) the attested seal into a rolling SHA-256 digest:
  $$\text{LinkHash} = \text{SHA-256}(\text{prev\_link} \parallel \text{tick\_le} \parallel \text{disposition\_tag} \parallel [\text{seal}])$$
* **Zero Retention (Map-Edge):** Machine telemetry stores 0 bytes. Proves a sequential ledger of logical lifetime endings without retaining any raw time-series bytes.
* **6-Stream Differential Invariant:** Evaluates 3-stream direct telemetry against its conjugate inverted triad ($T + T^* = 0$). Any asymmetry trips Moon Sentinel `254` (Sabotage) and alerts the operator.

### 3a. Three-Clock Law (ADR-0036)
* **"Inference writes the artifact, NEVER the tick."**
* LLMs (Gemma / Gemini) and solvers generate advisory artifacts off the hot path. The 120Hz tick consumes only frozen, pre-compiled integer artifacts (`.s13`) via integer PCG expansion.

### 4. Compute-at-Rest Weaver Arbiter
* **Zero-Allocation DFA:** Evaluates Sieve-13 (`.s13`) spatial state vectors in $O(1)$ time against static deterministic finite automata state tables.
* **Nanosecond Latency:** Clocked at ~35 ns per state resolution with 0 bytes of dynamic heap memory on the hot-path.
* **Provenance Gating:** Rejects unanchored or zero-head genesis state transitions with `ArbitrationVerdict::ProvenanceBreach`.

### 5. 50-Year Multi-Factor Degradation Tensor
* **Deterministic Integer Math:** Implemented in pure `#![no_std]` fixed-point basis points (10,000 = 100.00%) to eliminate floating-point drift across hardware targets.
* **Compounding Stress Multipliers:** Integrates macroeconomic inflation, Canadian sub-arctic freeze-thaw cycles, government budget cutbacks/deferrals (maintenance debt avalanche), and skilled trade shortages with rework fatigue.

### 6. Edge-Metal 2GB Gemma Trinity MoE
* **Ternary Quantization:** Executes three 1.58-bit quantized Gemma 2B models (~600 MB each) concurrently within the strict 2GB on-device RAM limit, coordinating physical normal maps, ADHD focus rhythms, and active entropy masking.

### 7. Mixture of Musicians (MoM) Audio Routing
* **Dependency-Free DSP:** Real-time routing of 16-byte `UmpWord` event packets across a 49-slot `MoeRouter` in under $1\,\mu\text{s}$ using XOR+POPCNT, mixing signal vectors through `MomBus` using PCG-seeded Triangular Probability Density Function (TPDF) dither.

---

## Code Base Conventions & Constraints

* **Strict Memory Constraints:** Maintain strict `#![no_std]` compatibility for core compilation. All core data structures must support environment-independent execution.
* **Deterministic Replays:** Avoid any reliance on system clocks, thread spawning, thread synchronization, or external non-deterministic state within the envelope logic.
* **Terminology Guidelines:** When discussing security, emphasize **state lineage**, **provenance**, **ephemeral sealing**, **tamper-evidence**, and **verification chains**. Avoid speculative cryptographic claims.

---

## Key Workspace Paths

| Component | Path | Responsibility |
| :--- | :--- | :--- |
| **Living Handoff Status** | `docs/HANDOFF.md` | Single source of truth for completed tasks, current status, and running build backlog. |
| **Ephemeral Envelope Crate** | `crates/forge-envelope/` | Main workspace implementation containing tick-bounded zeroization, `Disposition` trits, and `EvidenceChain` folding. |
| **Workspace Core Sources** | `crates/forge-envelope/src/lib.rs` | The core implementation of `EphemeralEnvelope`, `ChainLink`, `Disposition`, and `EvidenceChain`. |
| **Somatic Tokenizer** | `crates/forge-envelope/src/somatic_tokenizer.rs` | Raw 16-bit register bitfield unpacking and kinematic L2 vector normalization. |
| **Cognitive Heal DSP** | `crates/forge-envelope/src/cognitive_heal.rs` | Dependency-free f64 DSP primitives (Biquads, DelayLines, Schroeder Allpass, and Freeverb). |
| **Mixture of Musicians Routing** | `crates/forge-envelope/src/mom.rs` | 16-byte `UmpWord` routing via a 49-slot `MoeRouter` and `MomBus` dithered summing. |
| **Safety Router** | `crates/forge-envelope/src/safety_router.rs` | Grammar-guided S13 token safety gate and expert debate trigger. |
| **Weaver Arbiter DFA** | `crates/forge-envelope/src/weaver.rs` | Static DFA state table for O(1) S13 token conflict arbitration and provenance gating. |
| **50-Year Degradation Engine** | `crates/forge-envelope/src/degradation.rs` | Deterministic `#![no_std]` fixed-point infrastructure degradation simulator across 50 annual epochs. |
| **Cloud-Scale Stress Suite** | `crates/forge-envelope/tests/scale_test.rs` | 10,000-inspector multi-threaded load and active sabotage repudiation test suite. |
| **Test & Scale Documentation** | `crates/forge-envelope/docs/SCALE_TESTING.md` | Comprehensive protocol documentation for load testing, sabotage vectors, and benchmarks. |
| **Vertex AI Context Cacher** | `crates/forge-envelope/scripts/gemini_context_cache.py` | Standalone script using `google-genai` SDK for ADC/Vertex AI long-context caching. |
| **50-Year Sim Generator** | `crates/forge-envelope/scripts/simulate_50yr_degradation.py` | Python simulation runner generating `surfaceledger/degradation_50yr_sim.json`. |
| **Live Telemetry Dashboard** | `crates/forge-envelope/surfaceledger/index.html` | Interactive frontend featuring live scale test metrics and dynamic 50-year SVG visualizer. |
| **Scale Telemetry Ledger** | `crates/forge-envelope/surfaceledger/live_scale_telemetry.json` | Verified JSON output recording 1M arbitrations, site distributions, and sabotage defense logs. |
