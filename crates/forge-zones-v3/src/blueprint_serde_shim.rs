//! Serde shims for `forge_core_v3::zones::blueprint::NodeId` — that type is
//! dependency-free by law (Crate Zero) and cannot derive `Serialize`/
//! `Deserialize` itself, and the orphan rule forbids implementing a foreign
//! trait for a foreign type from here. These shims (de)serialize `NodeId`
//! as its bare `u32`, used via `#[serde(with = "...")]` on the exact field
//! shapes this crate actually needs.

use forge_core_v3::zones::blueprint::NodeId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `#[serde(with = "blueprint_serde_shim::opt_node_id")]` for `Option<NodeId>` fields.
pub mod opt_node_id {
    use super::*;

    /// Serialize `Option<NodeId>` as its bare `Option<u32>`.
    pub fn serialize<S: Serializer>(v: &Option<NodeId>, s: S) -> Result<S::Ok, S::Error> {
        v.map(|id| id.0).serialize(s)
    }

    /// Deserialize `Option<NodeId>` from its bare `Option<u32>`.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NodeId>, D::Error> {
        Ok(Option::<u32>::deserialize(d)?.map(NodeId))
    }
}

/// `#[serde(with = "blueprint_serde_shim::vec_node_id")]` for `Vec<NodeId>` fields.
pub mod vec_node_id {
    use super::*;

    /// Serialize `Vec<NodeId>` as its bare `Vec<u32>`.
    pub fn serialize<S: Serializer>(v: &[NodeId], s: S) -> Result<S::Ok, S::Error> {
        v.iter().map(|id| id.0).collect::<Vec<_>>().serialize(s)
    }

    /// Deserialize `Vec<NodeId>` from its bare `Vec<u32>`.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<NodeId>, D::Error> {
        Ok(Vec::<u32>::deserialize(d)?.into_iter().map(NodeId).collect())
    }
}
