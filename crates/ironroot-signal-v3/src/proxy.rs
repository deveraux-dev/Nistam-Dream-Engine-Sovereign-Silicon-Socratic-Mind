#![allow(missing_docs)]
//! Async-facing signal proxy primitives.

use crate::filter::{clamp_q, ema_q, variance_q};
use crate::ids::SignalSourceId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SignalHealth {
    Missing,
    Stale,
    Noisy,
    Stable,
}

impl Default for SignalHealth {
    fn default() -> Self {
        Self::Missing
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RawSignalFrame {
    pub timestamp_us: u64,
    pub channels: [i32; 8],
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FilteredSignalFrame {
    pub tick_seen: u64,
    pub intensity_q: i16,
    pub variance_q: i16,
    pub drift_q: i16,
    pub pulse_q: i16,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SignalProxy {
    pub source_id: SignalSourceId,
    pub latest_raw: RawSignalFrame,
    pub filtered: FilteredSignalFrame,
    pub health: SignalHealth,
}

impl SignalProxy {
    pub const fn new(source_id: SignalSourceId) -> Self {
        Self {
            source_id,
            latest_raw: RawSignalFrame { timestamp_us: 0, channels: [0; 8] },
            filtered: FilteredSignalFrame { tick_seen: 0, intensity_q: 0, variance_q: 0, drift_q: 0, pulse_q: 0 },
            health: SignalHealth::Missing,
        }
    }

    /// Push a raw frame through a fixed-point smoothing lane.
    pub fn ingest(&mut self, tick_seen: u64, raw: RawSignalFrame, alpha_q: i32) -> FilteredSignalFrame {
        let prev = self.filtered;
        let intensity_sample = raw.channels[0].clamp(0, 10000);
        let intensity = ema_q(prev.intensity_q as i32, intensity_sample, alpha_q);
        let variance = variance_q(&raw.channels);
        let drift = (intensity - prev.intensity_q as i32).clamp(-10000, 10000) as i16;
        let pulse = clamp_q((raw.channels[1].abs() % 10001).clamp(0, 10000));

        self.latest_raw = raw;
        self.filtered = FilteredSignalFrame {
            tick_seen,
            intensity_q: clamp_q(intensity),
            variance_q: variance,
            drift_q: drift,
            pulse_q: pulse,
        };
        self.health = if variance > 8000 { SignalHealth::Noisy } else { SignalHealth::Stable };
        self.filtered
    }
}
