# Implementation Plan: Gemma Hybrid GPU Inference (Matmul/WGSL)

## Objective
Migrate the Gemma engine (`crates/forge-daemon/src/gemma_engine.rs`) from CPU-only (`Device::Cpu`) to a hybrid GPU-accelerated inference backend using `candle-core` with `Device::Wgpu`, leveraging existing `wgpu` infrastructure for cross-platform portability.

## Background & Motivation
The current CPU-bound inference is limited by bandwidth and compute on the main CPU thread. Leveraging `wgpu` for matmul and shader-based kernel execution will offload heavy tensor math to the GPU, aligning with the `13Forge` sovereign architecture goal of portable, high-performance compute on all supported hardware.

## Scope & Impact
- **Affected File:** `crates/forge-daemon/src/gemma_engine.rs`
- **Dependencies:** `candle-core` (requires `wgpu` feature enabled), `wgpu` (already used in project).
- **Architecture:** Transition `GemmaTier` from `Device::Cpu` to `Device::Wgpu`. 
- **Hybrid Strategy:** Initialize `Device::Wgpu` using the existing GPU context (if possible, to avoid context overhead). Use `candle`'s built-in `wgpu` matmul implementation as the primary acceleration layer.

## Proposed Solution
1. **Infrastructure:** Enable `wgpu` feature for `candle-core` in `crates/forge-daemon/Cargo.toml`.
2. **Context Sharing:** Integrate with `crate::platform` to obtain the existing `wgpu` instance/device to prevent context-sharing contention or redundant driver calls.
3. **Engine Update:** Update `GemmaTier` to store `Device::Wgpu`.
4. **Fallback:** Maintain `Device::Cpu` as a failsafe if `wgpu` initialization fails, as per current Signal Law guidelines ("LOUD `Err`").
5. **Validation:** Implement unit tests to verify `Device::Wgpu` initialization and parity against current CPU outputs.

## Alternatives Considered
- **Direct CUDA/Metal:** Rejected due to portability constraints (13Forge mandates platform-independent WGSL/wgpu).
- **Full Custom Shaders:** Considered for specific kernels (e.g., rotary embeddings), but `candle`'s `wgpu` matmul is sufficient for baseline acceleration.

## Phased Implementation Plan
1. **Ph1:** Dependency enablement (Cargo.toml).
2. **Ph2:** GPU Device initialization integration within `load()`.
3. **Ph3:** Refactor `GemmaTier` to handle `Device` switching (or hybrid device).
4. **Ph4:** Parity validation and performance benchmarking.

## Verification
- `cargo test -p forge-daemon --features gemma,wgpu`
- Empirical proof of GPU utilization (monitor tool/debugger).
- Parity: `gemma_infer` output (GPU) == `gemma_infer` output (CPU) within float-epsilon tolerance.

## Migration & Rollback
- **Rollback:** Revert `Device` instantiation to `Device::Cpu` if `wgpu` device fails to acquire.
