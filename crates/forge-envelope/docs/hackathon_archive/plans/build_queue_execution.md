# Implementation Plan: Sovereign Engine Locked Build Queue Execution

## Objective
Execute the remaining tasks from the August 17th LOCKED build plan in strict sequential order. This will finalize the "Surface Ledger" submission for the All Things Agentic Hackathon, ensuring all physical attestation loops, cryptographic evidence chains, and thermodynamic governor components are fully realized and verifiable.

## Background & Motivation
The `forge-envelope` crate establishes a zero-retention, tamper-evident cryptographic chain. While the core workspace and 29 unit tests are complete, the `attest` CLI tool requires tests and specific sabotage defense logic. Following this, the python orchestration layer (`agent_loop.py` and Vertex AI schema client) requires hardening to meet the rigorous constraints set out by the Lead Systems Engineer.

## Scope & Impact
This plan covers the immediate engineering tasks (Steps 1-3) required to unblock the rest of the deployment and submission pipeline:
1.  **`attest.rs` hardening:** Adding batch processing tests and the L18 sabotage check to the Rust CLI binary.
2.  **Schema Client Model Bump:** Updating the Vertex GenAI target model to the required `gemini-3.7-flash` version.
3.  **Agent Loop Reconstruction:** Building `agent_loop.py` to seamlessly pipe data into the `attest` binary and manage tmpfs staging/firestore ACKs.

## Implementation Steps

### Phase 1: Harden the First Weld (`attest.rs`)
1.  **Analyze Sabotage Context:** Verify the exact conditions for the "L18 sabotage check" based on the `scale_test.rs` active wire-level tampering vectors.
2.  **Add Unit Tests:** Implement a `#[cfg(test)]` module inside `src/bin/attest.rs` testing:
    *   Batch NDJSON input parsing.
    *   Verification of `asset` and `operator` events mapping to their corresponding dispositions.
    *   The "L18 sabotage check" (validating that injected corrupted links or revoked payloads cannot carry an attested seal and trigger an abort).

### Phase 2: Python Orchestration (Steps 2 & 3)
1.  **Model Bump:** Modify `scripts/vertex_schema_client.py` at line 27 to update `DEFAULT_MODEL = "gemini-2.5-flash"` to `DEFAULT_MODEL = "gemini-3.7-flash"`.
2.  **Construct `agent_loop.py`:** Create `scripts/agent_loop.py` incorporating the strict rules from the handoff:
    *   Invoke the `attest` binary natively.
    *   Ensure divergence paths write a receipted escalation record (no silent fall-throughs).
    *   Utilize tmpfs for staging via `STAGING_DIR` env.
    *   Add the `--manual` identical path trigger.

### Phase 3: Downstream Queue (Steps 4+)
1.  **Gemma Serving / Deployment Documentation:** Audit and lock the documentation for Gemma 4 E2B serving and the Google Cloud Run deployment strategy.
2.  **README Updates:** Fix proof-states and finalize the public repository facing files in alignment with the "Sovereign Parity" story spine.

## Verification & Testing
*   Execute `cargo test --bin attest` to ensure the CLI tests pass and the L18 sabotage check successfully trips.
*   Execute a dry-run of the `agent_loop.py` utilizing the new 3.7-flash model target to confirm end-to-end integration without cloud-retention leakage.
*   Ensure all edits conform strictly to the zero-allocation/zero-heap principles of the Sovereign Engine.