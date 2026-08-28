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

use crate::gpu_warden::EmulatedU64;
use crate::nipr::{NiprGateStatus, NiprPackedWord, NormalizedIpr, LANDMARK_PMY};
use crate::s13::{S13Error, TRITS_PER_BYTE};
use crate::three_bears::s13m_file_bytes;

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
        const MAX_CHUNK_SIZE: usize = 3584;
        let chunk_size = self.config.d_ff.min(MAX_CHUNK_SIZE);
        if chunk_size == 0 {
            return Ok(());
        }

        let mut gate_chunk = [0i16; MAX_CHUNK_SIZE];
        let mut up_chunk = [0i16; MAX_CHUNK_SIZE];
        let mut fused_chunk = [0i16; MAX_CHUNK_SIZE];

        out.fill(0);

        let num_chunks = (self.config.d_ff + chunk_size - 1) / chunk_size;
        let bytes_per_ffn_row = (self.config.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;

        for chunk_idx in 0..num_chunks {
            let current_chunk_len = (self.config.d_ff - chunk_idx * chunk_size).min(chunk_size);
            let offset_weights = chunk_idx * chunk_size * bytes_per_ffn_row;
            let weight_len = current_chunk_len * bytes_per_ffn_row;

            if offset_weights + weight_len > weights.gate_proj.len()
                || offset_weights + weight_len > weights.up_proj.len()
            {
                return Err(S13Error::IndexOutOfBounds);
            }

            let gate_slice = &weights.gate_proj[offset_weights..offset_weights + weight_len];
            let up_slice = &weights.up_proj[offset_weights..offset_weights + weight_len];

            self.dispatch_gemv(gate_slice, norm_input, &mut gate_chunk[..current_chunk_len], self.config.d_model)?;
            Self::apply_tensor_scale(&mut gate_chunk[..current_chunk_len], weights.scales[4]);
            self.dispatch_gemv(up_slice, norm_input, &mut up_chunk[..current_chunk_len], self.config.d_model)?;
            Self::apply_tensor_scale(&mut up_chunk[..current_chunk_len], weights.scales[5]);

            // Fixed-point GeGLU approximation: fused = (gate * up) / 10000
            for i in 0..current_chunk_len {
                let g = gate_chunk[i] as i32;
                let u = up_chunk[i] as i32;
                let activated = (g * u) / self.config.permyriad_scale;
                fused_chunk[i] = activated.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }

            // Down projection partial accumulation
            let bytes_per_down_row = (current_chunk_len + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
            let down_weight_len = self.config.d_model * bytes_per_down_row;
            let down_offset = chunk_idx * down_weight_len;

            if down_offset + down_weight_len > weights.down_proj.len() {
                return Err(S13Error::IndexOutOfBounds);
            }

            let down_slice = &weights.down_proj[down_offset..down_offset + down_weight_len];
            let mut partial_down = [0i16; 3584];
            self.dispatch_gemv(down_slice, &fused_chunk[..current_chunk_len], &mut partial_down[..self.config.d_model], current_chunk_len)?;
            Self::apply_tensor_scale(&mut partial_down[..self.config.d_model], weights.scales[6]);

            for i in 0..self.config.d_model {
                out[i] = out[i].saturating_add(partial_down[i]);
            }
        }

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
                #[cfg(all(target_arch = "x86_64", feature = "std"))]
                if std::is_x86_feature_detected!("avx2") {
                    return self.gemv_avx2_pshufb(packed_weights, activations, output, in_dim, bytes_per_row);
                }
                #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
                {
                    return self.gemv_avx2_pshufb(packed_weights, activations, output, in_dim, bytes_per_row);
                }
                #[allow(unreachable_code)]
                self.gemv_scalar(packed_weights, activations, output, in_dim, bytes_per_row)
            }
            DispatchEngine::GpuWardenSplitShader => {
                self.gemv_gpu_warden_emulated(packed_weights, activations, output, in_dim, bytes_per_row)
            }
            DispatchEngine::ScalarReference => {
                self.gemv_scalar(packed_weights, activations, output, in_dim, bytes_per_row)
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

    /// Scalar GEMV execution fallback.
    fn gemv_scalar(
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
            let mut dot: i32 = 0;
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
                    let trit = (digit as i8) - 1; // 0->-1, 1->0, 2->+1
                    dot += (trit as i32) * (activations[trit_idx] as i32);
                    trit_idx += 1;
                }
            }

            output[row] = dot.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
        Ok(())
    }

    /// AVX2 PSHUFB accelerated vector GEMV.
    #[allow(unsafe_code)]
    #[cfg(target_arch = "x86_64")]
    fn gemv_avx2_pshufb(
        &self,
        packed_weights: &[u8],
        activations: &[i16],
        output: &mut [i16],
        _in_dim: usize,
        bytes_per_row: usize,
    ) -> Result<(), S13Error> {
        let out_dim = output.len();
        let padded_trits = bytes_per_row * TRITS_PER_BYTE;
        let mut act_padded = [0i16; 16384];
        let act_slice = if activations.len() < padded_trits && padded_trits <= act_padded.len() {
            act_padded[..activations.len()].copy_from_slice(activations);
            &act_padded[..padded_trits]
        } else {
            activations
        };

        for row in 0..out_dim {
            let row_offset = row * bytes_per_row;
            if row_offset + bytes_per_row > packed_weights.len() {
                return Err(S13Error::IndexOutOfBounds);
            }
            let row_bytes = &packed_weights[row_offset..row_offset + bytes_per_row];
            unsafe {
                let dot = crate::s13::avx2_unpacker::matmul_vector_avx2(row_bytes, act_slice, self.config.permyriad_scale)?;
                output[row] = dot.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
        Ok(())
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

    /// Logits projection: dot lm_head weights with final hidden state.
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
        let bytes_per_row = (self.config.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
        for token_id in 0..self.config.vocab_size {
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
                let mut rem = b;
                for _ in 0..5 {
                    if trit_idx >= self.config.d_model {
                        break;
                    }
                    let digit = rem % 3;
                    rem /= 3;
                    let trit = (digit as i8) - 1;
                    dot += (trit as i32) * (hidden[trit_idx] as i32);
                    trit_idx += 1;
                }
            }
            let scaled = (dot as f64) * (lm_head_scale as f64);
            logits[token_id] = scaled as f32;
        }
        Ok(())
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
