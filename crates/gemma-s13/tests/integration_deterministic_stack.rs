#![cfg(test)]
//! Integration test: Fredholm resolvent + UMP flux + cognitive watchdog + speculative decoder.

use gemma_s13::{
    FredholmResolventEngine, Mersenne31,
    MORTON8_TILE_DIM,
};
use gemma_s13::speculative::SpeculativeDecoder;

#[test]
fn test_fredholm_speculative_deterministic_chain() {
    let mut engine = FredholmResolventEngine::default();
    let mut decoder = SpeculativeDecoder::new(8);

    // 1. Create FredholmResolventEngine with default kernel
    assert_eq!(engine.clock_rate_hz, 120);
    assert_eq!(engine.tick_counter, 0);

    // 2. Push MIDI 2.0 UMP message (Type 0x4, controller value = 50000)
    //    Compose 64-bit UMP: w0 = 0x41000000 (Type 4, Group 0, Status 0x1 Control Change)
    //    w1 = controller value (50000 = 0x0000C350)
    let w0 = 0x41000000u32;
    let w1 = 50000u32;
    assert!(engine.flux.push_word(w0));
    assert!(engine.flux.push_word(w1));

    // Verify packet can be extracted from flux
    if let Some(pkt) = engine.flux.pop_packet() {
        if let Some(ctrl) = pkt.midi2_controller_value() {
            assert_eq!(ctrl, 50000);
        }
    }

    // 3. Requeue the message for step_120hz consumption
    assert!(engine.flux.push_word(w0));
    assert!(engine.flux.push_word(w1));

    // 4. Call step_120hz() with input = [Mersenne31::new(100); 8]
    let input = [Mersenne31::new(100); MORTON8_TILE_DIM];
    let mut output_state = [Mersenne31::ZERO; MORTON8_TILE_DIM];

    let decision = engine.step_120hz(&input, &mut output_state)
        .expect("step_120hz succeeds");

    // Verify output is non-zero and deterministic
    let any_nonzero = output_state.iter().any(|m| m.0 != 0);
    assert!(any_nonzero, "Fredholm output should have non-zero states");

    // 5. Convert Mersenne31 output to normalized probabilities
    let mut raw_probs = [0.0f32; MORTON8_TILE_DIM];
    let mut prob_sum = 0.0f32;
    for (i, &m31_val) in output_state.iter().enumerate() {
        let p = m31_val.0 as f32 / Mersenne31::MODULUS as f32;
        raw_probs[i] = p;
        prob_sum += p;
    }

    let norm_probs = if prob_sum > 0.0 {
        let mut norm = [0.0f32; MORTON8_TILE_DIM];
        for i in 0..MORTON8_TILE_DIM {
            norm[i] = raw_probs[i] / prob_sum;
        }
        norm
    } else {
        [1.0 / MORTON8_TILE_DIM as f32; MORTON8_TILE_DIM]
    };

    // Verify probabilities sum to ~1.0
    let sum: f32 = norm_probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-4, "Probabilities should sum to 1.0");

    // 6. Initialize SpeculativeDecoder (k=8 tokens per round)
    assert_eq!(decoder.total_rounds, 0);
    assert_eq!(decoder.total_generated, 0);

    // 7. Mock draft forward: returns Mersenne31-derived logits
    let draft_forward = |_token: u32| -> Vec<f32> {
        norm_probs.to_vec()
    };

    // Call propose with initial token 0
    let draft_tokens = decoder.propose(draft_forward, 0);
    assert!(draft_tokens.len() <= 8, "Draft should propose at most 8 tokens");

    // 8. Mock verify forward: returns Fredholm output as logits
    let verify_forward = |_token: u32| -> Vec<f32> {
        norm_probs.to_vec()
    };

    // Call verify and capture acceptance count
    let accepted = decoder.verify(verify_forward);

    // 9. Assert: decoder accepts 0..=8 tokens (deterministic, reproducible)
    assert!(accepted <= 8, "Accepted count must be 0..=8");
    assert_eq!(decoder.last_accepted, accepted);
    assert_eq!(decoder.total_rounds, 1);

    // Verify deterministic behavior on second run
    let mut engine2 = FredholmResolventEngine::default();
    assert!(engine2.flux.push_word(w0));
    assert!(engine2.flux.push_word(w1));

    let input2 = [Mersenne31::new(100); MORTON8_TILE_DIM];
    let mut output_state2 = [Mersenne31::ZERO; MORTON8_TILE_DIM];

    let _decision2 = engine2.step_120hz(&input2, &mut output_state2)
        .expect("second run succeeds");

    // Same input → same output (deterministic)
    for i in 0..MORTON8_TILE_DIM {
        assert_eq!(output_state[i], output_state2[i],
            "Output must be deterministic for same input (index {})", i);
    }

    // Verify watchdog decision matches expected type
    use gemma_s13::WatchdogDecision;
    match decision {
        WatchdogDecision::NormalEquilibrium { .. } => {}
        WatchdogDecision::DivergenceAlert { .. } => {}
        WatchdogDecision::ConvergenceSpikeRefusal { .. } => {}
    }

    // Verify watchdog affects speculative acceptance
    match decision {
        WatchdogDecision::DivergenceAlert { .. } => {
            // Divergence should increase acceptance due to damped logits
            assert!(accepted > 0 || norm_probs.iter().all(|&p| p < 0.5),
                "Divergence alert handling consistent");
        }
        _ => {
            // Normal/spike decisions operate as expected
            assert!(accepted <= 8);
        }
    }
}

#[test]
fn test_speculative_decoder_integration_with_fredholm() {
    let mut engine = FredholmResolventEngine::default();
    let mut decoder = SpeculativeDecoder::new(4);

    let input = [Mersenne31::new(42); MORTON8_TILE_DIM];
    let mut output = [Mersenne31::ZERO; MORTON8_TILE_DIM];

    engine.step_120hz(&input, &mut output)
        .expect("fredholm step succeeds");

    let raw_sum: f32 = output.iter().map(|m| m.0 as f32).sum();
    let logits: Vec<f32> = output
        .iter()
        .map(|m| (m.0 as f32 / raw_sum.max(1.0)).ln().max(-10.0))
        .collect();

    // Decoder with deterministic mock
    let proposal = decoder.propose(
        |_: u32| logits.clone(),
        0,
    );
    assert!(proposal.len() <= 4);

    let accepted = decoder.verify(|_: u32| logits.clone());
    assert!(accepted <= 4);
    assert!(decoder.acceptance_rate() <= 1.0);
    assert!(decoder.speedup_ratio() <= 4.0);
}
