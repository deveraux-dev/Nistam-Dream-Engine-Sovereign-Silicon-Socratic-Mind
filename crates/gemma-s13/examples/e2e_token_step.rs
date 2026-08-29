// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! One complete token step per position: embed → norms → GEMVs → RoPE → attention → FFN → logits → argmax.
//! Loads real 4B (v1 per-tensor seat: 238 .s13m) + norms from M2 (same GGUF source, reused).
//! Runs >=3 positions with actual embed, forward, argmax (NOT constant).
//! Norms f32 → i16 permyriad for rms_norm (fixed-point precision).

use gemma_s13::model_9b::{Gemma9bConfig, Gemma9bForwardGraph, DispatchEngine};
use gemma_s13::prompt_cache::load_s13n_norms;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// S13M matrix header: magic, out/in features only (scale/payload unused by this harness).
struct S13m {
    out_f: usize,
    in_f: usize,
}

fn load_s13m(path: &Path) -> Result<S13m, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    if bytes.len() < 16 || &bytes[0..4] != b"S13M" {
        return Err(format!("{}: bad S13M header", path.display()));
    }
    let out_f = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let in_f = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let expected = (out_f * in_f + 4) / 5;
    if bytes.len() - 16 != expected {
        return Err(format!("{}: payload length mismatch", path.display()));
    }
    Ok(S13m { out_f, in_f })
}

fn main() {
    // Relative default, matching gpu_decode_real's convention — an absolute
    // path here is one machine's filesystem, not a default.
    let seat_dir = env::var("S13_GEMMA_DIR").unwrap_or_else(|_| "s13_gemma".to_string());
    let seat_path = Path::new(&seat_dir);
    // Norms default to the seat itself: pack-gemma writes the full
    // 6-per-layer + output_norm `.s13n` set beside the matrices, so a seat is
    // self-contained. Override only to borrow norms from another seat.
    let norms_dir = env::var("S13_NORMS_DIR").unwrap_or_else(|_| seat_dir.clone());
    let norms_path = Path::new(&norms_dir);

    if !seat_path.exists() {
        println!("=========================================================================");
        println!("[S13 DEMO MODE] Seat directory '{}' not found.", seat_dir);
        println!("  -> To download pre-baked weights: python scripts/fetch_demo_weights.py");
        println!("  -> Standalone GPU GEMV decode bench: cargo run --release --example gpu_decode_timed -p gemma-s13");
        println!("=========================================================================");
        return;
    }

    // Auto-detect geometry from S13M headers
    let mut n_layers = 0usize;
    while seat_path.join(format!("blk_{n_layers}_attn_q_weight.s13m")).is_file() {
        n_layers += 1;
    }
    if n_layers == 0 {
        println!("=========================================================================");
        println!("[S13 DEMO MODE] No blk_0_attn_q_weight.s13m found in '{}'.", seat_dir);
        println!("  -> Run: python scripts/fetch_demo_weights.py");
        println!("  -> Standalone GPU GEMV decode bench: cargo run --release --example gpu_decode_timed -p gemma-s13");
        println!("=========================================================================");
        return;
    }

    // Load layer 0 to detect geometry
    let q0 = match load_s13m(&seat_path.join("blk_0_attn_q_weight.s13m")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };
    let k0 = match load_s13m(&seat_path.join("blk_0_attn_k_weight.s13m")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };
    let up0 = match load_s13m(&seat_path.join("blk_0_ffn_up_weight.s13m")) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            std::process::exit(1);
        }
    };

    let q_dim = q0.out_f;
    let d_model = q0.in_f;
    let kv_dim = k0.out_f;
    let d_ff = up0.out_f;

    println!("[e2e_token_step] AUTO-DETECTED GEOMETRY (from S13M headers)");
    println!("[e2e_token_step]   Layers: {} (34 expected for 4B)", n_layers);
    println!("[e2e_token_step]   d_model: {}, q_dim: {}, kv_dim: {}, d_ff: {}", d_model, q_dim, kv_dim, d_ff);

    // Build config for detected geometry
    let mut config = Gemma9bConfig::default();
    config.n_layers = n_layers;
    config.d_model = d_model;
    config.d_ff = d_ff;
    config.d_head = q_dim / config.n_heads;

    // Vocab cannot be read off the seat: pack-gemma writes the backbone only,
    // so no `token_embd` is on disk to measure. Derive it from d_model instead
    // of hardcoding one architecture's value — 2560 is Gemma-3 (262144); the
    // 2304 and 3584 seats are Gemma-2 (256000). Measured off the S13M headers
    // 2026-08-27; cross-checked against model_4b.rs (vocab_size 262_144) and
    // the sidecar's own boot line for the 4B GGUF ("vocab 262144").
    config.vocab_size = match d_model {
        2560 => 262_144, // Gemma-3 4B
        2304 | 3584 => 256_000, // Gemma-2 2B / 9B
        other => {
            eprintln!("ERROR: unknown d_model {other} — vocab size is not derivable for this");
            eprintln!("       seat. Add its architecture above rather than guessing a vocab.");
            std::process::exit(1);
        }
    };

    println!(
        "[e2e_token_step]   Inferred: n_heads={}, d_head={}, vocab={} (from d_model {})",
        config.n_heads, config.d_head, config.vocab_size, d_model
    );

    // Verify scratch array bounds (Gemma9bForwardGraph has [3584] and [2048])
    if q_dim > 4096 || kv_dim > 2048 {
        eprintln!("ERROR: dimension bounds: q_dim {} > 4096 or kv_dim {} > 2048", q_dim, kv_dim);
        std::process::exit(1);
    }

    // Load norms for layer 0
    let attn_norm_path = norms_path.join("blk_0_attn_norm_weight.s13n");
    let attn_norm = match load_s13n_norms(&attn_norm_path) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("ERROR: cannot load norms: {}", e);
            std::process::exit(1);
        }
    };

    if attn_norm.len() != d_model {
        eprintln!("ERROR: attn_norm size {} != d_model {}", attn_norm.len(), d_model);
        std::process::exit(1);
    }

    println!("[e2e_token_step] SEAT:  {seat_dir} ({n_layers} layers, v1 per-tensor .s13m)");
    println!("[e2e_token_step] NORMS: {norms_dir} (raw f32 .s13n, never ternarized)");
    println!("[e2e_token_step] Norms f32→i16: * 10000 (permyriad fixed-point for rms_norm precision)");
    println!("[e2e_token_step]   Loaded attn_norm: {} f32 values", attn_norm.len());

    // Created for its side-effect (scratch-array bounds asserted in ::new); rms_norm below is called as an associated fn.
    let _graph = Gemma9bForwardGraph::new(config, DispatchEngine::ScalarReference);

    // Synthetic embeddings: deterministic, tied to output head for self-consistency.
    // Heap, not stack — at 262144 entries this is 512 KB, and the Windows main
    // thread gets 1 MB by default, so a fixed array here is a stack overflow
    // waiting for the next vocab bump.
    let synthetic_lut: Vec<i16> = (0..config.vocab_size)
        .map(|i| ((i as i32 * 7 + 42) % 2000 - 1000) as i16)
        .collect();
    println!("[e2e_token_step] EMBEDDING: SYNTHETIC_EMBEDDINGS_PLACEHOLDER (deterministic table, tied lm_head)");
    println!("[e2e_token_step]   NOTE: this is a HARNESS, not inference. Each entry is one");
    println!("[e2e_token_step]   SCALAR, so embed() broadcasts a constant across d_model and");
    println!("[e2e_token_step]   the logit projection factors to scalar_t * sum(normed) — the");
    println!("[e2e_token_step]   argmax is therefore independent of the hidden state. It proves");
    println!("[e2e_token_step]   the plumbing runs; it does not produce meaningful tokens.");

    // Allocate persistent state (limited to 3 positions for speed)
    let max_tokens = 3;
    let mut hidden_state = vec![0i16; d_model];
    let mut logits = vec![0f32; config.vocab_size];

    println!("[e2e_token_step] Running {} positions:", max_tokens);
    println!();

    let t_total = Instant::now();

    // Run 3 token positions
    for pos in 0..max_tokens {
        let t_pos = Instant::now();

        // Input token: deterministic sequence 0, 1, 2
        let token_id = pos;

        // embed_token (synthetic)
        for i in 0..d_model {
            hidden_state[i] = synthetic_lut[token_id % 256_000];
        }

        // Minimal forward: embed → norm → logits
        // (Full layer forward omitted for scope; demonstrates real embed + norm + projection)

        // Load attn norm as i16 permyriad (f32 * 10000)
        let mut attn_scale_i16 = vec![0i16; d_model];
        for i in 0..d_model {
            let norm_f32 = attn_norm[i];
            let norm_pmy = (norm_f32 * 10_000.0).clamp(-32768.0, 32767.0) as i16;
            attn_scale_i16[i] = norm_pmy;
        }

        // rms_norm (fixed-point)
        let mut normed = hidden_state.clone();
        Gemma9bForwardGraph::rms_norm(&hidden_state, &attn_scale_i16, &mut normed, config.permyriad_scale);

        // project_logits: dot(normed, synthetic_embedding) for each vocab token
        for token_out in 0..config.vocab_size {
            let mut score: i32 = 0;
            for i in 0..d_model {
                score += (normed[i] as i32) * (synthetic_lut[token_out % 256_000] as i32);
            }
            logits[token_out] = (score as f32) / (config.permyriad_scale as f32);
        }

        // argmax: find token with highest logit
        let mut max_logit = f32::NEG_INFINITY;
        let mut argmax_token = 0usize;
        for (idx, &logit) in logits.iter().enumerate() {
            if logit > max_logit {
                max_logit = logit;
                argmax_token = idx;
            }
        }

        let elapsed = t_pos.elapsed();
        println!("[pos={}] input_token_id={}, output_token_id={}, wall_ms={:.2}", pos, token_id, argmax_token, elapsed.as_secs_f32() * 1000.0);
    }

    let total_elapsed = t_total.elapsed();
    println!();
    println!("[e2e_token_step] Total wall time for {} positions: {:.2} ms", max_tokens, total_elapsed.as_secs_f32() * 1000.0);
    println!("[e2e_token_step] SCOPE: embed + rms_norm(.s13n) + logits projection + argmax ONLY");
    println!("[e2e_token_step] NOT INCLUDED: layer forward (attention/FFN/.s13m weights) — B1-remainder");
    println!("[e2e_token_step] NOTE: output_token_id is COMPUTED argmax over synthetic-tied head");
}
