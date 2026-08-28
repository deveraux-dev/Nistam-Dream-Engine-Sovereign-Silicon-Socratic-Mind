//! Minimal buffer-only HAL trait — a deliberately narrowed sketch, NOT a port,
//! of `F:\NewRepo\crates\forge-hal`'s real `HalBackend` trait.
//!
//! The donor crate's `HalBackend` has 20 methods (buffers, textures, samplers,
//! render/compute pipelines, bind groups, frame submission, fences, resize,
//! device-lost recovery) backing a full zero-alloc command-recording GPU HAL
//! with an `ash` (raw Vulkan) production backend. `hal_bridge.rs` — the only
//! v3 consumer so far — calls exactly two of those methods: `create_buffer`
//! and `write_buffer`. Porting the other 18 (and their supporting
//! `RenderCmd`/`ComputeCmd`/`TransferCmd`/`FrameCommands`/pipeline machinery)
//! would be net-new speculative surface with zero callers, which CLAUDE.md's
//! wire-first law forbids. This trait grows by porting more of the donor's
//! real methods the moment a second v3 caller needs one — never ahead of a
//! caller (ghostmoon-merge Wave 2c, 2026-08-24, Sean-directed: "sketch a
//! minimal forge_hal trait first").

/// Opaque handle to a HAL-owned buffer. Backend-assigned, caller-opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub(crate) u32);

/// Buffer usage flags — bitset matching the donor's `BufferUsage` (subset:
/// only the flags `hal_bridge.rs` actually passes).
#[derive(Clone, Copy, Debug)]
pub struct BufferUsage(pub u32);

impl BufferUsage {
    /// Usable as a GPU vertex buffer.
    pub const VERTEX: Self = Self(1 << 0);
    /// Usable as a GPU uniform buffer.
    pub const UNIFORM: Self = Self(1 << 2);
    /// Writable via `HalBackend::write_buffer`.
    pub const COPY_DST: Self = Self(1 << 5);
}

impl std::ops::BitOr for BufferUsage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

/// Descriptor for buffer creation.
pub struct BufferDesc {
    /// Debug label surfaced by the backend (GPU debuggers, panics).
    pub label: &'static str,
    /// Buffer size in bytes.
    pub size: u64,
    /// Usage flags (see [`BufferUsage`]).
    pub usage: BufferUsage,
    /// Whether the buffer starts mapped for immediate CPU writes.
    pub mapped_at_creation: bool,
}

/// Minimal hardware-abstraction backend: create a buffer, write into it.
/// See the module doc for why this is 2 methods, not the donor's 20.
pub trait HalBackend {
    /// Allocate a new buffer per `desc`. Cold path — allocation is legal.
    fn create_buffer(&mut self, desc: &BufferDesc) -> BufferHandle;
    /// Upload `data` at `offset` bytes into an existing buffer.
    fn write_buffer(&mut self, buf: BufferHandle, offset: u64, data: &[u8]);
}

/// Test-only backend that records buffer creation without touching a GPU.
/// Ported (buffer methods only) from `F:\NewRepo\crates\forge-hal\src\mock.rs`.
pub mod mock {
    use super::{BufferDesc, BufferHandle, HalBackend};

    /// Mock backend: hands out sequential [`BufferHandle`]s, `write_buffer` is a no-op.
    pub struct MockHalBackend {
        next_buffer: u32,
    }

    impl MockHalBackend {
        /// Build a fresh mock backend with no buffers allocated yet.
        pub fn new() -> Self {
            Self { next_buffer: 0 }
        }
    }

    impl Default for MockHalBackend {
        fn default() -> Self { Self::new() }
    }

    impl HalBackend for MockHalBackend {
        fn create_buffer(&mut self, _desc: &BufferDesc) -> BufferHandle {
            let h = BufferHandle(self.next_buffer);
            self.next_buffer += 1;
            h
        }

        fn write_buffer(&mut self, _buf: BufferHandle, _offset: u64, _data: &[u8]) {}
    }
}
