# Federated Edge-to-Cloud Architecture Plan (Gemini 3 + Gemma 4)

## Objective
Establish a state-of-the-art "Edge Metal & Sovereign Edge" architecture for the Surface Ledger submission, leveraging the absolute latest Google AI ecosystem (mid-2026). This plan upgrades the architecture to utilize Gemma 4 (via Rust `candle-core`) for edge-native, zero-compute DFA arbitration, and Gemini 3.7 Flash (via Google Cloud SDK) for deep regulatory reasoning, orchestrated by the Antigravity CLI.

## Key Components

1.  **Sovereign Edge: Gemma 4 via `candle-core`**
    *   **Role:** Local, offline RAMUS branch pruning and initial S13 spatial token analysis.
    *   **Model:** Gemma 4 (e.g., `google/gemma-4-5B-it`) executed natively in Rust using Hugging Face's `candle-core`.
    *   **Benefit:** Achieves true "Compute at Rest" by performing critical inference without leaving the physical device (e.g., an inspector's tablet in a remote location), ensuring privacy and zero latency.

2.  **Edge Metal: Gemini 3.7 Flash via Vertex AI SDK**
    *   **Role:** Deep regulatory reasoning and cloud-level attestation.
    *   **Capabilities:** Utilizes the massive 2M token context window to ingest the entire 23-year VARS dictionary via Vertex AI Context Caching.
    *   **Trigger:** When the edge model (Gemma) identifies a critical anomaly, the high-value spatial data (e.g., `EPHEMERIS-SYNC-T-ZERO-ALIGNMENT`) is packed into a `forge-envelope` and escalated to Gemini 3.7 Flash for a legally binding compliance verdict.

3.  **Command Center: Antigravity CLI**
    *   **Role:** The production operator interface and deployment orchestrator.
    *   **Function:** Visualizes the local Gemma inference, monitors the Chaos Monkey sabotage gates, and streams cloud verdicts in a high-performance terminal UI.

4.  **Weaver Arbiter (Zero-Compute DFA)**
    *   **Role:** Deterministic conflict resolution.
    *   **Function:** Maps the S13 physical state token to a resolution state using a static, pre-compiled state machine (DFA) in `#![no_std]` Rust (`src/weaver.rs`), completely bypassing runtime compute overhead.

## Implementation Steps

1.  **Refactor `chaos_monkey.rs` Payloads:**
    *   Replace the legacy "secret" payload naming convention with high-value spatial and timezone alignment data (e.g., `b"EPHEMERIS-SYNC-T-ZERO-ALIGNMENT"`).
2.  **Build Weaver Arbiter (`src/weaver.rs`):**
    *   Implement the DFA-based conflict resolution module.
    *   Ensure it interrogates the `forge-envelope` `EvidenceChain` head in O(1) time.
3.  **Update Documentation (`ARCHITECTURE.md` & `SUBMISSION_ENTRY.md`):**
    *   Formalize the transition to the Gemini 3 / Gemma 4 federated architecture.
    *   Highlight the roles of `candle-core` and Antigravity CLI.

## Verification & Testing
*   Ensure all Rust components (`forge-envelope`, `chaos_monkey`, `weaver`) compile successfully (`cargo check`).
*   Verify the Chaos Monkey daemon runs locally and successfully simulates the sabotage gates with the updated Ephemeris payloads.