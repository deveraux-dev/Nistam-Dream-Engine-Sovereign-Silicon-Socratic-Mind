//! The .zrk replay file system — orthogonal to forge-engine-v3's RollbackRing
//! and forge-daemon-door's timeline tape (compact replay export, not a rollback
//! ring). Ported from abraxas/ironroot-edict shared_core, 2026-08-28 fold.
//!
//! A replay is a cryptographic proof of a run. It stores only:
//! - Validation metadata (version hash, ledger SHA-256, class ID)
//! - The master seed (regenerates the entire world)
//! - The input stream (per-frame u8 bitmasks)

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ZrkReplay {
    pub version_hash: String,
    pub ledger_sha256: String,
    pub player_loadout: u8,
    pub master_seed: u64,
    pub input_stream: Vec<FrameInput>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FrameInput {
    pub frame: u32,
    pub input_mask: u8,
}

/// In-memory recorder that captures inputs during a live session.
pub struct ZrkReplayRecorder {
    pub replay: ZrkReplay,
}

impl ZrkReplayRecorder {
    pub fn new(master_seed: u64, class_id: u8, version_hash: String, ledger_hash: String) -> Self {
        Self {
            replay: ZrkReplay {
                version_hash,
                ledger_sha256: ledger_hash,
                player_loadout: class_id,
                master_seed,
                input_stream: Vec::with_capacity(60 * 60 * 15), // ~15 min at 60fps
            },
        }
    }

    pub fn record_input(&mut self, frame: u32, input_mask: u8) {
        self.replay.input_stream.push(FrameInput { frame, input_mask });
    }

    pub fn export_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self.replay).expect("Replay serialization must not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_exports() {
        let mut rec = ZrkReplayRecorder::new(42, 1, "v1".into(), "abc123".into());
        rec.record_input(0, 0x01);
        rec.record_input(1, 0x03);
        assert_eq!(rec.replay.input_stream.len(), 2);
        let bytes = rec.export_bytes();
        let restored: ZrkReplay = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.master_seed, 42);
        assert_eq!(restored.input_stream.len(), 2);
    }
}
