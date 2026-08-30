# ARCH-018 — COMPUTE-AT-REST & EMERGENT TOKENIZATION

**STATUS:** CANONICAL ARCHITECTURE
**SUBSYSTEMS:** `120Hz Hot-Path`, `AOT Compilers`, `QuadraticRouter`, `CollisionBridge`

---

## 1. THE AXIOM OF PRE-COMPUTATION
"Compute at rest" dictates that cognitive overhead, string parsing, and complex allocations MUST NOT occur during the 120Hz deterministic runtime. The engine does not solve problems at runtime; it merely reads the pre-calculated answers from disk.

## 2. THE FOUR STATIC COMPILERS
To enforce the 2.0ms execution budget, expensive operations are shifted entirely to author-time compilers:
*   **`.shaderbind` Compiler:** Ingests `*.shaderbind.vixi` files to generate synchronized Rust `wgpu` layouts and WGSL uniform structures, mathematically preventing GPU memory misalignment.
*   **AOT State-Machine Compiler:** Compiles VixiScript and voice-command grammars into zero-allocation Deterministic Finite Automata (DFA). Lexical parsing becomes a flat array lookup.
*   **Cartridge Static VFS Packer:** Compiles loose textures, materials, and audio assets into a single byte-reproducible `.cart` envelope mapped to a Rust `include_bytes!` API. 
*   **Synesthetic Patch-to-Assembly:** Flattens pointer-heavy audio node graphs into unrolled, branchless arithmetic arrays to guarantee zero heap-allocation on the audio thread.

## 3. EMERGENT TOKENIZATION
Language processing requires dynamic flexibility, which fundamentally conflicts with static "compute at rest." 13Forge bridges this via Emergent Tokenization.
*   **Domain-Expert Routing:** Instead of standard context-agnostic BPE, the `QuadraticRouter` uses ellipsoidal boundaries to dynamically learn domain-specific byte groupings. 
*   **Router-Conditional Parametric Cache:** Embedding tables are replaced by an MLP. Different experts produce completely different structural representations for the exact same word, resolving polysemy dynamically on the inference stack.
*   **No Fixed Dictionary:** The vocabulary is not a static file. It is a dynamic function of routing decisions.

## 4. THE 120HZ INTEGER COLLAPSE
To execute these dynamic concepts on a deterministic engine, the Emergent Tokenizer operates entirely asynchronously (author-time). Once the AI inference stack resolves a novel phrase, it collapses the concept into a pure-integer `VixelDiff` or `PackedPoint105` structural coordinate. 

**Known truth:** The 120Hz simulation tick MUST NOT parse strings. It exclusively reads the collapsed integer payload pushed across the `CollisionBridge`. The vocabulary is infinite; the runtime execution is static.

## 5. THE FOREIGN-VOCAB BOUNDARY (gauged 2026-07-26)

Three tokenizers live in the tree, and only one is ours:

| Lane | Tokenizer | Home |
| :--- | :--- | :--- |
| 5D index | cremantic trit pack (5 trits/byte = base-243, 105 trits = 21B) | `forge-calligraphy::cremantic` via `repo_query::pack_point_trits` |
| resident Gemma | hand-rolled SPM, vocab read from GGUF metadata (`tokenizer.ggml.tokens`) | `forge-daemon/src/gemma_engine.rs:76-124` |
| Teacher tier | HuggingFace BPE | `daemon/infer_thread.rs:908 TIER2_TOKENIZER` |

**Boundary law:** a pretrained GGUF's embedding matrix is keyed to ITS OWN vocab ids. Emergent tokens fed to borrowed weights produce noise — swapping the tokenizer under pretrained weights is a RETRAIN, never a wiring job. Gemma therefore keeps its foreign SPM permanently; that is a boundary, NOT drift. The Emergent Tokenizer belongs to the `.nde` ladder (`sovereign.nde` / `teacher.nde` / `master.nde`), trained on the native vocab.

**The near weld (`master.nde` decoder):** `forge_ml::master_decode::decode_to_5d_frames` already emits `Frame [x, y, z, θ_mdeg, w]` — the SAME five lanes `pack_point_trits` collapses to 105 trits. It is the emergent tokenizer's skeleton with the geometry already right. Two gaps: (1) `sniff_ext` covers eight media/symbolic formats (mp3·mp4·mkv·gif·mid·midi2·musicxml·vixi) and prose has NO leg; (2) the routed output must feed the `.nde` ladder, never Gemma (see boundary law). Name collision to hold: `model:"master"` routes to the media DECODER (`dream_wire.rs:424-427`), while `master.nde` is the ladder's master TIER — one token, two organs.

## 6. THE SOULWORD GENERATION SPLIT (Sean 2026-07-26)

One name was covering two organs. The split is MIDI-shaped — two protocol generations, both live:

| Gen | Type | Size | Role | Writer |
| :--- | :--- | :--- | :--- | :--- |
| MIDI-1.0-gen | `outland::SoulWord` | 64B sealed cell (asserts `soulword.rs:24-25`) | trits + parent + hash, cache-line STORE | `SoulWord::seal` (`crates/outland/src/soulword.rs`) |
| MIDI-2.0-gen | `RoutedUmp` | 16B UMP packet | routed WIRE/stream primitive, `ToSoulWord` target | ump bridge (`technothesia/ump_bridge.rs`, `UmpAuthorityTicket`) |

**Name law:** bare `SoulWord` has ONE owner — the 64B cell. The 16B packet is always `RoutedUmp`, never aliased "(SoulWord)" (`UNIVERSAL_DISTILLATION_ARCHITECTURE.md:8` corrected 07-26, tape `32c080dd1a70618b`). Two writers stand; neither folds into the other. Like §5's vocab boundary this is a generation boundary — NOT drift, NOT twins.

**Held prerequisite:** fold the duplicate ump homes (`forge-ump` vs `forge-harmonics::ump`) before `ToSoulWord` trait release; until then the trait plan (`MODALITY_MAPPING_PLAN.md`) stays parked.

## 7. SPCC — SOLITON-PHASE CONTEXT COLLAPSE (Sean 2026-07-27)

Context eviction as anti-phase interference over the 5D frame, folded beside the 64B cell — compute-at-rest applied to CONTEXT: collapse before transfer, never re-parse downstream.

| Piece | Home | Contract |
| :--- | :--- | :--- |
| `Frame5D` + `collide` | `outland::soulword` (tape `de0266c1`) | z gates (mass/family), θ min-arc on Z₃₆₀₀₀₀ via `rem_euclid` — Δθ=0 Redundant (keep one) · Δθ=180 000 AntiPhase (pair annihilates) |
| `compress_frames` | same | in-place swap-with-tail, zero-alloc; 0 survivors → `DESTRUCTIVE_RESONANCE_CASCADE` breaker (stderr + `FORGE_RECOVERY`-seamed append, never a halt) |
| `spcc` verb | `outland::run` dispatch (tape `dbd17ff5`) | studio-arm + example shim, e2e `tests/spcc_e2e.rs` |
| `cree_frame` | `forge_ml::nearest_neighbor` (tape `4fb1e80e`) | cree coords → frame: z/θ/w ride, x/y = stream position; dev-dep-only edge, forge-ml stays isolated |

**Predicate law:** on the 25×4 cree grid the predicate is TOTAL — z = family×14 400 (injective), θ ∈ {0°,90°,180°,270°} exact, so every pair resolves with zero near-miss. Anti-phase = PROOF↔LAW / ACT↔STATE **in-family only**; z shields all cross-family duals (proven `cree_frame_seam_tests`, 4G).

**Discharge ruling (Sean-approved 07-27):** a LAW met by its PROOF (ACT by its STATE) in one family discharges — both leave working context. Semantic ruling, not math; revisit only by tablet §.

**Held:** decoder leg first (§5's prose-leg gap = SPCC's intake, board row NDE-TEXT-LEG); routing tiers (bq→meta→hierarchical→safety) consume post-collapse, one seam per pass; Gemma spec-only (§5 boundary law, nde-ladder).
