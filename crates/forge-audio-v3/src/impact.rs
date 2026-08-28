//! Impact voice + rumble — SAME material registry row the shader and the
//! collision atom read. Look, physics and acoustics cite one row, never
//! three tables (seam: material.truth / every.act.sounds).

use std::fmt;

/// Permyriad fixed-point: 10_000 = 1.0 (engine-wide integer unit).
pub type Permyriad = i32;

/// Minimal material-registry row contract. The real registry row implements
/// this — forge-audio borrows the columns it needs, it never clones a rival
/// table.
pub trait MaterialRow {
    /// Stable material identity, shared with the shader/atom row.
    fn material_id(&self) -> u32;
    /// Mass column, permyriad-scaled (10_000 = 1.0).
    fn mass_pmy(&self) -> Permyriad;
    /// Friction column, permyriad-scaled (10_000 = 1.0).
    fn friction_pmy(&self) -> Permyriad;
}

/// A single collision impact — same id space the shader/atom emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactEvent {
    pub material_id: u32,
    pub velocity_pmy: Permyriad,
    pub at_ns: u64,
}

/// Typed lookup failure — a missing row is never silently defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialRowMissing(pub u32);

impl fmt::Display for MaterialRowMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no material row for material_id {}", self.0)
    }
}

impl std::error::Error for MaterialRowMissing {}

/// Finds the row for `id` in the registry slice, or a typed error.
pub fn find_row<'a>(
    id: u32,
    rows: &'a [&'a dyn MaterialRow],
) -> Result<&'a dyn MaterialRow, MaterialRowMissing> {
    rows.iter()
        .copied()
        .find(|r| r.material_id() == id)
        .ok_or(MaterialRowMissing(id))
}

/// The audible voice of one impact: pitch/gain/decay/timbre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactVoice {
    pub pitch_hz: f32,
    pub gain_pmy: Permyriad,
    pub decay_ms: u32,
    pub timbre: Permyriad,
}

/// Voices one impact from its event and the SAME material row the
/// shader/atom read. Heavier mass -> lower pitch; zero velocity -> silent.
pub fn voice(ev: &ImpactEvent, row: &dyn MaterialRow) -> ImpactVoice {
    let mass = (row.mass_pmy().max(1) as f32) / 10_000.0;
    let friction = (row.friction_pmy().max(0) as f32) / 10_000.0;
    let vel = (ev.velocity_pmy.max(0) as f32) / 10_000.0;

    let pitch_hz = (2_000.0 / mass.sqrt().max(0.01)).clamp(20.0, 8_000.0);
    let gain_pmy = (vel * 10_000.0).clamp(0.0, 10_000.0) as Permyriad;
    let decay_ms = (80.0 + mass * 400.0).clamp(20.0, 4_000.0) as u32;
    let timbre = ((friction * 10_000.0) as Permyriad).clamp(0, 10_000);

    ImpactVoice { pitch_hz, gain_pmy, decay_ms, timbre }
}

/// Low-frequency device rumble curve — the SAME event and row `voice` used,
/// never a second dispatch. Emits data only; the device lane consumes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RumbleCurve {
    pub lo_pmy: Permyriad,
    pub hi_pmy: Permyriad,
    pub ms: u32,
}

/// Derives the rumble curve from the same impact event + row `voice` used.
pub fn rumble(ev: &ImpactEvent, row: &dyn MaterialRow) -> RumbleCurve {
    let v = voice(ev, row);
    let hi_pmy = (v.gain_pmy * v.timbre / 10_000).clamp(0, 10_000);
    RumbleCurve { lo_pmy: v.gain_pmy, hi_pmy, ms: v.decay_ms }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRow {
        id: u32,
        mass_pmy: Permyriad,
        friction_pmy: Permyriad,
    }

    impl MaterialRow for TestRow {
        fn material_id(&self) -> u32 { self.id }
        fn mass_pmy(&self) -> Permyriad { self.mass_pmy }
        fn friction_pmy(&self) -> Permyriad { self.friction_pmy }
    }

    #[test]
    fn heavier_material_voices_lower() {
        let light = TestRow { id: 1, mass_pmy: 2_000, friction_pmy: 3_000 };
        let heavy = TestRow { id: 2, mass_pmy: 90_000, friction_pmy: 3_000 };
        let ev_light = ImpactEvent { material_id: 1, velocity_pmy: 5_000, at_ns: 10 };
        let ev_heavy = ImpactEvent { material_id: 2, velocity_pmy: 5_000, at_ns: 10 };
        assert!(voice(&ev_heavy, &heavy).pitch_hz < voice(&ev_light, &light).pitch_hz);
    }

    #[test]
    fn zero_velocity_voices_silent() {
        let row = TestRow { id: 1, mass_pmy: 10_000, friction_pmy: 5_000 };
        let ev = ImpactEvent { material_id: 1, velocity_pmy: 0, at_ns: 0 };
        assert_eq!(voice(&ev, &row).gain_pmy, 0);
    }

    #[test]
    fn voice_and_rumble_derive_from_one_event() {
        let row = TestRow { id: 7, mass_pmy: 40_000, friction_pmy: 6_000 };
        let ev = ImpactEvent { material_id: 7, velocity_pmy: 8_000, at_ns: 12_345 };
        let v = voice(&ev, &row);
        let r = rumble(&ev, &row);
        assert_eq!(r.lo_pmy, v.gain_pmy);
        assert_eq!(r.ms, v.decay_ms);
        assert_eq!(ev.material_id, row.material_id());
        assert_eq!(ev.at_ns, 12_345);
    }

    #[test]
    fn missing_row_is_typed_error() {
        let a = TestRow { id: 1, mass_pmy: 1_000, friction_pmy: 1_000 };
        let rows: [&dyn MaterialRow; 1] = [&a];
        assert!(matches!(find_row(99, &rows), Err(MaterialRowMissing(99))));
        assert!(find_row(1, &rows).is_ok());
    }
}
