# Integration Plan

## Target

This patch gives the repo two bounded Rust surfaces:

1. `forge-ump`: byte-stream to typed UMP event parser.
2. `ironroot-signal`: generic signal-routed creation and ambience primitives.

## Placement

Recommended repo layout:

```text
crates/
  forge-ump/
  ironroot-signal/
docs/
  deterministic_signal_architecture.md
  expanded_signal_considerations.md
  integration_plan.md
  theory_engine_compiler_report.md
```

## Non-authoritative signal rule

External signals may shape presentation, creation previews, ambience, and authored asset metadata. They should not directly mutate combat truth, save proofs, economy, hitboxes, damage, or deterministic replay state.

## Runtime lanes

```text
Lane 0: critical simulation and save proofs
Lane 1: deterministic gameplay modifiers
Lane 2: authored asset metadata
Lane 3: speculative presentation
Lane 4: discardable ambience and UI flourish
```

`ironroot-signal` is intended for lanes 2-4 by default. Lane 1 requires explicit quantization, bounded deltas, and ledger records.

## First integration steps

1. Add both crates to the workspace.
2. Run tests.
3. Replace placeholder ID wrappers with repo canonical IDs if they already exist.
4. Replace `CreationStamp::stable_hash` with the repo proof-hash function if available.
5. Route live input only into `SignalProxy` or a similar async boundary.
6. Commit only quantized `CreationEvent` / `WorldFluxEvent` records.

## Cargo

```bash
cargo test -p forge-ump
cargo test -p ironroot-signal
```
