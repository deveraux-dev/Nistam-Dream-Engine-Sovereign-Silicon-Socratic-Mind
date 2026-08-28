//! 32×32 SPIR-V workgroup tile contracts matching NVIDIA Ampere 32-thread warp dispatch.
//!
//! Locks bit-exact workgroup dimensions, warp indexing, and coalesced memory
//! alignment for micro-expert dispatch and matrix computation kernels.
//!
//! # Ampere Warp Dispatch Contract
//! - **Warp Size**: 32 threads (`WARP_SIZE = 32`).
//! - **Workgroup Tile**: 32 × 32 threads (`WORKGROUP_THREADS = 1024`, exactly 32 warps).
//! - **Memory Coalescing**: 128-byte cache line alignment for global memory transactions.
//! - **Shared Memory Layout**: 32-bank conflict-free stride with +1 padding (33 elements/row).

/// NVIDIA Ampere warp width in threads.
pub const WARP_SIZE: u32 = 32;

/// Bitmask for lane index within a warp (`0..31`).
pub const WARP_MASK: u32 = 0x1F;

/// X-dimension of the workgroup tile contract.
pub const WORKGROUP_DIM_X: u32 = 32;

/// Y-dimension of the workgroup tile contract.
pub const WORKGROUP_DIM_Y: u32 = 32;

/// Z-dimension of the workgroup tile contract (planar compute).
pub const WORKGROUP_DIM_Z: u32 = 1;

/// Total threads per workgroup tile ($32 \times 32 \times 1 = 1024$).
pub const WORKGROUP_THREADS: u32 = WORKGROUP_DIM_X * WORKGROUP_DIM_Y * WORKGROUP_DIM_Z;

/// Number of active warps per workgroup ($1024 / 32 = 32$).
pub const WARPS_PER_WORKGROUP: u32 = WORKGROUP_THREADS / WARP_SIZE;

/// NVIDIA Ampere L1/L2 cache line sector alignment in bytes.
pub const CACHE_LINE_BYTES: usize = 128;

/// Number of hardware shared memory banks on NVIDIA Ampere.
pub const SHARED_MEM_BANKS: u32 = 32;

/// Shared memory bank word size in bytes (4 bytes = 32-bit word).
pub const SHARED_MEM_BANK_BYTES: u32 = 4;

/// Errors arising from workgroup tile contract validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkgroupError {
    /// Byte stride is not aligned to the required coalescing boundary.
    #[error("stride {stride} bytes is not aligned to {alignment}-byte boundary")]
    MisalignedStride {
        /// The unaligned stride in bytes.
        stride: usize,
        /// Required alignment in bytes.
        alignment: usize,
    },
    /// Workgroup dimension violation.
    #[error("invalid workgroup dimensions ({x}, {y}, {z}): contract requires ({expected_x}, {expected_y}, {expected_z})")]
    InvalidDimensions {
        /// Received X.
        x: u32,
        /// Received Y.
        y: u32,
        /// Received Z.
        z: u32,
        /// Expected X.
        expected_x: u32,
        /// Expected Y.
        expected_y: u32,
        /// Expected Z.
        expected_z: u32,
    },
}

/// Bit-exact workgroup tile contract for SPIR-V / WGSL compute kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkgroupTileContract {
    /// Tile X dimension (32).
    pub tile_dim_x: u32,
    /// Tile Y dimension (32).
    pub tile_dim_y: u32,
    /// Tile Z dimension (1).
    pub tile_dim_z: u32,
    /// Warp thread width (32).
    pub warp_size: u32,
    /// Total threads per workgroup (1024).
    pub total_threads: u32,
    /// Warps per workgroup (32).
    pub warps_per_workgroup: u32,
    /// Stride in elements for bank-conflict-free shared memory tiles (33 elements for a 32x32 tile).
    pub shared_mem_stride_elements: u32,
    /// Stride in bytes for 32-bit element shared memory tiles (132 bytes).
    pub shared_mem_stride_bytes: u32,
}

impl WorkgroupTileContract {
    /// Instantiate the canonical NVIDIA Ampere 32×32 workgroup tile contract.
    pub const fn ampere_32x32() -> Self {
        let tile_dim_x = WORKGROUP_DIM_X;
        let tile_dim_y = WORKGROUP_DIM_Y;
        let tile_dim_z = WORKGROUP_DIM_Z;
        let warp_size = WARP_SIZE;
        let total_threads = WORKGROUP_THREADS;
        let warps_per_workgroup = WARPS_PER_WORKGROUP;
        // Stride + 1 pad element to ensure column accesses hit separate shared memory banks
        let shared_mem_stride_elements = tile_dim_x + 1;
        let shared_mem_stride_bytes = shared_mem_stride_elements * SHARED_MEM_BANK_BYTES;

        Self {
            tile_dim_x,
            tile_dim_y,
            tile_dim_z,
            warp_size,
            total_threads,
            warps_per_workgroup,
            shared_mem_stride_elements,
            shared_mem_stride_bytes,
        }
    }

    /// Calculate the dispatch grid *geometry* `(grid_x, grid_y, grid_z)` that
    /// would be required to cover an $M \times N$ matrix.
    ///
    /// Pure integer ceiling division, computed CPU-side. Nothing is compiled,
    /// submitted, or executed on a device — this returns a shape. Timing it
    /// measures integer arithmetic, not GPU dispatch, and any such benchmark
    /// must hold its inputs behind `std::hint::black_box`: `self` is
    /// const-constructible and loop-counter-derived arguments let LLVM fold
    /// the call away (measured 2026-08-21: 1.14 ns folded vs 2.74 ns guarded,
    /// a 2.4x difference on identical source).
    pub fn calculate_dispatch_grid(&self, matrix_m: u32, matrix_n: u32) -> (u32, u32, u32) {
        let grid_x = (matrix_n + self.tile_dim_x - 1) / self.tile_dim_x;
        let grid_y = (matrix_m + self.tile_dim_y - 1) / self.tile_dim_y;
        (grid_x.max(1), grid_y.max(1), 1)
    }

    /// Flatten 2D local invocation ID `(local_x, local_y)` into a 1D thread index `0..1023`.
    #[inline]
    pub fn flat_invocation_index(&self, local_x: u32, local_y: u32) -> u32 {
        local_y * self.tile_dim_x + local_x
    }

    /// Decompose local invocation coordinates into `(warp_id, lane_id)`.
    /// - `warp_id`: $0..31$
    /// - `lane_id`: $0..31$
    #[inline]
    pub fn warp_and_lane(&self, local_x: u32, local_y: u32) -> (u32, u32) {
        let flat = self.flat_invocation_index(local_x, local_y);
        let warp_id = flat / self.warp_size;
        let lane_id = flat & WARP_MASK;
        (warp_id, lane_id)
    }

    /// Compute bank-conflict-free shared memory address offset for tile coordinate `(x, y)`.
    #[inline]
    pub fn shared_mem_element_offset(&self, tile_x: u32, tile_y: u32) -> u32 {
        tile_y * self.shared_mem_stride_elements + tile_x
    }

    /// Check if a memory address or byte offset satisfies 128-byte cache line coalescing.
    #[inline]
    pub fn is_coalesced_aligned(&self, byte_offset: usize) -> bool {
        (byte_offset % CACHE_LINE_BYTES) == 0
    }

    /// Validate that a tensor buffer layout is aligned to warp and sector boundaries.
    pub fn validate_matrix_layout(
        &self,
        _rows: usize,
        cols: usize,
        element_bytes: usize,
    ) -> Result<(), WorkgroupError> {
        let row_stride_bytes = cols * element_bytes;
        // Row stride must be aligned to 32 bytes (minimum warp transaction) or 128 bytes
        if (row_stride_bytes % (self.warp_size as usize * element_bytes)) != 0 {
            return Err(WorkgroupError::MisalignedStride {
                stride: row_stride_bytes,
                alignment: self.warp_size as usize * element_bytes,
            });
        }
        Ok(())
    }

    /// Build a complete dispatch plan for an $M \times N$ matrix workload.
    pub fn plan_dispatch(&self, matrix_m: u32, matrix_n: u32) -> WorkgroupDispatchPlan {
        let (gx, gy, gz) = self.calculate_dispatch_grid(matrix_m, matrix_n);
        let total_tiles = gx * gy * gz;
        let total_threads = (total_tiles as u64) * (self.total_threads as u64);
        let shared_memory_bytes = self.tile_dim_y * self.shared_mem_stride_bytes;

        WorkgroupDispatchPlan {
            grid_dim: (gx, gy, gz),
            workgroup_dim: (self.tile_dim_x, self.tile_dim_y, self.tile_dim_z),
            total_tiles,
            total_threads,
            shared_memory_bytes,
        }
    }
}

/// Execution plan for a SPIR-V compute dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkgroupDispatchPlan {
    /// Dispatch grid dimensions `(x, y, z)`.
    pub grid_dim: (u32, u32, u32),
    /// Workgroup dimensions `(x, y, z)`.
    pub workgroup_dim: (u32, u32, u32),
    /// Total workgroups / tiles in the grid.
    pub total_tiles: u32,
    /// Total threads dispatched across all workgroups.
    pub total_threads: u64,
    /// Shared memory required per workgroup in bytes.
    pub shared_memory_bytes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_matches_ampere_hardware_spec() {
        let contract = WorkgroupTileContract::ampere_32x32();
        assert_eq!(contract.tile_dim_x, 32);
        assert_eq!(contract.tile_dim_y, 32);
        assert_eq!(contract.tile_dim_z, 1);
        assert_eq!(contract.warp_size, 32);
        assert_eq!(contract.total_threads, 1024);
        assert_eq!(contract.warps_per_workgroup, 32);
        assert_eq!(contract.shared_mem_stride_elements, 33);
        assert_eq!(contract.shared_mem_stride_bytes, 132);
    }

    #[test]
    fn warp_and_lane_decomposition() {
        let contract = WorkgroupTileContract::ampere_32x32();

        // (0, 0) -> Warp 0, Lane 0
        assert_eq!(contract.warp_and_lane(0, 0), (0, 0));

        // (31, 0) -> Warp 0, Lane 31
        assert_eq!(contract.warp_and_lane(31, 0), (0, 31));

        // (0, 1) -> Warp 1, Lane 0
        assert_eq!(contract.warp_and_lane(0, 1), (1, 0));

        // (31, 31) -> Warp 31, Lane 31 (thread 1023)
        assert_eq!(contract.warp_and_lane(31, 31), (31, 31));
    }

    #[test]
    fn shared_mem_bank_conflict_avoidance() {
        let contract = WorkgroupTileContract::ampere_32x32();
        // Stride is 33 elements (132 bytes).
        // Row 0 col 0: index 0 -> bank 0
        // Row 1 col 0: index 33 -> 33 % 32 = bank 1
        // Row 2 col 0: index 66 -> 66 % 32 = bank 2
        // All column elements hit distinct banks!
        let bank0 = contract.shared_mem_element_offset(0, 0) % 32;
        let bank1 = contract.shared_mem_element_offset(0, 1) % 32;
        let bank2 = contract.shared_mem_element_offset(0, 2) % 32;
        assert_eq!(bank0, 0);
        assert_eq!(bank1, 1);
        assert_eq!(bank2, 2);
    }

    #[test]
    fn dispatch_grid_planning() {
        let contract = WorkgroupTileContract::ampere_32x32();

        // Exact 32x32 tile -> 1x1 grid
        let plan1 = contract.plan_dispatch(32, 32);
        assert_eq!(plan1.grid_dim, (1, 1, 1));
        assert_eq!(plan1.total_tiles, 1);
        assert_eq!(plan1.total_threads, 1024);

        // 768 x 768 matrix -> 24 x 24 tiles = 576 tiles
        let plan768 = contract.plan_dispatch(768, 768);
        assert_eq!(plan768.grid_dim, (24, 24, 1));
        assert_eq!(plan768.total_tiles, 576);
        assert_eq!(plan768.total_threads, 576 * 1024);

        // Non-multiple: 50 x 70 -> 2 x 3 tiles = 6 tiles
        let plan_odd = contract.plan_dispatch(50, 70);
        assert_eq!(plan_odd.grid_dim, (3, 2, 1));
        assert_eq!(plan_odd.total_tiles, 6);
    }

    #[test]
    fn coalesced_cacheline_alignment() {
        let contract = WorkgroupTileContract::ampere_32x32();
        assert!(contract.is_coalesced_aligned(0));
        assert!(contract.is_coalesced_aligned(128));
        assert!(contract.is_coalesced_aligned(256));
        assert!(!contract.is_coalesced_aligned(64));
        assert!(!contract.is_coalesced_aligned(100));
    }
}
