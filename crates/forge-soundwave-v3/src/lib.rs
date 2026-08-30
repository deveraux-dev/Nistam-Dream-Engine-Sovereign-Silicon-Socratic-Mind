//! forge-soundwave-v3 — T7 of the forge-vision drain. The soundwave-ecology
//! word (`EcologyPCM8`, 8 bytes, exact): altitude/slope permyriad channels
//! plus an opaque habitat-discontinuity event-flag word. Slope's
//! 5000-neutral offset encoding is `[ASSUMED]` (T7-ecologypcm8-BRIEF.md); the
//! event_flags bit schema is deferred by ARCH000 per L17 and this crate
//! roundtrips it opaquely, inventing no flag meanings.

mod ecology;

pub use ecology::{EcologyPCM8, PMY_MAX, SLOPE_NEUTRAL};
