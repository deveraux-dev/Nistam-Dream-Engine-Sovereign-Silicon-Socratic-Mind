# ARCH-015 — FAN-OUT LAW

> Graduated 2026-07-10, Sean-named (/workflow codify). Doctrine twin: `_plans/DOCTRINE-6IN1.md` E6 ROLLER.
> Every receipt below tagged per Truth Ladder. No mask.

## §1 Mutation contracts only

Subagents receive MUTATION CONTRACTS only. Reconnaissance never fans out.
Scans run inline in the conductor's own context (rg sweeps).

Receipt [PROVEN]: 8-agent scan wf_f1b3906a-1a9 = 4,056,956 tokens processed
(352,915 uncached in · 34,093 out · 1,759,250 cache-read · 1,910,698 cache-write,
traced from 9 transcript jsonl), 0 receipts returned (journal: 8 started / 0 completed),
0 mtime movement on the repo. Inline redo of the same recon = ~12 greps, same session.

## §2 Pre-flight gate before ANY fan-out — all four pass or it runs inline

a. Lanes bounded and file-disjoint — zero overlap in touched files.
b. Design-heavy per lane — mechanical batches stay inline.
   Receipt [PROVEN]: $0.015/edit at 15 batched edits beats fan-out (ledger 07-10 throughput row).
c. Directive-cap declared BEFORE launch: agent count × context ceiling × tool budget,
   written to the board.
d. Exit per lane: byte-green `cargo check -p && cargo test -p`, else full rollback.

Receipt [PROVEN] (the gate passing, same day): 3-agent WIDE-GLUE pass = 249,444 subagent
tokens → 10 files mutated across 4 crates + root in a 4m41s disk window (mtimes 14:22:55→14:27:36),
354 green test executions, 3× EXIT 0 (board RESULTS, ledger row 07-10).

## §3 Metric of record: receipts per token

Cache reads are not free — N contexts re-reading one repo is the same knowledge
bought N times, kept by nobody.

Receipt [PROVEN]: 3.67M cache tokens across 8 scanner contexts, zero knowledge kept
(wf_f1b3906a-1a9 transcript parse, 2026-07-10).
