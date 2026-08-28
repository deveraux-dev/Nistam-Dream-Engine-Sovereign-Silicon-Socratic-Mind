// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! KV-prefill snapshot receipt: measures the REAL prefill skip.
//! Cold = compute prefill via `forward_token` (42 layers, measured per token).
//! Warm = `KvSnapshot::open` + hash guard + `restore_into` (pure memcpy).
//! Restored KV is asserted bit-identical to the computed KV. Weights are
//! SYNTHETIC (aliased across layers) — this stopwatch times the mechanism,
//! not model quality; per-token cost is measured, longer-prefix cold times
//! are labeled extrapolations.

use gemma_s13::model_9b::{DispatchEngine, Gemma9bConfig, Gemma9bForwardGraph, Gemma9bLayerWeights, Gemma9bModel};
use gemma_s13::prompt_cache::{fnv1a, snapshot_kv, KvSnapshot};
use gemma_s13::s13::{pack_5_trits, TRITS_PER_BYTE};
use std::time::Instant;

fn main() {
    println!("===============================================================================");
    println!("   S13 KV-PREFILL SNAPSHOT — MEASURED COLD PREFILL vs WARM RESTORE");
    println!("===============================================================================");

    let config = Gemma9bConfig::default();
    let d_model = config.d_model;
    let kv_dim = config.n_kv_heads * config.d_head;
    let n_layers = config.n_layers;
    let n_positions = 256usize; // KV window this snapshot carries
    let n_prefill = 4usize; // tokens actually computed (per-token cost is measured)

    // Synthetic packed weights, aliased across all 42 layers (mechanism
    // stopwatch — no quality claim).
    let bytes_per_row = |cols: usize| (cols + TRITS_PER_BYTE - 1) / TRITS_PER_BYTE;
    let q_dim = config.n_heads * config.d_head;
    let q_packed = vec![pack_5_trits([1, 0, -1, 0, 1]).unwrap(); q_dim * bytes_per_row(d_model)];
    let k_packed = vec![pack_5_trits([0, 1, 0, -1, 0]).unwrap(); kv_dim * bytes_per_row(d_model)];
    let v_packed = vec![pack_5_trits([-1, 0, 1, 0, -1]).unwrap(); kv_dim * bytes_per_row(d_model)];
    let o_packed = vec![pack_5_trits([1, 1, 0, -1, -1]).unwrap(); d_model * bytes_per_row(q_dim)];
    let gate_packed = vec![pack_5_trits([1, -1, 0, 1, 0]).unwrap(); config.d_ff * bytes_per_row(d_model)];
    let up_packed = vec![pack_5_trits([0, 1, -1, 1, 0]).unwrap(); config.d_ff * bytes_per_row(d_model)];
    let down_packed = vec![pack_5_trits([1, 0, 1, -1, 0]).unwrap(); d_model * bytes_per_row(config.d_ff)];
    let norm_scale = vec![100i16; d_model];
    let tensor_scales = vec![1.0f32; 7];

    let layer = Gemma9bLayerWeights {
        q_proj: &q_packed,
        k_proj: &k_packed,
        v_proj: &v_packed,
        o_proj: &o_packed,
        gate_proj: &gate_packed,
        up_proj: &up_packed,
        down_proj: &down_packed,
        input_norm_scale: &norm_scale,
        post_attention_norm_scale: &norm_scale,
        scales: &tensor_scales,
    };
    let mut model = Gemma9bModel::new(config, &norm_scale, &[]);
    for i in 0..n_layers {
        model.set_layer(i, layer).unwrap();
    }
    let mut graph = Gemma9bForwardGraph::new(config, DispatchEngine::Avx2Pshufb);

    // The static prefix this KV state stands for (hash-guarded provenance).
    let prefix: &[u8] = b"[system prompt v3] [vixi context] [m5 manifold] [tool schemas] [static agent preamble]";
    let prefix_hash = fnv1a(prefix);

    // Caller-owned KV: one flat lane per layer, sized for the full window.
    let kv_len = kv_dim * n_positions;
    let mut k_store: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];
    let mut v_store: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];

    // ── [1] COLD: measured prefill compute ──
    let mut hidden = vec![0i16; d_model];
    for (i, h) in hidden.iter_mut().enumerate() {
        *h = ((prefix[i % prefix.len()] as i32) - 64) as i16;
    }
    let t0 = Instant::now();
    {
        let mut k_refs: Vec<&mut [i16]> = k_store.iter_mut().map(|v| v.as_mut_slice()).collect();
        let mut v_refs: Vec<&mut [i16]> = v_store.iter_mut().map(|v| v.as_mut_slice()).collect();
        for t in 0..n_prefill {
            graph.forward_token(&mut hidden, &mut k_refs, &mut v_refs, t, &model).unwrap();
        }
    }
    let cold = t0.elapsed();
    let ms_per_token = cold.as_secs_f64() * 1e3 / n_prefill as f64;
    println!("  [1] COLD prefill (computed)  : {n_prefill} tokens x {n_layers} layers = {:.1} ms ({:.1} ms/token measured)", cold.as_secs_f64() * 1e3, ms_per_token);
    println!("      extrapolated (labeled)   : 256-token prefix ~{:.1} s | 2000-token prefix ~{:.1} s", ms_per_token * 256.0 / 1e3, ms_per_token * 2000.0 / 1e3);

    // ── [2] SNAPSHOT to the durable home ──
    let cache_dir = std::env::var("S13_KV_CACHE_DIR").unwrap_or_else(|_| "kv-cache".to_string());
    std::fs::create_dir_all(&cache_dir).unwrap();
    let path = std::path::Path::new(&cache_dir).join(format!("{prefix_hash:016x}.s13kv"));
    let k_refs: Vec<&[i16]> = k_store.iter().map(|v| v.as_slice()).collect();
    let v_refs: Vec<&[i16]> = v_store.iter().map(|v| v.as_slice()).collect();
    let t0 = Instant::now();
    snapshot_kv(&path, prefix_hash, n_prefill as u32, &k_refs, &v_refs).unwrap();
    let snap_ms = t0.elapsed().as_secs_f64() * 1e3;
    let file_mb = std::fs::metadata(&path).unwrap().len() as f64 / 1_048_576.0;
    println!("  [2] SNAPSHOT written         : {} ({:.1} MB in {:.1} ms)", path.display(), file_mb, snap_ms);

    // ── [3] WARM: open + hash guard + restore into FRESH buffers ──
    let mut k_fresh: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];
    let mut v_fresh: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];
    let t0 = Instant::now();
    let snap = KvSnapshot::open(&path).unwrap();
    assert!(snap.matches_prefix(prefix), "hash guard must accept the exact prefix");
    assert!(!snap.matches_prefix(b"tampered prefix"), "hash guard must reject any other prefix");
    {
        let mut k_mut: Vec<&mut [i16]> = k_fresh.iter_mut().map(|v| v.as_mut_slice()).collect();
        let mut v_mut: Vec<&mut [i16]> = v_fresh.iter_mut().map(|v| v.as_mut_slice()).collect();
        snap.restore_into(&mut k_mut, &mut v_mut).unwrap();
    }
    let warm_ms = t0.elapsed().as_secs_f64() * 1e3;
    println!("  [3] WARM restore (mmap+copy) : {:.1} ms for the full {} MB window (token_pos resumes at {})", warm_ms, file_mb.round(), snap.token_pos);

    // ── [4] Bit-identity: restored KV == computed KV ──
    assert_eq!(k_fresh, k_store, "restored K must be bit-identical to computed K");
    assert_eq!(v_fresh, v_store, "restored V must be bit-identical to computed V");
    println!("  [4] PARITY                   : restored KV BIT-IDENTICAL to computed KV ({} layers)", n_layers);

    let ratio_2000 = (ms_per_token * 2000.0) / warm_ms;
    println!("  [5] VERDICT                  : warm restore replaces a 2000-token cold prefill at ~{:.0}x less wall time (extrapolated cold, measured warm)", ratio_2000);
    println!("===============================================================================");
}
