//! Counting global-allocator wrapper for the audio-hot-path no-alloc proof.
//!
//! The audio (cpal callback) thread registers itself on its FIRST invocation via
//! [`register_audio_thread`]; any allocation that then happens on that thread
//! increments [`AUDIO_ALLOC_COUNT`]. The F12 Audio Telemetry panel reads it. If
//! it ever exceeds 0, the `#![no_alloc]` audio-hot-path doctrine (native DSP 2.5 ms
//! deadline) has been violated and the headline number on the panel goes red.
//!
//! WHY A THREAD-LOCAL FLAG, NOT `GetCurrentThreadId`
//! -------------------------------------------------
//! This reuses the proven allocator-safety pattern from
//! `tests/engine_hotpath_alloc.rs`: a `const`-initialised thread-local [`Cell`]
//! has no lazy heap init and no destructor, so reading it from *inside*
//! `GlobalAlloc::alloc` cannot itself allocate or re-enter the allocator. A
//! per-thread `bool` is cheaper than a `GetCurrentThreadId` syscall plus an
//! atomic compare, and — unlike the brief's original sketch — needs no
//! `windows-sys` dependency (forge-audio does not depend on it).
//!
//! WIRING
//! ------
//! This type is only a wrapper. The `#[global_allocator]` attribute MUST be set
//! in the final *binary* crate (forge-app / dreadpirateradio), NEVER in a
//! library — see `docs/tickets/FORGE-AUDIO-RT-WIRE-001.md`. Applying it here
//! would conflict with every other binary's allocator choice.
//!
//! ```ignore
//! // in the binary crate's main.rs:
//! #[global_allocator]
//! static ALLOC: forge_audio::alloc_tracer::AllocTracer =
//!     forge_audio::alloc_tracer::AllocTracer;
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

/// Allocations observed on the registered audio thread. GREEN == 0.
pub static AUDIO_ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
/// Deallocations observed on the registered audio thread.
pub static AUDIO_DEALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
/// Lifetime allocation count across ALL threads (process-wide cross-check).
pub static TOTAL_ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
/// Lifetime allocated bytes across ALL threads.
pub static TOTAL_ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    // `const`-initialised: a plain per-thread slot with no lazy heap init and no
    // destructor, so reading/writing it from inside `GlobalAlloc::alloc` cannot
    // allocate or re-enter the allocator.
    static IS_AUDIO_THREAD: Cell<bool> = const { Cell::new(false) };
}

/// Whether the *calling* thread is the registered audio thread. `try_with`
/// tolerates the thread-teardown window where the TLS slot is already gone.
#[inline]
fn on_audio_thread() -> bool {
    IS_AUDIO_THREAD.try_with(|c| c.get()).unwrap_or(false)
}

/// Counting global allocator. Forwards every call to the system allocator and
/// adds relaxed atomic increments (plus a `const`-TLS `bool` read on the audio
/// thread); none of that can allocate, so the allocator cannot recurse.
pub struct AllocTracer;

// SAFETY: every method forwards directly to `System`, a sound `GlobalAlloc`. The
// only added work is relaxed atomic increments plus a read of a
// `const`-initialised thread-local `Cell` — neither allocates nor panics, so the
// allocator cannot recurse or re-enter. (Edition 2021: the `unsafe fn` body is
// itself an unsafe context, so the `System` calls need no inner `unsafe` block —
// mirrors the proven wrapper in `tests/engine_hotpath_alloc.rs`.)
unsafe impl GlobalAlloc for AllocTracer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TOTAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        if on_audio_thread() {
            AUDIO_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        TOTAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        if on_audio_thread() {
            AUDIO_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        TOTAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        if new_size > layout.size() {
            TOTAL_ALLOC_BYTES.fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed);
        }
        if on_audio_thread() {
            AUDIO_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if on_audio_thread() {
            AUDIO_DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.dealloc(ptr, layout)
    }
}

/// Register the CALLING thread as the audio thread. Call this INSIDE the first
/// cpal callback invocation — NOT at thread spawn, because the Windows CRT
/// allocates during thread setup, which would be counted as a false positive.
pub fn register_audio_thread() {
    let _ = IS_AUDIO_THREAD.try_with(|c| c.set(true));
}

/// True if the calling thread is the registered audio thread.
pub fn is_audio_thread() -> bool {
    on_audio_thread()
}

/// Current audio-thread allocation count — the headline F12 number (GREEN == 0).
pub fn audio_alloc_count() -> u64 {
    AUDIO_ALLOC_COUNT.load(Ordering::Relaxed)
}

/// Current audio-thread deallocation count.
pub fn audio_dealloc_count() -> u64 {
    AUDIO_DEALLOC_COUNT.load(Ordering::Relaxed)
}

/// Zero the audio-thread alloc/dealloc counters (the panel's "clean since" knob).
pub fn reset_audio_alloc() {
    AUDIO_ALLOC_COUNT.store(0, Ordering::Relaxed);
    AUDIO_DEALLOC_COUNT.store(0, Ordering::Relaxed);
}

/// Process-wide (all-thread) lifetime allocation count and bytes.
pub fn total_allocs() -> (u64, u64) {
    (
        TOTAL_ALLOC_COUNT.load(Ordering::Relaxed),
        TOTAL_ALLOC_BYTES.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Layout;
    use std::sync::Mutex;

    // The audio counters are process-global statics; only EXPLICIT `AllocTracer`
    // calls move them (the test binary's own `Box`/`Vec` go through the default
    // System global allocator, bypassing this wrapper). This mutex serialises the
    // tests that explicitly allocate through the wrapper WHILE a thread is
    // registered, so their `AUDIO_*` deltas can't contaminate each other.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        // Ignore poisoning — a panicked test must not cascade-fail the rest.
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn audio_thread_flag_is_thread_local() {
        // No test ever registers on a libtest worker thread (only on spawned
        // threads), so this thread must start — and stay — unregistered.
        assert!(!is_audio_thread(), "worker thread must not start registered");
        std::thread::spawn(|| {
            assert!(!is_audio_thread());
            register_audio_thread();
            assert!(is_audio_thread());
        })
        .join()
        .unwrap();
        assert!(!is_audio_thread(), "registration must not leak across threads");
    }

    #[test]
    fn alloc_on_registered_thread_is_counted() {
        let _g = serial();
        reset_audio_alloc();
        std::thread::spawn(|| {
            register_audio_thread();
            let before = audio_alloc_count();
            let layout = Layout::from_size_align(64, 8).unwrap();
            // SAFETY: layout is valid and non-zero; we free exactly what we got.
            unsafe {
                let p = AllocTracer.alloc(layout);
                assert!(!p.is_null());
                AllocTracer.dealloc(p, layout);
            }
            assert!(
                audio_alloc_count() >= before + 1,
                "an alloc on the registered audio thread must be counted"
            );
            assert!(
                audio_dealloc_count() >= 1,
                "a dealloc on the registered audio thread must be counted"
            );
        })
        .join()
        .unwrap();
    }

    #[test]
    fn alloc_off_registered_thread_is_not_counted_as_audio() {
        let _g = serial();
        reset_audio_alloc();
        // This (unregistered) worker thread allocating through the wrapper must
        // NOT move the audio counter.
        assert!(!is_audio_thread());
        let layout = Layout::from_size_align(128, 16).unwrap();
        // SAFETY: valid non-zero layout; freed with the same layout.
        unsafe {
            let p = AllocTracer.alloc(layout);
            assert!(!p.is_null());
            AllocTracer.dealloc(p, layout);
        }
        assert_eq!(
            audio_alloc_count(),
            0,
            "an alloc off the audio thread must not increment AUDIO_ALLOC_COUNT"
        );
    }

    #[test]
    fn reset_clears_audio_counters() {
        let _g = serial();
        std::thread::spawn(|| {
            register_audio_thread();
            let layout = Layout::from_size_align(32, 8).unwrap();
            // SAFETY: valid non-zero layout; freed with the same layout.
            unsafe {
                let p = AllocTracer.alloc(layout);
                AllocTracer.dealloc(p, layout);
            }
            reset_audio_alloc();
            // No heap op between reset and these loads on this thread → exactly 0.
            assert_eq!(audio_alloc_count(), 0);
            assert_eq!(audio_dealloc_count(), 0);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn total_allocs_is_monotonic() {
        let (before, _) = total_allocs();
        let layout = Layout::from_size_align(48, 8).unwrap();
        // SAFETY: valid non-zero layout; freed with the same layout.
        unsafe {
            let p = AllocTracer.alloc(layout);
            assert!(!p.is_null());
            AllocTracer.dealloc(p, layout);
        }
        let (after, _) = total_allocs();
        assert!(after >= before + 1, "TOTAL_ALLOC_COUNT must count every alloc");
    }
}
</content>
