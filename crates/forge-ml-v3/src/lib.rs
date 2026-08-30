//! Sovereign MoE training engine — see [`moe_train`] for the model.
//!
//! [`gpu_train`] is a permanent CPU-fallback stub in this crate (see its own
//! doc and the crate's Cargo.toml header for the scope cut).

pub mod gpu_train;
pub mod moe_train;
