# Nistam Dream Engine — Sovereign Silicon & Socratic Mind

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.22176968.svg)](https://doi.org/10.5281/zenodo.22176968)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Google Gemini Competition](https://img.shields.io/badge/Google%20Gemini-Developer%20Competition-orange.svg)](docs/SUBMISSION_ENTRY.md)

**A 1.58-bit Gemma that runs under 8 GB of VRAM, a Plains Cree tokenizer that obeys grammar instead of byte statistics, and a Gemini 2.5 Flash governor that audits the machine without ever seeing the language it protects.**

> **Author's note.**
> Built by Sean Morin — half Cree, half Welsh, Edmonton river valley, Alberta.
>
> Twenty-three years an industrial painter, thirteen of them in refineries. Nine months a software engineer. I taught myself the Rust, the math, and the balanced-ternary architecture from zero, and this engine is what came out.
>
> Like me, it is real, direct, and flawed. It runs on my own silicon. It has raw edges. Where something is a stub, this README says stub. Where a measurement contradicted my hypothesis, the whitepaper says so and withdraws the old number. In nine months it has done everything I set out to make it do — 5D relativistic coordinate manifolds, balanced-ternary involutions, zero-retention cultural airgaps, live context caching.
>
> I built this with my own hands. I did my best.

---

## What it is

Three things, wired together in one Rust workspace:

1. **Gemma at 1.58 bits (S13).** Balanced-ternary quantization: weights are −1 / 0 / +1, five to a byte (3⁵ = 243 states). A byte holds 256, so 13 values can never be a weight — those are the alarms. A malformed byte has no state to decode into; nothing gets in. Gemma 2B and 9B are quantized from real weights. The 9B decodes end-to-end on CPU through all 42 layers; the 9B and 2B GEMV kernels are timed on an RTX 3070 with parity against the host reference.
2. **Plains Cree tokenized by grammar, not frequency.** The Zero-Shot Polysynthetic State Resolver (ZPSR): a Giellatekno/ALTLab FST for valid morpheme paths, GBNF logit masking so the decoder can only sample legal continuations, and an ASP/Clingo layer for animacy, obviation and direction-hierarchy agreement. Measured on the 1,587 corpus words the strict analyser covers: **2,545 FST segments vs 8,363 GPT-2 BPE tokens (−69.6%)**.
3. **Gemini 2.5 Flash as governor.** Vertex AI, temperature 0.0, context caching over a 41,002-token cached prefix, **74.2% measured input-token savings**. It audits scrubbed state envelopes and writes verdicts to Firestore. It never receives Cree text.

Underneath: the 119,625-star HYG catalog projected through SO(5) plane rotations and a Lorentz boost at **44.45 million stars/s**, zero heap on the hot path. The sky is not decoration — aspect angles (conjunct / trine) produce a fixed-point resonance multiplier that feeds the router.

## Where Gemini is — and where it isn't

This is a data-sovereignty project entered in a Google competition. That only works if the boundary is exact, so here it is.

| Data | Sent to Gemini 2.5 Flash | Stays on the host |
|---|:---:|:---:|
| Scrubbed state envelopes, hashes, audit prompts, cost receipts | ✓ | |
| Cree text, UCAS syllabics, macrons — *Wave 1* | | ✓ |
| Canonical verb stems — *Wave 2* | | ✓ |
| 13-Moons law names, OCAP markers — *Wave 3* | | ✓ |
| Model weights, inference, tokenization | | ✓ |

`forge-envelope` runs the three-wave filter before any outbound call (sub-45 ns constant-time check per envelope). On a hit the branch rolls back and every buffer is zeroized — ADR-0026, zero retention. `crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py` is the red/green proof. `scripts/demo_cloud_agent.ps1` shows the exact payload that crosses the wire; read it before trusting this table.

## Judge it in three commands

Windows host with Google Cloud credentials (see `docs/JUDGE-BUILD.md`). The Rust crates and Python scripts are cross-platform; the `.ps1` wrappers and the shell's ConPTY terminal are not.

```powershell
# End-to-end: Rust envelope -> Gemini 2.5 Flash -> Firestore -> scrub
.\scripts\demo_cloud_agent.ps1

# One context-cached Gemini call with a live token/cost receipt
python scripts/vertex_flash_cache.py --prompt "Audit surface envelope hash #01 for degradation"

# 3-minute suite: 401+ tests, measured silicon, zero mocks
python scripts/run_competition_tests_3min.py
```

## What's real, what's a stub, what's missing

Read this before the benchmarks.

| Claim | Status |
|---|---|
| Gemma 9B S13 end-to-end decode, CPU, 42 layers | **Measured.** 0.48 tok/s (AVX2 + Rayon); 0.03 tok/s scalar reference. Slow. Correct. |
| Gemma 9B S13 GEMV on GPU (RTX 3070) | **Kernel measured.** 51.3 passes/s (19.48 ms) through all 42 layers — 168 chained GEMV dispatches per pass, 1.67 GB streamed, parity with the host reference. A pass is the layer GEMVs; nothing else is timed here, so no GPU tokens/sec figure is claimed. |
| Gemma 2B S13 (`s13_gemma_2b_m3`, 446.4 MB) | Quantized from real weights and verified. GPU GEMV timed on real file: 95.0 passes/s (10.52 ms), 104 dispatches per pass. Wired for dual-stream involution checks. |
| Gemma M2 S13 (`s13_gemma_m2`, 765.2 MB) | **Sentry & Routing seat.** 34 layers with layer norms & bundle LoRA; handles protocol guards & sentinel checks. |
| Model count & storage | Three canonical directories on disk (`s13_gemma_9b_m3`, `s13_gemma_2b_m3`, `s13_gemma_m2`), 3,049.6 MB (~3.0 GB) total, driving 5 concurrent execution seats in VRAM via weight sharing. |
| ZPSR −69.6% tokens vs BPE | **Measured on Plains Cree** — GiellaLT `lang-crk` public texts, 2,443 words — over the 65% the strict analyser accepts. Speaker check of morpheme boundaries pending (ALTLab replication). |
| ZPSR "purity" (N·IPR) beats BPE | **Reversed by the data.** v1 figures were constants in a script, not measurements — withdrawn in the v2 erratum. FST segments came out with a *larger* effective vocabulary (89.0) than BPE (39.9 / 52.2). The metric stands; the story didn't. |
| Vertex context caching, 74.2% savings | **Measured.** `docs/RECEIPT-RUN-2026-08-27.txt`; printed live by `vertex_flash_cache.py`. |
| 3-wave airgap + zeroize | Red/green test in repo. The sub-45 ns figure is a timing microbenchmark. |
| Cree example glosses | **Unverified.** Every gloss in the whitepaper is flagged "verify with a speaker." ALTLab replication pending. |

## Numbers my machine printed

Every one of these was true on one x86-64 host with an RTX 3070 on 2026-08-27 and 2026-08-31 (`docs/RECEIPT-RUN-2026-08-27.txt`, `docs/RECEIPT-RUN-2026-08-31.txt`, live `full_inference` runs). A number in a pitch is a claim, not a measurement. Don't trust these — run the suite and trust what yours prints.

| Layer | Result | Mechanism |
|---|---|---|
| 512-bit BQ MetaRouter | 1.76–2.8 M decisions/s, single core (568 ns/decision) | XOR + POPCNT Hamming distance against 7 centroids |
| 5D star projection | 44.45 M stars/s | SO(5) double-plane rotation + Lorentz boost, 119,625 stars |
| 400×400 conjugate grid inversion, scalar | 2.57 Gtrits/s (62.26 µs/pass) | 160 KB, L2-resident |
| 400×400 conjugate grid inversion, AVX2 | 37.06 Gtrits/s (4.32 µs/pass) | `PSHUFB` |
| Double-buffered host staging | 59.62 GB/s, 57.99 M swaps/s (17.25 ns/swap) | 2 × 64 KB ping-pong |
| Tile geometry planning | 358.17 M plans/s (2.79 ns/plan) | Integer ceiling division |
| Gemma 9B GEMV pass, GPU (42 layers, 168 dispatches) | 51.3 passes/s (19.48 ms/pass), 427.4 Gweights/s | `gpu_decode_timed.rs`, 1.67 GB streamed per pass |
| Gemma 2B GEMV pass, GPU (104 dispatches) | 95.0 passes/s (10.52 ms/pass), 192.4 Gweights/s | `gpu_decode_real.rs`, real 404.9 MB weights from disk |
| Gemma 9B end-to-end, CPU AVX2 + Rayon | 0.48 tok/s (2.08 s/tok), 17.4× over scalar | `TRIT_LUT_243` + `_mm256_madd_epi16`, `full_inference.rs` |
| Gemma 9B end-to-end, CPU scalar | 0.03 tok/s (36.3 s/tok) | Single-core reference, no SIMD |

## Architecture

![Nistam Sovereign Architecture Blueprint](patex_fullstack.png)
*PaTeX 5D drafting sheet — Somatic Tokenizer (120 Hz), 16-byte UmpWord SPSC bus, S13 spectral quantization, SplitShader GPU warden, Sovereign Crucible (ASP + FST + GBNF), Gemini 2.5 Flash governor.*

```
  119,625 HYG catalog stars
  SO(5) plane rotations (θ_zw, φ_wv) + Lorentz boost (β)
  44.45 M projected stars/s, zero heap on the hot path
        │
        ├──► Astrolabe resonance multiplier    fixed-point; conjunct 9k / trine 8.5k
        │
        └──► 512-bit BQ MetaRouter             7 Hamming centroids, 3σ gate, 568 ns/decision
                    │
                    ▼
  S13 model directories  (1.58 bit/param, ~3.0 GB on disk, 5 resident execution seats)
    s13_gemma_9b_m3   Gemma 9B Backbone   1,838.0 MB  decodes end-to-end on CPU; GPU GEMV timed
    s13_gemma_2b_m3   Gemma 2B Direct     446.4 MB    verified real weights; GPU GEMV timed; shares weights with mirror
    s13_gemma_m2      Gemma M2 Sentry     765.2 MB    34 layers + norms + LoRA; sentry & protocol guard seat
                    │
                    ▼
  forge-envelope   3-wave filter ─► Hearthkeeper tone ─► ADR-0026 zeroize
                    │
                    │  scrubbed envelope only — no Cree text, no sacred markers
                    ▼
  Vertex AI · Gemini 2.5 Flash · temperature 0.0 · 41,002-token context cache
                    │
                    ▼
  Firestore verdict ─► scrub
```

Native shell: Tauri v2, no Node, no Python server. WebGL2 5D star sky, Three Bears VRAM telemetry, astrolabe volatility dials, ConPTY glass terminal on a lock-free 50,000-line triple buffer.

## The whitepaper

*Little Nistam and The Lattice of Harmony* — `docs/whitepaper/`, DOI 10.5281/zenodo.22176968.

The claim: grammar-constrained tokenization beats BPE on polysynthetic languages, and you can measure why with N·IPR = N·Σpᵢ² instead of Shannon entropy — a dot product, no logarithm, two FMA cycles per token, L1-resident.

The result (2026-08-30): token count and bytes-per-token hold (−69.6%; 6.260 vs 1.905 B/token, 3.29×). The purity hypothesis inverted. The v1 numbers turned out to be constants in a benchmark script, not measurements, and are withdrawn. Chapter VI reports only what `measure_zpsr_vs_bpe.py` computed from the corpus, with SHA-256 receipts for every input and output.

That erratum is the most important paragraph in the paper. Next: replicate on a speaker-verified ALTLab corpus with gold morpheme boundaries, then restate the hypotheses against what the metric actually does.

## Quickstart

```bash
# 0. S13 weights (2B + 9B) from Hugging Face Hub — required before any inference example
python scripts/fetch_demo_weights.py

# 1. Native Tauri shell
cargo run --manifest-path crates/studio-tauri/Cargo.toml

# 2. Full workspace tests
cargo test --workspace

# 3. S13 ternary + WebGPU compute tests
cargo test --manifest-path crates/gemma-s13/Cargo.toml

# 4. 3-wave sovereign airgap, red/green
python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py

# 5. Vertex AI governor + strict context-cache verification (>= 32k-token census)
python scripts/test_vertex_cache_strict.py
```

## Layout

```
.
├── Cargo.toml                          # Workspace manifest
├── README.md
├── carts/                              # Authored RON cartridges (ironroot, base, weaver_arbiter)
├── crates/
│   ├── gemma-s13/                      # S13 ternary engine, 5D astrolabe, fleet budget (9B / 2B / M2 sentry)
│   ├── forge-envelope/                 # Sovereign vault, Hearthkeeper, 3-wave filter, ADR-0026 zeroize
│   ├── forge-ml-bqrouter/              # 512-bit BQ centroid router
│   ├── forge-core-v3/                  # UmpWord, triad arithmetic, trit LUT, celestial types
│   ├── forge-gpu-warden-v3/            # SplitShader WebGPU compute + timeline staging
│   ├── forge-daemon-door/              # MMA-over-Nostr protocol, BIP-340 Schnorr gates
│   ├── forge-cart-v3/                  # RON cartridge parser, bake engine, Arbiter judge
│   ├── forge-tui-v3/                   # ConPTY terminal engine, 50,000-line scrollback
│   └── studio-tauri/                   # Tauri v2 native shell
├── shell/assets/hyg_baked.bin          # 119,625-star HYG catalog, compile-time embedded
├── docs/
│   ├── DEVPOST.md                      # Submission narrative
│   ├── SUBMISSION_ENTRY.md             # Entry form and disclosures
│   ├── JUDGE-BUILD.md                  # Build and verification instructions for judges
│   ├── RECEIPT-RUN-2026-08-27.txt      # Measured benchmark receipts
│   ├── RECEIPT-RUN-2026-08-31.txt      # GPU GEMV run, raw stdout
│   ├── whitepaper/                     # Little Nistam and The Lattice of Harmony (.tex, .pdf)
│   └── patex_fullstack.png             # PaTeX drafting sheet
└── scripts/
    ├── fetch_demo_weights.py           # Download S13 Gemma weights
    ├── demo_cloud_agent.ps1            # 1-click live cloud run for judges
    ├── vertex_flash_cache.py           # Vertex AI context-cached call with cost receipt
    ├── run_competition_tests_3min.py   # 3-minute master suite
    ├── test_vertex_cache_strict.py     # Token census (>= 32k) + strict cache verification
    └── deploy_vertex_cloudrun.ps1      # Optional one-time GCP provisioning
```

## License & attribution

Apache-2.0 or MIT, at your option (`LICENSE-APACHE`, `LICENSE`).
Whitepaper: [10.5281/zenodo.22176968](https://doi.org/10.5281/zenodo.22176968). Earlier mathematical research: [10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676).
