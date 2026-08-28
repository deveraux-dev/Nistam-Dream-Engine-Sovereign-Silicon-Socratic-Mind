//! UMP authority-ticket bridge — 16-byte Pod handle for deterministic lane routing.
//! Ported from F:\NewRepo\crates\forge-ump\src\ticket.rs (v2). Payload bytes live in a
//! side table addressed by hash; the ticket is pure metadata, GPU-shareable unmarshalled.

use bytemuck::{Pod, Zeroable};
use forge_core_v3::spine::{BrutalHash, CarrierKind, Lane};
use forge_vcs_v3::hash::BrutalHashExt;

use crate::packet::Ump;

/// Schema version of the [`UmpAuthorityTicket::deterministic_hash`] byte layout.
pub const UMP_TICKET_SCHEMA_V0: u16 = 0;
/// The carrier kind every ticket stamps: `CarrierKind::UmpTicketPack` (10).
pub const UMP_TICKET_KIND: CarrierKind = CarrierKind::UmpTicketPack;

/// Compact ledger-facing handle for a parsed UMP packet or packet group.
/// Exactly 16 bytes (const-asserted below). Lane maps `forge_core_v3::spine::Lane`
/// 0..=4; see that enum for the decimation semantics per lane.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct UmpAuthorityTicket {
    /// Hash of the world/session authority scope.
    pub world_hash: u64,
    /// Tick or source-time value used as the deterministic ordering anchor.
    pub source_tick: u32,
    /// UMP group nibble (0..=15), widened for stable C layout.
    pub group: u8,
    /// `forge_core_v3::spine::Lane` discriminant (0..=4).
    pub lane: u8,
    /// `CarrierKind::UmpTicketPack` (discriminant 10), stored as u16 for 16-byte symmetry.
    pub carrier_kind: u16,
}

const _: () = assert!(core::mem::size_of::<UmpAuthorityTicket>() == 16);
const _: () = assert!(core::mem::align_of::<UmpAuthorityTicket>() <= 16);

impl UmpAuthorityTicket {
    /// Build a ticket; `group` is masked to its low nibble.
    #[inline]
    pub const fn new(world_hash: u64, source_tick: u32, group: u8, lane: Lane) -> Self {
        Self {
            world_hash,
            source_tick,
            group: group & 0x0f,
            lane: lane.as_u8(),
            carrier_kind: UMP_TICKET_KIND.as_u8() as u16,
        }
    }

    /// Build from a packet, extracting the group nibble from word 0.
    #[inline]
    pub fn from_ump(world_hash: u64, source_tick: u32, lane: Lane, ump: Ump) -> Self {
        let group = ((ump.words[0] >> 24) & 0x0f) as u8;
        Self::new(world_hash, source_tick, group, lane)
    }

    /// Decode the lane discriminant; `None` when out of range.
    #[inline]
    pub const fn lane(self) -> Option<Lane> {
        Lane::from_u8(self.lane)
    }

    /// Decode the carrier kind; `None` when out of range.
    #[inline]
    pub const fn carrier_kind(self) -> Option<CarrierKind> {
        CarrierKind::from_u8(self.carrier_kind as u8)
    }

    /// Well-formedness: known lane, exact carrier kind, group within nibble.
    #[inline]
    pub const fn is_valid(self) -> bool {
        Lane::from_u8(self.lane).is_some()
            && self.carrier_kind == UMP_TICKET_KIND.as_u8() as u16
            && self.group <= 0x0f
    }

    /// Stable spine-domain hash: ticket metadata + caller actor/subject/payload
    /// hashes in fixed little-endian order, through the canonical blake3-truncated
    /// [`BrutalHash`]. The byte layout IS the schema contract — any reorder or
    /// width change bumps [`UMP_TICKET_SCHEMA_V0`].
    pub fn deterministic_hash(
        self,
        actor_hash: u64,
        subject_hash: u64,
        payload_hash: u64,
    ) -> BrutalHash {
        let mut buf = [0u8; 44];
        buf[0] = UMP_TICKET_KIND.as_u8();
        buf[1] = self.group;
        buf[2] = self.lane;
        buf[3] = 0; // pad
        buf[4..12].copy_from_slice(&self.world_hash.to_le_bytes());
        buf[12..16].copy_from_slice(&self.source_tick.to_le_bytes());
        buf[16..24].copy_from_slice(&actor_hash.to_le_bytes());
        buf[24..32].copy_from_slice(&subject_hash.to_le_bytes());
        buf[32..40].copy_from_slice(&payload_hash.to_le_bytes());
        buf[40..42].copy_from_slice(&UMP_TICKET_SCHEMA_V0.to_le_bytes());
        buf[42] = 0;
        buf[43] = 0;
        BrutalHash::of(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ump_authority_ticket_is_16_bytes() {
        assert_eq!(core::mem::size_of::<UmpAuthorityTicket>(), 16);
        assert_eq!(core::mem::align_of::<UmpAuthorityTicket>(), 8);
    }

    #[test]
    fn ump_authority_ticket_is_pod_safe() {
        fn assert_pod<T: Pod>() {}
        fn assert_zeroable<T: Zeroable>() {}
        assert_pod::<UmpAuthorityTicket>();
        assert_zeroable::<UmpAuthorityTicket>();
    }

    #[test]
    fn ump_authority_ticket_uses_carrier_kind_10() {
        let ticket = UmpAuthorityTicket::new(1, 2, 3, Lane::Speculative);
        assert_eq!(ticket.carrier_kind, 10);
        assert_eq!(ticket.carrier_kind(), Some(CarrierKind::UmpTicketPack));
        assert!(ticket.is_valid());
    }

    #[test]
    fn ump_authority_ticket_extracts_group_from_ump() {
        let ump = Ump::new([0x4b90_4000, 0, 0, 0]);
        let ticket = UmpAuthorityTicket::from_ump(1, 120, Lane::NearFuture, ump);
        assert_eq!(ticket.group, 0x0b);
        assert_eq!(ticket.lane(), Some(Lane::NearFuture));
    }

    #[test]
    fn ump_authority_ticket_hash_is_deterministic() {
        let a = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::PriorAuthority)
            .deterministic_hash(1, 2, 3);
        let b = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::PriorAuthority)
            .deterministic_hash(1, 2, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn ump_authority_ticket_hash_changes_on_payload() {
        let a = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::PriorAuthority)
            .deterministic_hash(1, 2, 3);
        let b = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::PriorAuthority)
            .deterministic_hash(1, 2, 99);
        assert_ne!(a, b);
    }

    #[test]
    fn invalid_lane_discriminant_rejected() {
        let mut ticket = UmpAuthorityTicket::new(0, 0, 0, Lane::Critical);
        ticket.lane = 99;
        assert!(!ticket.is_valid());
        assert_eq!(ticket.lane(), None);
    }
}
