//! Fleet VRAM budget oracle: what the resident models and their KV actually cost.
//! Every geometry here is MEASURED off the packed bears (2026-08-26), never declared.
//! Tests assert the arithmetic against real on-disk byte counts.

/// Bytes in one MB, as the driver and `nvidia-smi` report them.
pub const BYTES_PER_MB: usize = 1024 * 1024;

/// Packed-tensor container. Both appear in the live fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum S13Format {
    /// `S13M`: 5 balanced trits per byte behind a 16-byte header.
    S13m,
    /// `S133`: `S13M`'s trits plus one scale byte per 32 weights, 20-byte header.
    S133,
}

impl S13Format {
    /// Header bytes ahead of the payload.
    pub const fn header_bytes(self) -> usize {
        match self {
            S13Format::S13m => 16,
            S13Format::S133 => 20,
        }
    }

    /// Weights covered by one shared scale byte; `S13M` carries no scales.
    pub const fn weights_per_scale(self) -> usize {
        match self {
            S13Format::S13m => 0,
            S13Format::S133 => 32,
        }
    }

    /// On-disk size of one tensor file holding `weights` trits.
    pub const fn file_bytes(self, weights: usize) -> usize {
        let trits = weights.div_ceil(5);
        let scales = match self.weights_per_scale() {
            0 => 0,
            n => weights / n,
        };
        trits + scales + self.header_bytes()
    }
}

/// KV cache element width. The single biggest lever on context length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvWidth {
    /// `i16` — what `Gemma9bForwardGraph::forward_token` takes today.
    I16,
    /// `i8` — halves the cache, doubles the affordable context.
    I8,
}

impl KvWidth {
    /// Bytes per cached element.
    pub const fn bytes(self) -> usize {
        match self {
            KvWidth::I16 => 2,
            KvWidth::I8 => 1,
        }
    }
}

/// One model's measured shape. `extra_bytes` carries non-`blk_*` payload
/// (LoRA bundles, model-level norms) that geometry alone cannot derive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelGeometry {
    /// Directory name under the repo root.
    pub name: &'static str,
    /// Hidden dimension.
    pub d_model: usize,
    /// Feed-forward intermediate dimension.
    pub d_ff: usize,
    /// Decoder layer count.
    pub n_layers: usize,
    /// Query attention heads.
    pub n_heads: usize,
    /// Key/value heads (grouped-query).
    pub n_kv_heads: usize,
    /// Dimension per head.
    pub d_head: usize,
    /// Packed-tensor container.
    pub format: S13Format,
    /// Per-layer `.s13n` norm bytes, 0 when the model ships none.
    pub norm_bytes_per_layer: usize,
    /// Non-`blk_*` bytes (model-level norms, LoRA bundle).
    pub extra_bytes: usize,
}

impl ModelGeometry {
    /// On-disk bytes for the seven `blk_*` weight tensors of one layer.
    pub const fn core_bytes_per_layer(&self) -> usize {
        let q = self.d_model * self.n_heads * self.d_head;
        let kv = self.d_model * self.n_kv_heads * self.d_head;
        let ffn = self.d_model * self.d_ff;
        self.format.file_bytes(q) * 2      // q_proj + o_proj
            + self.format.file_bytes(kv) * 2 // k_proj + v_proj
            + self.format.file_bytes(ffn) * 3 // gate + up + down
    }

    /// Every byte the model occupies on disk: core tensors, per-layer norms,
    /// and any non-`blk_*` payload.
    pub const fn weight_bytes(&self) -> usize {
        (self.core_bytes_per_layer() + self.norm_bytes_per_layer) * self.n_layers + self.extra_bytes
    }

    /// KV bytes for a single token: K and V, every layer, every kv head.
    pub const fn kv_bytes_per_token(&self, width: KvWidth) -> usize {
        2 * self.n_layers * self.n_kv_heads * self.d_head * width.bytes()
    }

    /// KV bytes to hold `tokens` of context.
    pub const fn kv_bytes_for_context(&self, tokens: usize, width: KvWidth) -> usize {
        self.kv_bytes_per_token(width) * tokens
    }
}

/// `s13_gemma_9b` — 42 layers, the fleet's intent backbone.
pub const GEMMA_9B: ModelGeometry = ModelGeometry {
    name: "s13_gemma_9b",
    d_model: 3584, d_ff: 14336, n_layers: 42,
    n_heads: 16, n_kv_heads: 8, d_head: 256,
    format: S13Format::S13m, norm_bytes_per_layer: 0, extra_bytes: 0,
};

/// `s13_gemma_2b` — 26 layers. Both the direct and mirror slots resolve here.
pub const GEMMA_2B: ModelGeometry = ModelGeometry {
    name: "s13_gemma_2b",
    d_model: 2304, d_ff: 9216, n_layers: 26,
    n_heads: 8, n_kv_heads: 4, d_head: 256,
    format: S13Format::S13m, norm_bytes_per_layer: 0, extra_bytes: 0,
};

/// `s13_gemma_2b_m3` — the 2B architecture in the scale-blocked `S133` format.
pub const GEMMA_2B_M3: ModelGeometry = ModelGeometry {
    name: "s13_gemma_2b_m3",
    d_model: 2304, d_ff: 9216, n_layers: 26,
    n_heads: 8, n_kv_heads: 4, d_head: 256,
    format: S13Format::S133, norm_bytes_per_layer: 0, extra_bytes: 0,
};

/// `s13_gemma` — 34 layers, 2560-wide.
pub const GEMMA_34L: ModelGeometry = ModelGeometry {
    name: "s13_gemma",
    d_model: 2560, d_ff: 10240, n_layers: 34,
    n_heads: 8, n_kv_heads: 4, d_head: 256,
    format: S13Format::S13m, norm_bytes_per_layer: 0, extra_bytes: 0,
};

/// `s13_gemma_m2` — the 34-layer architecture in `S133`, plus four `.s13n`
/// norms per layer and a `bundle.s3lora`.
pub const GEMMA_M2: ModelGeometry = ModelGeometry {
    name: "s13_gemma_m2",
    d_model: 2560, d_ff: 10240, n_layers: 34,
    n_heads: 8, n_kv_heads: 4, d_head: 256,
    format: S13Format::S133, norm_bytes_per_layer: 22_560, extra_bytes: 59_632_626,
};

/// A fleet slot. `shares_weights` marks a model already resident under another
/// slot — the anti-expert mirror reads the same directory as the direct bear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FleetMember {
    /// The slot's measured geometry.
    pub geom: ModelGeometry,
    /// True when this slot reuses a prior slot's resident weights.
    pub shares_weights: bool,
}

/// Everything resident that is not model weights or KV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overheads {
    /// Our own presentation swapchain and UI raster targets.
    pub framebuffers_mb: u32,
    /// Timeline-semaphore staging and DMA command rings.
    pub warden_rings_mb: u32,
    /// Double-buffered dequant tile: the 9B's largest FFN tensor at fp16.
    pub dequant_staging_mb: u32,
}

impl Overheads {
    /// Total overhead bytes.
    pub const fn bytes(&self) -> usize {
        (self.framebuffers_mb + self.warden_rings_mb + self.dequant_staging_mb) as usize
            * BYTES_PER_MB
    }
}

/// The demo's overheads. The dequant tile is the 9B's 3584x14336 FFN at fp16
/// (98 MB), double-buffered and rounded up.
pub const DEMO_OVERHEADS: Overheads = Overheads {
    framebuffers_mb: 512,
    warden_rings_mb: 128,
    dequant_staging_mb: 256,
};

/// The judge-demo roster: 9B backbone, direct 2B, its anti-expert mirror
/// (same directory, shared weights), the manifold codec, and the sentry.
pub const DEMO_FLEET: [FleetMember; 5] = [
    FleetMember { geom: GEMMA_9B,     shares_weights: false },
    FleetMember { geom: GEMMA_2B,     shares_weights: false },
    FleetMember { geom: GEMMA_2B,     shares_weights: true },
    FleetMember { geom: GEMMA_2B_M3,  shares_weights: false },
    FleetMember { geom: GEMMA_M2,     shares_weights: false },
];

/// A fleet sized against a real card, with a measured idle baseline.
#[derive(Clone, Copy, Debug)]
pub struct FleetBudget<'a> {
    /// Card capacity in MB as the driver reports it.
    pub card_mb: u32,
    /// MB already resident before the fleet loads (desktop compositor, browser,
    /// driver). MEASURED, not assumed — see the module tests.
    pub baseline_resident_mb: u32,
    /// The roster.
    pub members: &'a [FleetMember],
    /// Context length every member must hold simultaneously.
    pub ctx_tokens: usize,
    /// KV element width.
    pub kv_width: KvWidth,
    /// Non-model resident cost.
    pub overheads: Overheads,
}

impl FleetBudget<'_> {
    /// Resident weight bytes, counting a shared-weight slot only once.
    pub fn weight_bytes(&self) -> usize {
        self.members
            .iter()
            .filter(|m| !m.shares_weights)
            .map(|m| m.geom.weight_bytes())
            .sum()
    }

    /// KV bytes for one token across every slot. A shared-weight mirror still
    /// needs its OWN cache — it shares weights, not attention state.
    pub fn kv_bytes_per_token(&self) -> usize {
        self.members.iter().map(|m| m.geom.kv_bytes_per_token(self.kv_width)).sum()
    }

    /// KV bytes at the configured context.
    pub fn kv_bytes(&self) -> usize {
        self.kv_bytes_per_token() * self.ctx_tokens
    }

    /// Everything the fleet adds to the card.
    pub fn committed_bytes(&self) -> usize {
        self.weight_bytes() + self.kv_bytes() + self.overheads.bytes()
    }

    /// Bytes left for the fleet after the card's idle baseline.
    pub fn usable_bytes(&self) -> usize {
        (self.card_mb.saturating_sub(self.baseline_resident_mb)) as usize * BYTES_PER_MB
    }

    /// Whether the fleet fits alongside what is already resident.
    pub fn fits(&self) -> bool {
        self.committed_bytes() <= self.usable_bytes()
    }

    /// Bytes to spare, 0 when over budget.
    pub fn headroom_bytes(&self) -> usize {
        self.usable_bytes().saturating_sub(self.committed_bytes())
    }

    /// Largest context that still fits, given the weights and overheads.
    pub fn max_ctx_tokens(&self) -> usize {
        let fixed = self.weight_bytes() + self.overheads.bytes();
        let per_token = self.kv_bytes_per_token();
        if per_token == 0 {
            return 0;
        }
        self.usable_bytes().saturating_sub(fixed) / per_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Byte counts measured off F:\v3\s13_gemma*\ on 2026-08-26.
    const DISK_9B: usize = 1_664_724_054;
    const DISK_2B: usize = 404_858_194;
    const DISK_2B_M3: usize = 468_117_546;
    const DISK_34L: usize = 641_732_320;
    const DISK_M2: usize = 802_403_018;

    #[test]
    fn geometry_reproduces_every_packed_bear_byte_for_byte() {
        assert_eq!(GEMMA_9B.weight_bytes(), DISK_9B, "s13_gemma_9b");
        assert_eq!(GEMMA_2B.weight_bytes(), DISK_2B, "s13_gemma_2b");
        assert_eq!(GEMMA_2B_M3.weight_bytes(), DISK_2B_M3, "s13_gemma_2b_m3");
        assert_eq!(GEMMA_34L.weight_bytes(), DISK_34L, "s13_gemma");
        assert_eq!(GEMMA_M2.weight_bytes(), DISK_M2, "s13_gemma_m2");
    }

    #[test]
    fn the_two_container_formats_match_measured_tensor_files() {
        // s13_gemma_9b/blk_0_ffn_up_weight.s13m
        assert_eq!(S13Format::S13m.file_bytes(3584 * 14336), 10_276_061);
        // s13_gemma_9b/blk_0_attn_k_weight.s13m
        assert_eq!(S13Format::S13m.file_bytes(3584 * 2048), 1_468_023);
        // s13_gemma_2b/blk_0_ffn_up_weight.s13m
        assert_eq!(S13Format::S13m.file_bytes(2304 * 9216), 4_246_749);
        // s13_gemma_2b_m3/blk_0_ffn_up_weight.s13m — same dims, scale bytes added
        assert_eq!(S13Format::S133.file_bytes(2304 * 9216), 4_910_305);
        // s13_gemma_m2/blk_0_attn_k_weight.s13m
        assert_eq!(S13Format::S133.file_bytes(2560 * 1024), 606_228);
    }

    #[test]
    fn kv_per_token_matches_the_proven_attention_geometry() {
        assert_eq!(GEMMA_9B.kv_bytes_per_token(KvWidth::I16), 344_064);
        assert_eq!(GEMMA_9B.kv_bytes_per_token(KvWidth::I8), 172_032);
        assert_eq!(GEMMA_2B.kv_bytes_per_token(KvWidth::I16), 106_496);
        assert_eq!(GEMMA_2B.kv_bytes_per_token(KvWidth::I8), 53_248);
        assert_eq!(GEMMA_M2.kv_bytes_per_token(KvWidth::I8), 69_632);
    }

    fn demo_budget(ctx: usize, width: KvWidth) -> FleetBudget<'static> {
        FleetBudget {
            card_mb: 8192,
            baseline_resident_mb: 1604,
            members: &DEMO_FLEET,
            ctx_tokens: ctx,
            kv_width: width,
            overheads: DEMO_OVERHEADS,
        }
    }

    #[test]
    fn the_mirror_slot_shares_weights_but_not_its_cache() {
        let b = demo_budget(4096, KvWidth::I8);
        // Four distinct directories resident, not five.
        assert_eq!(
            b.weight_bytes(),
            DISK_9B + DISK_2B + DISK_2B_M3 + DISK_M2,
            "the mirror must not pay for a second copy of s13_gemma_2b"
        );
        // But five caches: 9B + 2B + mirror-2B + m3 + m2.
        assert_eq!(b.kv_bytes_per_token(), 172_032 + 53_248 + 53_248 + 53_248 + 69_632);
        assert_eq!(b.kv_bytes_per_token(), 401_408);
    }

    #[test]
    fn the_128k_fleet_does_not_fit_an_8gb_card() {
        let b = demo_budget(131_072, KvWidth::I16);
        assert!(!b.fits(), "128k context must not fit — the doc claimed it did");
        // Not marginal: at i16 the cache alone is ~105 GB, fifteen times the
        // whole usable card. The doc budgeted 736 MB for it.
        assert!(
            b.kv_bytes() > 10 * b.usable_bytes(),
            "kv {} vs usable {}",
            b.kv_bytes(),
            b.usable_bytes()
        );
    }

    #[test]
    fn the_fleet_fits_at_4k_on_i8_kv() {
        let b = demo_budget(4096, KvWidth::I8);
        assert!(b.fits(), "committed {} usable {}", b.committed_bytes(), b.usable_bytes());
        assert!(b.headroom_bytes() > 0);
    }

    #[test]
    fn i8_buys_back_double_the_context() {
        let wide = demo_budget(0, KvWidth::I16).max_ctx_tokens();
        let narrow = demo_budget(0, KvWidth::I8).max_ctx_tokens();
        // Floor division: halving the element width doubles the budget, but the
        // truncated quotient can land one token higher than twice the wide one.
        assert!(narrow == wide * 2 || narrow == wide * 2 + 1, "wide {wide} narrow {narrow}");
        // The honest ceiling, with the mirror sharing the direct bear's weights.
        assert!(narrow > 6_000 && narrow < 7_000, "i8 ceiling was {narrow}");
        assert!(wide > 3_000 && wide < 3_500, "i16 ceiling was {wide}");
    }

    #[test]
    fn kv_overtakes_the_weights_once_context_is_real() {
        let b = demo_budget(4096, KvWidth::I8);
        // At 4k the weights still dominate — this is why the intuition holds early.
        assert!(b.kv_bytes() < b.weight_bytes());
        // By 16k the cache is the larger half, and the card is long gone.
        let deep = demo_budget(16_384, KvWidth::I8);
        assert!(deep.kv_bytes() > deep.weight_bytes());
        assert!(!deep.fits());
    }
}
