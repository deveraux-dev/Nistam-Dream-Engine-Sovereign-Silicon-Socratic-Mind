//! CPU-only training context (GPU dispatch lane deliberately scoped out).
//!
//! This crate intentionally carries NO GPU support, wgpu, or GPU-dispatch features. The v2
//! `forge-ml` crate conditionally compiled a wgpu-backed GPU lane via `#[cfg(feature = "wgpu-dispatch")]`;
//! this v3 port permanently takes the CPU-only stub path for scope/maintenance reasons (C09 aperture).
//!
//! The public boundary (`shared()`, `has_gpu()`, `matmul()`) is deliberately preserved so that if a
//! GPU lane is wired back in the future, call sites in `moe_train.rs` never need to change — only
//! this module's internals. `has_gpu()` always returns `false`, and `matmul()` always delegates to
//! `cpu_matmul()`.

use std::sync::OnceLock;

/// GPU training context (stub). Lazily initialized, reused across distill batches.
/// This version is CPU-only; the wgpu lane was scope-cut from this port.
pub struct GpuTrainContext;

/// Test serial guard: tests that ride a shared device serialize here.
/// Parallel dispatch over a single GPU device raced 7 tests in v2; same cure
/// applied here (always serializes tests for consistency).
#[cfg(test)]
pub(crate) fn test_serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// Process-wide training context — ONE logical device; created once and reused.
pub fn shared() -> &'static GpuTrainContext {
    static CTX: OnceLock<GpuTrainContext> = OnceLock::new();
    CTX.get_or_init(GpuTrainContext::new)
}

impl GpuTrainContext {
    /// Initialize training context (CPU-only stub).
    pub fn new() -> Self {
        Self
    }

    /// Whether GPU matmul is available. Always returns `false` in this CPU-only build.
    pub fn has_gpu(&self) -> bool {
        false
    }

    /// Dispatch matmul to GPU or CPU. Falls back to `cpu_matmul()` since GPU is unavailable.
    ///
    /// A[M×K] @ B[K×N] → C[M×N].
    pub fn matmul(&self, a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
        cpu_matmul(a, b, m, n, k)
    }
}

/// CPU matmul fallback: A[M×K] @ B[K×N] → C[M×N], row-major.
pub fn cpu_matmul(a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f32;
            for p in 0..k {
                sum += a[i * k + p] * b[p * n + j];
            }
            c[i * n + j] = sum;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_matmul_identity() {
        // 2x2 identity @ [1,2,3,4] = [1,2,3,4]
        let a = vec![1.0, 0.0, 0.0, 1.0];
        let b = vec![1.0, 2.0, 3.0, 4.0];
        let c = cpu_matmul(&a, &b, 2, 2, 2);
        assert_eq!(c, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn gpu_train_context_initializes() {
        let ctx = GpuTrainContext::new();
        // Should not panic regardless of GPU availability
        let a = vec![1.0f32; 4];
        let b = vec![1.0f32; 4];
        let c = ctx.matmul(&a, &b, 2, 2, 2);
        assert_eq!(c.len(), 4);
    }
}
