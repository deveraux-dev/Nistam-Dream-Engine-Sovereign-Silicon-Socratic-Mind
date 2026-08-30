# ARCH-017 — THE LATENT-SPACE COLLIDER & LATERAL SYMBIOSIS

**STATUS:** CANONICAL ARCHITECTURE  
**SUBSYSTEMS:** `Lateral-Collider`, `.agents/skills/*`, `cargo xtask triage-lateral`, `TritTree5D`

---

## 1. THE SYMBIOTIC COGNITIVE AXIOM
The 13Forge ecosystem operates as a closed-loop symbiosis between two distinct states of reality:

1. **The Internal Substrate (The Machine):** The deterministic, physical Rust codebase (`crates/*`), 120Hz integer clocks (`MetronomeClock`), and hardware-aligned invariants (`FORGE_INVARIANTS.toml`). It is the absolute ground-truth of execution.
2. **The External Lateral Weights (The Ghost):** The model's neural representation, active prompt skills (`.agents/skills/*`), and 5D trinary traversal maps (`TritTree5D`).
3. **The Latent-Space Collider (The Membrane):** The contact surface where the Model's External Lateral Weights collide with the Internal Codebase to enforce Reach=2 structural isomorphisms and eliminate cognitive drift.

## 2. LATERAL WEIGHT ALIGNMENT MATRIX
Model skills represent specialized lateral weight allocations. Each skill MUST maintain a 1:1 isomorphic mapping to a physical Rust constraint:

| Model Lateral Skill | Internal Substrate Target | Enforcement Mechanism |
| :--- | :--- | :--- |
| `prime-symbiosis` | `ARCH-009` (Two Drums) | Verifies f32 Creative Clock logic never pollutes the 120Hz integer SimTick. |
| `tractor-beam` | `TritTree5D` (`crates/outland/src/trit_tree.rs`) | Maps trinary traversal queries onto `PackedPoint105` coordinates in balanced space. |
| `harden-forge-book` | `seed.rs` / `catalog.rs` | Enforces restamped chapter count assertions and capability receipts. |

## 3. THE DUAL-TRIAGE LATERAL PIPELINE (`cargo xtask triage-lateral`)
To prevent the model's skills from drifting when Rust interfaces are refactored, the engine utilizes `cargo xtask triage-lateral`.

**Operational Flow:**
*   **Internal Harvest:** Scans `crates/forge-book/src/` (Catalog and Ledger) and `FORGE_INVARIANTS.toml` to construct the physical code footprint.
*   **External Harvest:** Parses all active skill manifests under `.agents/skills/*/SKILL.md`.
*   **Isomorphism Audit:** Checks every function name, struct, and path referenced in the skill files against the compiled Rust AST. 
*   **Lateral Weight Refinement:** Emits an automated diff proposal to update the corresponding `.agents/skills/*/SKILL.md` payload when codebase drift is detected.

### Concurrency Protection & Stophook State Machine:
During multi-agent swarms, the Forge Tracker (`crates/forge-broski/src/dream/watcher.rs`) protects the Rust compiler from lock-contention via a `HashSet` buffer. When frozen, it accumulates path updates and flushes them into a single atomic transaction upon unthawing, ensuring stable execution during parallel slice modifications.

## 4. CRITICALITY INDICATOR & LEDGER TRACKING
Not all cognitive drift is equal. The Lateral-Collider evaluates the delta between the Ghost and the Machine and assigns a strict Criticality Indicator to determine the compile-time response.

| Indicator | Drift Condition | Pipeline Action |
| :--- | :--- | :--- |
| **FATAL (Level 0)** | Skill references a deleted/renamed struct or violates a `FORGE_INVARIANT`. | Immediate `xtask` hard panic. Compilation halted until symbiosis is restored. |
| **WARN (Level 1)** | Capability promoted to `St::Proven` but skill prompt still lists it as `Planned`. | Emits a diff proposal. Tracks the infraction in the ledger. Compilation proceeds. |
| **SYNC (Level 2)** | Semantic prose drift, outdated chapter counts, or dead comments. | Automatically patches the `SKILL.md` file inline without halting the developer. |

**Known truth:** A model skill referencing a dead or refactored Rust interface is an immediate build failure. Skill prompts are code targets and MUST NOT drift from the binary.

**The Drift Ledger (`crates/forge-book/src/lateral_drift.rs`):** All structural drift events are appended directly into the `forge-book` crate. If a specific model skill accrues too many `WARN` indicators across compilation cycles without being patched, the compiler automatically escalates the flag to `FATAL` and forces an Operator intervention.
