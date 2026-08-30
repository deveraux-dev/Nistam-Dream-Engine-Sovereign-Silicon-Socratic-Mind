//! World generation state aggregation for ForgeVisionMap inspection.
//!
//! `WorldGenState` collects statistics from generated chunks and provides
//! a snapshot of the world generator's current state.

use crate::chunk::PrimeSieveChunkHasher;
use crate::types::{BiomeDensity, Chunk, VoxelType};
use forge_zones_v3::UlamSpiral3D;

/// Runtime state snapshot for world generation inspection.
#[derive(Debug, Clone)]
pub struct WorldGenState {
    /// Master seed used for generation.
    pub master_seed: u64,
    /// Upper bound of the sieve.
    pub sieve_upper_bound: u64,
    /// Number of chunks generated.
    pub chunks_generated: u64,
    /// Total void voxels across all chunks.
    pub total_voids: u64,
    /// Total matter voxels across all chunks.
    pub total_matter: u64,
    /// Histogram of biome types: [Barren, Sparse, Moderate, Dense, Lush].
    pub biome_histogram: [u64; 5],
    /// Content hash of the last chunk.
    pub last_chunk_hash: String,
    /// Whether determinism verification passed.
    pub determinism_verified: bool,
}

impl WorldGenState {
    /// Aggregate state from a set of generated chunks.
    ///
    /// Computes biome_histogram by counting cells per BiomeHint variant
    /// across all chunks. Only matter voxels contribute to the histogram.
    ///
    /// `determinism_verified` is set by regenerating the first chunk and
    /// comparing its content_hash against the stored one.
    pub fn from_chunks(seed: u64, sieve_bound: u64, chunks: &[Chunk]) -> Self {
        let chunks_generated = chunks.len() as u64;
        let mut total_voids: u64 = 0;
        let mut total_matter: u64 = 0;
        let mut biome_histogram = [0u64; 5];
        let mut last_chunk_hash = String::new();

        for chunk in chunks {
            total_voids += chunk.void_count as u64;
            total_matter += chunk.matter_count as u64;
            last_chunk_hash = chunk.content_hash.clone();

            // Count biome hints from matter voxels
            for cell in &chunk.voxels {
                if let VoxelType::Matter { factor_count } = cell.voxel_type {
                    let density = BiomeDensity::from_factor_count(factor_count);
                    biome_histogram[density.hint as usize] += 1;
                }
            }
        }

        // Verify determinism by regenerating the first chunk and comparing hashes
        let determinism_verified = if let Some(first_chunk) = chunks.first() {
            if let Ok(hasher) = PrimeSieveChunkHasher::new(seed, sieve_bound) {
                let spiral = UlamSpiral3D::new(8192);
                let (cx, cy, cz) = first_chunk.coord;
                let regenerated = hasher.generate_chunk(cx, cy, cz, &spiral);
                regenerated.content_hash == first_chunk.content_hash
            } else {
                false
            }
        } else {
            // Empty slice: nothing to verify, vacuously true
            true
        };

        Self {
            master_seed: seed,
            sieve_upper_bound: sieve_bound,
            chunks_generated,
            total_voids,
            total_matter,
            biome_histogram,
            last_chunk_hash,
            determinism_verified,
        }
    }
}
