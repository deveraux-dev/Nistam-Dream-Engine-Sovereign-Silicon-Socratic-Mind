// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! ADR-0026 Zero-Retention Sovereign Data Vault & 256-Bit Zeroize Sweeper.
//!
//! Enforces immediate memory wiping across all intermediate staging, visual, and audio registers
//! past their simulation tick deadlines to guarantee zero cloud and zero host retention.

#![deny(unsafe_code)]

use core::sync::atomic::{compiler_fence, Ordering};

/// Sweep chunk width in bytes (32 = 256 bits). Unroll factor, not an instruction width.
pub const SIMD_256_BYTES: usize = 32;

/// Transient Staging Register Buffer capacity.
pub const STAGING_VAULT_CAPACITY: usize = 256;

/// Zero-Retention Sovereign Data Vault.
#[derive(Debug, PartialEq, Eq)]
pub struct ZeroRetentionVault {
    /// Transient intermediate byte registers.
    pub staging_registers: [u8; STAGING_VAULT_CAPACITY],
    /// Simulation tick expiration deadline.
    pub expiration_tick: u64,
    /// Whether the vault is currently active and holding valid transient data.
    pub is_active: bool,
}

impl Default for ZeroRetentionVault {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroRetentionVault {
    /// Initialize a clean zeroed vault.
    pub const fn new() -> Self {
        Self {
            staging_registers: [0u8; STAGING_VAULT_CAPACITY],
            expiration_tick: 0,
            is_active: false,
        }
    }

    /// Store transient intermediate data with a strict simulation tick deadline.
    pub fn stage_transient_data(&mut self, data: &[u8], current_tick: u64, ttl_ticks: u64) -> bool {
        if data.len() > STAGING_VAULT_CAPACITY {
            return false;
        }

        // Copy transient data
        for (i, &b) in data.iter().enumerate() {
            self.staging_registers[i] = b;
        }

        self.expiration_tick = current_tick.wrapping_add(ttl_ticks);
        self.is_active = true;
        true
    }

    /// Check simulation tick deadline and apply zeroize sweep if expired.
    pub fn sweep_if_expired(&mut self, current_tick: u64) -> bool {
        if self.is_active && current_tick >= self.expiration_tick {
            self.zeroize();
            true
        } else {
            false
        }
    }

    /// Zeroize all staging registers, then fence (ADR-0026).
    pub fn zeroize(&mut self) {
        zeroize_256_slice(&mut self.staging_registers);
        self.expiration_tick = 0;
        self.is_active = false;
        compiler_fence(Ordering::SeqCst);
    }
}

/// Zeroize a byte slice in 32-byte unrolled chunks. Scalar stores; the caller's
/// `compiler_fence(SeqCst)` is what holds them against reordering.
#[inline]
pub fn zeroize_256_slice(slice: &mut [u8]) {
    let len = slice.len();
    let mut i = 0;

    // 256-bit (32-byte) unrolled sweeps
    while i + SIMD_256_BYTES <= len {
        let chunk = &mut slice[i..i + SIMD_256_BYTES];
        for b in chunk.iter_mut() {
            *b = 0;
        }
        i += SIMD_256_BYTES;
    }

    // Remainder sweep
    while i < len {
        slice[i] = 0;
        i += 1;
    }

    compiler_fence(Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeroize_256_slice() {
        let mut buffer = [0xAAu8; 128];
        zeroize_256_slice(&mut buffer);
        for &b in buffer.iter() {
            assert_eq!(b, 0);
        }
    }

    #[test]
    fn test_zero_retention_vault_ttl_expiration() {
        let mut vault = ZeroRetentionVault::new();
        let payload = [0x55u8; 64];

        assert!(vault.stage_transient_data(&payload, 100, 10));
        assert!(vault.is_active);
        assert_eq!(vault.staging_registers[0], 0x55);

        // Tick 105: not expired
        assert!(!vault.sweep_if_expired(105));
        assert!(vault.is_active);

        // Tick 110: expired -> swept
        assert!(vault.sweep_if_expired(110));
        assert!(!vault.is_active);
        assert_eq!(vault.staging_registers[0], 0);
    }

    #[test]
    fn test_manual_zeroize_sweep() {
        let mut vault = ZeroRetentionVault::new();
        vault.staging_registers[10] = 0xFF;
        vault.is_active = true;

        vault.zeroize();
        assert!(!vault.is_active);
        assert_eq!(vault.staging_registers[10], 0);
    }

    #[test]
    fn test_zero_retention_capacity_overflow() {
        let mut vault = ZeroRetentionVault::new();
        let big_payload = [0x11u8; STAGING_VAULT_CAPACITY + 1];
        assert!(!vault.stage_transient_data(&big_payload, 0, 10));
    }

    #[test]
    fn test_unexpired_sweep_returns_false() {
        let mut vault = ZeroRetentionVault::new();
        assert!(!vault.sweep_if_expired(100));
    }
}
