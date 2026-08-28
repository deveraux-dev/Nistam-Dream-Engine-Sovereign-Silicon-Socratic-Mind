//! 50-Year Multi-Factor Physical Infrastructure Degradation Engine.
//!
//! Models the long-term structural health index and fiscal replacement liability
//! over a 50-year horizon (18,250 daily ticks or 50 annual epochs) incorporating:
//! 1. **Environmental / Weather Stress Multiplier:** Freeze-thaw cycles, chloride ingress, thermal extremes.
//! 2. **Macroeconomic Inflation:** Construction material & skilled labor cost compounding.
//! 3. **Government Cutbacks & Budget Deferral:** Maintenance debt accumulation & super-exponential decay.
//! 4. **Skilled Trades Deficit & Rework Cascade:** Installation error rates, latent defect escaping, and rework fatigue.
//!
//! Implemented strictly in deterministic, `#![no_std]` integer/fixed-point arithmetic (basis points: 10,000 = 100.00%).

/// Parametric inputs configuring the 50-year infrastructure operating environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DegradationEnvironment {
    /// Baseline initial structural condition index (0..10,000, where 10,000 = 100.0% pristine new construction).
    pub initial_condition_bps: u32,
    /// Annual compounding construction material & labor inflation in basis points (e.g., 350 = 3.50% / yr).
    pub annual_inflation_bps: u32,
    /// Annual freeze-thaw severity & chloride chemical stress multiplier (100 = nominal, 300 = severe Canadian Arctic).
    pub climate_severity_factor: u32,
    /// Government budget deferral factor (100 = fully funded maintenance, 250 = severe municipal austerity / 4-yr freeze).
    pub government_deferral_factor: u32,
    /// Skilled trades deficit in basis points (e.g., 4,000 = 40% deficit of NACE/CWB certified journeymen).
    pub skilled_trades_deficit_bps: u32,
    /// Rework multiplier on defective installations (e.g., 230 = 2.3x cost and substrate fatigue on re-application).
    pub rework_penalty_factor: u32,
}

impl Default for DegradationEnvironment {
    fn default() -> Self {
        Self {
            initial_condition_bps: 10_000,     // 100.00%
            annual_inflation_bps: 385,          // 3.85% annual inflation
            climate_severity_factor: 180,       // Northern freeze-thaw climate
            government_deferral_factor: 150,    // Moderate maintenance deferral
            skilled_trades_deficit_bps: 3500,   // 35% certified trade deficit
            rework_penalty_factor: 230,         // 2.3x rework penalty
        }
    }
}

/// The computed structural and financial state of an asset at year `t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearAuditRecord {
    /// Operating year (0..50).
    pub year: u32,
    /// Structural Condition Index (0..10,000, where 10,000 = 100.00%).
    pub condition_bps: u32,
    /// Escalated replacement cost multiplier in basis points (10,000 = 1.00x baseline capital cost).
    pub replacement_cost_multiplier_bps: u32,
    /// Cumulative maintenance debt in basis points relative to baseline cost.
    pub accumulated_maintenance_debt_bps: u32,
    /// Latent defect risk index (0..100).
    pub latent_defect_risk_pct: u32,
    /// The generated Sieve-13 (S13) balanced-ternary vector for this annual state.
    pub s13_token: [i8; 13],
}

impl DegradationEnvironment {
    /// Computes the condition and fiscal liability progression for year `year` (0..50).
    pub fn evaluate_year(&self, year: u32) -> YearAuditRecord {
        if year == 0 {
            return YearAuditRecord {
                year: 0,
                condition_bps: self.initial_condition_bps,
                replacement_cost_multiplier_bps: 10_000,
                accumulated_maintenance_debt_bps: 0,
                latent_defect_risk_pct: 0,
                s13_token: [0i8; 13],
            };
        }

        // 1. Compound Macroeconomic Inflation: (1 + r)^t
        let mut infl_mult: u64 = 10_000;
        for _ in 0..year {
            infl_mult = (infl_mult * (10_000 + self.annual_inflation_bps as u64)) / 10_000;
        }

        // 2. Multi-Factor Degradation Integration
        let base_annual_decay_bps: u64 = 60; // 0.60% base wear per year
        let climate_accel = self.climate_severity_factor as u64; // 100 = 1.0x
        let gov_accel = self.government_deferral_factor as u64;   // 100 = 1.0x
        let labor_rework_multiplier = 10_000 + ((self.skilled_trades_deficit_bps as u64 * self.rework_penalty_factor as u64) / 100);

        let mut current_condition: u64 = self.initial_condition_bps as u64;

        for y in 1..=year {
            // Time acceleration if maintenance is deferred
            let time_accel = if y > 15 && self.government_deferral_factor > 100 {
                100 + ((y as u64 - 15) * 8)
            } else {
                100
            };

            let annual_loss = (base_annual_decay_bps * climate_accel * gov_accel * labor_rework_multiplier * time_accel)
                / (100 * 100 * 10_000 * 100);

            current_condition = current_condition.saturating_sub(annual_loss);
        }

        let condition_bps = current_condition as u32;

        // 3. Maintenance Debt Avalanche
        // Unfunded maintenance compounds at capital cost + inflation
        let maintenance_debt_bps = if self.government_deferral_factor > 100 {
            let deferral_excess = (self.government_deferral_factor - 100) as u64;
            let debt_raw = (deferral_excess * year as u64 * 350 * infl_mult) / (100 * 10_000);
            debt_raw.min(1_000_000) as u32
        } else {
            0
        };

        // 4. Latent Defect Risk
        let latent_risk = (((self.skilled_trades_deficit_bps as u64 * year as u64) / 500)
            .min(100)) as u32;

        // 5. Synthesize S13 Balanced-Ternary State Vector
        // Lanes 0..3: Physical condition
        // Lanes 4..6: Environmental stress
        // Lanes 7..9: Fiscal / Inflation / Deferral
        // Lanes 10..12: Labor quality & Rework
        let mut s13 = [0i8; 13];

        // Physical condition trits
        if condition_bps < 3_000 {
            s13[0] = 1; s13[1] = 1; s13[2] = 1; s13[3] = 1; // Critical structural loss
        } else if condition_bps < 6_500 {
            s13[0] = 1; s13[1] = 0; s13[2] = 1; s13[3] = 0; // Moderate surface/rebar wear
        }

        // Climate stress trits
        if self.climate_severity_factor > 200 {
            s13[4] = 1; s13[5] = 1; s13[6] = 1; // Severe freeze-thaw & chloride
        } else if self.climate_severity_factor > 120 {
            s13[4] = 1; s13[5] = 0; s13[6] = 0;
        }

        // Fiscal & Deferral trits
        if self.government_deferral_factor > 180 || infl_mult > 35_000 {
            s13[7] = 1; s13[8] = 1; s13[9] = 1; // Severe capital debt & inflation surge
        } else if self.government_deferral_factor > 120 {
            s13[7] = 1; s13[8] = 0; s13[9] = 0;
        }

        // Skilled trade & rework trits
        if self.skilled_trades_deficit_bps > 4_500 || latent_risk > 60 {
            s13[10] = 1; s13[11] = 1; s13[12] = 1; // High rework cascade & un-attested defects
        } else if self.skilled_trades_deficit_bps > 2_500 {
            s13[10] = 1; s13[11] = 0; s13[12] = 0;
        }

        YearAuditRecord {
            year,
            condition_bps,
            replacement_cost_multiplier_bps: infl_mult as u32,
            accumulated_maintenance_debt_bps: maintenance_debt_bps,
            latent_defect_risk_pct: latent_risk,
            s13_token: s13,
        }
    }

    /// Simulates the entire 50-year lifecycle into an array of 51 yearly records.
    pub fn simulate_50_years(&self) -> [YearAuditRecord; 51] {
        let mut results = [YearAuditRecord {
            year: 0,
            condition_bps: 0,
            replacement_cost_multiplier_bps: 0,
            accumulated_maintenance_debt_bps: 0,
            latent_defect_risk_pct: 0,
            s13_token: [0i8; 13],
        }; 51];

        for yr in 0..=50 {
            results[yr as usize] = self.evaluate_year(yr);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weaver::{ArbitrationVerdict, WeaverArbiter};
    use crate::{Disposition, EvidenceChain};

    #[test]
    fn test_50_year_nominal_vs_austerity_degradation() {
        let nominal = DegradationEnvironment {
            initial_condition_bps: 10_000,
            annual_inflation_bps: 250,       // 2.5%
            climate_severity_factor: 100,    // Nominal
            government_deferral_factor: 100, // Fully funded
            skilled_trades_deficit_bps: 1000,// 10% deficit
            rework_penalty_factor: 120,      // Low rework
        };

        let austerity = DegradationEnvironment {
            initial_condition_bps: 10_000,
            annual_inflation_bps: 550,       // 5.5% inflation
            climate_severity_factor: 250,    // Severe freeze-thaw
            government_deferral_factor: 220, // Severe municipal cuts
            skilled_trades_deficit_bps: 5000,// 50% skilled labor deficit
            rework_penalty_factor: 280,      // 2.8x rework penalty
        };

        let nom_50 = nominal.evaluate_year(50);
        let aus_50 = austerity.evaluate_year(50);

        // Nominal environment maintains integrity significantly better
        assert!(nom_50.condition_bps > aus_50.condition_bps);
        // Austerity suffers astronomical replacement and maintenance debt
        assert!(aus_50.accumulated_maintenance_debt_bps > nom_50.accumulated_maintenance_debt_bps);

        // Evaluate state transitions with Weaver Arbiter
        let mut chain = EvidenceChain::new();
        chain.append(18250, Disposition::Expired);

        let verdict_nom = WeaverArbiter::arbitrate(&chain, &nom_50.s13_token);
        let verdict_aus = WeaverArbiter::arbitrate(&chain, &aus_50.s13_token);

        assert_eq!(verdict_aus, ArbitrationVerdict::CriticalEscalation);
        assert!(matches!(verdict_nom, ArbitrationVerdict::ScheduledMaintenance | ArbitrationVerdict::StructuralEquilibrium));
    }
}
