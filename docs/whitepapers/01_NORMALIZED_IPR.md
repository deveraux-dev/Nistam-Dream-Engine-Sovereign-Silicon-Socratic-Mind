# 01_NORMALIZED_IPR: Zero-Transcendental State Localization

**Specification Version:** 1.1.0  
**Status:** Canonical Spec  
**Classification:** Bare-Metal Architecture / Substrate Primitive  

---

## 1. Executive Summary & Core Invariant

The **Normalized Inverse Participation Ratio (N × IPR)** specification defines a deterministic, zero-transcendental metric for measuring the localization and entropy of computational, neural, and synesthetic state vectors.

In traditional dynamical and quantum systems, the Inverse Participation Ratio ($IPR$) measures the degree of localization of a normalized state vector across a finite discrete basis:
$$IPR = \sum_{i=1}^N |a_i|^4$$
For a completely delocalized state over dimension $N$ (uniform distribution $a_i = 1/\sqrt{N}$), $IPR = 1/N$. For a fully localized state on a single basis coordinate ($a_k = 1, a_{i \neq k} = 0$), $IPR = 1$.

In bare-metal deterministic execution environments, standard IEEE 754 floating-point transcendental functions ($\exp, \ln, \sin, \cos$) introduce non-deterministic compiler intrinsics, microarchitectural jitter, and unbounded cycle latency. **Normalized IPR** replaces transcendental metrics with an exact integer fixed-point formulation scaled in **Permyriad** units ($1\text{ pmy} = 0.01\% = 10^{-4}$):

$$\text{N} \times \text{IPR} \in [0, 10000]\text{ pmy}$$

---

## 2. Mathematical Formalism

Let $V = [v_1, v_2, \dots, v_N] \in \mathbb{Z}_{\ge 0}^N$ be a non-negative discrete activation or mass vector over an $N$-dimensional space.

### 2.1 Integer Mass Summation
The total activation mass $S_1$ and quadratic mass $S_2$ are computed using standard 64-bit unsigned accumulators without dynamic allocation:
$$S_1 = \sum_{i=1}^N v_i, \quad S_2 = \sum_{i=1}^N v_i^2$$

### 2.2 Unnormalized IPR Ratio
The raw discrete participation ratio is given by:
$$\mathcal{P}_{\text{raw}} = \frac{S_2}{(S_1)^2} \in \left[\frac{1}{N}, 1\right]$$

### 2.3 Permyriad Scaling & Normalization
To decouple the localization metric from the dimensionality $N$ and map it linearly onto the fixed-point domain $[0, 10000]$, the normalized metric is evaluated as:
$$\text{N} \times \text{IPR} = \left\lfloor \frac{N \cdot S_2 - S_1^2}{(N - 1) \cdot S_1^2} \times 10000 \right\rfloor$$

For $S_1 = 0$ (zero-mass / dead context, any $N$):
$$\text{N} \times \text{IPR} \triangleq 0\text{ pmy}$$

For $N = 1$ with $S_1 > 0$ (single nonzero element):
$$\text{N} \times \text{IPR} \triangleq 10000\text{ pmy}$$

---

## 3. Zero-Transcendental State Localization Architecture

```
+-------------------------------------------------------------------+
|                   ZERO-TRANSCENDENTAL PIPELINE                    |
|                                                                   |
|   Discrete Token / State Vector  V in Z^N                         |
|                      |                                            |
|                      v                                            |
|   +--------------------------------------+                        |
|   | S1 = sum(v_i)    S2 = sum(v_i^2)     |  Integer Accumulation  |
|   | (Safe Atomic Packed Words / u64)     |  hotpath_alloc = 0     |
|   +--------------------------------------+                        |
|                      |                                            |
|                      v                                            |
|   +--------------------------------------+                        |
|   | N x IPR = (N*S2 - S1^2) / ((N-1)*S1^2)| Exact Fixed-Point     |
|   | x 10000 pmy                          | 0 .. 10000 pmy         |
|   +--------------------------------------+                        |
|                      |                                            |
|          +-----------+-----------+                                |
|          |                       |                                |
|          v                       v                                |
|   [ pmy >= T_local ]      [ pmy < T_local ]                       |
|   LOCALIZED STATE         DELOCALIZED / CHAOTIC                   |
|   -> Pass Gate            -> Intercept / Quota Reject             |
+-------------------------------------------------------------------+
```

### Invariants:
1. **Zero Heap Allocation (`hotpath_heap_bytes == 0`)**: All state vector calculations operate in-place over fixed-size ring buffers or static slices.
2. **Zero Transcendental Instructions**: Transcendental calls (`f32::sin`, `f64::exp`, `powf`) are strictly banned on the hot path. Trigonometric and exponential transforms map to integer lookup tables (LUTs) or balanced ternary Chebyshev expansions.
3. **Atomic State Localization Invariant**: State localization values are packed into a single `AtomicU64` bitfield:
   - Bits 0..15: `pmy_level` (0..10000)
   - Bits 16..31: `dimension_n` (0..65535)
   - Bits 32..47: `gate_status` (0: INIT, 1: ACTIVE, 2: FALLBACK, 3: FAULT)
   - Bits 48..63: `sequence_tick` (wrapping counter)

**Dimension Representation Dual:** `NormalizedIpr::dimension` is a `u32` to support contexts exceeding 65535 dimensions (e.g., 128k-length KV-cache slices); the `NiprPackedWord` bitfield's `dimension_n` saturates at `u16::MAX` (0xFFFF sentinel = overflow). Read true $N$ from the `NormalizedIpr` struct, never the packed telemetry word.

---

## 4. Gate Integration & Telemetry

When driving local inference models (e.g. Gemma S13) or synesthetic audio/visual render passes:
- **High Localization ($\ge 7500\text{ pmy}$)**: Singular, sharp attractor state. Direct execution granted.
- **Moderate Localization ($2500 - 7499\text{ pmy}$)**: Multimodal balanced state. GBNF sampling grammar constrained execution.
- **Low Localization / Delocalized ($< 2500\text{ pmy}$)**: High entropy/hallucinatory drift. Triggers early termination or fallback anchor dump.

---

## 5. Bare-Metal Conformance Test Vectors

| $N$ | Vector $V$ | $S_1$ | $S_2$ | Expected $\text{N} \times \text{IPR}$ (pmy) | Verdict |
|---|---|---|---|---|---|
| 4 | `[10, 0, 0, 0]` | 10 | 100 | 10000 | Pure Localized |
| 4 | `[10, 10, 10, 10]` | 40 | 400 | 0 | Pure Delocalized |
| 4 | `[10, 10, 0, 0]` | 20 | 200 | 3333 | Bimodal |
| 8 | `[100, 0, 0, 0, 0, 0, 0, 0]` | 100 | 10000 | 10000 | Singular Anchor |
| 4 | `[0, 0, 0, 0]` | 0 | 0 | 0 | Zero Mass / Dead Context |
