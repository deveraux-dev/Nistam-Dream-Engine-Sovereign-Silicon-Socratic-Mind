//! 13link desktop daemon -- TLS-TOFU pairing + config. Ported from
//! `F:\NewRepo\crates\link-core` (2026-08-19, the `we-got-sdk-the-fancy-
//! rainbow` plan, Wave 3), with `connection` rewritten on `std::net` +
//! `std::thread` instead of the donor's tokio async transport (this
//! workspace has no other tokio dependency; see `connection`'s module doc).
//!
//! The donor's `plugins/` (calls/gmail/messenger/telegram/whatsapp/sms
//! relay), `cache.rs` (SQLite/LRU message cache), `notification.rs` (DND
//! filter), and `notify_desktop.rs` (OS toast notifications) are
//! deliberately NOT ported -- confirmed by recon this session to be part of
//! a notification-relay feature set out of scope for the card-game/quest-
//! board/check-in app this crate serves.

pub mod bridge;
pub mod config;
pub mod connection;
pub mod tls;
