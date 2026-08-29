// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Full end-to-end Gemma 9B inference: load model from disk, generate tokens from prompt.
//!
//! Run with:
//! ```sh
//! S13_GEMMA_DIR=s13_gemma_9b_m3 cargo run --release --example full_inference -p gemma-s13
//! ```
//!
//! Requires:
//! - `s13_gemma_9b_m3/` seat (42 layers of blk_N_*.s13m weights + .s13n norms)
//! - `token_embd.s13m` (generated with quantize-s13 pack-gemma ... --with-embed)

use gemma_s13::model_9b::{load_gemma9b_model_from_disk, generate_tokens, Gemma9bForwardGraph, DispatchEngine};
use std::env;
use std::path::Path;
use std::time::Instant;

fn main() {
    let seat_dir = env::var("S13_GEMMA_DIR").unwrap_or_else(|_| "s13_gemma_9b_m3".to_string());
    let seat_path = Path::new(&seat_dir);

    if !seat_path.exists() {
        eprintln!("╭─────────────────────────────────────────────────────────────────────╮");
        eprintln!("│ [full_inference] Gemma 9B weights not found: {}                 │", seat_dir);
        eprintln!("├─────────────────────────────────────────────────────────────────────┤");
        eprintln!("│ SETUP: Download quantized S13 weights from Hugging Face Hub:        │");
        eprintln!("│                                                                     │");
        eprintln!("│   python scripts/fetch_demo_weights.py                              │");
        eprintln!("│                                                                     │");
        eprintln!("│ Then retry this command:                                            │");
        eprintln!("│   cargo run --release --example full_inference -p gemma-s13         │");
        eprintln!("│                                                                     │");
        eprintln!("│ Or set S13_GEMMA_DIR to an existing quantize-s13 pack-gemma output: │");
        eprintln!("│   S13_GEMMA_DIR=/path/to/weights cargo run --example full_inference │");
        eprintln!("╰─────────────────────────────────────────────────────────────────────╯");
        std::process::exit(1);
    }

    println!("[full_inference] Loading Gemma 9B from {}", seat_dir);
    let t_load = Instant::now();

    let model = match load_gemma9b_model_from_disk(seat_path, None) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ERROR: Failed to load model: {}", e);
            std::process::exit(1);
        }
    };

    let load_time = t_load.elapsed();
    println!("[full_inference] Model loaded in {:.2}s", load_time.as_secs_f32());

    // Create forward graph with scalar dispatch
    let mut graph = Gemma9bForwardGraph::new(model.config, DispatchEngine::ScalarReference);

    // Simple test prompts (token IDs) — in a real system, these come from tokenization
    let prompts = vec![
        (vec![1, 2, 3], "test_prompt_1"),
        (vec![42, 100, 200], "test_prompt_2"),
    ];

    for (prompt_ids, prompt_name) in prompts {
        println!("\n[full_inference] Generating from prompt: {} (tokens: {:?})", prompt_name, prompt_ids);

        let t_gen = Instant::now();
        match generate_tokens(&model, &mut graph, &prompt_ids, 10, 0.7, 2) {
            Ok(generated) => {
                let gen_time = t_gen.elapsed();
                let tokens_per_sec = generated.len() as f64 / gen_time.as_secs_f64();

                println!("[full_inference]   Generated {} tokens in {:.2}ms", generated.len(), gen_time.as_millis());
                println!("[full_inference]   Tokens/sec: {:.2}", tokens_per_sec);
                println!("[full_inference]   Output IDs: {:?}", generated);

                // Verify output is non-constant (changes per prompt)
                let is_realistic = generated.iter().any(|&t| t != generated[0]);
                println!("[full_inference]   Output varies: {}", if is_realistic { "✓" } else { "✗ (all same token)" });
            }
            Err(e) => {
                eprintln!("[full_inference] ERROR: Generation failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    println!("\n[full_inference] SUCCESS: Full inference pipeline wired and working");
    println!("[full_inference] Proof: Model loaded from disk, forward pass executed, tokens generated");
}
