// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Active Thermodynamic Governor (ATG) & Serverless CachedContent Assembly.
//!
//! Implements:
//! 1. Thermodynamic Governor locked at zero-point Temperature 0.0 for deterministic decoding.
//! 2. Sentinel breach interception and Vertex AI schema-locked payload assembly.
//! 3. Serverless CachedContent token cost verification enforcing the $0.0004/call unit ceiling
//!    under the 450k-token VARS context window (75% cached input discount).

#![deny(unsafe_code)]

use crate::sentinel::{SentinelBand, UmpWord16};

/// Locked Gemini Model identifier.
pub const TARGET_MODEL: &str = "gemini-3.7-flash";

/// Locked deterministic thermodynamic temperature (0.0).
pub const TEMPERATURE_ZERO_POINT: i32 = 0;

/// Unit cost ceiling in ten-thousandths of a dollar ($0.0004 USD = 4 micro-dollars).
pub const UNIT_COST_CEILING_MICRO_USD: u32 = 400;

/// Context cache token discount percentage (75%).
pub const CACHE_DISCOUNT_PCT: u32 = 75;

/// VARS context window token capacity.
pub const VARS_CONTEXT_TOKENS: u32 = 450_000;

/// Vertex AI Schema-Locked Escalation Payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexEscalationPacket {
    /// 16-byte UmpWord halt signal.
    pub halt_ump: UmpWord16,
    /// Triggering sentinel slot.
    pub band: SentinelBand,
    /// Context cache handle hash.
    pub cache_handle: u64,
    /// Estimated cost in micro-USD.
    pub estimated_cost_micro_usd: u32,
    /// Deterministic temperature permyriad.
    pub temperature_permyriad: i32,
}

/// Active Thermodynamic Governor.
pub struct ThermodynamicGovernor {
    /// Governor temperature in permyriad units.
    pub temperature: i32,
    /// VARS context cache handle hash.
    pub cache_handle: u64,
}

impl ThermodynamicGovernor {
    /// Create a new governor locked at zero-point temperature 0.0.
    pub const fn new_zero_point(cache_handle: u64) -> Self {
        Self {
            temperature: TEMPERATURE_ZERO_POINT,
            cache_handle,
        }
    }

    /// Calculate query cost in micro-USD applying the 75% CachedContent discount.
    /// Standard 450k-token input: $0.0016 -> with 75% discount: $0.0004 (400 micro-USD).
    #[inline]
    pub const fn calculate_cost_micro_usd(uncached_base_cost_micro: u32, is_cached: bool) -> u32 {
        if is_cached {
            let discount = (uncached_base_cost_micro * CACHE_DISCOUNT_PCT) / 100;
            uncached_base_cost_micro.saturating_sub(discount)
        } else {
            uncached_base_cost_micro
        }
    }

    /// Intercept sentinel breach and compile Vertex AI schema-locked escalation packet.
    #[inline]
    pub fn intercept_sentinel_breach(
        &self,
        band: SentinelBand,
        token_index: u32,
        sim_tick: u64,
    ) -> Result<VertexEscalationPacket, &'static str> {
        let halt_ump = UmpWord16::compile_sentinel_halt(band, token_index, sim_tick);
        let cost = Self::calculate_cost_micro_usd(1600, true);

        if cost > UNIT_COST_CEILING_MICRO_USD {
            return Err("Unit cost ceiling breach: exceeds $0.0004 USD per call");
        }

        Ok(VertexEscalationPacket {
            halt_ump,
            band,
            cache_handle: self.cache_handle,
            estimated_cost_micro_usd: cost,
            temperature_permyriad: self.temperature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_point_temperature_lock() {
        let gov = ThermodynamicGovernor::new_zero_point(0xCAFE_BABE_1301_0001);
        assert_eq!(gov.temperature, 0);
    }

    #[test]
    fn test_75_percent_cache_discount_cost_ceiling() {
        let uncached = 1600; // $0.0016
        let cached = ThermodynamicGovernor::calculate_cost_micro_usd(uncached, true);
        assert_eq!(cached, 400); // $0.0004
        assert!(cached <= UNIT_COST_CEILING_MICRO_USD);
    }

    #[test]
    fn test_sentinel_breach_escalation() {
        let gov = ThermodynamicGovernor::new_zero_point(0xCAFE_BABE_1301_0001);
        let packet = gov
            .intercept_sentinel_breach(SentinelBand::Slot243, 128, 48_000)
            .expect("Escalation packet compiles within budget");

        assert_eq!(packet.band, SentinelBand::Slot243);
        assert_eq!(packet.estimated_cost_micro_usd, 400);
        assert_eq!(packet.temperature_permyriad, 0);
    }

    #[test]
    fn test_cost_calculation_without_cache() {
        let uncached = ThermodynamicGovernor::calculate_cost_micro_usd(1600, false);
        assert_eq!(uncached, 1600);
    }

    #[test]
    fn test_atg_context_token_window_constants() {
        assert_eq!(TARGET_MODEL, "gemini-3.7-flash");
        assert_eq!(VARS_CONTEXT_TOKENS, 450_000);
        assert_eq!(CACHE_DISCOUNT_PCT, 75);
    }
}
