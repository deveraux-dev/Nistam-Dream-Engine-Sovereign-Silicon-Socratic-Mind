//! Ephemeral transport wire types for invention #86 — "we don't store
//! anything" extended to the network hop, not just the layout. A NOSTR
//! ephemeral event (kind 20000-29999) is defined by NIP-01 as never persisted
//! by a compliant relay: it is forwarded live to active subscribers and then
//! dropped, so this is the transport whose own storage policy already matches
//! the crate's.
//!
//! Zero network I/O lives here on purpose (ARCH000 precedent:
//! forge-wasibox-v3's WASI lane is the same shape — wire types + a trait now,
//! a relay-client dependency decision later). `EphemeralChannel` is the seam
//! a real `nostr-sdk`-backed implementation plugs into.

/// NOSTR ephemeral kind reserved for constellation gravity-position pushes.
/// Ephemeral range is 20000-29999 (NIP-01) — a compliant relay never stores
/// these, matching invention #86's no-persistence requirement.
pub const NOTMAIL_GRAVITY_KIND: u16 = 21122;

/// One published gravity update for a single constellation node. Carries
/// exactly what `constellation::GravityInput` needs to be re-derived by a
/// receiver — never a rendered position, so the layout math itself is never
/// shipped over the wire either.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotmailEvent {
    /// Sender's NOSTR public key (32-byte schnorr pubkey) — this IS the
    /// authorization; there is no separate auth token.
    pub pubkey: [u8; 32],
    /// Which constellation node this update applies to.
    pub node_id: u32,
    /// Total interactions with this node, as of `created_at`.
    pub interaction_count: u32,
    /// Seconds since the most recent interaction, as of `created_at`.
    pub seconds_since_last: u64,
    /// Unix seconds, for replay-window / freshness checks at the receiver.
    pub created_at: u64,
}

/// Why a channel operation failed. Kept structured (not `String`) per the
/// same invertible-error discipline `forge-daemon`'s `ApplyFault` uses —
/// the failure's own data survives the refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelFault {
    /// The event's signature did not verify against its claimed pubkey.
    BadSignature,
    /// `pubkey` is not on the receiver's allowlist.
    NotAllowed {
        /// The pubkey that was rejected.
        pubkey: [u8; 32],
    },
    /// This exact event was already processed (replay guard).
    Replay,
    /// The underlying relay connection/transport failed.
    TransportError {
        /// Implementation-specific failure description.
        detail: String,
    },
}

impl std::fmt::Display for ChannelFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSignature => write!(f, "signature verification failed"),
            Self::NotAllowed { pubkey } => write!(f, "pubkey {:02x?} not on allowlist", pubkey),
            Self::Replay => write!(f, "event already processed (replay)"),
            Self::TransportError { detail } => write!(f, "transport error: {detail}"),
        }
    }
}

impl std::error::Error for ChannelFault {}

/// The seam a live relay client implements. No implementation ships in this
/// crate — publishing/subscribing over an actual NOSTR relay is an explicit
/// ARCH000 dependency decision (which relay crate, own-relay vs public,
/// NIP-44 encryption for the tag payload), not something to pull in silently.
pub trait EphemeralChannel {
    /// Publish one gravity update. Implementations must never write it to
    /// local disk beyond what the underlying relay client needs in-flight.
    fn publish(&self, event: &NotmailEvent) -> Result<(), ChannelFault>;

    /// Drain events received since the last call. Implementations own their
    /// own replay-guard (`ChannelFault::Replay`) and allowlist enforcement
    /// (`ChannelFault::NotAllowed`) — this trait only defines the seam.
    fn poll(&mut self) -> Vec<Result<NotmailEvent, ChannelFault>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_kind_is_in_the_nip01_ephemeral_range() {
        assert!((20000..30000).contains(&(NOTMAIL_GRAVITY_KIND as u32)));
    }

    #[test]
    fn channel_fault_display_carries_its_own_data() {
        let f = ChannelFault::NotAllowed { pubkey: [0xab; 32] };
        assert!(f.to_string().contains("ab"), "refusal must carry the pubkey, not just a generic message");
    }
}
