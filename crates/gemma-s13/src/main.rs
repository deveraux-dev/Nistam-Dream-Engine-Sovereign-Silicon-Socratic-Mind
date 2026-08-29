// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! # gemma-s13 Main Execution Engine
//!
//! Demonstrates zero-heap 1.58-bit balanced ternary inference, static vocabulary lookup,
//! 13 Moons sentinel governor, WebGPU warden math, DSP audio bus, Cree grammar ASP verification,
//! RAG-DAG logit masking, and ADR-0026 zero-retention memory sweep.

#![deny(unsafe_code)]

use gemma_s13::{
    atg::{ThermodynamicGovernor, TARGET_MODEL, UNIT_COST_CEILING_MICRO_USD, VARS_CONTEXT_TOKENS},
    audio_bus::{BiquadFilterFixed, MoeRouter, SpscRingBuffer, METRONOME_HZ},
    cree_grammar::{Animacy, AspGrammarSolver, CreeTransducer, ObviationTier},
    gpu_warden::{DeterministicNormal, EmulatedU64},
    logit_mask::RagDag,
    s13::{pack_5_trits, unpack_5_trits, Coordinate13},
    sentinel::{is_sentinel_branchless, SentinelBand, UmpWord16},
    vault::ZeroRetentionVault,
    vocab::{AutoEncoderWeights, StaticVocabTable, VocabEntry, D_MODEL},
};

fn main() {
    // 1. S13 Balanced Ternary Packing Check
    let packed = pack_5_trits([-1, 0, 1, 0, -1]).expect("Valid 5-trit pack");
    let trits = unpack_5_trits(packed).expect("Valid 5-trit unpack");
    assert_eq!(trits, [-1, 0, 1, 0, -1]);

    // 2. Coordinate13 Origin Invariant
    let origin = Coordinate13::ORIGIN;
    assert!(origin.is_origin());

    // 3. Static Vocab & Autoencoder
    static LUT: [u8; 8] = *b"GEMMA-13";
    static ENTRIES: [VocabEntry; 1] = [VocabEntry {
        offset: 0,
        len: 8,
        token_id: 0,
    }];
    let vocab_table = StaticVocabTable::new(&LUT, &ENTRIES);
    assert_eq!(vocab_table.get_token_bytes(0), Some(&b"GEMMA-13"[..]));

    let ae = AutoEncoderWeights::default_fixed();
    let mut dmodel = [0i32; D_MODEL];
    ae.byte_to_dmodel(packed, &mut dmodel);

    // 4. Sentinel Governor & 13-slot band check
    assert_eq!(is_sentinel_branchless(242), 0);
    assert_eq!(is_sentinel_branchless(243), 1);
    let band = SentinelBand::from_byte(243).expect("sentinel slot 0 of 13");
    let halt_packet = UmpWord16::compile_sentinel_halt(band, 42, 120);
    assert_eq!(halt_packet.bytes[0], 0xF0);

    // 5. GPU Warden 64-Bit Fixed-Point
    let a = EmulatedU64::from_u64(0x1_0000_0000);
    let b = EmulatedU64::from_u64(0x2_0000_0000);
    assert_eq!(a.add(b).to_u64(), 0x3_0000_0000);

    let norm = DeterministicNormal::UP;
    assert_eq!(norm.dot_emulated(&DeterministicNormal::UP), 100_000_000);

    // 6. Audio Bus & 120Hz Metronome
    let mut ring = SpscRingBuffer::<u16>::new(0);
    ring.push(440);
    assert_eq!(ring.pop(), Some(440));
    assert_eq!(METRONOME_HZ, 120);
    let mut filter = BiquadFilterFixed::lowpass_smoothing();
    let _ = filter.process_sample(10_000);
    let moe = MoeRouter::new();
    let (expert, _) = moe.route_centroid(0x1111_2222_3333_4444);
    assert_eq!(expert, 0);

    // 7. Cree Grammar & ASP Obviation Solver
    let slot = CreeTransducer::parse_stroke_bytes(b"wapamew").expect("Valid VTA parse");
    AspGrammarSolver::solve_constraints(
        &slot,
        ObviationTier::ThirdProximate,
        Some(Animacy::Animate),
        Some(ObviationTier::ThirdObviative),
    )
    .expect("ASP Constraints satisfied");

    // 8. RAG-DAG Logit Masking
    let rag = RagDag::compile_canonical();
    let mut logits = [1000i32; 10];
    rag.apply_logit_mask(0, &mut logits);
    assert_eq!(logits[1], 1000);

    // 9. Active Thermodynamic Governor
    let atg = ThermodynamicGovernor::new_zero_point(0xCAFE);
    assert_eq!(TARGET_MODEL, "gemini-2.5-flash");
    assert_eq!(VARS_CONTEXT_TOKENS, 450_000);
    let packet = atg
        .intercept_sentinel_breach(band, 1, 120)
        .expect("Escalation within budget");
    assert!(packet.estimated_cost_micro_usd <= UNIT_COST_CEILING_MICRO_USD);

    // 10. ADR-0026 Zero-Retention Vault Sweep
    let mut vault = ZeroRetentionVault::new();
    vault.stage_transient_data(&[0xDE, 0xAD, 0xBE, 0xEF], 100, 10);
    assert!(vault.sweep_if_expired(110));
    assert_eq!(vault.staging_registers[0], 0);

    // 11. Fleet VRAM Budget & 5-Bear Ledger Oracle (ADR-0026 / Zero-Mock Ledger)
    let budget = gemma_s13::vram_budget::FleetBudget {
        card_mb: 8192,
        baseline_resident_mb: 1604,
        members: &gemma_s13::vram_budget::DEMO_FLEET,
        ctx_tokens: 4096,
        kv_width: gemma_s13::vram_budget::KvWidth::I8,
        overheads: gemma_s13::vram_budget::DEMO_OVERHEADS,
    };
    assert!(budget.fits());
    assert_eq!(budget.weight_bytes(), 3_340_102_812); // 9B + 2B + 2B_M3 + M2
    assert_eq!(budget.kv_bytes_per_token(), 401_408);

    println!("=== DEMO FLEET VRAM LEDGER (5 BEARS / 8GB VRAM ORACLE) ===");
    for (i, m) in budget.members.iter().enumerate() {
        println!(
            "  [{}] {:15} | Weights: {:>10} B | KV/tok: {:>6} B | Shared: {}",
            i, m.geom.name, m.geom.weight_bytes(), m.geom.kv_bytes_per_token(budget.kv_width), m.shares_weights
        );
    }
    println!("  Total Weights Resident : {:>10} B ({} MB)", budget.weight_bytes(), budget.weight_bytes() / (1024 * 1024));
    println!("  KV Cache (4k ctx, i8)  : {:>10} B ({} MB)", budget.kv_bytes(), budget.kv_bytes() / (1024 * 1024));
    println!("  Overheads (FB+Ring+Stg): {:>10} B ({} MB)", budget.overheads.bytes(), budget.overheads.bytes() / (1024 * 1024));
    println!("  Committed VRAM         : {:>10} B ({} MB)", budget.committed_bytes(), budget.committed_bytes() / (1024 * 1024));
    println!("  Usable VRAM (8GB-base) : {:>10} B ({} MB)", budget.usable_bytes(), budget.usable_bytes() / (1024 * 1024));
    println!("  Headroom               : {:>10} B ({} MB)", budget.headroom_bytes(), budget.headroom_bytes() / (1024 * 1024));
    println!("  Max Safe Context Tokens: {}", budget.max_ctx_tokens());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_execution_invariants_pass() {
        main();
    }
}
