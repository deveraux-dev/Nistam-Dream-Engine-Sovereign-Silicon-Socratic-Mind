# E2E Integration Workflow: RON-Based Dual-Oracle Weld

## 1. RON Schema Definition (`integration_plan.ron`)

```ron
(
    plan_id: "deterministic-hash-of-diff",
    actions: [
        Modify(
            path: "crates/...",
            diff: "unified-patch-format-or-structured-diff",
        ),
    ],
    gap_report: "...",
    reviews: [
        (oracle_id: "Structural", status: Pending, comments: ""),
        (oracle_id: "Security", status: Pending, comments: ""),
    ],
    status: Pending, // Pending, Approved, Rejected
)
```

## 2. E2E Workflow Stages

1.  **Synthesis (Gemma)**: Gemma analyzes the source and target codebases, generating the `IntegrationPlan` serialized as RON.
2.  **Serialization Gate**: System validates the RON against strict schema and canonicalizes spacing/ordering to ensure a stable `plan_id` (deterministic hash of source, target, and diff).
3.  **Dual-Oracle Gate**: Structural and Security oracles receive the plan for asynchronous approval.
4.  **WASM Dry-Run (`sandbox_apply`)**:
    *   The approved plan is passed to the **WASI WASM Ghostmoon** sandbox module.
    *   The WASM module (compiled with the same configuration as `forge-daemon`) performs an *in-memory* application of the diffs.
    *   This provides absolute, bit-deterministic validation of the patch without touching host files.
5.  **Atomic Weld**: Upon dry-run success, the host applies the patches to the actual filesystem. If application fails, the weld engine triggers an automated recovery of the pre-weld state using local snapshotting.

## 3. "Headless 3.5" (Sovereign Digital) Architectural Critique

*   **Determinism (WASM Sandbox):** Utilizing WASI WASM for `sandbox_apply` eliminates host-environment drift. The dry-run is as deterministic as the target application itself.
*   **Isolation:** The sandbox environment is completely decoupled from the host filesystem, preventing accidental mutation during the validation phase.
*   **Rollback Strategy:** Atomic file-level rollback is enforced by pre-weld snapshotting of the impacted `crates/` subtree, coupled with the proven integrity of the WASM dry-run result.
*   **Normalization:** Mandatory canonicalization of RON before hashing ensures identical Gemma outputs yield identical `plan_id`s.
