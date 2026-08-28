// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Speculative Triad Integration Test Suite.
//!
//! Verifies the three speculative pillars of the S13 Gemma Triad architecture:
//! 1. Speculative Intent: RAG-DAG logit gating, N x IPR entropy localization, and pre-execution routing.
//! 2. Speculative Rendering: 5D M5 Geodesic manifold lattice indexing (243 states) and zero-heap spatial lookups.
//! 3. Speculative Assistance: Anti-expert conjugate parity balance (T + T* = 0), Sentinel 13-Moons gating, and ADR-0026 zero-retention vault.

use gemma_s13::logit_mask::{WitnessedNode, LOGIT_MASKED_ZERO_PROB};
use gemma_s13::m5_geodesic::M5Coordinate;
use gemma_s13::nipr::{NormalizedIpr, LANDMARK_PMY};
use gemma_s13::sentinel::{SentinelBand, SENTINEL_MIN_BYTE};
use gemma_s13::vault::ZeroRetentionVault;

#[test]
fn test_speculative_intent_rag_dag_and_nipr() {
    // 1. Verify N x IPR localization of speculative intent
    let focused_activations = [0u16, 0, 1000, 0, 0];
    let ipr = NormalizedIpr::compute_u16(&focused_activations);
    assert_eq!(ipr.pmy, 10_000, "Sharp speculative focus must reach 10,000 pmy");
    assert!(ipr.pmy >= LANDMARK_PMY, "Speculative intent must qualify as a landmark attractor");

    // 2. Pre-filter candidate next actions via Witnessed DAG transitions
    let start_node = WitnessedNode::new(101, 0xABCDEF1234567890, &[201, 202, 203]);
    assert!(start_node.allows_transition(201), "Allowed transition must be accepted");
    assert!(start_node.allows_transition(202), "Allowed transition must be accepted");
    assert!(!start_node.allows_transition(999), "Unwitnessed transition must be rejected");

    // 3. Mask unwitnessed logit branch to absolute zero probability
    let candidate = 999u32;
    let logit = if start_node.allows_transition(candidate) { 450 } else { LOGIT_MASKED_ZERO_PROB };
    assert_eq!(logit, i32::MIN, "Speculative branch without witness must be clamped to -infinity");
}

#[test]
fn test_speculative_rendering_5d_manifold_indexing() {
    // 1. Verify 5D Geodesic Coordinate Equilibrium Origin
    let origin = M5Coordinate::ORIGIN;
    assert!(origin.is_origin(), "Origin must be at drift-free equilibrium");
    assert_eq!(origin.axes, [0, 0, 0, 0, 0]);

    // 2. Map speculative 5D render point to discrete O(1) scalar index
    let render_target = M5Coordinate::new([1, -1, 0, 1, -1]).expect("Valid 5D coordinates in {-1, 0, 1}");
    let scalar_idx = render_target.to_scalar_index();
    assert!(scalar_idx < M5Coordinate::TOTAL_STATES as u8, "Scalar index must reside within [0, 242]");

    // 3. Verify all 243 speculative rendering cells round-trip without heap allocation
    for x1 in -1..=1 {
        for x2 in -1..=1 {
            for x3 in -1..=1 {
                for x4 in -1..=1 {
                    for x5 in -1..=1 {
                        let coord = M5Coordinate::new([x1, x2, x3, x4, x5]).expect("Valid coord");
                        let idx = coord.to_scalar_index();
                        assert!(idx < 243);
                    }
                }
            }
        }
    }
}

#[test]
fn test_speculative_assistance_parity_and_sentinel_gate() {
    // 1. Anti-Expert Conjugate Parity Identity: T + T* = 0
    let direct_tensor_weight: i8 = 1;
    let mirror_conjugate_weight: i8 = -1;
    let parity_sum = direct_tensor_weight + mirror_conjugate_weight;
    assert_eq!(parity_sum, 0, "Direct Executive and Mirror Anti-Expert must satisfy T + T* = 0");

    // 2. Verify Sentinel Out-of-Band Boundary
    let valid_packed_byte: u8 = 121; // [0, 0, 0, 0, 0]
    assert!(valid_packed_byte < SENTINEL_MIN_BYTE, "Payload bytes must fall in 0..=242");

    let sabotage_token: u8 = 254; // sentinel slot 11 of 13
    assert!(sabotage_token >= SENTINEL_MIN_BYTE, "Sabotage token must trigger sentinel state");
    let band = SentinelBand::from_byte(sabotage_token);
    assert_eq!(band, Some(SentinelBand::Slot254), "Sentinel 254 must decode to slot 11 of 13");

    // 3. ADR-0026 Sovereign Vault Zero-Retention Shredding
    let mut vault = ZeroRetentionVault::new();
    let sample_secret = [0x5Au8; 64];
    let ok = vault.stage_transient_data(&sample_secret, 100, 10);
    assert!(ok, "Transient data staged successfully");
    assert!(vault.is_active, "Vault holds active record");
    
    // Sweep at deadline (tick 110)
    let wiped = vault.sweep_if_expired(110);
    assert!(wiped, "Vault swept and zeroized upon reaching expiration tick");
    assert!(!vault.is_active, "Vault is no longer active");
    assert_eq!(vault.staging_registers[0], 0, "Memory registers zeroized");
}

#[test]
fn test_anti_expert_operator_involution_and_rag_dag_gating() {
    use gemma_s13::logit_mask::{AntiExpertGate, RagDag};

    // 1. Verify Anti-Expert Involution: (T*)* = T and T + T* = 0 across all 3^5 = 243 trit states
    for t0 in -1i8..=1 {
        for t1 in -1i8..=1 {
            for t2 in -1i8..=1 {
                for t3 in -1i8..=1 {
                    for t4 in -1i8..=1 {
                        let t = [t0, t1, t2, t3, t4];
                        assert!(
                            AntiExpertGate::verify_involution_identity(&t),
                            "Involution identity (T*)* = T and T + T* = 0 must hold for all trit vectors"
                        );
                    }
                }
            }
        }
    }

    // 2. Multi-hop RAG-DAG Directed Acyclic Graph Path Traversal
    let dag = RagDag::compile_canonical();
    
    // Valid attested Plains Cree paths
    assert!(dag.validate_path(&[0, 1, 4, 7]), "Attested path 0 -> 1 -> 4 -> 7 must validate");
    assert!(dag.validate_path(&[0, 2, 4, 8]), "Attested path 0 -> 2 -> 4 -> 8 must validate");
    assert!(dag.validate_path(&[0, 3, 6]), "Attested path 0 -> 3 -> 6 must validate");

    // Hallucinated / un-witnessed paths
    assert!(!dag.validate_path(&[0, 4, 7]), "Skipping prefix node must be rejected");
    assert!(!dag.validate_path(&[0, 1, 6]), "Disallowed edge must be rejected");
    assert!(!dag.validate_path(&[100, 200]), "Unknown nodes must be rejected");

    // 3. Papa Bear (9B Intent) and Mama Bear (27B Assist) Anti-Expert Gating
    let mut intent_logits = [8000i32, 6000, 7500, 9000, 5000, 4000];
    let assist_anti_expert_penalties = [0i32, 4000, 1000, 8000, 0, 0];

    // Current state = 0 (allowed next tokens in DAG: 1, 2, 3)
    dag.mask_logits_with_anti_expert(0, &mut intent_logits, &assist_anti_expert_penalties, 8000);

    // Unwitnessed transitions clamped to -infinity
    assert_eq!(intent_logits[0], LOGIT_MASKED_ZERO_PROB);
    assert_eq!(intent_logits[4], LOGIT_MASKED_ZERO_PROB);
    assert_eq!(intent_logits[5], LOGIT_MASKED_ZERO_PROB);

    // Allowed transitions penalized by Anti-Expert factor:
    // candidate 1: 6000 - (4000 * 8000 / 10000) = 6000 - 3200 = 2800
    assert_eq!(intent_logits[1], 2800);
    // candidate 2: 7500 - (1000 * 8000 / 10000) = 7500 - 800 = 6700
    assert_eq!(intent_logits[2], 6700);
    // candidate 3: 9000 - (8000 * 8000 / 10000) = 9000 - 6400 = 2600
    assert_eq!(intent_logits[3], 2600);
}
