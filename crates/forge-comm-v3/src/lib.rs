//! Invention #86 — spatial memory-palace communication
//! (`F:\docs\CORP-2748684-ALBERTA-LTD\legal\inventions\
//! 086-spatial-memory-palace-communication.md`). Contact/conversation layout
//! is a pure function of behavioral-gravity inputs, recomputed on every
//! `Constellation::rebuild` and never persisted — "the layout is the index,"
//! not a stored one. `transport` defines the matching no-persistence wire
//! shape for a NOSTR ephemeral-event transport (relay never stores kind
//! 20000-29999 either), with the live relay client left as an explicit
//! ARCH000 dependency decision.

pub mod constellation;
pub mod transport;

pub use constellation::{gravity_position, Constellation, GravityInput};
pub use transport::{ChannelFault, EphemeralChannel, NotmailEvent, NOTMAIL_GRAVITY_KIND};
