//! UMP authority-ticket bridge for deterministic lane routing.
//!
//! A ticket is the compact, ledger-facing handle for a parsed UMP packet or
//! packet group. It does not store the full UMP message payload. Store the UMP
//! bytes/message elsewhere and bind this ticket through `payload_hash`.

use bytemuck::{Pod, Zeroable};
use forge_core::{BrutalHash, BrutalHashInput, CarrierKind, Lane};

use crate::packet::Ump;

pub const UMP_TICKET_SCHEMA_V0: u16 = 0;
pub const UMP_TICKET_KIND: CarrierKind = CarrierKind::UmpTicketPack;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct UmpAuthorityTicket {
    /// Hash of the world/session authority scope.
    pub world_hash: u64,
    /// Tick or source-time value used as the deterministic ordering anchor.
    pub source_tick: u32,
    /// UMP group nibble, widened for stable C layout.
    pub group: u8,
    /// forge-core spine lane. Expected values are `Lane::{0..4}`.
    pub lane: u8,
    /// `CarrierKind::UmpTicketPack`, stored as u16 for exact layout.
    pub carrier_kind: u16,
}

impl UmpAuthorityTicket {
    #[inline]
    pub const fn new(world_hash: u64, source_tick: u32, group: u8, lane: Lane) -> Self {
        Self {
            world_hash,
            source_tick,
            group: group & 0x0f,
            lane: lane.as_u8(),
            carrier_kind: UMP_TICKET_KIND.as_u16(),
        }
    }

    #[inline]
    pub fn from_ump(world_hash: u64, source_tick: u32, lane: Lane, ump: Ump) -> Self {
        let group = ((ump.words[0] >> 24) & 0x0f) as u8;
        Self::new(world_hash, source_tick, group, lane)
    }

    #[inline]
    pub const fn lane(self) -> Option<Lane> {
        Lane::from_u8(self.lane)
    }

    #[inline]
    pub const fn carrier_kind(self) -> Option<CarrierKind> {
        CarrierKind::from_u16(self.carrier_kind)
    }

    #[inline]
    pub const fn is_valid(self) -> bool {
        Lane::from_u8(self.lane).is_some()
            && matches!(CarrierKind::from_u16(self.carrier_kind), Some(CarrierKind::UmpTicketPack))
            && self.group <= 0x0f
    }

    #[inline]
    pub const fn deterministic_hash(
        self,
        actor_hash: u64,
        subject_hash: u64,
        payload_hash: u64,
    ) -> BrutalHash {
        BrutalHashInput {
            kind: UMP_TICKET_KIND.as_u16(),
            world: self.world_hash,
            actor: actor_hash,
            subject: subject_hash,
            source_tick: self.source_tick as u64,
            payload_hash,
            schema: UMP_TICKET_SCHEMA_V0,
        }
        .deterministic_hash()
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
        let ticket = UmpAuthorityTicket::from_ump(1, 120, Lane::Deterministic, ump);
        assert_eq!(ticket.group, 0x0b);
        assert_eq!(ticket.lane(), Some(Lane::Deterministic));
    }

    #[test]
    fn ump_authority_ticket_hash_is_deterministic() {
        let a = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::Authored)
            .deterministic_hash(1, 2, 3);
        let b = UmpAuthorityTicket::new(0xabc, 120, 2, Lane::Authored)
            .deterministic_hash(1, 2, 3);
        assert_eq!(a, b);
    }
}
