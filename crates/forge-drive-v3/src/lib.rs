//! T6 — `InputFrame64` (frame.rs) plus the window-focus/key-injection
//! functions over it (inject.rs), landed as this crate's own later-tranche
//! addition per its Cargo.toml note ("window driving... arrive in later
//! tranches as functions over this type, never as a second home").

mod frame;
pub mod inject;

pub use frame::{InputFrame64, MODIFIER_MASK_MAX, PMY_MAX, STICK_NEUTRAL};
