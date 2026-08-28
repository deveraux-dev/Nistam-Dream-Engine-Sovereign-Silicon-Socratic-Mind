//! Opaque state pointer. Warden never dereferences.
//!
//! Canonical contract: NDE_OPAQUE_CUDA_DMA_SPEC.md. The sovereign engine
//! threads raw memory boundaries as *const c_void into the dispatch pipeline
//! so the GPU reads the state via DMA. Zero heap allocation, zero memcpy.

use core::ffi::c_void;

/// Opaque pointer into sovereign state memory.
///
/// Warden promises not to dereference, only to schedule. The dispatching
/// Sieve owns provenance and must guarantee the pointer remains valid until
/// the returned `DispatchFence` completes.
#[repr(transparent)]
pub struct OpaqueSieveState(pub *const c_void);

/// SAFETY: The pointer is opaque to Warden. The dispatching Sieve is
/// responsible for thread-safe access to the underlying memory.
///
/// This workspace bans `unsafe_code` (`F:\v3\Cargo.toml` `[workspace.lints.rust]`,
/// ARCH000 2026-08-09) as a hard default. This is a deliberate, load-bearing
/// exception, not an oversight: `OpaqueSieveState` is the zero-heap-alloc,
/// zero-memcpy DMA passthrough for GPU dispatch (`NDE_OPAQUE_CUDA_DMA_SPEC.md`)
/// — the whole point is that Warden threads a raw pointer through scheduling
/// without touching it. A safe typed handle (index into a registry) was
/// considered and rejected for Phase 1: it would give up the zero-copy
/// property this type exists for, before Phase 2 even wires a real GPU queue
/// to make that trade worth it. Confined to this file; nothing outside it
/// needs `unsafe`.
#[allow(unsafe_code)]
unsafe impl Send for OpaqueSieveState {}
#[allow(unsafe_code)]
unsafe impl Sync for OpaqueSieveState {}

impl OpaqueSieveState {
    /// Wrap a raw pointer into an opaque state handle.
    ///
    /// # Safety
    /// Caller guarantees `ptr` stays valid until the associated
    /// DispatchFence completes. Warden never dereferences.
    #[allow(unsafe_code)]
    pub unsafe fn wrap(ptr: *const c_void) -> Self {
        Self(ptr)
    }

    /// Sentinel value for tests and idle tickets. Never dispatch this.
    pub fn null() -> Self {
        Self(core::ptr::null())
    }

    /// The wrapped raw pointer.
    pub fn as_raw(&self) -> *const c_void {
        self.0
    }
}

impl Clone for OpaqueSieveState {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl core::fmt::Debug for OpaqueSieveState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "OpaqueSieveState({:p})", self.0)
    }
}
