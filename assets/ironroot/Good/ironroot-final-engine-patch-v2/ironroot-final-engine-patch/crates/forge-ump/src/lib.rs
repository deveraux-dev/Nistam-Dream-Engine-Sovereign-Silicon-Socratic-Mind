//! forge-ump: zero-alloc MIDI 2.0 UMP stream parser.
//!
//! Design constraints:
//! - Lazy single-pass parsing over `&[u8]`.
//! - No allocation in the parser hot path.
//! - Deterministic output from identical input bytes.
//! - Unknown/reserved message types pass through as `Message::Unknown`.
//! - JR timestamps accumulate into `Stamped::universal_tick_us` at 32 us per JR tick.

pub mod message;
pub mod packet;
pub mod stream;
pub mod ticket;

pub use message::{Message, ParseError};
pub use packet::{Channel, Group, Stamped, Ump};
pub use stream::UmpReader;
pub use ticket::UmpAuthorityTicket;
