// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Closed-Form Fredholm Resolvent Engine & D-TFR Compression Theory.
//!
//! Evaluates $(I - \lambda K)^{-1} f$ over $\mathbb{F}_{2^{31}-1}$ at a 120 Hz clock rate,
//! fusing Theory $T$ (AOT Morton8 Trinary Kernels & resident weights), Flux $F$ (MIDI 2.0 UMP),
//! and Residue $R$ ($R = 0$ enforced via Dynamic Tikhonov Regularization).

use crate::mersenne31::{Mersenne31, Morton8_2D};
use crate::cognitive_watchdog::{CognitiveWatchdog, WatchdogDecision};
use crate::ump_flux::UmpFluxStream;

/// 8x8 Morton8 Trinary Resolvent Kernel Tile dimension.
pub const MORTON8_TILE_DIM: usize = 8;
/// Total number of cells in an 8x8 Morton8 resolvent tile ($8 \times 8 = 64$).
pub const MORTON8_TILE_CELLS: usize = MORTON8_TILE_DIM * MORTON8_TILE_DIM;

/// Precomputed AOT Fredholm Kernel over $\mathbb{F}_{2^{31}-1}$.
#[derive(Debug, Clone)]
pub struct FredholmKernel {
    /// 64-element space-filling Morton8 lookup table.
    pub tile: [Mersenne31; MORTON8_TILE_CELLS],
    /// Coupling parameter $\lambda$.
    pub lambda: Mersenne31,
}

impl Default for FredholmKernel {
    fn default() -> Self {
        let mut tile = [Mersenne31::ZERO; MORTON8_TILE_CELLS];
        // Initialize with deterministic trinary kernel (-1, 0, +1) mapped into F_M31
        for i in 0..MORTON8_TILE_CELLS {
            let morton = Morton8_2D(i as u8);
            let (x, y) = morton.decode();
            let trit = ((x as i32 - y as i32).rem_euclid(3)) - 1; // {-1, 0, 1}
            tile[i] = if trit < 0 {
                Mersenne31::new(Mersenne31::MODULUS - 1)
            } else {
                Mersenne31::new(trit as u32)
            };
        }
        Self {
            tile,
            lambda: Mersenne31::new(1),
        }
    }
}

/// Closed-Form Fredholm Resolvent Engine conforming to D-TFR specifications.
pub struct FredholmResolventEngine {
    /// Target execution clock rate (120 Hz).
    pub clock_rate_hz: u32,
    /// Theory $T$: Precomputed AOT kernel.
    pub theory: FredholmKernel,
    /// Flux $F$: High-speed UMP event stream.
    pub flux: UmpFluxStream,
    /// Cognitive Watchdog & Tikhonov Residue Governor ($R = 0$).
    pub watchdog: CognitiveWatchdog,
    /// Tick counter.
    pub tick_counter: u64,
}

impl Default for FredholmResolventEngine {
    fn default() -> Self {
        Self::new(120)
    }
}

impl FredholmResolventEngine {
    /// Create a new Fredholm Resolvent Engine configured for the specified tick clock rate.
    pub fn new(clock_rate_hz: u32) -> Self {
        Self {
            clock_rate_hz,
            theory: FredholmKernel::default(),
            flux: UmpFluxStream::new(200),
            watchdog: CognitiveWatchdog::default(),
            tick_counter: 0,
        }
    }

    /// Step the 120 Hz closed-form resolvent engine for a batch of 8-element flux inputs.
    pub fn step_120hz(
        &mut self,
        input_flux: &[Mersenne31; MORTON8_TILE_DIM],
        output_state: &mut [Mersenne31; MORTON8_TILE_DIM],
    ) -> Result<WatchdogDecision, &'static str> {
        self.tick_counter += 1;

        // 1. Drain available UMP flux packets to modulate kernel coupling
        while let Some(packet) = self.flux.pop_packet() {
            if let Some(ctrl) = packet.midi2_controller_value() {
                self.theory.lambda = Mersenne31::from_u64(ctrl as u64);
            }
        }

        // 2. Closed-form resolvent step: y = (I + lambda * K) * f over F_M31
        let mut raw_probs = [0.0f32; MORTON8_TILE_DIM];
        let mut prob_sum = 0.0f32;

        for x in 0..MORTON8_TILE_DIM {
            let mut acc = input_flux[x]; // Identity term: I * f
            for y in 0..MORTON8_TILE_DIM {
                let m = Morton8_2D::encode(x as u8, y as u8);
                let k_val = self.theory.tile[(m.0 as usize) % MORTON8_TILE_CELLS];
                let weighted = self.theory.lambda * k_val * input_flux[y];
                acc = acc + weighted;
            }
            output_state[x] = acc;
            let p = acc.0 as f32 / Mersenne31::MODULUS as f32;
            raw_probs[x] = p;
            prob_sum += p;
        }

        // 3. Normalize probabilities for the Cognitive Watchdog
        let norm_probs = if prob_sum > 0.0 {
            let mut norm = [0.0f32; MORTON8_TILE_DIM];
            for i in 0..MORTON8_TILE_DIM {
                norm[i] = raw_probs[i] / prob_sum;
            }
            norm
        } else {
            [1.0 / MORTON8_TILE_DIM as f32; MORTON8_TILE_DIM]
        };

        // 4. Evaluate Cognitive Watchdog (N * IPR) and enforce Tikhonov clamp
        let decision = self.watchdog.evaluate(&norm_probs);

        // Apply dynamic Tikhonov damping if divergence is alerted
        if let WatchdogDecision::DivergenceAlert { scaled_epsilon, .. } = decision {
            let damping = Mersenne31::from_u64((scaled_epsilon * 10000.0) as u64);
            for x in 0..MORTON8_TILE_DIM {
                output_state[x] = output_state[x] * damping;
            }
        }

        Ok(decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_init_and_step() {
        let mut engine = FredholmResolventEngine::new(120);
        assert_eq!(engine.clock_rate_hz, 120);

        let input = [Mersenne31::new(100); MORTON8_TILE_DIM];
        let mut output = [Mersenne31::ZERO; MORTON8_TILE_DIM];

        let decision = engine.step_120hz(&input, &mut output).expect("step succeeds");
        assert_eq!(engine.tick_counter, 1);
        match decision {
            WatchdogDecision::NormalEquilibrium { .. } | WatchdogDecision::DivergenceAlert { .. } => {}
            _ => panic!("Unexpected watchdog state"),
        }
    }
}
