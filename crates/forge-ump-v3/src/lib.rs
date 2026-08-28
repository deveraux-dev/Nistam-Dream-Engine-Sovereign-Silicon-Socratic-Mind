//! UMP provenance tape (NOT the MIDI 2.0 wire format) — ported from `F:\NewRepo\crates\forge-ump`. Layer 2
//! landed the `timeline_recorder.rs` closure: `packet`, `stamp_chain`,
//! `provenance_tag`, `timeline`, `recorder`. Layer 2.5 added the futuresight
//! trio `ghost`, `futuresight`, `oracle` (daemon-door's `timeline_recorder.rs`
//! calls `crate::timeline_futuresight::advance_now` unconditionally, so these
//! are a hard Layer 3 dependency, not optional). `ticket` landed 2026-08-24
//! (16-byte UmpAuthorityTicket, first consumer of CarrierKind::UmpTicketPack).
//! The remaining v2 modules (event, ring, tick_spectrum, phrase, property,
//! jr_clock, wacom_bridge, bridge, replay) are still out of scope — see
//! `TODO/handoffs/HANDOFF-2026-08-14-SPOOL-FULL-CHAIN-SPEC.md`.

pub mod futuresight;
pub mod ghost;
pub mod oracle;
pub mod packet;
pub mod provenance_tag;
pub mod recorder;
pub mod stamp_chain;
pub mod ticket;
pub mod timeline;
pub mod message;
pub mod stream;

pub use futuresight::{Alert, Futuresight, LawBook, LawContact, LockedLaw, Verdict};
pub use ghost::{CollisionKind, CollisionRadar, Contact, GhostPlayhead, HologramMap, Projection};
pub use oracle::{Admission, BranchVerdict, FutureOracle, PendingMoment, ProjectionDiff};
pub use packet::{Channel, Group, Stamped, Ump};
pub use provenance_tag::{
    required_source_kind, seal_with_kind, seal_with_kind_moon, seal_with_moon, seal_with_tier, Tier,
};
pub use recorder::{MoonTransition, Recorder, TapeIndex};
pub use stamp_chain::hash_raw;
pub use ticket::{UmpAuthorityTicket, UMP_TICKET_KIND, UMP_TICKET_SCHEMA_V0};
pub use timeline::{Scrubber, ScrubResult, SealedTuple, TickCoord, TimelineError, TimelineTape};
pub use message::{Message, ParseError};
pub use stream::UmpReader;
