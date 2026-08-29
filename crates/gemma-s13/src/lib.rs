// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! # gemma-s13
//!
//! Sovereign, zero-heap, 1.58-bit balanced ternary inference, static vocabulary,
//! Nehiyaw Natural Law sentinel governor, WebGPU warden, DSP audio bus,
//! Zero-Generative Cree grammar verification, RAG-DAG logit masking,
//! Active Thermodynamic Governor, and ADR-0026 zero-retention data vault for Gemma.

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
pub mod nipr;
#[cfg(feature = "std")]
pub mod prompt_cache;
pub mod s13;
pub mod sentinel;
#[cfg(feature = "std")]
pub mod speculative;
pub mod star_codebook;
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
pub use s13::{MerkleMorinHeader, MerkleMorinMatrix, S13Error, S13_MERKLE_MAGIC, S13TensorView, S13M_MAGIC, S133_MAGIC};
#[cfg(feature = "std")]
pub use prompt_cache::{load_s13n_norms, S13N_MAGIC};

pub mod mersenne31;
pub mod cognitive_watchdog;
pub mod ump_flux;
pub mod fredholm_resolvent;
pub mod somatic_profile;
pub mod model_27b;
pub mod accessibility_gate;

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
    Gemma27bConfig, Somatic27bProjectionWeights, D_FF_27B, D_HEAD_27B, D_MODEL_27B,
    N_HEADS_27B, N_KV_HEADS_27B, N_LAYERS_27B,
};
pub use accessibility_gate::{
    AccessibilityGateEngine, AccessibilityOutput, TriStateChoice,
};

