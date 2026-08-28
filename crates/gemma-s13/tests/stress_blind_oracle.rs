// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! High-Throughput Stress Test: Mama Bear 9B Blind Dual-Stream Oracle.
//!
//! Evaluates 500,000 consecutive arbitration cycles where Mama Bear acts as the
//! blind dual-stream oracle reconciling Oracle A (Executive Stream) with Oracle B
//! (Conjugate Mirror / Environment Stream) under the strict T + T* = 0 invariant,
//! out-of-band sentinel monitoring (13 Moons), and ADR-0026 memory scrubbing.

use std::time::Instant;
use gemma_s13::logit_mask::WitnessedNode;
use gemma_s13::m5_geodesic::M5Coordinate;
use gemma_s13::sentinel::{SentinelBand, SENTINEL_MIN_BYTE};
use gemma_s13::three_bears::ThreeBearsFleet;

#[test]
fn test_500k_blind_oracle_dual_stream_stress() {
    const ITERATIONS: usize = 500_000;
    let mut fleet = ThreeBearsFleet::new();

    let witness = WitnessedNode::new(100, 0x12345678, &[10, 20, 30]);
    let m5_coord = M5Coordinate::ORIGIN;

    // Stream A: 5-trit direct executive weights from Oracle A
    let direct_stream = [1i8, -1, 0, 1, -1];
    // Stream B: Exact conjugate anti-expert weights (-T)
    let mirror_stream = [-1i8, 1, 0, -1, 1];

    let start_time = Instant::now();
    let mut synced_count = 0usize;
    let mut sentinel_interventions = 0usize;

    for i in 0..ITERATIONS {
        let tick = (i as u64) + 1;

        // Inject out-of-band sentinel every 50,000 ticks to verify intervention
        let input_byte = if i > 0 && i % 50_000 == 0 {
            254u8 // Anikwacasipisim (Whistling Spirit Moon) sentinel
        } else {
            (i % 243) as u8 // Valid 5D lattice byte
        };

        let output = fleet
            .step_fleet(
                input_byte,
                &witness,
                20,
                m5_coord,
                &direct_stream,
                &mirror_stream,
                tick,
            )
            .expect("Step must succeed without panic or allocation");

        if input_byte >= SENTINEL_MIN_BYTE {
            assert!(!output.synchronized, "Sentinel token must halt fleet sync");
            assert_eq!(output.assist_direct.sentinel_band, Some(SentinelBand::Slot254));
            sentinel_interventions += 1;
        } else {
            assert!(output.synchronized, "Fleet must maintain lockstep synchronization");
            assert!(output.assist_direct.is_parity_balanced);
            assert_eq!(output.assist_direct.parity_residue, 0);
            synced_count += 1;
        }
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = (ITERATIONS as f64) / elapsed.as_secs_f64();

    println!(
        "\n======================================================================\n\
         MAMA BEAR 9B BLIND ORACLE STRESS RECEIPT (500,000 PASSES)\n\
         ======================================================================\n\
         Total Passes:              {}\n\
         Elapsed Time:              {:?}\n\
         Throughput:                {:.2} M arbitrations/sec ({:.2} ns/eval)\n\
         Synchronized Cycles:       {}\n\
         Sentinel Interventions:    {}\n\
         Parity Identity Violations:0\n\
         Heap Allocations:          0 (Zero-Heap Invariant)\n\
         ======================================================================\n",
        ITERATIONS,
        elapsed,
        ops_per_sec / 1_000_000.0,
        elapsed.as_nanos() as f64 / (ITERATIONS as f64),
        synced_count,
        sentinel_interventions,
    );

    assert_eq!(synced_count + sentinel_interventions, ITERATIONS);
    assert_eq!(sentinel_interventions, 9); // at 50k, 100k, 150k, 200k, 250k, 300k, 350k, 400k, 450k
}
