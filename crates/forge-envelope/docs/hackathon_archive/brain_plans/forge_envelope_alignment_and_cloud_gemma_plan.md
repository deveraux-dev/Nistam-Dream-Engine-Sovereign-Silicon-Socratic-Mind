# Implementation Plan: Forge-Envelope Document Alignment & Cloud Gemma Deployment

Align `forge-envelope` documentation, core Rust state-engines, and cloud pipeline with the **Pararity 6-Stream Differential Inversion**, the **Sovereign Triad of Mercy and Authority** (`ADR-0026`, `mercy-tick-metabolic-ttl.md`, `ADR-0036`), the **Zenodo DOI: 10.5281/zenodo.22020676** citation, and deploy containerized lightweight Gemma-S13 sidecar engines to Google Cloud Platform (`nde1-493505`).

---

## Goal Description

1. **Document Alignment & Sovereign Canon:**
   * Integrate formal citations for Sean Morin's published Zenodo work: *Pararity: the fixed-point residue of an involution, and why we need it* ([DOI: 10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676)).
   * Align [`docs/SOVEREIGN_MASTER_CANON.md`](file:///F:/v3/crates/forge-envelope/docs/SOVEREIGN_MASTER_CANON.md), [`docs/SUBMISSION_ENTRY.md`](file:///F:/v3/crates/forge-envelope/docs/SUBMISSION_ENTRY.md), [`docs/HANDOFF.md`](file:///F:/v3/crates/forge-envelope/docs/HANDOFF.md), and [`GEMINI.md`](file:///F:/v3/crates/forge-envelope/GEMINI.md) to reflect:
     * **Multi-stream sensory governance** (beyond single-photo inspection) with lightweight industrial protocols (RS-485, CAN bus, CoAP/CBOR, GPIO).
     * **3-stream to 1-trit reduction & 6-stream inverted differential signaling** (common-mode noise rejection, fail-closed phase cancelation).
     * **Sovereign Triad:**
       * `ADR-0026`: Self-attestation, hybrid vault seam (`SOURCE_HUMAN_AUTHORED` vs Map-Edge 0-byte machine storage, anti-laundering).
       * `mercy-tick-metabolic-ttl.md`: Mercy tick, crypto-erasure of HKDF seeds/salts, allostasis / metabolic renewal.
       * `ADR-0036`: Three-clock separation (Author/Inference writes artifact, Bake freezes, 120Hz Tick consumes).
2. **Core Rust Engine Primitives:**
   * Implement 6-stream conjugate / differential pair reduction in [`somatic_tokenizer.rs`](file:///F:/v3/crates/forge-envelope/src/somatic_tokenizer.rs) and [`s13.rs`](file:///F:/v3/crates/forge-envelope/src/s13.rs).
   * Implement explicit Mercy-Tick crypto-shredding and zero-day watchdog lifecycle in [`lib.rs`](file:///F:/v3/crates/forge-envelope/src/lib.rs).
3. **Cloud Gemma-S13 Deployment & Autonomous Looping:**
   * Build a lightweight container for the Gemma-S13 / 1-byte autoencoder inference engine.
   * Deploy the container to Google Cloud Run in `northamerica-northeast1` on project `nde1-493505`.
   * Wire Cloud Run with GCS inbox (`nde1-493505-s13-inbox`), Firestore state persistence, and [`billing_guard.py`](file:///F:/v3/crates/forge-envelope/scripts/billing_guard.py) for the autonomous 21+ day loop (competition + 13 days).

---

## User Review Required

> [!IMPORTANT]
> **Container Base Image & Cloud Run Sizing:**
> The lightweight Gemma-S13 container is designed to run under a **2 CPU / 4GB RAM** Cloud Run instance tier (cost-efficient, fitting comfortably within the promotional credit allocation). We will package the Rust `attest` binary alongside Python `agent_loop.py` and Gemma-S13 quantizer/LUT weights.

> [!WARNING]
> **GCP Cloud Run Deployment Permissions:**
> Deployment requires `gcloud` authenticated with permissions to push to Google Artifact Registry / Google Container Registry on project `nde1-493505`.

---

## Open Questions

> [!NOTE]
> **Model Endpoint Priority for Cloud Loop:**
> By default, the Cloud Run sentry will use `gemini-2.5-flash` for high-frequency $O(1)$ triage queries, with `gemini-1.5-pro` reserved for initial 450k-token VARS context caching and high-severity contested audits. If you prefer a pure Flash loop, we can lock all calls to Flash.

---

## Proposed Changes

```
┌───────────────────────────────────────────────────────────────────────────┐
│                           PLAN EXECUTION PHASES                           │
│                                                                           │
│  Phase 1: Canonical Document Alignment & Zenodo Citation                  │
│           (SOVEREIGN_MASTER_CANON, SUBMISSION_ENTRY, HANDOFF, GEMINI.md)  │
│                                │                                          │
│  Phase 2: Rust Engine Upgrades (Differential Inversion & Mercy-Tick)      │
│           (lib.rs, s13.rs, somatic_tokenizer.rs, bin/chaos_monkey.rs)     │
│                                │                                          │
│  Phase 3: Gemma-S13 Cloud Containerization                                │
│           (Dockerfile, scripts/agent_loop.py, scripts/cloud_run_client.py)│
│                                │                                          │
│  Phase 4: GCP Deployment & Cloud Looping Activation                       │
│           (deploy_vertex_cloudrun.ps1, billing_guard.py, test loop)       │
└───────────────────────────────────────────────────────────────────────────┘
```

---

### Component 1: Crate Documentation & Master Canon

#### [MODIFY] [`docs/SOVEREIGN_MASTER_CANON.md`](file:///F:/v3/crates/forge-envelope/docs/SOVEREIGN_MASTER_CANON.md)
* Add formal Zenodo DOI citation badge and metadata ([DOI: 10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676)).
* Expand Section 3 to include the **3-Stream $\to$ 1-Trit & 6-Stream Differential Inversion** mathematics ($T + T^* = 0$).
* Add formal architectural sections for:
  * `ADR-0026`: Self-Attestation & Hybrid Vault Seam (Map-Edge 0-byte machine storage vs Human-Authored Permanent Vault).
  * `mercy-tick-metabolic-ttl.md`: The Mercy Tick, HKDF seed crypto-erasure, and allostatic renewal.
  * `ADR-0036`: Three-Clock Division (Author Async $\to$ Bake Freeze $\to$ 120Hz Tick).

#### [MODIFY] [`docs/SUBMISSION_ENTRY.md`](file:///F:/v3/crates/forge-envelope/docs/SUBMISSION_ENTRY.md)
* Update system overview to reflect multi-stream sensory governance (strain, freeze-thaw, acoustic emission, tactile registers) alongside multimodal imaging.
* Add Zenodo citation and link to published Pararity research.
* Clarify the Dual-Oracle division: Local Gemma-S13 (Weaver) proposes $\to$ Cloud Gemini 3.7 Flash/Pro (Arbiter) mechanically validates.

#### [MODIFY] [`docs/HANDOFF.md`](file:///F:/v3/crates/forge-envelope/docs/HANDOFF.md)
* Record the landing of Zenodo citation, differential stream laws, and sovereign triad invariants.
* Update running build queue to track Cloud Run deployment on `nde1-493505`.

#### [MODIFY] [`GEMINI.md`](file:///F:/v3/crates/forge-envelope/GEMINI.md)
* Update architectural context table and coding guidelines with the Three-Clock Law and Mercy-Tick constraints.

---

### Component 2: Core Rust Engine Upgrades

#### [MODIFY] [`src/s13.rs`](file:///F:/v3/crates/forge-envelope/src/s13.rs)
* Add `DifferentialTriad` struct implementing 3-stream $\to$ 1-trit reduction and 6-stream inverted conjugate evaluation:
```rust
/// 3-Stream physical triad collapsing into 1 balanced trit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriadStream {
    pub positive: i32,
    pub neutral: i32,
    pub negative: i32,
}

impl TriadStream {
    /// Collapses 3 streams into 1 balanced trit (-1, 0, +1) with deadband epsilon.
    pub fn resolve_trit(&self, deadband: i32) -> i8 {
        let diff = self.positive - self.negative;
        if diff > deadband { 1 }
        else if diff < -deadband { -1 }
        else { 0 }
    }
}

/// 6-Stream differential pair (direct + conjugate inverted triad).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialTriad {
    pub direct: TriadStream,
    pub inverted: TriadStream,
}

impl DifferentialTriad {
    /// Evaluates common-mode noise cancellation and invariant check (T + T* == 0).
    pub fn evaluate(&self, deadband: i32) -> Result<i8, LunarSentinel> {
        let t_direct = self.direct.resolve_trit(deadband);
        let t_inverted = self.inverted.resolve_trit(deadband);
        if t_direct + t_inverted != 0 {
            // Symmetry broken: common-mode failure or physical tampering
            Err(LunarSentinel::MikikapisePisim) // Moon 254: Sabotage/Tamper Sentinel
        } else {
            Ok(t_direct)
        }
    }
}
```

#### [MODIFY] [`src/somatic_tokenizer.rs`](file:///F:/v3/crates/forge-envelope/src/somatic_tokenizer.rs)
* Add multi-sensor register unpacking methods (Modbus 16-bit registers, CAN bus 8-byte frames).

#### [MODIFY] [`src/lib.rs`](file:///F:/v3/crates/forge-envelope/src/lib.rs)
* Integrate formal Mercy-Tick TTL expiry and HKDF salt crypto-shredding helper.
* Ensure `Disposition::Expired` triggers metabolic renewal and zeroization.

#### [MODIFY] [`src/bin/chaos_monkey.rs`](file:///F:/v3/crates/forge-envelope/src/bin/chaos_monkey.rs)
* Add Gate E: **6-Stream Differential Asymmetry Injection** to verify that sensor spoofing trips the Sabotage Moon Sentinel and commits a violation receipt to `EvidenceChain`.

---

### Component 3: Gemma-S13 Cloud Containerization & Deployment

#### [MODIFY] [`Dockerfile`](file:///F:/v3/crates/forge-envelope/Dockerfile)
* Ensure multi-stage build:
  1. Rust builder stage compiles `attest` and `chaos_monkey` in release mode.
  2. Python/Debian slim runtime installs `google-genai`, `google-cloud-firestore`, `google-cloud-storage`, and copies binaries.
  3. Pre-packages LUT vocabulary arrays (`<2.6MB`) for Gemma-S13.

#### [MODIFY] [`scripts/agent_loop.py`](file:///F:/v3/crates/forge-envelope/scripts/agent_loop.py)
* Update the loop to support multi-stream sensor JSON telemetry alongside image blobs.
* Integrate [`billing_guard.py`](file:///F:/v3/crates/forge-envelope/scripts/billing_guard.py) directly into the polling loop to prevent budget overruns.

#### [MODIFY] [`scripts/deploy_vertex_cloudrun.ps1`](file:///F:/v3/crates/forge-envelope/scripts/deploy_vertex_cloudrun.ps1)
* Ensure automated Cloud Run deployment to `nde1-493505` in `northamerica-northeast1` with correct service account bindings (`roles/aiplatform.user`, `roles/datastore.user`, `roles/storage.objectAdmin`).

---

## Verification Plan

### Automated Tests
```powershell
# 1. Run all workspace unit and doc tests
cargo test --workspace

# 2. Run high-throughput scale and sabotage defense benchmarks
cargo test -p forge-envelope --test scale_test -- --nocapture

# 3. Run Chaos Monkey self-sabotage verification (Gates A, B, C, D, E)
cargo run -p forge-envelope --bin chaos_monkey

# 4. Verify live billing script and schema validator with Gemini
python F:\v3\crates\forge-envelope\scripts\verify_billing_draw.py --model gemini-2.5-flash --queries 1 --no-confirm
```

### Manual Verification
1. Verify that [`docs/SOVEREIGN_MASTER_CANON.md`](file:///F:/v3/crates/forge-envelope/docs/SOVEREIGN_MASTER_CANON.md) displays the Zenodo DOI badge and academic citation correctly.
2. Confirm that `billing_guard_state.json` and `live_chaos_report.json` update in real time.
3. Validate Cloud Run deployment status on GCP project `nde1-493505` via `gcloud run services describe surface-ledger-sentry --region northamerica-northeast1`.
