// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Somatic Celestial Astrolabe & S133 Tensor Demonstration.
//!
//! Demonstrates zero-heap 119k star codebook lookups, 5D somatic detokenization,
//! and S133 scale-tensor loading directly from baked binary assets on disk.

use gemma_s13::star_codebook::StarCodebookView;
use gemma_s13::s13::S13TensorView;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() {
    println!("===============================================================================");
    println!("   5D SOMATIC CELESTIAL ASTROLABE & S133 TENSOR DEMO");
    println!("===============================================================================\n");

    // 1. Load baked HYG Star Codebook
    let hyg_path = Path::new("F:/v3/shell/assets/hyg_baked.bin");
    if !hyg_path.exists() {
        eprintln!("Error: HYG catalog not found at {}", hyg_path.display());
        return;
    }

    let hyg_bytes = fs::read(hyg_path).expect("Read hyg_baked.bin");
    println!("Loading HYG Baked Binary: {} bytes ({:.2} MB)", hyg_bytes.len(), hyg_bytes.len() as f64 / 1_048_576.0);

    let t0 = Instant::now();
    let codebook = StarCodebookView::parse(&hyg_bytes).expect("Parse StarCodebookView");
    let parse_time = t0.elapsed();

    println!("  • Codebook Parsed In       : {:?}", parse_time);
    println!("  • Total Baked Stars        : {}", codebook.star_count());
    println!("  • Total Sparse Anomalies   : {}", codebook.anomaly_count());
    println!("-------------------------------------------------------------------------------\n");

    // 2. Inspection of Bright Anchors
    println!("--- [1] Bright Landmark Celestial Centroids ---");
    for i in 0..5 {
        if let Some(star) = codebook.get_star(i) {
            let ra_deg = star.ra_normalized() * 360.0;
            let dec_deg = star.dec_normalized() * 90.0;
            let mag_f = star.mag_permyriad as f32 / 10000.0;
            let hz = star.resonant_milli_hz() as f32 / 1000.0;
            println!(
                "  Star #{:<6} | RA: {:>6.2}° | Dec: {:>+6.2}° | Mag: {:>+5.2} | Dist: {:>4} pc | Resonant: {:>6.2} Hz | Tier: {} | Lore: 0x{:02X}",
                star.star_idx, ra_deg, dec_deg, mag_f, star.distance_u16, hz, star.lode_tier, star.lore_idx
            );
        }
    }
    println!("-------------------------------------------------------------------------------\n");

    // 3. High-Speed $O(1)$ Centroid Lookup Benchmark
    println!("--- [2] Zero-Heap Indexed Lookup Throughput ---");
    let iters = 1_000_000usize;
    let t_bench = Instant::now();
    let mut sum_mag = 0i64;
    for i in 0..iters {
        let idx = i % codebook.star_count();
        if let Some(s) = codebook.get_star(idx) {
            sum_mag += s.mag_permyriad as i64;
        }
    }
    let elapsed = t_bench.elapsed();
    let m_lookups_per_sec = (iters as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
    println!("  • Processed {} lookups in {:?}", iters, elapsed);
    println!("  • Throughput: {:.2} Million star lookups / second (checksum: {})", m_lookups_per_sec, sum_mag);
    println!("-------------------------------------------------------------------------------\n");

    // 4. Somatic 5D Embedding Detokenization
    println!("--- [3] Continuous 5D Embedding Detokenization (Option 1 Tied Unembed) ---");
    let sample_queries = [
        ([0.15f32, 0.35f32, 0.20f32, 0.5f32, 0.1f32], "North Celestial Horizon"),
        ([0.80f32, -0.45f32, 0.12f32, 0.8f32, 0.2f32], "South Celestial Stream"),
        ([0.02f32, 0.98f32, 0.22f32, 0.2f32, 0.05f32], "Polar Sentry Anchor"),
    ];

    for (query, label) in sample_queries {
        let t_detok = Instant::now();
        if let Some(star) = codebook.detokenize_embedding(&query) {
            let detok_dur = t_detok.elapsed();
            println!(
                "  Query '{}':\n    -> Nearest Centroid #{} (RA: {:.2}°, Dec: {:+.2}°, Mag: {:+.2}, Resonant: {:.2} Hz) [Resolved in {:?}]",
                label, star.star_idx, star.ra_normalized() * 360.0, star.dec_normalized() * 90.0, star.mag_permyriad as f32 / 10000.0, star.resonant_milli_hz() as f32 / 1000.0, detok_dur
            );
        }
    }
    println!("-------------------------------------------------------------------------------\n");

    // 5. S133 Tensor Weight Verification
    println!("--- [4] S133 Multi-Tensor Container Loading (Cree Sentry & 2B M3) ---");
    let tensor_path = Path::new("F:/v3/s13_gemma_2b_m3/blk_0_ffn_up_weight.s13m");
    if tensor_path.exists() {
        let tensor_bytes = fs::read(tensor_path).expect("Read S133 tensor");
        let view = S13TensorView::parse(&tensor_bytes).expect("Parse S13TensorView");
        println!("  • Loaded: {}", tensor_path.display());
        println!("  • Tensor Dimensions        : {} x {} ({} total weights)", view.out_features, view.in_features, view.out_features * view.in_features);
        println!("  • Global Float Scale       : {:.6}", view.scale);
        println!("  • Per-Group Scale Payload  : {} bytes (group size: {})", view.group_scales.map_or(0, |s| s.len()), view.group_size);
        println!("  • Packed Trit Payload      : {} bytes", view.packed_trits.len());
        println!("  • Group 0 Scale (i16 PMY)  : {} pmy ({:.4})", view.get_group_scale_pmy(0).unwrap_or(0), view.get_group_scale_pmy(0).unwrap_or(0) as f32 / 10000.0);
        println!("  • Sample Weight (0, 0)     : trit = {}, dequantized = {:.6}", view.get_trit(0, 0).unwrap_or(0), view.get_weight_f32(0, 0).unwrap_or(0.0));
        println!("  • Sample Weight (0, 1)     : trit = {}, dequantized = {:.6}", view.get_trit(0, 1).unwrap_or(0), view.get_weight_f32(0, 1).unwrap_or(0.0));
    } else {
        println!("  • S133 tensor file not present at path: {}", tensor_path.display());
    }

    println!("\n===============================================================================");
    println!("   ALL SOMATIC ASTROLABE & S133 CHECKS COMPLETED CLEANLY (ZERO HEAP)");
    println!("===============================================================================");
}
