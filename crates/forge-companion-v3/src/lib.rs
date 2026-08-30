#![deny(missing_docs)]
//! forge-companion-v3 — The ONE home for companion shared types + OODA brain.
//!
//! This crate kills the v2 dependency cycle between link-behavior and link-companion
//! by providing a single authoritative home for:
//! - Behavior state machine (BehaviorState, ActiveContext, etc.)
//! - OODA event loop (SensorEvent → AnimCommand)
//! - Shared state snapshot for REST API access
//! - Animation state machine (clip selection, blending)
//! - Procedural secondary motion (physics springs, idle variants)
//! - Procedural model generation (skeleton, geometry, clips, texture)
//!
//! Zero dependencies by design — only std. Ported 2026-08-17.

pub mod animation;
pub mod generator;
pub mod model;
pub mod ooda;
pub mod physics;
pub mod types;

// Re-export the public API
pub use animation::AnimStateMachine;
pub use generator::generate_painter;
pub use model::{AnimationClip, CompanionModel, Joint, Keyframe, TextureData, Transform, Vertex};
pub use ooda::{spawn_behavior, BehaviorSnapshot, OodaEngine, SensorEvent};
pub use physics::{IdleVariant, SecondaryMotion};
pub use types::{spawn_companion, ActiveContext, AnimCommand, BehaviorState, Reaction};
