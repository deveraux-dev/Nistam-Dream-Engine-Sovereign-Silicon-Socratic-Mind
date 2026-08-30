# XIV · Sovereign PKM and the Autonomous Flywheel

The Personal Knowledge Management system (`forge-pkm`) must not remain a passive human filing cabinet. In the 13Forge Sovereign Stack, it is an autonomous, self-distilling cognitive process that sits directly at the seam of the **Ramist split** (inside `forge-daemon`). It actively bridges the deterministic simulation (DET) and the real-time presentation (CREATIVE) layers.

By combining low-latency local extraction via `Gemma` with high-cognition background synthesis powered by `ORACLE_B`, the system continuously transforms raw developer logs, session handoffs, and verification milestones into structural, tamper-evident Knowledge Atoms.

---

## 1. The 7-7-7 Dual-School Distillation Cascade

The knowledge pipeline is modeled as a tiered educational institution, progressing systematically from raw, chaotic inputs to highly structured, interconnected master formulas.

```
       [Raw Inputs: Logs, Handoffs, MDs]
                     │
                     ▼
  Student Tier ──► Chunking & Deduplication (SHA-256)
                     │
                     ▼ (Gemma Clustering)
  Teacher Tier ──► Cross-Referenced Context Clusters
                     │
                     ▼ (ORACLE_B Synthesis)
   Master Tier ──► Sealed Knowledge Atoms
                     │
                     ▼
  Lateral Links ─► Cross-Domain Mappings & ADRs
```

### The Student Tier: Chunks & Provenance
Raw documents, session diaries, and build transcripts are digested at the boundary. The student engine performs:
- **Semantic Chunking:** Splitting raw text into bounded blocks.
- **Content-Addressable Dedup:** SHA-256 fingerprinting ensures that identical files or fragments never clutter the core repository.
- **Staleness Decay:** Applied permyriad multipliers track knowledge decay over time, prioritizing active contexts.

### The Teacher Tier: Clustering
Using low-overhead, local `Gemma` embeddings, chunks are grouped into semantic clusters based on spatial-locality and conceptual overlap. 

### The Master Tier: Atoms & Synthesis
The highest echelon of the cascade is reserved for **Knowledge Atoms**. When a cluster matures or a major verification gate passes green, the background daemon schedules an `ORACLE_B` agent dispatch. 
The agent synthesizes the cluster into clear, immutable, signed specifications (like this book, or permanent ADRs), sealing them with cryptographic provenance.

---

## 2. Structural Seam & Orchestration Flow

The PKM daemon operates as a lock-free, asynchronous loop within the **Brain**'s general runtime.

```
+───────────────────────────────────────────────────────────+
│                      ORCHESTRATION                        │
│                                                           │
│  [File Change / Handoff]                                  │
│            │                                              │
│            ▼                                              │
│     PKM Watcher Loop (Ingests)                            │
│            │                                              │
│            ▼                                              │
│     Gemma Embedding Thread (Embeds & Clusters)            │
│            │                                              │
│            ▼                                              │
│     ORACLE_B Background Service                           │
│     (Synthesizes Atoms, publishes verified AGENT.mds)     │
│                                                           │
+───────────────────────────────────────────────────────────+
```

### The Autonomous Loop
1. **Intake:** The daemon detects a new session handoff file or a series of green compilation milestones on the living board.
2. **Deconstruction:** `forge-pkm` breaks down the changes into content-addressed semantic chunks.
3. **Distillation:** The flywheel schedules an `ORACLE_B` batch execution to review the chunks and extract foundational invariants, design constraints, and alchemical pairings.
4. **Attestation:** The synthesized knowledge is sealed, signed with its provenance receipt, and committed back to the persistent store.

---

## 3. Load-Bearing Integration Constraints

To safeguard the determinism of the simulation while ensuring a warm-started context, the autonomous PKM loop must respect these invariant boundaries:

- **Cycle Prevention:** Neither `forge-pkm` nor the daemon may depend on the higher-level game/world engines. They reside at the orchestration boundary, feeding warm context downward into the execution capsules.
- **Weld-Safe Pipeline:** Output artifacts of the distillation cascade (such as lateral connection indexes and system-prompt matrices) must be serialized in a deterministic format, enabling the `cargo xtask` suite to package them without dynamic compiler recursion.
- **Tamper-Evident Signatures:** Every synthesized master atom must be stamped with a SHA-256 provenance receipt, matching the calligraphy law of the Sovereign Stack, guaranteeing that no rogue agent can inject unverified code paths.
