# 05_BARE_METAL_HOOK_ISOLATION: cmd.exe Fail-Open and Build Lock Elimination

**Specification Version:** 1.0.0  
**Status:** Canonical Spec  
**Classification:** OS Substrate / Process Isolation / Runtime Hardening  

---

## 1. Executive Summary & The Windows Substrate Problem

On bare-metal Windows execution targets, multi-process orchestration faces two critical failure modes:
1. **Subprocess Hanging & Deadlocks**: Interactive console subprocesses (`cmd.exe`, `powershell.exe`) blocking on unconsumed stdin/stderr or unhandled child handles.
2. **File & Binary Build Lock Contention**: Antivirus scanners, indexing services, and running executables holding exclusive file locks (`ERROR_SHARING_VIOLATION`), blocking compiler output (`cargo build`, `xtask`, `link.exe`).

The **Bare-Metal Hook Isolation** architecture eliminates these failure modes through three hardware-grade primitives:
- **Fail-Open Process Harness**: Non-blocking asynchronous I/O with hard execution deadlines and automatic orphan reaping.
- **Zero-Lock Staging Pipeline**: Ephemeral staging directories with atomic swap semantics (`MoveFileExW`).
- **Visual-on-Glass IPC**: Real-time telemetry broadcasting over local socket `127.0.0.1:13013` to `HUD.html`.

```
        +-------------------------------------------------------------+
        |                 ORCHESTRATOR / XTASK RUNNER                 |
        +-------------------------------------------------------------+
                                       |
                   +-------------------+-------------------+
                   |                                       |
                   v                                       v
   +-------------------------------+       +-------------------------------+
   |      FAIL-OPEN SUBPROCESS     |       |    ZERO-LOCK STAGING VAULT    |
   |  - Non-blocking async stdout  |       |  - Write to ephemeral stage   |
   |  - Hard 1200ms deadline guard |       |  - Atomic rename on success   |
   |  - Ghost Reaper orphan scrub  |       |  - Wipe on receipt ACK        |
   +-------------------------------+       +-------------------------------+
                   |                                       |
                   +-------------------+-------------------+
                                       |
                                       v
                       +-------------------------------+
                       |    TELEMETRY EMISSION (IPC)   |
                       |    TCP / WS 127.0.0.1:13013   |
                       |     -> HUD.html (On-Glass)    |
                       +-------------------------------+
```

---

## 2. Fail-Open Process Harness & Ghost Reaper (IMPLEMENTED)

### 2.1 Non-Hanging Bounded Polling Execution Protocol
Every spawned child process must be wrapped in a non-hanging bounded execution envelope:
1. Stdio streams (`stdin`, `stdout`, `stderr`) are piped into bounded reader loops.
2. An absolute hardware timer deadline is armed upon launch (e.g., $T_{\text{deadline}} = 1200\text{ ms}$).
3. If the process completes within $T_{\text{deadline}}$, status `OK` is returned and stdout is parsed.
4. If the timer fires before exit, the **Ghost Reaper** (forge-daemon-door::ghost_reaper, src/ghost_reaper.rs) immediately dispatches `TerminateProcess` / `taskkill /F /T /PID` to purge the entire process tree, writes the input intent to `.forge/hook-snapshots/objects/<hash16hex>` with status `FALLBACK_ANCHOR`, and yields control back to the caller in $< 5\text{ms}$.

### 2.2 Ghost Reaper Invariant
No orphaned worker threads or detached background processes are permitted. Unclaimed processes older than their declared deadline are systematically reaped by `forge-daemon-door::ghost_reaper`.

---

## 3. Future Work: Zero-Lock Staging Pipeline (specification only)

**Status**: The following architecture describes ephemeral staging and atomic commit semantics for eliminating build lock errors on Windows. This is specification-only; no implementation currently exists in the codebase.

To completely ban `LNK1104` / `Access Denied` build lock errors on Windows, a proposed design:

### 3.1 Ephemeral Target Namespacing
Compilers and code generators would write to ephemeral staging paths rather than destination binary paths (`target/release/app.exe`). Proposed path scheme:
$$\text{Path}_{\text{stage}} = \text{target/stage\_}\langle\text{PID}\rangle\_\langle\text{timestamp}\rangle\text{/app.exe}$$

### 3.2 Atomic Commit via `MoveFileExW`
Upon compilation or artifact completion:
1. Destination file would be replaced atomically using `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`.
2. Staging directories would be wiped immediately upon receipt acknowledgment, satisfying the zero-retention invariant (ADR-0026 / G20).

---

## 4. Visual-on-Glass Law (HUD Telemetry Contract)

**Fundamental Invariant:** Zero silent or background jobs. If an execution tick, status change, or IPC pulse is not rendered on glass in `HUD.html`, it does not exist.

### IPC Packet Schema (`127.0.0.1:13013`) (IMPLEMENTED):
```json
{
  "tick": 4812,
  "status": "RUNNING | FALLBACK | FAULT",
  "ipr": 8420,
  "progress": 33.3,
  "tps": 142.5,
  "log": "Gemma S13 token stream active | GBNF mask valid"
}
```

The `tps` field (tokens per second, float Hz) is emitted by forge-mud-v3/src/organs/nde_chat.rs::broadcast_hud_telemetry().

Every execution harness in `xtask` or `nde_chat.rs` must stream this payload on every state transition.
