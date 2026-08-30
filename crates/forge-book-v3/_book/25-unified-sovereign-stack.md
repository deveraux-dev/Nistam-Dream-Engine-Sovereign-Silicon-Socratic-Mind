# The Unified Sovereign Stack: Neuro-UDLE, Endless Silk, and the Sensory Neurohud Blueprint

**Date:** Tuesday, August 4, 2026  
**Author:** Oracle B (Cognitive Alignment with Sean's Desk)  
**Status:** Canonical Sovereign Blueprint (Persisted in `_plans/NEW-UDLE-NEUROHUD-DOCTRINE.md` and compiled in `forge-book` Chapter 25)

---

## 1. Executive Summary & Cognitive Alignment (Pure auDHD Flow)

The legacy UDLE (Universal Distillation Ledger Engine) designed in April 2026 has been completely refactored. The three-layer runbook (Claude.ai Web Projects, Code-in-CLI, and Cowork shell automations) has been abolished due to context fragmentation and the high cognitive tax of manual time/ledger entry, which triggers executive dysfunction in an auDHD (Attention Deficit Hyperactivity Disorder + Autistic) developer.

In its place stands the **Unified Sovereign Stack**—a closed-loop, local, offline development and execution environment collapsed entirely into **`13forge-studio.exe`**. 

### The Core Mandates
*   **No Industrial Product:** SLAPP (the industrial digital twin) is entirely deported. All development efforts focus strictly on the pure creative, gaming, and neuro-acoustic stack (*13 Moons Over the Prairie*, *Dead Drop DAW*, and *Technothesia*).
*   **Zero-Sales Passive Distribution:** Reflecting private project memory, there is no marketing, no company-management overhead, and no billing administration. Code acts as the sole beacon. The system compiles standalone releases and deploys them to self-service off-grid mirrors passively.
*   **Autonomic Life-Loop:** No manual time tracking or form filling. The system harvests attention density, compile metrics, and AST deltas passively. It utilizes a local Three-Tier model to auto-synthesize audit-ready SR&ED lines on session exit.

---

## 2. Legacy vs. Modernized UDLE Matrix (The Shift)

| Core Dimension | Stale Legacy UDLE (April 2026) | Modernized Sovereign UDLE (August 2026) |
| :--- | :--- | :--- |
| **Workspace Architecture** | Fragmented across 6 distinct Claude.ai Web Projects and background PowerShell loops. | Consolidated into **`13forge-studio.exe`**—a local, offline, dual-clock sovereign monolith. |
| **Planning & Roadmapping** | Manual, calendar-bound timelines prone to executive burnout. | Interest-aligned, capacity-driven **`aspire.rs`** roadmap governed by the Bayesian Hypervisor. |
| **Activity Tracking** | Manual bookkeeping of wall-time, durations, and task lists in markdown tables. | Passive, autonomic harvesting of file modification times (`mtimes`), CST node deltas, and compiler outcomes. |
| **Model Ingestion & RAG** | Conversational priming, manual prompt writing, and dense context windows. | Real-time **Source-Compiler** that tokenizes CST mutations and fine-tunes local model weights via LoRA backpropagation. |
| **Verification & Gates** | Standard Rust compiler checks. | Multiclass **5-Layer Gate Validator** (`L-GPU`, `L-FPS`, `L-SYNC`, `L-UNIFIED`, `L-AUDIO`) with bit-exact replay determinism. |
| **Public Presence** | Manual release notes, package compiles, and active marketing overhead. | **YouTube-Forge** captures GPU readbacks and parses SR&ED ledgers to generate video devlogs automatically. |
| **Industrial Scope (SLAPP)** | Blast radius digital twins requiring client and commercial administrative overhead. | **Fully Deported.** Solvers and photogrammetric loops salvaged to drive procedural texture synthesis (Spriteblob) and HRTF acoustic propagation. |

---

## 3. System Architecture: The Unified Sovereign Stack

```
 +-------------------------------------------------------------------------------------------------+
 |                                       13FORGE-STUDIO.EXE                                        |
 |                                                                                                 |
 |  T1 CPU (120Hz DET-CLOCK)                                     T3 GPU (CREATIVE-LANE)            |
 |  +-------------------------------------+                      +------------------------------+  |
 |  |  Deterministic Core                 |                      |  SovereignWindow (wgpu)      |  |
 |  |  * EQ Game Logic (13 Moons)         |                      |  * VixiScript Layout Engine  |  |
 |  |  * TritTree5D Spatial Index         |                      |  * Ubershaders & Spriteblob  |  |
 |  |  * CDK Cosmic Dissonance            |                      |  * Neuro_HUD Canvas          |  |
 |  +------------------+------------------+                      +--------------+---------------+  |
 |                     |                                                        ^                  |
 |                     v                                                        |                  |
 |              [TripleBuffer] <======= ENDLESS SILK (Z-SYNC) ===================+                  |
 |                     |                                                                           |
 |                     v                                                                           |
 |  SHADOW-INFER THREAD (In-Process ML & DSP)                                                      |
 |  +-------------------------------------------------------------------------------------------+  |
 |  |  Candle-Core Engine                                                                       |  |
 |  |  * NDE x3 Models: master.nde, teacher.nde, student.nde                                    |  |
 |  |  * Lorekeeper MoE: 40 Domain Experts + 9 Cognitive Regulators                             |  |
 |  |  * MOM DPS GPU: Parallel audio synthesis & HRTF spatialization                            |  |
 |  |  * Bqr_router (BqRouter / BqRouterDrain): Fast NDJSON/HTTP controller traffic filter        |  |
 +-------------------------------------------------------------------------------------------------+
```

---

## 4. Z-Sync & The Z-Clock (Deterministic 120Hz Concurrency)

A core challenge of the Sovereign Monolith is the synchronization of **T1 (the deterministic 120Hz CPU simulation)** and **T3 (the uncapped GPU presentation loop)** without introducing dynamic memory allocations, context switching, or thread-locking contention. 13Forge solves this using **Z-Sync** and the space-time-scale clock primitive:

```
  T1 Thread (120Hz CPU)             Shared TripleBuffer             T3 Thread (Uncapped GPU)
  +-------------------+             +------------------+            +-------------------+
  | Writes Sim State  | ======>     | [ Slot A (Write) ]            | Presents Frames   |
  | (Q16.16 Fixedpt)  |             +------------------+            | (F32 Layout)      |
  +---------+---------+                      ||                     +---------+---------+
            | (Green Build)                  ||                               ^
            v                                v                                |
  [Atomic Pointer Swap] <================>  [ Slot B (Middle) ]               |
            |                                v                                |
            |                                ||                               |
            +----------------------------> [Slot C (Read) ] ==================+
                                             (Reads live z_phase and interpolates)
```

### A. The Lock-Free Triple Buffer
Z-Sync utilizes a lock-free `TripleBuffer` containing three memory slots: `Write` (owned by T1), `Read` (owned by T3), and `Middle` (shared). 
-   When T1 completes a simulation tick, it performs an **atomic pointer swap** with `Middle`.
-   When T3 is ready to render a frame, it performs an **atomic pointer swap** with `Middle` and reads the state into `Read`.
-   This synchronization occurs in exactly **one atomic exchange instruction** ($\approx 10\text{ns}$), guaranteeing zero thread blockage, zero priority inversion, and zero runtime heap allocations.

### B. The Space-Time-Scale Primitive: `Cell {x, y, z, t, s}`
At the bottom of the execution pipeline sits the unified **`Cell {x, y, z, t, s}`** primitive, which fuses space, time, and scale into a single mathematical atom:
-   **`x, y, z` (Space):** Fixed-point trinary coordinate placements.
-   **`t` (Time / Z-Clock):** The deterministic, sequence-exact **120Hz integer ticktime**. Every consequence and physics event derives from this single, deterministic hash chain—fully locked to music and frame-sync.
-   **`s` (Scale / Sub-Tick Permyriad):** The **permyriad integer scale (0..10000)** representing sub-tick spatial and temporal interpolation. T3 reads the fractional difference between present frame timestamps and the T1 `z_phase` ticktime, converting it to a permyriad value ($0..10000$) to interpolate coordinate placements and RMS audio profiles smoothly without floating-point drifting.

### C. Backpressure & Thermal Governor
If GPU presentation falls below a critical frame budget ($60\text{Hz}$), the atomic exchange gate detects backpressure. The system's **Resource Governor** automatically scales down particle generation (Spriteblob density) and throttles local model token generation, lowering physical system fan noise and thermal load to restore a silent, sensory-calm physical space.

---

## 5. The Unified SPIR-V WebGPU Kernel (`sprv`)

At the core of 13Forge's graphics and compute architecture sits the **Unified SPIR-V WebGPU Kernel** (`sprv`). It bridges native high-performance hardware execution with sandboxed web deployment through a single consolidated pipeline.

### A. Naga-Validation and SPIR-V Emit
All shaders are authored in declarative WebGPU Shading Language (WGSL), e.g., `canvas_quad.wgsl`. On-save, the **`forge-shader-build`** compiler leverages **`naga`** (the compiler frontend) to perform structural static checks:
1.  **Uniform Offset Proving:** Naga asserts that byte alignments and struct padding defined in the shader uniforms exactly match the memory layouts of our Rust-side types.
2.  **Safety Verification:** It checks array-index boundary access, textures bounds, and registers to guarantee zero GPU-side out-of-bounds panics.
3.  **SPIR-V Binary Emit:** Once validated, naga compiles the WGSL representation directly into a bit-exact, highly optimized **SPIR-V binary block** (`.spv`).

### B. Dual-Clock Cross-Platform Parity
The emitted SPIR-V binary is cross-platform by design, running on our native and web targets with bit-exact parity:
-   **Native Presentation:** Runs natively inside the Vulkan / DX12 backend of `13forge-studio.exe` via `wgpu` for zero-overhead, demoscene-grade present execution.
-   **Web Presentation:** Executes natively inside browsers via the browser's raw **WebGPU API** inside `sf-wasm`.
-   **Computation & Render Unification:** A single unified `.spv` kernel handles both graphics rendering (drawing Ubershaders, procedural noise, and Spriteblob buffers) and complex compute shader runs (MOM DPS GPU parallel audio sifting and TritTree5D proximity queries) simultaneously, eliminating runtime shader switching.

---

## 6. The Vixio State Reactor (The UDLE Connective Engine)

To replace high-overhead event brokers, asynchronous polling delays, and race conditions, the Connective Layer of the New UDLE is driven by the **Vixio State Reactor** (or Vix Reactor). It operates as a deterministic, clock-aligned state engine that manages the entire lifecycle of system-wide changes, modeling outputs, and developer stress states.

```
 +---------------------------------------------------------------------------------+
 |                              VIXIO STATE REACTOR                                |
 |                                                                                 |
 |  Inputs: CST mutations · LSP warnings · Allostatic Telemetries · GGUF outputs  |
 |         \                                                    /                  |
 |          v                                                  v                   |
 |   +--------------------------------------------------------------+              |
 |   |               Deterministic Reduction Pass                   |              |
 |   |               * Ticks at strict 60Hz T3 presentation budget  |              |
 |   |               * Evaluates system-wide "Edicts"               |              |
 |   +------------------------------+-------------------------------+              |
 |                                  |                                              |
 |                                  v                                              |
 |  Outputs: Real-time VibeMatrix · Technothesia Drones · Shadow-Infer Prompts    |
 +---------------------------------------------------------------------------------+
```

### A. 60Hz Deterministic State Reduction
The Vixio Reactor bypasses traditional background polling loops. It runs a single, synchronized reduction pass locked directly to the **60Hz presentation tick of the T3 GPU present-loop**:
-   **State Ingestion:** It ingests raw telemetry streams (keystroke cadence, CST node deltas, compile diagnostics, local model GGUF text chunks) and packages them into a single, unified `VixioState` tree.
-   **Edict Mapping:** The Reactor evaluates this tree against a declarative list of system-wide **"Edicts"**. For example, when `StressState` is high and compile errors are detected, the Reactor resolves an Edict to dim visual palettes and override local models to concise monosyllables.
-   **Autonomic Synchronization:** It directly updates the GPU's `VibeMatrix` uniforms, the `CDK` dissonance frequencies, and the `shadow-infer` GGUF prompt weights in a single unified step.

### B. The Five-Step Life-Loop
The Reactor structurally enforces a strict **Five-Step Action Plan** for every developmental transaction, ensuring that progress remains continuous and never stalls:
1.  **Ingest / Observe:** Detect and parse on-disk CST mutations.
2.  **Triage / Gate:** Check capability registries (`catalog.rs` and `aspire.rs`), triggering a *Dream Halt* if necessary to block horizon-level work.
3.  **Align / Proof:** Verify byte alignments and boundaries with the *Unified Theory Compiler*.
4.  **Act / Weld:** Execute mass refactoring runs (`massweld`).
5.  **Distill / Seal:** Run the *Source-Compiler* to fine-tune local LoRA weights in the background.
*   *Note:* The Reactor's core loop never terminates or marks the plan as "done" because the development cycle is an endless, breathing ribbon—the **Endless Silk** loop.

### C. Candle Oracle & Gemma Convergence
Through the Vixio Reactor, local Candle-Core model outputs are fully integrated as reactive state parameters:
-   **State Ingestion:** Finding outputs generated by local foundation models are parsed and stored as declarative variables inside the `VixioState` tree (the test `the_candle_oracle_finding_survives_gemma` proves this persistence).
-   **Flow State:** The Hypervisor uses these persistent neural variables to dynamically adjust system resource schedules and direct your attention without introducing interface latency.

---

## 7. Core Spatial Primitives: PackedPoint105 & TritNode5D

For ultra-high-velocity 5D proximity queries and physical collision, the engine bypasses traditional floating-point bounding volumes, relying entirely on the **TritTree5D** balanced trinary spatial index. The core data primitives are engineered for L1/L2 cache efficiency and packed memory layouts.

```
 PackedPoint105 Primitive (Exactly 32 Bytes - 2 per cache line)
 +-----------------------------+------------+--------------------+
 | bytes: [u8; 21] (105 Trits) | id: u32    | _padding: [u8; 7]  |
 +-----------------------------+------------+--------------------+

 TritNode5D Primitive (Exactly 32 Bytes - 2 per cache line)
 +-----------------+-----------------+------------------+-----------------+
 | split_val: i32  | child_neg: u32  | child_zero: u32  | child_pos: u32  |
 +-----------------+-----------------+------------------+-----------------+
 | point_idx: u32  | min_z: i16      | max_z: i16       | split_axis: u8  |
 +-----------------+-----------------+------------------+-----------------+
 |                 | _padding: [u8; 11]                                   |
 +-----------------+------------------------------------------------------+
```

### A. The 105-Trit Packed Point (`PackedPoint105`)
To represent 5D spatial locations (3D spatial coordinates $X, Y, Z$, plus $2\text{D}$ situational vectors representing compass orientation $\theta$ and local field-tension $\phi$), the engine uses a custom 105-trit balanced trinary packing layout:
-   **Structure:**
    ```rust
    #[repr(C, align(32))]
    pub struct PackedPoint105 {
        pub bytes: [u8; 21],    // 105 trits packed at exactly 5 trits/byte
        pub id: u32,            // Unique point identifier (e.g. NPC or sound ID)
        pub _padding: [u8; 7],  // Aligns struct to exactly 32 bytes
    }
    ```
-   **High-Speed Distance Hamming:** Measuring distance between 5D points utilizes a pre-computed Hamming lut (`TRIT_HAMMING_SHEET` from `forge-calligraphy`) in a lock-free loop, evaluating the distance of 105 trits in just 21 byte index lookups, executing in under $3\text{ns}$ on a single core.

### B. The 5D Spatial Node (`TritNode5D`)
The search tree structure is composed of tightly packed, 32-byte trinary branch nodes:
-   **Structure:**
    ```rust
    #[repr(C, align(32))]
    pub struct TritNode5D {
        pub split_val: i32,      // Gray-coded split coordinate on selected axis
        pub child_neg: u32,      // Trit -1 branch index (Below boundary / <)
        pub child_zero: u32,     // Trit  0 branch index (Sovereign Anchor / 0)
        pub child_pos: u32,      // Trit +1 branch index (Above boundary / >)
        pub point_idx: u32,      // Index of leaf point in arena (u32::MAX if internal node)
        pub min_z: i16,          // Lower z-axis semantic family boundary
        pub max_z: i16,          // Upper z-axis semantic family boundary
        pub split_axis: u8,      // Split axis (0..=4)
        pub _padding: [u8; 11],  // Aligns struct to exactly 32 bytes
    }
    ```
-   **Cache Parity:** By aligning both `PackedPoint105` and `TritNode5D` to exactly 32 bytes, the T1 physics engine guarantees that exactly **2 nodes or points fit into a single standard 64-byte CPU cache line**. This completely eliminates L1/L2 cache misses during high-dimensional tree traversals.

---

## 8. The Brain: NDE x3, Lorekeeper MoE, and Concept Lexicon

The neural core of the stack runs entirely offline via `candle-core` inside the `shadow-infer` thread.

### A. The NDE x3 Model Tier
1.  **`master.nde` (E:/):** The frozen, multi-modal foundation model. It acts as the source of truth for soft-targets and target logit computations.
2.  **`teacher.nde` (F:/):** LoRA-adapted, intermediate reasoning model. It guides the active developer sessions and provides contextual cognitive support.
3.  **`student.nde` (F:/):** LoRA-adapted, high-speed execution model. It runs local code generation, compiles AST changes, and auto-synthesizes SR&ED ledgers.

### B. Lorekeeper MoE (40 Experts + 9)
The local Mixture of Experts (MoE) is split to isolate cognitive load:
-   **40 Specialized Domain Experts:** Individual, highly focused expert networks optimized for specific domains (e.g., `expert_dsp` for audio filters, `expert_wgsl` for shader rendering, `expert_5d` for trinary geometry, and `expert_rust` for memory-safety).
-   **9 Cognitive Regulators:** Oversee and throttle systemic health. These include `regulator_audit` (enforcing invariant checks), `regulator_sred` (framing technical uncertainties), and `regulator_neurohud` (conditioning visual palettes based on sensory state).

### C. The Concept Lexicon
The `concept_lexicon` maps raw identifiers, AST symbols, and file-paths directly to abstract concepts. This enables semantic-syntactic raycasting (`mcp_forge_raycast` over `river.idx`), allowing the MoE to navigate the workspace using conceptual intent rather than literal text matching.

---

## 9. Compiler Infrastructure: AST, CST, LSP, and DSL

To guarantee maximum speed, zero cognitive overhead, and bulletproof correctness, 13Forge features a custom parser and editor pipeline running natively in Rust.

### A. Domain-Specific Languages (DSL)
We reject dense, standard configuration layouts. The entire stack is driven by hyper-focused, declarative DSLs:
-   `.kit.vixi` (Visual layout declarations using line-based syntax, avoiding braces or CSS noise).
-   `.sheet.vixi` (Theme definitions, compiling down to raw 8-palette color slots).
-   `.shaderbind.vixi` (GPU binding pipelines, specifying uniforms, samplers, and textures).
-   `.renderpass.vixi` (Draw passes and Splitshader GPU synchronization configurations).
-   `.vibe` (Real-time physical and mental tension vectors for gameplay and audio).
-   `.semantic` (Concept indexing tags for raycasting and MoE routing).

### B. Lossless Concrete Syntax Tree (CST) vs. AST
Every DSL file is parsed in two distinct phases:
1.  **Concrete Syntax Tree (CST):** Handled by `forge-ast` and `tree-sitter-vixel`. The CST is completely **lossless**—it preserves every space, tab, newline, line comment, and structural formatting byte. This is crucial for local LLMs (`student.nde`) and our automated refactoring tools (`massweld`), allowing them to modify files programmatically without losing developer-specific formatting or comments.
2.  **Abstract Syntax Tree (AST):** The lossless CST is lowered into a lightweight, memory-aligned AST inside the `vixio` compiler. Geometry values are evaluated to integer MilliUnits (`mu`), color references are mapped to palette tokens, and all floats are fully prohibited. The AST is compiled into binary `LoweredUi` blocks, ready to push directly to the GPU present loop.

### C. Language Server Protocol (LSP)
The **`forge-vix-lsp`** daemon provides instant IDE-side intelligence within a 15ms budget:
-   **Static Analysis:** On-save, the LSP analyzes layout boxes, contrast ratios, and button hit targets.
-   **Validation Gates:** Verifies color assignments against active `.sheet.vixi` profiles and signals warnings if layout guidelines (like the 4±1 group rule) are violated.
-   **Sensory Pipeline:** Diagnostic errors and warnings are not just printed to a terminal. The LSP pipes the AST compile state directly to the active **Neuro_HUD VibeMatrix**, feeding visual cellular distortions and drone pitches in real-time.

---

## 10. The Unified Theory Compiler (Synthesis of 30+ Compilers)

Across 13Forge's 100+ modular crates, the system utilizes **30+ domain-specific compilers, interpreters, and transpilers**. To prevent cross-language drift, runtime panics, and resource misalignment, the stack runs a meta-synthesis layer called the **Unified Theory Compiler**.

### A. The 30+ Domain-Specific Compilers
Each specialized subsystem has its own translation layer to lower high-level declarative intent to optimized, lock-free runtime state. Key compilers include:
1.  **Vixi Layout Compiler (`forge-vix`):** Compiles visual `.kit.vixi` templates to low-level GPU drawing lists.
2.  **Theme Sheet Compiler (`forge-colour`):** Compiles `.sheet` palettes into direct GPU shader indices.
3.  **Shaderbind Compiler (`forge-shader-build`):** Maps uniform registers, samplers, and bindings to pipeline structures.
4.  **Audio DSP Graph Compiler (`forge-audio`):** Transpiles procedural audio filters and intent grids to lock-free ring-buffer parameter pipelines.
5.  **TritTree5D Geometry Compiler (`forge-geo`):** Compiles 5D coordinate maps to balanced trinary node points.
6.  **And 25+ other compilers** managing physics configurations, behavior networks, asset packers, font calligraphies, and neural weight loaders.

### B. The Unified Theory Compiler (The Meta-Synthesis Ring)
Traditional architectures compile distinct languages in complete isolation, leaving type and offset discrepancies to trigger runtime panics (e.g., mismatched struct alignment between Rust and WGSL shaders). The **Unified Theory Compiler** resolves this by compiling the *entire systemic theory* down to a single verified state-vector:
-   **Static Structural Proving:** It reads all 30+ compilers' intermediate representations (IR) on-disk, validating boundaries:
    -   *Shader-to-Rust alignment:* Proves that the WGSL uniform block offsets in `.shaderbind.vixi` map byte-for-byte to the `bytemuck`-implemented Rust struct in `forge-render`.
    -   *Layout-to-Controller alignment:* Proves that `.kit.vixi` visual layout slots refer exactly to fields exported by the host `MudView` struct in `sf-wasm`.
    -   *Audio-to-Telemetry alignment:* Proves that the sound-sifter drone indices in `dead-drop` are fully covered by the `VibeMatrix` uniforms.
-   **Outcome:** If any cross-domain boundary exhibits even a 1-bit or 1-byte misalignment, the Unified Theory Compiler halts compilation on-disk during a single compilation epoch. This guarantees absolute, bit-exact structural parity across 30+ compilers *before* a single frame or audio sample is drawn on-host.

---

## 11. The Source-Compiler (Self-Ingesting Neural Loop)

To complete the sovereign, off-grid cycle, 13Forge features a specialized **Source-Compiler** running natively inside `forge-pkm` and `forge-ml`. It completely removes the need to write prompts, documentation, or explanation notes to synchronize our local models with code changes.

### A. Code as the Training Dataset
Traditional local AI assistants require massive external context windows, RAG indexing, or conversational priming to understand a developer's code. The **Source-Compiler** treats the active codebase and its structural CST nodes *directly* as a highly structured training dataset:
-   **Continuous Ingestion:** Whenever the **Unified Theory Compiler** validates a clean, compile-green system-wide build on-disk, the Source-Compiler triggers.
-   **Structural Sequencing:** It maps active files, functions, and layout binds to structured token sequences (using the shared offline `SpmTokenizer` and the `concept_lexicon`).

### B. In-Process Weight Synthesis (Live LoRA Tuning)
Instead of converting code to text documents to feed a search index, the Source-Compiler compiles the code directly into the model's neural weights:
-   **Backpropagation Pass:** It performs an in-process training epoch using `candle-core` tensor operations inside the dedicated `shadow-infer` thread.
-   **Weights Update:** Only the local LoRA adapter weights of `student.nde` (the execution model) and `teacher.nde` (the reasoning assistant) are updated.
-   **Result:** The local models learn your architectural modifications, style conventions, and module definitions **organically and passively** as you write them. The AI literally "grows" with the codebase. You never have to explain your code to the assistant—the Source-Compiler compiles it straight into its brain.

---

## 12. The Living Atlas & Roadmap: Forge-Book & Aspire

To maintain absolute structural alignment and direct the development of the Sovereign Stack without human-maintained wiki files, the stack compiles its documentation, architecture-laws, and roadmap using **`forge-book`** and **`aspire`**.

### A. The Capability Catalog (`catalog.rs`)
`forge-book` partitions all current and future architectural modules into discrete, tracked capabilities:
-   **Verification Status (`catalog.rs`):** Tracks whether a component is `Proven` (active, fully tested), `Wired` (integrated but awaiting full gate tests), `Planned` (drafted design spec), or `Study` (research phase).

### B. The `aspire` Living Roadmap (`aspire.rs`)
**`F:\NewRepo\crates\forge-book\src\aspire.rs`** serves as the **Living Roadmap Engine** for the entire Sovereign Stack. It organizes development into high-dimensional priority horizons that map our current implementation tasks directly to multi-year goals:
-   **`NOW` (Active Execution):** The immediate on-disk welds (Endless Silk routing, Vixi layout completions, and PTY terminal bridges).
-   **`NEXT` (Taping & Verification):** The next milestone (Splitshaders, MOM DPS GPU, and raw hardware bindings).
-   **`LATER` (Core Deep Tech):** 5D Spatial Index (TritTree5D) integrations, local MoE scaling, and offline translation.
-   **`HORIZON` (The Sovereign Dream):** Autistic Flow-Fortress (fully automatic packaging, offline P2P mirror seeding, and complete sandbox-isolation).
-   **`EDGE` (Speculative Research):** Multi-modal neuro-telemetry inputs, offline off-grid hardware nodes, and custom silicon syntheses.

### C. The "Dream Halt" Guardrail
To contain executive dysfunction, hyperfocus rabbit-holes, and over-engineering, the compiler enforces the **Dream Halt** protocol. 
*   If a developer (or an automated AI agent) attempts to implement a system whose capability is marked as `Planned` / `Study` in the catalog, or categorized under `LATER` / `HORIZON` / `EDGE` in aspirations, the compiler enforces a strict Dream Halt, halting build execution immediately.

---

## 13. The 100 Law & The 1000 Drop (Sovereign Limits)

To prevent cognitive fragmentation, structural decay, and the developer-burnout loop, 13Forge enforces strict mathematical limits on the workspace and release engine.

### A. The 100 Law (Cognitive Working-Set Cap)
The auDHD mind is highly prone to starting infinite parallel initiatives, leading to cognitive bloat. The **100 Law** establishes a hard structural constraint on active workspace elements:
-   **Capability Catalog Limit:** The active capability list (`catalog.rs`) has a strict, compiler-enforced cap of exactly **100 active capability nodes**. 
-   **The Trade-Off Rule:** To register a new capability, the developer must either:
    1.  Fully prove and merge an existing `Wired` node into `Proven` status.
    2.  Prune and delete an inactive, stale `Study` or `Planned` node.
    -   If compilation detects more than 100 entries, `forge-book` compilation fails with a loud `Allostatic Overload` error. This enforces a lean, highly focused working set.

### B. The 1000 Drop (Hands-Off Release Engineering)
Manual packaging, writing release notes, sending announcement emails, and publishing updates represent heavy administrative overhead that drains focus. 13Forge completely automates release engineering via the **1000 Drop**:
-   **Execution:** Every 1000 completed velocity units or 5-layer gate validation passes recorded passively in the `velocity-ledger.md`, `13forge-studio` triggers an autonomic release build.
-   **Compilation & Signing:** The engine self-compiles a signed, optimized standalone executable (`13forge-studio.exe`).
-   **Passive Deployment:** It packages the release alongside the compiled `_book/` Living Atlas and automatically writes the bundle to off-grid mirrors, local storage networks, and peer-to-peer magnet anchors with **zero marketing or manual coordination**. Code remains the sole, quiet beacon.

---

## 14. The Network, Spine, & Body

### A. Bqr_router (`BqRouter` & `BqRouterDrain`)
The network boundary is guarded by `BqRouter` and `BqRouterDrain`, running on ports 13013 (`FORGE_PORT` control) and 13016 (`FORGE_MCP_PORT` MCP door). It filters NDJSON/HTTP traffic, validates bearer tokens against `FORGE_MCP_TOKEN`, and routes commands directly to the core state engine without introducing latency.

### B. CDK (Conductor Development Kit)
The CDK serves as the synchronization clock-line. It packages all audio, geometry, and layout events into 16-byte `RoutedUmp` packets. These packets carry `Conductor` markers, ensuring that training loops, state mutations, and audio ticks align perfectly to the master `Vixio` clock-tick.

### C. Endless Silk (The Core-to-UI Ribbon)
**Endless Silk** is the high-speed ribbon that wires T1 (the deterministic 120Hz CPU core) directly to T3 (the GPU rendering surface) via the lock-free `TripleBuffer`. It binds fields like `perception_q` directly to the VixiScript layout engine without intermediate allocations or Mutex locks. 
*   **The birth screen,** the **PTY terminal,** **Technothesia voice pipelines,** and **HUD panels** are all hot-welded onto this single ribbon.

### D. TritTree5D Spatial Index
For high-velocity open-world rendering in *13 Moons*, the engine uses the **TritTree5D** spatial index.
-   **Structure:** A balanced trinary partitioning index packed into 105-trit boundaries.
-   **Role:** Near-instantaneous spatial proximity lookups for NPC AI, sound HRTF emitters, and environmental intent fields.
-   **Invariants:** All coordinate math is integer-only (fixed-point Q16.16) to ensure bit-exact determinism during ForgeWright replay-testing.

### E. The Graphics Body (Splitshaders, Ubershaders, Spriteblob, and Tuiblob)
-   **Splitshaders:** Decouples computational pre-simulation (`gemma_forward.renderpass.vixi`) from GPU present passes.
-   **Ubershaders:** Consolidated state-free WGSL pipelines (`canvas_quad.wgsl`) that process complex material systems under the `VibeMatrix`.
-   **Spriteblob:** GPU-side cellular-automata-driven pixel buffers producing organic 2D normal-mapped textures.
-   **Tuiblob:** Translates scrolling TUI text streams into Vixi layout cards on the GPU canvas.

---

## 15. Sound, Vision, & Stress-Management: MOM DPS GPU, Neuro_HUD, and Broski

### A. MOM DPS GPU (MoE DSP GPU)
The audio thread of *Dead Drop* and *Technothesia* offloads massive, multi-voice HRTF spatialization and procedural audio sifting to the GPU using wgpu compute shaders. The **MOM DPS GPU** handles hundreds of simultaneous 3D spatialized voices, calculating auditory fields based on NPC proximity and intent grids.

### B. Neuro_HUD (Preattentive Sensory Dashboard)
The Neuro_HUD is written in line-based `.kit.vixi` syntax, respecting the Cognitive Law of **4±1 groups** to prevent sensory exhaustion.

```
#vixi:kit v1
surface: neurohud_deck
profile: molten
classification: sovereign_hud

# Base canvas with deep background slot
slot root kind=region layout=stack_v gap=mu(4) padding=mu(12) bind=palette.bg_far

# Group 1: Cognitive State (Keen, Flowing, Recharging)
slot root.state kind=region layout=stack_h size=mu(24)
slot root.state.label kind=text ramp=type.ramp[1] color=palette.fg_muted source=cognitive_mode
slot root.state.accent kind=widget name=glow_dot size=mu(8) bind=palette.accent_primary

# Group 2: Flow Density Dial (AST mutations & compilation state)
slot root.attention kind=region layout=stack_v size=mu(64) bind=palette.bg_near
slot root.attention.meter kind=widget name=gauge value=source.ast_delta_q label="FLOW DENSITY"

# Group 3: Passive Gate-Pass Indicators
slot root.gates kind=region layout=stack_h size=mu(32) gap=mu(8)
slot root.gates.gpu kind=widget name=gate_indicator state=source.gate_gpu_state label="GPU"
slot root.gates.fps kind=widget name=gate_indicator state=source.gate_fps_state label="FPS"
slot root.gates.sync kind=widget name=gate_indicator state=source.gate_sync_state label="SYNC"
slot root.gates.audio kind=widget name=gate_indicator state=source.gate_audio_state label="AUD"

# Group 4: Passive SR&ED Synthesizer
slot root.sred kind=region layout=stack_v bind=palette.bg_near padding=mu(8)
slot root.sred.title kind=text ramp=type.ramp[0] color=palette.accent_secondary source=sred_summary

# Cognitive validation rules
gate contrast_min = 4.5
gate hit_target_min = mu(44)
gate float_in_ir = forbidden
```

### C. Allostatic Bayesian Hypervisor & Resource Governor (`forge-broski`)
The stress engine is managed by the **Allostatic Bayesian Hypervisor (Commander Broski)** and the **System Resource Governor**:

1.  **Bayesian Belief Network:** Instead of static reactive thresholds, the Hypervisor runs an active **probabilistic (Bayesian) belief network** that continuously updates its posterior probability of your immediate cognitive states (e.g., *Flow*, *Fatigue*, *Hyperfocus Collapse*, *Distraction*, *Executive Block*) based on real-time streaming priors (AST edit intervals, keystroke cadence, compiler failures, and audio feedback levels).
2.  **The Commander Loop:** The Commander reads your instantaneous cognitive probability map and matches it directly against the **`aspire.rs` Living Roadmap**:
    -   If the probability of high-focus flow is **high (≥85%)**, the Commander recommends high-density architectural or wiring tasks under the **`NOW`** and **`NEXT`** buckets. The **Governor** allocates maximum T1 CPU priorities and resident memory (RSS) to compiling and modeling.
    -   If the probability of cognitive fatigue or executive blocks is **high (≥70%)**, the Commander overrides the workspace:
        1.  **Quiet Mode:** Local models are instantly locked to monosyllabic, quiet responses to minimize reading load.
        2.  **Roadmap Locking:** Writing code is locked out. The Commander hides complex `NOW` tasks and prompts you to engage in **`RECHARGING`** or speculative **`EDGE`** tasks (e.g., world-walking inside *13 Moons* or running Technothesia audio sifting).
        3.  **Visual & Auditory Dimming:** Visuals shift to a scotopic-safe low-blue midnight indigo. Sounds are restricted to a deep 55Hz Technothesia sub-bass hum.
        4.  **Hardware Governor Cooling:** The Governor throttles down local model processing and compile cycles, lowering physical system fan noise and heat to provide a calm, silent development environment.

---

## 16. Passive Outreach: YouTube-Forge

To support our passive distribution model, the workspace generates its public presence entirely through automation, shielding the programmer from marketing and social media overhead.

### YouTube-Forge Media Pipeline
`youtube-forge` is an in-process media generator running silently inside `13forge-studio`.
-   **High-Fidelity Capturing:** On successful 5-Layer Gate validations, the pipeline automatically records gameplay footage from *13 Moons* or audio sessions from *Dead Drop* directly off the GPU readback buffer.
-   **Devlog Compilations:** The pipeline reads your passive SR&ED ledger files and AST delta streams, generating a text devlog script mapping out the "uncertainties resolved" during the session.
-   **Passive Beacons:** It compiles the recorded video, synthesizes the audio (using Technothesia spatial streams), and packages them into clean, high-impact video assets. These are staged silently for passive upload to YouTube/media channels. 
-   **Outcome:** The video acts as a passive beacon, pulling like-minded developers directly to our open-source, self-service off-grid mirrors, with **zero active marketing or sales outreach required**.

---

## 17. Outland & Goldminer (The Sovereign Loop)

The developer's workspace operates as an off-grid environment.

### A. Outland
Outland is the physical and digital environment of the sovereign, off-grid developer. All network communication is blocked; there are no external SaaS dependencies, API key endpoints, or centralized telemetry nodes. Data representation uses the distinct 64-byte sealed `SoulWord` cell for offline identity and integrity verification.

### B. Goldminer
**Goldminer** (`goldminer-core`, `goldminer-app`) is a silent, background task runner. When the developer is in a **RECHARGING** state, Goldminer actively scans the local repository and parsing history. It mines the AST of unused code, identifies redundant structures, runs dead-dependency scans, and feeds these insights back to the local `teacher.nde` model, optimizing the local `concept_lexicon` while the developer rests.

---

## 18. Code-to-Disk (Weld & Vixi Verification)

When writing or modifying code in this stack, future AI agents must execute the **OODA-Weld-Vixi** verification protocol:

1.  **Weld Dry-Run:** Validate edits with `massweld --dry-run` to prove exactly 1 verbatim anchor hit.
2.  **Vixi Off-Disk Validator:** Run `pwsh -NoProfile -File .claude/skills/vixi-uiux/check.ps1 <file.vixi>` to ensure the kit compile exits with code 0 and has zero warnings.
3.  **Husk Gate:** Off-host, the kit layout engine must prove it compiles and binds all slots. Text slots must bind `source=<key>` to a host before merging.
4.  **No Allocations:** Reject any proposal that introduces heap allocations (`Vec::push`, `Box::new`) inside the 120Hz deterministic physics loop or the 60Hz present loop.
5.  **Living Atlas Gate:** When adding design patterns or specification notes, register them in `F:\NewRepo\crates\forge-book\src\catalog.rs` and seed them in `F:\NewRepo\crates\forge-book\src\seed.rs` to maintain 100% on-disk alignment with compiled documentation counts. Enforce the **100 Law** on catalog capacity.
6.  **Unified Theory Proof:** Cross-domain compilers must run validation checks (verifying byte offsets and AST/CST bindings across boundary lanes) before a weld is declared clean.
7.  **Self-Ingestion Trigger:** Once the Unified Theory Compiler compiles a clean green build, the **Source-Compiler** automatically tokenizes and ingests code changes to update the local LoRA adapters of `student.nde` and `teacher.nde` in-process.
