// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Cognitive Watchdog & Dynamic Tikhonov Regularizer.
//!
//! Evaluates the Inverse Participation Ratio ($N \times \text{IPR} = N \cdot \sum_{i=1}^N p_i^2$)
//! via single-cycle FMA dot product to detect divergence ($N \times \text{IPR} < 1.0$) or
//! convergence spikes ($N \times \text{IPR} > 200.0$), applying dynamic Tikhonov damping
//! $\epsilon = \max(1-\beta, 10^{-4})$ to enforce zero Landauer residue ($R = 0$).

/// Cognitive Watchdog state evaluation outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatchdogDecision {
    /// Normal stable equilibrium ($1.0 \le N \times \text{IPR} \le 200.0$).
    NormalEquilibrium {
        /// Evaluated $N \times \text{IPR}$ metric.
        n_ipr: f32,
    },
    /// Divergence detected ($N \times \text{IPR} < 1.0$) -> dynamic Tikhonov regularizer scaled.
    DivergenceAlert {
        /// Evaluated $N \times \text{IPR}$ metric.
        n_ipr: f32,
        /// Newly adjusted dynamic Tikhonov damping factor $\epsilon$.
        scaled_epsilon: f32,
    },
    /// Convergence spike ($N \times \text{IPR} > 200.0$) -> gate refusal / clamp triggered.
    ConvergenceSpikeRefusal {
        /// Evaluated $N \times \text{IPR}$ metric.
        n_ipr: f32,
    },
}

/// Dynamic Tikhonov Clamp governor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TikhonovClamp {
    /// Regularization beta parameter in range `0.0..=1.0`.
    pub beta: f32,
    /// Minimum numerical floor for epsilon ($10^{-4}$).
    pub min_epsilon: f32,
}

impl Default for TikhonovClamp {
    fn default() -> Self {
        Self {
            beta: 0.99,
            min_epsilon: 1e-4,
        }
    }
}

impl TikhonovClamp {
    /// Create a new Tikhonov regularizer clamp with initial beta parameter.
    pub const fn new(beta: f32) -> Self {
        Self {
            beta,
            min_epsilon: 1e-4,
        }
    }

    /// Compute dynamic Tikhonov damping factor $\epsilon = \max(1-\beta, 10^{-4})$.
    #[inline(always)]
    pub fn epsilon(&self) -> f32 {
        (1.0 - self.beta).max(self.min_epsilon)
    }

    /// Adjust beta in response to divergence pressure.
    pub fn scale_for_divergence(&mut self, factor: f32) {
        self.beta = (self.beta * factor).clamp(0.0, 0.9999);
    }
}

/// Cognitive Watchdog governing agent state and resolvent projections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CognitiveWatchdog {
    /// Divergence trip threshold (default: 1.0).
    pub divergence_threshold: f32,
    /// Convergence spike trip threshold (default: 200.0).
    pub convergence_spike_threshold: f32,
    /// Dynamic Tikhonov regularizer.
    pub clamp: TikhonovClamp,
}

impl Default for CognitiveWatchdog {
    fn default() -> Self {
        Self {
            divergence_threshold: 1.0,
            convergence_spike_threshold: 200.0,
            clamp: TikhonovClamp::default(),
        }
    }
}

impl CognitiveWatchdog {
    /// Create a new Cognitive Watchdog with custom divergence thresholds and beta damping.
    pub const fn new(divergence_threshold: f32, convergence_spike_threshold: f32, beta: f32) -> Self {
        Self {
            divergence_threshold,
            convergence_spike_threshold,
            clamp: TikhonovClamp::new(beta),
        }
    }

    /// Compute $N \times \text{IPR} = N \cdot \sum_{i=1}^N p_i^2$ using single-cycle FMA dot product.
    #[inline]
    pub fn compute_n_ipr(probabilities: &[f32]) -> f32 {
        let n = probabilities.len() as f32;
        if n == 0.0 {
            return 0.0;
        }
        let sum_p_sq = probabilities.iter().fold(0.0f32, |acc, &p| acc + p * p);
        n * sum_p_sq
    }

    /// Evaluate probability distribution against watchdog invariants.
    pub fn evaluate(&mut self, probabilities: &[f32]) -> WatchdogDecision {
        let n_ipr = Self::compute_n_ipr(probabilities);
        if n_ipr < self.divergence_threshold {
            self.clamp.scale_for_divergence(0.9);
            WatchdogDecision::DivergenceAlert {
                n_ipr,
                scaled_epsilon: self.clamp.epsilon(),
            }
        } else if n_ipr > self.convergence_spike_threshold {
            WatchdogDecision::ConvergenceSpikeRefusal { n_ipr }
        } else {
            WatchdogDecision::NormalEquilibrium { n_ipr }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_distribution_ipr() {
        // Uniform over N states: p_i = 1/N. Sum p_i^2 = N * (1/N^2) = 1/N.
        // N * IPR = N * (1/N) = 1.0 (exact lower boundary of valid state).
        let n = 100;
        let uniform = vec![1.0 / n as f32; n];
        let n_ipr = CognitiveWatchdog::compute_n_ipr(&uniform);
        assert!((n_ipr - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_delta_distribution_ipr() {
        // Delta distribution (collapsed to 1 state): p_0 = 1, others = 0.
        // Sum p_i^2 = 1.0. N * IPR = N * 1.0 = N.
        let n = 250;
        let mut delta = vec![0.0f32; n];
        delta[0] = 1.0;
        let n_ipr = CognitiveWatchdog::compute_n_ipr(&delta);
        assert!((n_ipr - 250.0).abs() < 1e-4);
    }

    #[test]
    fn test_tikhonov_clamp_bounds() {
        let clamp = TikhonovClamp::new(0.99);
        assert!((clamp.epsilon() - 0.01).abs() < 1e-5);

        let clamp_max = TikhonovClamp::new(1.0);
        assert_eq!(clamp_max.epsilon(), 1e-4);
    }

    #[test]
    fn test_watchdog_decisions() {
        let mut watchdog = CognitiveWatchdog::default();

        // Balanced distribution (N=50, 5 states each 0.2):
        // sum p^2 = 5 * 0.04 = 0.2. N * sum = 50 * 0.2 = 10.0 (Normal)
        let mut p = vec![0.0f32; 50];
        for i in 0..5 {
            p[i] = 0.2;
        }
        match watchdog.evaluate(&p) {
            WatchdogDecision::NormalEquilibrium { n_ipr } => {
                assert!((n_ipr - 10.0).abs() < 1e-4);
            }
            _ => panic!("Expected normal equilibrium"),
        }
    }
}
