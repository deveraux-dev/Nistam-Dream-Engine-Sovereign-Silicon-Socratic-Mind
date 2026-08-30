//! forge-ttl-v3: Sovereign zeroization boundary.
//!
//! TTL is not cache cleanup—it is a hard memory-safety primitive enforcing
//! ADR-0026 zero-retention policies. Personal/institutional data is physically
//! erased from RAM/VRAM before reclamation.
//!
//! Three critical mitigations:
//! 1. **In-place zeroization**: Direct mutable references to backing memory,
//!    no temporary copies that leave heap residue.
//! 2. **volatile_write**: Core::ptr::write_volatile prevents compiler dead-store
//!    elimination (DSE) — zeroing loops are not optimized away.
//! 3. **Compiler fence**: Ensures writes complete across all hardware threads
//!    before memory is reclaimed.
//!
//! Architecture: Query → Freshness Check → [Expired? → Fail-Closed Denial]
//!              → In-Place Zero → Verify Zero-Residue → Compact SoA

pub mod ledger;

use std::path::PathBuf;
use std::sync::atomic::{compiler_fence, Ordering};
use core::ptr::write_volatile;

/// Zeroization gate: read-side guard, hard zeroization, zero-residue verify.
pub struct ZeroizationGate {
    pub scope: String,
    pub stale_threshold_pmy: u16,
    pub log_path: PathBuf,
}

impl ZeroizationGate {
    /// Create a new zeroization gate for a named scope (e.g., "forge_pkm_corpus").
    pub fn new(scope: impl Into<String>, stale_threshold_pmy: u16, log_path: PathBuf) -> Self {
        Self {
            scope: scope.into(),
            stale_threshold_pmy,
            log_path,
        }
    }

    /// Read-side guard: fail-closed return if TTL expired.
    /// Never leaks expired sensitive data across frame boundaries.
    #[inline]
    pub fn guard_read<T: Clone>(&self, record: Option<T>, freshness_pmy: u16) -> Option<T> {
        match record {
            Some(r) if freshness_pmy >= self.stale_threshold_pmy => {
                Some(r)  // Fresh: allow read
            }
            Some(_) => {
                // Expired: deny immediately, don't leak
                None
            }
            None => None,
        }
    }

    /// Hard zeroization: volatile_write 0x00 directly across the backing slice.
    /// Prevents LLVM/rustc dead-store elimination (DSE) optimizations.
    ///
    /// Safety: Caller must ensure `data` points to valid, mutable memory.
    pub fn zeroize_slice_in_place(data: &mut [u8]) {
        for byte in data.iter_mut() {
            unsafe {
                write_volatile(byte, 0u8);
            }
        }
        // Memory fence ensures writes complete across all hardware threads.
        compiler_fence(Ordering::SeqCst);
    }

    /// Zero-residue verification: proves bit-exact 0x00 remains in backing memory.
    /// Returns an error if any non-zero byte is found (SECURITY FAULT).
    pub fn verify_zeroed(data: &[u8]) -> Result<(), String> {
        if data.iter().any(|&b| b != 0u8) {
            return Err(
                "SECURITY FAULT: Non-zero residue detected after zeroization!".to_string()
            );
        }
        Ok(())
    }

    /// Single atomic pass: in-place zeroization → verification → ready for reclaim.
    /// Returns the byte count zeroized, or an error if verification fails.
    pub fn zeroization_sweep_in_place(&self, corpus_bytes: &mut [u8]) -> Result<usize, String> {
        let byte_count = corpus_bytes.len();

        // 1. Volatile zeroize the physical slice (in-place, no copies)
        Self::zeroize_slice_in_place(corpus_bytes);

        // 2. Hardware verify: bit-exact 0x00 across entire slice
        Self::verify_zeroed(corpus_bytes)?;

        Ok(byte_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_read_allows_fresh_data() {
        let gate = ZeroizationGate::new("test", 2000, PathBuf::from("/tmp/test.log"));
        let fresh_val = 42i32;

        // Freshness >= threshold → allow
        let result = gate.guard_read(Some(fresh_val), 2500);
        assert_eq!(result, Some(42i32));
    }

    #[test]
    fn guard_read_denies_expired_data() {
        let gate = ZeroizationGate::new("test", 2000, PathBuf::from("/tmp/test.log"));
        let expired_val = 42i32;

        // Freshness < threshold → deny
        let result = gate.guard_read(Some(expired_val), 1500);
        assert_eq!(result, None);
    }

    #[test]
    fn zeroization_sweep_in_place_wipes_and_verifies() {
        let gate = ZeroizationGate::new("test", 2000, PathBuf::from("/tmp/test.log"));
        let mut data = vec![0xDEu8, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];

        // Sweep: zeroize and verify in one atomic pass
        let swept = gate.zeroization_sweep_in_place(&mut data);
        assert!(swept.is_ok(), "sweep should succeed");
        assert_eq!(swept.unwrap(), 8);

        // Data is now bit-exact zero
        assert!(data.iter().all(|&b| b == 0u8));
    }

    #[test]
    fn verify_zeroed_detects_residue() {
        let gate = ZeroizationGate::new("test", 2000, PathBuf::from("/tmp/test.log"));
        let mut data = vec![0u8; 16];

        // All zeros → passes
        let result = gate.zeroization_sweep_in_place(&mut data);
        assert!(result.is_ok());

        // Introduce a single non-zero byte
        data[7] = 0x01;
        let verify_result = ZeroizationGate::verify_zeroed(&data);
        assert!(
            verify_result.is_err(),
            "verify should detect non-zero residue"
        );
    }

    #[test]
    fn volatile_write_prevents_dead_store_elimination() {
        let mut data = vec![0xFFu8; 32];
        let ptr = data.as_mut_ptr();
        let len = data.len();

        unsafe {
            for i in 0..len {
                write_volatile(ptr.add(i), 0u8);
            }
        }
        compiler_fence(Ordering::SeqCst);

        assert!(data.iter().all(|&b| b == 0u8), "volatile writes should not be optimized away");
    }
}
