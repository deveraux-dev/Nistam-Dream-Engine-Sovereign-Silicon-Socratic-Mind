# Benchmarks — Method and Reproduction

This document does not publish figures. Every benchmark below is a command that
prints its own numbers on the machine that runs it. Fill the Result column from
your own run; that is the only number this project asks you to trust.

Prior dated receipts are retained unmodified under
`docs/_archive-benchmarks-2026-08-27/`. They record specific hardware on specific
days and are superseded by whatever your run prints.

## Why no numbers here

These are single-core, cache-resident microbenchmarks. They move with CPU model,
thermal state, cache pressure, and compiler version. A figure copied into prose
goes stale on the next run and then reads as a claim rather than a measurement.
The commands are the durable artifact.

## Hardware to record with any result

CPU model · core count · RAM · GPU model and VRAM · OS · `rustc --version`.
Without these a throughput number means nothing.

## CPU and memory

| Benchmark | Command | Result |
|---|---|---|
| Conjugate grid sign inversion (scalar + AVX2), BQ MetaRouter routing, host staging, tile planning | `cargo run --release --example mtok_throughput_bench -p forge-gpu-warden-v3` | _(prints on run)_ |
| Ternary distance LUT vs per-trit decode | `cargo run --release --example trit_dist_bench -p forge-core-v3` | _(prints on run)_ |
| Star-lock attitude resolve over the HYG catalog, zero-heap | `cargo run --release --example probe_astrolabe_runtime -p gemma-s13` | _(prints on run)_ |
| Clockspine contention under load | `cargo run --release --example trit_dist_contention -p forge-hal-clockspine` | _(prints on run)_ |

## GPU decode

Requires a WebGPU adapter.

| Benchmark | Command | Result |
|---|---|---|
| 9B geometry, synthetic weights | `cargo run --release --example gpu_decode_timed -p gemma-s13` | _(prints on run)_ |
| Real quantized seat, bit-parity checked | `S13_GEMMA_DIR=<dir> cargo run --release --example gpu_decode_real -p gemma-s13` | _(prints on run)_ |
| KV prefill snapshot restore | `cargo run --release --example kv_prefill_cache -p gemma-s13` | _(prints on run)_ |
| GPU dispatch floor | `cargo run --release --example gpu_dispatch_floor -p gemma-s13` | _(prints on run)_ |

`gpu_decode_real` needs `.s13m` tensors produced by `quantize-s13 pack-gemma` from
a Gemma checkpoint. Quantized weights are not distributed in this repository, so
this row is unrunnable without supplying your own checkpoint. Every other row runs
from a clean clone.

## Protocol and sovereignty

| Check | Command | Result |
|---|---|---|
| MMA-over-Nostr: BIP-340 attestation, O(1) Merkle gate, Byzantine injection refused, ADR-0026 scrub | `cargo run --release --example mma_nostr_live_demo -p forge-daemon-door` | _(prints on run)_ |
| Airgap red/green vectors | `python scripts/test_sovereign_airgap_red_green.py` | _(prints on run)_ |

The airgap script imports `vertex_flash_cache` from the same directory and needs the
Python dependencies in `crates/forge-envelope/requirements.txt`
(`pip install -r crates/forge-envelope/requirements.txt`).

## Test suites

| Suite | Command | Result |
|---|---|---|
| Workspace | `cargo test --workspace` | _(prints on run)_ |
| Demo shell (excluded from the workspace, `Cargo.toml:44`) | `cargo test --manifest-path crates/studio-tauri/Cargo.toml` | _(prints on run)_ |

`crates/studio-tauri` and `shell` are excluded at `Cargo.toml:44`, so
`cargo test --workspace` does not gate them. Run them separately.

## Control surface

The Forge Engine daemon listens on `127.0.0.1:13013`
(`crates/forge-daemon-door/src/protocol.rs:9`). Its verb table is asserted at a
fixed length by a unit test in `crates/forge-daemon-door/src/wire.rs`; read the
assertion for the current count rather than quoting one here.

```
cargo run -p forge-daemon-door --bin door -- status
```
