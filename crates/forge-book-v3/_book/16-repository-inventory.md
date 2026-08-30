# 16. Repository Inventory

## Overview
This chapter provides a consolidated inventory of the project's repository structure, cataloging major functional areas to ensure transparency and enable efficient navigation of the 13Forge Sovereign Stack.

### Technical Landscape Overview

| Category | Key Directories | Function |
| :--- | :--- | :--- |
| **Engine Core** | `crates/`, `forge-gemma/`, `nde-models/` | The heart of the Ironroot engine: ML models, core Rust crates, and NDE inference machinery. |
| **Agent Infrastructure**| `.agents/`, `.claude/`, `.forge/` | Configuration, agent definitions, skill libraries (`.skill`), hooks, memory, and task state tracking. |
| **Project Documentation**| `docs/`, `_book/`, `_scratch/` | Intellectual scaffold, technical specifications, invariants (`FORGE_INVARIANTS.toml`), and roadmap ledgers. |
| **Procedural Logic** | `forge-vision/`, `lora_swarm/`, `forge-dialogue/` | Tools for procedural generation, rendering proofs, audio, and agent-interaction management. |
| **Staging & Vaults** | `_staging/`, `_vault/`, `_refinery/` | Intermediate work-in-progress, long-term archival, and experimental integration zones. |

### Governance and Structural Principles
- **Fractal Gating:** Enforcement of "Isolate ➔ Synthesize" patterns via sandboxed WASI execution.
- **Dual-Clock Firewall:** Separation between 120Hz CPU deterministic clock and GPU presentation clock.
- **Modular Ecosystem:** Trait-based seams for pluggable backends within the `crates/` directory.

---
*Inventory status: Synchronized as of 2026-07-27.*
