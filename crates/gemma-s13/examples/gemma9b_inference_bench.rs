// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Gemma 9B S13 Balanced Ternary Inference & Mtok/s Throughput Benchmark.

#![allow(unsafe_code)]

use gemma_s13::gpu_warden::EmulatedU64;
use gemma_s13::model_9b::{DispatchEngine, Gemma9bConfig, Gemma9bForwardGraph, Gemma9bLayerWeights, Gemma9bModel};
use gemma_s13::nipr::NormalizedIpr;
use gemma_s13::s13::{pack_5_trits, TRITS_PER_BYTE};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    println!("===============================================================================");
    println!("   GEMMA 9B S13 BALANCED TERNARY INFERENCE & MTOK/S THROUGHPUT BENCHMARK");
    println!("===============================================================================\n");

    let config = Gemma9bConfig::default();
    println!("Gemma 9B Architecture Configuration:");
    println!("  • Hidden Dimension (d_model)   : {}", config.d_model);
    println!("  • Query Heads / KV Heads       : {} / {} (d_head = {})", config.n_heads, config.n_kv_heads, config.d_head);
    println!("  • Transformer Layers           : {}", config.n_layers);
    println!("  • Intermediate FFN (d_ff)      : {}", config.d_ff);
    println!("  • Weights Per Layer            : {} weights", config.weights_per_layer());
    println!("  • S13 Packed Per Layer         : {} bytes ({:.2} MB)", config.packed_bytes_per_layer(), config.packed_bytes_per_layer() as f64 / 1_048_576.0);
    println!("  • Total 42-Layer Backbone      : {} bytes ({:.3} GB)", config.total_backbone_packed_bytes(), config.total_backbone_packed_bytes() as f64 / 1_073_741_824.0);
    println!("  • Total Footprint (with 256k)  : {} bytes ({:.3} GB)", config.total_model_packed_bytes(), config.total_model_packed_bytes() as f64 / 1_073_741_824.0);
    println!("-------------------------------------------------------------------------------\n");

    // ========================================================================
    // Benchmark 1: Raw AVX2 PSHUFB In-Register Vector MatMul Kernel
    // ========================================================================
    println!("--- [1] Raw Vector Kernel Throughput (S13 Ternary Dot Product) ---");
    let num_weights = 10_000_000usize;
    let num_bytes = (num_weights + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let mut packed_data = vec![0u8; num_bytes];
    for (i, b) in packed_data.iter_mut().enumerate() {
        let t0 = ((i * 3) % 3) as i8 - 1;
        let t1 = ((i * 5 + 1) % 3) as i8 - 1;
        let t2 = ((i * 7 + 2) % 3) as i8 - 1;
        let t3 = ((i * 11) % 3) as i8 - 1;
        let t4 = ((i * 13 + 1) % 3) as i8 - 1;
        *b = pack_5_trits([t0, t1, t2, t3, t4]).unwrap_or(0);
    }
    let mut activations = vec![0i16; num_weights];
    for (i, a) in activations.iter_mut().enumerate() {
        *a = (((i * 17) % 2000) as i32 - 1000) as i16;
    }

    // Warm-up
    for _ in 0..3 {
        let _ = black_box(gemma_s13::s13::ternary_matmul_vector_scalar(&packed_data[..1000], &activations[..5000], 10_000));
    }

    let iters_raw = 50usize;
    let t0 = Instant::now();
    let mut sum_scalar = 0i64;
    for _ in 0..iters_raw {
        let res = gemma_s13::s13::ternary_matmul_vector_scalar(&packed_data, &activations, 10_000).unwrap();
        sum_scalar = sum_scalar.wrapping_add(res as i64);
    }
    let dur_scalar = t0.elapsed();
    let total_weights_evaluated = num_weights as f64 * iters_raw as f64;
    let gweights_sec_scalar = (total_weights_evaluated / dur_scalar.as_secs_f64()) / 1e9;
    let mtok_sec_scalar = (total_weights_evaluated / 3584.0 / dur_scalar.as_secs_f64()) / 1e6;

    println!("  • Scalar Reference Kernel      : {:.3} Gweights/s | {:.2} Mtok/s (checksum: {})", gweights_sec_scalar, mtok_sec_scalar, sum_scalar);

    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        let t0_avx2 = Instant::now();
        let mut sum_avx2 = 0i64;
        for _ in 0..iters_raw {
            unsafe {
                let res = gemma_s13::s13::avx2_unpacker::matmul_vector_avx2(&packed_data, &activations, 10_000).unwrap();
                sum_avx2 = sum_avx2.wrapping_add(res as i64);
            }
        }
        let dur_avx2 = t0_avx2.elapsed();
        let gweights_sec_avx2 = (total_weights_evaluated / dur_avx2.as_secs_f64()) / 1e9;
        let mtok_sec_avx2 = (total_weights_evaluated / 3584.0 / dur_avx2.as_secs_f64()) / 1e6;
        let speedup = dur_scalar.as_secs_f64() / dur_avx2.as_secs_f64();
        println!("  • AVX2 PSHUFB In-Register SIMD : {:.3} Gweights/s | {:.2} Mtok/s (speedup: {:.2}x)", gweights_sec_avx2, mtok_sec_avx2, speedup);
    }

    // ========================================================================
    // Benchmark 2: Gemma 9B Attention Projection Block GEMV
    // ========================================================================
    println!("\n--- [2] Gemma 9B Attention Projection GEMV (d_model=3584 -> 4096) ---");
    let q_dim = config.n_heads * config.d_head; // 4096
    let bytes_per_q_row = (config.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let q_weights_bytes = q_dim * bytes_per_q_row;
    let q_packed = vec![pack_5_trits([1, 0, -1, 0, 1]).unwrap(); q_weights_bytes];
    let norm_act = vec![100i16; config.d_model];
    let mut q_out = vec![0i16; q_dim];

    let graph_avx2 = Gemma9bForwardGraph::new(config, DispatchEngine::Avx2Pshufb);
    let iters_gemv = 200usize;
    let t0 = Instant::now();
    for _ in 0..iters_gemv {
        graph_avx2.dispatch_gemv(&q_packed, &norm_act, &mut q_out, config.d_model).unwrap();
    }
    let dur_gemv = t0.elapsed();
    let time_per_proj_us = (dur_gemv.as_secs_f64() * 1e6) / iters_gemv as f64;
    let q_weights_total = (config.d_model * q_dim) as f64 * iters_gemv as f64;
    let q_gweights_sec = (q_weights_total / dur_gemv.as_secs_f64()) / 1e9;

    println!("  • Q-Projection (3584 x 4096)   : {:.2} µs/proj | {:.3} Gweights/s", time_per_proj_us, q_gweights_sec);

    // ========================================================================
    // Benchmark 3: Gemma 9B Gated FFN Block GEMV (3584 -> 14336 -> 3584)
    // ========================================================================
    println!("\n--- [3] Gemma 9B Gated FFN Block GEMV (3584 -> 14336 -> 3584) ---");
    let ffn_row_bytes = (config.d_model + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let ffn_gate_bytes = config.d_ff * ffn_row_bytes;
    let gate_packed = vec![pack_5_trits([1, -1, 0, 1, 0]).unwrap(); ffn_gate_bytes];
    let up_packed = vec![pack_5_trits([0, 1, -1, 1, 0]).unwrap(); ffn_gate_bytes];
    let down_row_bytes = (config.d_ff + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let down_packed = vec![pack_5_trits([1, 0, 1, -1, 0]).unwrap(); config.d_model * down_row_bytes];

    // KV and O projection sizes
    let kv_dim = config.n_kv_heads * config.d_head; // 2048
    let k_bytes = kv_dim * bytes_per_q_row;
    let k_packed = vec![pack_5_trits([0, 1, 0, -1, 0]).unwrap(); k_bytes];
    let v_packed = vec![pack_5_trits([-1, 0, 1, 0, -1]).unwrap(); k_bytes];
    let o_row_bytes = (q_dim + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let o_packed = vec![pack_5_trits([1, 1, 0, -1, -1]).unwrap(); config.d_model * o_row_bytes];

    let tensor_scales = vec![1.0f32; 7];
    let full_layer_weights = Gemma9bLayerWeights {
        q_proj: &q_packed,
        k_proj: &k_packed,
        v_proj: &v_packed,
        o_proj: &o_packed,
        gate_proj: &gate_packed,
        up_proj: &up_packed,
        down_proj: &down_packed,
        input_norm_scale: &norm_act,
        post_attention_norm_scale: &norm_act,
        scales: &tensor_scales,
    };

    let mut ffn_out = vec![0i16; config.d_model];
    let iters_ffn = 50usize;
    let t0 = Instant::now();
    for _ in 0..iters_ffn {
        graph_avx2.ffn_block(&norm_act, &full_layer_weights, &mut ffn_out).unwrap();
    }
    let dur_ffn = t0.elapsed();
    let time_per_ffn_us = (dur_ffn.as_secs_f64() * 1e6) / iters_ffn as f64;
    let ffn_weights_total = (config.d_model * config.d_ff * 3) as f64 * iters_ffn as f64;
    let ffn_gweights_sec = (ffn_weights_total / dur_ffn.as_secs_f64()) / 1e9;

    println!("  • Gated FFN (Gate+Up+GeGLU+Down): {:.2} µs/step ({:.3} ms) | {:.3} Gweights/s", time_per_ffn_us, time_per_ffn_us / 1000.0, ffn_gweights_sec);

    // ========================================================================
    // Benchmark 4: Single Full Gemma 9B Layer Decode Step
    // ========================================================================
    println!("\n--- [4] Single Full Transformer Layer Step (RMSNorm + Attn + FFN + Residuals) ---");
    let mut single_layer_config = config;
    single_layer_config.n_layers = 1;

    let mut layer_model = Gemma9bModel::new(single_layer_config, &norm_act, &[]);
    layer_model.set_layer(0, full_layer_weights).unwrap();

    let mut hidden_state = vec![150i16; config.d_model];
    let mut k_cache_layer = vec![0i16; config.d_head * config.n_kv_heads * 64];
    let mut v_cache_layer = vec![0i16; config.d_head * config.n_kv_heads * 64];
    let mut k_refs = vec![&mut k_cache_layer[..]];
    let mut v_refs = vec![&mut v_cache_layer[..]];

    let mut single_layer_graph = Gemma9bForwardGraph::new(single_layer_config, DispatchEngine::Avx2Pshufb);

    let iters_layer = 20usize;
    let t0 = Instant::now();
    for t in 0..iters_layer {
        let _ = single_layer_graph.forward_token(&mut hidden_state, &mut k_refs, &mut v_refs, t, &layer_model).unwrap();
    }
    let dur_layer = t0.elapsed();
    let time_per_layer_ms = (dur_layer.as_secs_f64() * 1000.0) / iters_layer as f64;
    let layer_weights_total = config.weights_per_layer() as f64 * iters_layer as f64;
    let layer_gweights_sec = (layer_weights_total / dur_layer.as_secs_f64()) / 1e9;

    println!("  • Single 9B Layer Step Latency : {:.3} ms/layer | {:.3} Gweights/s", time_per_layer_ms, layer_gweights_sec);

    // ========================================================================
    // Benchmark 5: 42-Layer Full Backbone Extrapolation & Hardware Profiles
    // ========================================================================
    println!("\n--- [5] Full 42-Layer Gemma 9B Forward Decode Projection ---");
    let full_42_latency_ms = time_per_layer_ms * 42.0;
    let cpu_tok_sec = 1000.0 / full_42_latency_ms;

    println!("  • CPU Host (Single-Thread)      : {:.2} ms/token => {:.2} tokens/sec", full_42_latency_ms, cpu_tok_sec);

    // Hardware GPU Warden RTX 3070 Roofline Model
    let rtx_3070_bandwidth_gb_s = 448.0;
    let s13_footprint_gb = config.total_model_packed_bytes() as f64 / 1_073_741_824.0;
    let gpu_decode_latency_ms = (s13_footprint_gb / rtx_3070_bandwidth_gb_s) * 1000.0;
    let gpu_tok_sec = 1000.0 / gpu_decode_latency_ms;

    println!("  • GPU SplitShader (RTX 3070)    : {:.2} ms/token => {:.1} tokens/sec ({:.2} GB VRAM)", gpu_decode_latency_ms, gpu_tok_sec, s13_footprint_gb);

    // ========================================================================
    // Benchmark 6: Normalized IPR (N × IPR) Dimensional Collapse Sieve
    // ========================================================================
    println!("\n--- [6] Normalized IPR (N × IPR) Zero-Transcendental Attention Sieve ---");
    let n_dim = 4096usize;
    let mut attn_vec = vec![0u16; n_dim];
    for (i, v) in attn_vec.iter_mut().enumerate() {
        *v = if i == 42 { 9500 } else { (i % 20) as u16 };
    }

    let iters_ipr = 100_000usize;
    let t0 = Instant::now();
    let mut ipr_acc = 0u64;
    for _ in 0..iters_ipr {
        let ipr = NormalizedIpr::compute_u16(&attn_vec);
        ipr_acc = ipr_acc.wrapping_add(ipr.pmy as u64);
    }
    let dur_ipr = t0.elapsed();
    let time_per_ipr_ns = (dur_ipr.as_secs_f64() * 1e9) / iters_ipr as f64;
    let ipr_evals_sec = (iters_ipr as f64 / dur_ipr.as_secs_f64()) / 1e6;

    println!("  • N×IPR Sieve (N=4096)         : {:.2} ns/eval | {:.2} Meval/s (sample pmy: {})", time_per_ipr_ns, ipr_evals_sec, ipr_acc / iters_ipr as u64);

    // ========================================================================
    // Benchmark 7: GPU Warden Emulated 64-Bit Fixed-Point Math
    // ========================================================================
    println!("\n--- [7] GPU Warden Emulated 64-Bit Fixed-Point Arithmetic ---");
    let iters_emu = 10_000_000usize;
    let mut emu_a = EmulatedU64::from_u64(0x1234_5678_9ABC_DEF0);
    let emu_b = EmulatedU64::from_u64(0x0FED_CBA9_8765_4321);

    let t0 = Instant::now();
    for _ in 0..iters_emu {
        emu_a = black_box(emu_a.add(emu_b));
    }
    let dur_emu = t0.elapsed();
    let time_per_emu_ns = (dur_emu.as_secs_f64() * 1e9) / iters_emu as f64;
    let emu_mops_sec = (iters_emu as f64 / dur_emu.as_secs_f64()) / 1e6;

    println!("  • Emulated 64-bit Dual Add/Sub : {:.2} ns/op | {:.2} Mops/s (final: {:016X})", time_per_emu_ns, emu_mops_sec, emu_a.to_u64());

    println!("\n===============================================================================");
    println!("                           BENCHMARK SUITE COMPLETE");
    println!("===============================================================================");
}
