# 02_TIMELESS_COMPRESSION: D = T + F + R and CAS Hook Deduplication

**Specification Version:** 1.0.0  
**Status:** Canonical Spec  
**Classification:** Information Theory / Content-Addressable Architecture  

---

## 1. The Core Equation: D = T + F + R

Data streams in bare-metal synthetic cognition and software orchestration exhibit extreme structural redundancy when stored or transmitted naively. **Timeless Compression** formalizes the orthogonal factorization of any state or payload $D$ into three deterministic components:

$$D = T + F + R$$

Where:
- **$T$ (Topology / Type Home):** The invariant relational taxonomy and type signature (e.g., Crate ID, Schema Hash, Struct Layout, Trait Contract). $T$ is immutable across runtime generations.
- **$F$ (Form / Grammar / AST):** The structural template, syntax tree, or GBNF grammar harness. $F$ defines the valid transition space and structural scaffolding without runtime values.
- **$R$ (Residual / Innovation Delta):** The pure information-theoretic entropy and dynamic literals unique to this specific state execution instance.

```
       +-------------------------------------------------------+
       |                  DATA PAYLOAD (D)                     |
       +-------------------------------------------------------+
                                   |
         Decompose into Orthogonal Orthotopes (Zero Leakage)
                                   |
         +-------------------------+-------------------------+
         |                         |                         |
         v                         v                         v
+------------------+      +------------------+      +------------------+
|  TOPOLOGY (T)    |      |    FORM (F)      |      |   RESIDUAL (R)   |
|  - Type Home     |      |  - AST / Grammar |      |  - Literal Values|
|  - Schema Hash   |      |  - Layout Tree   |      |  - Entropy Delta |
|  - Invariant UID |      |  - Fixed Tokens  |      |  - Unique State  |
+------------------+      +------------------+      +------------------+
         |                         |                         |
         +-------------------------+-------------------------+
                                   |
                                   v
                   Content-Addressed CAS Injection
                     .forge/objects/<blake3_hash>
```

---

## 2. Content-Addressable Storage (CAS) Specification

**Status:** Specification. Reproduction commands for the measured lanes live in `docs/BENCHMARKS.md`.

All decomposed components ($T$, $F$, $R$) and complete artifacts are stored in an append-only, zero-lock Content-Addressable Store located under `.forge/objects/`:

```
.forge/
  objects/
    0a/
      0a3f81e29c...  <- 64-char BLAKE3 content-addressed blob
    e4/
      e4917a221b...
```

### 2.1 Addressing & Hashing Invariant
- **Primary Digest**: 256-bit BLAKE3 cryptographic hash formatted as 64 lowercase hex characters.
- **Fast In-Memory Tag**: 64-bit FNV1a / wyhash integer for lock-free atomic register comparison.
- **Storage Path**: `.forge/objects/{hash[0..2]}/{hash[2..64]}`.

### 2.2 Storage Deduplication Mechanics
When emitting an execution receipt, token payload, or code merge artifact:
1. Hash component payload $C \to H = \text{BLAKE3}(C)$.
2. If file `.forge/objects/{H[0..2]}/{H[2..64]}` exists, skip I/O write (zero disk cycle penalty).
3. If absent, write to temporary scratch file `.forge/objects/tmp_{PID}_{timestamp}` and execute an atomic file rename (`MoveFileExW` / `REPLACE_EXISTING`).
4. Emit CAS reference envelope: `CASRef { hash: [u8; 32], size: u64 }`.

---

## 3. Discard-to-Anchor Fallback Protocol

Under strict real-time deadlines (e.g. 1200ms latency envelope), when inference, synthesis, or compilation cannot complete:
1. Execution halts immediately without dropping input intent.
2. The raw unresolved intent payload is assigned $R_{\text{raw}}$, with $T = T_{\text{fallback}}$, $F = F_{\text{raw\_blob}}$.
3. The raw payload is committed to `.forge/objects/<hash>` within $\le 5\text{ms}$.
4. Status `FALLBACK_ANCHOR` is emitted over socket 13013 to `HUD.html`.
5. No memory is leaked, no locks are held, and the system state remains 100% sound.

---

## 4. Empirical Compression Ratios

**Table 1:** Illustrative examples under the D=T+F+R decomposition (not measured benchmarks).

| Data Category | Raw Size | $T+F$ Shared Ref | $R$ Residual | Total CAS Delta | Compression Ratio |
|---|---|---|---|---|---|
| Gemma AST Merge | 148.2 KB | 120 B (Ref) | 4.8 KB | 4.92 KB | **30.1 : 1** |
| Synesthetic Audio Frame | 32.0 KB | 64 B (Ref) | 0.9 KB | 0.96 KB | **33.3 : 1** |
| Mud Room State Delta | 16.4 KB | 64 B (Ref) | 0.3 KB | 0.36 KB | **45.5 : 1** |

Measurement methodology and reproduction commands: see `docs/BENCHMARKS.md`.
