# Ironroot Final Engine Patch

This bundle contains two repo-ready Rust crates plus deterministic architecture notes.

## Crates

- `crates/forge-ump`: zero-alloc MIDI 2.0 UMP byte-stream to typed-event parser.
- `crates/ironroot-signal`: generic signal-routed creation and ambience pipeline for Ironroot-style deterministic engines.

## Docs

- `docs/deterministic_signal_architecture.md`
- `docs/expanded_signal_considerations.md`
- `docs/integration_plan.md`
- `docs/theory_engine_compiler_report.md`

## Install

Copy the contents into your repo root, or copy individual crates into your existing `crates/` directory.

```bash
cargo test -p forge-ump
cargo test -p ironroot-signal
```

## Design law

Live external signals are expressive, unstable, and non-authoritative. Simulation state is deterministic, quantized, ledgered, and replayable.


## UMP alpha patch notes

See `docs/ump_alpha_fixes.md` for the applied verification fixes: invalid-status test vector correction, `bytemuck` dependency cleanup, `UmpAuthorityTicket`, minimal `forge-core` lane/hash surface, and `CarrierKind::UmpTicketPack = 10`.
