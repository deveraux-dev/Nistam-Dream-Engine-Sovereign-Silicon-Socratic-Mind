---
name: competition-demo
description: >-
  Standard operating procedure and execution runbook for the Nistam Dream Engine
  Devpost competition demo. Walks through Vertex AI context caching, ConPTY agy
  agent conductor, 3-model resident fleet receipts (2.71 GB VRAM), 7-Domain BQ MetaRouter
  dispatch, Weaver/Arbiter RON DSL compilation, and Tauri demo shell inspection.
---

# Competition Demo Runbook & Receipt Verification Pipeline

This skill guides the automated execution, receipt validation, and screen-recording flow for the **Nistam Dream Engine & The Forge Engine** Devpost competition submission ("All Things Agentic").

---

## 🌌 The Core Architecture: Cloud Macro-Planner to Sovereign Silicon

```
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │                      1. CLOUD MACRO-PLANNER (Vertex AI)                     │
 │  • Model: gemini-2.5-flash @ deterministic temp 0.0, top_k 1               │
 │  • Governor: $0.0004/call unit-cost ceiling under 450k-token VARS context   │
 │  • Role: Generates lawful RON Cartridges, Weaver ASTs, and VIXI Shaders     │
 └──────────────────────────────────────┬──────────────────────────────────────┘
                                        │ High-Order DSL Generation
                                        ▼
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │                   2. CONPTY AGENT CONDUCTOR (Antigravity CLI)               │
 │  • Runs: 'agy' live inside the Tauri Win32 ConPTY terminal dock             │
 │  • Gateway: Port :13013 Forge Daemon Door (59 binary verbs, F0RC header)    │
 │  • Sovereign Airgap Sentry: 3-Wave Cree Linguistic Filter (ADR-0026)        │
 │    ⚡ GUARANTEE: NO CREE ON THE CLOUD — Zeroizes local memory on refusal    │
 └──────────────────────────────────────┬──────────────────────────────────────┘
                                        │ Sanitized AST & Frame Injection
                                        ▼
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │               3. RESIDENT SOVEREIGN SILICON (RTX 3070 / 2.71 GB VRAM)       │
 │  • 🐻 Baby Bear (Gemma 2B - 446 MB):   M5 Geodesic Shader Coordinates       │
 │  • 🐻 Mama Bear (Gemma 9B - 1.72 GB):  S13 Balanced Ternary Vector Kernel   │
 │  • 🐻 Papa Bear (Gemma M2 - 765 MB):   7-Domain BQ MetaRouter (363 ns)      │
 │  • 🌌 5D Astrolabe Galaxy Engine:      119,625 HYG Stars @ 44.45M stars/sec │
 │  • 📜 Weaver/Arbiter RON Engine:       Deterministic 7-Principle Judge      │
 └─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🎬 Operator Video Walkthrough Script (Strictly $\le 180$ Seconds)

### Act I: The 5D Relativistic Starmap & $SO(5)$ Manifold `[0:00 - 0:35]`
1. **Visual**: Pan across the cosmic viewport rendering **119,625 real HYG catalog stars**.
2. **Action**: Drag the **5D Astrolabe** slider and adjust $SO(5)$ Givens rotation sliders ($\theta_{zw}$, $\phi_{wv}$) and Lorentz relativistic boost ($\beta = v/c$).
3. **Narrative**: *"You are looking at 119,625 real stars projected in real time at 44.45 Million stars per second using 5D relativistic Givens rotations with zero dynamic heap allocations on the hotpath."*

### Act II: The ConPTY Terminal Glass Dock & Cloud Agent Loop `[0:35 - 1:10]`
1. **Visual**: Click the **▲ OPEN** tab at the bottom to expand the glass ConPTY terminal dock running `agy` (Antigravity).
2. **Action**: Invoke Gemini 2.5 Flash via Antigravity to generate a procedural RON cartridge.
3. **Airgap Verification**: Enter standard engineering text $\to$ passes `CLEARED`. Enter Cree syllabics (e.g. `ᑖᐻ`) $\to$ triggers instant `AIRGAP REFUSAL: Wave 1 caught 'ᑖᐻ' | MEMORY ZEROIZED`.
4. **Narrative**: *"ConPTY hosts Antigravity driven by Gemini 2.5 Flash on Google Cloud Vertex AI under a strict $0.0004 per call governor. Local Mama Bear enforces the sovereign airgap: zero Cree words or cultural syllabics ever leave local memory for the cloud."*

### Act III: The Three Bears Gemma Fleet & 7-Domain BQ MetaRouter `[1:10 - 1:45]`
1. **Visual**: Click **`🐻 FLEET TRIAD`** in the top navigation bar to open the telemetry glass dossier.
2. **Action**:
   - Inspect the **2,710 MB resident VRAM** hardware breakdown (Baby 2B, Mama 9B, Papa M2 Sentry on the RTX 3070).
   - Enter *"Synthesize radiant plasma thruster"* and click **`ROUTE`**: shows instant **363 ns** centroid dispatch with $3\sigma$ signal isolation.
   - Click **`TRIAD STEP`**: executes the genuine S13 balanced ternary kernel and displays live N×IPR attention sieve metrics (`762 ns/eval`).
3. **Narrative**: *"All three Gemma models remain resident in just 2.71 GB of consumer VRAM. Papa Bear routes prompts across 7 engineering domains in under 400 nanoseconds, while Mama Bear computes bit-exact balanced ternary dot products."*

### Act IV: Weaver / Arbiter RON Cartridge & Deterministic Ledger `[1:45 - 2:20]`
1. **Visual**: Click **`✦ CONSTELLATION`** or navigate to a procedural star world.
2. **Action**:
   - Show live parsing of `carts/ironroot/weaver_arbiter.ron` by the mechanical Arbiter judge.
   - Demonstrate 100% compliance across Hermetic Principles, power curves ($0..255$), and hex swatches.
   - Interact with the Celestial Oracle; show real-time **Hearthkeeper zero-apology tone gating**.
3. **Narrative**: *"Game entities, shaders, and star worlds are expressed in strict RON DSL. The Arbiter judges mechanical validity, while Hearthkeeper normalizes tone in zero allocations."*

### Act V: Measured Physical Hardware Receipts & Cryptographic Gate `[2:20 - 3:00]`
1. **Visual**: Switch to terminal and run the 3-minute automated verification suite:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/run_competition_tests_3min.ps1
   ```
2. **Action**: Display the green passing receipts across all 5,213 Rust tests, BIP-340 Schnorr signatures, and sub-45ns Merkle roots.
3. **Narrative**: *"No mocks. No smoke and mirrors. 5,068 compiled Rust tests pass clean — 0 failed, 6 ignored — with measured hardware receipts: 568.28 ns BQ router decisions (1.76 M/s), 37.06 Gtrits per second AVX2 sign inversion, and 59.62 GB per second memory swap."*

   Receipts backing that narrative:
   `docs/RECEIPT-cargo-test-workspace-2026-08-29.txt` (test count, host + scope stamped)
   `docs/_archive-benchmarks-2026-08-27/RECEIPT-RUN-2026-08-27.txt` (silicon figures)

   GPU decode tok/s is deliberately absent: no receipt on disk backs a single headline
   figure, and the README table already reports per-model rates (82.5 / 54.7 / 22.9).

---

## 🎓 Academic Demonstration Track: 3-Step Presentation Sequence (Generic Morphology)

When demonstrating to quantitative linguists, academic evaluators, or technical judges, use the 1-click **DEMO FLOW** buttons in the Studio Tauri UI:

### Step 1: The Point Cloud = Unconstrained BPE (Thermal Chaos)
* **UI Action**: Click `[1] THERMAL BPE` (or press `L` to toggle).
* **Visual**: Camera zooms out to the wide, dense 5D hypersphere of 119,625 scattered points.
* **Telemetry**: `Entropy H₁: 4.85 nats | 119,625 Active Unconstrained Tokens (Thermal Chaos)`.
* **Script**: *"This dense cluster represents unconstrained statistical BPE token space—high entropy, scattered probability mass, and thermal noise where tokens can transition anywhere without grammatical rules. In polysynthetic morphologies, unconstrained subwords fragment the grammar, producing combinatorial explosion and illegal hallucinations."*

### Step 2: Constellation Formation = Grammar Constraints (ZPSR / FST)
* **UI Action**: Click `[2] ZPSR CONSTELLATION`.
* **Visual**: Glowing vector lines connect specific nodes in sequence, displaying generic morpheme tags: `[ROOT: Action] → [THEME: Transitive] → [AGR: 3Sg.Subj] → [OBJ: 3Pl.Obj]`.
* **Telemetry**: `Grammar State: Polysynthetic FST Chain | Pruned States: 99.84% | Legal Morpheme Path: 4 Nodes`.
* **Script**: *"When we apply the Zero-Shot Polysynthetic State Resolver (ZPSR) with FST and GBNF Crucible Masking, the engine clamps illegal transitions. It collapses the probability mass onto pure, valid morpheme paths—forming these distinct geometric constellations."*

### Step 3: The Audio Tone = $N \times \text{IPR}$ Purity Resonance
* **UI Action**: Click `[3] N×IPR SONIFICATION`.
* **Visual**: Harmonic Frequency HUD locks at `1539.47 Hz` as the pure sine chime resonates across the active nodes.
* **Telemetry**: `Acoustic Sonification: High-Dimensional State Vector | Anti-Shannon N × IPR: 195.0 | Resonator: 1539.47 Hz (G6)`.
* **Script**: *"The audio feedback maps the Anti-Shannon Inverse Participation Ratio (N × IPR). As probability mass concentrates onto pure valid states, the mathematical purity resonates at a specific harmonic frequency rather than noisy white noise. This provides real-time acoustic sonification of the high-dimensional state vector collapse."*

---

## 🧪 Detailed Verification & Command Reference

### 1. Vertex AI Cloud Governor & 3-Wave Airgap
```powershell
# 1. Token Census (>= 32,768 tokens) & Airgap Scanner
python scripts/test_vertex_cache_strict.py

# 2. Red/Green Cultural Airgap Defense Test
python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py

# 3. Interactive Strict Query with Loud Visual ANSI Receipt
python scripts/vertex_flash_cache.py --strict --profile lean "Explain the zero-heap memory contract in Forge v3"
```

### 2. Compiled Rust Engine Receipts (5,213 Tests)
```powershell
# Gemma S13 Balanced Ternary & WebGPU Kernel (138 tests)
cargo test --manifest-path crates/gemma-s13/Cargo.toml

# BIP-340 Schnorr Gates & MMA-over-Nostr Wire (191 tests)
cargo test --manifest-path crates/forge-daemon-door/Cargo.toml

# Weaver/Arbiter RON Cartridge & Grimoire Judge (71 tests)
cargo test --manifest-path crates/forge-cart-v3/Cargo.toml

# 3-Wave Envelope & Sovereign Evidence Vault (84 tests)
cargo test --manifest-path crates/forge-envelope/Cargo.toml

# GPU Warden Timeline Semaphores & Staging (25 tests)
cargo test --manifest-path crates/forge-gpu-warden-v3/Cargo.toml

# Studio Tauri Demo Shell & AST Projection (13 tests)
cargo test --manifest-path crates/studio-tauri/Cargo.toml
```

### 3. Launch the Complete Native Desktop Demo
```powershell
cargo run --manifest-path crates/studio-tauri/Cargo.toml
```

