//! Chunk generation via HMAC coordinate hashing and voxel classification.
//!
//! `PrimeSieveChunkHasher` derives deterministic per-chunk seeds from world coordinates
//! using HMAC-SHA256, then populates 16x16x16 voxel grids.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::classify::{VoxelClassifier, PrimeClassificationSieve};
use crate::types::{
    BiomeDensity, BiomeHint, Chunk, VoxelCell, VoxelType, CHUNK_SIZE, CHUNK_VOLUME,
};
use forge_zones_v3::UlamSpiral3D;

type HmacSha256 = Hmac<Sha256>;

/// Deterministic chunk seed derivation and voxel generation.
#[derive(Debug)]
pub struct PrimeSieveChunkHasher {
    master_seed: u64,
    sieve: PrimeClassificationSieve,
}

impl PrimeSieveChunkHasher {
    /// Create a new hasher.
    ///
    /// # Errors
    /// - `master_seed == 0` -> `Err("master_seed must be non-zero")`
    /// - `sieve_upper_bound < CHUNK_VOLUME` -> `Err(...)`
    pub fn new(master_seed: u64, sieve_upper_bound: u64) -> Result<Self, String> {
        if master_seed == 0 {
            return Err("master_seed must be non-zero".to_string());
        }
        if sieve_upper_bound < CHUNK_VOLUME as u64 {
            return Err(format!(
                "sieve_upper_bound must be >= CHUNK_VOLUME ({})",
                CHUNK_VOLUME
            ));
        }
        let sieve = PrimeClassificationSieve::new(sieve_upper_bound);
        Ok(Self { master_seed, sieve })
    }

    /// Deterministic hash of chunk coordinates to a chunk-local seed.
    ///
    /// Uses HMAC-SHA256 keyed by the master seed, with the context string
    /// `"chunk|{cx}|{cy}|{cz}"`.
    pub fn hash_chunk_coord(&self, cx: i32, cy: i32, cz: i32) -> u64 {
        let key = self.master_seed.to_le_bytes();
        let context = format!("chunk|{}|{}|{}", cx, cy, cz);

        let mut mac =
            HmacSha256::new_from_slice(&key).expect("HMAC accepts any key length");
        mac.update(context.as_bytes());
        let result = mac.finalize().into_bytes();

        // Take the first 8 bytes as a little-endian u64.
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&result[..8]);
        u64::from_le_bytes(buf)
    }

    /// Borrow the underlying sieve.
    pub fn sieve(&self) -> &PrimeClassificationSieve {
        &self.sieve
    }

    /// Compute a deterministic HMAC hash of voxel data for content verification.
    fn compute_chunk_hash(voxels: &[VoxelCell], seed: u64) -> String {
        let key = seed.to_le_bytes();
        let mut mac =
            HmacSha256::new_from_slice(&key).expect("HMAC accepts any key length");

        for v in voxels {
            mac.update(&v.spiral_index.to_le_bytes());
            match v.voxel_type {
                VoxelType::Void => mac.update(&[0u8]),
                VoxelType::Matter { factor_count } => {
                    mac.update(&[1u8]);
                    mac.update(&[factor_count]);
                }
            }
            mac.update(&[v.local_pos.0, v.local_pos.1, v.local_pos.2]);
        }

        let result = mac.finalize().into_bytes();
        // Hex-encode the full 32-byte hash.
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Generate all 16x16x16 voxels for a chunk at the given coordinates.
    pub fn generate_chunk(
        &self,
        cx: i32,
        cy: i32,
        cz: i32,
        spiral: &UlamSpiral3D,
    ) -> Chunk {
        self.generate_chunk_with(cx, cy, cz, spiral, false)
    }

    /// The same chunk, entered from the inverted castle.
    ///
    /// Identical coordinates, identical chunk seed, identical spiral indices —
    /// only `classify` is read the other way round (see
    /// [`VoxelClassifier::classify_inverted`]). The geometry does not move;
    /// what the geometry MEANS does. `content_hash` differs, because the
    /// contents genuinely differ.
    pub fn generate_chunk_inverted(
        &self,
        cx: i32,
        cy: i32,
        cz: i32,
        spiral: &UlamSpiral3D,
    ) -> Chunk {
        self.generate_chunk_with(cx, cy, cz, spiral, true)
    }

    /// One assembly, two readings — the chunk-building code has a single home
    /// and the inversion is a parameter, not a copy of it.
    fn generate_chunk_with(
        &self,
        cx: i32,
        cy: i32,
        cz: i32,
        _spiral: &UlamSpiral3D,
        inverted: bool,
    ) -> Chunk {
        let classifier = VoxelClassifier::new(self.sieve.clone());
        let seed = self.hash_chunk_coord(cx, cy, cz);
        let base_index = seed % self.sieve.upper_bound;

        let mut voxels = Vec::with_capacity(CHUNK_VOLUME);
        let mut void_count: u32 = 0;
        let mut matter_count: u32 = 0;
        let mut biome_counts = [0u32; 5];

        for lz in 0..CHUNK_SIZE as u8 {
            for ly in 0..CHUNK_SIZE as u8 {
                for lx in 0..CHUNK_SIZE as u8 {
                    let local_linear =
                        (lz as u64) * 256 + (ly as u64) * 16 + (lx as u64);
                    let spiral_index = base_index.wrapping_add(local_linear);
                    let voxel_type = if inverted {
                        classifier.classify_inverted(spiral_index)
                    } else {
                        classifier.classify(spiral_index)
                    };

                    match voxel_type {
                        VoxelType::Void => void_count += 1,
                        VoxelType::Matter { factor_count } => {
                            matter_count += 1;
                            let density =
                                BiomeDensity::from_factor_count(factor_count);
                            biome_counts[density.hint as usize] += 1;
                        }
                    }

                    voxels.push(VoxelCell {
                        spiral_index,
                        voxel_type,
                        local_pos: (lx, ly, lz),
                    });
                }
            }
        }

        let dominant_biome = biome_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| BiomeHint::from_index(i))
            .unwrap_or(BiomeHint::Barren);

        let avg_density = if matter_count > 0 {
            (biome_counts
                .iter()
                .enumerate()
                .map(|(i, &c)| ((i as u64) + 1) * c as u64)
                .sum::<u64>()
                / matter_count as u64) as u8
        } else {
            0
        };

        let content_hash = Self::compute_chunk_hash(&voxels, seed);

        Chunk {
            coord: (cx, cy, cz),
            seed,
            voxels,
            void_count,
            matter_count,
            dominant_biome,
            avg_density,
            content_hash,
        }
    }

    /// Batch-generate multiple chunks.
    pub fn generate_region(
        &self,
        chunks: &[(i32, i32, i32)],
        spiral: &UlamSpiral3D,
    ) -> Vec<Chunk> {
        chunks
            .iter()
            .map(|&(cx, cy, cz)| self.generate_chunk(cx, cy, cz, spiral))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── The inverted castle, at chunk level ─────────────────────────────

    fn castles() -> (Chunk, Chunk) {
        let h = PrimeSieveChunkHasher::new(0xC0FFEE, 100_000).expect("hasher");
        let spiral = UlamSpiral3D::new(8192);
        (h.generate_chunk(3, -1, 2, &spiral), h.generate_chunk_inverted(3, -1, 2, &spiral))
    }

    /// Same castle, entered twice: identical coordinates, identical seed, and
    /// identical spiral indices voxel for voxel. Only the meaning flips.
    #[test]
    fn the_inverted_chunk_stands_on_the_same_geometry() {
        let (upright, inverted) = castles();
        assert_eq!(upright.coord, inverted.coord);
        assert_eq!(upright.seed, inverted.seed, "the inversion does not re-seed");
        assert_eq!(upright.voxels.len(), inverted.voxels.len());
        for (a, b) in upright.voxels.iter().zip(&inverted.voxels) {
            assert_eq!(a.spiral_index, b.spiral_index, "the index range is untouched");
            assert_eq!(a.local_pos, b.local_pos, "and so is where it sits");
        }
    }

    /// Void and matter trade places exactly — the counts swap, they do not
    /// merely differ.
    #[test]
    fn void_and_matter_trade_places_exactly() {
        let (upright, inverted) = castles();
        assert_eq!(upright.void_count, inverted.matter_count);
        assert_eq!(upright.matter_count, inverted.void_count);
        assert!(upright.void_count > 0 && upright.matter_count > 0, "a mixed chunk");
    }

    /// The contents really changed, so the content hash must too — otherwise
    /// a cache would hand back the wrong castle.
    #[test]
    fn the_two_castles_hash_differently() {
        let (upright, inverted) = castles();
        assert_ne!(
            upright.content_hash, inverted.content_hash,
            "same hash for different contents would collide in any chunk cache"
        );
    }

    #[test]
    fn the_inverted_chunk_replays_identically() {
        let (_, a) = castles();
        let (_, b) = castles();
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.void_count, b.void_count);
    }

    #[test]
    fn test_new_rejects_zero_seed() {
        let err = PrimeSieveChunkHasher::new(0, 10_000).unwrap_err();
        assert_eq!(err, "master_seed must be non-zero");
    }

    #[test]
    fn test_new_rejects_small_sieve() {
        let err = PrimeSieveChunkHasher::new(42, 100).unwrap_err();
        assert!(err.contains("CHUNK_VOLUME"));
    }

    #[test]
    fn test_new_ok() {
        let h = PrimeSieveChunkHasher::new(42, 10_000);
        assert!(h.is_ok());
    }

    #[test]
    fn test_hash_deterministic() {
        let h = PrimeSieveChunkHasher::new(42, 10_000).unwrap();
        let a = h.hash_chunk_coord(1, 2, 3);
        let b = h.hash_chunk_coord(1, 2, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_different_coords() {
        let h = PrimeSieveChunkHasher::new(42, 10_000).unwrap();
        let a = h.hash_chunk_coord(0, 0, 0);
        let b = h.hash_chunk_coord(1, 0, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn test_generate_chunk_volume() {
        let h = PrimeSieveChunkHasher::new(42, 100_000).unwrap();
        let spiral = UlamSpiral3D::new(8192);
        let chunk = h.generate_chunk(0, 0, 0, &spiral);
        assert_eq!(chunk.voxels.len(), CHUNK_VOLUME);
        assert_eq!(chunk.void_count + chunk.matter_count, CHUNK_VOLUME as u32);
    }

    #[test]
    fn test_generate_chunk_determinism() {
        let h = PrimeSieveChunkHasher::new(42, 100_000).unwrap();
        let spiral = UlamSpiral3D::new(8192);
        let c1 = h.generate_chunk(5, -3, 7, &spiral);
        let c2 = h.generate_chunk(5, -3, 7, &spiral);
        assert_eq!(c1.content_hash, c2.content_hash);
    }
}
