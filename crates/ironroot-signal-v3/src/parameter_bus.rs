#![allow(missing_docs)]
//! Non-authoritative parameter bus for presentation and creation surfaces.

use crate::filter::clamp_q;
use crate::proxy::FilteredSignalFrame;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WorldParameterBus {
    pub calm_q: i16,
    pub tension_q: i16,
    pub variance_q: i16,
    pub pulse_phase_q: i16,
    pub metronome_phase_q: i16,
}

impl WorldParameterBus {
    pub const fn new() -> Self {
        Self { calm_q: 10000, tension_q: 0, variance_q: 0, pulse_phase_q: 0, metronome_phase_q: 0 }
    }

    pub fn from_signal(signal: FilteredSignalFrame, metronome_phase_q: i16) -> Self {
        let tension = signal.intensity_q as i32;
        Self {
            calm_q: clamp_q(10000 - tension),
            tension_q: clamp_q(tension),
            variance_q: signal.variance_q,
            pulse_phase_q: signal.pulse_q,
            metronome_phase_q,
        }
    }
}
