# ARCH-016 — THE SOVEREIGN TRIAGE & THE ASPIRE MATRIX

**STATUS:** CANONICAL ARCHITECTURE
**SUBSYSTEMS:** `aspire.rs`, `cargo xtask triage-check`, `board_status.json`

---

## 1. THE DRIFT VULNERABILITY
In a sovereign monolithic engine, architectural intent must remain tightly bound to on-disk reality. Historically, a gap existed between the static `ASPIRE` capability array (`crates/forge-book/src/aspire.rs`) and the actual implementation state scattered across crate unit tests and `board_status.json`. This allowed capabilities to "land" without the central matrix registering the completion, causing silent metadata drift.

**Known truth:** The compiler must physically verify progress. Manual comment adjustments (e.g., `// LANDED 07-21`) are inherently fragile and MUST NOT be the primary tracking mechanism.

## 2. THE ASPIRE MATRIX (INTERNAL INTENT)
The `ASPIRE` array is the single, hardcoded source of truth for future-forward capabilities. It operates strictly on a 30-item capacity limit.
*   **Buckets:** `NOW`, `NEXT`, `LATER`, `HORIZON`, `EDGE`
*   **ROI (Return on Investment):** `H` (High), `M` (Medium), `L` (Low), `E` (Exploratory)
*   **Constraint:** To add a new capability to the array, an old one must be promoted to the compiled `catalog.rs` or explicitly deprecated.

## 3. THE SOVEREIGN TRIAGE PIPELINE
To permanently fuse intent with physical execution, the engine relies on the `cargo xtask triage-check` pipeline. This tool operates completely read-only, maintaining the zero-chatter, compile-time speed of the Living Atlas.

### The Execution Loop:
1.  **The Matrix Import:** The `xtask` dynamically imports `forge_book::aspire::ASPIRE`.
2.  **The Ledger Cross-Reference:** It scans the active `board_status.json` and the `catalog.rs` receipt registry.
3.  **Automatic Status Injection (The Ratchet):** If a capability listed in `aspire.rs` is fully verified by a matching `[BOARD:]` tag or a compiled capability receipt, the tool flags a success. 
4.  **The Output:** It outputs a future-forward drift report. If a task is physically complete but marked incomplete in the matrix, the `xtask` outputs a diff proposal to update the static array automatically, closing the loop.
