//! Narrative Persistence — save/load NarrativeState as a flat binary snapshot.
//!
//! Uses a fixed-size byte buffer (no serde dependency on internal types).
//! Format: version byte + raw field copies. Deterministic, no alloc.

use super::NarrativeState;
use super::state::{ShadowRelation, ShadowTier};
use super::oath::OathDiscipline;

/// Binary format version.
const VERSION: u8 = 1;
/// Fixed snapshot size (~512 bytes) for save/load buffer.
pub const SNAPSHOT_SIZE: usize = 512;

/// Serialize a NarrativeState into a fixed-size byte buffer.
pub fn serialize(state: &NarrativeState) -> [u8; SNAPSHOT_SIZE] {
    let mut buf = [0u8; SNAPSHOT_SIZE];
    let mut pos = 0;

    buf[pos] = VERSION; pos += 1;

    // PlayerState
    buf[pos] = state.player.oath_discipline.map(|d| d as u8 + 1).unwrap_or(0); pos += 1;
    buf[pos..pos+16].copy_from_slice(bytemuck_cast_slice(&state.player.combat_profile)); pos += 16;
    buf[pos..pos+16].copy_from_slice(bytemuck_cast_slice(&state.player.ability_profile)); pos += 16;
    buf[pos] = state.player.wound_load; pos += 1;
    buf[pos] = state.player.ash_debt; pos += 1;
    buf[pos..pos+2].copy_from_slice(&state.player.spirit_deaths.to_le_bytes()); pos += 2;
    buf[pos] = match state.player.shadow_relation {
        None => 0, Some(ShadowRelation::Reject) => 1, Some(ShadowRelation::Integrate) => 2,
        Some(ShadowRelation::EmbraceVoid) => 3, Some(ShadowRelation::Unresolved) => 4,
        Some(ShadowRelation::Margin) => 5,
    }; pos += 1;
    buf[pos..pos+4].copy_from_slice(&state.player.ledger_marks.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.player.mercy_flags.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.player.erasure_flags.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.player.witness_flags.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.player.root_scars.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.player.silence_flags.to_le_bytes()); pos += 4;

    // WorldState
    buf[pos] = state.world.root_bloom; pos += 1;
    buf[pos] = state.world.ledger_control; pos += 1;
    buf[pos] = state.world.spirit_leak; pos += 1;
    buf[pos] = state.world.memory_integrity; pos += 1;
    buf[pos] = state.world.entropy_debt; pos += 1;
    buf[pos] = state.world.public_fear; pos += 1;
    buf[pos] = state.world.event_volatility; pos += 1;
    buf[pos] = state.world.shadow_tier as u8; pos += 1;
    buf[pos..pos+8].copy_from_slice(&state.world.faction_pressure); pos += 8;
    buf[pos..pos+4].copy_from_slice(&state.world.ending_mask.to_le_bytes()); pos += 4;

    // EntropyLedger
    buf[pos..pos+4].copy_from_slice(&state.entropy.total.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.entropy.memory_entropy.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.entropy.death_entropy.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.entropy.faction_entropy.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.entropy.shadow_entropy.to_le_bytes()); pos += 4;
    buf[pos..pos+4].copy_from_slice(&state.entropy.name_entropy.to_le_bytes()); pos += 4;

    // EndingPressure (13 × i16 = 26 bytes)
    for &s in &state.pressure.scores {
        buf[pos..pos+2].copy_from_slice(&s.to_le_bytes()); pos += 2;
    }

    // ShadowMemory (key fields)
    for &d in &state.shadow.repeated_attack_dir {
        buf[pos..pos+2].copy_from_slice(&d.to_le_bytes()); pos += 2;
    }
    for &a in &state.shadow.repeated_ability_use {
        buf[pos..pos+2].copy_from_slice(&a.to_le_bytes()); pos += 2;
    }
    buf[pos..pos+2].copy_from_slice(&state.shadow.parry_count.to_le_bytes()); pos += 2;
    buf[pos..pos+2].copy_from_slice(&state.shadow.dodge_count.to_le_bytes()); pos += 2;
    buf[pos..pos+2].copy_from_slice(&state.shadow.execution_count.to_le_bytes()); pos += 2;
    buf[pos..pos+2].copy_from_slice(&state.shadow.death_count.to_le_bytes()); pos += 2;
    buf[pos..pos+4].copy_from_slice(&state.shadow.total_inputs.to_le_bytes()); pos += 4;

    // ZoneDiscovery (13 zones × 6 bytes)
    for zd in &state.zone_discovery {
        buf[pos..pos+2].copy_from_slice(&zd.zone_id.to_le_bytes()); pos += 2;
        buf[pos..pos+2].copy_from_slice(&zd.tells_present.to_le_bytes()); pos += 2;
        buf[pos..pos+2].copy_from_slice(&zd.tells_found.to_le_bytes()); pos += 2;
    }

    // Events (13 × 4 bytes: id + resolved_mode + angle_bits)
    for ev in &state.events {
        buf[pos..pos+2].copy_from_slice(&ev.id.to_le_bytes()); pos += 2;
        buf[pos] = ev.resolved_mode.map(|m| m as u8 + 1).unwrap_or(0); pos += 1;
        buf[pos..pos+2].copy_from_slice(&ev.discovered_angles.to_le_bytes()); pos += 2;
    }

    let _ = pos; // silence unused warning
    buf
}

/// Deserialize a NarrativeState from a byte buffer; returns None if format is incompatible.
pub fn deserialize(buf: &[u8; SNAPSHOT_SIZE], seed: u64) -> Option<NarrativeState> {
    if buf[0] != VERSION { return None; }
    let mut state = NarrativeState::new(seed);
    let mut pos = 1;

    // PlayerState
    state.player.oath_discipline = match buf[pos] {
        0 => None,
        n => Some(OathDiscipline::from_index(n - 1)),
    }; pos += 1;
    state.player.combat_profile = read_u16_array::<8>(&buf[pos..]); pos += 16;
    state.player.ability_profile = read_u16_array::<8>(&buf[pos..]); pos += 16;
    state.player.wound_load = buf[pos]; pos += 1;
    state.player.ash_debt = buf[pos]; pos += 1;
    state.player.spirit_deaths = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    state.player.shadow_relation = match buf[pos] {
        1 => Some(ShadowRelation::Reject), 2 => Some(ShadowRelation::Integrate),
        3 => Some(ShadowRelation::EmbraceVoid), 4 => Some(ShadowRelation::Unresolved),
        5 => Some(ShadowRelation::Margin), _ => None,
    }; pos += 1;
    state.player.ledger_marks = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.player.mercy_flags = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.player.erasure_flags = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.player.witness_flags = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.player.root_scars = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.player.silence_flags = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;

    // WorldState
    state.world.root_bloom = buf[pos]; pos += 1;
    state.world.ledger_control = buf[pos]; pos += 1;
    state.world.spirit_leak = buf[pos]; pos += 1;
    state.world.memory_integrity = buf[pos]; pos += 1;
    state.world.entropy_debt = buf[pos]; pos += 1;
    state.world.public_fear = buf[pos]; pos += 1;
    state.world.event_volatility = buf[pos]; pos += 1;
    state.world.shadow_tier = match buf[pos] {
        1 => ShadowTier::Stalker, 2 => ShadowTier::Blighted, 3 => ShadowTier::Harbinger,
        _ => ShadowTier::None,
    }; pos += 1;
    state.world.faction_pressure.copy_from_slice(&buf[pos..pos+8]); pos += 8;
    state.world.ending_mask = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;

    // EntropyLedger
    state.entropy.total = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.entropy.memory_entropy = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.entropy.death_entropy = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.entropy.faction_entropy = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.entropy.shadow_entropy = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;
    state.entropy.name_entropy = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;

    // EndingPressure
    for i in 0..13 {
        state.pressure.scores[i] = i16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    }

    // ShadowMemory
    for i in 0..4 {
        state.shadow.repeated_attack_dir[i] = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    }
    for i in 0..8 {
        state.shadow.repeated_ability_use[i] = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    }
    state.shadow.parry_count = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    state.shadow.dodge_count = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    state.shadow.execution_count = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    state.shadow.death_count = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    state.shadow.total_inputs = u32::from_le_bytes([buf[pos], buf[pos+1], buf[pos+2], buf[pos+3]]); pos += 4;

    // ZoneDiscovery
    for i in 0..13 {
        state.zone_discovery[i].zone_id = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
        state.zone_discovery[i].tells_present = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
        state.zone_discovery[i].tells_found = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    }

    // Events
    for i in 0..13 {
        state.events[i].id = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
        state.events[i].resolved_mode = match buf[pos] {
            0 => None,
            n => Some(match n - 1 {
                0 => super::event::ResolutionMode::Kill,
                1 => super::event::ResolutionMode::Spare,
                2 => super::event::ResolutionMode::Expose,
                3 => super::event::ResolutionMode::Bind,
                4 => super::event::ResolutionMode::Erase,
                5 => super::event::ResolutionMode::Inherit,
                _ => super::event::ResolutionMode::Abandon,
            }),
        }; pos += 1;
        state.events[i].discovered_angles = u16::from_le_bytes([buf[pos], buf[pos+1]]); pos += 2;
    }

    let _ = pos;
    Some(state)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Cast a u16 array to a u8 slice for binary serialization.
#[allow(unsafe_code)]
fn bytemuck_cast_slice(arr: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(arr.as_ptr() as *const u8, arr.len() * 2) }
}

/// Read N little-endian u16 values from a byte buffer.
fn read_u16_array<const N: usize>(buf: &[u8]) -> [u16; N] {
    let mut out = [0u16; N];
    for i in 0..N {
        out[i] = u16::from_le_bytes([buf[i*2], buf[i*2+1]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let state = NarrativeState::new(42);
        let buf = serialize(&state);
        let restored = deserialize(&buf, 42).unwrap();
        assert_eq!(restored.player.spirit_deaths, 0);
        assert_eq!(restored.world.memory_integrity, 0);
        assert_eq!(restored.entropy.total, 0);
    }

    #[test]
    fn roundtrip_with_data() {
        let mut state = NarrativeState::new(99);
        state.player.spirit_deaths = 7;
        state.player.oath_discipline = Some(OathDiscipline::Bell);
        state.world.memory_integrity = 200;
        state.world.shadow_tier = ShadowTier::Blighted;
        state.entropy.total = 150;
        state.pressure.scores[3] = -42;
        state.shadow.parry_count = 100;
        state.zone_discovery[5].tells_found = 0b1010;
        state.events[2].resolved_mode = Some(super::super::event::ResolutionMode::Expose);

        let buf = serialize(&state);
        let r = deserialize(&buf, 99).unwrap();
        assert_eq!(r.player.spirit_deaths, 7);
        assert_eq!(r.player.oath_discipline, Some(OathDiscipline::Bell));
        assert_eq!(r.world.memory_integrity, 200);
        assert_eq!(r.world.shadow_tier, ShadowTier::Blighted);
        assert_eq!(r.entropy.total, 150);
        assert_eq!(r.pressure.scores[3], -42);
        assert_eq!(r.shadow.parry_count, 100);
        assert_eq!(r.zone_discovery[5].tells_found, 0b1010);
        assert!(r.events[2].is_resolved());
    }

    #[test]
    fn version_mismatch_returns_none() {
        let mut buf = [0u8; SNAPSHOT_SIZE];
        buf[0] = 255; // wrong version
        assert!(deserialize(&buf, 42).is_none());
    }
}
