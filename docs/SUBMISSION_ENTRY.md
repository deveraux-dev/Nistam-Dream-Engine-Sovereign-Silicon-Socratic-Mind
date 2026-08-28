# Nistam Dream Engine & the Forge Engine — Competition Entry

**Project:** 1.58-bit balanced-ternary Gemma inference — a 9B backbone in 1.72 GB, 2.0 Gweights/s per CPU core, and a GPU kernel measured at 22.9 tok/s on an RTX 3070 (bit-identical to the CPU reference; 268 tok/s bandwidth roofline named) — with a 3-model resident quantized fleet (2.71 GB total VRAM), a 7-centroid BQ MetaRouter MoE, a Google Cloud Vertex AI gemini-3.7-flash cloud governor, and a playable **Tauri v2 demo shell**: a full-window 5D star sky (119,625-star HYG catalog, WebGL2 bloom + godrays), a birth-rite / CYOA narrative arc that completes at the Toll Gate and persists to disk, an M5 worldbuilder canvas, and a ConPTY glass terminal.

**Author:** Sean Morin, Edmonton, Alberta.
**License:** MIT OR Apache-2.0. **Research DOI:** [10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676) — *Pararity: the fixed-point residue of an involution, and why we need it* (Zenodo, 2026).

## What it is: The Three-Legged Sovereign Architecture

1. **Containment (The Architecture of Equilibrium)**:
   - S13 balanced-ternary lattice invariants ($\mathbb{T} = \{-1, 0, +1\}$) eliminate offset bias and drift without floating-point accumulation.
   - Zero-heap execution mandate with ADR-0026 zeroization on drop.

2. **Integrity (The Merkle-Morin Architecture / MMA)**:
   - 1.58-bit packed ternary weights (5 trits/byte) unpacked in registers via AVX2 `PSHUFB` (2.57 Gtrits/s scalar, 37.06 Gtrits/s AVX2).
   - **Three Bears fleet** (`gemma-s13`): 3 models concurrently resident in GPU VRAM (Baby Bear 2B, Blind Mama Bear 9B, Papa Bear 27B head = **2.71 GB total** VRAM, fits an 8 GB consumer GPU) with 59.62 GB/s staging memory rotation and 7-centroid BQ MetaRouter MoE dispatch.

3. **Transport & Autonomous Agent**:
   - `crates/forge-envelope/scripts/agent_loop.py` — an autonomous watcher, not a chat interface: polls a Cloud Storage inbox, runs deterministic ByteSieve triage, requests a schema-locked audit from gemini-3.7-flash on Vertex AI at temperature 0.0, cross-checks against a degradation model, writes the evidence-chain head to Firestore, and wipes local staging only after acknowledgement. No human in the loop.
   - Cloud Governor: serverless Vertex AI context cache (`gemini-3.7-flash`, temperature 0.0, `$0.0004/call` governor ceiling).
   - Demo shell (`crates/studio-tauri`): 5D star sky, CYOA arc, M5 canvas, and ConPTY glass terminal. Builds with plain `cargo build --release` (no Node).

## Measured receipts

Every number below is a command you can run yourself; method and raw stdout are in [`docs/BENCHMARKS.md`](BENCHMARKS.md). This section states only what is verified in this session's own tool output plus the numbers already published in `README.md` and `docs/DEVPOST.md` of this repository — no figure here is asserted beyond what those files already carry.

| Claim | Measured |
| :--- | :--- |
| 512-bit BQ MetaRouter routing | 1.90–2.75 M decisions/s single-core |
| 400×400 conjugate grid inversion | 2.57–2.68 Gtrits/s scalar / 37.06 Gtrits/s AVX2 |
| Double-buffered host staging | 59.62–60.30 GB/s |
| Tile geometry planning (Ampere 32×32) | 358.17–364.56 M plans/s |
| Three Bears resident fleet VRAM | 2.71 GB total (Baby Bear 2B + Blind Mama Bear 9B + Papa Bear 27B head) |
| Gemma 2B / 3.2B / 9B decode (GPU, RTX 3070) | 82.5 / 54.7 / 22.9 tok/s |
| 5D star sky projection (119,625 HYG stars) | 44.45 M stars/sec, zero heap |
| Airgap red/green | 5/5 red vectors blocked (`scripts/test_sovereign_airgap_red_green.py`) |

## Demo

- **Video:** [https://youtu.be/ttMofC_9-G0](https://youtu.be/ttMofC_9-G0) (≤4:00, English subtitles).
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

**Submission Period: August 3 – August 31, 2026.** This entry incorporates the author's own pre-existing Rust workspace (referred to internally as the Forge Engine / 13Forge V3) as a dependency library — the trit primitives, GPU warden, and BQ router substrate predate this window (earliest active development 2026-05-09, per file records in the source tree). The work built and submitted during this window is: the S13 balanced-ternary Gemma fleet quantization, the CPU/GPU benchmark suite, the autonomous Vertex AI audit agent (`agent_loop.py`), the 5D relativistic star-sky projection, and the Tauri v2 demo shell (`crates/studio-tauri`) that plays all of it back.

The formal mathematics underlying the ternary encoding was published in-window: *"Pararity: the fixed-point residue of an involution, and why we need it"*, [DOI 10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676), CC BY 4.0.

**Third-party open source incorporated** (permitted per rule G10; enhances rather than repackages): `candle` / `candle-transformers` (HuggingFace, Apache-2.0 OR MIT), plus their unmodified dependencies (`candle-core`, `candle-nn`, `tokenizers`, `safetensors`, `half`, `zip`, `bytemuck`), and `windows-sys` (Windows targets only).

`<CONFIRM>` — Sean: verify the disclosure above against your own recollection of what predates August 3 vs. what was built in-window; the dates are read from disk this session, the account of history is yours to correct.
