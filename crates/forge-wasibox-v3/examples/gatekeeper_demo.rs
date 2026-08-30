//! WASIBOX gatekeeper demo — the RUNTIME witness for the deterministic spine.
//!
//! GREEN lane: identical CPU/GPU buffers verify, the value is sealed via
//! forge-kv-math-v3 and read back through the tool boundary.
//! RED lane: a planted one-word fault produces a FIRST DIFF and the
//! deterministic `<ForgeHandoff>` escalation packet.
//!
//! Run: `cargo run -p forge-wasibox-v3 --example gatekeeper_demo`

use forge_wasibox_v3::{
    evaluate_hardware_parity, CompressionRouter, DeterministicCompressor, ForgeHandoff,
    HostMemoryCache, SealedI64, SystemState, UpwardToolCall,
};

fn main() {
    // ── GREEN lane ──────────────────────────────────────────────────────────
    let cpu: Vec<i64> = (0..64).map(|i| (i - 32) * 10_001).collect();
    let gpu = cpu.clone(); // pretend readback, bit-identical
    match evaluate_hardware_parity(&cpu, &gpu, "permyriad_mul_div_i64 [emulated]") {
        SystemState::Verified(data) => {
            println!("[gatekeeper] GREEN — bit-identical parity across {} elems", data.len());
            let mut cache = HostMemoryCache::new();
            cache.insert_sealed(SealedI64::seal("hp_max", data[40], 7919));
            cache.insert_staging("i64_emulated", data);
            let call = UpwardToolCall::parse("QuerySemanticPrimitive:hp_max").unwrap();
            println!("[gatekeeper] sealed read  -> {}", cache.dispatch_tool(&call).unwrap());
            let call = UpwardToolCall::parse("QuerySystemChecksum:i64_emulated").unwrap();
            println!("[gatekeeper] checksum     -> {}", cache.dispatch_tool(&call).unwrap());
        }
        SystemState::Diverged { .. } => unreachable!("identical buffers cannot diverge"),
    }

    // ── RED lane ────────────────────────────────────────────────────────────
    let mut bad = cpu.clone();
    bad[17] ^= 1; // the smallest planted fault
    match evaluate_hardware_parity(&cpu, &bad, "permyriad_mul_div_i64 [emulated], corpus +/-5") {
        SystemState::Verified(_) => unreachable!("planted fault must be caught"),
        state @ SystemState::Diverged { .. } => {
            let handoff = ForgeHandoff::from_state(&state).unwrap();
            println!("\n[gatekeeper] RED — FIRST DIFF caught, escalation payload:");
            println!("{}", DeterministicCompressor.compress(&handoff));
        }
    }
}
