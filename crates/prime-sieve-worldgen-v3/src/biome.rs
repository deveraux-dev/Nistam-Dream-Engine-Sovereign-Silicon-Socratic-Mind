//! Biome density mapping from prime factor counts.
//!
//! Maps the distinct prime factor count (omega function) of composite voxels
//! to biome density levels and hints.

use crate::types::{BiomeDensity, BiomeHint, VoxelCell, VoxelType};

impl BiomeDensity {
    /// Map a factor count to a biome density level and hint.
    ///
    /// | factor_count | level | hint     |
    /// |-------------|-------|----------|
    /// | 0           | 0     | Barren   |
    /// | 1           | 1     | Barren   |
    /// | 2           | 2     | Sparse   |
    /// | 3           | 3     | Moderate |
    /// | 4           | 4     | Dense    |
    /// | 5+          | 5     | Lush     |
    pub fn from_factor_count(factor_count: u8) -> Self {
        match factor_count {
            0 => BiomeDensity { level: 0, hint: BiomeHint::Barren },
            1 => BiomeDensity { level: 1, hint: BiomeHint::Barren },
            2 => BiomeDensity { level: 2, hint: BiomeHint::Sparse },
            3 => BiomeDensity { level: 3, hint: BiomeHint::Moderate },
            4 => BiomeDensity { level: 4, hint: BiomeHint::Dense },
            _ => BiomeDensity { level: 5, hint: BiomeHint::Lush },
        }
    }

    /// Compute the dominant biome density for a chunk of voxels.
    ///
    /// Aggregates density across all matter voxels. Returns `None` if
    /// the chunk contains no matter voxels (all voids).
    pub fn chunk_density(cells: &[VoxelCell]) -> Option<Self> {
        let mut biome_counts = [0u32; 5]; // Barren, Sparse, Moderate, Dense, Lush
        let mut matter_count = 0u32;

        for cell in cells {
            if let VoxelType::Matter { factor_count } = cell.voxel_type {
                matter_count += 1;
                let density = Self::from_factor_count(factor_count);
                biome_counts[density.hint as usize] += 1;
            }
        }

        if matter_count == 0 {
            return None;
        }

        // Dominant biome: the hint with the highest count
        let (dominant_idx, _) = biome_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .unwrap(); // safe: matter_count > 0

        let dominant_hint = BiomeHint::from_index(dominant_idx);

        // Average density as weighted mean of levels (integer arithmetic)
        // Use the level that each biome hint maps to for weighting
        let weighted_sum: u64 = biome_counts
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let level = match i {
                    0 => 0u64, // Barren -> levels 0-1, weight as 0 (conservative)
                    1 => 2,    // Sparse
                    2 => 3,    // Moderate
                    3 => 4,    // Dense
                    4 => 5,    // Lush
                    _ => 0,
                };
                level * c as u64
            })
            .sum();
        let avg_level = (weighted_sum / matter_count as u64) as u8;

        Some(BiomeDensity {
            level: avg_level,
            hint: dominant_hint,
        })
    }
}
