# Sovereign Synthesis: Outland, Goldminer, Gemma, and the `/tracktor-beam` Process

This document consolidates the architectural reviews of the **Outland** and **Goldminer** 5D search systems, analyzes their overlap, and synthesizes a new process—the **`/tracktor-beam` (Unified Codebase Assimilation Cycle)**—that integrates Outland, Goldminer, and the **Gemma** reasoning engine to safely haul external codebases and modules into the canonical GhostMoon monolith.

---

## 1. Core System Reviews & Overlaps

### A. Outland-Index (`crates/outland`)
*   **Target Scope:** Hierarchical directory and path structures (e.g., matching search queries against indexed files and folders).
*   **Core Projection:** The `embed5` pipeline maps file paths to a 5-dimensional coordinate vector `[x, y, z, theta, w]`.
    *   *Lane 2 (z, dominant):* Maps semantic file categories (e.g., `io`, `memory`, `graphics`) to discrete blocks scaled by `FAMILY_STEP` (16,384).
    *   *Lanes 0, 1, 3 (x, y, theta):* Computes MinHash values over unified lowercase stems (Jaccard token similarity approximation).
    *   *Lane 4 (w):* A whole-string exact fold acting as a tie-breaker.
*   **Delivery Pipeline:** Binds directly to the deterministic `vixio` reactor. Delivers indexed path hits sequentially over network sockets, spacing them out exactly tick-by-tick (Rank $k$ streams on Tick $k+1$).
*   **Dependencies:** Zero-dependency, lightweight, and local.

### B. Goldminer-Core (`crates/goldminer-core`)
*   **Target Scope:** Line-level search over raw source codebases. Designed as a standalone, sellable SKU (GM-60 product) for secure, air-gapped developer environments.
*   **Core Projection:** Uses the standard `embed_river_line` projection from `forge-ml`.
    *   *Lane 0 (x):* FNV-1a of the line prefix/tag (e.g., `MAP`, `SPILL`).
    *   *Lane 1 (y):* Full payload discriminator (avalanche hash ensuring distinct lines stay distinct).
    *   *Lane 2 (z):* Shape locality (capped byte-length and tab counts).
    *   *Lanes 3, 4 (theta, w):* Position-salted token order and order-insensitive token-set multiset parities.
*   **Delivery Pipeline:** Executes raw, in-memory array scans. Returns results instantaneously for IDE integration.
*   **Dependencies:** Binds directly to the `forge-ml` utility crate.

### C. The Overlap & Intersect
*   **The Shared 5D Substrate:** Both systems map text strings to the 5D GhostMoon box `[x, y, z, theta, w]` and compute distance using exact integer squared-Euclidean metrics. This guarantees 100% bit-reproducible searches across all target architectures with zero floating-point drift.
*   **The Angular Lane ($\theta$, Lane 3):** Both models treat Lane 3 as an angular coordinate wrapping at $360,000$ millidegrees, resolving wrap-aware shortest-path vectors.
*   **The Raycast Engine:** Both execute ray queries by casting a 5D vector between a `from` and `toward` anchor, identifying candidate elements based on perpendicular distance to the ray trajectory.
*   **Deterministic and "Gemma-Free":** Both avoid running local neural-network inference during indexing. By relying on fast hashing (`fnv1a`), they can index tens of thousands of lines per second on low-power hardware, maintaining absolute isolation from heavy LLM execution bounds.

---

## 2. Synthesis: The `/tracktor-beam` Codebase Assimilation Process

The **`/tracktor-beam`** process is a unified, automated codebase pipeline. It acts as a "tractor beam" that points at a distant repository or isolated external module, determines its semantic relationship to the canonical workspace, synthesizes an integration blueprint, and merges (welds) its files safely into the monolith.

```
                  [/tracktor-beam <target-path>]
                                │
                                ▼
        [STAGE 1: Target Lock (Goldminer Line-Scan)]
          • Project target files into 5D Codebook
          • Identify raw code patterns & tags
                                │
                                ▼
     [STAGE 2: Trajectory Alignment (Outland Path-Raycast)]
          • Map directory structures to 5D Space
          • Cast ray from target footprint toward crates/
          • Resolve semantic overlap (z-axis & Jaccard)
                                │
                                ▼
         [STAGE 3: Weld Strategy (Gemma warm LLM)]
          • Load aligned context into Warm LLM
          • Synthesize step-by-step AGENT.md Weld Plan
                                │
                                ▼
       [STAGE 4: Monolith Fusion (Weld Mode Execution)]
          • Apply edits via OODA-Weld loop (YOLO)
          • Validate compilation, tests, and ASP invariants
```

### Stage 1: Target Lock (Goldminer Line-Scan)
*   **Action:** The process points `goldminer-core` at the target external codebase.
*   **Mechanics:** It compiles a fresh line-level 5D index of all source files in the target directory, skipping VCS and dependency noise (`node_modules`, `.git`, `target`).
*   **Output:** A high-resolution codebook mapping every line of code to its 5D syntactic signature.

### Stage 2: Trajectory Alignment (Outland Path-Raycast)
*   **Action:** The process uses `outland-index` to map the workspace directory hierarchy.
*   **Mechanics:** It projects the paths of both the target codebase and the canonical workspace into the 5D path-space. It casts a 5D ray `from` the external codebase footprint `toward` the workspace structure (`F:\NewRepo\crates`).
*   **Output:** The system identifies exact semantic alignments (e.g., detecting that an external module's `sound/` components align with the `crates/forge-audio` z-coordinate and MinHash token signatures). It maps where each source component should land in the monolith.

### Stage 3: Weld Strategy (Gemma Synthesis)
*   **Action:** The pipeline passes the 5D spatial alignments, matched path mappings, and associated code blocks to the warm **Gemma** LLM reasoning engine.
*   **Mechanics:** Gemma performs deep semantic analysis, comparing file interfaces and identifying structural overlaps (e.g., public API methods, configuration keys, or duplicate utilities).
*   **Output:** Gemma writes a deterministic, step-by-step integration blueprint: `AGENT-weld-tracktor.md`. This file lists the exact files to create, files to modify, and functions to unify, leaving zero room for human guesswork.

### Stage 4: Monolith Fusion (Weld Mode Execution)
*   **Action:** The workspace transitions into safe `WELD-MODE` [PROVEN:GEMINI.md:ooda-weld] to execute the weld plan.
*   **Mechanics:** The runner executes the instructions in `AGENT-weld-tracktor.md` sequentially. Each edit is applied surgically, followed by:
    1.  Running `cargo check` on the modified crate to ensure strict type safety.
    2.  Executing `cargo test -p <crate>` to verify behavioral correctness.
    3.  Feeding generated material or configuration specs through local Answer Set Programming (ASP) solvers to guarantee compliance with the physical world laws of the simulation engine.
*   **Outcome:** The external module is completely dissolved and integrated into the single-engine monolith, leaving no stale files, no dependency drift, and maintaining 100% architectural integrity.
