#![allow(missing_docs)]
//! Creation stamps and bounded gameplay influence.

use crate::ids::{ToolId, ZoneId};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CreationStamp {
    pub creator_hash: u64,
    pub session_hash: u64,
    pub tool_id: ToolId,
    pub zone_id: ZoneId,
    pub pressure_mean_q: i16,
    pub pressure_variance_q: i16,
    pub stroke_speed_q: i16,
    pub signal_intensity_q: i16,
    pub signal_variance_q: i16,
    pub metronome_phase_q: i16,
    pub material_bias: u16,
    pub created_tick: u64,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CreationStampHash(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StampGameplayEffect {
    CosmeticOnly,
    AudioTint,
    MaterialTint,
    LoreWitness,
    BoundedResonanceBias { max_delta_q: i16 },
}

impl CreationStamp {
    /// Stable FNV-1a style compact hash. Replace with the repo's canonical proof hash if available.
    pub fn stable_hash(&self) -> CreationStampHash {
        let mut h = 0xcbf29ce484222325u64;
        fn mix(h: &mut u64, v: u64) {
            *h ^= v;
            *h = h.wrapping_mul(0x100000001b3);
        }
        mix(&mut h, self.creator_hash);
        mix(&mut h, self.session_hash);
        mix(&mut h, self.tool_id.0 as u64);
        mix(&mut h, self.zone_id.0 as u64);
        mix(&mut h, self.pressure_mean_q as i64 as u64);
        mix(&mut h, self.pressure_variance_q as i64 as u64);
        mix(&mut h, self.stroke_speed_q as i64 as u64);
        mix(&mut h, self.signal_intensity_q as i64 as u64);
        mix(&mut h, self.signal_variance_q as i64 as u64);
        mix(&mut h, self.metronome_phase_q as i64 as u64);
        mix(&mut h, self.material_bias as u64);
        mix(&mut h, self.created_tick);
        CreationStampHash(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_stamp_hash_is_stable() {
        let s = CreationStamp { creator_hash: 1, session_hash: 2, tool_id: ToolId(3), zone_id: ZoneId(4), pressure_mean_q: 5, pressure_variance_q: 6, stroke_speed_q: 7, signal_intensity_q: 8, signal_variance_q: 9, metronome_phase_q: 10, material_bias: 11, created_tick: 12 };
        assert_eq!(s.stable_hash(), s.stable_hash());
    }
}
