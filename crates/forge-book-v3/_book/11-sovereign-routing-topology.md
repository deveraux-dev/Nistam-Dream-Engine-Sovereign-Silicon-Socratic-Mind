# XV · Sovereign Routing Plane and Offline Inference Topology

This chapter formalizes the architectural layout of the 13Forge routing plane, details its multi-tier expert, safety, and consequence routing integration with the 3-tier Mixture of Experts (MoE) dual-distillation flywheel (identifying our local `sovereign.nde` models), maps the client boot and execution lifecycles (across `13forge-studio.exe`, `nde_chat`, and `glass-terminal`), and specifies the execution of the Six-Pattern Directed Acyclic Graph (DAG) task orchestrator.

---

## 1. The 3-Tier NDE Local Model Ladder & Offline Inference Matrix

To achieve 100% offline edge autonomy, the 13Forge routing and inference architecture is organized into three disjoint, cascading local execution tiers, followed by an external cloud oracle fallback. Each tier corresponds to an explicit on-disk `.nde` model file and matches a specific latency, resource, and cognitive threshold [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:46].

```
       [ USER / CLIENT INPUT ]
                  │
                  ▼
   ┌──────────────────────────────┐
   │        STUDENT TIER          │
   │  • File: sovereign.nde       │ ──► [ 23.8M params, <1µs BQ dispatch ]
   │  • bq_router (CPU-only)      │ ──► [ Centroid check prior to GPU ]
   │  • OneByteExpert Classifier  │ ──► [ Screens raw bytes for danger ]
   └──────────────┬───────────────┘
                  │ (Fallback / Escalation)
                  ▼
   ┌──────────────────────────────┐
   │        TEACHER TIER          │
   │  • File: teacher.nde         │ ──► [ ~500M params, LOCAL MoE router ]
   │  • GPU-based execution       │ ──► [ 9 experts, d_model=512, Q8Bf ]
   │  • TIER2_TOKENIZER (BPE)     │ ──► [ 16,384 vocab, context blocks ]
   └──────────────┬───────────────┘
                  │ (Validation Failure / Background Synthesis)
                  ▼
   ┌──────────────────────────────┐
   │         MASTER TIER          │
   │  • File: master.nde          │ ──► [ Compiled parametric apex, Q8 ]
   │  • LoraAdapter Hot-Fusion    │ ──► [ Dynamic domain-pivot weight sync ]
   │  • Lazy-load & Hotswap-ready │ ──► [ gpu-warden opaque-semaphore ]
   └──────────────────────────────┘
```

### The Student Tier: sovereign.nde (Sub-1µs Deterministic Pre-Filtering)
At the outermost frontier of the client, incoming prompts and system payloads are evaluated completely on the CPU without allocating GPU memory or launching neural network inference:
*   **`sovereign.nde` Model:** Our 23.8M parameter byte student model [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:50]. It operates at the byte-level (vocab_size=256) over a sequence length of 2,048, utilizing a 7-expert DCGS topology [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:58].
*   **`bq_router` Centroid Routing:** A binary-quantized CPU-only meta-router that replaces high-latency quadratic routers with sub-100ns Hamming distance checks [PROVEN:F:\NewRepo\crates\forge-ml\src\bq_router.rs:1]. Incoming queries are projected into a 512-bit (64-byte) binary manifold, matching against 7 domain specialist centroids using fast XOR + POPCNT instructions to determine specialist affinity prior to any model dispatch [PROVEN:F:\NewRepo\crates\forge-ml\src\bq_router.rs:14].
*   **`OneByteExpert` (ByteSequenceClassifier) Safety Gate:** An independent, un-tokenized multi-layer perceptron (MLP) that pre-screens raw UTF-8 byte sequences to assign binary (SAFE/DANGER) verdicts [PROVEN:F:\NewRepo\crates\forge-ml\src\byte_classifier.rs:1]. Operating directly on raw bytes isolates the system from tokenizer-bypassing payload injection attacks.
*   **Static Lexical Mapping:** Grounded by `dictionary.txt` [PROVEN:_vault/output/corpus/corpus/dictionary.txt:1]. Standard programming terms, file extensions, and English vocabularies are mapped directly to static token boundaries, avoiding real-time subword tokenization at the client frontend.

### The Teacher Tier: teacher.nde (Apex Local MoE Router)
When the student tier detects a complex query, the execution is escalated to the local GPU-based inference loop:
*   **`teacher.nde` Model:** Our local GPU-accelerated MoE model with ~500M parameters, quantized at Q8Bf [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:70]. It features 9 experts, 20 layers, a hidden dimension `d_model` of 512, and a vocabulary size of 16,384 [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:74]. It serves as the apex local validator and the neural teacher for the dual-school distillation flywheel [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:104].
*   **Teacher-Level Tokenization:** Embeds a v16384 BPE model (`TIER2_TOKENIZER`) to perform local semantic clustering and context-bundling [PROVEN:crates/daemon/infer_thread.rs:906].

### The Master Tier: master.nde (Resident Parametric Apex)
*   **`master.nde` Model:** Compiled from PyTorch/SafeTensors checkpoints (such as `ppl108/best_model.pt`) using the `compile-nde` utility, targeted at Q8 quantization [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:129]. It is lazy-loaded and hotswap-ready behind the `gpu-warden` opaque-semaphore [PROVEN:_vault/_plans/archive/ADR-0019-nde-model-tier-registry.md:118].
*   **`LoraAdapter` Hot-Fusion:** Low-rank adapters are fused directly into base linear layers in-memory ($W_{\text{effective}} = W + (B \times A) \times \frac{\alpha}{r}$) [PROVEN:F:\NewRepo\crates\forge-ml\src\lora.rs:1]. Zero-initialization of matrix $B$ ensures smooth convergence [PROVEN:F:\NewRepo\crates\forge-ml\src\lora.rs:8]. Adapters add net-new capacity to bridge selective knowledge gaps [PROVEN:F:\NewRepo\crates\forge-ml\src\lora.rs:11] and are updated locally using `corpus_live.jsonl` streams, allowing the engine to pivot its specialization (e.g., from Acoustics to Vixi graphics) without reloading base weights [PROVEN:F:\NewRepo\crates\forge-ml\src\lora.rs:12].

### Grounding of Resident Development Tools (The Gemma Frontier)
*   **Resident-Optional Integration:** While historically purely offline, foundational models (such as `gemma-3-4b-it`) can now be managed as resident runtime components. They are loaded in-process via `gemma_engine` within the `door-live` container (11.7GB) [PROVEN:F:\NewRepo\crates\forge-daemon\src\gemma_engine.rs:1].
*   **Runtime Capability:** Gemma serves as the runtime core for synthesizing real-time integration plans and context-aware RAG extraction, bridging the gap between design-time synthesis and production-time execution [PROVEN:outland_goldminer_gemma_synthesis.md:50]. Air-gapped builds remain supported via configurable build-gates that exclude the `door-live` container.

---

## 1.1 Dual Cognitive Axes: Semantic (RAG) and Syntactic (DAG) Mapping

The 13Forge cognitive substrate processes instruction sets and environment mutations across two independent, orthogonal mapping axes that match the duality of human/machine alignment:

```
   ┌────────────────────────────────────────────────────────┐
   │                  THE COGNITIVE SHIELD                  │
   ├───────────────────────────┬────────────────────────────┤
   │    SEMANTIC AXIS (RAG)    │    SYNTACTIC AXIS (DAG)    │
   ├───────────────────────────┼────────────────────────────┤
   │  • Content-Addressed      │  • Structural Execution    │
   │  • Semantic Affinity      │  • AST Parser Grammars     │
   │  • Knowledge Atoms (PKM)  │  • Dependency Resolution   │
   │  • Multi-Domain Corpora   │  • ExecutionDag Nodes      │
   │  • Approximate Context    │  • Type-Safe Invariants    │
   └───────────────────────────┴────────────────────────────┘
```

### The Semantic Axis (RAG — Retrieval-Augmented Generation)
The Semantic Axis governs *meaning, associations, intent, and context representation*. It handles approximate, soft-aligned information processing:
*   **`forge-pkm` Integration:** Serves as our dedicated, leaf-level RAG retrieval tier [PROVEN:forge-daemon/Cargo.toml:16]. It indexes developer logs, milestone reports, and session files into content-addressed semantic chunks.
*   **Affinity & Specialist Centroids:** When queries arrive, the `bq_router` CPU checks and semantic clustering algorithms determine conceptual closeness to existing specialized corpora (`corpus_a` through `corpus_v`) [PROVEN:_vault/output/corpus/corpus/corpus_manifest.json:1], performing approximate search and retrieval before escalating to deep local model processing.

### The Syntactic Axis (DAG — Directed Acyclic Graph)
The Syntactic (or Structural) Axis governs *grammar, execution dependencies, constraints, and compile-time rules*. It handles integer-exact, provable, and type-safe process execution:
*   **`forge-dag` Integration:** Orchestrates complex, sequential task execution over native substrates [PROVEN:forge-dag/Cargo.toml:2]. Task flows are mapped to the strict, schema-validated `ExecutionDag` which enforces topological sorting and conflict serialization to ensure compile-safe dependency progression [PROVEN:forge-dag/CLAUDE.md:3].
*   **Grammar & Compilation Gates:** Source files, MUD level graphs (`MudEngine` [PROVEN:F:\NewRepo\crates\forge-studio\src\nde_chat.rs:10]), and graphics parameters are compiled against rigid VixiScript and AST grammars [PROVEN:forge-vix/src/loader.rs:265], ensuring that generated outputs satisfy physical and mathematical invariants before being stamped or integrated [PROVEN:_book/09-architecture-gates.md:65].

---

## 2. Client Boot & Execution Lifecycles (The Entry Points)

The 13Forge application suite is accessible through three distinct client environments, each binding differently to our local NDE models:

### A. Graphical Desktop Monolith (`13forge-studio.exe` / `forge-studio`)
*   **Dual-Clock Metronome Boot:** Automatically instantiates the dual-clock architecture [PROVEN:F:\NewRepo\crates\GEMINI.md:8]. The 120Hz deterministic clock (T1 DET-CLOCK) drives game physics, inputs, and cellular automaton state sweeps [PROVEN:F:\NewRepo\crates\GEMINI.md:8], while the uncapped presentation loop (T3 CREATIVE-LANE) renders the `SovereignWindow` via `wgpu` [PROVEN:F:\NewRepo\crates\GEMINI.md:10].
*   **Triple-Buffer Bridging:** Low-level cellular automaton updates and player movements are serialized as 18-byte `VixelDiff` packets and shared across the thread boundaries using `TripleBuffer` [PROVEN:_book/08-latent-space-collider.md:100]. This eliminates rendering bottlenecks and guarantees zero frame-stutter.
*   **Local NDE Escalation:** Clicks, interactions, or user text commands first hit the `bq_router` CPU centring check on T1. If student classification falls back, the query is pushed to the background `teacher.nde` GPU thread, returning the result to the main presentation scene without dropping frames.

### B. Interactive Adventure Loop (`nde_chat` / `MudChat`)
*   **Deterministic Game Mechanics:** Boots the Quest Finite State Machine (FSM) and drives the ironroot text world using `MudEngine` [PROVEN:F:\NewRepo\crates\forge-studio\src\nde_chat.rs:10].
*   **Click-to-Action Binding:** Verbs (`look`, `strike`, `craft`) are surfaced as clickable graphical buttons [PROVEN:F:\NewRepo\crates\forge-studio\src\nde_chat.rs:6]. When a player clicks a verb, the game mechanics execute deterministically on the **Consequence (Arity 49)** routing axis, resolving outcomes immediately on the local thread without triggering FFI or GPU overhead [PROVEN:F:\NewRepo\crates\forge-studio\src\nde_chat.rs:24].
*   **Sonic Resonance Feedback:** Game actions trigger corresponding musical and acoustic signals. The **ExpertDispatch** axis's `MomRouter` maps current chord positions and game events via `UmpWord` Hamming distances into the GPU frequency-domain synthesis loop `moe-gpu-dsp` [PROVEN:F:\NewRepo\crates\nde_core\src\mom_router.rs:1].

### C. Command-Line Terminal (`glass-terminal` / `forge-tui`)
*   **Vixio Reactor Loop:** Boots the terminal interface within Vixio's deterministic runtime using `Runtime::block_on` [PROVEN:F:\v3\.forge\attic\2026-08-29\vixio-v3\src\lib.rs:20]. Standard OS asynchronous selectors are bypassed; the terminal interface is driven by Vixio's integer tick-clock.
*   **Discrete Input Raycasting:** Key presses and mouse click coordinates are projected into discrete integer volumes using the `TritTree5D` spatial index [PROVEN:_book/08-latent-space-collider.md:150]. This projects continuous inputs into bitwise-precise grid coordinates, preventing rounding errors.
*   **NDJSON Stream Ingestion:** Scans and search queries (driven by `outland-index` and `goldminer-core`) communicate over TCP Port 13013, returning exactly one NDJSON hit line per Vixio tick ($k+1$ for Rank $k$ hits) to space out data ingestion and maintain smooth rendering [PROVEN:outland/EDGE.md:11].

---

## 3. Complete Thread, Tick, and Routing Topology

The following diagram illustrates how our seven-axes routing, tokenization, swarming, and agent pipelines converge to execute tasks within the monolithic stack:

```
   ┌────────────────────────────────────────────────────────┐
   │                  T1 DET-CLOCK (120Hz)                  │
   │  • Drives MetronomeClock & TickEngine (deterministic)  │
   │  • Updates OneGrid cellular-automaton physics          │
   │  • Decodes 18-byte VixelDiff packets into the scene    │
   └───────────┬────────────────────────────────┬───────────┘
               │                                │
               │ (Vixio Cond Wakes)             │ (TripleBuffer Sync)
               ▼                                ▼
   ┌────────────────────────┐      ┌────────────────────────┐
   │     VIXIO REACTOR      │      │  T3 GPU CREATIVE-LANE  │
   │  • Evaluates sleep_ticks│      │  • Runs present loops  │
   │  • Evaluates raycasts  │      │  • Draws native UI     │
   │  • Drains run-queue    │      │  • Computes audio DSP  │
   └────────────────────────┘      └────────────────────────┘
               │
               │ (Off-thread OS channel)
               ▼
   ┌────────────────────────────────────────────────────────┐
   │                BACKGROUND SWARM THREADS                │
   │  • run_findings_swarm: fans out research tasks         │
   │  • SwarmConfig::distillation: caps at 7 concurrent     │
   │  • sovereign.nde & teacher.nde local weights running   │
   └────────────────────────────────────────────────────────┘
```

---

## 4. The Six-Pattern DAG Orchestrator (`orchestrator.rs`)

The `ExecutionDag` is designed to run complex code modification, verification, and synthesis workloads. The orchestrator maps Anthropic's classic task patterns onto our deterministic substrate [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:1].

### A. Graph Execution Nodes
*   **Nodes 1–3 (Fan-Out Coders):** Distribute feature implementation tasks to parallel coder slots, routed under `ModelRoute::Execute` and pinned to Sonnet [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:10].
*   **Node 4 (Adversarial-Verify HALT):** Marked with the `VERIFY_GATE` tag [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:11]. This node executes an adversarial code-review pass. If the design oracle yields a verdict other than `Outcome::Success`, the entire graph halts execution immediately [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:56].
*   **Nodes 5–10 (Scope-Scaling Loops):** Run iterative loop-until-done cycles to refine code interfaces and apply modifications based on the results of the verification gate [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:12].

### B. Core Execution Invariants
1.  **Strict Serialization:** The graph is driven sequentially via `OrchestratorConfig::locked()`, which limits execution to a single active agent, enforces synchronous blocking, and completely forbids sub-agent delegation [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:26].
2.  **Context Isolation:** To prevent file-bloat, code files are isolated during execution; only a typed `Outcome` status and an evidence ledger are returned to the caller [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:8].
3.  **Prompt-Injection Shutoff:** Every node executor compiles a `NodeReport` comprising a verbatim `EvidenceEntry` ledger and a binary `security_flag` [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:75]. If an incoming instruction attempts to exfiltrate state or bypass authorities, the `security_flag` is tripped, causing an immediate hard halt [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\orchestrator.rs:70].

---

## 5. Swarm Orchestration & Lock-Free Parallelization (`swarm.rs`)

To optimize the distillation process, `F:\NewRepo\crates\forge-broski\src\dream\swarm.rs` provides a lock-free, de-tokio-fied parallel execution plane for `DreamDriver` [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:1].

*   **Native OS Thread Fan-Out:** Rather than running on Tokio task pools, the swarm fans out $N$ findings across $M$ concurrent, native OS threads [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:7]. Concurrency is bounded deterministically using a crossbeam-channel permit pool [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:8].
*   **Swarm Concurrency Presets:**
    *   `SwarmConfig::sequential()`: Configured with a concurrency cap of 1 and restricted to critical/high severities [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:44]. This is the only configuration permitted for active, code-writing Dream Worker agents [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:45].
    *   `SwarmConfig::distillation()`: Configured with a concurrency cap of 7 (matching the first rung of our 7-700-7 dual-distillation flywheel) and a highly permissive severity allowlist [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:55]. This is utilized for background knowledge extraction and clustering, where no code is written to disk [PROVEN:F:\NewRepo\crates\forge-broski\src\dream\swarm.rs:56].
