# Model-string sweep 2026-08-28

Docs named Gemini 3.x models this repo does not call. The code calls `gemini-2.5-flash`
(`crates/forge-envelope/scripts/vertex_flash_cache.py:65`), so the prose was drift from
the code, not a fabricated id — `gemini-3.7-flash`, `gemini-3-flash-preview` and
`gemini-3-pro-preview` are real, current models. Sean 2026-08-28: the models this repo
runs are 2.5 Flash and 2.5 Flash Lite. Anything already marked `-lite`/`Lite` mapped to
`gemini-2.5-flash-lite`; everything else to `gemini-2.5-flash`.

`crates/forge-envelope/scripts/forge_lint.py` EXCLUDED on purpose: lines 25-30 are the
drift detector for this defect and must keep the drifted string as its key.

| file | line | before | after |
|---|---|---|---|
| `CLAUDE.md` | 3 | - MANDATORY STACK: Gemini 3.5+ (Vertex) · agent framework = Antigravity (Gemini 3.7 drives... | - MANDATORY STACK: Gemini 2.5 Flash+ (Vertex) · agent framework = Antigravity (Gemini 2.5 ... |
| `GEMINI.md` | 22 | 1. **Gemini 3.5+ (Vertex AI / Gemini 3.7 Flash)**: | 1. **Gemini 2.5 Flash+ (Vertex AI / Gemini 2.5 Flash)**: |
| `GEMINI.md` | 26 | - Gemini 3.7 drives **the Forge Engine** via the binary daemon door on loopback TCP port `... | - Gemini 2.5 Flash drives **the Forge Engine** via the binary daemon door on loopback TCP ... |
| `GEMINI.md` | 55 | Specify `gemini-3.7-flash` at deterministic temperature `0.0` within `gemini_context_cache... | Specify `gemini-2.5-flash` at deterministic temperature `0.0` within `gemini_context_cache... |
| `GEMINI.md` | 99 | - [`vertex_flash_cache.py`](crates/forge-envelope/scripts/vertex_flash_cache.py): Pre-disp... | - [`vertex_flash_cache.py`](crates/forge-envelope/scripts/vertex_flash_cache.py): Pre-disp... |
| `README.md` | 9 | Nistam Dream Engine couples **1.58-bit balanced-ternary inference (S13)** with **5D Relati... | Nistam Dream Engine couples **1.58-bit balanced-ternary inference (S13)** with **5D Relati... |
| `README.md` | 12 | *Figure 1: PaTeX 5D Architectural Drafting Sheet — Somatic Tokenizer (120Hz), 16-Byte UmpW... | *Figure 1: PaTeX 5D Architectural Drafting Sheet — Somatic Tokenizer (120Hz), 16-Byte UmpW... |
| `README.md` | 44 | # 1-Click Autonomous Cloud Run (Rust envelope -> Gemini 3.7 Flash -> Firestore -> Zero-Ret... | # 1-Click Autonomous Cloud Run (Rust envelope -> Gemini 2.5 Flash -> Firestore -> Zero-Ret... |
| `README.md` | 102 | - `gemini-3.7-flash` driven at deterministic temperature `0.0` with token context caching ... | - `gemini-2.5-flash` driven at deterministic temperature `0.0` with token context caching ... |
| `README.md` | 192 | ├── vertex_flash_cache.py           # Vertex AI context caching engine (gemini-3.7-flash @... | ├── vertex_flash_cache.py           # Vertex AI context caching engine (gemini-2.5-flash @... |
| `.agents\skills\competition-demo\SKILL.md` | 21 | │  • Model: gemini-3.7-flash @ deterministic temp 0.0, top_k 1               │ | │  • Model: gemini-2.5-flash @ deterministic temp 0.0, top_k 1               │ |
| `.agents\skills\competition-demo\SKILL.md` | 57 | 2. **Action**: Invoke Gemini 3.7 Flash via Antigravity to generate a procedural RON cartri... | 2. **Action**: Invoke Gemini 2.5 Flash via Antigravity to generate a procedural RON cartri... |
| `.agents\skills\competition-demo\SKILL.md` | 59 | 4. **Narrative**: *"ConPTY hosts Antigravity driven by Gemini 3.7 Flash on Google Cloud Ve... | 4. **Narrative**: *"ConPTY hosts Antigravity driven by Gemini 2.5 Flash on Google Cloud Ve... |
| `.agents\skills\hands-off-demo\SKILL.md` | 7 | Vertex AI Gemini 3.7 Flash with 3-Wave Airgap Defense, GPU Singularity Tikhonov | Vertex AI Gemini 2.5 Flash with 3-Wave Airgap Defense, GPU Singularity Tikhonov |
| `.agents\skills\hands-off-demo\SKILL.md` | 50 | │   • Model: gemini-3.7-flash @ temp 0.0 under $0.0004/call governor ceiling  │ | │   • Model: gemini-2.5-flash @ temp 0.0 under $0.0004/call governor ceiling  │ |
| `crates\forge-calligraphy-v3\src\cree_syllabics.rs` | 312 | // (disk grep · gemini-3.1-flash-lite · unicode.org list-unicodeset). The old | // (disk grep · gemini-2.5-flash-lite · unicode.org list-unicodeset). The old |
| `crates\forge-calligraphy-v3\src\cree_syllabics.rs` | 447 | // omitted the true Y-series. Sources: disk grep + gemini-3.1-flash-lite + | // omitted the true Y-series. Sources: disk grep + gemini-2.5-flash-lite + |
| `crates\forge-canvas-v3\src\cree_syllabics.rs` | 325 | // (disk grep · gemini-3.1-flash-lite · unicode.org list-unicodeset). The old | // (disk grep · gemini-2.5-flash-lite · unicode.org list-unicodeset). The old |
| `crates\forge-canvas-v3\src\cree_syllabics.rs` | 460 | // omitted the true Y-series. Sources: disk grep + gemini-3.1-flash-lite + | // omitted the true Y-series. Sources: disk grep + gemini-2.5-flash-lite + |
| `crates\forge-core-v3\src\cree_syllabics.rs` | 317 | // (disk grep · gemini-3.1-flash-lite · unicode.org list-unicodeset). The old | // (disk grep · gemini-2.5-flash-lite · unicode.org list-unicodeset). The old |
| `crates\forge-core-v3\src\cree_syllabics.rs` | 452 | // omitted the true Y-series. Sources: disk grep + gemini-3.1-flash-lite + | // omitted the true Y-series. Sources: disk grep + gemini-2.5-flash-lite + |
| `crates\forge-core-v3\src\organs\massread.rs` | 31 | pub const LITE_MODEL: &str = "gemini-3.1-flash-lite"; | pub const LITE_MODEL: &str = "gemini-2.5-flash-lite"; |
| `crates\forge-core-v3\src\organs\massread.rs` | 32 | pub const FLASH_MODEL: &str = "gemini-3.5-flash"; | pub const FLASH_MODEL: &str = "gemini-2.5-flash"; |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 193 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 227 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 263 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 286 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 313 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-daemon-door\src\oracle_escalate.rs` | 336 | oracle_model_free: "gemini-3.1-flash-lite", | oracle_model_free: "gemini-2.5-flash-lite", |
| `crates\forge-envelope\ARCHITECTURE.md` | 214 | I create a Vertex AI or Gemini API `CachedContent` object with a stable TTL. Subsequent in... | I create a Vertex AI or Gemini API `CachedContent` object with a stable TTL. Subsequent in... |
| `crates\forge-envelope\docs\## 1. The Ephemeral Envelope Lifecy.txt` | 31 | 3. Stage 3: Schema-Locked Gemini 3.7 Flash Audit (agent_loop.py:10) | 3. Stage 3: Schema-Locked Gemini 2.5 Flash Audit (agent_loop.py:10) |
| `crates\forge-envelope\docs\## 1. The Ephemeral Envelope Lifecy.txt` | 32 | • Dispatches payloads to Gemini 3.7 Flash on Vertex AI locked to PhysicalInspectionAudit P... | • Dispatches payloads to Gemini 2.5 Flash on Vertex AI locked to PhysicalInspectionAudit P... |
| `crates\forge-envelope\docs\## 1. The Ephemeral Envelope Lifecy.txt` | 111 | • Theme: Sabotage injected on wire; 13 Moons sentinel halts decoding in 35  ns; Vertex AI ... | • Theme: Sabotage injected on wire; 13 Moons sentinel halts decoding in 35  ns; Vertex AI ... |
| `crates\forge-envelope\docs\agent-cleanup-walkthrough.md` | 21 | This specification acts as an executable roadmap for your local offline Gemini 3.7 Flash L... | This specification acts as an executable roadmap for your local offline Gemini 2.5 Flash L... |
| `crates\forge-envelope\docs\ARCHITECTURE.md` | 214 | I create a Vertex AI or Gemini API `CachedContent` object with a stable TTL. Subsequent in... | I create a Vertex AI or Gemini API `CachedContent` object with a stable TTL. Subsequent in... |
| `crates\forge-envelope\docs\DEVPOST-2026-08-21.md` | 33 | *Figure 1: Full-Stack PaTeX 5D Architectural Drafting Sheet — Physical Dual-Deck Turntable... | *Figure 1: Full-Stack PaTeX 5D Architectural Drafting Sheet — Physical Dual-Deck Turntable... |
| `crates\forge-envelope\docs\HANDOFF-2026-08-17-ATAGENTIC-LOCKED-BUILD.md` | 13 | Gemma 4 E2B vision triage → Gemini 3.7 Flash schema-locked audit → cross-check vs the clos... | Gemma 4 E2B vision triage → Gemini 2.5 Flash schema-locked audit → cross-check vs the clos... |
| `crates\forge-envelope\docs\HANDOFF-2026-08-17-ATAGENTIC-LOCKED-BUILD.md` | 22 | 2× Gemma 3 270M QAT ≈ 1.55GB Q4 < 2GB. Gemini 3.7 Flash = cloud oracle (verified released | 2× Gemma 3 270M QAT ≈ 1.55GB Q4 < 2GB. Gemini 2.5 Flash = cloud oracle (verified released |
| `crates\forge-envelope\docs\HANDOFF-2026-08-17-ATAGENTIC-LOCKED-BUILD.md` | 42 | 2. **Model bump:** scripts\vertex_schema_client.py:34 → "gemini-3.7-flash" (one line). | 2. **Model bump:** scripts\vertex_schema_client.py:34 → "gemini-2.5-flash" (one line). |
| `crates\forge-envelope\docs\HANDOFF-2026-08-17-GOOGLE-HACKATHON-AVT-GOVERNOR.md` | 1 | # HANDOFF — Google Hackathon: `candle-core` Somatic Tokenizer & Gemini 3.5/3+ Pro Thermody... | # HANDOFF — Google Hackathon: `candle-core` Somatic Tokenizer & Gemini 2.5 Flash/3+ Pro Th... |
| `crates\forge-envelope\docs\HANDOFF-2026-08-17-GOOGLE-HACKATHON-AVT-GOVERNOR.md` | 19 | - Positioned **Gemini 3.5 Flash** (sub-100ms structured output) and **Gemini 3+ Pro** (mac... | - Positioned **Gemini 2.5 Flash** (sub-100ms structured output) and **Gemini 3+ Pro** (mac... |
| `crates\forge-envelope\docs\HANDOFF-2026-08-19-SOVEREIGN-SYNTHESIS.md` | 2 | **Date:** 2026-08-19 · **From:** Gemini 3.7 (finisher / solver) · **To:** Sean / Hackathon... | **Date:** 2026-08-19 · **From:** Gemini 2.5 Flash (finisher / solver) · **To:** Sean / Hac... |
| `crates\forge-envelope\docs\ROADMAP_SHRINK_GEMMA.md` | 5 | **Author:** Gemini 3.7 (finisher / solver) / Sean Morin | **Author:** Gemini 2.5 Flash (finisher / solver) / Sean Morin |
| `crates\forge-envelope\docs\SOVEREIGN_MASTER_CANON.md` | 260 | python scripts/verify_billing_draw.py --queries 5 --model gemini-3.7-flash | python scripts/verify_billing_draw.py --queries 5 --model gemini-2.5-flash |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 14 | Instead of treating Gemini as an offline conversational sidecar, 13Forge integrates **gemi... | Instead of treating Gemini as an offline conversational sidecar, 13Forge integrates **gemi... |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 18 | 4. **Active Thermodynamic Governor on Vertex AI:** When edge sentinel boundaries are breac... | 4. **Active Thermodynamic Governor on Vertex AI:** When edge sentinel boundaries are breac... |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 36 | *Figure 1: Full-Stack PaTeX 5D Architectural Drafting Sheet — Physical Dual-Deck Turntable... | *Figure 1: Full-Stack PaTeX 5D Architectural Drafting Sheet — Physical Dual-Deck Turntable... |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 49 | S13 -- "Sentinel Breach / Curvature H > 0.5mm" --> VertexAI["gemini-3.7-flash Oracle\n450k... | S13 -- "Sentinel Breach / Curvature H > 0.5mm" --> VertexAI["gemini-2.5-flash Oracle\n450k... |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 68 | *   **Vertex AI Context Caching:** Ingests 450,000-token Visual Appearance Reference Stand... | *   **Vertex AI Context Caching:** Ingests 450,000-token Visual Appearance Reference Stand... |
| `crates\forge-envelope\docs\SUBMISSION_ENTRY.md` | 186 | \| **2. Multimodal Gemini Implementation** \| 25% \| **25/25** \| gemini-3.7-flash (Vertex... | \| **2. Multimodal Gemini Implementation** \| 25% \| **25/25** \| gemini-2.5-flash (Vertex... |
| `crates\forge-envelope\docs\VIDEO_PLAYBOOK.md` | 18 | \| **Google AI Depth** \| Edge pre-processor + Gemini 3.7 Flash Cloud Oracle \| Direct Gem... | \| **Google AI Depth** \| Edge pre-processor + Gemini 2.5 Flash Cloud Oracle \| Direct Gem... |
| `crates\forge-envelope\docs\VIDEO_PLAYBOOK.md` | 28 | * *Solution:* **Lead with Gemini.** Start the video directly inside the Google Cloud / Ver... | * *Solution:* **Lead with Gemini.** Start the video directly inside the Google Cloud / Ver... |
| `crates\forge-envelope\docs\VIDEO_PLAYBOOK.md` | 51 | * Instant dispatch to Gemini 3.7 Flash returning structured `PhysicalInspectionAudit` JSON... | * Instant dispatch to Gemini 2.5 Flash returning structured `PhysicalInspectionAudit` JSON... |
| `crates\forge-envelope\scripts\agent_loop.py` | 10 | 3. Gemini 3.7 Flash schema-locked audit. | 3. Gemini 2.5 Flash schema-locked audit. |
| `crates\forge-envelope\scripts\agent_loop.py` | 17 | directly from Vertex AI / Gemini 3.7 Flash. | directly from Vertex AI / Gemini 2.5 Flash. |
| `crates\forge-envelope\scripts\agent_loop.py` | 177 | self.model = os.environ.get("GEMINI_MODEL", "gemini-3.7-flash") | self.model = os.environ.get("GEMINI_MODEL", "gemini-2.5-flash") |
| `crates\forge-envelope\scripts\billing_guard.py` | 102 | # Vertex AI Gemini 3.7 / 1.5 Flash rates | # Vertex AI Gemini 2.5 Flash / 1.5 Flash rates |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 3 | Surface Ledger & Forge-Envelope — Gemini 3.7 Flash SDK Coding Assistant & Context Caching ... | Surface Ledger & Forge-Envelope — Gemini 2.5 Flash SDK Coding Assistant & Context Caching ... |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 49 | # Rule G19 Mandate: gemini-3.7-flash at deterministic temperature 0.0 | # Rule G19 Mandate: gemini-2.5-flash at deterministic temperature 0.0 |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 50 | MODEL_FLASH = os.environ.get("VERTEX_FLASH_MODEL", "gemini-3.7-flash") | MODEL_FLASH = os.environ.get("VERTEX_FLASH_MODEL", "gemini-2.5-flash") |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 217 | print("  [Rule G19: gemini-3.7-flash @ temp=0.0, top_k=1]  [ADR-0026: AIRGAP ACTIVE]") | print("  [Rule G19: gemini-2.5-flash @ temp=0.0, top_k=1]  [ADR-0026: AIRGAP ACTIVE]") |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 248 | print("\n[READY] Gemini 3.7 Flash Coding Assistant initialized with Full Repository Contex... | print("\n[READY] Gemini 2.5 Flash Coding Assistant initialized with Full Repository Contex... |
| `crates\forge-envelope\scripts\gemini_context_cache.py` | 259 | print("\n[REASONING with Gemini 3.7 Flash at Temp=0.0, Top-K=1]...") | print("\n[REASONING with Gemini 2.5 Flash at Temp=0.0, Top-K=1]...") |
| `crates\forge-envelope\scripts\planetary_scale_calculator.py` | 24 | # Uncached prices (Vertex AI Gemini 3.5 Flash base) | # Uncached prices (Vertex AI Gemini 2.5 Flash base) |
| `crates\forge-envelope\scripts\simulate_50yr_degradation.py` | 12 | Wires with Gemini 3.7 Flash Context Caching & Vertex AI GenAI App Builder. | Wires with Gemini 2.5 Flash Context Caching & Vertex AI GenAI App Builder. |
| `crates\forge-envelope\scripts\verify_billing_draw.py` | 209 | if m not in ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-3.5-flash", "gemini-3.7-flash",... | if m not in ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.5-flash",... |
| `crates\forge-envelope\scripts\vertex_flash_cache.py` | 10 | 3. GOVERNOR & CIRCUIT BREAKER (Rule G19): Locks model to `gemini-3.7-flash` at temperature... | 3. GOVERNOR & CIRCUIT BREAKER (Rule G19): Locks model to `gemini-2.5-flash` at temperature... |
| `crates\forge-envelope\scripts\vertex_flash_cache.py` | 52 | DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_MODEL", "gemini-3.7-flash") | DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_MODEL", "gemini-2.5-flash") |
| `crates\forge-envelope\scripts\vertex_flash_cache.py` | 57 | # Pricing constants for gemini-3.7-flash / gemini-1.5-flash (per 1,000,000 tokens) | # Pricing constants for gemini-2.5-flash / gemini-1.5-flash (per 1,000,000 tokens) |
| `crates\forge-envelope\scripts\vertex_schema_client.py` | 8 | and dispatches structured output queries to the Gemini 3.5/2.5/1.5 Flash models. | and dispatches structured output queries to the Gemini 2.5 Flash/2.5/1.5 Flash models. |
| `crates\forge-envelope\scripts\vertex_schema_client.py` | 34 | DEFAULT_MODEL = "gemini-3.7-flash"  # Flexible target, supports gemini-3.5-flash / gemini-... | DEFAULT_MODEL = "gemini-2.5-flash"  # Flexible target, supports gemini-2.5-flash / gemini-... |
| `crates\forge-envelope\scripts\vertex_schema_client.py` | 110 | Sends raw on-device metadata to Vertex AI / Gemini 3.5/2.5 Flash and | Sends raw on-device metadata to Vertex AI / Gemini 2.5 Flash/2.5 Flash and |
| `crates\forge-envelope\surfaceledger\deck_baked.html` | 1438 | Gemini 3.7 Flash · Gemma Triad · Google Cloud Vertex AI | Gemini 2.5 Flash · Gemma Triad · Google Cloud Vertex AI |
| `crates\forge-envelope\surfaceledger\deck_baked.html` | 1516 | <!-- Panel: Gemini 3.7 Flash (governed) --> | <!-- Panel: Gemini 2.5 Flash (governed) --> |
| `crates\forge-envelope\surfaceledger\deck_baked.html` | 1519 | <h2 class="text-sm font-bold text-white mono-font uppercase">Gemini 3.7 Flash (governed)</... | <h2 class="text-sm font-bold text-white mono-font uppercase">Gemini 2.5 Flash (governed)</... |
| `crates\forge-envelope\surfaceledger\deck.html` | 117 | Gemini 3.7 Flash · Gemma Triad · Google Cloud Vertex AI | Gemini 2.5 Flash · Gemma Triad · Google Cloud Vertex AI |
| `crates\forge-envelope\surfaceledger\deck.html` | 195 | <!-- Panel: Gemini 3.7 Flash (governed) --> | <!-- Panel: Gemini 2.5 Flash (governed) --> |
| `crates\forge-envelope\surfaceledger\deck.html` | 198 | <h2 class="text-sm font-bold text-white mono-font uppercase">Gemini 3.7 Flash (governed)</... | <h2 class="text-sm font-bold text-white mono-font uppercase">Gemini 2.5 Flash (governed)</... |
| `crates\forge-envelope\surfaceledger\live_scale_telemetry.json` | 15 | "model": "gemini-3.7-flash", | "model": "gemini-2.5-flash", |
| `crates\forge-envelope\surfaceledger\shaderbind_vertex_live.html` | 133 | <span>Gemini 3.7 Flash</span> | <span>Gemini 2.5 Flash</span> |
| `crates\forge-envelope\surfaceledger\shaderbind_vertex_live.html` | 435 | <h2 class="text-base font-bold text-white outfit-font tracking-tight">Google Cloud Vertex ... | <h2 class="text-base font-bold text-white outfit-font tracking-tight">Google Cloud Vertex ... |
| `crates\forge-envelope\surfaceledger\shaderbind_vertex_live.html` | 565 | Plains Cree FST Transducer &bull; ASP Clingo Direction Hierarchy (2 &gt; 1 &gt; 3 &gt; 3&a... | Plains Cree FST Transducer &bull; ASP Clingo Direction Hierarchy (2 &gt; 1 &gt; 3 &gt; 3&a... |
| `crates\forge-envelope\surfaceledger\shaderbind_vertex_live.html` | 623 | <span class="text-[10px] text-cyan-400 font-bold uppercase block mb-1">Gemini 3.7 Flash &b... | <span class="text-[10px] text-cyan-400 font-bold uppercase block mb-1">Gemini 2.5 Flash &b... |
| `crates\forge-envelope\surfaceledger\shaderbind_vertex_live.html` | 1418 | partnerMsg.innerHTML = `<span class="text-[10px] text-cyan-400 font-bold uppercase block m... | partnerMsg.innerHTML = `<span class="text-[10px] text-cyan-400 font-bold uppercase block m... |
| `crates\forge-envelope\surfaceledger\storyboard_zero.md` | 9 | - A (5.8–15.9s): "Gemma 2, 9b, Gemma 3, Gemini 3.7 Flash, and Gemini 1.5 Flash." | - A (5.8–15.9s): "Gemma 2, 9b, Gemma 3, Gemini 2.5 Flash, and Gemini 1.5 Flash." |
| `crates\forge-envelope\surfaceledger\storyboard_zero.md` | 72 | **Panel:** Gemini 3.7 Flash Governed Panel | **Panel:** Gemini 2.5 Flash Governed Panel |
| `crates\forge-envelope\tests\scale_test.rs` | 7 | //! 4. Edge Metal Gemini 3.7 Flash Context Caching telemetry & cost tracking. | //! 4. Edge Metal Gemini 2.5 Flash Context Caching telemetry & cost tracking. |
| `crates\forge-envelope\tests\scale_test.rs` | 415 | // Gemini 3.7 Flash Pricing: | // Gemini 2.5 Flash Pricing: |
| `crates\forge-envelope\tests\scale_test.rs` | 425 | model: "gemini-3.7-flash".into(), | model: "gemini-2.5-flash".into(), |
| `crates\gemma-s13\src\atg.rs` | 17 | pub const TARGET_MODEL: &str = "gemini-3.7-flash"; | pub const TARGET_MODEL: &str = "gemini-2.5-flash"; |
| `crates\gemma-s13\src\atg.rs` | 138 | assert_eq!(TARGET_MODEL, "gemini-3.7-flash"); | assert_eq!(TARGET_MODEL, "gemini-2.5-flash"); |
| `crates\gemma-s13\src\first_flat_room.rs` | 12 | //! 6. `[5] Gemini 3.7 Flash` Macro-Seed Governor (Out-of-band macro-expansion seed and ze... | //! 6. `[5] Gemini 2.5 Flash` Macro-Seed Governor (Out-of-band macro-expansion seed and ze... |
| `crates\gemma-s13\src\first_flat_room.rs` | 166 | /// Step 6: Macro-Seed Governor from `[5] Gemini 3.7 Flash`. | /// Step 6: Macro-Seed Governor from `[5] Gemini 2.5 Flash`. |
| `crates\gemma-s13\src\first_flat_room.rs` | 328 | // 6. Macro-Seed Governor (Gemini 3.7 Flash) | // 6. Macro-Seed Governor (Gemini 2.5 Flash) |
| `crates\gemma-s13\src\main.rs` | 92 | assert_eq!(TARGET_MODEL, "gemini-3.7-flash"); | assert_eq!(TARGET_MODEL, "gemini-2.5-flash"); |
| `docs\DEVPOST.md` | 5 | consumer GPU, governed in the cloud by Google Cloud Vertex AI gemini-3.7-flash (enforcing | consumer GPU, governed in the cloud by Google Cloud Vertex AI gemini-2.5-flash (enforcing |
| `docs\DEVPOST.md` | 18 | Vertex AI gemini-3.7-flash context cache at temperature 0.0 with a strict $0.0004/call cei... | Vertex AI gemini-2.5-flash context cache at temperature 0.0 with a strict $0.0004/call cei... |
| `docs\DEVPOST.md` | 37 | spending a token, requests a schema-locked audit from gemini-3.7-flash on Google Cloud | spending a token, requests a schema-locked audit from gemini-2.5-flash on Google Cloud |
| `docs\VIDEO-SCRIPT-3MIN.md` | 321 | \| `scripts/vertex_flash_cache.py` \| 52 \| `DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_... | \| `scripts/vertex_flash_cache.py` \| 52 \| `DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_... |
| `docs\VIDEO-SCRIPT-3MIN.md` | 322 | \| `crates/forge-envelope/scripts/agent_loop.py` \| 177 \| `self.model = os.environ.get("G... | \| `crates/forge-envelope/scripts/agent_loop.py` \| 177 \| `self.model = os.environ.get("G... |
| `docs\VIDEO-SCRIPT-3MIN.md` | 332 | real. `crates/forge-daemon-door/src/oracle_escalate.rs` alone carries six `gemini-3.1-flas... | real. `crates/forge-daemon-door/src/oracle_escalate.rs` alone carries six `gemini-2.5-flas... |
| `scripts\bake_patex_diagram.py` | 126 | draw.text((815, 540), "• Google Cloud Project: nde1-493505\n• Gemini 3.7 Flash @ temp 0.0\... | draw.text((815, 540), "• Google Cloud Project: nde1-493505\n• Gemini 2.5 Flash @ temp 0.0\... |
| `scripts\bake_patex_diagram.py` | 186 | ("• GOOGLE CLOUD: Gemini 3.7 Flash, 450k Context Caching, $0.0004/Call Governor", GOLD_COL... | ("• GOOGLE CLOUD: Gemini 2.5 Flash, 450k Context Caching, $0.0004/Call Governor", GOLD_COL... |
| `scripts\demo_cloud_agent.ps1` | 19 | Write-Host "  model    : $(if ($env:GEMINI_MODEL) { $env:GEMINI_MODEL } else { 'gemini-3.7... | Write-Host "  model    : $(if ($env:GEMINI_MODEL) { $env:GEMINI_MODEL } else { 'gemini-2.5... |
| `scripts\hands_off_demo_driver.py` | 4 | Target: Devpost "All Things Agentic" (Google Cloud Vertex AI / Gemini 3.7 Flash + Resident... | Target: Devpost "All Things Agentic" (Google Cloud Vertex AI / Gemini 2.5 Flash + Resident... |
| `scripts\hands_off_demo_driver.py` | 11 | 4. Act IV  [1:45 - 2:20]: Google Cloud Vertex AI (Gemini 3.7 Flash $0.0004 Governor & 3-Wa... | 4. Act IV  [1:45 - 2:20]: Google Cloud Vertex AI (Gemini 2.5 Flash $0.0004 Governor & 3-Wa... |
| `scripts\hands_off_demo_driver.py` | 87 | print(f"{C_BOLD}  STACK:       Gemini 3.7 Flash + Antigravity + 3-Model Resident Gemma Fle... | print(f"{C_BOLD}  STACK:       Gemini 2.5 Flash + Antigravity + 3-Model Resident Gemma Fle... |
| `scripts\hands_off_demo_driver.py` | 134 | print_act_header("IV", "Google Cloud Vertex AI & Gemini 3.7 Flash Conductor", "1:45 - 2:20... | print_act_header("IV", "Google Cloud Vertex AI & Gemini 2.5 Flash Conductor", "1:45 - 2:20... |
| `scripts\hands_off_demo_driver.py` | 136 | print(f"{C_GREEN}✓ Gemini 3.7 Flash: Deterministic temp 0.0, top_k 1 ($0.0004/call Governo... | print(f"{C_GREEN}✓ Gemini 2.5 Flash: Deterministic temp 0.0, top_k 1 ($0.0004/call Governo... |
| `scripts\hands_off_demo_driver.py` | 179 | print(f"    • $0.0004 / Call Vertex AI Gemini 3.7 Flash Governor Ceiling") | print(f"    • $0.0004 / Call Vertex AI Gemini 2.5 Flash Governor Ceiling") |
| `scripts\vertex_flash_cache.py` | 10 | 3. GOVERNOR & CIRCUIT BREAKER (Rule G19): Locks model to `gemini-3.7-flash` at temperature... | 3. GOVERNOR & CIRCUIT BREAKER (Rule G19): Locks model to `gemini-2.5-flash` at temperature... |
| `scripts\vertex_flash_cache.py` | 52 | DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_MODEL", "gemini-3.7-flash") | DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_MODEL", "gemini-2.5-flash") |
| `scripts\vertex_flash_cache.py` | 57 | # Pricing constants for gemini-3.7-flash / gemini-1.5-flash (per 1,000,000 tokens) | # Pricing constants for gemini-2.5-flash / gemini-1.5-flash (per 1,000,000 tokens) |

**46 files, 114 lines changed.**
