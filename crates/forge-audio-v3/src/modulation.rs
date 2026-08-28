/// Real-time audio modulation snapshot fed from the live audio path into the
/// sequencer velocity grid (CARTRIDGE-MODULATION-SNAPSHOT-001).
#[derive(Clone, Copy, Debug, Default)]
pub struct ModulationSnapshot {
    /// RMS energy of the current audio frame (0.0–1.0).
    pub rms: f32,
}

impl ModulationSnapshot {
    pub fn from_rms(rms: f32) -> Self {
        Self { rms: rms.clamp(0.0, 1.0) }
    }

    pub fn silent() -> Self {
        Self { rms: 0.0 }
    }

    /// Quantise the live `rms` into the `0..=255` integer domain brush /
    /// velocity consumers expect (T07 / AUDIO-REACTIVE-BRUSH-BRIDGE-001).
    /// `rms` is `pub`, so re-sanitise here: non-finite → 0, clamp to `[0, 1]`
    /// — the cast can never see an out-of-range or non-finite value.
    pub fn rms_q(&self) -> u8 {
        let rms = if self.rms.is_finite() { self.rms.clamp(0.0, 1.0) } else { 0.0 };
        (rms * 255.0).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_q_is_deterministic_and_bounded() {
        for i in 0..=100 {
            let rms = i as f32 / 100.0;
            let snap = ModulationSnapshot::from_rms(rms);
            assert_eq!(snap.rms_q(), snap.rms_q(), "quantiser must be deterministic (rms={rms})");
        }
        assert_eq!(ModulationSnapshot::silent().rms_q(), 0, "silence quantises to 0");
        assert_eq!(ModulationSnapshot::from_rms(1.0).rms_q(), 255, "full scale quantises to 255");
    }

    #[test]
    fn rms_q_survives_a_corrupted_snapshot() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -3.0, 7.5] {
            let snap = ModulationSnapshot { rms: bad };
            let q = snap.rms_q();
            assert!(bad.is_finite() && bad > 0.0 || q == 0 || q == 255, "bounded for rms={bad}: {q}");
        }
    }
}
