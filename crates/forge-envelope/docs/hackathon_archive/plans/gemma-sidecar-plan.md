# Architecture & Implementation Plan: The Non-Hallucinating 5GB-Capped Gemma Sidecar

## Objective
To engineer a local, offline, and mathematically constrained **Gemma Sidecar Assistant** running natively in Rust via `candle-core`. The assistant is bound to a strict **5GB total memory envelope** (weights, context, working buffers, and KV cache) and uses a **RAG DAG + RAMUS** (Directed Acyclic Graph & Branch Routing) execution model with a **local Vision Encoder (ViT)** to **guarantee zero semantic hallucinations and 100% offline edge-native visual attestation**.

It works alongside the GPU-accelerated Tri-Domain DSP layers (`moe-dsp-gpu` and `mom dsp gpu`) to perform verified state attestation at the edge.

---

## Part 1: Core Subsystems & GPU Acceleration

### 1. `moe-dsp-gpu` (Mixture of Experts - DSP - GPU)
*   **The Concept:** A GPU-accelerated Digital Signal Processing Mixture of Experts model. Instead of running a heavy monolithic GPU pipeline, signal processing is delegated to small, highly specialized expert shaders (e.g., FFT filters, Wavelet analyzers, spatial frequency experts).
*   **The Routing:** Ingests raw inputs and uses the `QuadraticRouter` to assign them to the optimized expert shader, keeping execution fast and zero-allocation.

### 2. `mom dsp gpu` (Mixture of Modalities - DSP - GPU)
*   **The Concept:** The GPU implementation of the **Tri-Domain Proof Matrix**. It coordinates visual (Glass), acoustic (Sound), and logical (Memory) streams simultaneously.
*   **The Execution:** Performs high-performance GPU-side steganography—applying LSB watermarks directly into raw RGBA texture arrays and PCM audio streams in parallel, ensuring that generated media holds immutable proof of its origin and time of capture.

---

## Part 2: The 5GB Memory Capping Strategy

Running local LLM + Vision models under a strict 5GB hardware ceiling requires rigorous, static memory allocation to prevent memory fragmentation.

```
+-----------------------------------------------------------------------------------+
|                              5GB TOTAL MEMORY CAP                                 |
+-----------------------------------------------------------------------------------+
| [mmap Gemma Weights] | [Local Vision ViT] | [KV Cache: Bounded] | [Slab Arena]    |
| ~1.5GB - 3.2GB       | ~150MB - 400MB     | ~256MB - 512MB      | ~512MB          |
+-----------------------------------------------------------------------------------+
```

### 1. Weights & Vision: Zero-Copy Memory Mapping (`mmap`)
*   **Gemma Quantization:** Quantized to 4-bit (GGF / GGUF) or custom S13 quantized formats, reducing weight footprint to **~1.5GB - 3.2GB**.
*   **Vision Encoder (ViT / CNN):** A lightweight native Vision Transformer (e.g., ViT-B or custom S13 visual feature extractor) is compiled directly in Candle, taking up only **~150MB - 400MB** of memory.
*   **Memory Mapping:** We bypass the heap by memory-mapping both the quantized LLM weights and the Vision model file using the `memmap2` crate. Tensors are read directly from disk in a zero-copy fashion.

### 2. KV Cache: Bound & Evict
*   We enforce a strict **sliding-window attention boundary** (e.g., 2048 max context tokens).
*   KV cache allocations are pre-allocated on startup and bounded at **~512MB max**, using a circular buffer to evict older context tokens rather than allocating new memory dynamically.

### 3. Static Tensor Working Space (`SlabArena`)
*   All intermediate activation tensors are allocated within a pre-allocated memory slab on the heap during startup (~512MB). 
*   We use a custom bump allocator that resets after each token inference, ensuring zero dynamic heap-allocations during runtime.

---

## Part 3: RAG DAG + RAMUS + Local Vision Pipeline

An assistant "who cannot hallucinate" must be stripped of generative freedom. Instead of naive flat-document vector retrieval (standard RAG) or raw, unchecked visual inputs, we implement a **Local Vision -> RAG DAG + RAMUS** pipeline:

```
           +-----------------------------------------+
           |         Raw On-Site Image / Camera      |
           +-----------------------------------------+
                                |
                                v
           +-----------------------------------------+
           |   Local Candle Vision Encoder (ViT)     | -> Extracts raw visual features
           +-----------------------------------------+    locally on edge device
                                |
                                v
           +-----------------------------------------+
           |   RAMUS Dynamic Branch Router (GPU)     | -> Projects visual tokens directly
           +-----------------------------------------+    to select active pathways (Rami)
                                |
                                v
           +-----------------------------------------+
           |   RAG DAG (Directed Acyclic Graph)      | -> Prunes unrelated nodes; isolates
           +-----------------------------------------+    VARS references and thresholds
                                |
                                v
           +-----------------------------------------+
           |   Gemma Local Inference Engine          | -> Evaluates pruned active branches
           +-----------------------------------------+
                                |
                                v
           +-----------------------------------------+
           |  Grammar-Guided Decoder (Constrained)   | -> Forces output to strictly
           +-----------------------------------------+    match S13 state schema
                                |
                                v
           +-----------------------------------------+
           |  EvidenceChain Sealed Proof (S13 Token) | -> Computes SHA-256 rolling link
           +-----------------------------------------+
```

### 1. Local Vision Encoder (ViT Embedding)
*   A lightweight, local **Vision Transformer (ViT)** parses the raw camera feed or photos on-site, converting pixel data into dense visual embeddings (vectors) completely offline.
*   By performing feature extraction locally, we eliminate the need to upload heavy raw image files to the cloud, guaranteeing data privacy.

### 2. RAG DAG (Directed Acyclic Graph Context)
*   Your 23-year VARS visual reference library, building codes, and insurance regulations are structured as a **Directed Acyclic Graph (DAG)**.
*   Each node represents an atomic, verified assertion (e.g., a specific hinge tolerance, an edge transition threshold, a material standard).
*   Edges represent explicit causal, temporal, or logical dependencies.

### 3. RAMUS (Branching Retrieval Algorithm)
*   The **RAMUS** engine maps the dense visual embedding from our Local Vision Encoder directly onto the RAG DAG.
*   It walks the graph to identify the exact **branch (ramus)** of visual/logical standards that applies to the active observation (e.g. if the ViT embedding maps to a door hinge, RAMUS isolates *only* the door and hinge branches, pruning all unrelated rami).
*   Only this clean, isolated, and logically complete branch context is injected into Gemma, keeping the context window tight, focused, and completely factual.

### 4. Logit Bias & Grammar-Guided Decoders (BNF / JSON Schema)
*   We implement **constrained token sampling** at the logits level using a Context-Free Grammar (CFG) or BNF schema.
*   During token selection, the probability of tokens that violate the target schema (e.g., the VARS dictionary or valid `s13.rs` syntax) is set to $-\infty$.
*   This makes it structurally impossible for Gemma to generate open-ended prose. It is physically forced to ONLY output valid S13 state tokens, integers, or `NOT_FOUND`.

### 5. Weaver Arbiter Verification Loop
*   Before the sidecar returns any text output, the **Weaver Arbiter** validates the state transition against the rolling hashes in the `EvidenceChain`. If a transition does not have a corresponding attested hash link, the response is rejected before leaving the sidecar boundary.

---

## Part 4: Implementation Roadmap

### Phase 1: Bounding the Memory Envelope (Candle Setup)
1.  Integrate `candle-core` with `memmap2` to load your quantized custom Gemma weights and lightweight native Vision weights.
2.  Pre-allocate the KV Cache ring buffer and define the strict sliding-window attention context limits.
3.  Set up the `SlabArena` allocator for intermediate activation tensors.

### Phase 2: Building the RAG DAG + RAMUS Engine
1.  Define the DAG database structure representing your locked VARS references and S13 schemas.
2.  Develop the **RAMUS** branch-routing search algorithm to map ViT visual embeddings onto the active RAG DAG branches.
3.  Write the custom token-selector that applies strict logit masking against the RAG DAG branch outcomes.

### Phase 3: Integrating the GPU & Proof Layers
1.  Link the sidecar to the GPU DSP layers (`moe-dsp-gpu` and `mom dsp gpu`) to handle parallel steganographic visual and audio watermarking.
2.  Verify the entire loop with the `EvidenceChain` in `forge-envelope`, ensuring that the collapsed S13 states are cryptographically sealed.
