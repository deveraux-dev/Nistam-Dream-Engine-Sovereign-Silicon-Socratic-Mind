//! Canonical `VixelAtom`/`VixelDiff` layout — the unified 8-byte engine atom
//! and its 18-byte transaction diff, for bit-deterministic IPC over loopback.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-daemon-types\src\atom.rs`
//! (2026-08-15) as part of the daemon-types drain into `forge-daemon-door`.

/// A single, unified 8-byte engine atom.
///
/// Fuses matter, energy, and AST logic coordinates into one atomic unit.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VixelAtom {
    /// UI element identifier, AST Node ID, or 3D asset material slug.
    pub material_id: u16,
    /// Maps directly to VixiScript variable indices and OKLCH colour lookup tables.
    pub color_id: u16,
    /// Frequency value or state index driving the MoE router and the DDSP synthesis engine.
    pub resonance_id: u16,
    /// Depth coordinate within the volumetric Magic Canvas bounding box (Z <= 0).
    pub local_z: u8,
    /// Identifies the routing origin (e.g. raw shell text vs AOT VixiScript 3D geometry).
    pub router_tag: u8,
}

const _: () = assert!(std::mem::size_of::<VixelAtom>() == 8);

/// A cache-aligned 64-byte volumetric chunk.
///
/// Packs a 2x2x2 volumetric cube of [`VixelAtom`]s to match a single CPU L1
/// cache line.
#[repr(C, align(64))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtomicCanvasChunk {
    /// 8 Atoms x 8 bytes = 64 bytes. Fits CPU L1d cache boundaries perfectly.
    pub atoms: [VixelAtom; 8],
}

const _: () = assert!(std::mem::size_of::<AtomicCanvasChunk>() == 64);

/// An 18-byte transaction diff packet.
///
/// The sole mechanism for mutating the state of the unified voxel grid.
/// Serialized and transmitted across the loopback network in little-endian
/// byte order. No `router_tag`: unlike [`VixelAtom`] (composed from multiple
/// authoring sources), a diff travels one known IPC channel — per-atom
/// routing provenance lives on the atom itself, not re-stamped on every
/// mutation of it.
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VixelDiff {
    /// Monotonic metronome clock tick when the state mutation occurred.
    pub tick: u64,
    /// Flattened index within the contiguous SOA canvas memory pool.
    pub index: u32,
    /// New resonance frequency (milli-Hertz) assigned to the targeted atom.
    pub resonance_id: u16,
    /// Updated material state (density, bounce, friction).
    pub material_id: u16,
    /// New OKLCH colour-science palette mapping ID.
    pub color_id: u8,
    /// Updated local depth layout offset.
    pub local_z: u8,
}

const _: () = assert!(std::mem::size_of::<VixelDiff>() == 18);

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn vixel_atom_bytes_roundtrip_over_loopback() {
        let atom = VixelAtom { material_id: 7, color_id: 42, resonance_id: 1000, local_z: 3, router_tag: 1 };
        let bytes = bytemuck::bytes_of(&atom);
        assert_eq!(bytes.len(), 8);
        let back: VixelAtom = *bytemuck::from_bytes(bytes);
        assert_eq!(atom, back);
    }

    #[test]
    fn vixel_diff_bytes_roundtrip_over_loopback() {
        let diff = VixelDiff { tick: 123_456, index: 999, resonance_id: 50, material_id: 9, color_id: 6, local_z: 2 };
        let bytes = bytemuck::bytes_of(&diff);
        assert_eq!(bytes.len(), 18);
        let back: VixelDiff = *bytemuck::from_bytes(bytes);
        assert_eq!(diff, back);
    }

    #[test]
    fn atomic_canvas_chunk_packs_8_atoms() {
        let chunk = AtomicCanvasChunk { atoms: [VixelAtom::zeroed(); 8] };
        assert_eq!(bytemuck::bytes_of(&chunk).len(), 64);
    }
}
