//! Micro-Expert & Trit-MoE CPU Throughput Benchmark
//!
//! EVERY BLOCK BELOW IS PURE CPU. No CUDA, no SPIR-V, no device submission, no
//! GPU is touched or required. Block names describe the workload being timed,
//! not hardware — read them literally:
//!
//! 1. L1D-resident: 512-bit BQ MetaRouter hamming ROUTING decisions (tokens
//!    routed/s — routing throughput, NOT token generation or inference)
//! 2. L2-resident: 400x400 conjugate triad grid sign inversion (trits/s)
//! 3. Host staging: 2 x 64 KB heap buffer pair swapped behind a timeline
//!    semaphore (swaps/s & MB/s — this is memcpy between two `Box`ed arrays)
//! 4. Tile geometry: integer dispatch-grid planning arithmetic (plans/s — the
//!    planner models an Ampere 32x32 tile contract; it dispatches nothing)
//!
//! Run with: `cargo run --release --example mtok_throughput_bench -p forge-gpu-warden-v3`

use forge_envelope::s13::ConjugateTriadGrid400;
use forge_gpu_warden_v3::fence::TimelineSemaphore;
use forge_gpu_warden_v3::vram_staging::DoubleBufferedStagingBuffers;
use forge_gpu_warden_v3::workgroup::WorkgroupTileContract;
use forge_ml_bqrouter::{BqCentroid, BqRouter, BQ_BYTES, NUM_SPECIALISTS};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("===============================================================================");
    println!("        13FORGE CPU THROUGHPUT BENCHMARK (no GPU involved)");
    println!("===============================================================================\n");

    // ------------------------------------------------------------------------
    // Benchmark 1: L1D Cache BQ MetaRouter Throughput (512-bit / 64-byte Tokens)
    // ------------------------------------------------------------------------
    println!("--- [1] L1D-resident: 512-bit BQ MetaRouter hamming ROUTING (7 specialists) ---");
    println!("        one \"token\" = one 512-element i8 query vector routed to an expert;");
    println!("        this measures routing decisions/s, NOT token generation.");
    let mut router = BqRouter::new(512);
    for sid in 0..NUM_SPECIALISTS {
        let mut bits = [0u8; BQ_BYTES];
        for (i, b) in bits.iter_mut().enumerate() {
            *b = ((sid * 41 + i * 17) % 256) as u8;
        }
        router.centroids[sid] = BqCentroid {
            bits,
            record_count: 50,
            positive_count: 45,
            active: true,
        };
    }

    // Generate 50,000 query tokens (i8 vectors)
    let num_tokens = 50_000usize;
    let mut queries = Vec::with_capacity(num_tokens);
    for t in 0..num_tokens {
        let mut q = vec![0i8; 512];
        for (i, v) in q.iter_mut().enumerate() {
            *v = (((t * 31 + i * 7) % 255) as i16 - 128) as i8;
        }
        queries.push(q);
    }

    // Warm-up pass
    for q in queries.iter().take(100) {
        black_box(router.route(q));
    }

    let iters = 10usize;
    let total_evals = num_tokens * iters;
    let t0 = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..iters {
        for q in &queries {
            if let Some((id, margin)) = router.route(q) {
                checksum += id as u64 + margin as u64;
            }
        }
    }
    let elapsed = t0.elapsed();
    let mtoks_single_core = (total_evals as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
    black_box(checksum);

    println!("  Routed vectors:    {} vectors ({} iterations)", num_tokens, iters);
    println!("  Total time:        {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Single-core latency: {:.2} ns/routing decision", elapsed.as_nanos() as f64 / total_evals as f64);
    println!("  Single-core rate:    \x1b[1;32m{:.2} M routings/s\x1b[0m", mtoks_single_core);
    println!("  (single core, measured. No multi-core figure is reported here:");
    println!("   multiplying one core by a scaling factor is an estimate, not a");
    println!("   measurement. Measure N threads if an N-core number is needed.)\n");

    // ------------------------------------------------------------------------
    // Benchmark 2: L2 Cache 400x400 Conjugate Triad Grid & Resolvent (160 KB)
    // ------------------------------------------------------------------------
    println!("--- [2] L2-resident: 400x400 conjugate triad grid SIGN INVERSION (160 KB) ---");
    let mut grid = ConjugateTriadGrid400::new();
    for y in 0..400 {
        for x in 0..400 {
            let trit = match (x + y * 3) % 3 {
                0 => -1,
                1 => 0,
                _ => 1,
            };
            grid.set(x, y, trit);
        }
    }

    let grid_involutions = 10_000usize;

    // Warm-up: fault in the pages and settle i-cache/branch state before timing.
    for _ in 0..200 {
        black_box(black_box(&grid).invert());
    }

    let t1 = Instant::now();
    let mut inv_checksum = 0i64;
    for _ in 0..grid_involutions {
        let inverted = black_box(&grid).invert();
        inv_checksum += inverted.get(0, 0).unwrap_or(0) as i64;
    }
    let elapsed_inv = t1.elapsed();
    black_box(inv_checksum);
    let trits_per_grid = 400 * 400;
    let total_trits = (grid_involutions * trits_per_grid) as f64;
    let mtrits_sec = (total_trits / elapsed_inv.as_secs_f64()) / 1_000_000.0;

    println!("  Grid size:         400 x 400 = 160,000 trits (160 KB L2 resident)");
    println!("  Involution passes: {}", grid_involutions);
    println!("  Involution time:   {:.3} ms ({:.2} µs/pass)", elapsed_inv.as_secs_f64() * 1000.0, elapsed_inv.as_micros() as f64 / grid_involutions as f64);
    println!("  L2 trit throughput: \x1b[1;32m{:.2} Mtrits/s\x1b[0m\n", mtrits_sec);

    // ------------------------------------------------------------------------
    // Benchmark 3: host-side double-buffered staging pair + timeline semaphore.
    // Both buffers are heap allocations (`Box<[u8; STAGING_SLOT_SIZE]>`); the
    // swap is a memcpy plus an atomic index flip. No device memory, no PCIe.
    // ------------------------------------------------------------------------
    println!("--- [3] Host staging: 2 x 64 KB heap buffer pair, timeline-gated swap ---");
    println!("        buffers are heap-allocated; this is memcpy, not device transfer.");
    let sem = Arc::new(TimelineSemaphore::new(0));
    let mut staging = DoubleBufferedStagingBuffers::new(sem.clone());
    let micro_expert_weights = vec![0x5A; 1078]; // 768-dim S13 centroids = 1,078 bytes

    let num_swaps = 20_000usize;
    let warm_swaps = 500usize;

    // Warm-up: fault in both staging slots and settle the allocator. Timeline
    // points must stay monotonic, so the measured run continues the counter
    // rather than restarting it.
    for step in 1..=warm_swaps {
        let pt = step as u64;
        staging.stage_weights(step as u32, &micro_expert_weights, pt).unwrap();
        sem.signal(pt).unwrap();
        assert!(staging.try_swap().unwrap());
    }

    let t2 = Instant::now();
    for step in (warm_swaps + 1)..=(warm_swaps + num_swaps) {
        let pt = step as u64;
        staging.stage_weights(step as u32, &micro_expert_weights, pt).unwrap();
        sem.signal(pt).unwrap();
        assert!(staging.try_swap().unwrap());
    }
    let elapsed_swaps = t2.elapsed();
    let swaps_per_sec = num_swaps as f64 / elapsed_swaps.as_secs_f64();
    let staging_bw_mb = (num_swaps as f64 * micro_expert_weights.len() as f64) / (elapsed_swaps.as_secs_f64() * 1024.0 * 1024.0);

    println!("  Staged payload:    {} B per micro-expert", micro_expert_weights.len());
    println!("  Total swaps:       {} zero-stall buffer swaps", num_swaps);
    println!("  Swap rate:         \x1b[1;32m{:.2} swaps/sec\x1b[0m ({:.2} ns/swap)", swaps_per_sec, elapsed_swaps.as_nanos() as f64 / num_swaps as f64);
    println!("  Host memcpy rate:  \x1b[1;32m{:.2} MB/s\x1b[0m (heap-to-heap)\n", staging_bw_mb);

    // ------------------------------------------------------------------------
    // Benchmark 4: tile geometry planning. `plan_dispatch` is integer ceiling
    // division over an Ampere 32x32 tile contract. It computes a grid shape;
    // it does not compile, submit, or execute anything on a device.
    // ------------------------------------------------------------------------
    println!("--- [4] Tile geometry: integer dispatch-grid planning (Ampere 32x32 contract) ---");
    println!("        pure integer arithmetic; nothing is dispatched to a device.");
    let contract = WorkgroupTileContract::ampere_32x32();
    let num_plans = 100_000usize;

    // `m`/`n` derive from the loop counter and `contract` is const-constructible,
    // so without an opaque barrier on the INPUTS LLVM is free to const-fold or
    // unroll this loop away — a `black_box` on the output alone only prevents
    // dead-code elimination. Guard both sides, and warm up first.
    for i in 0..2_000usize {
        let m = black_box(((i % 1024) + 32) as u32);
        let n = black_box((((i * 7) % 1024) + 32) as u32);
        black_box(black_box(&contract).plan_dispatch(m, n));
    }

    let t3 = Instant::now();
    let mut total_threads = 0u64;
    for i in 0..num_plans {
        let m = black_box(((i % 1024) + 32) as u32);
        let n = black_box((((i * 7) % 1024) + 32) as u32);
        let plan = black_box(&contract).plan_dispatch(m, n);
        total_threads += plan.total_threads;
    }
    let elapsed_plans = t3.elapsed();
    black_box(total_threads);
    let plans_per_sec = num_plans as f64 / elapsed_plans.as_secs_f64();

    println!(
        "  Tile contract:     {}x{} ({} threads/tile, warp size {})",
        contract.tile_dim_x,
        contract.tile_dim_y,
        contract.tile_dim_x * contract.tile_dim_y,
        contract.warp_size
    );
    println!(
        "  Shared mem stride: {} elements ({} bytes, bank-conflict-free)",
        contract.shared_mem_stride_elements, contract.shared_mem_stride_bytes
    );
    println!("  Plans computed:    \x1b[1;32m{:.2} Mplans/sec\x1b[0m ({:.2} ns/plan)\n", plans_per_sec / 1_000_000.0, elapsed_plans.as_nanos() as f64 / num_plans as f64);

    println!("===============================================================================");
    println!("                          ALL CPU BENCHMARKS GREEN");
    println!("        (no GPU was used; see the module header for what each block times)");
    println!("===============================================================================");
}
