//! Sovereign Rust training engine for the full MoE architecture.
//!
//! Manual backpropagation — no autograd, no PyTorch, no external ML runtime.
//! f32 forward pass with gradient cache, Adam optimizer, gradient accumulation.
//!
//! Architecture: embed → trunk (2 layers) → router → top-1 expert (argmax) →
//!               MLP memory → DCGS → ForgetGate → LM head
//!
//! HEADER CORRECTED 2026-08-06: this line read "top-2 experts" while `forward`
//! has always selected ONE expert by argmax over the router probabilities
//! (see the `max_by(total_cmp)` dispatch), and `backward_add_into` routes the
//! gradient through that single selected expert. A header that disagrees with its
//! own body is the cheapest kind of false green — it gets quoted as fact by
//! anything that reads the module without reading the code, which is exactly how
//! it survived this long.
//!
//! Inventions: #79 (MoE routing), #111 (ForgetGate), #112 (DCGS),
//!             #116 (integer-exact training), #131 (quality-gated)

// ── Loss Functions ────────────────────────────────────────────────────────

/// Cross-entropy loss: -log(p[target])
pub fn cross_entropy(logits: &[f32], target: usize) -> (f32, Vec<f32>) {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|&v| (v - max).exp()).collect();
    let sum: f32 = exp.iter().sum::<f32>().max(1e-9);
    let probs: Vec<f32> = exp.iter().map(|&e| e / sum).collect();
    let loss = -(probs[target].max(1e-12)).ln();
    // Gradient of CE w.r.t. logits: probs - one_hot(target)
    let mut dlogits = probs.clone();
    dlogits[target] -= 1.0;
    (loss, dlogits)
}

/// KL divergence: sum(p * log(p/q)) — for bidirectional distillation
pub fn kl_divergence(p: &[f32], q: &[f32]) -> f32 {
    p.iter().zip(q.iter())
        .map(|(&pi, &qi)| {
            if pi > 1e-12 { pi * (pi / qi.max(1e-12)).ln() } else { 0.0 }
        })
        .sum()
}

/// Softmax
pub fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|&v| (v - max).exp()).collect();
    let sum: f32 = exp.iter().sum::<f32>().max(1e-9);
    exp.iter().map(|&e| e / sum).collect()
}

// ── Activation Functions + Derivatives ────────────────────────────────────

/// Gaussian Error Linear Unit activation.
pub fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + ((2.0 / std::f32::consts::PI).sqrt() * (x + 0.044715 * x.powi(3))).tanh())
}

/// GELU derivative.
pub fn gelu_grad(x: f32) -> f32 {
    let s = (2.0 / std::f32::consts::PI).sqrt() * (x + 0.044715 * x.powi(3));
    let t = s.tanh();
    let dt = 1.0 - t * t;
    let ds = (2.0 / std::f32::consts::PI).sqrt() * (1.0 + 3.0 * 0.044715 * x * x);
    0.5 * (1.0 + t) + 0.5 * x * dt * ds
}

/// Sigmoid Linear Unit activation.
pub fn silu(x: f32) -> f32 { x / (1.0 + (-x).exp()) }

/// SiLU derivative.
pub fn silu_grad(x: f32) -> f32 {
    let s = 1.0 / (1.0 + (-x).exp());
    s * (1.0 + x * (1.0 - s))
}

/// Sigmoid activation.
pub fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }

/// Sigmoid derivative.
pub fn sigmoid_grad(s: f32) -> f32 { s * (1.0 - s) }

// ── Dense Layer (forward + backward) ──────────────────────────────────────

/// Workload floor for GPU dispatch. Below it the wgpu round-trip costs more than the
/// arithmetic saves, so the train step stays on the CPU lane it was proven on.
///
/// RAISED 16x from the inherited `hybrid-infer` gate of 65536 (2026-08-03, measured on
/// an RTX 3070 at d=512, 20 reps back to back in one process): a [out,in]@[in,1] matvec
/// at 262k elements ran 0.28 ms on the device against 0.18 ms on the CPU — the old floor
/// dispatched work the GPU *loses*. The [ne·d,d]@[d,1] router block at 1.8M elements is
/// the smallest shape measured that wins (0.96 vs 1.26 ms, 1.31x), so the bar sits just
/// under it. Re-measure before moving this; it is a device fact, not a preference.
const GPU_MATMUL_FLOOR: usize = 1_048_576;

/// y = x @ W^T + b, returns (y, cache for backward)
///
/// FLY-STUDENT GPU lane: above [`GPU_MATMUL_FLOOR`] this dispatches through
/// `gpu_train::shared()`, which is warden-gated and falls back to CPU on
/// refusal or on a build without `wgpu-dispatch`. The dispatch is expressed as
/// `W @ x` — `[out×in] @ [in×1]` — NOT `x @ Wᵀ`, so the row-major weight block
/// goes to the device as it already lies in memory. A transpose per call would
/// have eaten the win, and the infer-side `wt_cache` cannot be reused here
/// because training mutates W every step.
pub fn linear_forward(x: &[f32], w: &[f32], b: Option<&[f32]>, out_dim: usize, in_dim: usize) -> Vec<f32> {
    if out_dim * in_dim > GPU_MATMUL_FLOOR {
        let ctx = crate::gpu_train::shared();
        if ctx.has_gpu() {
            let mut y = ctx.matmul(w, x, out_dim, 1, in_dim);
            if let Some(b) = b {
                for (yo, &bo) in y.iter_mut().zip(b.iter()) {
                    *yo += bo;
                }
            }
            return y;
        }
    }
    let mut y = vec![0.0f32; out_dim]; // @forge:allow_alloc — CPU lane, unchanged from the proven path
    for o in 0..out_dim {
        let row = &w[o * in_dim..(o + 1) * in_dim];
        y[o] = row.iter().zip(x.iter()).map(|(&wi, &xi)| wi * xi).sum();
        if let Some(b) = b { y[o] += b[o]; }
    }
    y
}

/// Backward through linear: given dy, compute dx, dw, db
pub fn linear_backward(
    dy: &[f32], x: &[f32], w: &[f32],
    out_dim: usize, in_dim: usize,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    // NO GPU ARM HERE, and it is not an omission (measured 2026-08-03, RTX 3070, d=512,
    // 20 reps in-process back to back): `dx` as [1,out]@[out,in] costs 1.41 ms on the
    // device against 0.38 ms on the CPU (0.27x), `dw` as [out,1]@[1,in] 0.46 vs 0.38
    // (0.83x). Both are memory-bound matvecs — 262k MACs is less arithmetic than one
    // dispatch's upload+submit+map+poll buys. An arm here makes the step SLOWER.
    // dx = dy @ W
    let mut dx = vec![0.0f32; in_dim]; // @forge:allow_alloc trainer CPU lane, unchanged from the proven path
    for o in 0..out_dim {
        let row = &w[o * in_dim..(o + 1) * in_dim];
        for i in 0..in_dim {
            dx[i] += dy[o] * row[i];
        }
    }
    // dw = dy^T @ x (outer product)
    let mut dw = vec![0.0f32; out_dim * in_dim]; // @forge:allow_alloc trainer CPU lane, unchanged from the proven path
    for o in 0..out_dim {
        for i in 0..in_dim {
            dw[o * in_dim + i] = dy[o] * x[i];
        }
    }
    // db = dy
    let db = dy.to_vec();
    (dx, dw, db)
}

// ── RMSNorm (forward + backward) ─────────────────────────────────────────

/// RMS Normalization forward pass.
pub fn rms_norm_forward(x: &[f32], weight: &[f32]) -> (Vec<f32>, f32) {
    let eps = 1e-6f32;
    let mean_sq = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
    let scale = (mean_sq + eps).sqrt().recip();
    let y: Vec<f32> = x.iter().zip(weight.iter()).map(|(&xi, &wi)| xi * scale * wi).collect();
    (y, scale)
}

/// RMS Normalization backward pass.
pub fn rms_norm_backward(dy: &[f32], x: &[f32], weight: &[f32], scale: f32) -> (Vec<f32>, Vec<f32>) {
    let _n = x.len() as f32;
    // dweight = dy * x * scale
    let dweight: Vec<f32> = dy.iter().zip(x.iter()).map(|(&dyi, &xi)| dyi * xi * scale).collect();
    // dx (simplified — ignoring second-order scale gradient for efficiency)
    let dx: Vec<f32> = dy.iter().zip(weight.iter()).map(|(&dyi, &wi)| dyi * wi * scale).collect();
    (dx, dweight)
}

// ── Adam Optimizer ────────────────────────────────────────────────────────

/// Adam optimizer state.
pub struct AdamState {
    /// First moment estimate.
    pub m: Vec<f32>,
    /// Second moment estimate.
    pub v: Vec<f32>,
    /// Timestep counter.
    pub t: u32,
    /// Steps whose gradient buffer was NON-FINITE and therefore dropped whole.
    ///
    /// A silent guard is a guard you cannot reason about (Sean 2026-08-02). The capacity
    /// run converges ONTO the uniform prior — `true 5.5452 vs rival 5.5452`, both exactly
    /// `ln(256)` — and two explanations fit that equally: the branch damp is too strong, or
    /// most steps are being zeroed and the net simply decays. This counter separates them
    /// without another 394s guess. Optimizer state is its home because a skipped step is a
    /// step Adam never took.
    pub non_finite_steps: u32,
}

impl AdamState {
    /// Create a new Adam optimizer state for the given parameter count.
    pub fn new(num_params: usize) -> Self {
        Self { m: vec![0.0; num_params], v: vec![0.0; num_params], t: 0, non_finite_steps: 0 } // @forge:allow_alloc optimizer state, once per run
    }

    /// Apply Adam update to params given gradients.
    pub fn step(&mut self, params: &mut [f32], grads: &[f32], lr: f32, beta1: f32, beta2: f32, eps: f32) {
        self.t += 1;
        let bc1 = 1.0 - beta1.powi(self.t as i32);
        let bc2 = 1.0 - beta2.powi(self.t as i32);
        for i in 0..params.len() {
            self.m[i] = beta1 * self.m[i] + (1.0 - beta1) * grads[i];
            self.v[i] = beta2 * self.v[i] + (1.0 - beta2) * grads[i] * grads[i];
            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;
            params[i] -= lr * m_hat / (v_hat.sqrt() + eps);
        }
    }
}

// ── Gradient Accumulator ──────────────────────────────────────────────────

/// Gradient accumulator for gradient accumulation across micro-batches.
pub struct GradAccumulator {
    /// Accumulated gradient buffer.
    pub grads: Vec<f32>,
    /// Number of accumulated steps.
    pub steps: u32,
}

impl GradAccumulator {
    /// Create a new gradient accumulator for the given parameter count.
    pub fn new(num_params: usize) -> Self {
        Self { grads: vec![0.0; num_params], steps: 0 }
    }

    /// Accumulate gradients from one micro-batch
    pub fn accumulate(&mut self, batch_grads: &[f32]) {
        for (g, &bg) in self.grads.iter_mut().zip(batch_grads.iter()) {
            *g += bg;
        }
        self.steps += 1;
    }

    /// The accumulator's own buffer, written in place. `backward_add_into` +
    /// [`Self::count_step`] land a token's gradient here directly: the old path filled a
    /// second `n`-float buffer with zeros, wrote a handful of spans into it, then read all
    /// `n` back to add them in — 1.2 GB of traffic per token at d=512 to move a few MB of
    /// nonzeros (measured 2026-08-03: fill+accumulate was ~40% of a step).
    pub fn buffer_mut(&mut self) -> &mut [f32] {
        &mut self.grads
    }

    /// Count one micro-batch whose gradient was written straight into [`Self::buffer_mut`].
    pub fn count_step(&mut self) {
        self.steps += 1;
    }

    /// Average accumulated gradients and reset
    pub fn flush(&mut self) -> &[f32] {
        self.flush_mut()
    }

    /// [`Self::flush`] with the buffer kept mutable, so the clip can scale IN PLACE
    /// instead of copying `n` floats into a second buffer first. The caller zeroes this
    /// buffer at the end of the window either way, so in-place is the same arithmetic.
    pub fn flush_mut(&mut self) -> &mut [f32] {
        if self.steps > 1 {
            let scale = 1.0 / self.steps as f32;
            for g in self.grads.iter_mut() { *g *= scale; }
        }
        &mut self.grads
    }

    /// Zero out accumulated gradients and reset step counter.
    pub fn zero(&mut self) {
        self.grads.fill(0.0);
        self.steps = 0;
    }
}

// ── MoE Training Model (f32, with gradient cache) ─────────────────────────

/// Flat parameter vector for the full MoE model.
/// All weights stored contiguously for Adam to operate on.
pub struct MoeParams {
    /// Contiguous parameter buffer.
    pub data: Vec<f32>,
    /// Offset into data for embedding weights.
    pub embed_offset: usize,
    /// Offset into data for trunk weights (2 layers of transformations).
    pub trunk_offset: usize,
    /// Offset into data for router weights ([num_experts, d_model, d_model] + bias).
    pub router_offset: usize,
    /// Offset into data for expert weights.
    pub experts_offset: usize,
    /// Offset into data for DCGS weights (quality_head + coherence_head + schema_attn + schema_ffn).
    pub dcgs_offset: usize,
    /// Offset into data for forget gate weights.
    pub fgate_offset: usize,
    /// Offset into data for language model head.
    pub lm_head_offset: usize,
    /// Total parameter count.
    pub total_params: usize,
    /// Vocabulary size.
    pub vocab_size: usize,
    /// Model dimension.
    pub d_model: usize,
    /// Number of experts.
    pub num_experts: usize,
    /// Number of layers per expert.
    pub expert_layers: usize,
}

impl MoeParams {
    /// Calculate total parameter count and offsets for the architecture.
    pub fn layout(vocab_size: usize, d_model: usize, num_experts: usize, expert_layers: usize) -> Self {
        let d2 = d_model * d_model;
        let mlp_hidden = d_model * 4;

        let embed_size = vocab_size * d_model;
        // Trunk: 2 layers × (ln_w + q + k + v + out + ln2_w + ff1 + ff2 + ff3)
        let trunk_layer = d_model + d2 * 4 + d_model + d_model * mlp_hidden * 3;
        let trunk_size = trunk_layer * 2;
        // Router: quadratic gate [experts, d, d] + bias [experts]
        let router_size = num_experts * d2 + num_experts;
        // Experts: each has expert_layers × same as trunk_layer, plus final ln
        let expert_size = (trunk_layer * expert_layers + d_model) * num_experts;
        // DCGS: quality_head (d*2→d→1) + coherence_head (d→d/2→1) + schema_attn + schema_ffn
        let dcgs_size = (d_model * 2 * d_model + d_model) + (d_model * d_model / 2 + d_model / 2)
            + d2 * 4 + d_model * d_model * 2 * 2 + d_model;
        // ForgetGate: gate weights
        let fgate_size = d_model * d_model * 2 + d_model;
        // LM head — UNTIED (Sean 2026-08-02, measured). Tied, `W_embed` had to be both the
        // input coordinate system and the output classifier, and cross-entropy spent the
        // input geometry to buy output logits: the four classes arrived 19.4° apart and the
        // head crushed them to 11.2°. No router can partition a space that has been
        // flattened. This is a REAL allocation, not a re-wire — `lm_head_offset` pointed at
        // the end of the buffer with size 0.
        let lm_head_size = vocab_size * d_model;

        let mut offset = 0;
        let embed_offset = offset; offset += embed_size;
        let trunk_offset = offset; offset += trunk_size;
        let router_offset = offset; offset += router_size;
        let experts_offset = offset; offset += expert_size;
        let dcgs_offset = offset; offset += dcgs_size;
        let fgate_offset = offset; offset += fgate_size;
        let lm_head_offset = offset; offset += lm_head_size;

        Self {
            data: vec![0.0; offset],
            embed_offset, trunk_offset, router_offset, experts_offset,
            dcgs_offset, fgate_offset, lm_head_offset,
            total_params: offset,
            vocab_size, d_model, num_experts, expert_layers,
        }
    }

    /// Xavier-scale init (deterministic PRNG): residual bodies (trunk + experts)
    /// draw ±√(3/d_model); embed/router/DCGS/gate keep the flat ±0.02. Depth damping
    /// lives in `forward`'s 1/√(2·L) branch multiplier — one modulus, one home;
    /// damping both here and there would vanish the branch as depth grows.
    pub fn init_xavier(&mut self, seed: u64) {
        let mut rng = seed;
        let residual = (3.0 / self.d_model as f32).sqrt();
        let (t0, t1) = (self.trunk_offset, self.router_offset);
        let (e0, e1) = (self.experts_offset, self.dcgs_offset);
        for (i, p) in self.data.iter_mut().enumerate() {
            // Xorshift64
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let u = (rng >> 11) as f32 / (1u64 << 53) as f32;
            let bound = if (t0..t1).contains(&i) || (e0..e1).contains(&i) { residual } else { 0.02 };
            *p = (u * 2.0 - 1.0) * bound;
        }
    }

    /// Get embedding weights slice.
    pub fn embed(&self) -> &[f32] {
        &self.data[self.embed_offset..self.embed_offset + self.vocab_size * self.d_model]
    }

    /// Get total parameter count.
    pub fn param_count(&self) -> usize { self.total_params }
}

// ── Forward Pass with Gradient Cache ──────────────────────────────────────

/// Cache from a forward pass — stores all intermediates needed for backprop.
pub struct ForwardCache {
    /// Input token ID.
    pub token_id: usize,
    /// Embedded token ([d_model]).
    pub embedded: Vec<f32>,
    /// Trunk output ([d_model]).
    pub trunk_out: Vec<f32>,
    /// Router probabilities ([num_experts]).
    pub router_probs: Vec<f32>,
    /// The expert stack's output BEFORE the gate multiply, and the gate probability
    /// applied to it. `argmax` is not differentiable, so with a hard one-hot dispatch the
    /// router weights received no gradient at all — `router_offset` appeared in `forward`
    /// and in `init_xavier` and NOWHERE in `backward_into`. The routing was frozen at its
    /// init values for every run ever measured: 2 of 9 experts from epoch 0, unchanged.
    /// Scaling by `p` puts the router back on the graph without softening the dispatch.
    pub expert_raw: Vec<f32>,
    /// Gate probability for selected expert.
    pub gate_p: f32,
    /// The vector the ROUTER actually saw: `trunk_out` centred and unit-normalised. The
    /// router's gradient outer product must use this, not `trunk_out` — they are different
    /// vectors, and using the wrong one trains `A_e` against an input it never received.
    pub router_in: Vec<f32>,
    /// The router logits went NaN this step. A verdict, not a crash — see `forward`.
    pub router_diverged: bool,
    /// Selected expert index.
    pub expert_id: usize,
    /// Each expert layer's INPUT — `layer_inputs[l]` is what layer `l` saw. Backprop
    /// through a residual stack needs the per-layer input, not one final activation:
    /// reusing `expert_out` for every layer is what made the depth gradient fiction.
    pub layer_inputs: Vec<Vec<f32>>,
    /// Each layer's PRE-ACTIVATION `W·input`. GELU's derivative is taken at the value
    /// that entered it, not at the layer's input.
    pub layer_pre: Vec<Vec<f32>>,
    /// Each layer's residual sum BEFORE normalisation, and the norm scale that followed.
    /// A 20-deep residual stream with no norm diverges — the first residual run moved the
    /// loss off the ln(256) pin and straight to 9.15, which is the unbounded stream.
    pub layer_sum: Vec<Vec<f32>>,
    /// RMS norm scale per layer.
    pub layer_scale: Vec<f32>,
    /// Expert output ([d_model]).
    pub expert_out: Vec<f32>,
    /// Logits ([vocab_size]).
    pub logits: Vec<f32>,
    /// Cross-entropy loss.
    pub loss: f32,
    /// Target token ID.
    pub target: usize,
}

/// Single-token forward pass through the full MoE, returning cache for backprop.
pub fn forward(params: &MoeParams, token_id: usize, target: usize) -> ForwardCache {
    let d = params.d_model;
    let v = params.vocab_size;

    // Embed
    let embed_row = &params.embed()[token_id * d..(token_id + 1) * d];
    let embedded = embed_row.to_vec();

    // Trunk (simplified: 2-layer MLP for now, full attention later)
    let trunk_w = &params.data[params.trunk_offset..params.trunk_offset + d * d];
    let trunk_out = linear_forward(&embedded, trunk_w, None, d, d);
    // TRUNK SKIP + CENTRE (Sean 2026-08-02, localised by measurement at two points).
    //
    // The embedding delivers the classes 19.4° apart (max cosine 0.942803) and the bare
    // `gelu(W·embed)` handed the router 8.7° (0.988522) — the trunk was destroying more
    // than half the separation it was given. One unconstrained d×d matrix trained on
    // next-token loss projects toward whatever direction lowers that loss; nothing in the
    // objective pays it to keep classes apart, and `gelu` folds the negative half onto the
    // same line. The identity skip means `W` can only ADD to the embedding's geometry, so
    // 19.4° is a floor rather than a starting point it is free to spend.
    // Centring then removes the shared DC component `gelu` leaves behind.
    let trunk_act: Vec<f32> = trunk_out.iter().map(|&x| gelu(x)).collect(); // @forge:allow_alloc trainer
    let skipped: Vec<f32> = embedded.iter().zip(trunk_act.iter()).map(|(&e, &a)| e + a).collect(); // @forge:allow_alloc trainer
    let tmean = skipped.iter().sum::<f32>() / d as f32;
    let trunk_out: Vec<f32> = skipped.iter().map(|&x| x - tmean).collect(); // @forge:allow_alloc trainer

    // ROUTER INPUT = trunk_out CENTRED and unit-normalised (Sean 2026-08-02, measured).
    //
    // `gelu` is one-sided: every trunk_out lands in the positive orthant, so all four class
    // vectors share a large DC component and point almost the same way — max cosine 0.999331
    // between classes, ~2° apart, while `xᵀAx` scales with |x|². The router was handed four
    // near-parallel vectors of differing length and did the only thing it could: pick the
    // same expert every time, by magnitude. Subtracting the mean removes the shared
    // component that carries no class information; unit-normalising removes the magnitude
    // that was standing in for it. What remains is direction — the only part that differs.
    let ne = params.num_experts;
    let mean = trunk_out.iter().sum::<f32>() / d as f32;
    let centred: Vec<f32> = trunk_out.iter().map(|&x| x - mean).collect(); // @forge:allow_alloc trainer
    let rnorm = centred.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    let router_in: Vec<f32> = centred.iter().map(|&x| x / rnorm).collect(); // @forge:allow_alloc trainer

    // Router: quadratic x^T A x for each expert. The experts' A blocks are CONTIGUOUS
    // (router_offset + e·d·d), so all `ne` of them are one [ne·d, d] matrix and the whole
    // router is a single dispatch instead of `ne` scalar d×d loops — at d=512 that was 7
    // × 262k MACs per token on one core, the largest CPU block left in the forward pass
    // (Sean 2026-08-03 "can you not build more GPU arms?").
    let mut router_logits = vec![0.0f32; ne]; // @forge:allow_alloc trainer, one per token
    let router_all = &params.data[params.router_offset..params.router_offset + ne * d * d];
    let ax_all: Option<Vec<f32>> = if ne * d * d > GPU_MATMUL_FLOOR {
        let ctx = crate::gpu_train::shared();
        ctx.has_gpu().then(|| ctx.matmul(router_all, &router_in, ne * d, 1, d))
    } else {
        None
    };
    for e in 0..ne {
        match &ax_all {
            Some(ax) => {
                let row = &ax[e * d..(e + 1) * d];
                router_logits[e] = row.iter().zip(router_in.iter()).map(|(&a, &x)| a * x).sum();
            }
            None => {
                let a = &router_all[e * d * d..(e + 1) * d * d];
                let mut ax = vec![0.0f32; d]; // @forge:allow_alloc trainer, one per expert
                for i in 0..d {
                    ax[i] = a[i * d..(i + 1) * d].iter().zip(router_in.iter())
                        .map(|(&w, &x)| w * x).sum();
                }
                router_logits[e] = ax.iter().zip(router_in.iter()).map(|(&a, &x)| a * x).sum();
            }
        }
    }
    let router_probs = softmax(&router_logits);
    // `partial_cmp().unwrap()` made divergence a PANIC: once a NaN reaches the router the
    // argmax aborts the process, so the run that would name the cause destroys itself
    // instead. `total_cmp` is total over f32 (NaN sorts, never compares None), and the
    // divergence rides out on the cache as a fact the caller can read.
    let router_diverged = router_probs.iter().any(|p| p.is_nan());
    let expert_id = router_probs.iter().enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i).unwrap_or(0);

    // Expert forward — a RESIDUAL stack, not a chain. Without the skip the loss sat at
    // exactly ln(vocab) whatever the depth: each layer attenuated the signal and the
    // gradient reaching the head was the uniform prior. `h + f(h)` keeps a path of
    // derivative 1 from the head to every layer, which is the whole reason depth trains.
    // Same width in and out, so the add needs no projection.
    // Normalised after every add, because an unnormalised 20-deep stream diverges: the
    // first residual build moved the loss off the ln(256) pin and then blew up to 9.15.
    // Unit gain — no new parameters, the norm is a stabiliser here, not a learned layer.
    let ones = vec![1.0f32; d]; // @forge:allow_alloc trainer, unit RMS gain
    // Branch damp 1/√(2·L): bounds the residual stream's variance growth so the RMS
    // backward's 1/RMS(x) factor never meets an exploding input. The damp lives HERE,
    // not in init — one modulus, one home; damping both would vanish the branch.
    let damp = 1.0 / (2.0 * params.expert_layers.max(1) as f32).sqrt();
    let mut h = trunk_out.clone(); // @forge:allow_alloc trainer, not the inference lane
    let mut layer_inputs: Vec<Vec<f32>> = Vec::with_capacity(params.expert_layers); // @forge:allow_alloc backprop cache
    let mut layer_pre: Vec<Vec<f32>> = Vec::with_capacity(params.expert_layers); // @forge:allow_alloc backprop cache
    let mut layer_sum: Vec<Vec<f32>> = Vec::with_capacity(params.expert_layers); // @forge:allow_alloc backprop cache
    let mut layer_scale: Vec<f32> = Vec::with_capacity(params.expert_layers); // @forge:allow_alloc backprop cache
    for layer in 0..params.expert_layers {
        let w_offset = params.experts_offset + (expert_id * params.expert_layers + layer) * d * d;
        let w = &params.data[w_offset..w_offset + d * d];
        let input = h.clone(); // @forge:allow_alloc one cache row per layer
        let projected = linear_forward(&input, w, None, d, d);
        let mut summed = input.clone(); // @forge:allow_alloc one cache row per layer
        for (slot, &p) in summed.iter_mut().zip(projected.iter()) {
            *slot += gelu(p) * damp;
        }
        let (normed, scale) = rms_norm_forward(&summed, &ones);
        layer_inputs.push(input); // @forge:allow_alloc backprop cache
        layer_pre.push(projected); // @forge:allow_alloc backprop cache
        layer_sum.push(summed); // @forge:allow_alloc backprop cache
        layer_scale.push(scale); // @forge:allow_alloc backprop cache
        h = normed;
    }
    // GATE MULTIPLY — the router's only path onto the gradient graph. `argmax` picks WHICH
    // expert; `p` is HOW MUCH, and it is differentiable. Without it the router is decoration.
    let expert_raw = h;
    let gate_p = router_probs[expert_id];
    let expert_out: Vec<f32> = expert_raw.iter().map(|&x| x * gate_p).collect(); // @forge:allow_alloc trainer

    // LM head (tied with embed)
    let lm_head = &params.data[params.lm_head_offset..params.lm_head_offset + v * d];
    let logits = linear_forward(&expert_out, lm_head, None, v, d);

    let (loss, _dlogits) = cross_entropy(&logits, target);

    ForwardCache {
        token_id, embedded, trunk_out, router_probs, router_diverged, expert_id,
        layer_inputs, layer_pre, layer_sum, layer_scale,
        expert_raw, gate_p, router_in, expert_out, logits, loss, target,
    }
}

/// Backward pass: compute gradients for all parameters.
/// Returns gradient vector same size as params.data.
///
/// Convenience wrapper — allocates one full-size gradient vector per call. The
/// training loop must NOT use this: at 3M params that is a 12 MB allocate-and-
/// zero for every token, which dominated the step and left the GPU idle
/// (measured 2026-07-27: one core pegged for the whole run). Use
/// [`backward_into`] with a buffer hoisted out of the loop.
pub fn backward(params: &MoeParams, cache: &ForwardCache) -> Vec<f32> {
    let mut grads = vec![0.0f32; params.total_params]; // @forge:allow_alloc — one-shot API, not the train loop
    backward_into(params, cache, &mut grads);
    grads
}

/// Backward pass into a caller-owned buffer, reused across steps.
///
/// Zeroes `grads` first, so one buffer serves the whole epoch. Numerics are
/// identical to [`backward`] — this changes where the memory comes from,
/// nothing else.
/// Switch-Transformer load balancing, applied straight to the router logits.
///
/// A trainable router is a positive feedback loop: the expert with the highest initial
/// weight wins the token, gets the gradient, and wins harder. Measured, not argued — with
/// `router_offset` frozen the run used 2 of 9 experts; the moment the router could learn it
/// went to 1 of 9 and the loss got WORSE (0.9658 → 1.7092).
///
/// `∂L_balance/∂logit_i = α·(f_i − 1/N)`, where `f_i` is the running dispatch fraction: an
/// expert over its share is pushed down, one under its share is pushed up. The push reaches
/// the weights by the same route the task gradient does — `logit_e = xᵀA_e x`, so
/// `dA_e = dlogit · x xᵀ`.
///
/// `α = 0` disables it entirely, which is how the frozen-router behaviour stays reachable.
pub fn balance_push(
    params: &MoeParams,
    cache: &ForwardCache,
    dispatch: &[u32],
    seen: u32,
    alpha: f32,
    grads: &mut [f32],
) {
    if alpha == 0.0 || seen == 0 {
        return;
    }
    let d = params.d_model;
    let ne = params.num_experts;
    let share = 1.0 / ne as f32;
    for e in 0..ne {
        let f_e = dispatch[e] as f32 / seen as f32;
        let dlogit = alpha * (f_e - share);
        if dlogit == 0.0 {
            continue;
        }
        let a_offset = params.router_offset + e * d * d;
        for i in 0..d {
            let xi = cache.router_in[i];
            for j in 0..d {
                grads[a_offset + i * d + j] += dlogit * xi * cache.router_in[j];
            }
        }
    }
}

/// Backward pass into a caller-owned buffer, zeroing it first.
pub fn backward_into(params: &MoeParams, cache: &ForwardCache, grads: &mut [f32]) {
    assert_eq!(
        grads.len(),
        params.total_params,
        "gradient buffer must match the parameter count"
    );
    grads.fill(0.0);
    backward_add_into(params, cache, grads);
}

/// Backward pass WITHOUT the zero-fill — every write below is already `+=`, so this
/// adds one token's gradient onto whatever the buffer holds. The train loop points it
/// straight at the accumulator and zeroes once per window instead of once per token.
pub fn backward_add_into(params: &MoeParams, cache: &ForwardCache, grads: &mut [f32]) {
    assert_eq!(
        grads.len(),
        params.total_params,
        "gradient buffer must match the parameter count"
    );
    let d = params.d_model;
    let v = params.vocab_size;

    // dL/dlogits from cross-entropy
    let (_, dlogits) = cross_entropy(&cache.logits, cache.target);

    // LM head backward — UNTIED. The head's gradient lands on the HEAD, not on the
    // embedding table. That separation is the whole fix: the embedding is now updated only
    // by what flows back through the network, so it is free to hold class geometry instead
    // of spending it on output logits.
    let lm_head = &params.data[params.lm_head_offset..params.lm_head_offset + v * d];
    let (dexpert_out, dhead, _) = linear_backward(&dlogits, &cache.expert_out, lm_head, v, d);
    for (i, &g) in dhead.iter().enumerate() {
        grads[params.lm_head_offset + i] += g;
    }

    // ── ROUTER BACKWARD ──────────────────────────────────────────────────
    // The gate multiply splits the gradient two ways: through the expert stack (scaled by
    // p) and through p itself. The second branch is the one that never existed — the reason
    // the router sat frozen at init while the loss fell around it.
    let dgate_p: f32 = dexpert_out.iter().zip(cache.expert_raw.iter())
        .map(|(&dy, &x)| dy * x).sum();
    let dexpert_out: Vec<f32> = dexpert_out.iter().map(|&g| g * cache.gate_p).collect(); // @forge:allow_alloc trainer

    // Softmax backward for the ONE selected probability: dL/dlogit_e = p_e (δ_ie − p_i).
    let ne = params.num_experts;
    let sel = cache.expert_id;
    let p_sel = cache.gate_p;
    for e in 0..ne {
        let dlogit = dgate_p * p_sel * (if e == sel { 1.0 } else { 0.0 } - cache.router_probs[e]);
        if dlogit == 0.0 {
            continue;
        }
        // logit_e = xᵀ A_e x, so dA_e = dlogit · x xᵀ — the outer product of the router's
        // own input with itself, which is `trunk_out` here.
        let a_offset = params.router_offset + e * d * d;
        for i in 0..d {
            let xi = cache.router_in[i];
            for j in 0..d {
                grads[a_offset + i * d + j] += dlogit * xi * cache.router_in[j];
            }
        }
    }

    // Expert backward
    let damp = 1.0 / (2.0 * params.expert_layers.max(1) as f32).sqrt();
    let mut dh = dexpert_out;
    for layer in (0..params.expert_layers).rev() {
        let w_offset = params.experts_offset
            + (cache.expert_id * params.expert_layers + layer) * d * d;
        let w = &params.data[w_offset..w_offset + d * d];

        // Every layer used to be handed `expert_out` — the FINAL activation — so the
        // gradient it computed was for an input it never saw. These are its own.
        let input = &cache.layer_inputs[layer];

        // Back through the RMS norm that closed this layer.
        let ones = vec![1.0f32; d]; // @forge:allow_alloc trainer, unit RMS gain
        let (dsummed, _dgain) = rms_norm_backward(
            &dh, &cache.layer_sum[layer], &ones, cache.layer_scale[layer],
        );

        // GELU backward at the value that actually entered it, through the same
        // 1/√(2·L) branch damp the forward applied.
        let dproj: Vec<f32> = dsummed.iter().zip(cache.layer_pre[layer].iter())
            .map(|(&dy, &p)| dy * gelu_grad(p) * damp).collect(); // @forge:allow_alloc trainer, one vec per layer

        let (dx, dw, _db) = linear_backward(&dproj, input, w, d, d);

        // Store expert weight grads
        for (i, &g) in dw.iter().enumerate() {
            grads[w_offset + i] += g;
        }
        // RESIDUAL BACKWARD: an add passes gradient straight through, so the SKIP term
        // and the BRANCH term sum. Dropping the skip rebuilds the vanishing path the
        // forward residual exists to remove.
        dh = dsummed;
        for (slot, &g) in dh.iter_mut().zip(dx.iter()) {
            *slot += g;
        }
    }

    // Trunk backward
    let dtrunk_pre: Vec<f32> = dh.iter().zip(cache.trunk_out.iter())
        .map(|(&dy, &x)| dy * gelu_grad(x)).collect();
    let trunk_w = &params.data[params.trunk_offset..params.trunk_offset + d * d];
    let (_dx, dw, _db) = linear_backward(&dtrunk_pre, &cache.embedded, trunk_w, d, d);
    for (i, &g) in dw.iter().enumerate() {
        grads[params.trunk_offset + i] += g;
    }

    // Embed backward (from trunk input)
    let dembed_row = &dtrunk_pre; // simplified
    let row_offset = params.embed_offset + cache.token_id * d;
    for (i, &g) in dembed_row.iter().enumerate() {
        if row_offset + i < grads.len() {
            grads[row_offset + i] += g;
        }
    }
}

// ── Training Loop ─────────────────────────────────────────────────────────

/// Training configuration.
pub struct TrainConfig {
    /// Learning rate.
    pub lr: f32,
    /// Adam beta1 parameter.
    pub beta1: f32,
    /// Adam beta2 parameter.
    pub beta2: f32,
    /// Adam epsilon parameter.
    pub eps: f32,
    /// Number of gradient accumulation steps.
    pub grad_accum_steps: u32,
    /// Maximum gradient norm for clipping.
    pub max_grad_norm: f32,
    /// Load-balancing strength. `0.0` = off (the collapsing router), `0.02` = the Switch
    /// default. See [`balance_push`] for why a trainable router needs it at all.
    pub balance_alpha: f32,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            lr: 3e-4, beta1: 0.9, beta2: 0.999, eps: 1e-8,
            grad_accum_steps: 4, max_grad_norm: 1.0, balance_alpha: 0.02,
        }
    }
}

/// Clip gradient norm in-place. Returns the original norm.
///
/// A NON-FINITE norm ZEROES the buffer instead of scaling it (Sean 2026-08-02). `NaN >
/// max_norm` is `false`, so every comparison-based clip is blind to exactly the value it
/// most needs to stop: one NaN gradient sailed through untouched, Adam wrote it into the
/// weights, and from then on every router logit was NaN — the argmax `partial_cmp` panic
/// at the top of `forward` is that poison surfacing several steps downstream of its cause.
/// A non-finite gradient carries no information, so dropping the step loses nothing and
/// keeps the parameters usable.
pub fn clip_grad_norm(grads: &mut [f32], max_norm: f32) -> f32 {
    let norm: f32 = grads.iter().map(|g| g * g).sum::<f32>().sqrt();
    if !norm.is_finite() {
        grads.fill(0.0);
        return norm;
    }
    if norm > max_norm {
        let scale = max_norm / norm;
        for g in grads.iter_mut() { *g *= scale; }
    }
    norm
}

/// Train one epoch over token sequences. Returns average loss.
pub fn train_epoch(
    params: &mut MoeParams,
    adam: &mut AdamState,
    accum: &mut GradAccumulator,
    tokens: &[u32],  // flat token sequence
    cfg: &TrainConfig,
) -> f32 {
    let mut total_loss = 0.0f32;
    let mut count = 0u32;

    // FUSED into the accumulator (2026-08-03, measured): the per-token gradient buffer and
    // the per-flush clip buffer are both gone. Backward adds straight into `accum`, and the
    // clip scales that same buffer in place — the two staging buffers cost 3 full passes
    // over `total_params` per token (fill + add + copy) to carry a few MB of nonzeros, and
    // at d=512 that was ~40% of a step while the GPU matmuls were ~20%.
    // Running dispatch tally — [`balance_push`] needs f_i, and an epoch-long running
    // fraction is the cheapest honest estimate of it without a second pass.
    let mut dispatch = vec![0u32; params.num_experts]; // @forge:allow_alloc — one per epoch
    let mut seen = 0u32;

    for i in 0..tokens.len().saturating_sub(1) {
        let input = tokens[i] as usize;
        let target = tokens[i + 1] as usize;

        let cache = forward(params, input, target);
        // A non-finite step must never reach `adam.step`: Adam's m/v are RUNNING buffers,
        // so one NaN gradient poisons every later step and the divergence outlives the
        // token that caused it. Refuse the epoch at the first bad step and say which.
        if !cache.loss.is_finite() || cache.router_diverged {
            eprintln!(
                "moe train: REFUSED at token {i} — loss={} router_diverged={} (lr={}); \
                 params are the last finite state, Adam moments untouched",
                cache.loss, cache.router_diverged, cfg.lr
            );
            // NaN, never the partial average: an epoch that refused is not an epoch that
            // scored, and `loss_final < loss1` must not read a truncated run as progress.
            // The f32 return carries the verdict — a typed outcome here would ripple
            // through 19 call sites in 5 files to say what `is_finite()` already says.
            return f32::NAN;
        }
        total_loss += cache.loss;
        count += 1;

        dispatch[cache.expert_id] += 1;
        seen += 1;
        {
            let g = accum.buffer_mut();
            backward_add_into(params, &cache, g);
            balance_push(params, &cache, &dispatch, seen, cfg.balance_alpha, g);
        }
        accum.count_step();

        if accum.steps >= cfg.grad_accum_steps {
            let clipped = accum.flush_mut();
            if !clip_grad_norm(clipped, cfg.max_grad_norm).is_finite() {
                adam.non_finite_steps += 1;
            }
            adam.step(&mut params.data, clipped, cfg.lr, cfg.beta1, cfg.beta2, cfg.eps);
            accum.zero();
        }
    }

    // Flush remaining accumulated grads
    if accum.steps > 0 {
        let clipped = accum.flush_mut();
        if !clip_grad_norm(clipped, cfg.max_grad_norm).is_finite() {
            adam.non_finite_steps += 1;
        }
        adam.step(&mut params.data, clipped, cfg.lr, cfg.beta1, cfg.beta2, cfg.eps);
        accum.zero();
    }

    if count > 0 { total_loss / count as f32 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_entropy_gradient_sums_to_zero() {
        let logits = vec![1.0, 2.0, 0.5, -1.0];
        let (_, dlogits) = cross_entropy(&logits, 1);
        let sum: f32 = dlogits.iter().sum();
        assert!(sum.abs() < 1e-5, "CE gradient should sum to ~0, got {}", sum);
    }

    /// FLY-STUDENT GPU lane: above the dispatch floor `linear_forward` must
    /// agree with the CPU arithmetic it replaced, elementwise. On a machine
    /// with no device (CI, headless) the warden falls back and this still
    /// gates the CPU path — but the receipt names which lane actually ran, so
    /// a silent fallback can never read as a proven GPU dispatch.
    #[test]
    fn gpu_dispatch_agrees_with_cpu_above_the_floor() {
        let _serial = crate::gpu_train::test_serial();
        // Above the RAISED floor (2026-08-03): 2M elements, the size class where the
        // device actually beats the CPU loop. 256x512 no longer dispatches at all.
        let (out_dim, in_dim) = (2048usize, 1024usize);
        assert!(out_dim * in_dim > GPU_MATMUL_FLOOR, "dims must trip the dispatch floor");

        // Deterministic weights/activations — no rng dependency.
        let mut w = vec![0.0f32; out_dim * in_dim]; // @forge:allow_alloc — test
        for (i, slot) in w.iter_mut().enumerate() {
            *slot = ((i % 17) as f32 - 8.0) / 32.0;
        }
        let mut x = vec![0.0f32; in_dim]; // @forge:allow_alloc — test
        for (i, slot) in x.iter_mut().enumerate() {
            *slot = ((i % 11) as f32 - 5.0) / 16.0;
        }

        let got = linear_forward(&x, &w, None, out_dim, in_dim);
        let want = crate::gpu_train::cpu_matmul(&w, &x, out_dim, 1, in_dim);
        assert_eq!(got.len(), out_dim);
        for (o, (g, c)) in got.iter().zip(want.iter()).enumerate() {
            assert!(
                (g - c).abs() <= 1e-3 * c.abs().max(1.0),
                "row {o} diverged: dispatch {g} vs cpu {c} (lane gpu={})",
                crate::gpu_train::shared().has_gpu()
            );
        }
        eprintln!(
            "[moe_train] GPU dispatch gate: lane gpu={} dims {out_dim}x{in_dim}",
            crate::gpu_train::shared().has_gpu()
        );
    }

    #[test]
    fn softmax_sums_to_one() {
        let p = softmax(&[1.0, 2.0, 3.0]);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn adam_moves_params() {
        let mut params = vec![1.0f32; 4];
        let grads = vec![0.1, -0.1, 0.2, -0.2];
        let mut adam = AdamState::new(4);
        adam.step(&mut params, &grads, 0.01, 0.9, 0.999, 1e-8);
        assert_ne!(params, vec![1.0f32; 4]);
    }

    #[test]
    fn grad_accum_averages() {
        let mut acc = GradAccumulator::new(3);
        acc.accumulate(&[1.0, 2.0, 3.0]);
        acc.accumulate(&[3.0, 2.0, 1.0]);
        let avg = acc.flush();
        assert!((avg[0] - 2.0).abs() < 1e-5);
        assert!((avg[1] - 2.0).abs() < 1e-5);
        assert!((avg[2] - 2.0).abs() < 1e-5);
    }

    /// The blind spot every comparison-based clip has: `NaN > max` is FALSE, so the one
    /// value that must never reach Adam is the one value the clip lets past untouched.
    #[test]
    fn a_non_finite_gradient_is_zeroed_not_waved_through() {
        for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut grads = vec![1.0, poison, 2.0]; // @forge:allow_alloc — test
            let norm = clip_grad_norm(&mut grads, 1.0);
            assert!(!norm.is_finite(), "the norm of a poisoned buffer is not finite");
            assert!(grads.iter().all(|g| *g == 0.0), "a non-finite step is dropped whole");
        }
        // And the ordinary path is untouched by the guard.
        let mut ok = vec![3.0, 4.0]; // @forge:allow_alloc — test
        clip_grad_norm(&mut ok, 10.0);
        assert!(ok[0] == 3.0 && ok[1] == 4.0, "a norm under the cap is left alone");
    }

    #[test]
    fn clip_grad_norm_clips() {
        let mut grads = vec![3.0, 4.0]; // norm = 5
        let norm = clip_grad_norm(&mut grads, 1.0);
        assert!((norm - 5.0).abs() < 1e-5);
        let new_norm: f32 = grads.iter().map(|g| g * g).sum::<f32>().sqrt();
        assert!((new_norm - 1.0).abs() < 1e-4);
    }

    #[test]
    fn forward_backward_runs() {
        let mut params = MoeParams::layout(256, 32, 3, 2);
        params.init_xavier(42);
        let cache = forward(&params, 65, 66); // 'A' -> 'B'
        assert_eq!(cache.logits.len(), 256);
        assert!(cache.loss > 0.0);
        let grads = backward(&params, &cache);
        assert_eq!(grads.len(), params.total_params);
        // At least some gradients should be nonzero
        let nonzero = grads.iter().filter(|&&g| g.abs() > 1e-12).count();
        assert!(nonzero > 0, "expected nonzero gradients");
    }

    /// MOE-DEPTH-RESIDUAL: the depth cliff. Before the residual stack (`forward`:424),
    /// the RMS-after-every-add (:426-429) and the 1/√(2·L) branch damp (:433), a deep
    /// net pinned at ln(256) — every class reading the uniform prior. Depth 12 must now
    /// train, and `non_finite_steps` must be 0, because a loss that falls only because
    /// most steps were dropped is decay wearing convergence's face (Sean 08-02, :159-164).
    // [BOARD: MOE-DEPTH-RESIDUAL]
    #[test]
    fn a_deep_stack_trains_instead_of_pinning_at_the_uniform_prior() {
        let mut params = MoeParams::layout(256, 16, 2, 12);
        params.init_xavier(0x13F0);
        let mut adam = AdamState::new(params.total_params);
        let mut accum = GradAccumulator::new(params.total_params);
        let cfg = TrainConfig { grad_accum_steps: 1, lr: 1e-3, ..Default::default() };
        let mut tokens: Vec<u32> = Vec::new(); // @forge:allow_alloc — test
        for _ in 0..16 {
            tokens.extend_from_slice(&[97, 119, 124, 98, 120, 124]); // @forge:allow_alloc — test
        }
        let first = train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        for _ in 0..48 {
            train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        }
        let last = train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        let pin = (256f32).ln();
        assert!(last < first, "depth-12 stack did not train: {first:.4} -> {last:.4}");
        assert!(last < pin, "loss pinned at the uniform prior ln(256)={pin:.4}: {last:.4}");
        assert_eq!(
            adam.non_finite_steps, 0,
            "the fall is decay, not training — {} steps were dropped whole",
            adam.non_finite_steps,
        );
    }

    #[test]
    fn train_epoch_reduces_loss() {
        let mut params = MoeParams::layout(256, 32, 3, 2);
        params.init_xavier(42);
        let mut adam = AdamState::new(params.total_params);
        let mut accum = GradAccumulator::new(params.total_params);
        let cfg = TrainConfig { grad_accum_steps: 1, lr: 1e-3, ..Default::default() };
        let tokens: Vec<u32> = b"hello world, this is a longer sequence for training to converge on".iter().map(|&b| b as u32).collect();
        let loss1 = train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        for _ in 0..20 {
            train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        }
        let loss_final = train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        assert!(loss_final < loss1, "loss should decrease: {:.4} -> {:.4}", loss1, loss_final);
    }

    #[test]
    fn param_layout_sizes() {
        let p = MoeParams::layout(256, 512, 7, 5);
        println!("Total params: {} ({:.1}M)", p.total_params, p.total_params as f64 / 1e6);
        assert!(p.total_params > 1_000_000, "should be >1M params");
    }

    /// A poisoned parameter set must REFUSE, not train: the guard fires before
    /// `backward_into`, so Adam's running m/v never see the NaN and the epoch scores NaN
    /// rather than a partial average that reads as progress.
    #[test]
    fn a_diverged_forward_refuses_the_epoch_and_leaves_adam_untouched() {
        let mut params = MoeParams::layout(256, 32, 3, 2);
        params.init_xavier(42);
        params.data[params.router_offset] = f32::NAN; // one poisoned router weight
        let mut adam = AdamState::new(params.total_params);
        let mut accum = GradAccumulator::new(params.total_params);
        let cfg = TrainConfig { grad_accum_steps: 1, ..Default::default() };
        let tokens: Vec<u32> = b"abcdef".iter().map(|&b| b as u32).collect(); // @forge:allow_alloc test fixture

        let loss = train_epoch(&mut params, &mut adam, &mut accum, &tokens, &cfg);
        assert!(loss.is_nan(), "a refused epoch scores NaN, not a partial average: {loss}");
        assert_eq!(adam.t, 0, "the refusal must land BEFORE adam.step");
        assert!(
            adam.m.iter().all(|v| *v == 0.0) && adam.v.iter().all(|v| *v == 0.0),
            "Adam's moments must be untouched by a diverged step"
        );
    }

    /// [BOARD: cadence-depth-aware-init] ONE home for the depth modulus. Init draws plain
    /// Xavier ±√(3/d); `forward` applies `1/√(2·L)` to the branch. Damping in both places
    /// multiplies to `1/(2L)` and starves the branch as depth grows — this pins the split.
    #[test]
    fn the_depth_modulus_lives_in_forward_not_in_init() {
        let mut p = MoeParams::layout(256, 32, 3, 2);
        p.init_xavier(42);
        let max_abs = |s: &[f32]| s.iter().fold(0f32, |a, &v| a.max(v.abs()));
        let embed_max = max_abs(&p.data[p.embed_offset..p.trunk_offset]);
        let trunk_max = max_abs(&p.data[p.trunk_offset..p.router_offset]);
        let expert_max = max_abs(&p.data[p.experts_offset..p.dcgs_offset]);
        // d_model 32 → plain Xavier bound √(3/32) ≈ 0.306; the OLD doubly-damped init
        // capped at √(3/32)/√8 ≈ 0.108, so this floor fails if the damp creeps back in.
        let xavier = (3.0f32 / 32.0).sqrt();
        assert!(embed_max <= 0.02 + 1e-6, "embed keeps the flat bound: {embed_max}");
        for (seg, got) in [("trunk", trunk_max), ("experts", expert_max)] {
            assert!(got > 0.15, "{seg} must be UNDAMPED Xavier, got {got} (damp crept into init)");
            assert!(got <= xavier + 1e-6, "{seg} may not exceed ±√(3/d): {got} > {xavier}");
        }
    }
}
