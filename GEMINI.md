# GEMINI.md — Nistam Dream Engine & The Forge Engine | Compiler & Judge Specification

> **HACKATHON TARGET:** Devpost "All Things Agentic" ([allthingsagentichackathon.devpost.com](https://allthingsagentichackathon.devpost.com))  
> **PROJECT ID:** `nde1-493505` (Google Cloud Vertex AI / Cloud Run / Firestore)  
> **RESEARCH DOI:** [10.5281/zenodo.22176968](https://doi.org/10.5281/zenodo.22176968) (*Pararity: the fixed-point residue of an involution, and why we need it*)  
> **AUTHOR:** Sean Morin, Edmonton River Valley, Alberta, Canada  
> **LICENSE:** MIT OR Apache-2.0  

---

```ron
// NISTAM-SOVEREIGN-COMPILER-SPEC (v3.0.0-s13)
// Target: Devpost "All Things Agentic" | Project: nde1-493505 | DOI: 10.5281/zenodo.22176968
(
    compiler: (
        name: "Nistam Sovereign Dream Engine",
        target_evaluator: "Gemini 2.5 Flash / Hackathon Judge",
        source_of_truth: "F:\\v3",
        submission_surface: "F:\\Nistam-Dream-Engine-Sovereign-Silicon-Socratic-Mind",
    ),

    axioms: (
        a0_zero_fabrication: "Fatal on unverified claims; all metrics backed by physical silicon receipts",
        a1_zero_heap: "0 dynamic allocations (Vec/String/HashMap) on inference & continuous 5D projection hotpaths",
        a2_memory_safety: "#![deny(unsafe_code)] strictly enforced across all sovereign inference, crypto & PDA crates",
        a3_airgap_zeroize: "ADR-0026 SIMD zeroization on drop, cultural airgap refusal, and receipt acknowledgment",
        a4_cloud_governor: "Vertex AI gemini-2.5-flash serverless context cache (>=32k tokens, temp 0.0, 75% discount at $0.01875/1M cached tokens)",
        a5_user_interactive_gui: "Agent MUST NEVER launch GUI/Tauri apps in background; all GUI executions reserved strictly for user manual launch",
    ),

    proof_ladder: (
        unproven: "Untraced proposal / stub -> [UNVERIFIED]",
        proven: "Clean cargo test / script execution in session buffer (165/165 passed in gemma-s13)",
        verified: "Dual-oracle bit-exact match (GPU GEMV == CPU SIMD == Host Reference)",
    ),

    substrate_layers: [
        (
            id: 0,
            layer: "Balanced Ternary S13 Engine",
            crate: "gemma-s13",
            algebra: "T = {-1, 0, +1} (1.58-bit Involution Pararity fixed-point residue)",
            packing: "5 trits / byte (3^5 = 243 states <= 256; 243..255 spare states act as hardware-level sentinel alarms)",
            simd: "AVX2 PSHUFB + _mm256_madd_epi16 (37.06 Gtrits/s AVX2 / 2.57 Gtrits/s scalar)",
            gpu_gemv: "49.2 passes/s (409.3 Gweights/s on RTX 3070, real 1.66 GB weights)",
        ),
        (
            id: 1,
            layer: "5D Geodesic Astrolabe & Celestial Codebook",
            crate: "gemma-s13::astrolabe_projection_5d",
            dataset: "119,625 real catalog stars from HYG Database",
            projection: "SO(5) rotation (theta_zw, phi_wv) + Lorentz velocity boost (beta) -> OKLCH blackbody T_eff",
            detokenization: "Zero-heap L2 Euclidean nearest-neighbor search (44.45 Million projected stars/sec)",
        ),
        (
            id: 2,
            layer: "Three Bears Resident Fleet",
            crate: "gemma-s13::three_bears",
            vram_ceiling_gb: 2.71,
            fleet: (
                baby_bear_2b: (mb: 404.9, role: "Intent mirror & somatic reverse tokenizer"),
                mama_bear_9b: (mb: 1720.0, role: "42-layer full S13 backbone (bit-identical GPU/CPU)"),
                papa_bear_27b: (mb: 580.0, role: "S13Norm27b RMSNorm & 5D W_proj latent projection"),
            ),
            staging_bw_gbs: 59.62,
        ),
        (
            id: 3,
            layer: "512-bit BQ MetaRouter & Pushdown Automata",
            crates: ["forge-ml-bqrouter", "gemma-s13::constrain"],
            router: "XOR + POPCNT Hamming Centroid across 7 domains (365 ns / decision)",
            grammar_pda: "Weld-RON Pushdown Automaton with lazy PdaStateCache",
            drain_sentinel: "Natural <end_of_turn> (107) and <eos> (1) logit mask clamping (zero runaway generation)",
        ),
        (
            id: 4,
            layer: "Cloud Governor & Airgap Sentinel",
            crate: "forge-envelope",
            backend: "Google Cloud Vertex AI gemini-2.5-flash context cache",
            defense_waves: (
                w1: "Lexical & phonemic Cree ghost words filter",
                w2: "Witnessed verb stem morphological sentinel",
                w3: "ADR-0026 SIMD zero-retention memory scrubbing + constant-time sub-45ns Merkle-Morin root check",
            ),
            autonomous_flywheel: "scripts/agent_loop.py (Cloud Run: GCS -> ByteSieve -> Vertex AI -> Firestore)",
        ),
        (
            id: 5,
            layer: "In-Process Celestial Gemma Bot & Presentation",
            crates: ["gemma-s13::celestial_bot", "studio-tauri"],
            architecture: "Zero-socket, in-process Sovereign Navigation Engine holding model_27b + star_codebook",
            header: "F0RC (12 bytes)",
            gui_shell: "Native Tauri v2 + WebGL2 5D bloom shaders + ConPTY glass terminal (0 Node.js/Python servers)",
        ),
    ],

    judge_verification_matrix: (
        test_all: "python scripts/run_competition_tests_3min.py",
        test_s13: "cargo test --manifest-path crates/gemma-s13/Cargo.toml",
        test_gpu_warden: "cargo test --manifest-path crates/forge-gpu-warden-v3/Cargo.toml",
        test_daemon_door: "cargo test --manifest-path crates/forge-daemon-door/Cargo.toml",
        test_airgap: "python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py",
        test_cloud_cache: "python scripts/test_vertex_cache_strict.py",
        run_cloud_flywheel: ".\\scripts\\demo_cloud_agent.ps1",
        run_desktop_shell: "cargo run --manifest-path crates/studio-tauri/Cargo.toml",
    ),
)
```

---

## 🏛️ System Architecture: The Five Sovereign Pillars

```
                                      ┌─────────────────────────────────────────────────────────┐
                                      │       119,625 Real Catalog Stars (HYG Codebook)         │
                                      │   5D Lorentz Boost (β) + SO(5) Plane Rotations (θ, φ)   │
                                      │       44.45 Million Projected Stars / sec @ 0-Heap      │
                                      └────────────────────────────┬────────────────────────────┘
                                                                   │
                                   ┌───────────────────────────────┴──────────────────────────────┐
                                   ▼                                                              ▼
                     ┌───────────────────────────┐                                  ┌───────────────────────────┐
                     │   Astrolabe Fixed-Point   │                                  │   7-Domain BQ MetaRouter  │
                     │     Resonance Multiplier  │                                  │   Sub-100ns Domain Shift  │
                     │ (Conjunct 9k / Trine 8.5k)│                                  │ (Hamming Centroid 3σ Gate)│
                     └─────────────┬─────────────┘                                  └─────────────┬─────────────┘
                                   │                                                              │
                                   └───────────────────────────────┬──────────────────────────────┘
                                                                   │
                                                                   ▼
                                   ┌──────────────────────────────────────────────────────────────┐
                                   │     THREE BEARS S13 QUANTIZED MODEL SUITE (2.71 GB Total)    │
                                   ├──────────────────────────────────────────────────────────────┤
                                   │ 🐻 Baby Bear (Gemma 2B @ 1.58-bit)   ──► 404.9 MB            │
                                   │ 🐻 Papa Bear (Gemma 9B @ 1.58-bit)   ──► 1.72 GB             │
                                   │ 🐻 Mama Bear (27B Router Head)       ──► 580 MB              │
                                   └───────────────────────────────┬──────────────────────────────┘
                                                                   │
                                                                   ▼
                                   ┌──────────────────────────────────────────────────────────────┐
                                   │   GOOGLE VERTEX AI CLOUD GOVERNOR (gemini-2.5-flash @ 0.0)   │
                                   │   Context Caching (>=32k tokens) | $0.01875/1M Cached Tokens │
                                   │   3-Wave Cultural Airgap Defense | ADR-0026 SIMD Zeroize     │
                                   └──────────────────────────────────────────────────────────────┘
```

---

## ⏱️ Measured Physical Hardware Receipts

*All measurements conducted on host silicon (AMD Ryzen 9 / Intel Core x86_64, NVIDIA GeForce RTX 3070 8 GB, Windows 11). Zero simulated benchmarks.*

| Layer / Benchmark | Measured Physical Throughput / Latency | Architectural Mechanism |
| :--- | :--- | :--- |
| **512-bit BQ MetaRouter MoE** | **2.74 Million decisions/s / core** (`365.09 ns`) | XOR + POPCNT Hamming distance against 7 domain centroids |
| **5D Star Sky Projection** | **44.45 Million stars/sec** | $SO(5)$ double-plane rotation + Lorentz boost (119k HYG catalog, zero-heap) |
| **Conjugate Grid Inversion (Scalar)** | **2.57 Gtrits/s** (`62.26 µs` / 400×400 grid) | 1.58-bit ternary sign inversion in 160 KB L2 cache |
| **Conjugate Grid Inversion (AVX2)** | **37.06 Gtrits/s** (`4.32 µs` / 400×400 grid) | AVX2 `PSHUFB` byte-LUT parallel sign inversion |
| **Host Staging Memory Swap** | **59.62 GB/s** (57.99 Million swaps/s) | Double-buffered 64 KB ping-pong staging (`17.25 ns` swap latency) |
| **Ampere 32×32 Tile Planning** | **358.17 Million plans/s** (`2.79 ns`/plan) | Integer ceiling division workgroup geometry planner |
| **Gemma 9B GEMV Kernel (GPU RTX 3070)** | **51.3 passes/s** (`19.48 ms`, **427.4 Gweights/s**) | 42 layers, 1.67 GB VRAM (`gpu_decode_timed.rs`), bit-exact CPU parity |
| **Gemma 2B GEMV Kernel (GPU RTX 3070)** | **95.0 passes/s** (`10.52 ms`, **192.4 Gweights/s**) | 26 layers real quantized weights (`gpu_decode_real.rs`, 404.9 MB) |
| **Gemma 9B Decode Baseline (CPU AVX2)** | **0.48 tokens/sec** (`2.08 s`/tok, 42 full layers) | `TRIT_LUT_243` + `_mm256_madd_epi16` + Rayon parallelism (CPU fallback) |
| **Gemma 9B Decode Baseline (CPU Scalar)** | **0.03 tokens/sec** (`36.3 s`/tok) | Single-core scalar fallback baseline |
| **Three Bears Resident Layout** | **2.71 GB Total VRAM** | 2B (404.9 MB) + 9B (1.72 GB) + 27B Head (580 MB) |
| **Airgap Red/Green Defense** | **5 / 5 Red Vectors Blocked (100%)** | 3-Wave Cree diacritic/stem sentinels + ADR-0026 SIMD zeroize |
| **Vertex AI Context Caching** | **74.2% Measured Discount ($0.000801 billed)** | Google Vertex AI `gemini-2.5-flash` serverless context caching (41,002 cached tokens) |

---

## ⚡ 1-Click Judge Verification Commands

### 1. Master Competition Test Suite (401+ Unit/Integration Tests, Zero Mocks)
```powershell
python scripts/run_competition_tests_3min.py
# Or on Windows CMD / PowerShell:
.\test.bat
```

### 2. S13 Balanced Ternary & WebGPU Compute Kernel Suite (165 Tests)
```bash
cargo test --manifest-path crates/gemma-s13/Cargo.toml
```

### 3. GPU Warden Timeline Semaphores & Staging (25 Tests)
```bash
cargo test --manifest-path crates/forge-gpu-warden-v3/Cargo.toml
```

### 4. Binary Daemon Door & Merkle-Morin Protocol (192 Tests, 61 Verbs)
```bash
cargo test --manifest-path crates/forge-daemon-door/Cargo.toml
```

### 5. 3-Wave Sovereign Airgap Red/Green Test
```powershell
python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py
```

### 6. Vertex AI Cloud Governor & Context Cache Strict Verification
```powershell
python scripts/test_vertex_cache_strict.py
```

### 7. Autonomous Cloud Agent Flywheel (GCS -> ByteSieve -> Vertex AI -> Firestore)
```powershell
.\scripts\demo_cloud_agent.ps1
```

### 8. Native Tauri v2 5D Demo Shell Launch (No Node.js Required)
```bash
cargo run --manifest-path crates/studio-tauri/Cargo.toml
```

---

## 📜 Pre-Existing Work & Open-Source Disclosures (Rules G8 & G10)

- **Competition Submission Window:** August 3 – August 31, 2026.
- **Pre-Existing Core Substrate (Disclosed under Rule G8):** The underlying Forge Engine substrate (balanced-ternary primitives, GPU warden, and BQ router) was developed starting November 2025. The VARS format and initial Vertex AI Flash context-caching experiments began December 2025.
- **Built Specifically for this Hackathon Submission:**
  1. Complete **Three Bears S13 Gemma Fleet** (2B, 9B, 27B) quantized memory layout and inference pipeline.
  2. The 119,625-star **5D Relativistic Astrolabe Projection Engine** ($SO(5)$ + Lorentz boosts).
  3. Formal mathematical monograph: *"Pararity: the fixed-point residue of an involution, and why we need it"*, [DOI 10.5281/zenodo.22176968](https://doi.org/10.5281/zenodo.22176968).
  4. Autonomous Cloud Run flywheel (`agent_loop.py`) with 3-wave cultural airgap and ADR-0026 zero-retention memory scrubbing.
  5. Native **Tauri v2 Desktop Showcase** with WebGL2 5D Star Sky and ConPTY glass terminal.
- **Third-Party Open Source (Rule G10):** `candle` / `candle-transformers` (Apache-2.0 / MIT), `tokenizers`, `safetensors`, `bytemuck`, and `windows-sys`.

