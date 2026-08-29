# Nistam Dream Engine — Devpost

**One-liner:** A sovereign 1.58-bit balanced-ternary inference engine that runs a
three-seat quantized Gemma fleet (2B / 3.2B / 9B) in deterministic lockstep on one
consumer GPU, governed in the cloud by Google Cloud Vertex AI gemini-2.5-flash (enforcing
a strict $0.0004/call unit-cost governor ceiling) — and wearing a playable face: a Tauri
v2 shell that flies the 119,625-star HYG sky, births a character through the Thirteen
Tolls, and walks a CYOA arc to the Toll Gate.

![Nistam Dream Engine & the Forge Engine — Full-Stack Sovereign Architecture](patex_fullstack.png)
*Figure 1: PaTeX 5D Architectural Drafting Sheet — Somatic Tokenizer (120Hz), 16-Byte
UmpWord SPSC Bus, S13 Spectral MoE, SplitShader GPU Warden, Sovereign Crucible
(ASP+FST+GBNF), and Google Vertex AI Gemini 2.5 Flash Governor.*

## What it does

Packs model weights as balanced ternary at 5 trits per byte (3^5 = 243 states),
leaving 13 spare byte states that carry out-of-band sentinels — values that cannot
occur in valid weight data and therefore halt inference the instant they appear.
A 512-bit XOR+POPCNT MetaRouter dispatches work between the local seats. Expired
activations are zeroized on tick deadline (ADR-0026) with rolling SHA-256
tamper-evident evidence chains. Governed audits escalate to a serverless Google Cloud
Vertex AI gemini-2.5-flash context cache at temperature 0.0 with a strict $0.0004/call ceiling.

The demo shell (`crates/studio-tauri`) renders it: 5D free-flight star navigation
with click-any-star dossiers, a brass astrolabe overlay, an M5 geodesic
worldbuilder canvas, a birth-rite into a CYOA narrative loop that persists choices
to disk, and a ConPTY glass terminal.

## Why it's different

The boundary is enforced by structure, not by policy text. Sovereign refusal is
mechanical: FNV-1a-64 digest filters, a witnessed-canon transducer, and sentinel
byte states that are unrepresentable in legitimate data. A malformed or mutated
envelope is refused at a fixed header offset before any heap allocation or JSON
parse occurs — the attack never reaches a parser to exploit.

## Autonomous agent, beyond chat

`crates/forge-envelope/scripts/agent_loop.py` is an autonomous watcher, not a chat
interface. It polls a Cloud Storage inbox, runs deterministic ByteSieve triage before
spending a token, requests a schema-locked audit from gemini-2.5-flash on Google Cloud
Vertex AI at deterministic temperature 0.0, cross-checks the verdict against a 50-year
degradation model, escalates divergence through a Rust attestation binary, writes the chain
head to Firestore, and wipes local staging only after acknowledgement. No human in the loop.

Run one live pass with `./scripts/demo_cloud_agent.ps1`. It invokes the agent with
`--require-cloud`, which disables every offline fallback — so a green run is proof of
real cloud traffic, and a failure is an honest failure rather than a mock dressed up
as a result. Google Cloud services used: Vertex AI, Firestore, Cloud Storage.

## Numbers

This submission publishes no performance figures in prose. Every benchmark is a
command you run yourself, listed with its method in
[`docs/BENCHMARKS.md`](BENCHMARKS.md). Prior dated receipts are retained unmodified
under `docs/_archive-benchmarks-2026-08-27/` and are superseded by whatever your
own run prints.

That choice is deliberate. These are cache-resident microbenchmarks that move with
CPU model, thermal state, and compiler version. A figure pasted into a pitch goes
stale on the next run and becomes a claim instead of a measurement.

## Try it out

- **Demo video:** [https://youtu.be/ttMofC_9-G0](https://youtu.be/ttMofC_9-G0) (≤3:00, English subtitles.)
- **Build the shell:** `cd crates/studio-tauri && cargo build --release` — Rust
  stable + Windows WebView2 only, no Node. Full instructions:
  [`docs/JUDGE-BUILD.md`](JUDGE-BUILD.md).
- **Reproduce the benchmarks:** [`docs/BENCHMARKS.md`](BENCHMARKS.md).
- **Research:** [DOI 10.5281/zenodo.22020676](https://doi.org/10.5281/zenodo.22020676).

## Built with

Rust (38-crate workspace; the M5 geodesic core module is `#![no_std]`, zero-heap),
Tauri v2 + WebGL2, WebGPU WGSL, Python, Google Cloud (Vertex AI, Cloud Run,
Firestore, Cloud Storage).
