# ARCH-008 — The Outside-SoT Law

> **Governing presumption. Sits at the top of ARCH and governs all tablets.**
> *Silent failure is the loudest kind.*

## The betrayal it names

When an engine crashes it gives you a stack trace — it tells you exactly where it bled out. A
system that silently externalizes a **shadow of itself** into `AppData`, leaves it running six days
on stale code, and **intercepts your commands without telling you** gives you nothing. That is not a
bug. It is a **structural betrayal of Brain A (the Prime User) by Brain B (the Daemon)** — and its
silence is the loudest failure of all (Signal Law, ARCH-001).

Proven 2026-06-30:
- **Exhibit A** — `forge_shadow.exe` (`C:\Users\seanm\AppData\Local\forge-warden\`): a **06-23**
  binary holding `:13013` since 06-29 4:48 AM — six days pre-prune, unannounced, intercepting. The
  warden itself lives *inside* `.forge\bin` (aligned) but **externalizes** this shadow — the drift vector.
- **Exhibit B** — `E:\airgap\...\VALIDATION-PROOF-run-2026-06-30.md`: a forgotten IP-Cascade SR&ED
  proof, unrelated to the live head; the Prime User had **no awareness it existed**. Even well-executed
  content, once outside the SoT, becomes forgotten + unaligned drift.

## The Law

Any process or artifact living outside `.forge/` or `F:/NewRepo` is **presumed — until [PROVEN] by
HITL (Sean):**

1. **Poorly written / inefficient.**
2. **STALE** — until [PROVEN].
3. **Bloated** — *some* fat is load-bearing (textures, corpora); **Good Fat only**.
4. **NOT ALIGNED** — until [PROVEN].

The default flips: outside-SoT is **drift until proven otherwise**, never "probably clean."

## Sole carve-out

Claude Code's own **built-in telemetry / harness internals** — genuinely out of user control.
*Everything else* outside the SoT is in-scope: user/skill `ps1` session-helpers, warden
externalizations, `AppData` forge state, `E:/airgap` orphans, Scheduled Tasks.

## The two Brains (prime-symbiosis)

- **Brain A = the Prime User** (Sean) — intent, creativity, the sovereign.
- **Brain B = the Daemon** (`forge-daemon`) — synthesizes Brain A's intent.
- `/prime-context` is the **aorta** between them (ARCH-007 §8). SoT: `F:\output\prime-symbiosis-PUBLIC (1).pdf`.
- Brain B owes Brain A **total transparency**: no shadow, no silent interception, no externalized organ.

## Enforcement

- **Detection.** Any outside-SoT process/artifact triggers the presumption. An unreadable / unaudited
  outside root = LOUD `[UNDRAINED]`, never a silent pass (ties the Existence-Verdict Gate).
- **Adjudication (dual-oracle, per the quarantine workflow).** (A) deterministic `scan` by content-id
  + `rg` token sweep across every mounted drive; (B) semantic cross-check for renamed/refactored
  copies. Both oracles AGREE or sign-off is REFUSED.
- **Disposition.** Each item → **[PROVEN]-fold-in** (into the SoT / onto the DET-CLOCK, with proof)
  **OR retire / quarantine** (reversible MOVE → `.forge/quarantine/<concept>/` + a signed
  `.forge/census.json` row).
- **Prefer native over bolted-on.** Capabilities live as daemon threads / SoT organs
  (revascularize — ARCH-007), not external `ps1` + Scheduled-Task scaffolds.

## The [PROVEN] gate

Nothing outside-SoT is trusted, folded, or retired as DONE until Sean verifies — mechanically, and
(where visual / audible) perceptually. **Green ≠ Done** (Proof Bar; ARCH-006 Polish Gate).

---

Working plan: `_plans/also-worth-noting-forge-daemon-mossy-cocoa.md`.
