# 15. Road Mapping and Convergence

This chapter documents the synthesized strategic roadmap and the current convergence of the 13Forge Sovereign Stack.

## Strategic Foundation (Goldminer)
*   **Edge Autonomy:** Full independence from cloud dependencies; local inference (`13forge-studio.exe`).
*   **1-Atom Paradigm:** Unified atomic properties for visual/acoustic simulation (`VixelAtom`).
*   **Vixi Consolidation:** Migration of legacy hand-drawn UI to dynamic `.kit.vixi` interfaces (Shell Fold).
*   **Proven Distillation:** Bidirectional training flywheel feeding synthetic data into the NDE model ladder.

## Structural Cornerstones (Diamond)
*   **Fractal Gating:** Isolate ➔ Synthesize pattern (WASI sandboxing, OneByteExpert).
*   **Dual-Clock Firewall:** Deterministic 120Hz CPU clock vs. creative GPU presentation clock.
*   **Modular Ecosystem:** Trait-based seams for `forge-` crates (`ml`, `core`, `sieve`, `hal`).
*   **ML Flywheel:** Runtime agent decision-making via `MetaRouter` + LoRA procedural adapters.

## UI/UX: Constellation Training Bar
The **Constellation UI** serves as the primary visual interface for the Flywheel distillation process, implemented in `F:\NewRepo\crates\forge-gui\src\constellation_kit.rs`.

*   **Component:** `ConstellationState` + `render_constellation`.
*   **Visualization:** Displays Flywheel process status ("DISTILL: ALIVE"/"DEAD"), `total_cases_processed`, and `total_lora_generations` formatted by `fmt_stats`.
*   **Control:** The UI is a read-only observer of the `SignalBus`. Distillation state is controlled via `nde-live`.

## Current Roadmap Implementation
The implementation is gated by the `ROADMAP.json` file. Key active lanes include:
*   `foundation.host-vfs-sandbox` (Host-managed VFS)
*   `foundation.native-web-mirror` (Shared WGSL/CPU overlay)
*   `foundation.vixel-canvas` (Atomic painting/physics)
