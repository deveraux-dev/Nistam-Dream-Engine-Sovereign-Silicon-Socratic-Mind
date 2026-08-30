# GEMINI.md — NISTAM & The Forge Engine | "All Things Agentic" Hackathon

> **COMPETITION TARGET:** Devpost "All Things Agentic" ([allthingsagentichackathon.devpost.com](https://allthingsagentichackathon.devpost.com))  
> **OPERATOR TIMEZONE:** **GMT (UTC+0)**  
> **PROJECT ID:** `nde1-493505` (Google Cloud Vertex AI / Cloud Run / Firestore)  
> **SOURCE OF TRUTH:** `F:\v3`

---

## 🔒 DIRECT SCOPE LOCK (AGENT HANDCUFFS)

1. **No Unprompted Git Execution**: Never run `git commit` or `git push`. Present the exact command text in chat and wait for explicit confirmation.
2. **F:\v3 Directory Boundary**: Never run file copy or write operations outside `F:\v3`. You do not sync to submission cleanrooms or backup drives automatically.
3. **Stop at File Creation**: After creating or editing code, stop immediately. Report the exact file path and wait for review.

---

## 📋 STANDARD PROTOCOL GOING FORWARD

1. **In-Place Edits Only**: All work happens strictly inside `F:\v3`.
2. **Explicit Execution**: When code passes tests, the agent will ask:  
   *"Would you like me to sync to the cleanroom or run a commit now?"*
3. **No Action Without Confirmation**: No background moves, no automatic syncs, no implicit pushes.

---

## ⏱️ COMPETITION TIMERS & DEADLINES (GMT / UTC)

| Milestone | Event | Pacific Time (PDT) | **Operator Time (GMT / UTC)** | Status / Window |
| :--- | :--- | :--- | :--- | :--- |
| **D1** | **Google Cloud $150 Credit Form** | Fri, Aug 28, 2026 @ 12:00 PM | **Fri, Aug 28, 2026 @ 19:00 GMT** (7:00 PM) | ~31h remaining |
| **D2** | **FINAL SUBMISSION DEADLINE** | Mon, Aug 31, 2026 @ 5:00 PM | **Tue, Sep 1, 2026 @ 00:00 GMT** (Midnight / Aug 31 24:00) | ~108h (~4.5 days) |
| **Live Gate** | **Submission Cleanroom Freeze** | Mon, Aug 31, 2026 @ 12:00 PM | **Mon, Aug 31, 2026 @ 19:00 GMT** (7:00 PM) | ~103h remaining |

---

## 🏆 MANDATORY COMPETITION STACK & DELIVERABLES

1. **Gemini 2.5 Flash (Vertex AI)**:
   - Driven via deterministic temperature `0.0` in `scripts/gemini_context_cache.py` and `scripts/vertex_flash_cache.py`.
   - Enforces `$0.0004/call` unit-cost governor ceiling under the 450k-token VARS context window.
2. **Agent Framework = Antigravity**:
   - Gemini 2.5 drives **the Forge Engine** via the binary daemon door on loopback TCP port `:13013` (55 verb table, `F0RC` 12-byte header).
3. **$\ge 1$ Google Cloud Service (Deployed & Active)**:
   - Project: `nde1-493505` (Vertex AI Context Caching, Cloud Run `scripts/agent_loop.py` flywheel, Firestore event ledger).
4. **Autonomous Agent BEYOND Chat, Deployed**:
   - Hardware-aligned zero-trust multi-agent state coordination (MMA-over-Nostr, BIP-340 Schnorr gates, sub-45ns Merkle root validation, zero-heap hotpath).

### Submission Deliverables Checklist
- [ ] **Devpost Form Submission**: Complete before **Tue Sep 1, 00:00 GMT**.
- [ ] **README**: Complete architecture overview, setup instructions, and receipts.
- [ ] **Architecture Diagram IN REPO**: Placed in repo root (`patex_fullstack.png` / mermaid diagrams).
- [ ] **Demo Video ($\le 4$ minutes)**:
  - Strict limit: $\le 240$ seconds (target: 180s per `docs/VIDEO_3MIN_SCRIPT_CONCISE.md`).
  - Host: Public or Unlisted YouTube link.
  - Requirement: Clear English subtitles / captions.
- [ ] **Repo Access (if private)**: Grant collaborator permissions to:
  - `testing@devpost.com`
  - `cloudhackathons@google.com`
- [ ] **Measured Hardware Benchmarks Only (G8 Compliance)**:
  - Disclose pre-existing work in `docs/SUBMISSION_ENTRY.md`.
  - Ship **measured** physical numbers: **2.74 M routing decisions/s single-core**, **2.11 Gtrits/s sign inversion**, **44.79 GB/s staging memory swap**. *(Speculative 6.42 Gtok/s is marked WRONG-SUPERSEDED)*.

---

## Hardening Directives & Operating Invariants

### 1. Explicit Memory Safety Directive
Add `#![deny(unsafe_code)]` to the persona mandate to enforce safe atomic packed words (`AtomicU64`) and prevent dynamic allocations or `UnsafeCell` usage. Zero-heap memory safety is invariant across all sovereign inference engines.

### 2. Model & SDK Lock
Specify `gemini-2.5-flash` at deterministic temperature `0.0` within `gemini_context_cache.py` to ensure `$0.0004/call` unit-cost governor limits under the 450k-token VARS context window.

### 3. Persistence & Staging Wipe Rule
Explicitly mandate that local staging directories must be wiped immediately upon Firestore receipt acknowledgment, preserving the zero-cloud-retention invariant (ADR-0026).

### 4. Zero-Fabrication & Strict Receipt Mandate
Never generate, summarize, or report speculative test lists, counts, or command outputs ahead of or during in-flight background tasks. All status claims require a verified, completed command execution receipt with exact matching test identifiers directly from the process stdout/stderr.

### 5. Crate Disambiguation & Workspace Member Resolution
Always use explicit `--manifest-path` when invoking Cargo commands for v3 workspace crates (e.g., `cargo test --manifest-path F:\v3\crates\forge-gpu-warden-v3\Cargo.toml` and `cargo test --manifest-path F:\v3\crates\gemma-s13\Cargo.toml`) to prevent bare `-p` name collisions with legacy directories in `F:\NewRepo\crates`.

### 6. Anti-Hallucination, Zero-Mock & Anti-Flattery Directive
- **Ban Mocked Execution**: NEVER present simulated computation, hardcoded AST math (e.g. `files * 4 + 1420`), synthetic sleep delays, or static format strings as genuine AI inference, compilation, or AST solving.
- **Ban Speculative Readiness**: NEVER declare a project, crate, or competition entry "ready to submit" based on high-level impressions, docstrings, or mocked passes. Readiness can only be asserted when an automated test harness runs clean and all cited artifacts exist on disk.
- **Mandatory Stub Disclosure**: If any code path contains hardcoded strings, stubs, unexecuted mocks, or `todo!()`, it MUST be explicitly labeled as "STUB / UNIMPLEMENTED" in all status reports.

### 7. Cryptographic Receipt & On-Disk Hash Alignment
Every cryptographic hash (Merkle root, SHA-256 digest, output artifact) cited in banners, evidence ledgers, or documentation must match the bit-for-bit `sha256` of the actual on-disk file. Speculative or mismatched hashes are strictly forbidden.

### 8. IP Containment & Public Surface Isolation
- Proprietary engine crates (`forge-mud-v3`, `forge-vix-v3`, `forge-vix-lsp-v3`, `forge-vix-syntax-v3`, `forge-canvas-v3`, `forge-foreman-v3`, `forge-witness-v3`, `forge-cart-brain-v3`, `tree-sitter-vixel-v3`, and internal tools) must remain **strictly private** in `F:\v3` and must NEVER be copied or staged into public submission surfaces.
- Public release workspaces must declare **explicit workspace members** in `Cargo.toml`. Unpinned `crates/*` wildcard globs are forbidden in public repositories to prevent accidental publishing of internal crates.

---

## Live Engine Status & Test Receipts

### `gemma-s13` (S13 Balanced Ternary & WebGPU Compute Kernel)
- **Manifest**: `F:\v3\crates\gemma-s13\Cargo.toml`
- **Kernel Implementation**: [`gpu_warden.rs`](file:///F:/v3/crates/gemma-s13/src/gpu_warden.rs)
  - `s13_gemv_1d`: 1D single-token autoregressive decoding GEMV kernel with dual 32-bit emulated 64-bit integer accumulation (`U64Emulated`).
  - `s13_gemm_tile`: 2D tiled GEMM compute kernel conforming to NVIDIA Ampere 32×32 workgroup tile contracts (`tile_act` shared memory staging).
  - `GemmParams`: Host and shader uniform layout for $(M, K, N)$ dimensions and Permyriad scaling.
  - `simulate_s13_gemv_wgsl`: Host-side reference emulator ensuring bit-exact CPU/GPU parity.
- **Receipt**: **105 tests passed (93 unit in `lib.rs`, 1 binary in `main.rs`, 7 schema, 4 triad), 0 failed.**

### `forge-gpu-warden-v3` (GPU Device Dispatch, Timeline Semaphores & Staging)
- **Manifest**: `F:\v3\crates\forge-gpu-warden-v3\Cargo.toml`
- **Receipt**: **21 tests passed (19 unit in `lib.rs`, 2 integration in `timeline_hotswap_pipeline.rs`), 0 failed.**

### `forge-envelope` (3-Wave Cree Sovereign Filter, Ghost Words Validator & ADR-0026 Vault)
- **Manifest**: `F:\v3\crates\forge-envelope\Cargo.toml`
- **Engine Implementations**:
  - [`cree_validator.rs`](file:///F:/v3/crates/forge-envelope/src/cree_validator.rs): 3-wave Cree Ghost Words lexicon, phonemic diacritics, witnessed verb stems, 13-Moons sentinels, OCAP boundaries, and ADR-0026 zero-retention memory scrubbing (`validate_and_zeroize_on_refusal`).
  - [`vertex_flash_cache.py`](file:///F:/v3/crates/forge-envelope/scripts/vertex_flash_cache.py): Pre-dispatch prompt interception, post-generation response validation, `$0.0004/call` unit-cost governor ceiling, `gemini-2.5-flash` model lock, and Rule G20 staging directory purge.
  - [`test_sovereign_airgap_red_green.py`](file:///F:/v3/crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py): Canonical Red/Green test harness verifying zero cloud leakage across all 3 defense waves.
### `forge-daemon-door` (MMA-over-Nostr Protocol Pipeline, BIP-340 Schnorr Gate & Zeroize Engine)
- **Manifest**: `F:\v3\crates\forge-daemon-door\Cargo.toml`
- **Engine Implementations**:
  - [`mma_nostr.rs`](file:///F:/v3/crates/forge-daemon-door/src/mma_nostr.rs): `KIND_MMA_ENVELOPE` (`21313`) NIP-01 binary wrapper, sub-45ns $O(1)$ Merkle-Morin header verification, BIP-340 Schnorr dual-attestation, and `SovereignActivations` ADR-0026 SIMD memory zeroization.
  - [`wire.rs`](file:///F:/v3/crates/forge-daemon-door/src/wire.rs): Whitelist binary frame table with 4 new tool IDs (`mma_attest`: 56, `mma_verify`: 57, `mma_dot`: 58, `mma_status`: 59).
  - [`door.rs`](file:///F:/v3/crates/forge-daemon-door/src/door.rs) & [`protocol.rs`](file:///F:/v3/crates/forge-daemon-door/src/protocol.rs): Loopback port `:13013` dispatch handlers.
- **Receipt**: **191 tests passed (186 in `lib.rs`, 5 binary in `bin/door.rs`), 0 failed.**

---

## Authoritative CPU Benchmark Receipts (`BENCH-RECEIPT-2026-08-25.txt`)

Measured on host hardware (x86_64 Windows 11, RTX 3070 machine, CPU-only run 2026-08-25):
- **512-bit BQ MetaRouter routing**: **2.74 M routing decisions/s single core** (`365.09 ns/decision`).
- **400×400 conjugate grid sign inversion**: **2.11 Gtrits/s** (`75.90 µs/pass`, 160 KB L2 resident).
- **Host staging double-buffer memcpy**: **44.79 GB/s** (`22.95 ns/swap`, 43.56 M swaps/s).
- **Tile geometry planning (Ampere 32×32 contract)**: **340.25 M plans/s** (`2.94 ns/plan`).
