//! # prime-sieve-worldgen-v3
//!
//! Deterministic voxel world generator using Ulam spiral and prime sieve.
//! Maps prime-indexed cells to voids and composite-indexed cells to matter,
//! with biome density derived from prime factor counts.
//!
//! ## Overview
//!
//! The core insight: map the Ulam spiral into 3D space, where prime-indexed
//! cells become voids (empty space) and composite-indexed cells become matter
//! (solid blocks). Chunk generation is fully deterministic via HMAC coordinate
//! hashing. All arithmetic is integer-only.
//!
//! ## Quick Start
//!
//! ```rust
//! use prime_sieve_worldgen_v3::{
//!     UlamSpiral3D, PrimeSieveChunkHasher, VoxelClassifier, PrimeClassificationSieve,
//!     BiomeDensity, BiomeHint, VoxelType, CHUNK_VOLUME,
//! };
//!
//! // 1. Create sieve and components
//! let sieve = PrimeClassificationSieve::new(100_000);
//! let spiral = UlamSpiral3D::new(8192);
//! let hasher = PrimeSieveChunkHasher::new(42, 100_000).unwrap();
//! let classifier = VoxelClassifier::new(sieve.clone());
//!
//! // 2. Generate a chunk at origin
//! let chunk = hasher.generate_chunk(0, 0, 0, &spiral);
//! assert_eq!(chunk.voxels.len(), CHUNK_VOLUME);
//! assert_eq!(chunk.void_count + chunk.matter_count, CHUNK_VOLUME as u32);
//!
//! // 3. Verify determinism -- same coords, same result
//! let chunk2 = hasher.generate_chunk(0, 0, 0, &spiral);
//! assert_eq!(chunk.content_hash, chunk2.content_hash);
//!
//! // 4. Classify individual indices
//! assert_eq!(classifier.classify(7), VoxelType::Void);       // 7 is prime
//! assert_eq!(classifier.classify(12), VoxelType::Matter { factor_count: 2 }); // 12 = 2^2 * 3
//!
//! // 5. Biome density from factor count
//! let density = BiomeDensity::from_factor_count(3);
//! assert_eq!(density.hint, BiomeHint::Moderate);
//! ```
//!
//! ## Wright Integration (feature-gated)
//!
//! Enable the `wright_stats` feature to access `WorldGenState` and the
//! `ForgeVisionMap` trait implementation for engine-wide validation:
//!
//! ```rust,ignore
//! use prime_sieve_worldgen_v3::{WorldGenState, ForgeVisionMap, WrightValue};
//!
//! // Build state from generated chunks
//! let state = WorldGenState::from_chunks(42, 100_000, &[chunk]);
//! assert!(state.determinism_verified);
//!
//! // Read variables via ForgeVisionMap trait
//! let vars = state.wright_variables();
//! for var in &vars {
//!     let value = state.wright_read(&var.name).unwrap();
//! }
//! ```

pub mod types;
pub mod classify;
pub mod biome;
pub mod chunk;
pub mod state;
pub mod pipeline;

#[cfg(feature = "wright_stats")]
pub mod wright;

// Re-export core types for convenient access.
pub use types::{
    BiomeDensity, BiomeHint, Chunk, VoxelCell, VoxelType, CHUNK_SIZE, CHUNK_VOLUME,
};

// Re-export components.
pub use forge_zones_v3::UlamSpiral3D;
pub use classify::{PrimeClassificationSieve, VoxelClassifier};
pub use chunk::PrimeSieveChunkHasher;

// Re-export pipeline types and functions.
pub use pipeline::{
    PipelineVoxelChunk, PipelineVoxelCell, PipelineZoneGrid, PipelineZoneId,
    generate_pipeline_chunk, generate_world, derive_local_seed,
    PIPELINE_CHUNK_SIZE, PIPELINE_CHUNK_VOLUME, ZONE_EMPTY,
};

// Conditionally re-export Wright types and WorldGenState.
#[cfg(feature = "wright_stats")]
pub use state::WorldGenState;
#[cfg(feature = "wright_stats")]
pub use wright::{ForgeVisionMap, WrightValue, WrightVariable};
