//! IntentPacket — multidimensional intent inside `RouteIntent.args` (COMPILER-GROUPS.md §4 gap 2).
//!
//! Spatial/relational/physical channels of one interaction, AOT-lowered to
//! integers: `handling_class` is the compiled residue of an author-time
//! affordance gloss (e.g. Navajo "slender flexible object"), `aspect` the
//! compiled relational rank (e.g. Cree proximate/obviative). No string ever
//! reaches this layer; the author-time vocabulary lives in `.kg.json` theory
//! inputs and is enum-lowered at bake.
//!
//! Confidence is the evidentiality bound: permyriad, `1..=10_000`. Zero is the
//! ABSENT sentinel — a packet without an evidentiality bound refuses to decode
//! whole (never partially applied), and refuses to encode in the first place.
//!
//! Explicit byte codec, little-endian — "little nîstam" (ARCH000 2026-08-12:
//! nîstam is Plains Cree "first"; the little byte goes first, so the packet
//! that carries Cree obviation ranks names its own wire order in Cree). No
//! `bytemuck` cast: `args` sits at offset 1 inside the 32-byte `RouteIntent`,
//! so a `u16`-bearing view would be misaligned. Layout inside `args[0..8]`,
//! tail `args[8..31]` reserved must-be-zero:
//!
//! ```text
//! [0..2) actor LE  [2..4) patient LE  [4] aspect  [5] handling_class  [6..8) confidence_pmy LE
//! ```

use crate::ARGS_LEN;

/// Upper bound of the permyriad confidence channel (represents 1.0).
pub const CONFIDENCE_PMY_MAX: u16 = 10_000;

/// Bytes of `args` the packet occupies; the remaining tail is reserved zero.
pub const PACKET_USED_BYTES: usize = 8;

/// One interaction intent: who acts on whom, under which relational aspect and
/// handling class, with what evidentiality. All-integer; floats live author-side only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentPacket {
    /// Acting entity id.
    pub actor: u16,
    /// Acted-upon entity id.
    pub patient: u16,
    /// Compiled relational rank (author-time obviation/animacy vocabulary).
    pub aspect: u8,
    /// Compiled affordance class (author-time shape-class vocabulary).
    pub handling_class: u8,
    /// Evidentiality bound, permyriad. Valid `1..=10_000`; `0` means absent
    /// and is refused by both codec directions.
    pub confidence_pmy: u16,
}

impl IntentPacket {
    /// True when the evidentiality bound is present and in range.
    pub const fn confidence_valid(&self) -> bool {
        self.confidence_pmy >= 1 && self.confidence_pmy <= CONFIDENCE_PMY_MAX
    }

    /// Encode into a fresh `args` payload. Refuses (`None`) a packet whose
    /// evidentiality bound is absent or out of range — an unencodable packet
    /// is caught at the source, not downstream.
    pub fn encode(&self) -> Option<[u8; ARGS_LEN]> {
        if !self.confidence_valid() {
            return None;
        }
        let mut args = [0u8; ARGS_LEN];
        args[0..2].copy_from_slice(&self.actor.to_le_bytes());
        args[2..4].copy_from_slice(&self.patient.to_le_bytes());
        args[4] = self.aspect;
        args[5] = self.handling_class;
        args[6..8].copy_from_slice(&self.confidence_pmy.to_le_bytes());
        Some(args)
    }

    /// Decode from an `args` payload. Refused whole (`None`) when the
    /// evidentiality bound is absent/out-of-range or any reserved tail byte is
    /// nonzero — never partially applied.
    pub fn decode(args: &[u8; ARGS_LEN]) -> Option<Self> {
        let mut i = PACKET_USED_BYTES;
        while i < ARGS_LEN {
            if args[i] != 0 {
                return None;
            }
            i += 1;
        }
        let packet = Self {
            actor: u16::from_le_bytes([args[0], args[1]]),
            patient: u16::from_le_bytes([args[2], args[3]]),
            aspect: args[4],
            handling_class: args[5],
            confidence_pmy: u16::from_le_bytes([args[6], args[7]]),
        };
        if !packet.confidence_valid() {
            return None;
        }
        Some(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode as route_decode, RouteExpert, RouteIntent, INTENT_BYTES};

    fn packet(confidence_pmy: u16) -> IntentPacket {
        IntentPacket {
            actor: 7,
            patient: 11,
            aspect: 2,
            handling_class: 5,
            confidence_pmy,
        }
    }

    #[test]
    fn bijection_over_interior_and_edges() {
        // f_inv(f(x)) == x across interior, sentinel-adjacent, and edge values (L07).
        for (actor, patient) in [(0u16, 0u16), (1, u16::MAX), (u16::MAX, 1), (0x1234, 0xFEDC)] {
            for (aspect, handling) in [(0u8, 0u8), (255, 255), (1, 254)] {
                for confidence in [1u16, 2, 5_000, 9_999, CONFIDENCE_PMY_MAX] {
                    let p = IntentPacket { actor, patient, aspect, handling_class: handling, confidence_pmy: confidence };
                    let args = p.encode().expect("valid confidence must encode");
                    assert_eq!(IntentPacket::decode(&args), Some(p));
                }
            }
        }
    }

    #[test]
    fn bytes_roundtrip_through_decode_then_encode() {
        // f(f_inv(b)) == b: a valid payload re-encodes to identical bytes.
        let args = packet(9_413).encode().unwrap();
        let re = IntentPacket::decode(&args).unwrap().encode().unwrap();
        assert_eq!(re, args);
    }

    #[test]
    fn absent_confidence_refused_both_directions() {
        assert_eq!(packet(0).encode(), None, "confidence 0 = absent, refuse at encode");
        let mut args = packet(1).encode().unwrap();
        args[6] = 0;
        args[7] = 0;
        assert_eq!(IntentPacket::decode(&args), None, "absent confidence refuses decode whole");
    }

    #[test]
    fn out_of_range_confidence_refused_both_directions() {
        for bad in [CONFIDENCE_PMY_MAX + 1, 20_000, u16::MAX] {
            assert_eq!(packet(bad).encode(), None, "{bad} > 10_000 must refuse encode");
            let mut args = packet(1).encode().unwrap();
            args[6..8].copy_from_slice(&bad.to_le_bytes());
            assert_eq!(IntentPacket::decode(&args), None, "{bad} > 10_000 must refuse decode");
        }
    }

    #[test]
    fn nonzero_reserved_tail_refused_whole() {
        for dirty in [PACKET_USED_BYTES, PACKET_USED_BYTES + 9, ARGS_LEN - 1] {
            let mut args = packet(1).encode().unwrap();
            args[dirty] = 1;
            assert_eq!(IntentPacket::decode(&args), None, "reserved byte {dirty} nonzero must refuse whole");
        }
    }

    #[test]
    fn rides_inside_route_intent_unchanged() {
        let p = packet(CONFIDENCE_PMY_MAX);
        let intent = RouteIntent {
            discriminant: RouteExpert::Physics.as_u8(),
            args: p.encode().unwrap(),
        };
        let raw: &[u8] = bytemuck::bytes_of(&intent);
        assert_eq!(raw.len(), INTENT_BYTES);
        let (expert, args) = route_decode(raw).expect("valid route intent");
        assert_eq!(expert, RouteExpert::Physics);
        assert_eq!(IntentPacket::decode(args), Some(p));
    }
}
