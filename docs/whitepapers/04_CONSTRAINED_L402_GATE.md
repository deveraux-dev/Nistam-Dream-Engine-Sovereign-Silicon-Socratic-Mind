# 04_CONSTRAINED_L402_GATE: Sampling GBNF Masks and HTTP 402/L402 Gate

**Specification Version:** 1.0.0  
**Status:** Canonical Spec  
**Classification:** Tokenomics / Constrained Sampling Architecture  

---

## 1. Executive Summary

The **Constrained L402 Gate** unifies grammatical sampling constraints (GBNF logit masks) with cryptographic capability authentication (HTTP 402 / L402). This dual-gate architecture enforces two fundamental system invariants:

1. **Zero Syntax Waste**: Every token sampled is mathematically guaranteed to conform to the grammar schema (JSON, RON, or Vixi DSL) via token-level logit masking before argmax/sampling.
2. **Zero Unauthorized Execution**: Every compute invocation must present a valid L402 capability macaroon + preimage receipt, adhering to strict unit-cost governor limits ($0.0004/call).

```
                      +---------------------------------------+
                      |       INCOMING INFERENCE REQUEST      |
                      +---------------------------------------+
                                          |
                                          v
                      +---------------------------------------+
                      |         L402 CAPABILITY GATE          |
                      |   - Verify Macaroon Caveats           |
                      |   - Validate Preimage / Quota Token   |
                      +---------------------------------------+
                                          | Valid
                                          v
                      +---------------------------------------+
                      |         GBNF GRAMMAR COMPILER         |
                      |   - Compile Schema to Pushdown DFA    |
                      |   - Compute Valid Next-Token Mask     |
                      +---------------------------------------+
                                          |
                                          v
                      +---------------------------------------+
                      |      LOGIT BITMASK INTERCEPTOR        |
                      |   Logit[i] = -inf for invalid tokens  |
                      +---------------------------------------+
                                          |
                                          v
                      +---------------------------------------+
                      |      ZERO-WASTE TOKEN SAMPLING        |
                      |    (100% Schema-Compliant Output)     |
                      +---------------------------------------+
```

---

## 2. Morphological & Structural Logit Masking via RagDag (IMPLEMENTED)

The **Courtroom-Admissible RAG-DAG Logit Masking Engine** (`crates/gemma-s13/src/logit_mask.rs`) enforces zero-syntax waste by compiling an acyclic reference graph (RAG-DAG) containing strictly witnessed canonical morphological forms. At inference time, logit decoding distributions are dynamically masked to force unauthorized / un-witnessed transition paths to absolute zero probability (`i32::MIN` in fixed-point space).

### 2.1 Witnessed DAG State & Valid Transitions
The masking engine maintains a static acyclic reference graph (`RagDag`) with up to 32 nodes, each representing a witnessed morphological form. Every node carries:
- **Token Identifier**: vocabulary index of the form.
- **Provenance Tag**: courtroom citation / witness hash (`u64`).
- **Outward Edges**: up to 8 allowed next-token transitions.

For each generation step, the current DAG node's valid successor tokens are determined via the `allows_transition()` check.

### 2.2 Logit Masking Transform via Fixed-Point Masking
Before applying softmax or sampling (greedy / nucleus), the masking layer tests every vocabulary token against the current node's allowed transitions:

$$\text{masked\_logit}_i = \begin{cases} z_i & \text{if current node allows } i \\ i32::\text{MIN} & \text{otherwise} \end{cases}$$

This guarantees zero syntax waste: every sampled token is witnessable in the DAG, and no out-of-grammar utterance can escape the logit barrier.

**Note on JSON/RON/GBNF Generalization**: The current implementation enforces canonical morphological constraints. A generalized GBNF pushdown compiler for arbitrary JSON/RON/Vixi grammars is listed as **future work** (see Section 6).

---

## 3. Proposed Architecture: L402 Capability Gate (UNIMPLEMENTED — specification only)

**Status**: This section describes the architectural design for L402 HTTP-402 capability authentication. No implementation currently exists in the codebase; it is specification-only and subject to change.

The L402 protocol wraps standard HTTP status `402 Payment Required` into a cryptographically verifiable challenge-response loop:

### 3.1 Challenge Format (`WWW-Authenticate`)
```http
HTTP/1.1 402 Payment Required
WWW-Authenticate: L402 macaroon="AGF...==", invoice="lnbc10u1p..."
```

### 3.2 Authorization Header
Clients present the capability token and payment proof in the `Authorization` header:
```http
Authorization: L402 <base64_macaroon>:<32_byte_hex_preimage>
```

### 3.3 Macaroon Attenuation Rules:
- `time_expiry < uint64_utc`
- `max_tokens <= 2048`
- `target_model == gemma-9b-s13`
- `schema_hash == 0x7f83...`

---

## 4. Integration with N × IPR Gating (implemented + tested: src/nipr.rs)

The L402 gate couples directly with the **Normalized IPR** metric from Whitepaper 01:
1. When $\text{N} \times \text{IPR} \ge 7500\text{ pmy}$ (LANDMARK_PMY threshold), the query is highly focused: L402 credit consumption is scaled down ($1\times$ baseline cost).
2. When $\text{N} \times \text{IPR} < 2500\text{ pmy}$ (DIFFUSE_PMY threshold), the query exhibits excessive entropy: the system rejects execution or requires higher credit staking before running unconstrained sampling.

The N × IPR computation is deterministic, zero-transcendental, and exact across all platforms. Reference implementation: crates/gemma-s13/src/nipr.rs, with full test coverage in the test suite.

---

## 5. Cost Governance & Rate Limiting

**Unit Cost**: The baseline cost per inference call is set to **$0.0004** (ceiling constant; enforcement via the cloud governor scripts). This figure applies when no N × IPR attenuation is active. The governor maintains per-session quota state and rejects calls that would exceed the remaining allocation.
