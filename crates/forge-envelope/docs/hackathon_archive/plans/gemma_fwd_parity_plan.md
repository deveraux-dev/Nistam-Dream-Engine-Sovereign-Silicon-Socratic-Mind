# Plan for Gemma Forward GPU Parity Test

The goal is to provide a compilable `// [BOARD: GEMMA-FWD]`-tagged Rust `#[test]` demonstrating GPU forward token/argmax parity against the CPU oracle, supporting honest self-skipping if the GPU device or model is absent.

## Research Findings
1. **FEATURE-GATE VERDICT**: The module `gemma_gpu` is gated behind `#[cfg(all(feature = "gemma", feature = "gpu-train"))]` in `crates/forge-daemon/src/lib.rs:22-23`.
2. **HARVEST REACHABILITY**: `cargo xtask board` accepts custom arguments and forwards them to `cargo test`. Hence, running `cargo xtask board -p forge-daemon --features gemma,gpu-train` enables correct test compilation, execution, and tag harvesting.
3. **PLACEMENT**: The test code resides in `crates/forge-daemon/tests/gemma_fwd_parity.rs`.
4. **THE TEST CODE**: Provided below.

## Verification & Run Command
`cargo test -p forge-daemon --test gemma_fwd_parity --features gemma,gpu-train --release`
