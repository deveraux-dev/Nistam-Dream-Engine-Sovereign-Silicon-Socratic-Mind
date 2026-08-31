// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! # gemma-s13
//!
//! Sovereign, zero-heap, 1.58-bit balanced ternary inference, static vocabulary,
//! Nehiyaw Natural Law sentinel governor, WebGPU warden, DSP audio bus,
//! Zero-Generative Cree grammar verification, RAG-DAG logit masking,
//! Active Thermodynamic Governor, and ADR-0026 zero-retention data vault for Gemma.
//!
//! ## Deterministic Substrate Stack
//!
//! The core inference loop chains four deterministic modules for real-time closed-form control:
//!
//! **`UmpFluxStream`** → **`FredholmResolventEngine`** → **`CognitiveWatchdog`** → **`SpeculativeDecoder`**
//!
//! Each module operates with zero heap allocation, O(1) or O(n) bounded latency per tick, and deterministic
//! output given the same input state.
//!
//! ### Module Roles
//!
//! - **`UmpFluxStream`**: MIDI Universal Message Protocol packet ingress. Decodes real-time control messages
//!   (CC, pitch bend, note-on) into machine-friendly `UmpPacket` atoms. Use when handling external
//!   hardware or DAW automation feeds.
//! - **`FredholmResolventEngine`**: Closed-form spectral state solver. Accepts decoder state and
//!   resolves latent projections onto a 5D geodesic manifold via Morton-order tiling (64-point kernel).
//!   Use for spatial reasoning over model state; guarantees O(64) operations per tick.
//! - **`CognitiveWatchdog`**: Divergence detector. Monitors Fredholm output against running
//!   Tikhonov-regularized estimates to flag instability (NaN, inf, out-of-range). O(n) cost where
//!   n = state vector width (≤128 typical). Use to abort decoding or trigger fallback paths.
//! - **`SpeculativeDecoder`** (std feature): Lookahead acceptance sampler. Feeds Watchdog decisions
//!   back to kernel to adjust per-token acceptance threshold. O(1) ring-buffer operations.
//!
//! ### Integration Pattern
//!
//! ```text
//! fn inference_loop_120hz() {
//!     let mut engine = FredholmResolventEngine::new();
//!     let mut watchdog = CognitiveWatchdog::new(/* tikhonov_lambda */ 0.001);
//!     let mut ump_rx = UmpFluxStream::open_rx();
//!     let mut decoder = SpeculativeDecoder::new(/* threshold */ 0.5);
//!
//!     loop {
//!         // 1. Ingest external control (8.3ms window @ 120 Hz)
//!         if let Some(packet) = ump_rx.next() {
//!             engine.apply_ump_packet(&packet);
//!         }
//!
//!         // 2. Solve spectral state (O(64) Fredholm kernel)
//!         let resolvent = engine.step();
//!
//!         // 3. Monitor for instability (O(n) clamp checks)
//!         let watchdog_decision = watchdog.evaluate(&resolvent.state);
//!
//!         // 4. Accept or reject speculative token
//!         let token = match watchdog_decision {
//!             WatchdogDecision::Accept => decoder.accept(resolvent.logits),
//!             WatchdogDecision::Reject => decoder.fallback(),
//!         };
//!
//!         // Emit token and cycle
//!         output_tx.send(token);
//!         thread::sleep(Duration::from_millis(8));  // 120 Hz cadence
//!     }
//! }
//! ```
//!
//! ### Performance Characteristics
//!
//! | Module | Complexity | Heap | Notes |
//! |--------|-----------|------|-------|
//! | `FredholmResolventEngine` | O(64) | 0 | Morton8 tile dim = 8×8 kernel |
//! | `CognitiveWatchdog` | O(n) | 0 | n ≤ 128 typical; O(1) Tikhonov update |
//! | `UmpFluxStream` | O(1) | 0 | Ring buffer decode; stateless |
//! | `SpeculativeDecoder` | O(1) | 0 | Threshold adaptation via ring buffer |
//!
//! ### Invariants
//!
//! - **Deterministic**: Identical input state + UMP stream yields identical output tokens across runs.
//! - **Zero-heap**: All allocations are static or stack-local; no `Vec`, `HashMap`, or dynamic sizing.
//! - **Bounded latency**: No tick exceeds 6 ms (reserve budget for 120 Hz @ 12 ms window).
//! - **Seam**: Watchdog clamp values (min/max) must match decoder's speculative range to prevent aliasing.
//!

#![deny(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "std")]
extern crate std;

pub mod astrolabe_projection_5d;
pub mod atg;
pub mod audio_bus;
pub mod cree_canon;
pub mod cree_grammar;
pub mod first_flat_room;
pub mod gpu_warden;
pub mod logit_mask;
pub mod m5_geodesic;
pub mod model_4b;
pub mod model_9b;
#[cfg(feature = "std")]
pub mod prompt_cache;
pub mod s13;
pub mod sentinel;
#[cfg(feature = "std")]
pub mod speculative;
pub mod star_codebook;
#[cfg(feature = "std")]
pub mod star_tensor;
pub mod three_bears;
pub mod vault;
pub mod vocab;
pub mod vram_budget;

pub use astrolabe_projection_5d::{ProjectedStar, Star5D, project_star_batch, spectral_temperature_rgb};
pub use star_codebook::{BakedStarCentroid, StarCodebookView, HYG_MAGIC, BYTES_PER_STAR};

pub use first_flat_room::{
    ActionDraft, ChoiceArchetype, FirstFlatRoomEngine, FirstFlatRoomStepResult, MacroSeedExpansion,
    ParityFilterOutcome, RoomUmweltProse, SentryAuditResult, VoxelSensorySnapshot,
};
pub use logit_mask::{AntiExpertGate, RagDag, WitnessedNode, LOGIT_MASKED_ZERO_PROB};
pub use three_bears::{
    BabyBear2bConfig, IntentMirrorOutput, RenderCodecOutput, AssistDirectOutput,
    ThreeBearsFleet, ThreeBearsFleetOutput, compute_anti_expert_conjugate,
    ternary_dot_product_5trit,
};
pub use s13::{
    isqrt_u64, s13_rms_norm_i16, s13_rms_norm_i32, MerkleMorinHeader, MerkleMorinMatrix,
    S13Error, S13TensorView, S133_MAGIC, S13M_MAGIC, S13_MERKLE_MAGIC,
};
#[cfg(feature = "std")]
pub use prompt_cache::{load_s13n_norms, S13N_MAGIC};
#[cfg(feature = "std")]
pub use star_tensor::{
    fit_hosvd, fold, frobenius_error, jacobi_svd, mode_multiply, unfold, S13TensorCore,
    StarTensorConfig, StarTensorDecomposition, StarTensorError,
};

pub mod mersenne31;
pub mod cognitive_watchdog;
pub mod ump_flux;
pub mod fredholm_resolvent;
pub mod somatic_profile;
pub mod model_27b;
pub mod accessibility_gate;
pub mod two_drums;
#[cfg(feature = "std")]
pub mod constrain;
#[cfg(feature = "std")]
pub mod celestial_bot;

pub use mersenne31::{reduce_m31, Mersenne31, Morton8_2D, MERSENNE_31_MODULUS};
pub use cognitive_watchdog::{CognitiveWatchdog, TikhonovClamp, WatchdogDecision};
pub use ump_flux::{UmpFluxStream, UmpMessageType, UmpPacket};
pub use fredholm_resolvent::{FredholmKernel, FredholmResolventEngine, MORTON8_TILE_DIM};
pub use somatic_profile::{
    BlindnessProfile, CognitiveElderProfile, DeafnessProfile, MotorImpairmentProfile,
    NeurodivergentProfile, SomaticAccessibilityProfile, SpeechNonverbalProfile,
    TraumaRecoveryProfile,
};
pub use model_27b::{
    Gemma27bConfig, S13Norm27b, Somatic27bProjectionWeights, D_FF_27B, D_HEAD_27B, D_MODEL_27B,
    N_HEADS_27B, N_KV_HEADS_27B, N_LAYERS_27B,
};
pub use accessibility_gate::{
    AccessibilityGateEngine, AccessibilityOutput, TriStateChoice,
};
pub use two_drums::TwoDrums;
#[cfg(feature = "std")]
pub use constrain::{
    PdaStateCache, PdaStateDescriptor, PdaStateId, WeldConstraint, TOKEN_BOS, TOKEN_END_OF_TURN,
    TOKEN_EOS, TOKEN_PAD, TOKEN_START_OF_TURN, TOKEN_UNK,
};
#[cfg(feature = "std")]
pub use celestial_bot::{CelestialGemmaBot, StarHopResult, LANDMARK_NAMES};


