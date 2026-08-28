//! Real cross-core cache contention (not a single-shot evict-then-read) for the
//! `TRIT_DIST_LUT`-vs-`TritCell5D::trits()` question. The single-threaded evictor
//! sweep in `forge-core-v3/examples/trit_dist_bench.rs` found nothing — the LUT
//! self-rewarms within a handful of iterations once a loop starts hammering it,
//! so a one-time flush before the loop measures almost nothing. This is the real
//! test: a producer thread continuously publishing through a live `TripleBuffer`
//! (this crate's own `triple_buffer.rs`) while the LUT/decode measurement runs
//! concurrently on the main thread — genuine MESI cache-line traffic between
//! cores, not a simulated cold start.
//!
//! Grounded in the actual v2 ancestor, read before writing this (C06): `F:\NewRepo\
//! crates\forge-studio\src\triple_loop.rs` — `TripleLoop` is a real 3-thread
//! architecture (T1 logic / T2 rasterizer / T3 presentation), and `OverlayBridge`
//! (`triple_loop.rs:140-158`) is exactly `TripleBuffer<SizedPlane>` — an RGBA pixel
//! buffer, not an abstract payload. This example models T2's real workload: publish
//! a freshly-written RGBA plane every iteration, same shape `OverlayBridge` does.
//!
//! **Aperture (stated, not measured):** the payload size below (1920x1080x4 =
//! ~8MB) is an assumed representative overlay size, not read from a specific v3
//! window-size constant — none was found to cite. If the real number matters,
//! it needs a v3 receipt, not this comment.
//!
//! `cargo run --release --example trit_dist_contention -p forge-hal-clockspine`

use forge_core_v3::atom::TritCell5D;
use forge_core_v3::metarouter::TRIT_DIST_LUT;
use forge_core_v3::seed::Mulberry32;
use forge_hal_clockspine::triple_buffer::TripleBuffer;
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const OVERLAY_W: usize = 1920;
const OVERLAY_H: usize = 1080;
const OVERLAY_BYTES: usize = OVERLAY_W * OVERLAY_H * 4; // RGBA, matches SizedPlane's shape

#[inline]
fn dist_by_decode(c: u8, q: u8) -> u32 {
    let dc = TritCell5D(c).trits().unwrap_or([0i8; 5]);
    let dq = TritCell5D(q).trits().unwrap_or([0i8; 5]);
    let mut dist: u32 = 0;
    for k in 0..5 {
        let d = dc[k] - dq[k];
        dist += d.unsigned_abs() as u32;
    }
    dist
}

#[inline]
fn dist_by_lut(c: u8, q: u8) -> u32 {
    TRIT_DIST_LUT[((c as usize) << 8) | q as usize] as u32
}

fn random_pairs(n: usize, seed: u64) -> Vec<(u8, u8)> {
    let mut rng = Mulberry32::new(seed);
    (0..n).map(|_| (rng.below(243) as u8, rng.below(243) as u8)).collect()
}

/// T2's real workload, shape-matched to `OverlayBridge::publish` (`triple_loop.rs:155-158`):
/// write fresh bytes into the recycled plane (real memory writes, not a pointer swap),
/// then publish. Runs until `stop` is set.
fn run_overlay_producer(bridge: Arc<TripleBuffer<Vec<u8>>>, stop: Arc<AtomicBool>) {
    let mut frame = 0u8;
    let mut spare = vec![0u8; OVERLAY_BYTES];
    while !stop.load(Ordering::Relaxed) {
        frame = frame.wrapping_add(1);
        // Real writes across the whole plane — this is the actual cache traffic,
        // not the publish call itself (which is just a mem::swap).
        for byte in spare.iter_mut() {
            *byte = frame;
        }
        spare = bridge.publish(spare);
    }
}

fn bench_under_contention(label: &str, n: usize, pairs: &[(u8, u8)]) {
    const REPS: u32 = 8;

    let bridge = Arc::new(TripleBuffer::new(vec![0u8; OVERLAY_BYTES]));
    let stop = Arc::new(AtomicBool::new(false));
    let producer = {
        let bridge = Arc::clone(&bridge);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || run_overlay_producer(bridge, stop))
    };

    // Let the producer actually get running before measuring.
    std::thread::sleep(std::time::Duration::from_millis(5));

    let t0 = Instant::now();
    let mut lut_sum = 0u64;
    for _ in 0..REPS {
        for &(c, q) in pairs {
            lut_sum += black_box(dist_by_lut(black_box(c), black_box(q))) as u64;
        }
    }
    let lut_elapsed = t0.elapsed();

    let t1 = Instant::now();
    let mut dec_sum = 0u64;
    for _ in 0..REPS {
        for &(c, q) in pairs {
            dec_sum += black_box(dist_by_decode(black_box(c), black_box(q))) as u64;
        }
    }
    let dec_elapsed = t1.elapsed();

    stop.store(true, Ordering::Relaxed);
    producer.join().expect("producer thread must not panic");

    assert_eq!(lut_sum, dec_sum, "LUT and decode must agree even under contention");

    let per_op = |d: std::time::Duration| d.as_nanos() as f64 / (n as u128 * REPS as u128) as f64;
    println!(
        "{label:>10}  n={n:>8}  UNDER-CONTENTION  LUT={:>7.3} ns/op  DECODE={:>7.3} ns/op  ratio={:>5.2}",
        per_op(lut_elapsed), per_op(dec_elapsed), per_op(dec_elapsed) / per_op(lut_elapsed)
    );
}

fn bench_baseline(label: &str, n: usize, pairs: &[(u8, u8)]) {
    const REPS: u32 = 8;
    let t0 = Instant::now();
    let mut lut_sum = 0u64;
    for _ in 0..REPS {
        for &(c, q) in pairs {
            lut_sum += black_box(dist_by_lut(black_box(c), black_box(q))) as u64;
        }
    }
    let lut_elapsed = t0.elapsed();

    let t1 = Instant::now();
    let mut dec_sum = 0u64;
    for _ in 0..REPS {
        for &(c, q) in pairs {
            dec_sum += black_box(dist_by_decode(black_box(c), black_box(q))) as u64;
        }
    }
    let dec_elapsed = t1.elapsed();
    assert_eq!(lut_sum, dec_sum);

    let per_op = |d: std::time::Duration| d.as_nanos() as f64 / (n as u128 * REPS as u128) as f64;
    println!(
        "{label:>10}  n={n:>8}  BASELINE          LUT={:>7.3} ns/op  DECODE={:>7.3} ns/op  ratio={:>5.2}",
        per_op(lut_elapsed), per_op(dec_elapsed), per_op(dec_elapsed) / per_op(lut_elapsed)
    );
}

fn main() {
    println!(
        "Overlay producer publishes a {}x{} RGBA plane ({}MB) continuously via a real \
         TripleBuffer, on its own OS thread, while the main thread measures LUT vs decode.\n",
        OVERLAY_W, OVERLAY_H, OVERLAY_BYTES / (1024 * 1024)
    );

    let sizes: &[(&str, usize)] = &[
        ("tiny", 4_096),
        ("l1-edge", 16_384),
        ("over-lut", 40_000),
        ("l2-ish", 131_072),
        ("large", 1_048_576),
    ];

    for (label, n) in sizes {
        let pairs = random_pairs(*n, 0xC0FFEE ^ *n as u64);
        bench_baseline(label, *n, &pairs);
        bench_under_contention(label, *n, &pairs);
        println!();
    }
}
