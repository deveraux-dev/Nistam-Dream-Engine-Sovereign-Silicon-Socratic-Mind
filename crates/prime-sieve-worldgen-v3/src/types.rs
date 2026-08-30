//! Prime sieve world generation types.

pub use forge_core_v3::organs::worldgen_kit::{BiomeHint, VoxelType, VoxelCell};

/// Chunk size (16 voxels per axis).
pub const CHUNK_SIZE: usize = 16;
/// Total voxels per chunk (4096).
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Biome density level with hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeDensity {
    /// Density level (0-5).
    pub level: u8,
    /// Biome hint (Barren, Sparse, Moderate, Dense, Lush).
    pub hint: BiomeHint,
}

/// Generated voxel chunk.
#[derive(Debug, Clone)]
pub struct VoxelChunk {
    /// Chunk coordinates (cx, cy, cz).
    pub coord: (i32, i32, i32),
    /// Seed used for deterministic generation.
    pub seed: u64,
    /// All voxels in the chunk.
    pub voxels: Vec<VoxelCell>,
    /// Count of void voxels.
    pub void_count: u32,
    /// Count of matter voxels.
    pub matter_count: u32,
    /// Dominant biome hint.
    pub dominant_biome: BiomeHint,
    /// Average density level.
    pub avg_density: u8,
    /// Content hash for verification.
    pub content_hash: String,
}

/// Type alias for VoxelChunk.
pub type Chunk = VoxelChunk;
