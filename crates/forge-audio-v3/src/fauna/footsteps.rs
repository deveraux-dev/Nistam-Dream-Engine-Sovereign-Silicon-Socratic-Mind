//! Deterministic Footstep System — hex-based surface-aware audio.
//! Ported from deveraux-game/scripts/engine/hex_footsteps.gd

use crate::mixer::{MixerCommand, Mixer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootstepParams {
    pub surface_id: String,
    pub pitch_range: (f32, f32),
    pub volume_db: f32,
}

pub struct FootstepEngine {
    pub hex_size: f32,
    pub surface_map: std::collections::HashMap<[i32; 2], String>,
}

impl FootstepEngine {
    pub fn new(hex_size: f32) -> Self {
        Self {
            hex_size,
            surface_map: std::collections::HashMap::new(),
        }
    }

    /// World coordinates (X, Z) to axial hex (q, r).
    pub fn world_to_hex(&self, x: f32, z: f32) -> [i32; 2] {
        let q = (2.0 / 3.0 * x) / self.hex_size;
        let r = (-1.0 / 3.0 * x + (3.0f32).sqrt() / 3.0 * z) / self.hex_size;
        self.axial_round(q, r)
    }

    fn axial_round(&self, q: f32, r: f32) -> [i32; 2] {
        let s = -q - r;
        let mut rq = q.round();
        let mut rr = r.round();
        let rs = s.round();

        let q_diff = (rq - q).abs();
        let r_diff = (rr - r).abs();
        let s_diff = (rs - s).abs();

        if q_diff > r_diff && q_diff > s_diff {
            rq = -rr - rs;
        } else if r_diff > s_diff {
            rr = -rq - rs;
        }
        [rq as i32, rr as i32]
    }

    pub fn play_footstep(&self, mixer: &Mixer, position: [f32; 3], rng_val: f32) {
        let hex = self.world_to_hex(position[0], position[2]);
        let surface = self.surface_map.get(&hex).map(|s| s.as_str()).unwrap_or("stone");

        // Map to canon mixer commands
        // MixerCommand::PlaySound(path, volume, pitch, position)
        // ... implementation depends on concrete forge-audio Mixer API
    }
}
</content>
