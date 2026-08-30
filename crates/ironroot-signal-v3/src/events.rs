#![allow(missing_docs)]
//! Ledger-facing event shapes. These are compact, bounded, and replayable.

use crate::ids::{AssetId, Tick, ToolId, Vec3i, ZoneId};
use crate::stamp::CreationStampHash;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CreationEvent {
    AssetStarted { asset_id: AssetId, tool_id: ToolId, tick: Tick },
    AssetCommitted { asset_id: AssetId, stamp_hash: CreationStampHash, waveform_hash: u64, material_hash: u64, tick: Tick },
    AssetBoundToWorld { asset_id: AssetId, zone_id: ZoneId, position: Vec3i, tick: Tick },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AmbientParameter {
    FogDensity,
    WaterTurbulence,
    ShaderContrast,
    EmissivePulse,
    AudioSwell,
    ParticleFlow,
    MenuParticleSpeed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StampInfluenceKind {
    CosmeticOnly,
    AudioTint,
    MaterialTint,
    LoreWitness,
    BoundedResonanceBias,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WorldFluxEvent {
    MetronomePhaseAdvanced { phase_q: i16, tick: Tick },
    AmbientParameterChanged { zone_id: ZoneId, parameter: AmbientParameter, value_q: i16, tick: Tick },
    StampInfluenceApplied { asset_id: AssetId, influence_kind: StampInfluenceKind, bounded_delta_q: i16, tick: Tick },
    SignalPulse { intensity: u8, tick: Tick },
    SignalStateChange { new_state: u8, tick: Tick },
    AmbientModulation { value: i32, tick: Tick },
}
