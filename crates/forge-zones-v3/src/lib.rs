//! forge-zones-v3 — T9 of the forge-vision drain. The Ulam spiral ring walk
//! (`UlamSpiral3D`, quarried from `F:\NewRepo\crates\forge-zones`) and the
//! 64-byte spiral/biome word (`UlamCell64`) it addresses, plus the Blueprint
//! authoring->compile pipeline (`blueprint`/`zone_state`/`from_blueprint`/
//! `svg`, ported 2026-08-16 from the same v2 crate, `forge-zones`) — the
//! Blueprint side is a second, independent tranche of the same drain, not a
//! second home for the spiral types.

mod blueprint;
mod blueprint_serde_shim;
mod blueprint_constraint;
mod blueprint_validate;
mod blueprint_validators;
mod from_blueprint;
mod mesh_ledger;
mod spiral;
mod svg;
mod word;
mod zone_state;

pub use blueprint::{
    AuthoringState, Bounds3D, BlueprintDocument, BlueprintEdge, BlueprintGraph, BlueprintLayer,
    BlueprintMeta, BlueprintMode, BlueprintNode, EdgeId, EdgeType, LayerType, NodeBounds, NodeId,
    NodeType, Pos2D, Rect2D, Vec3M,
};
pub use blueprint_constraint::{
    ConstraintSet, HardConstraint, HardConstraintKind, Preference, PreferenceKind, SoftConstraint,
    SoftConstraintKind,
};
pub use blueprint_validate::{
    BlueprintMetrics, IssueSeverity, ValidationIssue, ValidationReport, ValidationStatus,
    ValidatorModule,
};
pub use blueprint_validators::{
    validate_blueprint, validate_connectivity, validate_door_contact, validate_encounter,
    validate_room_overlap, validate_spawn_safety, validate_traversal,
};
pub use from_blueprint::{zone_from_blueprint, ZoneLowerSidecar};
pub use mesh_ledger::{render_html, MeshIntent, MeshLedger, Replay};
pub use spiral::UlamSpiral3D;
pub use svg::{render_svg, svg_markup};
pub use word::UlamCell64;
pub use zone_state::{LightSource, Marker, Shape, Volume, ZoneState};
