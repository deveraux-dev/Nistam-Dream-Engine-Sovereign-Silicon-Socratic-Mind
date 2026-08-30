# SESSION HANDOFF & CONTEXT PRIMER — Outland, Goldminer, Gemma, and `/tracktor-beam`
**Date:** Sunday, July 26, 2026
**Session Status:** completed (Research & Synthesis completed; no code modified per Inquiry scope)
**Workspace Directories:** `F:\NewRepo`, `C:\Users\seanm\.gemini\tmp\newrepo`

---

## 1. Executive Summary & Session Outcomes

This session performed a deep, structural and mathematical review of the **Outland** and **Goldminer** 5D search systems in the 13Forge workspace, analyzed their relationship to the **Gemma** LLM engine, and synthesized a new process called **`/tracktor-beam`** for consolidating/folding external codebases into our single-engine monolith.

### Delivered Artifacts on Disk:
*   **Synthesis Report:** `F:\NewRepo\outland_goldminer_gemma_synthesis.md` [PROVEN:F:\NewRepo\outland_goldminer_gemma_synthesis.md:1]
*   **Sandbox Copy:** `C:\Users\seanm\.gemini\tmp\newrepo\outland_goldminer_gemma_synthesis.md` [PROVEN:C:\Users\seanm\.gemini\tmp\newrepo\outland_goldminer_gemma_synthesis.md:1]
*   **Handoff/Primer (This File):** `F:\NewRepo\outland_goldminer_gemma_synthesis_handoff.md`

---

## 2. Decisive Technical Findings & System Overlaps

### A. The 5D GhostMoon Box `[x, y, z, theta, w]`
Both Outland and Goldminer use an integer-exact, sum-of-squares squared-Euclidean distance formula:
$$d = \sum_{i=0}^{4} (\Delta c_i)^2$$

*   **Lane 3 ($\theta$, Angular Lane):** This lane wraps at $360,000$ millidegrees, requiring specialized wrap-aware subtraction [PROVEN:crates/forge-ml/src/nearest_neighbor.rs:41].
*   **Lane 2 ($z$, Semantic/Family Lane):**
    *   **In Outland:** Based on 20 predefined keyword/stem categories (scaled by $\text{FAMILY\_STEP} = 16,384$) [PROVEN:crates/outland/src/lib.rs:31].
    *   **In Goldminer/River:** Based on shape/line-length metric.
*   **Lanes 0, 1, 3, 4 ($x, y, \theta, w$):**
    *   **In Outland:** MinHash (Jaccard) token approximation for Lanes 0, 1, 3, and whole-string exact fold for Lane 4 [PROVEN:crates/outland/src/lib.rs:142].
    *   **In Goldminer/River:** Tag cluster, payload discriminator, token order, and token set.

### B. Core Overlap
1.  **Syntactic & Local:** Both systems are completely "Gemma-free" during indexing, utilizing high-throughput deterministic hashing (`fnv1a` / MinHash) rather than heavy neural net models. This allows sub-second indexing of 100,000-line repositories on standard hardware.
2.  **Raycasting:** Both rank matches based on perpendicular distance to a 5D ray cast from a `from` vector through a `toward` vector.
3.  **Compilation Health:** Both crates compile and pass verification tests flawlessly:
    *   `cargo check -p outland-index` -> `EXIT 0` [PROVEN:run_shell_command:cargo check -p outland-index]
    *   `cargo check --example outland_cli` -> `EXIT 0` [PROVEN:run_shell_command:cargo check --example outland_cli]

---

## 3. The Synthesized `/tracktor-beam` Process

This new process integrates the local line search of **Goldminer**, the path alignment of **Outland**, and the semantic reasoning of **Gemma** to haul and weld external code repositories or orphaned packages directly into our monolith:

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

1.  **Stage 1: Target Lock (Goldminer Line-Scan):** Uses `goldminer-core` [PROVEN:crates/goldminer-core/src/lib.rs:1] to scan the target codebase, creating a line-level 5D index.
2.  **Stage 2: Trajectory Alignment (Outland Path-Raycast):** Projects file path hierarchies into 5D space with `outland-index` [PROVEN:crates/outland/src/lib.rs:215], casting a ray toward `crates/` to match candidate structures.
3.  **Stage 3: Weld Strategy (Gemma Synthesis):** Feeds the matches into the warm **Gemma** model [PROVEN:MOAT_AUDIT.md:9], which writes a step-by-step integration plan: `AGENT-weld-tracktor.md`.
4.  **Stage 4: Monolith Fusion (Weld Mode Execution):** Exercises safe `WELD-MODE` [PROVEN:GEMINI.md:ooda-weld] to execute the plan sequentially, validating edits via `cargo check` and `cargo test` gates.

---

## 4. Next Steps for Next Session

Once a **Directive** is issued:
1.  **Implement the `/tracktor-beam` Tool:** Build a thin wrapper command or script (e.g., in `xtask` or `scripts/`) that exposes the synthesized `/tracktor-beam` workflow.
2.  **Verify the Integration:** Run `/tracktor-beam` on a sample external crate (or an attic-retired folder in `_attic/` [PROVEN:session_context:_attic]) and confirm it generates the expected `AGENT-weld-tracktor.md` plan and merges successfully.
