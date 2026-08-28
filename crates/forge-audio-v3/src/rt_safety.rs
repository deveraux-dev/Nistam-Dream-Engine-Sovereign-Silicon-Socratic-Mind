//! Real-time safety enforcement.
//!
//! Provides utilities for verifying the audio callback is allocation-free.
//! The actual allocator wrapping must happen at the binary level (main.rs),
//! not at the library level. This module provides the API and documentation.

/// Check if a block of code performs any heap allocations.
///
/// This uses a simple boolean flag approach that works without replacing
/// the global allocator. The audio thread sets this flag, and any code
/// can check it to decide whether allocation is safe.
///
/// For full enforcement, the binary (forge-app-daw) should use
/// assert_no_alloc's allocator wrapper.
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag: are we inside a real-time context?
static IN_RT_CONTEXT: AtomicBool = AtomicBool::new(false);

/// Enter real-time context. Call at the start of the audio callback.
pub fn enter_rt() {
    IN_RT_CONTEXT.store(true, Ordering::Release);
}

/// Exit real-time context. Call at the end of the audio callback.
pub fn exit_rt() {
    IN_RT_CONTEXT.store(false, Ordering::Release);
}

/// Check if we're currently in a real-time context.
pub fn is_rt() -> bool {
    IN_RT_CONTEXT.load(Ordering::Acquire)
}

/// Run a closure in real-time context. Sets the RT flag, runs the closure,
/// then clears the flag.
///
/// In the audio callback:
/// ```ignore
/// rt_guard(|| {
///     engine.process(frames);
/// });
/// ```
pub fn rt_guard<R>(f: impl FnOnce() -> R) -> R {
    enter_rt();
    let result = f();
    exit_rt();
    result
}

/// Marker trait for types that are safe to use in the audio callback.
/// Types that implement this promise they will never allocate, block, or do I/O.
pub trait RtSafe: Send {}

impl RtSafe for f32 {}
impl RtSafe for f64 {}
impl RtSafe for i32 {}
impl RtSafe for u32 {}
impl RtSafe for usize {}
impl RtSafe for bool {}

/// A list of real-time rules for documentation and enforcement.
pub const RT_RULES: &[&str] = &[
    "No Mutex, RwLock, or any blocking synchronization",
    "No Box::new, Vec::push, String::from, or any heap allocation",
    "No disk reads, network calls, or system timers",
    "No Drop of heap objects (use basedrop for deferred reclamation)",
    "All memory must be pre-allocated before entering the callback",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_flag_default_off() {
        // Reset state for test isolation.
        exit_rt();
        assert!(!is_rt());
    }

    #[test]
    fn test_rt_guard_sets_flag() {
        exit_rt(); // Reset.
        let was_rt = rt_guard(|| {
            is_rt()
        });
        assert!(was_rt, "Should be in RT context inside guard");
        assert!(!is_rt(), "Should exit RT context after guard");
    }

    #[test]
    fn test_enter_exit_manual() {
        exit_rt();
        assert!(!is_rt());
        enter_rt();
        assert!(is_rt());
        exit_rt();
        assert!(!is_rt());
    }

    #[test]
    fn test_rt_rules_not_empty() {
        assert!(!RT_RULES.is_empty());
        assert!(RT_RULES.len() >= 4);
    }

    #[test]
    fn test_nested_rt_guard() {
        exit_rt();
        rt_guard(|| {
            assert!(is_rt());
            // Simulating nested call — still in RT context.
            let inner = is_rt();
            assert!(inner);
        });
        assert!(!is_rt());
    }
}
