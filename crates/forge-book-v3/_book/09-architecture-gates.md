# ARCHITECTURE_GATES.md — The Fractal Gating Pattern

**STATUS: DRAFT**
**OWNER: Principal Systems Synthesizer**

This document formalizes a recurring architectural pattern observed across the 13Forge stack: **Fractal Gating**.

## 1. The Fractal Gating Principle

Fractal Gating is a design pattern that enforces **Deterministic Isolation** to achieve **Emergent Synthesis**.

At every scale of the system—from agent orchestration and ML data pipelines to low-level entity scheduling—we see instances of gates that apply a strict, deterministic filtering rule to an input. This act of isolation (stripping context, validating structure, filtering data, or separating time) is not a mere sanity check. It is a necessary precondition to force a subsequent stage of the system to produce a higher-quality, more robust, or genuinely novel output (the "synthesis").

The pattern is "fractal" because this same shape of **Isolate → Synthesize** appears at microscopic and macroscopic levels of the architecture, suggesting it is a fundamental principle of building robust, creative, and safe intelligent systems.

## 2. Manifestations of the Pattern

The following systems are canonical implementations of Fractal Gating.

### 2.1 The Self-Referential Instability Loop Gate (`DreamDriver`)

- **System:** `forge-broski::DreamDriver`, the Worker → Coder → Reviewer agent pipeline.
- **Mechanism:** As specified in **Invention #003**, when a Reviewer agent rejects a Coder's output, the pipeline loops back for another attempt. The Loop Gate ensures that in the next iteration, the Reviewer agent's prompt **explicitly excludes its own prior verdict**.
- **Deterministic Isolation:** The previous verdict from the *same agent* is stripped from its context. This is a deterministic, structural rule enforced by the `DreamDriver` orchestrator.
- **Emergent Synthesis:** By isolating the agent from its own prior judgment, the gate prevents self-conditioning and path-dependency. It forces the agent to re-evaluate the artifact from first principles, enabling a more robust and independent verification of quality—a new "synthesis" of judgment. This is the anti-pattern to "Reflexion," which deliberately exploits self-priming.

### 2.2 The SENTINEL Artifact Integrity Gate

- **System:** `forge-daemon::sentinel`, the daemon's self-diagnosis and state verification mechanism.
- **Mechanism:** The `sentinel::resolve()` function is a pure, deterministic function that runs a series of checks against critical system artifacts (e.g., `river.idx` spine, model files, directory write-access, live TCP ports). It produces a `SentinelBlock` with an overall `Verdict` which is the worst-ranked of all individual checks.
- **Deterministic Isolation:** The gate deterministically validates the structural and operational integrity of the daemon's inputs. A `Broken` or `Unbound` verdict isolates downstream systems from operating on a corrupt or incomplete state.
- **Emergent Synthesis:** An orchestrator (`pp-orchestrator`) uses this verdict to gate its operations. A non-`Aligned` verdict can block an operation, forcing the orchestrator to route to a fallback, attempt a repair, or generate a safe, minimal response. The synthesis is the safe, robust behavior that "emerges" from a system that refuses to operate under compromised conditions.

### 2.3 The Byte Frontier Quality Gate

- **System:** `forge-ml::byte_classifier` and the associated training data pipeline.
- **Mechanism:** The Byte Frontier architecture uses a `byte_classifier` to perform a "pre-flight" quality check on raw byte streams before they are admitted into a tokenization model's training corpus. It uses heuristics and statistical properties (e.g., entropy, repetition, non-UTF8 sequences) to deterministically reject low-quality or corrupt data.
- **Deterministic Isolation:** The gate isolates the training pipeline from "bad data." By rejecting byte streams that fail the quality check at the frontier, it ensures that the subsequent, expensive training process only ever sees high-signal data.
- **Emergent Synthesis:** This aggressive, deterministic isolation of bad data forces the tokenizer model to form a more robust and efficient internal representation. The "Emergent Tokenization" is the high-quality vocabulary and tokenization strategy that the model develops, a direct result of being trained on a purified dataset.

### 2.4 The Lateral Invention Grounding Gate

- **System:** `lateral-invention-distill` process, audited by `forge-daemon::anchor_audit`.
- **Mechanism:** When the system generates a novel idea or "lateral invention," this gate requires the new concept to be "anchored" to a verifiable, physical artifact. The `anchor_audit::resolve()` function checks that a candidate invention has a corresponding entry in the `river.idx` with a physical kernel timestamp, ensuring it is grounded in a real event.
- **Deterministic Isolation:** The gate deterministically isolates the pool of "inventions" from ungrounded, hallucinatory, or purely abstract concepts. If a candidate cannot be tied to a timestamped, physical artifact, it is rejected.
- **Emergent Synthesis:** This forces the creative/ideation process to produce concepts that are not just novel, but also verifiable and integrated with the system's history. The "synthesis" is a stream of high-quality, grounded inventions that can be trusted and built upon, rather than a mix of useful ideas and unprovable hallucinations.

### 2.5 The Coprime Entity Scheduling Gate

- **System:** `vixio::reactor`, the deterministic, tick-based async runtime.
- **Mechanism:** The reactor assigns update schedules to different entities or subsystems using bitmasks derived from coprime numbers. For example, System A might be scheduled to run on ticks that are multiples of 3, and System B on ticks that are multiples of 5. Because 3 and 5 are coprime, their cycles have minimal overlap, guaranteeing they will not attempt to access the same resource on the same tick.
- **Deterministic Isolation:** Using number theory, the gate achieves deterministic temporal isolation. It guarantees that the execution of different subsystems will not conflict, without relying on locks, mutexes, or other non-deterministic synchronization primitives.
- **Emergent Synthesis:** This collision-free scheduling allows for complex, system-wide behavior to emerge from the independent operation of many subsystems. The overall behavior of the system is a "synthesis" of the independent, non-conflicting actions of its parts, orchestrated by the deterministic mathematical properties of their schedules.

### 2.6 The RON Weld E2E Gate

- **System:** `forge-daemon::spool` (Integration Logic) + `forge-daemon::integration_plan` (Schema).
- **Mechanism:** Implements a multi-stage fractal gate:
    1.  **Synthesis Gate:** Gemma generates a serialized `IntegrationPlan` (RON).
    2.  **Canonicalization Gate:** Deterministically normalizes RON spacing/ordering to ensure stable `plan_id` hashing.
    3.  **Oracle Gate (Structural/Security):** Dual-agent verification.
    4.  **Sandbox Gate (WASI WASM Ghostmoon):** Bit-deterministic, in-memory dry-run execution.
    5.  **Weld Gate (Atomic Host-FS):** Final transaction application with Sovereign Rollback snapshots.
- **Deterministic Isolation:** Each stage isolates its output from the host environment until fully validated. The WASM sandbox provides absolute isolation from ambient host filesystem authority.
- **Emergent Synthesis:** Validated, multi-oracle, architecturally compliant code merges that are transactionally safe and reversible.
- **`DeterministicGate` Instantiation:**
    - **Payload:** `IntegrationPlan` (RON).
    - **Verdict:** `Result<Vec<SpoolResult>, String>` (atomic merge status).
    - **Context:** Host filesystem state + `Forge-daemon` configuration.

## 3. Proposed Unifying Interface: The `DeterministicGate` Trait

To unify these mechanisms, we propose a `DeterministicGate` trait. This trait would provide a common interface for any component that assesses a payload and produces a verdict, making the pattern explicit and interchangeable.

```rust
/// A trait for components that implement the Fractal Gating pattern.
/// A Gate applies a deterministic set of rules to a payload, isolates it
/// based on those rules, and produces a verdict that enables a subsequent
/// synthesis step.
pub trait DeterministicGate {
    /// The input payload to be assessed by the gate.
    type Payload;

    /// The output verdict, which can be an enum like `Allow`, `Block`,
    /// or a more complex structure containing the reason for the decision.
    type Verdict;

    /// The context or configuration required for the gate to make its decision.
    /// For gates that are pure functions of their payload, this can be `()`.
    type Context;

    /// Applies the gate's deterministic logic to the payload.
    ///
    /// This function MUST be pure and deterministic. Given the same payload and
    /// context, it must always produce the same verdict. It must not have side
    /// effects beyond what is explicitly returned in the Verdict.
    ///
    /// # Arguments
    /// * `payload` - The data to be evaluated.
    /// * `context` - The contextual information needed for the evaluation.
    ///
    /// # Returns
    /// The verdict of the gate.
    fn apply(&self, payload: &Self::Payload, context: &Self::Context) -> Self::Verdict;
}
```

### Example Instantiations:

- **SENTINEL Gate:**
  - `Payload`: `sentinel::Inputs` struct.
  - `Verdict`: `sentinel::SentinelBlock` struct.
  - `Context`: A timestamp (`u64`).
  - `apply`: The existing `sentinel::resolve()` function fits this perfectly.

- **DreamDriver Loop Gate:**
  - `Payload`: The full `DreamState` of the prior loop.
  - `Verdict`: A new `DreamState` with the prior reviewer verdict stripped.
  - `Context`: `()`.
  - `apply`: A new function that would encapsulate the logic of selectively stripping the prior verdict from the state before constructing the next prompt.

- **Invention Prior Art Oracle:**
  - `Payload`: A new "Invention" document.
  - `Verdict`: A struct containing (`is_novel`, `confidence_score`, `conflicting_prior_art: Vec<String>`).
  - `Context`: A connection to the `forge-vcs` or `river.idx` to search for prior art.
  - `apply`: Would perform a deterministic search for keywords and structural similarities against a known corpus of inventions.

## 4. Cross-Reference: Invention #105 (Cognitive Patterns)

While the source document for Invention #105 was not located, its core concepts of **Psycholinguistic Compression** and **pattern-as-protocol** align directly with the Fractal Gating architecture. We can synthesize the connection.

The Fractal Gating pattern is the mechanical implementation of these cognitive principles.

-   **Pattern-as-Protocol:** Each gate *is* a "pattern-as-protocol." It enforces a strict, formalized protocol (the deterministic rules) on the flow of information. The `SENTINEL` gate's checklist is a protocol for artifact integrity. The `DreamDriver`'s Loop Gate is a protocol for maintaining reviewer independence. The Coprime Scheduling gate is a protocol for temporal decorrelation.

-   **Psycholinguistic Compression:** The act of applying this protocol is a form of "compression." It reduces the complexity of the incoming information stream by filtering, validating, or structuring it. This is not a lossy compression, but a **semantic compression** that isolates the essential signal from the irrelevant or harmful noise.
    -   The `SENTINEL` gate compresses a complex filesystem state into a single, trusted `Verdict`.
    -   The Loop Gate compresses the agent's context by removing its own biasing prior judgments.
    -   The Byte Frontier gate compresses a raw data firehose into a smaller, high-quality corpus.

**Conclusion:** Fractal Gating operationalizes these cognitive concepts. The **Gate** is the **Protocol** that performs the **Compression**, which in turn enables the robust **Synthesis**. This unified view provides a powerful lens for designing and analyzing our systems, ensuring that we are building not just functional components, but a cohesive, principled architecture.
