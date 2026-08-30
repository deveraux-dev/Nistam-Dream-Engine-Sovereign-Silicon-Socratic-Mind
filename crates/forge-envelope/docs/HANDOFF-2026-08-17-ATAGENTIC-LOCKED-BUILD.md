# HANDOFF — All Things Agentic Hackathon: LOCKED build plan

**Deadline:** Devpost submission closes **Aug 31, 2026, 5:00 PM PT** (~14 days).
**Verdict authority:** `.forge/cascades/2026-08-17-atagentic-entry/K07b-LOCK.md` — LOCKED by Sean.
Everything below is receipt-backed; anything you add must be too. Truth law is absolute:
never claim a capability the code cannot demonstrate. Purged-forever claims: candle-core-in-this-
crate, Malachite BFT, 3×ternary-Gemma-as-fact, "Timeless Compression" as shipped. Those live ONLY
in a clearly-labeled Research Roadmap section.

## THE ENTRY (one paragraph)
"Surface Ledger" — Taskmaster category, Sean's first-person Sovereign Community voice. An
autonomous inspection auditor on Cloud Run: photos in → deterministic byte-sieve triage → local
Gemma 4 E2B vision triage → Gemini 3.7 Flash schema-locked audit → cross-check vs the closed-form
degradation model → forge-envelope attests the disposition into a rolling SHA-256 EvidenceChain →
32-byte sharded heads in Firestore → cloud staging wiped ONLY after Firestore ack. Raw evidence
stays with the inspector (zero-CLOUD-retention). Operator actions are self-attested onto the SAME
chain (accountability as self-attestation, not observation — Sean's framing, keep it).

## WHAT IS ALREADY DONE (do not redo)
- forge-envelope crate: 28 unit + 1 scale + 1 doc test green. Core is publishable-quality.
- Trinity locked: Gemma 4 E2B (multimodal, Apache 2.0, verified released 2026-03-31) +
  2× Gemma 3 270M QAT ≈ 1.55GB Q4 < 2GB. Gemini 3.7 Flash = cloud oracle (verified released
  2026-08-13). Floor beneath models: byte_sieve.s13 (72 bytes) + --manual identical-path trigger.
- Byte-frontier port: sidecar\src\ml\{byte_classifier,byte_corpus}.rs — 13/13 tests green.
- Passive training flywheel LIVE: .forge\train\flywheel\{capture-route,seed-route-pairs,
  pairs-to-dataset,flywheel-tick}.ps1 → trained candidate router-gate-weight-20260817-154740.s13
  (1115B, magic OK) from 38 real session labels. Promotion to canonical = Sean's act only.
- Site scaffold: crates\forge-envelope\surfaceledger\ is a deployable static root; deploy doctrine
  in web\DEPLOY.md (Cloudflare = marketing face only; judges need the GCP .run URL).
- Drafts (triaged, in .forge/cascades/2026-08-17-atagentic-entry/): D1 agent loop (needs fixes
  below), D2 deploy pack (GOOD — use as written), D3 README (fix proof-states), D4 video script
  (GOOD), D5 blog+social (one path fix). SUBMISSION_ENTRY.md is SEAN'S first-person document —
  never draft or edit it for him.

## THE BUILD QUEUE, IN ORDER
1. **`crates\forge-envelope\src\bin\attest.rs`** (~60-100 lines, THE first weld). D1's draft
   invented `--expect/--attest` CLI flags — no CLI exists yet. Build it: file-path or stdin JSON
   in → EphemeralEnvelope seal → EvidenceChain append → JSON {link_hash, prev, tick, disposition}
   on stdout. TWO event classes on one chain: asset events AND operator events (tag inside sealed
   payload; operator event precedes the audit it triggered). Batch mode (many records, one
   process) — C2's subprocess-tax ruling. Tests + L18 sabotage check.
2. **Model bump:** scripts\vertex_schema_client.py:34 → "gemini-3.7-flash" (one line).
3. **agent_loop.py:** start from D1 draft; fix: (a) call the real attest binary from step 1;
   (b) divergence path must WRITE a receipted escalation record, never log-and-return (that is
   the purgatory bucket — no third unrecorded state may exist); (c) staging = tmpfs (STAGING_DIR
   env per D2 Dockerfile), wipe only after Firestore ack; (d) --manual runs the identical path
   and lands an operator event on the chain.
4. **Gemma serving:** E2B + 2×270M inside the container (llama.cpp/ollama server — choice was
   left [ASSUMED]; pick one, state it). If it fights you past a day, ship the deterministic sieve
   floor and move the trinity to "landing this week" honestly — the loop must never depend on it.
5. **Deploy (by DAY 4 of the sprint):** D2 pack as written — Cloud Run 4GB min-instances=1,
   northamerica-northeast1, 16-bucket sharded chain_heads Firestore, minimal IAM. Sean has
   1200+ GCP credits; cost is not a constraint.
6. **README:** D3 draft with proof-states corrected — nothing is "LIVE" until deployed and
   witnessed; trinity is IN-SCOPE (not Roadmap); disclosure section: crate authored Aug 15-17
   2026 inside the v3 tree (inside the Aug 3-31 window; disclose lineage anyway). License file
   visible at repo top (MIT OR Apache-2.0 already in Cargo.toml).
7. **Video by DAY 11** (not 13): D4 script as written. Real console only. If latency bites, let
   the logs scroll — it proves live.
8. **Bonus stack:** blog + social from D5 (fix: candidate .s13 lives in .forge\train, not the
   repo). Gemma bonus = +0.2 once (per FAMILY, three instances still one bonus). Lyria +0.2
   ONLY if days 1-10 land early. Veo: skip. Max realistic Stage-3 = +0.8 incl. blog/social.
9. **Tee the flywheel:** any wave/loop you run, call
   `.forge\train\flywheel\capture-route.ps1 -Query <task> -Tier <0|1|2> -Outcome <what stood>`.

## HARD GOTCHAS (each cost us a bounce today)
- gemini CLI headless needs `GEMINI_CLI_TRUST_WORKSPACE=true` + cwd F:\v3; PS 5.1 mangles
  UTF-8 scripts (em-dashes) — use pwsh; `$LASTEXITCODE` is stale after PS-script children —
  verify artifacts, not exit codes.
- Foreman hook blocks `[System.IO.File]::` patterns near gated paths — use Format-Hex /
  Get-Content -AsByteStream for binary reads.
- sidecar is its OWN workspace: `cargo test --manifest-path F:\v3\sidecar\Cargo.toml --bin
  gemma-sidecar`. forge-envelope builds from repo root (`-p forge-envelope`).
- .forge\distill is write-gated for file tools; append via PowerShell Add-Content only.
- E:\ and quarry roots are read-only tape (never "newer-wins", never delete). Two S13s exist:
  the [i8;13] lane vector (this crate) and the sentinel bytes 243-255 (forge-core-v3\src\s13.rs)
  — never conflate them in docs.
- L21: state the smallest diff, then yield. L23: STATIC and RUNTIME are separate lines; "landed"
  requires a runtime receipt. Sean's word locks decisions; nothing self-closes.

## THE STORY SPINE (for any prose you touch — Sean's voice rules)
Human error, never computational rounding error: the arithmetic is integer and specified to the
last truncation — the only questions the ledger leaves open are human ones, and the operator
trail answers those by self-attestation. Rounding exists; rounding ERROR doesn't. LLM output is
testimony, not computation — recorded, cross-checked, sealed. Evidence side: no ending is silent.
Memory side (roadmap): no forgetting is free (spcc.rs). 23 years, Walterdale Bridge, Cree
developer, sovereignty for the small actor — Sean writes that part himself.
