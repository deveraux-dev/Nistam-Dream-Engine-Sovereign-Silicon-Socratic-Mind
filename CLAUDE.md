# NISTAM — demo/submission tree. THE COMPETITION: Devpost "All Things Agentic" (allthingsagentichackathon.devpost.com)
- DEADLINES: D1 Aug 28 12:00 PM PT — Google Cloud $150 credit form. D2 Aug 31 5:00 PM PT — submission closes.
- MANDATORY STACK: Gemini 3.5+ (Vertex) · agent framework = Antigravity (Gemini 3.7 drives the Forge Engine) · ≥1 Google Cloud service (Vertex caching / Cloud Run / Firestore, project nde1-493505) · autonomous agent BEYOND chat, deployed.
- RECEIPTS for the stack: `cargo run -p forge-daemon-door --bin door -- <verb>` (Antigravity's steering wheel, 59 verbs) · crates/forge-envelope/scripts/agent_loop.py (Cloud Run evidence flywheel) · scripts/test_sovereign_airgap_red_green.py (5/5 red vectors blocked).
- DELIVERABLES: Devpost form · README · repo access (testing@devpost.com + cloudhackathons@google.com if private) · architecture diagram IN REPO · demo video ≤4 min, English subtitles · GCP backend proof.
- NAMING (single vendor): judge-facing text says **the Forge Engine** (daemon :13013; inference = verb 9, same mouth as ast/cst/lsp/dsl). Never "sidecar/bridge/shim".
- TWO TREES: F:\v3 is source of truth — land+gate there, sync file-scoped here. `cargo test --workspace` does NOT reach `crates/studio-tauri` or `shell/` (firewalled): gate each separately.
- G8: pre-existing work disclosed in docs/SUBMISSION_ENTRY.md. Benchmarks: ship MEASURED numbers only (2.75 Mtok/s single-core; 6.42 Gtok/s is marked WRONG-SUPERSEDED).
- Full rules digest: C:\Users\seanm\Desktop\hackathon_archive\proofs_and_notes\Rules-Benchmarks.md · plan tracker: PROOF_TRACKER.md same dir.
