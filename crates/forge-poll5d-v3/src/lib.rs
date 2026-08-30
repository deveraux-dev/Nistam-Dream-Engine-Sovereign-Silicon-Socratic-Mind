//! forge-poll5d-v3 — T4 of the forge-vision drain. Two exact words for the
//! 5D poll engine's state substrate: `Octal64` (64-byte tier-slot pack) and
//! `Morton8` (8-byte 5-axis Z-order code). The k-NN/adaptive-poll
//! algorithms arrive in later tranches as functions over these types, never
//! as a second home for either.

mod morton;
mod octal;

pub use morton::{Morton8, AXIS_BITS, AXIS_MAX, MORTON_AXES};
pub use octal::{Octal64, OCTAL_SLOTS, OCTAL_SLOT_BITS, OCTAL_TIER_MAX};
