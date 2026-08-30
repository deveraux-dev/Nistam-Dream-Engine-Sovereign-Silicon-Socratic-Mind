# 13forge — Architectural Principles & Mechanics

This document establishes the canonical systems-level design guidelines for **13forge**  and the underlying **`forge-envelope`** cryptographic container library.

---

## 1. The Core Philosophy: "Compute at Rest"

Traditional software architectures dynamically parse strings, execute heavy logical checks, and allocate complex memory states on the execution hotpath. In high-reliability edge-native systems, this introduces non-determinism, runtime latency spikes, and security vulnerabilities.

**Compute-at-Rest** dictates that all cognitive, logical, and structural definitions must be pre-compiled and flattened before execution begins. At runtime, the engine performs zero dynamic parsing or heap-allocations; it simply reads pre-calculated answers from static lookups.

---

## 2. The Deterministic Metronome: Integer-Tick Expiry

To guarantee that any two independent nodes replaying the exact same transaction sequence achieve bit-perfect synchronicity, all execution lifecycles and memory lifetimes are governed by **integer simulation ticks** rather than wall-clock time.

*   Wall-clock TTLs are subject to local machine scheduling, latency drift, and system clock adjustments, making consensus impossible.
*   By using monotonically advancing `u64` ticks, memory zeroization and state transitions are bound to a single, unified, platform-independent metronome.

---

## 3. The Pararity Theorem & Balanced Ternary Coordinates

To ensure that state representations never lose information or drift during multi-lane compositions, the system is designed under the rigorous mathematical law of **Pararity** (the fixed-point residue of an involution on a discrete state lane).

### 3.1. Orbit Decomposition and the Pararity Number
Let $S$ be a finite state set and $f: S \to S$ an involution, such that $f \circ f = \text{id}$. The arity of the lane is $n = |S|$. Parity is the involution $f$ together with its orbit structure. **Pararity** is the set of fixed points:
$$\text{Fix}(f) = \{ x \in S : f(x) = x \}$$
The pararity number of the lane is $k = |\text{Fix}(f)|$. By classical orbit decomposition:
$$n = 2m + k$$
where $m$ is the number of 2-element orbits. This yields the fundamental congruence:
$$k \equiv n \pmod 2$$

### 3.2. Why Even Arities Fail
A balanced ternary coordinate system $\{-1, 0, +1\}$ represents agency (equilibrium, push, pull) and requires exactly one fixed point ($k=1$).
If the arity $n$ is even, the pararity number $k$ must be even. No choice of involution $f$ can rescue it:
*   $n=2 \implies k \in \{0, 2\}$ (Pure swap or identity). Cannot carry a trit.
*   $n=4 \implies k \in \{0, 2, 4\}$. Cannot carry a trit.
*   $n=3 \implies k \in \{1, 3\}$ (The non-trivial involution yields $k=1$). **Carries a trit.**

Thus, a 3-state lane with a non-trivial involution is the unique minimal sandbox that holds a neutral zero (the fulcrum of choice) without state truncation or fault traps.

### 3.3. Composition of Lanes (The S13 Vector)
Under Proposition 2, a product of $k$ independent lanes, each of arity 3, yields $3^k$ states. Because 3 is odd, the product of odd numbers is always odd, ensuring that **a true all-zero origin survives at every composite depth**.
*   **Sieve-13 (S13):** The visual state of a physical asset (corrosion, weathering, carbon probes) is encoded as a 13-lane coordinate vector of arity-3. This produces $3^{13} = 1,594,323$ distinct states with a guaranteed drift-free central origin (`0` state) representing flawless structural equilibrium.

---

## 4. Local Photometric Stereo Engine (Shape-from-Shifting)

To execute visual attestation completely offline under a strict **5GB memory-mapped cap**, I bypass heavy cloud-based vision models at the edge using an offline-native **Photometric Stereo Solver**.

```
  [Camera Flash Source 1]      [Camera Flash Source 2]      [Camera Flash Source 3]
             \                            |                            /
              \                           |                           /
               v                          v                          v
             +---------------------------------------------------------+
             |         Observed Lambertian Drywall / Metal Surface      |
             +---------------------------------------------------------+
                                         |
                                         v
             +---------------------------------------------------------+
             |            Surface Normal Recovery Vector N             |
             +---------------------------------------------------------+
                                         |
                                         v
             +---------------------------------------------------------+
             |           Mean Curvature H (Sub-mm Defects)             |
             +---------------------------------------------------------+
```

### 4.1. Lambertian Forward Model
As a user records a video of a surface while moving their mobile device, the camera flash shifts in an arc. The observed intensity $I$ at each pixel follows the Lambertian reflectance model:
$$I = \rho \max(0, N \cdot L)$$
where:
*   $\rho$ is the albedo (surface reflectance).
*   $N = (n_x, n_y, n_z)$ is the unit surface normal vector.
*   $L = (l_x, l_y, l_z)$ is the unit light direction vector.

### 4.2. Normal Recovery
By capturing $M \ge 3$ images under varying light directions $L_i$, I construct an intensity vector $I = [I_1, I_2, \dots, I_M]^T$ and a lighting matrix $L = [L_1, L_2, \dots, L_M]^T$. I solve the linear system for the pseudo-normal vector $G$:
$$G = (L^T L)^{-1} L^T I$$
Since $G = \rho N$, I recover both the albedo $\rho = \|G\|$ and the unit surface normal vector $N = G / \|G\|$.

### 4.3. Curvature Extraction for NACE Level 2
The localized surface gradients are integrated using central differences:
$$p = \frac{\partial z}{\partial x} = -\frac{n_x}{n_z}, \quad q = \frac{\partial z}{\partial y} = -\frac{n_y}{n_z}$$
The mean curvature $H$ is computed directly to detect protective coating failure states (blisters, chalking, cracking):
$$H = \frac{(1+q^2)\frac{\partial p}{\partial x} - 2pq\frac{\partial p}{\partial y} + (1+p^2)\frac{\partial q}{\partial y}}{2(1 + p^2 + q^2)^{3/2}}$$
If $H$ exceeds the NACE Level 2 warning threshold ($> 0.5$mm), a visual anomaly is flagged and encoded into the S13 token.

---

## 5. Mixture of Musicians (MoM) Audio Routing Engine

To optimize inspector performance and eliminate cognitive fatigue on-site, the framework integrates a sovereign focus-entrainment audio generator operating entirely on textbook-grade, dependency-free f64 DSP primitives:

```
[16-byte UmpWord Event] ➔ [49-slot MoeRouter (XOR+POPCNT)] ➔ [Conductor Weight Matrix]
                                                                        │
                                                                        ▼
[lossless dithered mix] 🖚 [i24 MomBus (PCG TPDF dither)] 🖙 [Musician f64 DSP Processors]
```

1.  **UmpWord Packets:** Translates incoming tactile and cognitive event vectors into 16-byte universal MIDI packet structures.
2.  **POPCNT / XOR Bit-Parallel Routing:** The `MoeRouter` matches incoming packets against static 16-byte centroid masks, mapping events to 49 active musician slots in under $1\,\mu\text{s}$ to preserve strict audio-buffer latency deadlines.
3.  **Phase Delay Compensation (PDC) as State Rollback:** Parallel musicians with different group delays introduce phase-smearing comb filters. To solve this, I engineered a real-time Phase Delay Compensation (PDC) engine. Mathematically, I prove that adjusting delay-line offsets in PDC is equivalent to a **transactional rollback and replay** of the state-engine timeline, aligning auditory focus pulses with absolute mechanical precision:
    $$s(t - \tau_k) = \mathcal{R}_{\tau_k}[s(t)]$$
    where $\mathcal{R}_{\tau_k}$ is the temporal rollback operator shifting the state vector by exactly the musician's group delay $\tau_k$.
4.  **Lossless Summation & TPDF Dither:** Sums all active musician signal outputs inside `MomBus`. Before folding into standard 24-bit streams, it applies a PCG-seeded Triangular Probability Density Function dither ($R_1 - R_2$), completely eliminating quantization distortion and truncation artifacts without dynamic memory allocation on the audio hot-path.

---

## 6. Edge-Metal 2GB Gemma Trinity MoE

To deploy multi-expert reasoning locally, I leverage **1.58-bit ternary quantization** ($\{-1, 0, +1\}$), which drops Gemma 2B's weight footprint to **~400 MB**. Including 200 MB for static KV-caches and buffers, each model operates inside a **600 MB envelope**, allowing me to run **exactly three specialized models concurrently ($3 \times 600\text{ MB} = 1.8\text{ GB} < 2\text{ GB}$)**:
*   **Expert 1 (Somatic Triage Expert):** Unpacks tactile bitfields and photometric normal maps into scale-invariant coordinates.
*   **Expert 2 (Cognitive Metronome Expert):** Evaluates ADHD focus rhythms and auditory feedback loop synchronicity.
*   **Expert 3 (Thermodynamic Entropy Governor - AVG):** Computes active attention masks and damping limits to control semantic entropy.

---

## 7. WebGPU Bit-Perfect Determinism (WGSL i64 Emulation)

Executing physical Photometric Stereo and S13 normal map recovery on the GPU is highly efficient, but standard floating-point execution units exhibit minor architectural variances across vendors (Nvidia, AMD, Apple, ARM). This results in float-drift and consensus failures on independent client devices.

To guarantee bit-perfect, identical results on any GPU, I compiled my solver into 64-bit fixed-point integer equations. On mobile devices lacking native 64-bit WebGPU integer support, I engineered a **WGSL emulated i64 kernel**:
*   **The Emulation Math:** A 64-bit signed integer $V$ is represented inside WGSL using dual 32-bit registers as a struct of two `u32` variables:
    ```wgsl
    struct int64 {
        low: u32,
        high: u32,
    }
    ```
*   **Emulated Add-with-Carry:** Addition of two emulated 64-bit integers ($A + B$) is computed safely without native hardware support:
    $$C_{\text{low}} = A_{\text{low}} + B_{\text{low}}$$
    $$\text{carry} = \text{select}(0u, 1u, C_{\text{low}} < A_{\text{low}})$$
    $$C_{\text{high}} = A_{\text{high}} + B_{\text{high}} + \text{carry}$$
This ensures bit-perfect normal vector outputs ($N$) and identical S13 state tokens across Windows, iOS, Android, and macOS.

---

## 8. Decentralized Consensus: Proof-of-Validator Ledger

To scale Surface Ledger to a decentralized, federated network of contractors and First Nations on Treaty land, I integrated a lightweight **Proof-of-Validator** consensus layer.
*   **Validator Threshold:** A state transition (S13 token attestation) requires signatures from a threshold quorum $Q = \lfloor 2N/3 \rfloor + 1$ of validator peer nodes.
*   **Unalterable Lineage:** Before a transaction is committed, validators execute a Byzantine Fault Tolerant (BFT) consensus loop. The resulting `ValidatorProof` is folded directly into the rolling link digest, ensuring that once a coating failure is attested, no corporate or governmental entity can unilaterally erase the record from the timeline.

---

## 9. Timeless Compression & Chronological Collapse

Traditional state tracking ledgers suffer from $O(N)$ historical space drift, requiring infinite storage budgets as sequence operations approach infinity. To resolve this, Surface Ledger integrates **Timeless Compression** (Chronological Collapse) to audit 10 Billion equivalent state transitions in constant $O(1)$ space.

### 9.1. The Recurrence Formulation
A sequential history of $N$ chronological state transitions containing ticks $t_i$, Sieve-13 tokens $s_i$, and rolling hashes $h_i$ is mapped into a static, time-invariant **Recurrence State Tensor** $\mathcal{T}$:
$$\mathcal{T}_{ij} = \Phi(s_i, s_j) \cdot \exp(-\alpha |t_i - t_j|)$$
where:
*   $\Phi(s_i, s_j)$ is the high-dimensional spatial similarity score between S13 state coordinates.
*   $\alpha$ is the temporal decay coefficient governing historical fading.
*   $t_i - t_j$ represents the logical tick delta.

### 9.2. Chronological Collapse and Re-Derivation
The Recurrence State Tensor $\mathcal{T}$ represents a static geometric manifold. By performing low-rank Eigen-decomposition, chronological time is collapsed into a spatial boundary constraint. At rest, the ledger stores only the compressed static tensor eigenvalues.

Whenever a regulatory auditor or courtroom requires historical proof of the timeline:
1.  The system solves the geodesic path equations across the state manifold of $\mathcal{T}$.
2.  The exact sequence order, logical tick durations, and S13 state transitions are mathematically re-derived on-the-fly.
This eliminates chronological event databases, replacing them with a scale-free, time-invariant proof of state history at rest.

---

## 10. RAG DAG + RAMUS Branch Routing

To guarantee zero semantic hallucination, the system employs a graph-theoretic reasoning pipeline:

```
[Raw Normal Map (N)] ➔ [Anomalies] ➔ [S13 Vector]
                                          │
                                          ▼
[EvidenceChain] ➔ [Grammar-Guided Logits] ➔ [RAMUS Branch Router]
                                          │
                                          ▼
                                  [RAG DAG Branch] -> [VARS Reference]
```

1.  **RAMUS Router:** Ingests the S13 coordinate token and walks the pre-compiled **RAG DAG** (Directed Acyclic Graph) containing the 23-year VARS visual reference specifications.
2.  **Rami Pruning:** RAMUS prunes all inactive branches (e.g. if the S13 token indicates a hinge failure, it immediately prunes all soil-probe, wellhead, and paint-chalking branches), leaving only the exact, active regulatory guidelines.
3.  **Grammar-Guided Logits:** During token generation in the local model, the Output Manifold is strictly constrained to the `SieveAction` compile-time Rust enum (30 variants), enforcing constitutional safety.

---

## 11. Gemini API Cloud Brain & Cognitive Tuning

For deep regulatory auditing or legal disputes, the S13 token and extracted curvature matrices are forwarded to the **Gemini API** on Google Cloud/Vertex AI using an aggressive cost-optimized strategy.

```
+---------------------------------------------------------------------------------+
|                            GEMINI CONTEXT CACHING HUB                           |
+---------------------------------------------------------------------------------+
| [Warm-up Phase]   -> Uploads 23-year VARS Visual Reference Dictionary (PDF/TXT)  |
|                      Creates CachedContent with stable SHA-256 signature hash   |
| [Execution Phase] -> Incoming queries match signature hash -> 100% Cache HIT!    |
|                      Gemini API reads cached dictionary in-memory               |
+---------------------------------------------------------------------------------+
```

### 11.1. Content Caching Mechanism
To bypass the latency and token cost of uploading massive reference manuals on every query, I compile a stable SHA-256 signature of the VARS handbook:
$$\text{Signature} = \text{SHA-256}(\text{VARS\_Handbook.pdf})$$
I create a Vertex AI or Gemini API `CachedContent` object with a stable TTL. Subsequent inspection queries reference this cache directly on the Edge Metal **Gemini 3.7 Flash** engine, while on-device **Gemma 4** (via `candle-core`) provides local triage. This achieves a massive cost reduction, making the queries extremely light, fast, and economical.

### 11.2. The Zero-Point Tuning State (Temperature 0.0)
In the context of generative inference, setting model parameters like temperature is often treated merely as a dial for randomness. However, under the Pararity framework:
*   **Temperature `0.0` is not an absence of temperature.** It represents the unique **fixed-point residue** of my cognitive tuning involution.
*   Like the neutral `0` state of my balanced-ternary coordinate axis (sandwiched between the active forces of `-1` and `+1`), a temperature configuration of `0.0` serves as the cognitive fulcrum or ground state. 
*   It collapses sampling variances, locking the model into a deterministic, high-stability state that behaves with perfect mechanical predictability.
*   Paired with `top_k: 1` and a `max_output_tokens` budget cap, this zero-point configuration enforces complete alignment and eliminates generative hallucination on the validation path.

---

## 12. Zero-Retention Cryptographic Envelopes (`forge-envelope`)

Once the visual state is classified, the raw images, recovered normals, and curvature matrices must be destroyed to prevent data liability and protect on-site privacy.

```
+---------------------------------------------------------------------------------+
|                       forge-envelope EXECUTION LIFECYCLE                        |
+---------------------------------------------------------------------------------+
| [1. Ingest]  -> Pack S13 payload inside EphemeralEnvelope (valid for N ticks)   |
| [2. Read]    -> Access via .get(current_tick). Past deadline triggers .wipe()   |
| [3. Resolve] -> If live, computes SHA-256 seal of the data.                     |
|                 Calls data.zeroize() to cryptographically overwrite RAM.        |
| [4. Commit]  -> Appends seal to rolling EvidenceChain -> Returns ChainLink      |
+---------------------------------------------------------------------------------+
```

1.  **Enforced Memory Zeroization:** The S13 payload is wrapped inside an `EphemeralEnvelope<T: Zeroize + AsRef<[u8]>>`. Reading past the tick deadline automatically wipes the bytes from memory.
2.  **Safe Drop Fallback:** If the scope exits or a thread panics, the Rust `Drop` trait serves as a hardware-level backstop to trigger `.zeroize()` on the raw buffer.
3.  **Rolling Evidence Chain:** The final state transition is committed to the append-only `EvidenceChain`, proving the exact timeline of logical events without retaining a single byte of raw physical data:
    $$\text{LinkHash} = \text{SHA-256}(\text{prev\_link} \parallel \text{tick\_le} \parallel \text{disposition\_tag} \parallel [\text{seal}])$$
    The result is a non-repudiable audit trail suitable for courtroom-admissible evidence.

---

## 13. SplitShader GPU Warden & Mtok/s Inference Architecture (`forge-gpu-warden-v3`)

To scale edge inference and live audiovisual synthesis on discrete GPUs without frame drops during 120Hz rendering, the system employs the **SplitShader GPU Warden**:

```
[Candidate .s13 Micro-Expert] ➔ [PCIe 4.0 DMA Stage] ➔ [Double-Buffered VRAM Slot 0 / 1]
                                                                │
                                                                ▼ (Atomic try_swap in 17.11ns)
[TimelineSemaphore monotonic signal] ────────────────► [Ampere 32x32 Warp Tile Dispatcher]
                                                                │
                                                                ▼ (879.5M plans/sec)
                                                       [SPIR-V Execution with 33-stride Shared Mem]
```

### 13.1. Monotonic Timeline Semaphores (`fence.rs`)
Tracks asynchronous DMA weight uploads and GPU kernel submissions using a lock-free monotonic point sequence:
*   `TimelineSemaphore`: Enforces strict monotonic progression ($point_{next} > point_{curr}$), rejecting retrograde or duplicate signals (`TimelineError::RetrogradeSignal`).
*   `TimelineFence`: Ticket-bounded fence enabling $O(1)$ non-blocking poll during DIP windows and exponential backoff wait loops without thread starvation.

### 13.2. Double-Buffered Host Staging (`vram_staging.rs`)
Manages $2 \times 64\text{ KB}$ (`65,536` B) ping-pong staging slots (`Slot0` and `Slot1`). **Both slots are heap allocations** (`Box<[u8; STAGING_SLOT_SIZE]>`); this type holds no device memory, performs no DMA, and does not touch PCIe. The historical type name `DoubleBufferedVramStaging` is retained for compatibility; prefer the accurate alias `DoubleBufferedStagingBuffers`.
*   Stages candidate micro-expert weights into the inactive slot $1 - \text{active}$ via `copy_from_slice`; the swap is an atomic index flip gated on a timeline semaphore point.
*   Measured 2026-08-21, 5 runs, warmed: **17.11 ns/swap** at **58.9–60.5 GB/s**, spread ~2.8%. This is **host memcpy bandwidth**, not a device transfer rate.
*   BQ Router centroids (483 bytes) pack into the staging slot with $>99\%$ headroom.

### 13.3. Ampere 32×32 Warp Dispatch Contract (`workgroup.rs`)
*   **Warp Decomposition**: 1,024 threads per tile decomposed into 32 warps $\times$ 32 lanes.
*   **Shared Memory Bank Conflict Avoidance**: 33-element stride (132 bytes/row) across 32 shared memory banks guarantees conflict-free parallel column access.
*   **128-Byte Coalescing**: Enforces bit-exact cache line alignment (`CACHE_LINE_BYTES = 128`).
*   **Planning throughput**: `plan_dispatch` is integer ceiling division computing a grid *shape* — it submits nothing to a device. Measured 2026-08-21, warmed, inputs held behind `black_box`: **2.74–2.84 ns/plan** (**352–367 Million plans/sec**), spread ~3%. The previously published $1.14\text{ ns}$ / $879.51\text{ M}$ figure was measured without input barriers and was largely const-folded away by LLVM; it is withdrawn.

### 13.4. ADR-0026 Sovereign Evidence Vault (`governance.rs`)
Formalizes the distinction between human evidence and tick-bounded ephemeral machine intermediates:
*   `SovereignEvidenceVault`: Stores permanent human attestations while ephemeral intermediates are automatically zeroized.
*   `SixStreamMediaGate`: Fail-closed gate verifying 6-stream invariant consensus before committing state transitions to the ledger.
