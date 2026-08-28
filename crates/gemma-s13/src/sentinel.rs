// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Sentinel Governor — 13-slot out-of-band state dispatcher.
//!
//! Implements:
//! 1. Branchless comparator checking byte ranges >= 243.
//! 2. Mapping of the upper 13 byte states (243..=255) to sentinel slots 0..12.
//! 3. 16-byte `UmpWord` payload compiler on sentinel breach to halt inference.
//! 4. Fast XOR + POPCNT Hamming distance centroid matching for audio alert routing.
//!
//! The band exists because S13 packs 5 trits per byte: 3^5 = 243 encodable states,
//! leaving 243..=255 unreachable by any legal trit word. Those 13 spare states are
//! therefore usable as out-of-band signals that no in-band data can forge.

#![deny(unsafe_code)]

/// Threshold for sentinel states.
pub const SENTINEL_MIN_BYTE: u8 = 243;

/// Total number of sentinel states (243..=255).
pub const SENTINEL_STATES_COUNT: usize = 13;

/// The 13 out-of-band sentinel slots, named by the byte each decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SentinelBand {
    /// Byte 243, slot index 0.
    Slot243 = 243,
    /// Byte 244, slot index 1.
    Slot244 = 244,
    /// Byte 245, slot index 2.
    Slot245 = 245,
    /// Byte 246, slot index 3.
    Slot246 = 246,
    /// Byte 247, slot index 4.
    Slot247 = 247,
    /// Byte 248, slot index 5.
    Slot248 = 248,
    /// Byte 249, slot index 6.
    Slot249 = 249,
    /// Byte 250, slot index 7.
    Slot250 = 250,
    /// Byte 251, slot index 8.
    Slot251 = 251,
    /// Byte 252, slot index 9.
    Slot252 = 252,
    /// Byte 253, slot index 10.
    Slot253 = 253,
    /// Byte 254, slot index 11.
    Slot254 = 254,
    /// Byte 255, slot index 12.
    Slot255 = 255,
}

impl SentinelBand {
    /// Decode a byte in range `243..=255` into its corresponding slot.
    #[inline]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            243 => Some(Self::Slot243),
            244 => Some(Self::Slot244),
            245 => Some(Self::Slot245),
            246 => Some(Self::Slot246),
            247 => Some(Self::Slot247),
            248 => Some(Self::Slot248),
            249 => Some(Self::Slot249),
            250 => Some(Self::Slot250),
            251 => Some(Self::Slot251),
            252 => Some(Self::Slot252),
            253 => Some(Self::Slot253),
            254 => Some(Self::Slot254),
            255 => Some(Self::Slot255),
            _ => None,
        }
    }

    /// Slot index `0..=12`, the offset of this byte above [`SENTINEL_MIN_BYTE`].
    /// This is the same value the halt packet carries in `bytes[3]`.
    #[inline]
    pub const fn index(&self) -> u8 {
        (*self as u8) - SENTINEL_MIN_BYTE
    }

    /// Stable structural label, e.g. `"sentinel slot 3 of 13 (byte 246)"`.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Slot243 => "sentinel slot 0 of 13 (byte 243)",
            Self::Slot244 => "sentinel slot 1 of 13 (byte 244)",
            Self::Slot245 => "sentinel slot 2 of 13 (byte 245)",
            Self::Slot246 => "sentinel slot 3 of 13 (byte 246)",
            Self::Slot247 => "sentinel slot 4 of 13 (byte 247)",
            Self::Slot248 => "sentinel slot 5 of 13 (byte 248)",
            Self::Slot249 => "sentinel slot 6 of 13 (byte 249)",
            Self::Slot250 => "sentinel slot 7 of 13 (byte 250)",
            Self::Slot251 => "sentinel slot 8 of 13 (byte 251)",
            Self::Slot252 => "sentinel slot 9 of 13 (byte 252)",
            Self::Slot253 => "sentinel slot 10 of 13 (byte 253)",
            Self::Slot254 => "sentinel slot 11 of 13 (byte 254)",
            Self::Slot255 => "sentinel slot 12 of 13 (byte 255)",
        }
    }
}

/// 16-byte UMP (Universal MIDI Packet / Unified Message Protocol) word payload.
/// Dispatched immediately to halt inference execution on sentinel breach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct UmpWord16 {
    /// 16 raw data bytes.
    pub bytes: [u8; 16],
}

impl UmpWord16 {
    /// Compile a 16-byte UmpWord halt packet from a detected sentinel slot.
    #[inline]
    pub const fn compile_sentinel_halt(band: SentinelBand, token_index: u32, sim_tick: u64) -> Self {
        let band_val = band as u8;
        let mut bytes = [0u8; 16];

        // UMP Header (Type 0xF Stream/Utility message, Status 0x13 Sentinel Alert)
        bytes[0] = 0xF0;
        bytes[1] = 0x13;
        bytes[2] = band_val;
        bytes[3] = (band_val.wrapping_sub(SENTINEL_MIN_BYTE)) & 0x0F;

        // Token index (big-endian u32 in bytes 4..8)
        bytes[4] = (token_index >> 24) as u8;
        bytes[5] = (token_index >> 16) as u8;
        bytes[6] = (token_index >> 8) as u8;
        bytes[7] = token_index as u8;

        // Simulation tick (big-endian u64 in bytes 8..16)
        bytes[8] = (sim_tick >> 56) as u8;
        bytes[9] = (sim_tick >> 48) as u8;
        bytes[10] = (sim_tick >> 40) as u8;
        bytes[11] = (sim_tick >> 32) as u8;
        bytes[12] = (sim_tick >> 24) as u8;
        bytes[13] = (sim_tick >> 16) as u8;
        bytes[14] = (sim_tick >> 8) as u8;
        bytes[15] = sim_tick as u8;

        Self { bytes }
    }
}

/// Branchless sentinel comparator.
/// Returns `1` if `byte >= 243`, returns `0` otherwise.
/// Executes without conditional branches to protect DFA pipeline latency (37.36ns).
#[inline(always)]
pub const fn is_sentinel_branchless(byte: u8) -> u8 {
    // ((242 - byte) >> 7) & 1: if byte >= 243, 242 - byte underflows (MSB = 1), so shifted result is 1.
    // If byte <= 242, 242 - byte is >= 0 (MSB = 0), so shifted result is 0.
    let diff = (242u32.wrapping_sub(byte as u32)) >> 31;
    diff as u8
}

/// Compute Hamming distance between two 64-bit fingerprint words using bitwise XOR and POPCNT.
#[inline(always)]
pub const fn hamming_distance_u64(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Audio alert centroid targets, one per sentinel slot (13 64-bit harmonic fingerprints).
pub const SENTINEL_AUDIO_CENTROIDS: [u64; 13] = [
    0x1301_3001_0000_0001,
    0x2302_3002_0000_0002,
    0x3303_3003_0000_0003,
    0x4304_3004_0000_0004,
    0x5305_3005_0000_0005,
    0x6306_3006_0000_0006,
    0x7307_3007_0000_0007,
    0x8308_3008_0000_0008,
    0x9309_3009_0000_0009,
    0xA30A_300A_0000_000A,
    0xB30B_300B_0000_000B,
    0xC30C_300C_0000_000C,
    0xD30D_300D_0000_000D,
];

/// Match an incoming 64-bit alert signal against the 13 sentinel centroids.
/// Returns the index (0..12) of the closest harmonic centroid and its Hamming distance.
#[inline]
pub fn route_alert_centroid(signal: u64) -> (usize, u32) {
    let mut best_idx = 0;
    let mut min_dist = u32::MAX;

    let mut i = 0;
    while i < SENTINEL_STATES_COUNT {
        let dist = hamming_distance_u64(signal, SENTINEL_AUDIO_CENTROIDS[i]);
        if dist < min_dist {
            min_dist = dist;
            best_idx = i;
        }
        i += 1;
    }

    (best_idx, min_dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branchless_comparator_exact_boundary() {
        for b in 0..=242u8 {
            assert_eq!(is_sentinel_branchless(b), 0, "Failed for byte {}", b);
        }
        for b in 243..=255u8 {
            assert_eq!(is_sentinel_branchless(b), 1, "Failed for byte {}", b);
        }
    }

    #[test]
    fn test_13_slot_mapping() {
        assert_eq!(SentinelBand::from_byte(243), Some(SentinelBand::Slot243));
        assert_eq!(SentinelBand::from_byte(255), Some(SentinelBand::Slot255));
        assert_eq!(SentinelBand::from_byte(242), None);
    }

    /// `index()` is the wire value: whatever the halt packet puts in `bytes[3]`.
    #[test]
    fn test_slot_index_matches_wire_offset() {
        for byte in SENTINEL_MIN_BYTE..=255u8 {
            let band = SentinelBand::from_byte(byte).expect("in band");
            assert_eq!(band.index(), byte - SENTINEL_MIN_BYTE);
            let packet = UmpWord16::compile_sentinel_halt(band, 0, 0);
            assert_eq!(packet.bytes[3], band.index(), "wire offset drifted for {byte}");
        }
        assert_eq!(SentinelBand::Slot255.index() as usize, SENTINEL_STATES_COUNT - 1);
    }

    #[test]
    fn test_umpword_halt_compilation() {
        let band = SentinelBand::Slot246;
        let packet = UmpWord16::compile_sentinel_halt(band, 1024, 120_000);

        assert_eq!(packet.bytes[0], 0xF0);
        assert_eq!(packet.bytes[1], 0x13);
        assert_eq!(packet.bytes[2], 246);
        assert_eq!(packet.bytes[3], 3); // 246 - 243 = 3
    }

    #[test]
    fn test_hamming_distance_and_routing() {
        let target = SENTINEL_AUDIO_CENTROIDS[3]; // slot index 3 == byte 246
        let (idx, dist) = route_alert_centroid(target);
        assert_eq!(idx, 3);
        assert_eq!(dist, 0);

        let noisy_target = target ^ 0b101; // 2 bit flips
        let (idx_noisy, dist_noisy) = route_alert_centroid(noisy_target);
        assert_eq!(idx_noisy, 3);
        assert_eq!(dist_noisy, 2);
    }

    #[test]
    fn test_slot_names_are_structural() {
        assert!(SentinelBand::Slot243.name().contains("slot 0 of 13"));
        assert!(SentinelBand::Slot243.name().contains("243"));
        assert!(SentinelBand::Slot255.name().contains("slot 12 of 13"));
        assert!(SentinelBand::Slot255.name().contains("255"));
    }

    #[test]
    fn test_branchless_comparator_exact_threshold_242_vs_243() {
        assert_eq!(is_sentinel_branchless(242), 0);
        assert_eq!(is_sentinel_branchless(243), 1);
    }
}
