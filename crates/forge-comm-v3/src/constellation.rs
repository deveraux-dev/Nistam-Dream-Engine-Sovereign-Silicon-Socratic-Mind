//! Spatial layout for invention #86. Position is a pure function of behavioral
//! gravity inputs, recomputed every `rebuild` — nothing here is ever written to
//! disk. `SceneBvh` (forge-geo-v3) backs proximity/pick queries over the result.

use forge_geo_v3::{SceneBvh, SceneSpatialIndex, AABB};
use glam::Vec3;

/// One node's behavioral-gravity inputs. The caller re-derives this from its
/// own event source (message log, contact table, whatever holds the truth) —
/// this struct itself is never stored.
#[derive(Debug, Clone, Copy)]
pub struct GravityInput {
    /// Constellation node identifier (contact/conversation).
    pub node_id: u32,
    /// Total interactions with this contact/conversation.
    pub interaction_count: u32,
    /// Seconds since the most recent interaction — larger drifts outward.
    pub seconds_since_last: u64,
    /// Stable per-node direction (e.g. a hash-derived unit vector), so
    /// re-deriving the same node lands it at the same bearing every time.
    pub bearing: Vec3,
}

/// Outward-drift half-life, in seconds (one week).
const DRIFT_HALF_LIFE_SECS: f64 = 604_800.0;
/// Innermost radius (frequent, live contact).
const RADIUS_NEAR: f32 = 2.0;
/// Outermost radius (dormant contact).
const RADIUS_FAR: f32 = 200.0;
/// Half-extent of a node's query AABB (contacts are points, not volumes).
const NODE_HALF_EXTENT: f32 = 0.5;

/// Derive a node's constellation position from its gravity input. Pure
/// function — same input always yields the same position; nothing is cached.
pub fn gravity_position(input: &GravityInput) -> Vec3 {
    let decay = 0.5f64.powf(input.seconds_since_last as f64 / DRIFT_HALF_LIFE_SECS) as f32;
    let mass = (input.interaction_count as f32 + 1.0).ln();
    let pull = (mass * decay).clamp(0.0, 1.0);
    let radius = RADIUS_FAR - pull * (RADIUS_FAR - RADIUS_NEAR);
    input.bearing.normalize_or_zero() * radius
}

/// The constellation itself — a thin `SceneBvh` wrapper. Holds only the
/// in-memory query index built by the most recent `rebuild`; there is no
/// save/load path, by design (invention #86: "we don't store anything" —
/// the layout is re-derived from source events, never persisted as itself).
#[derive(Default)]
pub struct Constellation {
    index: SceneBvh,
}

impl Constellation {
    /// Build (or fully replace) the index from this call's gravity inputs.
    /// Nothing survives a `rebuild` that isn't in `inputs` — there is no
    /// incremental or persisted state that can go stale.
    pub fn rebuild(&mut self, inputs: &[GravityInput]) {
        let half = Vec3::splat(NODE_HALF_EXTENT);
        let mut nodes = inputs.iter().map(|g| {
            let p = gravity_position(g);
            (g.node_id, AABB::new(p - half, p + half))
        });
        self.index.rebuild(&mut nodes);
    }

    /// Node IDs within `radius` of `center` (method-of-loci "what's near me").
    pub fn nearby(&self, center: Vec3, radius: f32) -> Vec<u32> {
        self.index.query_radius(center, radius)
    }

    /// Nearest node hit by a ray (click-to-select in the constellation view).
    pub fn pick(&self, origin: Vec3, dir: Vec3) -> Option<(u32, f32)> {
        self.index.ray_pick(origin, dir)
    }

    /// Number of nodes in the most recent `rebuild`.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// True if `rebuild` has never been called, or was last called with an empty slice.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: u32, count: u32, secs: u64, bearing: Vec3) -> GravityInput {
        GravityInput { node_id: id, interaction_count: count, seconds_since_last: secs, bearing }
    }

    #[test]
    fn frequent_recent_contact_lands_nearer_than_dormant_one() {
        let near = input(1, 500, 60, Vec3::X);
        let far = input(2, 1, 50_000_000, Vec3::X);
        let p_near = gravity_position(&near);
        let p_far = gravity_position(&far);
        assert!(
            p_near.length() < p_far.length(),
            "frequent/recent must sit closer to origin: near={p_near} far={p_far}"
        );
    }

    #[test]
    fn same_input_always_derives_the_same_position() {
        let g = input(7, 42, 3600, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(gravity_position(&g), gravity_position(&g), "pure function, no hidden state");
    }

    #[test]
    fn rebuild_is_stateless_no_leftover_nodes_survive() {
        let mut c = Constellation::default();
        c.rebuild(&[input(1, 10, 0, Vec3::X), input(2, 10, 0, Vec3::Y)]);
        assert_eq!(c.len(), 2);

        // A second rebuild with a disjoint node set must fully replace the
        // first — nothing "stored" carries over (the whole point of #86).
        c.rebuild(&[input(3, 10, 0, Vec3::Z)]);
        assert_eq!(c.len(), 1, "old nodes must not survive rebuild");
        assert!(c.nearby(Vec3::ZERO, 1000.0).contains(&3));
        assert!(!c.nearby(Vec3::ZERO, 1000.0).contains(&1));
    }

    #[test]
    fn nearby_and_pick_query_the_derived_layout() {
        let mut c = Constellation::default();
        c.rebuild(&[
            input(1, 1000, 1, Vec3::NEG_Z), // very close, in front
            input(2, 1, 100_000_000, Vec3::NEG_Z), // far, same bearing
        ]);
        let hit = c.pick(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(hit.map(|(id, _)| id), Some(1), "nearest node along the ray must win");

        let close_only = c.nearby(Vec3::ZERO, RADIUS_NEAR + 5.0);
        assert!(close_only.contains(&1));
        assert!(!close_only.contains(&2));
    }
}
