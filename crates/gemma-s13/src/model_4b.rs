// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Gemma 3 4B (Mama Bear Assist Direct seat) S13 configuration.
//!
//! The forward graph, GEMV dispatch engines, and weight containers in
//! [`crate::model_9b`] are fully config-driven (the 42-slot layer arena is an
//! upper bound; 4B uses 34 of it, and every scratch buffer is sized for the
//! larger 9B dims). This module therefore contributes only the Gemma 3 4B
//! architectural geometry:
//! - $d_{\text{model}} = 2560$, $n_{\text{heads}} = 8$, $n_{\text{kv\_heads}} = 4$, $d_{\text{head}} = 256$,
//! - $n_{\text{layers}} = 34$, $d_{\text{ff}} = 10240$, $\text{vocab\_size} = 262{,}144$.
//! - Total 1.58-bit balanced ternary weight footprint: $\approx 0.776\text{ GB}$ ($5\text{ trits}/\text{byte}$).

pub use crate::model_9b::{
    DispatchEngine, ForwardTelemetry, Gemma9bConfig, Gemma9bForwardGraph, Gemma9bLayerWeights,
    Gemma9bModel,
};

/// Gemma 3 4B seat configuration (shares the config-driven 9B machinery).
pub type Gemma4bConfig = Gemma9bConfig;

/// Gemma 3 4B architectural dimensions for the Mama Bear Assist Direct seat.
pub const fn gemma4b_config() -> Gemma4bConfig {
    Gemma4bConfig {
        d_model: 2560,
        n_heads: 8,
        n_kv_heads: 4,
        d_head: 256,
        n_layers: 34,
        d_ff: 10240,
        vocab_size: 262_144,
        max_seq_len: 8192,
        rope_theta: 10_000,
        permyriad_scale: 10_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemma_4b_config_dimensions_and_footprint() {
        let config = gemma4b_config();
        assert_eq!(config.d_model, 2560);
        assert_eq!(config.n_heads, 8);
        assert_eq!(config.n_kv_heads, 4);
        assert_eq!(config.d_head, 256);
        assert_eq!(config.n_layers, 34);
        assert_eq!(config.d_ff, 10240);

        // Per-layer weight count: q + k + v + o + 3×ffn
        let w_layer = config.weights_per_layer();
        assert_eq!(w_layer, 94_371_840);

        // On-disk bytes per layer: seven .s13m files, each padded to its own
        // 5-trits/byte boundary and each carrying a 16-byte header
        // (model_9b.rs:101-104). 18_874_368 is the header-less figure and is
        // short by exactly 7 * 16 == 112.
        let b_layer = config.packed_bytes_per_layer();
        assert_eq!(b_layer, 18_874_480);

        // Total 34-layer backbone footprint. This is not a derivation — it is
        // the measured size of the 34-layer seat on disk (238 = 34 * 7 files),
        // and it is what vram_budget::DISK_34L pins independently.
        let total_backbone = config.total_backbone_packed_bytes();
        assert_eq!(total_backbone, 641_732_320);
        assert_eq!(b_layer * config.n_layers, total_backbone);

        // Total model footprint including 262k static vocab embedding (~0.776 GB)
        let total_model = config.total_model_packed_bytes();
        assert!(total_model > 770_000_000 && total_model < 780_000_000);
    }

    #[test]
    fn test_4b_config_fits_the_shared_layer_arena_and_buffers() {
        let config = gemma4b_config();
        // 42-slot layer arena upper bound in Gemma9bModel
        assert!(config.n_layers <= 42);
        // scratch buffers in the shared forward graph are sized for 9B dims
        assert!(config.d_model <= 3584);
        assert!(config.n_heads * config.d_head <= 4096);
        assert!(config.n_kv_heads * config.d_head <= 2048);
    }

    #[test]
    fn test_4b_forward_graph_runs_on_shared_machinery() {
        let mut config = gemma4b_config();
        // shrink to a unit-scale layer so the test owns its weights on stack
        config.d_model = 10;
        config.n_heads = 2;
        config.n_kv_heads = 1;
        config.d_head = 4;
        config.n_layers = 1;
        config.d_ff = 20;

        let graph = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);
        let activations = [50i16, 20, -30, 40, -10];
        let row_bytes = [crate::s13::pack_5_trits([1, -1, 1, 0, -1]).unwrap()];
        let mut out = [0i16; 1];
        graph.dispatch_gemv(&row_bytes, &activations, &mut out, 5).unwrap();
        assert_ne!(out[0], 0);
    }
}
