//! Strict-enum airlock for RouteIntent. Zero-copy 32-byte FFI between Python airlock and no_alloc Rust.

use bytemuck::{Pod, Zeroable};

pub mod packet;
pub use packet::{IntentPacket, CONFIDENCE_PMY_MAX, PACKET_USED_BYTES};

/// Size in bytes of the on-the-wire RouteIntent representation.
pub const INTENT_BYTES: usize = 32;

/// Number of argument bytes carried by RouteIntent (total size minus 1 discriminant byte).
pub const ARGS_LEN: usize = 31;

/// 7-expert canonical routing vocabulary. Discriminants match
/// `forge-broski::observation::NdeEvent` variant order and
/// `forge-ml::dispatch::expert_for_event`. No fallback sentinel: unrecognized
/// discriminants fail closed at the FFI boundary instead of routing to a
/// silent default.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteExpert {
    /// Sound processing expert.
    Sound = 0,
    /// Visual processing expert.
    Visual = 1,
    /// Physics processing expert.
    Physics = 2,
    /// Sieve processing expert.
    Sieve = 3,
    /// Lorekeeper processing expert.
    Lorekeeper = 4,
    /// World processing expert.
    World = 5,
    /// Human interface expert.
    HumanInterface = 6,
}

impl RouteExpert {
    /// Returns the variant for the given byte, or `None` if the byte is outside 0..=6.
    /// Unrecognized discriminants never route to a default expert.
    #[inline]
    pub const fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Sound),
            1 => Some(Self::Visual),
            2 => Some(Self::Physics),
            3 => Some(Self::Sieve),
            4 => Some(Self::Lorekeeper),
            5 => Some(Self::World),
            6 => Some(Self::HumanInterface),
            _ => None,
        }
    }

    /// Converts this variant to its discriminant byte.
    /// Inverse of [`Self::from_u8`]. Round-trip identity:
    /// `RouteExpert::from_u8(x.as_u8()) == Some(x)` for all variants.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Fixed-size routing intent: discriminant (expert selector) and 31 bytes of arguments.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RouteIntent {
    /// Expert discriminant (0..=6). Outside this range fails the decode.
    pub discriminant: u8,
    /// Argument payload (31 bytes, packed after the discriminant).
    pub args: [u8; ARGS_LEN],
}

const _: () = assert!(core::mem::size_of::<RouteIntent>() == INTENT_BYTES);

/// Decode a 32-byte buffer into (expert, args). Returns None if the buffer
/// length is wrong or the discriminant is outside 0..=6.
pub fn decode(buf: &[u8]) -> Option<(RouteExpert, &[u8; ARGS_LEN])> {
    if buf.len() != INTENT_BYTES {
        return None;
    }
    let intent: &RouteIntent = bytemuck::from_bytes(buf);
    let expert = RouteExpert::from_u8(intent.discriminant)?;
    Some((expert, &intent.args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_7_experts_roundtrip() {
        for v in 0..=6u8 {
            let expert = RouteExpert::from_u8(v).expect("variant exists");
            assert_eq!(expert.as_u8(), v);
        }
    }

    #[test]
    fn unknown_expert_rejected() {
        for v in 7..=255u8 {
            assert!(RouteExpert::from_u8(v).is_none(), "{v} should be None");
        }
    }

    #[test]
    fn discriminant_extraction() {
        for v in 0..=6u8 {
            let expert = RouteExpert::from_u8(v).expect("variant exists");
            let intent = RouteIntent {
                discriminant: v,
                args: [0u8; ARGS_LEN],
            };
            let raw: &[u8] = bytemuck::bytes_of(&intent);
            assert_eq!(raw.len(), INTENT_BYTES);
            assert_eq!(raw[0], v, "byte 0 must be the discriminant");
            assert_eq!(raw[0], expert.as_u8(), "discriminant byte must equal enum repr");
        }
    }
}
