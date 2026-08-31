# Nistam Dream Engine & the Forge Engine — Competition Entry

**Project:** 1.58-bit balanced-ternary Gemma inference — a 9B backbone in 1.72 GB, 2.0 Gweights/s per CPU core, and a GPU GEMV decode kernel measured at 51.3 passes/s (427.4 Gweights/s) on an RTX 3070 (bit-identical to the CPU reference) — with a 3-tier quantized memory architecture (~3.0 GB on-disk layout), a 7-centroid BQ MetaRouter MoE, a Google Cloud Vertex AI gemini-2.5-flash cloud governor, and a playable **Tauri v2 demo shell**: a full-window 5D star sky (119,625-star HYG catalog, WebGL2 bloom + godrays), a birth-rite / CYOA narrative arc that completes at the Toll Gate and persists to disk, an M5 worldbuilder canvas, and a ConPTY glass terminal.

**Author:** Sean Morin, Edmonton, Alberta.
**License:** MIT OR Apache-2.0. **Research DOI:** [10.5281/zenodo.22176968](https://doi.org/10.5281/zenodo.22176968) — *Little Nistam and The Lattice of Harmony* (Zenodo, 2026). Mathematical foundations: [10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676) — *Pararity: the fixed-point residue of an involution, and why we need it* (Zenodo, 2026).

## What it is: The Three-Legged Sovereign Architecture

1. **Containment (The Architecture of Equilibrium)**:
   - S13 balanced-ternary lattice invariants ($\mathbb{T} = \{-1, 0, +1\}$) eliminate offset bias and drift without floating-point accumulation.
   - Zero-heap execution mandate with ADR-0026 zeroization on drop.

2. **Integrity (The Merkle-Morin Architecture / MMA)**:
   - 1.58-bit packed ternary weights (5 trits/byte) unpacked in registers via AVX2 `PSHUFB` (2.57 Gtrits/s scalar, 37.06 Gtrits/s AVX2).
   - **Quantized Fleet Layout** (`gemma-s13`): 3,049.6 MB (~3.0 GB) on-disk storage (`s13_gemma_9b_m3` 1,838.0 MB, `s13_gemma_2b_m3` 446.4 MB, `s13_gemma_m2` 765.2 MB) driving 5 concurrent resident execution seats in VRAM with 59.62 GB/s staging memory rotation and 7-centroid BQ MetaRouter MoE dispatch.

3. **Transport & Autonomous Agent**:
   - `crates/forge-envelope/scripts/agent_loop.py` — an autonomous watcher, not a chat interface: polls a Cloud Storage inbox, runs deterministic ByteSieve triage, requests a schema-locked audit from gemini-2.5-flash on Vertex AI at temperature 0.0, cross-checks against a degradation model, writes the evidence-chain head to Firestore, and wipes local staging only after acknowledgement. No human in the loop.
   - Cloud Governor: serverless Vertex AI context cache (`gemini-2.5-flash`, temperature 0.0, 75% input discount at $0.01875/1M cached tokens).
   - Agent framework: **Antigravity** — Gemini 2.5 Flash drives the Forge Engine (daemon :13013, inference = verb 9). Google Cloud project: `nde1-493505`.
   - Demo shell (`crates/studio-tauri`): 5D star sky, CYOA arc, M5 canvas, and ConPTY glass terminal. Builds with plain `cargo build --release` (no Node).

## Measured receipts

Every number below is a command you can run yourself; method and raw stdout are in [`docs/BENCHMARKS.md`](BENCHMARKS.md). This section states only what is verified in this session's own tool output plus the numbers already published in `README.md` and `docs/DEVPOST.md` of this repository — no figure here is asserted beyond what those files already carry.

| Claim | Measured |
| :--- | :--- |
| 512-bit BQ MetaRouter routing | 1.90–2.75 M decisions/s single-core |
| 400×400 conjugate grid inversion | 2.57–2.68 Gtrits/s scalar / 37.06 Gtrits/s AVX2 |
| Double-buffered host staging | 59.62–60.30 GB/s |
| Tile geometry planning (Ampere 32×32) | 358.17–364.56 M plans/s |
| Model Storage & Fleet Layout | 3,049.6 MB (~3.0 GB) on disk (`s13_gemma_9b_m3` + `s13_gemma_2b_m3` + `s13_gemma_m2`), 5 resident VRAM seats |
| Gemma Sidecar Live CUDA Generation (RTX 3070) | **25.81 tok/s** decode (38.75 ms/tok all-in), **55.4 tok/s** prefill (Gemma-3 4B Q4_K_M + 70 S13 overrides + 70 LoRA repairs) |
| SplitShader Determinism Proof (RTX 3070 Vulkan) | Bit-identical CPU == GPU (diff = 0 across native i64 and dual-u32 emu) |
| Gemma 9B GEMV Kernel (GPU, RTX 3070) | 51.3 passes/s (19.48 ms/pass, 427.4 Gweights/s, 42 layers in VRAM) |
| Gemma 2B GEMV Kernel (GPU, RTX 3070) | 95.0 passes/s (10.52 ms/pass, 192.4 Gweights/s, 26 layers real weights) |
| Gemma 9B CPU Decode (Fallback Baseline) | 0.48 tok/s (2.08 s/tok, AVX2 + Rayon) / 0.03 tok/s scalar |
| 5D star sky projection (119,625 HYG stars) | 44.45 M stars/sec, zero heap |
| Airgap red/green | 5/5 red vectors blocked (`crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py`) |
| Vertex AI Context Caching | 41,002 tokens cached, 74.2% measured cost reduction ($0.000801 billed) |

## Demo

- **Video:** [https://youtu.be/fkdUBRbHdfk](https://youtu.be/fkdUBRbHdfk) (≤3:00, English subtitles).
- **Run it yourself:** prerequisites are Rust stable and Windows WebView2 only (no Node). Full judge instructions: [`docs/JUDGE-BUILD.md`](JUDGE-BUILD.md).

```bash
cd crates/studio-tauri
cargo build --release
```

## Reproduce

```bash
./test.bat                                                         # 1-click master test suite
cargo test --workspace                                             # NOTE: does not reach crates/studio-tauri or shell/ (gate separately)
cargo test --manifest-path crates/gemma-s13/Cargo.toml
python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py
python scripts/test_vertex_cache_strict.py
```

Cloud verification requires your own GCP project (`GOOGLE_CLOUD_PROJECT`) and bills at Vertex AI list rates.

## Pre-existing work disclosure (competition rule G8)

**Submission Period: August 3 – August 31, 2026.** The author began developing this software in November 2025. The VARS format and the Vertex AI Flash context-caching layer were built in December 2025, and the author has been developing the underlying engine (referred to internally as the Forge Engine / 13Forge V3 — the trit primitives, GPU warden, and BQ router substrate) continuously since that time. All of this predates the Submission Period and is disclosed here as pre-existing work incorporated into the Project, per rule G8.

Built specifically for this submission, during the Submission Period: the five-plus technical white papers and the bulk of the deep mathematical/architectural work documenting and extending that engine — including the formal ternary-encoding mathematics published as *"Pararity: the fixed-point residue of an involution, and why we need it"*, [DOI 10.5281/zenodo.22176968](https://doi.org/10.5281/zenodo.22176968), CC BY 4.0 — together with the S13 balanced-ternary Gemma fleet quantization, the CPU/GPU benchmark suite, the autonomous Vertex AI audit agent (`agent_loop.py`), the 5D relativistic star-sky projection, and the Tauri v2 demo shell (`crates/studio-tauri`) that plays all of it back.

**Third-party open source incorporated** (permitted per rule G10; enhances rather than repackages): `candle` / `candle-transformers` (HuggingFace, Apache-2.0 OR MIT), plus their unmodified dependencies (`candle-core`, `candle-nn`, `tokenizers`, `safetensors`, `half`, `zip`, `bytemuck`), and `windows-sys` (Windows targets only).
