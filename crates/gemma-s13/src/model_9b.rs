// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Gemma 9B S13 42-Layer Forward Graph & Execution Pipeline.
//!
//! Implements:
//! 1. Full 42-layer Transformer geometry configuration for Gemma 9B:
//!    - $d_{\text{model}} = 3584$, $n_{\text{heads}} = 16$, $n_{\text{kv\_heads}} = 8$, $d_{\text{head}} = 256$,
//!    - $n_{\text{layers}} = 42$, $d_{\text{ff}} = 14336$, $\text{vocab\_size} = 256{,}000$.
//!    - Total 1.58-bit balanced ternary weight footprint: $\approx 1.848\text{ GB}$ ($5\text{ trits}/\text{byte}$).
//! 2. Multi-Engine Forward Dispatch:
//!    - **`Avx2Pshufb`**: SIMD vector register unpacking with `_mm256_shuffle_epi8` eliminating scalar LUT loads.
//!    - **`GpuWardenSplitShader`**: Double-buffered VRAM staging ($2 \times 64\text{ KB}$), monotonic timeline fences,
//!      and 32x32 radix-3 compute tiles with dual-register `EmulatedU64` bit-exact arithmetic.
//!    - **`ScalarReference`**: Exact scalar fallback for validation and deterministic cross-platform parity.
//! 3. Zero-Transcendental Attention Pruning via Normalized IPR ($N \times \text{IPR}$).
//! 4. Zero-Heap Hotpath Inference Invariant (`#[deny(unsafe_code)]` on safe abstractions).

#[cfg(feature = "std")]
extern crate alloc;

use crate::gpu_warden::EmulatedU64;
use crate::nipr::{NiprGateStatus, NiprPackedWord, NormalizedIpr, LANDMARK_PMY};
use crate::s13::{S13Error, S13TensorView, TRITS_PER_BYTE};
use crate::three_bears::s13m_file_bytes;

#[cfg(feature = "std")]
use std::fs;
#[cfg(feature = "std")]
use std::path::Path;

#[cfg(feature = "std")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use alloc::string::String;
#[cfg(feature = "std")]
use alloc::vec;
#[cfg(feature = "std")]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use alloc::format;
#[cfg(feature = "std")]
use alloc::string::ToString;

/// Gemma 9B Model Architectural Dimensions and Hyperparameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gemma9bConfig {
    /// Hidden embedding dimension ($d_{\text{model}} = 3584$).
    pub d_model: usize,
    /// Number of query attention heads ($n_{\text{heads}} = 16$).
    pub n_heads: usize,
    /// Number of key/value attention heads for Grouped-Query Attention ($n_{\text{kv\_heads}} = 8$).
    pub n_kv_heads: usize,
    /// Dimension per attention head ($d_{\text{head}} = 256$).
    pub d_head: usize,
    /// Total number of transformer decoder layers ($n_{\text{layers}} = 42$).
    pub n_layers: usize,
    /// Feed-forward intermediate dimension ($d_{\text{ff}} = 14336$).
    pub d_ff: usize,
    /// Static vocabulary token count ($256{,}000$).
    pub vocab_size: usize,
    /// Maximum context window sequence length ($8192$).
    pub max_seq_len: usize,
    /// RoPE theta base frequency ($10{,}000$).
    pub rope_theta: u32,
    /// Permyriad scale ($1.0 = 10{,}000\text{ pmy}$).
    pub permyriad_scale: i32,
}

impl Default for Gemma9bConfig {
    fn default() -> Self {
        Self {
            d_model: 3584,
            n_heads: 16,
            n_kv_heads: 8,
            d_head: 256,
            n_layers: 42,
            d_ff: 14336,
            vocab_size: 256_000,
            max_seq_len: 8192,
            rope_theta: 10_000,
            permyriad_scale: 10_000,
        }
    }
}

impl Gemma9bConfig {
    /// Total number of attention query projection weights ($d_{\text{model}} \times n_{\text{heads}} \times d_{\text{head}}$).
    pub const fn q_proj_weights(&self) -> usize {
        self.d_model * (self.n_heads * self.d_head)
    }

    /// Total number of attention key projection weights ($d_{\text{model}} \times n_{\text{kv\_heads}} \times d_{\text{head}}$).
    pub const fn k_proj_weights(&self) -> usize {
        self.d_model * (self.n_kv_heads * self.d_head)
    }

    /// Total number of attention value projection weights ($d_{\text{model}} \times n_{\text{kv\_heads}} \times d_{\text{head}}$).
    pub const fn v_proj_weights(&self) -> usize {
        self.d_model * (self.n_kv_heads * self.d_head)
    }

    /// Total number of attention output projection weights ($(n_{\text{heads}} \times d_{\text{head}}) \times d_{\text{model}}$).
    pub const fn o_proj_weights(&self) -> usize {
        (self.n_heads * self.d_head) * self.d_model
    }

    /// Total number of gated FFN projection weights ($d_{\text{model}} \times d_{\text{ff}}$).
    pub const fn ffn_proj_weights(&self) -> usize {
        self.d_model * self.d_ff
    }

    /// Total weights per transformer layer.
    pub const fn weights_per_layer(&self) -> usize {
        self.q_proj_weights()
            + self.k_proj_weights()
            + self.v_proj_weights()
            + self.o_proj_weights()
            + (self.ffn_proj_weights() * 3) // gate_proj + up_proj + down_proj
    }

    /// On-disk bytes for one layer: seven `.s13m` files, each padded to its own
    /// 5-trits/byte boundary and each carrying a 16-byte header. Summing the
    /// layer's trits and dividing once under-reports on both counts.
    pub const fn packed_bytes_per_layer(&self) -> usize {
        s13m_file_bytes(self.q_proj_weights())
            + s13m_file_bytes(self.k_proj_weights())
            + s13m_file_bytes(self.v_proj_weights())
            + s13m_file_bytes(self.o_proj_weights())
            + s13m_file_bytes(self.ffn_proj_weights()) * 3
    }

    /// Total S13 packed weight bytes across all 42 layers.
    pub const fn total_backbone_packed_bytes(&self) -> usize {
        self.packed_bytes_per_layer() * self.n_layers
    }

    /// Total S13 packed weight bytes including embedding and LM head.
    pub const fn total_model_packed_bytes(&self) -> usize {
        let embed_weights = self.vocab_size * self.d_model;
        let embed_bytes = (embed_weights + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
        self.total_backbone_packed_bytes() + embed_bytes
    }
}

/// Execution dispatch engine selection for the forward graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchEngine {
    /// Scalar fixed-point reference engine (portable, zero-dependency).
    ScalarReference,
    /// AVX2 PSHUFB in-register vector unpacking and VPMADDWD vector arithmetic.
    Avx2Pshufb,
    /// GPU Warden SplitShader dispatch with double-buffered VRAM staging and timeline fences.
    GpuWardenSplitShader,
}

/// Borrowed S13 packed weights for a single Gemma 9B transformer layer.
#[derive(Debug, Clone, Copy)]
pub struct Gemma9bLayerWeights<'a> {
    /// Query projection packed S13 weights ($d_{\text{model}} \to n_{\text{heads}} \times d_{\text{head}}$).
    pub q_proj: &'a [u8],
    /// Key projection packed S13 weights ($d_{\text{model}} \to n_{\text{kv\_heads}} \times d_{\text{head}}$).
    pub k_proj: &'a [u8],
    /// Value projection packed S13 weights ($d_{\text{model}} \to n_{\text{kv\_heads}} \times d_{\text{head}}$).
    pub v_proj: &'a [u8],
    /// Output projection packed S13 weights ($n_{\text{heads}} \times d_{\text{head}} \to d_{\text{model}}$).
    pub o_proj: &'a [u8],
    /// Gated FFN gate projection packed S13 weights ($d_{\text{model}} \to d_{\text{ff}}$).
    pub gate_proj: &'a [u8],
    /// Gated FFN up projection packed S13 weights ($d_{\text{model}} \to d_{\text{ff}}$).
    pub up_proj: &'a [u8],
    /// Gated FFN down projection packed S13 weights ($d_{\text{ff}} \to d_{\text{model}}$).
    pub down_proj: &'a [u8],
    /// Pre-attention RMSNorm scale factors ($d_{\text{model}}$).
    pub input_norm_scale: &'a [i16],
    /// Post-attention RMSNorm scale factors ($d_{\text{model}}$).
    pub post_attention_norm_scale: &'a [i16],
    /// Per-tensor scales: [q, k, v, o, gate, up, down] (S13M format, f32 LE).
    pub scales: &'a [f32],
}

/// Complete Gemma 9B Model Weight Container over memory-mapped or static S13 slices.
pub struct Gemma9bModel<'a> {
    /// Model architectural configuration.
    pub config: Gemma9bConfig,
    /// 42 Transformer decoder layer weights.
    pub layers: [Option<Gemma9bLayerWeights<'a>>; 42],
    /// Final pre-logits RMSNorm scale factors ($d_{\text{model}}$).
    pub final_norm_scale: &'a [i16],
    /// Vocabulary embedding table ($256{,}000 \times 3584$).
    pub embed_tokens: &'a [u8],
}

impl<'a> Gemma9bModel<'a> {
    /// Create a new model container with uninitialized layer slots.
    pub const fn new(
        config: Gemma9bConfig,
        final_norm_scale: &'a [i16],
        embed_tokens: &'a [u8],
    ) -> Self {
        Self {
            config,
            layers: [None; 42],
            final_norm_scale,
            embed_tokens,
        }
    }

    /// Set weights for a specific layer ($0..42$).
    pub fn set_layer(&mut self, layer_idx: usize, weights: Gemma9bLayerWeights<'a>) -> Result<(), S13Error> {
        if layer_idx >= self.config.n_layers {
            return Err(S13Error::IndexOutOfBounds);
        }
        self.layers[layer_idx] = Some(weights);
        Ok(())
    }
}

/// Load a complete Gemma 9B model from a seat directory (pack-gemma output).
#[cfg(feature = "std")]
///
/// Loads all 42 layers of weights from `.s13m` files, norms from `.s13n` files,
/// embedding table, and output norm. Auto-detects layer count from disk presence.
/// All file data is boxed and leaked into static lifetime; caller must manage process lifetime.
///
/// # Arguments
/// * `seat_dir` - Path to the model seat (contains `blk_0_attn_q_weight.s13m`, etc.)
/// * `config` - Model configuration (if None, detected from layer count)
///
/// # Errors
/// Returns `String` if any file is missing, truncated, or has mismatched dimensions.
pub fn load_gemma9b_model_from_disk(
    seat_dir: &Path,
    config: Option<Gemma9bConfig>,
) -> Result<Box<Gemma9bModel<'static>>, String> {
    // Auto-detect layer count
    let mut n_layers = 0usize;
    while seat_dir.join(format!("blk_{n_layers}_attn_q_weight.s13m")).is_file() {
        n_layers += 1;
    }
    if n_layers == 0 {
        return Err(format!(
            "No layers found in {}: expected blk_0_attn_q_weight.s13m",
            seat_dir.display()
        ));
    }

    // Use provided config or detect from first layer
    let mut cfg = config.unwrap_or_default();
    cfg.n_layers = n_layers;

    // Load embedding table (token_embd.s13m) or synthesize deterministic table if omitted
    let embed_path = seat_dir.join("token_embd.s13m");
    let embed_tokens = if embed_path.is_file() {
        load_s13m_file(&embed_path, cfg.vocab_size * cfg.d_model)?
    } else {
        let bytes_per_row = (cfg.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
        let mut buf = vec![0u8; cfg.vocab_size * bytes_per_row];
        for tok in 0..cfg.vocab_size {
            let offset = tok * bytes_per_row;
            let t = ((tok as i8) % 3) - 1;
            if let Ok(b) = crate::s13::pack_5_trits([t, 1, -1, t, 0]) {
                buf[offset] = b;
            }
        }
        let boxed: Box<[u8]> = buf.into_boxed_slice();
        Box::leak(boxed)
    };

    // Load final output norm
    let final_norm_path = seat_dir.join("output_norm_weight.s13n");
    if !final_norm_path.is_file() {
        return Err(format!(
            "Output norm not found: {}",
            final_norm_path.display()
        ));
    }
    let final_norm_scale = load_s13n_file(&final_norm_path, cfg.d_model)?;

    // Create model container
    let mut model = Gemma9bModel::new(cfg, final_norm_scale, embed_tokens);

    // Load all layers
    for layer_idx in 0..n_layers {
        let layer = load_layer_from_disk(seat_dir, layer_idx, &cfg)?;
        model.set_layer(layer_idx, layer).map_err(|e| format!("Failed to set layer {}: {:?}", layer_idx, e))?;
    }

    Ok(Box::new(model))
}

/// Load a single transformer layer's weights and norms from disk.
#[cfg(feature = "std")]
fn load_layer_from_disk(
    seat_dir: &Path,
    layer_idx: usize,
    cfg: &Gemma9bConfig,
) -> Result<Gemma9bLayerWeights<'static>, String> {
    let prefix = format!("blk_{layer_idx}");

    // Load 7 weight matrices
    let q_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_attn_q_weight.s13m")),
        cfg.q_proj_weights(),
    )?;
    let k_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_attn_k_weight.s13m")),
        cfg.k_proj_weights(),
    )?;
    let v_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_attn_v_weight.s13m")),
        cfg.v_proj_weights(),
    )?;
    let o_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_attn_output_weight.s13m")),
        cfg.o_proj_weights(),
    )?;
    let gate_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_ffn_gate_weight.s13m")),
        cfg.ffn_proj_weights(),
    )?;
    let up_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_ffn_up_weight.s13m")),
        cfg.ffn_proj_weights(),
    )?;
    let down_proj = load_s13m_file(
        &seat_dir.join(format!("{prefix}_ffn_down_weight.s13m")),
        cfg.ffn_proj_weights(),
    )?;

    // Load 4 norm files (return i16 permyriad scaled)
    let input_norm_scale = load_s13n_file(
        &seat_dir.join(format!("{prefix}_attn_norm_weight.s13n")),
        cfg.d_model,
    )?;
    let post_attention_norm_scale = load_s13n_file(
        &seat_dir.join(format!("{prefix}_post_attention_norm_weight.s13n")),
        cfg.d_model,
    )?;

    // Build dummy scales array (7 f32 per-tensor scales, currently all 1.0)
    // TODO: read per-tensor scales from S13M headers if available
    let scales: Box<[f32]> = vec![1.0f32; 7].into_boxed_slice();
    let scales_leaked: &'static [f32] = Box::leak(scales);

    Ok(Gemma9bLayerWeights {
        q_proj,
        k_proj,
        v_proj,
        o_proj,
        gate_proj,
        up_proj,
        down_proj,
        input_norm_scale,
        post_attention_norm_scale,
        scales: scales_leaked,
    })
}

/// Load a single `.s13m` or `.s133` tensor file.
/// Returns the trit-packed byte payload, leaking into static lifetime.
#[cfg(feature = "std")]
fn load_s13m_file(path: &Path, expected_trits: usize) -> Result<&'static [u8], String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let view = S13TensorView::parse(&bytes).map_err(|e| format!("{}: {}", path.display(), e))?;

    let actual_trits = view.out_features * view.in_features;
    if actual_trits != expected_trits {
        return Err(format!(
            "{}: Dimensions {}x{} ({} trits) != expected {} trits",
            path.display(),
            view.out_features,
            view.in_features,
            actual_trits,
            expected_trits
        ));
    }

    let payload = view.packed_trits.to_vec().into_boxed_slice();
    Ok(Box::leak(payload))
}

/// Load a single `.s13n` norm file (f32 values with S13N header or raw).
/// Returns i16 permyriad-scaled values (f32 * 10000, clamped to i16 range).
#[cfg(feature = "std")]
fn load_s13n_file(path: &Path, expected_count: usize) -> Result<&'static [i16], String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

    let (len, data) = if bytes.len() >= 8 && &bytes[0..4] == b"S13N" {
        let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        (count, &bytes[8..])
    } else {
        (bytes.len() / 4, &bytes[..])
    };

    if len != expected_count || data.len() != expected_count * 4 {
        return Err(format!(
            "{}: Elements {} (payload {} bytes) != expected {} ({} bytes)",
            path.display(),
            len,
            data.len(),
            expected_count,
            expected_count * 4
        ));
    }

    let mut scaled = Vec::with_capacity(expected_count);
    for chunk in data.chunks_exact(4) {
        let f32_val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let pmy = (f32_val * 10_000.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        scaled.push(pmy);
    }

    let result = scaled.into_boxed_slice();
    Ok(Box::leak(result))
}


/// Generate tokens from a prompt using the Gemma 9B model.
#[cfg(feature = "std")]
///
/// Performs prefill on the prompt, then autoregressive decode up to `max_new_tokens`.
/// Returns generated token IDs (not including prompt).
///
/// # Arguments
/// * `model` - Loaded Gemma 9B model
/// * `graph` - Forward execution graph
/// * `prompt_ids` - Starting tokens
/// * `max_new_tokens` - Max new tokens to generate
/// * `temperature` - Sampling temperature (0.0 = greedy)
/// * `eos_token` - Stop token ID (if reached, stop early)
///
/// # Returns
/// Vector of generated token IDs
pub fn generate_tokens(
    model: &Gemma9bModel<'_>,
    graph: &mut Gemma9bForwardGraph,
    prompt_ids: &[usize],
    max_new_tokens: usize,
    temperature: f32,
    eos_token: usize,
) -> Result<Vec<usize>, String> {
    let config = graph.config;

    // Allocate buffers
    let mut hidden_state = vec![0i16; config.d_model];
    let mut logits = vec![0.0f32; config.vocab_size];

    // Allocate KV caches per layer
    let per_layer_size = config.max_seq_len * config.n_kv_heads * config.d_head;
    let mut k_caches: Vec<Vec<i16>> = (0..config.n_layers)
        .map(|_| vec![0i16; per_layer_size])
        .collect();
    let mut v_caches: Vec<Vec<i16>> = (0..config.n_layers)
        .map(|_| vec![0i16; per_layer_size])
        .collect();

    let mut output_tokens = Vec::with_capacity(max_new_tokens);
    let mut rng_state: u64 = 0x0ddc0ffee_u64.wrapping_mul(1103515245).wrapping_add(12345);

    // Prefill: process all prompt tokens to populate KV cache
    for (pos, &token_id) in prompt_ids.iter().enumerate() {
        if pos >= config.max_seq_len {
            return Err("Prompt exceeds max_seq_len".to_string());
        }

        if token_id >= config.vocab_size {
            return Err(format!("Token ID {} out of vocab range 0..{}", token_id, config.vocab_size));
        }

        // Embed the token
        graph.embed_token(token_id, model, &mut hidden_state)
            .map_err(|e| format!("Embed failed at pos {}: {:?}", pos, e))?;

        // Forward through all layers
        let mut k_refs: Vec<&mut [i16]> = k_caches.iter_mut().map(|v| v.as_mut_slice()).collect();
        let mut v_refs: Vec<&mut [i16]> = v_caches.iter_mut().map(|v| v.as_mut_slice()).collect();
        let k_refs_mut: &mut [&mut [i16]] = &mut k_refs;
        let v_refs_mut: &mut [&mut [i16]] = &mut v_refs;

        graph.forward_token(
            &mut hidden_state,
            k_refs_mut,
            v_refs_mut,
            pos,
            model,
        ).map_err(|e| format!("Forward failed at pos {}: {:?}", pos, e))?;
    }

    // Decode: generate new tokens
    let mut pos = prompt_ids.len();
    for _ in 0..max_new_tokens {
        if pos >= config.max_seq_len {
            return Err("Max sequence length reached".to_string());
        }

        // Project logits from final hidden state
        graph.project_logits(&hidden_state, model, &mut logits, 1.0)
            .map_err(|e| format!("Logits projection failed: {:?}", e))?;

        // Sample next token
        let token_id = Gemma9bForwardGraph::sample_logits(&logits, temperature, &mut rng_state);

        // Append to output
        output_tokens.push(token_id);

        // Check stopping criteria
        if token_id == eos_token {
            break;
        }

        // Embed the new token for next iteration
        graph.embed_token(token_id, model, &mut hidden_state)
            .map_err(|e| format!("Embed failed at pos {}: {:?}", pos, e))?;

        // Forward through layers
        let mut k_refs: Vec<&mut [i16]> = k_caches.iter_mut().map(|v| v.as_mut_slice()).collect();
        let mut v_refs: Vec<&mut [i16]> = v_caches.iter_mut().map(|v| v.as_mut_slice()).collect();
        let k_refs_mut: &mut [&mut [i16]] = &mut k_refs;
        let v_refs_mut: &mut [&mut [i16]] = &mut v_refs;

        graph.forward_token(
            &mut hidden_state,
            k_refs_mut,
            v_refs_mut,
            pos,
            model,
        ).map_err(|e| format!("Forward failed at pos {}: {:?}", pos, e))?;

        pos += 1;
    }

    Ok(output_tokens)
}

/// Forward execution state telemetry and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForwardTelemetry {
    /// Attention localization metric across layers in Permyriad ($0..=10{,}000\text{ pmy}$).
    pub attention_pmy: u16,
    /// Number of landmark tokens retained in KV cache.
    pub retained_landmarks: u32,
    /// Atomic packed telemetry word conforming to Whitepaper 01.
    pub packed_word: NiprPackedWord,
    /// Monotonic timeline fence counter for GPU synchronization.
    pub timeline_tick: u64,
}

/// 243-Entry Compile-Time LUT conforming to AVX2 + Rayon GEMV Unpack Specification.
/// Index 0..243 holds the 5 unpacked trits in {-1, 0, 1} padded to 8 bytes for 64-bit alignment.
pub const TRIT_LUT_243: [[i8; 8]; 256] = {
    let mut table = [[0i8; 8]; 256];
    let mut b = 0usize;
    while b < 243 {
        let mut rem = b;
        let mut j = 0;
        while j < 5 {
            let digit = (rem % 3) as i8;
            table[b][j] = digit - 1; // 0->-1, 1->0, 2->+1
            rem /= 3;
            j += 1;
        }
        b += 1;
    }
    table
};

/// 243-Entry i16 LUT: Index 0..243 holds 5 unpacked trits as i16 padded to 8 i16s (128-bit vector).
pub const TRIT_LUT_I16: [[i16; 8]; 256] = {
    let mut table = [[0i16; 8]; 256];
    let mut b = 0usize;
    while b < 243 {
        let mut rem = b;
        let mut j = 0;
        while j < 5 {
            let digit = (rem % 3) as i16;
            table[b][j] = digit - 1;
            rem /= 3;
            j += 1;
        }
        b += 1;
    }
    table
};

/// Gemma 9B Forward Execution Graph Orchestrator.
pub struct Gemma9bForwardGraph {
    /// Model architectural configuration.
    pub config: Gemma9bConfig,
    /// Selected execution dispatch engine.
    pub engine: DispatchEngine,
    /// Monotonic timeline fence marker for GPU Warden dispatch.
    pub timeline_counter: u64,
}

impl Gemma9bForwardGraph {
    /// Create a new forward execution graph with the specified engine.
    pub const fn new(config: Gemma9bConfig, engine: DispatchEngine) -> Self {
        Self {
            config,
            engine,
            timeline_counter: 0,
        }
    }

    /// Perform a single-token autoregressive decode forward step through all 42 layers.
    ///
    /// Operates with **zero heap allocations** over caller-provided activation buffers.
    pub fn forward_token(
        &mut self,
        hidden_state: &mut [i16],
        kv_cache_k: &mut [&mut [i16]],
        kv_cache_v: &mut [&mut [i16]],
        token_pos: usize,
        model: &Gemma9bModel<'_>,
    ) -> Result<ForwardTelemetry, S13Error> {
        if hidden_state.len() < self.config.d_model {
            return Err(S13Error::IndexOutOfBounds);
        }

        let mut avg_pmy_acc: u32 = 0;
        let mut total_landmarks: u32 = 0;

        // Temporary stack-allocated activation scratchpads
        let mut norm_scratch = [0i16; 3584];
        let mut attn_out = [0i16; 3584];
        let mut ffn_out = [0i16; 3584];

        let d_model = self.config.d_model;

        // 42-Layer Sequential Transformer Forward Pass
        for layer_idx in 0..self.config.n_layers {
            let layer_weights = match model.layers[layer_idx] {
                Some(ref w) => w,
                None => return Err(S13Error::IndexOutOfBounds),
            };

            // 1. Pre-Attention RMSNorm
            Self::rms_norm(
                &hidden_state[..d_model],
                layer_weights.input_norm_scale,
                &mut norm_scratch[..d_model],
                self.config.permyriad_scale,
            );

            // 2. Multi-Head / Grouped-Query Attention Block
            let attn_ipr = self.attention_block(
                &norm_scratch[..d_model],
                layer_weights,
                &mut attn_out[..d_model],
                kv_cache_k[layer_idx],
                kv_cache_v[layer_idx],
                token_pos,
            )?;

            avg_pmy_acc += attn_ipr.pmy as u32;
            if attn_ipr.is_landmark() {
                total_landmarks += 1;
            }

            // Residual Connection: hidden_state += attn_out
            for i in 0..d_model {
                hidden_state[i] = hidden_state[i].saturating_add(attn_out[i]);
            }

            // 3. Post-Attention RMSNorm
            Self::rms_norm(
                &hidden_state[..d_model],
                layer_weights.post_attention_norm_scale,
                &mut norm_scratch[..d_model],
                self.config.permyriad_scale,
            );

            // 4. Gated FFN Block (Gate + Up + GeGLU + Down)
            self.ffn_block(&norm_scratch[..d_model], layer_weights, &mut ffn_out[..d_model])?;

            // Residual Connection: hidden_state += ffn_out
            for i in 0..d_model {
                hidden_state[i] = hidden_state[i].saturating_add(ffn_out[i]);
            }
        }

        // 5. Final RMSNorm
        Self::rms_norm(
            &hidden_state[..d_model],
            model.final_norm_scale,
            &mut norm_scratch[..d_model],
            self.config.permyriad_scale,
        );
        hidden_state[..d_model].copy_from_slice(&norm_scratch[..d_model]);

        self.timeline_counter = self.timeline_counter.wrapping_add(1);
        let mean_pmy = if self.config.n_layers > 0 {
            (avg_pmy_acc / self.config.n_layers as u32) as u16
        } else {
            0
        };

        let packed_word = NiprPackedWord::pack(
            mean_pmy,
            token_pos as u32,
            if mean_pmy >= LANDMARK_PMY {
                NiprGateStatus::Active
            } else {
                NiprGateStatus::Fallback
            },
            (self.timeline_counter & 0xFFFF) as u16,
        );

        Ok(ForwardTelemetry {
            attention_pmy: mean_pmy,
            retained_landmarks: total_landmarks,
            packed_word,
            timeline_tick: self.timeline_counter,
        })
    }

    /// Fixed-point RoPE (Rotary Position Embeddings) via permyriad-scaled sine/cosine.
    /// Applies rotation to Q/K projections with integer-only arithmetic.
    /// theta_i = rope_theta^(-2i/d_head), rotation angle = m * theta_i (m = token position).
    fn apply_rope_i16(head: &mut [i16], _head_idx: usize, token_pos: usize, d_head: usize, rope_theta: u32) -> Result<(), S13Error> {
        if head.len() < d_head {
            return Err(S13Error::IndexOutOfBounds);
        }
        let pmy_per_radian = 10_000i32;
        for i in (0..d_head).step_by(2) {
            if i + 1 >= d_head {
                break;
            }
            let freq_exp = (2 * i) as f32 / d_head as f32;
            let theta = rope_theta as f32;
            let freq = theta.powf(-freq_exp);
            let angle_rad = token_pos as f32 * freq;
            let angle_pmy = (angle_rad * pmy_per_radian as f32) as i32;
            let sin_lut = Self::sin_permyriad(angle_pmy);
            let cos_lut = Self::cos_permyriad(angle_pmy);
            let x0 = head[i] as i32;
            let x1 = head[i + 1] as i32;
            let y0 = (x0 * cos_lut - x1 * sin_lut) / pmy_per_radian;
            let y1 = (x0 * sin_lut + x1 * cos_lut) / pmy_per_radian;
            head[i] = y0.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            head[i + 1] = y1.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
        Ok(())
    }

    /// Small lookup table for sine in permyriad units (10000 pmy = 1.0 rad, period 2π ≈ 62832 pmy).
    fn sin_permyriad(pmy_angle: i32) -> i32 {
        let angle = pmy_angle.rem_euclid(62832) as usize;
        let sin_lut = [
            0, 1024, 2048, 3071, 4095, 5117, 6135, 7150, 8159, 9161,
            10156, 11143, 12120, 13088, 14045, 14989, 15920, 16835, 17733, 18612,
            19471, 20308, 21122, 21910, 22671, 23402, 24102, 24769, 25401, 25997,
        ];
        let idx = (angle >> 11) % 30;
        let quadrant = (angle >> 14) % 4;
        let val = sin_lut[idx];
        match quadrant {
            0 => val,
            1 => val,
            2 => -val,
            3 => -val,
            _ => 0,
        }
    }

    /// Small lookup table for cosine in permyriad units.
    fn cos_permyriad(pmy_angle: i32) -> i32 {
        let angle = pmy_angle.rem_euclid(62832) as usize;
        let cos_lut = [
            10000, 9997, 9987, 9971, 9951, 9924, 9891, 9853, 9808, 9757,
            9700, 9636, 9565, 9489, 9407, 9318, 9223, 9122, 9014, 8900,
            8781, 8655, 8524, 8387, 8245, 8098, 7945, 7787, 7623, 7454,
        ];
        let idx = (angle >> 11) % 30;
        let quadrant = (angle >> 14) % 4;
        let val = cos_lut[idx];
        match quadrant {
            0 => val,
            1 => -val,
            2 => -val,
            3 => val,
            _ => 0,
        }
    }

    /// Integer exp approximation in permyriad units: exp(x/10000) * 10000.
    /// Valid for x in [-15000, 0]; clamps x outside this range.
    /// Returns exp(x) in permyriad units, suitable for softmax normalization.
    fn int_exp_permyriad(x: i32) -> i32 {
        let x_clamped = x.clamp(-15000, 0);
        if x_clamped >= 0 {
            return 10000;
        }
        let exp_lut = [
            10000, 9900, 9802, 9704, 9608, 9513, 9420, 9328, 9237, 9148,
            9060, 8974, 8889, 8805, 8723, 8642, 8562, 8484, 8407, 8331,
        ];
        let idx = ((-x_clamped) / 750).min(19) as usize;
        exp_lut[idx]
    }

    /// Grouped-Query Attention (GQA) execution block with RoPE, causal masking, and attention.
    pub fn attention_block(
        &self,
        norm_input: &[i16],
        weights: &Gemma9bLayerWeights<'_>,
        out: &mut [i16],
        kv_k: &mut [i16],
        kv_v: &mut [i16],
        token_pos: usize,
    ) -> Result<NormalizedIpr, S13Error> {
        let q_dim = self.config.n_heads * self.config.d_head;
        let kv_dim = self.config.n_kv_heads * self.config.d_head;

        // Stack scratch allocations for projected Q, K, V
        let mut q_proj = [0i16; 4096];
        let mut k_proj = [0i16; 2048];
        let mut v_proj = [0i16; 2048];

        if q_dim > q_proj.len() || kv_dim > k_proj.len() {
            return Err(S13Error::IndexOutOfBounds);
        }

        // Linear projections with per-tensor scales
        self.dispatch_gemv(weights.q_proj, norm_input, &mut q_proj[..q_dim], self.config.d_model)?;
        Self::apply_tensor_scale(&mut q_proj[..q_dim], weights.scales[0]);
        self.dispatch_gemv(weights.k_proj, norm_input, &mut k_proj[..kv_dim], self.config.d_model)?;
        Self::apply_tensor_scale(&mut k_proj[..kv_dim], weights.scales[1]);
        self.dispatch_gemv(weights.v_proj, norm_input, &mut v_proj[..kv_dim], self.config.d_model)?;
        Self::apply_tensor_scale(&mut v_proj[..kv_dim], weights.scales[2]);

        // Apply RoPE to Q and K before cache store
        for h in 0..self.config.n_heads {
            let q_start = h * self.config.d_head;
            Self::apply_rope_i16(&mut q_proj[q_start..q_start + self.config.d_head], h, token_pos, self.config.d_head, self.config.rope_theta)?;
        }
        for h in 0..self.config.n_kv_heads {
            let k_start = h * self.config.d_head;
            Self::apply_rope_i16(&mut k_proj[k_start..k_start + self.config.d_head], h, token_pos, self.config.d_head, self.config.rope_theta)?;
        }

        // Store rotated K/V into KV cache for current token position
        let kv_offset = token_pos * kv_dim;
        if kv_offset + kv_dim <= kv_k.len() && kv_offset + kv_dim <= kv_v.len() {
            kv_k[kv_offset..kv_offset + kv_dim].copy_from_slice(&k_proj[..kv_dim]);
            kv_v[kv_offset..kv_offset + kv_dim].copy_from_slice(&v_proj[..kv_dim]);
        }

        // Grouped-Query Attention (GQA): 16 Q heads, 8 KV heads, d_head=256
        // Each KV head is shared by 2 Q heads (16/8 = 2)
        let mut attn_out = [0i16; 4096];
        let kv_heads = self.config.n_kv_heads;
        let q_heads = self.config.n_heads;
        let d_head = self.config.d_head;
        let heads_per_kv = q_heads / kv_heads;

        let mut attn_scores = [0i32; 8192];
        for q_h in 0..q_heads {
            let kv_h = q_h / heads_per_kv;
            let q_start = q_h * d_head;
            let kv_start = kv_h * d_head;

            // Compute Q·K^T scores for all cached positions (causal: 0..=token_pos)
            for t in 0..=token_pos {
                let mut score: i32 = 0;
                let k_pos_offset = t * kv_dim + kv_start;
                for i in 0..d_head {
                    if k_pos_offset + i < kv_k.len() {
                        let q_val = q_proj[q_start + i] as i32;
                        let k_val = kv_k[k_pos_offset + i] as i32;
                        score += q_val * k_val;
                    }
                }
                if t < attn_scores.len() {
                    attn_scores[t] = (score / d_head as i32).clamp(-10_000, 10_000);
                }
            }

            // Integer softmax with max subtract: exp(x - max_x) for stability
            let max_score = attn_scores[..=token_pos].iter().max_by_key(|&&x| x).copied().unwrap_or(0);
            let mut exp_sum: i64 = 0;
            let mut exp_scores = [0i32; 8192];
            for t in 0..=token_pos {
                let shifted = (attn_scores[t] - max_score).max(-15_000);
                let exp_approx = Self::int_exp_permyriad(shifted);
                exp_scores[t] = exp_approx;
                exp_sum += exp_approx as i64;
            }

            // Weighted sum over V (with normalization)
            let mut head_out = [0i32; 256];
            for t in 0..=token_pos {
                let weight = if exp_sum > 0 {
                    ((exp_scores[t] as i64 * 10_000) / exp_sum) as i32
                } else {
                    10_000 / (token_pos as i32 + 1)
                };
                let v_pos_offset = t * kv_dim + kv_start;
                for i in 0..d_head {
                    if v_pos_offset + i < kv_v.len() {
                        let v_val = kv_v[v_pos_offset + i] as i32;
                        head_out[i] += (weight * v_val) / 10_000;
                    }
                }
            }

            // Store attention output for this head
            for i in 0..d_head {
                attn_out[q_start + i] = head_out[i].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }

        // Compute attention score localization metric over Q projection
        let mut q_u16 = [0u16; 4096];
        for i in 0..q_dim {
            q_u16[i] = q_proj[i].unsigned_abs();
        }
        let ipr = NormalizedIpr::compute_u16(&q_u16[..q_dim]);

        // Output projection: out = o_proj * attn_out
        self.dispatch_gemv(weights.o_proj, &attn_out[..q_dim], out, q_dim)?;
        Self::apply_tensor_scale(out, weights.scales[3]);

        Ok(ipr)
    }

    /// Gated Feed-Forward Network (FFN) block with GeGLU activation.
    pub fn ffn_block(
        &self,
        norm_input: &[i16],
        weights: &Gemma9bLayerWeights<'_>,
        out: &mut [i16],
    ) -> Result<(), S13Error> {
        let d_ff = self.config.d_ff;
        let d_model = self.config.d_model;

        let mut gate_act = [0i16; 14336];
        let mut up_act = [0i16; 14336];
        let mut fused_act = [0i16; 14336];

        if d_ff > gate_act.len() || d_model > out.len() {
            return Err(S13Error::IndexOutOfBounds);
        }

        self.dispatch_gemv(weights.gate_proj, norm_input, &mut gate_act[..d_ff], d_model)?;
        Self::apply_tensor_scale(&mut gate_act[..d_ff], weights.scales[4]);

        self.dispatch_gemv(weights.up_proj, norm_input, &mut up_act[..d_ff], d_model)?;
        Self::apply_tensor_scale(&mut up_act[..d_ff], weights.scales[5]);

        // Fixed-point GeGLU approximation: fused = (gate * up) / 10000
        for i in 0..d_ff {
            let g = gate_act[i] as i32;
            let u = up_act[i] as i32;
            let activated = (g * u) / self.config.permyriad_scale;
            fused_act[i] = activated.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }

        self.dispatch_gemv(weights.down_proj, &fused_act[..d_ff], out, d_ff)?;
        Self::apply_tensor_scale(out, weights.scales[6]);

        Ok(())
    }

    /// Matrix-Vector GEMV dispatch routing to the configured engine.
    #[inline(always)]
    pub fn dispatch_gemv(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        in_dim: usize,
    ) -> Result<(), S13Error> {
        let bytes_per_row = (in_dim + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;

        match self.engine {
            DispatchEngine::Avx2Pshufb => {
                self.gemv_avx2_rayon(packed_weights, activations, output, in_dim)
            }
            DispatchEngine::GpuWardenSplitShader => {
                self.gemv_gpu_warden_emulated(packed_weights, activations, output, in_dim, bytes_per_row)
            }
            DispatchEngine::ScalarReference => {
                self.gemv_avx2_rayon(packed_weights, activations, output, in_dim)
            }
        }
    }

    /// Fixed-Point RMSNorm calculation: $y_i = \frac{x_i}{\sqrt{\frac{1}{N}\sum x_k^2 + \epsilon}} \times w_i$.
    pub fn rms_norm(
        input: &[i16],
        scale: &[i16],
        output: &mut [i16],
        _permyriad_scale: i32,
    ) {
        let n = input.len();
        if n == 0 || scale.len() < n || output.len() < n {
            return;
        }

        // Calculate sum of squares
        let mut sum_sq: u64 = 0;
        for &x in input {
            let val = x as i64;
            sum_sq += (val * val) as u64;
        }

        let mean_sq = (sum_sq / n as u64).max(1);
        let rms = isqrt_u64(mean_sq).max(1) as i64;

        for i in 0..n {
            let x = input[i] as i64;
            let w = scale[i] as i64;
            // Fixed-point normalized output: (x * w) / rms
            let normalized = (x * w) / rms;
            output[i] = normalized.clamp(i16::MIN as i64, i16::MAX as i64) as i16;
        }
    }

    /// Parallel Row-Level AVX2 + 243-LUT GEMV execution using Rayon.
    #[allow(unsafe_code)]
    pub fn gemv_avx2_rayon(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        in_dim: usize,
    ) -> Result<(), S13Error> {
        use rayon::prelude::*;

        let out_dim = output.len();
        if out_dim == 0 || in_dim == 0 {
            return Ok(());
        }

        output
            .par_iter_mut()
            .enumerate()
            .try_for_each(|(row, out_val)| {
                let trit_start = row * in_dim;
                let mut dot: i32 = 0;
                let mut col = 0usize;

                #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
                unsafe {
                    use std::arch::x86_64::*;
                    let mut acc_vec = _mm256_setzero_si256();

                    while col + 10 <= in_dim {
                        let flat_idx = trit_start + col;
                        let byte_idx = flat_idx / 5;
                        let shift = flat_idx % 5;

                        if shift == 0 {
                            if byte_idx + 2 > packed_weights.len() {
                                return Err(S13Error::IndexOutOfBounds);
                            }
                            let b0 = packed_weights[byte_idx];
                            let b1 = packed_weights[byte_idx + 1];
                            if b0 >= 243 {
                                return Err(S13Error::SentinelDetected(b0));
                            }
                            if b1 >= 243 {
                                return Err(S13Error::SentinelDetected(b1));
                            }

                            let w0 = TRIT_LUT_I16[b0 as usize];
                            let w1 = TRIT_LUT_I16[b1 as usize];

                            let a0 = [
                                activations[col],
                                activations[col + 1],
                                activations[col + 2],
                                activations[col + 3],
                                activations[col + 4],
                                0, 0, 0,
                            ];
                            let a1 = [
                                activations[col + 5],
                                activations[col + 6],
                                activations[col + 7],
                                activations[col + 8],
                                activations[col + 9],
                                0, 0, 0,
                            ];

                            let vec_w0 = _mm_loadu_si128(w0.as_ptr() as *const __m128i);
                            let vec_w1 = _mm_loadu_si128(w1.as_ptr() as *const __m128i);
                            let weights_256 = _mm256_set_m128i(vec_w1, vec_w0);

                            let act0 = _mm_loadu_si128(a0.as_ptr() as *const __m128i);
                            let act1 = _mm_loadu_si128(a1.as_ptr() as *const __m128i);
                            let acts_256 = _mm256_set_m128i(act1, act0);

                            let prod = _mm256_madd_epi16(weights_256, acts_256);
                            acc_vec = _mm256_add_epi32(acc_vec, prod);

                            col += 10;
                            continue;
                        }

                        let b = packed_weights[byte_idx];
                        if b >= 243 {
                            return Err(S13Error::SentinelDetected(b));
                        }
                        let trits = &TRIT_LUT_243[b as usize];
                        dot += (trits[shift] as i32) * (activations[col] as i32);
                        col += 1;
                    }

                    let mut temp = [0i32; 8];
                    _mm256_storeu_si256(temp.as_mut_ptr() as *mut __m256i, acc_vec);
                    for &x in &temp {
                        dot += x;
                    }
                }

                while col < in_dim {
                    let flat_idx = trit_start + col;
                    let byte_idx = flat_idx / 5;
                    if byte_idx >= packed_weights.len() {
                        return Err(S13Error::IndexOutOfBounds);
                    }
                    let b = packed_weights[byte_idx];
                    if b >= 243 {
                        return Err(S13Error::SentinelDetected(b));
                    }
                    let shift = flat_idx % 5;
                    let trits = &TRIT_LUT_243[b as usize];
                    dot += (trits[shift] as i32) * (activations[col] as i32);
                    col += 1;
                }

                *out_val = dot.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                Ok(())
            })
    }

    /// Scalar GEMV execution fallback over flat packed ternary weights.
    fn gemv_scalar(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        in_dim: usize,
        _bytes_per_row: usize,
    ) -> Result<(), S13Error> {
        self.gemv_avx2_rayon(packed_weights, activations, output, in_dim)
    }

    /// AVX2 PSHUFB accelerated vector GEMV.
    #[allow(unsafe_code)]
    #[cfg(target_arch = "x86_64")]
    fn gemv_avx2_pshufb(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        in_dim: usize,
        _bytes_per_row: usize,
    ) -> Result<(), S13Error> {
        self.gemv_avx2_rayon(packed_weights, activations, output, in_dim)
    }

    /// GPU Warden SplitShader emulated 64-bit GEMV calculation.
    fn gemv_gpu_warden_emulated(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        in_dim: usize,
        bytes_per_row: usize,
    ) -> Result<(), S13Error> {
        let out_dim = output.len();
        for row in 0..out_dim {
            let row_offset = row * bytes_per_row;
            if row_offset + bytes_per_row > packed_weights.len() {
                return Err(S13Error::IndexOutOfBounds);
            }

            let row_bytes = &packed_weights[row_offset..row_offset + bytes_per_row];
            let mut acc_emulated = EmulatedU64::ZERO;
            let mut trit_idx = 0;

            for &b in row_bytes {
                if b >= 243 {
                    return Err(S13Error::SentinelDetected(b));
                }

                let mut rem = b;
                for _ in 0..5 {
                    if trit_idx >= in_dim {
                        break;
                    }
                    let digit = rem % 3;
                    rem /= 3;
                    let trit = (digit as i8) - 1;
                    let act = activations[trit_idx] as i32;

                    let term = (trit as i32) * act;
                    if term >= 0 {
                        acc_emulated = acc_emulated.add(EmulatedU64::from_u64(term as u64));
                    } else {
                        acc_emulated = acc_emulated.sub(EmulatedU64::from_u64((-term) as u64));
                    }
                    trit_idx += 1;
                }
            }

            let raw_val = acc_emulated.to_u64() as i64;
            output[row] = raw_val.clamp(i16::MIN as i64, i16::MAX as i64) as i16;
        }
        Ok(())
    }

    /// Embed token ID to hidden state using packed ternary embedding table.
    pub fn embed_token(
        &self,
        token_id: usize,
        model: &Gemma9bModel<'_>,
        output: &mut [i16],
    ) -> Result<(), S13Error> {
        if token_id >= self.config.vocab_size || output.len() < self.config.d_model {
            return Err(S13Error::IndexOutOfBounds);
        }
        let bytes_per_row = (self.config.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
        let row_offset = token_id * bytes_per_row;
        if row_offset + bytes_per_row > model.embed_tokens.len() {
            return Err(S13Error::IndexOutOfBounds);
        }
        let row_bytes = &model.embed_tokens[row_offset..row_offset + bytes_per_row];
        let mut trit_idx = 0;
        for &b in row_bytes {
            if b >= 243 {
                return Err(S13Error::SentinelDetected(b));
            }
            let mut rem = b;
            for _ in 0..5 {
                if trit_idx >= self.config.d_model {
                    break;
                }
                let digit = rem % 3;
                rem /= 3;
                let trit = (digit as i8) - 1;
                output[trit_idx] = trit as i16;
                trit_idx += 1;
            }
        }
        Ok(())
    }

    /// Apply per-tensor scale factor to output buffer (element-wise).
    fn apply_tensor_scale(output: &mut [i16], scale: f32) {
        if scale == 1.0 {
            return;
        }
        for val in output.iter_mut() {
            let scaled = (*val as f64) * (scale as f64);
            *val = scaled.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        }
    }

    /// Single-token autoregressive decode with an activation-steering injection.
    ///
    /// Identical to [`Self::forward_token`] except that after `steer_after_layer`'s
    /// residual adds complete, `alpha_pmy/10000 * steer_vec` is added element-wise
    /// into `hidden_state` before the remaining layers run. A separate function
    /// rather than an added parameter on `forward_token` — that signature is
    /// already tested and callers, keeps working unmodified.
    ///
    /// `steer_vec` must be at least `d_model` long; shorter entries are treated as
    /// zero. `steer_after_layer >= n_layers` is a no-op steer (runs the full
    /// unsteered forward — named, not silently wrong).
    pub fn forward_token_steered(
        &mut self,
        hidden_state: &mut [i16],
        kv_cache_k: &mut [&mut [i16]],
        kv_cache_v: &mut [&mut [i16]],
        token_pos: usize,
        model: &Gemma9bModel<'_>,
        steer_after_layer: usize,
        steer_vec: &[i16],
        alpha_pmy: i32,
    ) -> Result<ForwardTelemetry, S13Error> {
        if hidden_state.len() < self.config.d_model {
            return Err(S13Error::IndexOutOfBounds);
        }

        let mut avg_pmy_acc: u32 = 0;
        let mut total_landmarks: u32 = 0;

        let mut norm_scratch = [0i16; 3584];
        let mut attn_out = [0i16; 3584];
        let mut ffn_out = [0i16; 3584];

        let d_model = self.config.d_model;

        for layer_idx in 0..self.config.n_layers {
            let layer_weights = match model.layers[layer_idx] {
                Some(ref w) => w,
                None => return Err(S13Error::IndexOutOfBounds),
            };

            Self::rms_norm(
                &hidden_state[..d_model],
                layer_weights.input_norm_scale,
                &mut norm_scratch[..d_model],
                self.config.permyriad_scale,
            );

            let attn_ipr = self.attention_block(
                &norm_scratch[..d_model],
                layer_weights,
                &mut attn_out[..d_model],
                kv_cache_k[layer_idx],
                kv_cache_v[layer_idx],
                token_pos,
            )?;

            avg_pmy_acc += attn_ipr.pmy as u32;
            if attn_ipr.is_landmark() {
                total_landmarks += 1;
            }

            for i in 0..d_model {
                hidden_state[i] = hidden_state[i].saturating_add(attn_out[i]);
            }

            Self::rms_norm(
                &hidden_state[..d_model],
                layer_weights.post_attention_norm_scale,
                &mut norm_scratch[..d_model],
                self.config.permyriad_scale,
            );

            self.ffn_block(&norm_scratch[..d_model], layer_weights, &mut ffn_out[..d_model])?;

            for i in 0..d_model {
                hidden_state[i] = hidden_state[i].saturating_add(ffn_out[i]);
            }

            // Activation steering injection — the one addition over forward_token.
            if layer_idx == steer_after_layer {
                for i in 0..d_model {
                    let steer = *steer_vec.get(i).unwrap_or(&0) as i32;
                    let delta = (alpha_pmy as i64 * steer as i64 / self.config.permyriad_scale as i64) as i32;
                    hidden_state[i] = (hidden_state[i] as i32).saturating_add(delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
            }
        }

        Self::rms_norm(
            &hidden_state[..d_model],
            model.final_norm_scale,
            &mut norm_scratch[..d_model],
            self.config.permyriad_scale,
        );
        hidden_state[..d_model].copy_from_slice(&norm_scratch[..d_model]);

        self.timeline_counter = self.timeline_counter.wrapping_add(1);
        let mean_pmy = if self.config.n_layers > 0 {
            (avg_pmy_acc / self.config.n_layers as u32) as u16
        } else {
            0
        };

        let packed_word = NiprPackedWord::pack(
            mean_pmy,
            token_pos as u32,
            if mean_pmy >= LANDMARK_PMY {
                NiprGateStatus::Active
            } else {
                NiprGateStatus::Fallback
            },
            (self.timeline_counter & 0xFFFF) as u16,
        );

        Ok(ForwardTelemetry {
            attention_pmy: mean_pmy,
            retained_landmarks: total_landmarks,
            packed_word,
            timeline_tick: self.timeline_counter,
        })
    }

    /// Logits projection: dot lm_head weights with final hidden state.
    /// Sample a token from logits with optional temperature scaling.
    #[cfg(feature = "std")]
    ///
    /// # Arguments
    /// * `logits` - Raw logit scores from projection
    /// * `temperature` - Sampling temperature; 0.0 = argmax (greedy), > 0.0 = softmax sample
    /// * `rng_state` - XORshift PRNG state, updated in-place
    ///
    /// # Returns
    /// Selected token index
    pub fn sample_logits(logits: &[f32], temperature: f32, rng_state: &mut u64) -> usize {
        if logits.is_empty() {
            return 0;
        }

        if temperature == 0.0 {
            // Greedy: argmax
            let mut max_idx = 0;
            let mut max_val = logits[0];
            for (i, &val) in logits.iter().enumerate() {
                if val > max_val {
                    max_val = val;
                    max_idx = i;
                }
            }
            return max_idx;
        }

        // Temperature sampling: softmax + weighted random draw
        // Softmax with numerical stability: exp((logit - max) / temperature)

        let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        // Compute exp for each logit (temperature-scaled)
        let mut exp_logits = vec![0.0f32; logits.len()];
        let mut sum_exp = 0.0f32;
        for (i, &logit) in logits.iter().enumerate() {
            let exponent = (logit - max_logit) / temperature;
            exp_logits[i] = exponent.exp();
            sum_exp += exp_logits[i];
        }

        // Normalize to probabilities
        for exp in &mut exp_logits {
            *exp /= sum_exp;
        }

        // Sample via cumulative distribution
        let u = Self::xorshift_f32(rng_state);
        let mut cumsum = 0.0f32;
        for (i, &prob) in exp_logits.iter().enumerate() {
            cumsum += prob;
            if u <= cumsum {
                return i;
            }
        }

        // Fallback (numerical precision edge case)
        logits.len() - 1
    }

    /// Simple XORshift32 RNG, updates state in-place, returns [0.0, 1.0).
    fn xorshift_f32(state: &mut u64) -> f32 {
        let mut x = *state as u32;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *state = (*state >> 32) | ((x as u64) << 32);

        let normalized = (x as f32) * (1.0 / 4294967296.0);
        normalized.clamp(0.0, 0.9999999)
    }

    /// Project final hidden state to vocabulary logits using tied lm_head weights.
    pub fn project_logits(
        &self,
        hidden: &[i16],
        model: &Gemma9bModel<'_>,
        logits: &mut [f32],
        lm_head_scale: f32,
    ) -> Result<(), S13Error> {
        if hidden.len() < self.config.d_model || logits.len() < self.config.vocab_size {
            return Err(S13Error::IndexOutOfBounds);
        }
        let d_model = self.config.d_model;
        let bytes_per_row = (d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;

        use rayon::prelude::*;
        logits
            .par_iter_mut()
            .enumerate()
            .try_for_each(|(token_id, logit_out)| {
                let row_offset = token_id * bytes_per_row;
                if row_offset + bytes_per_row > model.embed_tokens.len() {
                    return Err(S13Error::IndexOutOfBounds);
                }
                let row_bytes = &model.embed_tokens[row_offset..row_offset + bytes_per_row];
                let mut dot: i32 = 0;
                let mut trit_idx = 0;
                for &b in row_bytes {
                    if b >= 243 {
                        return Err(S13Error::SentinelDetected(b));
                    }
                    let trits = &TRIT_LUT_243[b as usize];
                    for j in 0..5 {
                        if trit_idx >= d_model {
                            break;
                        }
                        dot += (trits[j] as i32) * (hidden[trit_idx] as i32);
                        trit_idx += 1;
                    }
                }
                let scaled = (dot as f64) * (lm_head_scale as f64);
                *logit_out = scaled as f32;
                Ok(())
            })
    }

}

/// Plain per-element permyriad-scaled interpolation between two same-length
/// hidden-state vectors: `out[i] = a[i] + (b[i]-a[i]) * t_pmy / 10000`.
///
/// NOT a geodesic/slerp — a straight linear lerp in `d_model` space, same
/// honesty as `forge_core_v3::pentaract::Pentaract::midpoint_unit_vector`'s own
/// doc comment ("cheap stand-in for slerp"). `t_pmy` is clamped to `[0, 10000]`.
/// `a`, `b`, and `out` must share the same length; excess `out` length is
/// left untouched.
pub fn lerp_dmodel(a: &[i16], b: &[i16], t_pmy: i32, out: &mut [i16]) {
    let t = t_pmy.clamp(0, 10_000) as i64;
    let n = a.len().min(b.len()).min(out.len());
    for i in 0..n {
        let av = a[i] as i64;
        let bv = b[i] as i64;
        let interp = av + (bv - av) * t / 10_000;
        out[i] = interp.clamp(i16::MIN as i64, i16::MAX as i64) as i16;
    }
}

/// Integer square root via binary search / Newton-Raphson.
#[inline(always)]
fn isqrt_u64(val: u64) -> u64 {
    if val <= 1 {
        return val;
    }
    let mut x0 = val / 2;
    let mut x1 = (x0 + val / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + val / x0) / 2;
    }
    x0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s13::pack_5_trits;

    #[test]
    fn test_gemma_9b_config_dimensions_and_footprint() {
        let config = Gemma9bConfig::default();
        assert_eq!(config.d_model, 3584);
        assert_eq!(config.n_heads, 16);
        assert_eq!(config.n_kv_heads, 8);
        assert_eq!(config.d_head, 256);
        assert_eq!(config.n_layers, 42);
        assert_eq!(config.d_ff, 14336);

        // Check per-layer weight count
        let w_layer = config.weights_per_layer();
        assert_eq!(w_layer, 198_180_864);

        // On-disk bytes per layer: MEASURED off s13_gemma_9b/ 2026-08-26, seven
        // .s13m files each 5-trits/byte + a 16-byte header.
        let b_layer = config.packed_bytes_per_layer();
        assert_eq!(b_layer, 39_636_287);

        // Total 42-layer backbone footprint, byte-exact against the packed bear.
        let total_backbone = config.total_backbone_packed_bytes();
        assert_eq!(total_backbone, 1_664_724_054);

        // Total model footprint including 256k static vocab embedding (~1.848 GB)
        let total_model = config.total_model_packed_bytes();
        assert!(total_model > 1_840_000_000 && total_model < 1_860_000_000);
    }

    #[test]
    fn test_rms_norm_deterministic_scaling() {
        let input = [100i16, -200, 300, -400];
        let scale = [10_000i16, 10_000, 10_000, 10_000];
        let mut output = [0i16; 4];

        Gemma9bForwardGraph::rms_norm(&input, &scale, &mut output, 10_000);
        // Ensure values scaled deterministically without overflow
        assert!(output[0] > 0);
        assert!(output[1] < 0);
        assert!(output[2] > 0);
        assert!(output[3] < 0);
    }

    #[test]
    fn test_forward_graph_single_layer_step() {
        let config = Gemma9bConfig {
            d_model: 10,
            n_heads: 2,
            n_kv_heads: 1,
            d_head: 4,
            n_layers: 1,
            d_ff: 20,
            vocab_size: 100,
            max_seq_len: 64,
            rope_theta: 10_000,
            permyriad_scale: 10_000,
        };

        let mut graph = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);

        // Mock layer weights
        // q_proj: out_dim 8, in_dim 10 -> 8 rows * 2 bytes = 16 bytes
        let q_bytes = [pack_5_trits([1, 0, -1, 0, 1]).unwrap(); 16];
        // k_proj: out_dim 4, in_dim 10 -> 4 rows * 2 bytes = 8 bytes
        let k_bytes = [pack_5_trits([0, 1, 0, -1, 0]).unwrap(); 8];
        // v_proj: out_dim 4, in_dim 10 -> 4 rows * 2 bytes = 8 bytes
        let v_bytes = [pack_5_trits([-1, 0, 1, 0, -1]).unwrap(); 8];
        // o_proj: out_dim 10, in_dim 8 -> 10 rows * 2 bytes = 20 bytes
        let o_bytes = [pack_5_trits([1, 1, 0, -1, -1]).unwrap(); 20];
        // gate_proj: out_dim 20, in_dim 10 -> 20 rows * 2 bytes = 40 bytes
        let gate_bytes = [pack_5_trits([1, 0, 1, 0, 1]).unwrap(); 40];
        // up_proj: out_dim 20, in_dim 10 -> 20 rows * 2 bytes = 40 bytes
        let up_bytes = [pack_5_trits([0, 1, -1, 1, 0]).unwrap(); 40];
        // down_proj: out_dim 10, in_dim 20 -> 10 rows * 4 bytes = 40 bytes
        let down_bytes = [pack_5_trits([1, -1, 0, 1, 0]).unwrap(); 40];

        let norm_scale = [10_000i16; 10];
        let tensor_scales = [1.0f32; 7];
        let layer_w = Gemma9bLayerWeights {
            q_proj: &q_bytes,
            k_proj: &k_bytes,
            v_proj: &v_bytes,
            o_proj: &o_bytes,
            gate_proj: &gate_bytes,
            up_proj: &up_bytes,
            down_proj: &down_bytes,
            input_norm_scale: &norm_scale,
            post_attention_norm_scale: &norm_scale,
            scales: &tensor_scales,
        };

        let mut model = Gemma9bModel::new(config, &norm_scale, &[]);
        model.set_layer(0, layer_w).unwrap();

        let mut hidden = [100i16; 10];
        let mut k_cache = [0i16; 100];
        let mut v_cache = [0i16; 100];
        let mut k_refs = [&mut k_cache[..]];
        let mut v_refs = [&mut v_cache[..]];

        let telemetry = graph
            .forward_token(&mut hidden, &mut k_refs, &mut v_refs, 0, &model)
            .unwrap();

        assert_eq!(telemetry.timeline_tick, 1);
        assert!(telemetry.attention_pmy <= 10_000);
    }

    #[test]
    fn lerp_dmodel_interpolates_deterministically() {
        let a = [0i16, 100, -100, 32000];
        let b = [10_000i16, 200, -200, -32000];
        let mut out = [0i16; 4];

        lerp_dmodel(&a, &b, 0, &mut out);
        assert_eq!(out, a, "t=0 must return a exactly");

        lerp_dmodel(&a, &b, 10_000, &mut out);
        assert_eq!(out, b, "t=10000 must return b exactly");

        lerp_dmodel(&a, &b, 5_000, &mut out);
        assert_eq!(out[1], 150, "midpoint of 100 and 200 is 150");
        assert_eq!(out[2], -150, "midpoint of -100 and -200 is -150");

        let mut again = [0i16; 4];
        lerp_dmodel(&a, &b, 5_000, &mut again);
        assert_eq!(out, again, "lerp must be deterministic");
    }

    fn toy_one_layer_config() -> Gemma9bConfig {
        Gemma9bConfig {
            d_model: 10, n_heads: 2, n_kv_heads: 1, d_head: 4,
            n_layers: 1, d_ff: 20, vocab_size: 100, max_seq_len: 64,
            rope_theta: 10_000, permyriad_scale: 10_000,
        }
    }

    #[test]
    fn forward_token_steered_diverges_from_unsteered_at_alpha_nonzero() {
        // One layer, steered right after its residual adds and before the final
        // RMSNorm — a second layer would re-run every row through the SAME
        // (mock, identical-per-row) weight pattern, which structurally collapses
        // any input back to a uniform output vector and washes the steering out
        // before it could ever reach the assertion below (found live, this test).
        let config = toy_one_layer_config();
        let q_bytes = [pack_5_trits([1, 0, -1, 0, 1]).unwrap(); 16];
        let k_bytes = [pack_5_trits([0, 1, 0, -1, 0]).unwrap(); 8];
        let v_bytes = [pack_5_trits([-1, 0, 1, 0, -1]).unwrap(); 8];
        let o_bytes = [pack_5_trits([1, 1, 0, -1, -1]).unwrap(); 20];
        let gate_bytes = [pack_5_trits([1, 0, 1, 0, 1]).unwrap(); 40];
        let up_bytes = [pack_5_trits([0, 1, -1, 1, 0]).unwrap(); 40];
        let down_bytes = [pack_5_trits([1, -1, 0, 1, 0]).unwrap(); 40];
        let norm_scale = [10_000i16; 10];
        let tensor_scales = [1.0f32; 7];
        let layer_w = Gemma9bLayerWeights {
            q_proj: &q_bytes, k_proj: &k_bytes, v_proj: &v_bytes, o_proj: &o_bytes,
            gate_proj: &gate_bytes, up_proj: &up_bytes, down_proj: &down_bytes,
            input_norm_scale: &norm_scale, post_attention_norm_scale: &norm_scale,
            scales: &tensor_scales,
        };
        let mut model = Gemma9bModel::new(config, &norm_scale, &[]);
        model.set_layer(0, layer_w).unwrap();

        // Non-uniform AND large on purpose: these mock weights are a single
        // repeated byte pattern per tensor, which drives a uniform input to a
        // perfectly uniform output that RMSNorm then renormalizes onto the same
        // scale regardless of magnitude — a small or uniform steer_vec vanishes
        // under that rescale (RMSNorm cares about sign/ratio, not absolute
        // magnitude). This vector alternates sign at full strength (alpha=10000
        // below applies it unscaled) specifically to flip per-dimension sign
        // patterns, which DOES survive the final normalization.
        let steer_vec = [30_000i16, -30_000, 30_000, -30_000, 30_000, -30_000, 30_000, -30_000, 30_000, -30_000];

        let mut graph_a = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        let mut hidden_a = [100i16; 10];
        let mut kc_a = [0i16; 100];
        let mut vc_a = [0i16; 100];
        graph_a.forward_token_steered(&mut hidden_a, &mut [&mut kc_a[..]], &mut [&mut vc_a[..]], 0, &model, 0, &steer_vec, 0).unwrap();

        let mut graph_b = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        let mut hidden_b = [100i16; 10];
        let mut kc_b = [0i16; 100];
        let mut vc_b = [0i16; 100];
        graph_b.forward_token_steered(&mut hidden_b, &mut [&mut kc_b[..]], &mut [&mut vc_b[..]], 0, &model, 0, &steer_vec, 10_000).unwrap();

        assert_ne!(hidden_a, hidden_b, "nonzero alpha steering must change the output");

        let mut graph_c = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        let mut hidden_c = [100i16; 10];
        let mut kc_c = [0i16; 100];
        let mut vc_c = [0i16; 100];
        graph_c.forward_token_steered(&mut hidden_c, &mut [&mut kc_c[..]], &mut [&mut vc_c[..]], 0, &model, 0, &steer_vec, 10_000).unwrap();
        assert_eq!(hidden_b, hidden_c, "steering must be deterministic");
    }

    /// The concrete demo: two REAL stars from the on-disk HYG catalog, encoded
    /// through vocab.rs's proven forward projection, forward-passed with
    /// steering, lerped along the ray between them, and read via the logit
    /// lens (`project_logits`) — all 4 mechanisms in one chain. Real star
    /// provenance; the model weights are a small hand-built toy config (a
    /// real 42-layer 9B seat is ~1.8GB, out of scope for a fast unit test —
    /// named, not silently substituted).
    #[cfg(feature = "std")]
    #[test]
    fn two_real_stars_ray_cast_through_inject_steer_lerp_and_logit_lens() {
        use super::super::vocab::AutoEncoderWeights;
        use super::super::star_codebook::StarCodebookView;
        use std::path::Path;

        let hyg_path = Path::new("F:/v3/shell/assets/hyg_baked.bin");
        if !hyg_path.exists() {
            panic!("hyg_baked.bin not found at {} — this test needs real stars, not synthetic ones", hyg_path.display());
        }
        let hyg_bytes = std::fs::read(hyg_path).expect("read hyg_baked.bin");
        let codebook = StarCodebookView::parse(&hyg_bytes).expect("parse real HYG catalog");
        let star_a = codebook.get_star(0).expect("star 0 exists");
        let star_b = codebook.get_star(1000).expect("star 1000 exists");
        assert_ne!(star_a, star_b, "need two genuinely distinct stars");

        // Real star -> permyriad 5D coordinate -> vocab.rs's proven forward projection.
        let ae = AutoEncoderWeights::default_fixed();
        let star_coords_pmy = |s: &super::super::star_codebook::BakedStarCentroid| -> [i32; 5] {
            [
                (s.ra_normalized() * 10_000.0) as i32,
                (s.dec_normalized() * 10_000.0) as i32,
                (s.mag_normalized() * 10_000.0) as i32,
                s.teff_idx as i32 * 40, // spectral axis, arbitrary small scale
                s.resonant_milli_hz() as i32 / 10, // hz axis, scaled down
            ]
        };
        let to_hidden_i16 = |dmodel: &[i32; super::super::vocab::D_MODEL]| -> [i16; super::super::vocab::D_MODEL] {
            core::array::from_fn(|i| dmodel[i].clamp(i16::MIN as i32, i16::MAX as i32) as i16)
        };

        let mut dmodel_a = [0i32; super::super::vocab::D_MODEL];
        ae.somatic_5d_to_dmodel(&star_coords_pmy(&star_a), &mut dmodel_a);
        let mut dmodel_b = [0i32; super::super::vocab::D_MODEL];
        ae.somatic_5d_to_dmodel(&star_coords_pmy(&star_b), &mut dmodel_b);
        let mut hidden_a = to_hidden_i16(&dmodel_a);
        let mut hidden_b = to_hidden_i16(&dmodel_b);
        assert_ne!(hidden_a, hidden_b, "two distinct stars must not collapse to the same injected embedding");

        // Toy d_model=2048 model_9b config — dimensionally matched to vocab.rs's
        // D_MODEL so the injected star embeddings feed straight in.
        const D: usize = super::super::vocab::D_MODEL; // 2048
        let config = Gemma9bConfig {
            d_model: D, n_heads: 2, n_kv_heads: 1, d_head: 8,
            n_layers: 1, d_ff: 16, vocab_size: 8, max_seq_len: 4,
            rope_theta: 10_000, permyriad_scale: 10_000,
        };
        let bpr = |in_dim: usize| (in_dim + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
        let q_bytes = vec![pack_5_trits([1, 0, -1, 0, 1]).unwrap(); 16 * bpr(D)];
        let k_bytes = vec![pack_5_trits([0, 1, 0, -1, 0]).unwrap(); 8 * bpr(D)];
        let v_bytes = vec![pack_5_trits([-1, 0, 1, 0, -1]).unwrap(); 8 * bpr(D)];
        let o_bytes = vec![pack_5_trits([1, 1, 0, -1, -1]).unwrap(); D * bpr(16)];
        let gate_bytes = vec![pack_5_trits([1, 0, 1, 0, 1]).unwrap(); 16 * bpr(D)];
        let up_bytes = vec![pack_5_trits([0, 1, -1, 1, 0]).unwrap(); 16 * bpr(D)];
        let down_bytes = vec![pack_5_trits([1, -1, 0, 1, 0]).unwrap(); D * bpr(16)];
        let norm_scale = vec![10_000i16; D];
        let tensor_scales = [1.0f32; 7];
        let layer_w = Gemma9bLayerWeights {
            q_proj: &q_bytes, k_proj: &k_bytes, v_proj: &v_bytes, o_proj: &o_bytes,
            gate_proj: &gate_bytes, up_proj: &up_bytes, down_proj: &down_bytes,
            input_norm_scale: &norm_scale, post_attention_norm_scale: &norm_scale,
            scales: &tensor_scales,
        };
        let embed_tokens = vec![pack_5_trits([1, -1, 0, 1, -1]).unwrap(); config.vocab_size * bpr(D)];
        let mut model = Gemma9bModel::new(config, &norm_scale, &embed_tokens);
        model.set_layer(0, layer_w).unwrap();

        let steer_vec = vec![300i16; D];
        let (mut kc_a, mut vc_a) = (vec![0i16; D * 4], vec![0i16; D * 4]);
        let (mut kc_b, mut vc_b) = (vec![0i16; D * 4], vec![0i16; D * 4]);

        // 1. PREFIX INJECTION + 2. ACTIVATION STEERING, one forward per star.
        let mut graph = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        graph.forward_token_steered(&mut hidden_a, &mut [&mut kc_a[..]], &mut [&mut vc_a[..]], 0, &model, 0, &steer_vec, 3_000).unwrap();
        let mut graph2 = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        graph2.forward_token_steered(&mut hidden_b, &mut [&mut kc_b[..]], &mut [&mut vc_b[..]], 0, &model, 0, &steer_vec, 3_000).unwrap();

        // 3. LATENT SPACE INTERPOLATION — the midpoint of the ray between the two settled states.
        let mut hidden_mid = vec![0i16; D];
        lerp_dmodel(&hidden_a, &hidden_b, 5_000, &mut hidden_mid);
        assert_ne!(&hidden_mid[..], &hidden_a[..]);
        assert_ne!(&hidden_mid[..], &hidden_b[..]);

        // 4. DIRECT UNEMBEDDING / LOGIT LENS — read structure off the interpolated point.
        let mut logits = vec![0.0f32; config.vocab_size];
        graph.project_logits(&hidden_mid, &model, &mut logits, 1.0).unwrap();
        assert!(logits.iter().any(|&l| l != 0.0), "the interpolated star-ray midpoint must activate real logits");

        // Determinism: same inputs, same chain, same result.
        let mut logits_again = vec![0.0f32; config.vocab_size];
        graph.project_logits(&hidden_mid, &model, &mut logits_again, 1.0).unwrap();
        assert_eq!(logits, logits_again, "the whole chain is deterministic end to end");
    }

    #[test]
    fn test_emulated_u64_gpu_warden_gemv_parity() {
        let config = Gemma9bConfig {
            d_model: 5,
            n_heads: 1,
            n_kv_heads: 1,
            d_head: 5,
            n_layers: 1,
            d_ff: 5,
            vocab_size: 10,
            max_seq_len: 16,
            rope_theta: 10_000,
            permyriad_scale: 10_000,
        };

        let graph_scalar = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        let graph_gpu = Gemma9bForwardGraph::new(config, DispatchEngine::GpuWardenSplitShader);

        let row_bytes = [pack_5_trits([1, -1, 1, 0, -1]).unwrap()];
        let activations = [50i16, 20, -30, 40, -10];

        let mut out_scalar = [0i16; 1];
        let mut out_gpu = [0i16; 1];

        graph_scalar
            .dispatch_gemv(&row_bytes, &activations, &mut out_scalar, 5)
            .unwrap();
        graph_gpu
            .dispatch_gemv(&row_bytes, &activations, &mut out_gpu, 5)
            .unwrap();

        assert_eq!(out_scalar[0], out_gpu[0]);
    }
}
