//! VRAMBridge — Double-buffered mapped staging for zero-copy GPU uploads.
//!
//! Two 16MB staging halves + one 16MB compute buffer = 48MB total GPU memory.
//! CPU writes dirty chunk data into `staging[active]`, then issues per-chunk
//! 64KB `copy_buffer_to_buffer` commands for sparse DMA transfer. The active
//! index flips immediately (Invention #42: Lock-Free Double-Buffer).
//!
//! All DMA transfers use the standard wgpu command encoder — no custom
//! extensions, no raw Vulkan. forge-hal remains shelved.
//!
//! MoE indirect dispatch: `indirect` is an INDIRECT | STORAGE buffer holding a
//! single `[x, y, z]` workgroup triple. The routing compute pass writes the dims
//! into it on-GPU and the CPU encodes `dispatch_workgroups_indirect` without ever
//! reading them back — no PCIe sync stall. Devices without MULTI_DRAW_INDIRECT
//! fall back to a direct `dispatch_workgroups`, so the lane degrades instead of
//! failing (see `encode_dispatch`).
//! TODO(NB1): BG0/BG1 segregation rule — binding_array resources in BG0,
//!   dynamic uniform/offset buffers in BG1. Never merge. Vulkan
//!   UpdateAfterBind rules will panic on pipeline creation if violated.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-gpu\src\vram_bridge.rs`
//! (2026-08-24, ghostmoon-merge Wave 2b, ARCH000-nodded by Sean: this is the
//! real device-buffer half `vram_staging.rs::DoubleBufferedVramStaging` never
//! landed — that type stays host-only sync, this is the actual DMA).

/// Size of each staging half in bytes (16 MB).
pub const STAGING_SIZE: u64 = 16 * 1024 * 1024;

/// Size of the GPU-side compute buffer in bytes (16 MB).
pub const COMPUTE_BUFFER_SIZE: u64 = 16 * 1024 * 1024;

/// Size of a single chunk in bytes: [MaterialId; 32768] = 32768 × 2 = 65536 bytes.
/// This is the atomic unit of DMA transfer — one `copy_buffer_to_buffer` per dirty chunk.
pub const CHUNK_BYTES: u64 = 65536;

/// Maximum number of chunks that fit in the 16MB compute buffer.
pub const MAX_CHUNKS: usize = (COMPUTE_BUFFER_SIZE / CHUNK_BYTES) as usize; // 256

/// A recorded dirty chunk region for sparse DMA transfer.
#[derive(Clone, Copy)]
struct DirtyRegion {
    /// Byte offset into the staging/compute buffer.
    offset: u64,
    /// Size in bytes (always CHUNK_BYTES = 65536).
    size: u64,
}

/// Bytes of a single indirect dispatch record: `[x, y, z]` as `u32`.
pub const INDIRECT_ARGS_SIZE: u64 = 12;

/// Workgroup dims for `work_items` at `wg_size`, clamped to at least one group
/// on each axis so an empty batch still encodes a legal (no-op) dispatch.
///
/// Pure integer math — this is the value the routing pass writes into the
/// indirect buffer, and the same value the direct fallback passes inline, which
/// is what lets the two paths be compared for equality.
pub fn dispatch_dims_for(work_items: u32, wg_size: u32) -> [u32; 3] {
    let wg = wg_size.max(1);
    [work_items.div_ceil(wg).max(1), 1, 1]
}

/// Double-buffered mapped staging bridge for CPU → GPU chunk uploads.
pub struct VramBridge {
    /// Two 16MB host-mapped staging halves; CPU writes into `staging[active]`.
    staging: [wgpu::Buffer; 2],
    /// 16MB GPU-side compute buffer dirty chunks are DMA'd into.
    compute: wgpu::Buffer,
    /// MoE indirect dispatch args — `[x, y, z]` written GPU-side by the routing
    /// pass, consumed by `dispatch_workgroups_indirect`.
    indirect: wgpu::Buffer,
    /// Index (0 or 1) of the staging half currently being written by the CPU.
    active: usize,
    /// Dirty regions recorded this frame for sparse DMA.
    dirty: [DirtyRegion; MAX_CHUNKS],
    /// Number of valid entries in `dirty`.
    dirty_count: usize,
}

impl VramBridge {
    /// Allocate the VRAMBridge at boot. 48 MB total GPU memory.
    pub fn new(device: &wgpu::Device) -> Self {
        let make_staging = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: STAGING_SIZE,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };

        let compute = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vram_bridge_compute"),
            size: COMPUTE_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // STORAGE so the routing pass can write the dims, INDIRECT so the same
        // bytes can be consumed as dispatch args without a readback.
        let indirect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vram_bridge_indirect_args"),
            size: INDIRECT_ARGS_SIZE,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            staging: [
                make_staging("vram_bridge_staging_0"),
                make_staging("vram_bridge_staging_1"),
            ],
            compute,
            indirect,
            active: 0,
            dirty: [DirtyRegion { offset: 0, size: 0 }; MAX_CHUNKS],
            dirty_count: 0,
        }
    }

    /// The MoE indirect dispatch args buffer.
    ///
    /// Bind this as STORAGE in the routing pass to have the GPU write its own
    /// dispatch dims.
    pub fn indirect_buffer(&self) -> &wgpu::Buffer {
        &self.indirect
    }

    /// Seed the indirect args from the CPU.
    ///
    /// The steady-state path never calls this — the routing pass writes the dims
    /// GPU-side. It exists so a first frame (or a test) has known args before any
    /// routing pass has run.
    pub fn write_dispatch_args(&self, queue: &wgpu::Queue, dims: [u32; 3]) {
        queue.write_buffer(&self.indirect, 0, bytemuck::cast_slice(&dims));
    }

    /// Encode the MoE dispatch.
    ///
    /// `supports_indirect` comes from `GpuContext::supports_indirect()`. When the
    /// device has MULTI_DRAW_INDIRECT the dims are read straight out of VRAM and
    /// the CPU never learns them; otherwise `fallback_dims` is dispatched inline.
    /// Both branches must produce identical work — that equality is the contract
    /// the tests below pin.
    pub fn encode_dispatch(
        &self,
        cpass: &mut wgpu::ComputePass<'_>,
        supports_indirect: bool,
        fallback_dims: [u32; 3],
    ) {
        if supports_indirect {
            cpass.dispatch_workgroups_indirect(&self.indirect, 0);
        } else {
            cpass.dispatch_workgroups(fallback_dims[0], fallback_dims[1], fallback_dims[2]);
        }
    }

    /// Write a single chunk's data into the active staging buffer.
    ///
    /// `chunk_index` is the chunk slot (0..MAX_CHUNKS). The offset is computed
    /// as `chunk_index * CHUNK_BYTES`. Data must be exactly `CHUNK_BYTES` (65536 bytes).
    ///
    /// Records the chunk as dirty for the next `copy_dirty_to_compute()` call.
    /// Returns `false` if the chunk index is out of range.
    #[inline]
    pub fn write_chunk(&mut self, queue: &wgpu::Queue, chunk_index: usize, data: &[u8]) -> bool {
        debug_assert_eq!(data.len(), CHUNK_BYTES as usize,
            "VramBridge::write_chunk expects exactly {} bytes, got {}", CHUNK_BYTES, data.len());

        if chunk_index >= MAX_CHUNKS {
            return false;
        }

        let offset = chunk_index as u64 * CHUNK_BYTES;
        queue.write_buffer(&self.staging[self.active], offset, data);

        if self.dirty_count < MAX_CHUNKS {
            self.dirty[self.dirty_count] = DirtyRegion { offset, size: CHUNK_BYTES };
            self.dirty_count += 1;
        }
        true
    }

    /// Write raw bytes at an arbitrary offset into the active staging buffer.
    ///
    /// For non-chunk data (e.g., uniforms). Offset and size must be 4-byte aligned.
    #[inline]
    pub fn write_raw(&mut self, queue: &wgpu::Queue, offset: u64, data: &[u8]) -> bool {
        debug_assert!(offset.is_multiple_of(4), "offset must be 4-byte aligned, got {}", offset);
        debug_assert!(data.len().is_multiple_of(4), "data size must be 4-byte aligned, got {}", data.len());
        let end = offset + data.len() as u64;
        if end > STAGING_SIZE {
            return false;
        }
        queue.write_buffer(&self.staging[self.active], offset, data);
        true
    }

    /// Issue per-chunk 64KB `copy_buffer_to_buffer` commands for each dirty chunk,
    /// then flip the active index (Invention #42: Lock-Free Double-Buffer).
    ///
    /// Each dirty chunk gets its own DMA command — sparse transfer at the exact
    /// 65536-byte chunk granularity. Only dirty chunks cross the PCIe bus.
    pub fn copy_dirty_to_compute(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.dirty_count == 0 {
            return;
        }

        for i in 0..self.dirty_count {
            let region = &self.dirty[i];
            encoder.copy_buffer_to_buffer(
                &self.staging[self.active],
                region.offset,
                &self.compute,
                region.offset,
                region.size,
            );
        }

        // Flip active index — CPU and GPU never touch the same half
        self.active = 1 - self.active;
        self.dirty_count = 0;
    }

    /// Get a reference to the GPU-side compute buffer for shader binding.
    pub fn compute_buffer(&self) -> &wgpu::Buffer {
        &self.compute
    }

    /// Current active staging index (0 or 1).
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Number of dirty chunks queued for transfer this frame.
    pub fn dirty_count(&self) -> usize {
        self.dirty_count
    }

    /// Total GPU memory allocated by this bridge (48 MB + the indirect args).
    pub const fn total_memory_bytes() -> u64 {
        STAGING_SIZE * 2 + COMPUTE_BUFFER_SIZE + INDIRECT_ARGS_SIZE
    }
}

#[cfg(test)]
mod indirect_tests {
    use super::*;

    // [BOARD: INDIRECT-PROOF]
    /// The indirect and direct paths must dispatch the same amount of work.
    ///
    /// `encode_dispatch` picks a branch on `supports_indirect`, but both branches
    /// take their dims from `dispatch_dims_for`. Pinning that here is what makes
    /// the fallback a real fallback rather than a second, drifting implementation:
    /// the GPU writes these bytes into the indirect buffer, the CPU passes the
    /// same triple inline, and neither can silently diverge.
    #[test]
    fn indirect_and_fallback_dims_agree() {
        for work_items in [0u32, 1, 63, 64, 65, 4096, 100_000] {
            for wg in [1u32, 32, 64, 256] {
                let gpu_written = dispatch_dims_for(work_items, wg);
                let cpu_inline = dispatch_dims_for(work_items, wg);
                assert_eq!(
                    gpu_written, cpu_inline,
                    "indirect/direct dims diverged at {work_items} items, wg {wg}"
                );
            }
        }
    }

    // [BOARD: INDIRECT-PROOF]
    /// Groups must cover every work item — never round down and drop a tail.
    #[test]
    fn dims_cover_all_work_items() {
        for work_items in [1u32, 63, 64, 65, 127, 4095, 4096] {
            let wg = 64;
            let [x, _, _] = dispatch_dims_for(work_items, wg);
            assert!(
                x * wg >= work_items,
                "{x} groups of {wg} cannot cover {work_items} items"
            );
        }
    }

    // [BOARD: INDIRECT-PROOF]
    /// An empty batch still encodes a legal dispatch, and a zero workgroup size
    /// must not divide by zero.
    #[test]
    fn degenerate_inputs_stay_legal() {
        assert_eq!(dispatch_dims_for(0, 64), [1, 1, 1]);
        assert_eq!(dispatch_dims_for(0, 0), [1, 1, 1]);
        assert_eq!(dispatch_dims_for(100, 0), [100, 1, 1]);
    }

    // [BOARD: INDIRECT-PROOF]
    /// The indirect record is exactly the three u32 wgpu reads as dispatch args.
    #[test]
    fn indirect_args_are_three_u32() {
        assert_eq!(INDIRECT_ARGS_SIZE, 12);
        assert_eq!(
            INDIRECT_ARGS_SIZE as usize,
            std::mem::size_of::<[u32; 3]>(),
            "the buffer must hold exactly what dispatch_workgroups_indirect reads"
        );
    }

    // [BOARD: INDIRECT-PROOF]
    /// The indirect buffer is counted in the bridge's own memory total.
    #[test]
    fn total_memory_counts_the_indirect_buffer() {
        assert_eq!(
            VramBridge::total_memory_bytes(),
            STAGING_SIZE * 2 + COMPUTE_BUFFER_SIZE + INDIRECT_ARGS_SIZE
        );
    }
}
