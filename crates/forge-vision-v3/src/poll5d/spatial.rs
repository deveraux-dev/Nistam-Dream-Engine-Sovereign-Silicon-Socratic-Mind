//! From F:\NewRepo\crates\forge-vision\src\poll5d\spatial.rs (lines 1-250)
//! 5D index: Morton over 12b-quantized (X,Y,Z,T,S) + bounded present-ring + k-NN.

use std::collections::VecDeque;

/// A 5D point: X/Y in tile/pixel units, Z the layer/plane, T the tick, S the LoD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P5 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub t: u64,
    pub s: u32,
}

impl P5 {
    /// Create a new 5D point.
    #[inline]
    pub fn new(x: i32, y: i32, z: i32, t: u64, s: u32) -> Self {
        Self { x, y, z, t, s }
    }
}

pub const AXIS_BITS: u32 = 12;
pub const AXIS_MASK: u32 = (1 << AXIS_BITS) - 1;
const AXES: usize = 5;

/// Interleave the 12-bit-quantized (X,Y,Z,T,S) into a 60-bit Z-order code.
pub fn morton_encode(p: &P5) -> u64 {
    let coords = [
        (p.x as u32) & AXIS_MASK,
        (p.y as u32) & AXIS_MASK,
        (p.z as u32) & AXIS_MASK,
        (p.t as u32) & AXIS_MASK,
        p.s & AXIS_MASK,
    ];
    let mut code: u64 = 0;
    for bit in 0..AXIS_BITS {
        for (axis, c) in coords.iter().enumerate() {
            let b = (c >> bit) & 1;
            code |= (b as u64) << (bit as usize * AXES + axis);
        }
    }
    code
}

/// Inverse of morton_encode.
pub fn morton_decode(code: u64) -> P5 {
    let mut coords = [0u32; AXES];
    for bit in 0..AXIS_BITS {
        for (axis, coord) in coords.iter_mut().enumerate() {
            let bv = (code >> (bit as usize * AXES + axis)) & 1;
            *coord |= (bv as u32) << bit;
        }
    }
    P5::new(
        coords[0] as i32,
        coords[1] as i32,
        coords[2] as i32,
        coords[3] as u64,
        coords[4],
    )
}

/// One indexed contact: a positioned event with a monotonic id + a weight (poll5d).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poll5dContact {
    pub id: u32,
    pub p: P5,
    pub weight: u32,
    pub morton: u64,
}

#[inline]
fn dist_sq(a: &P5, b: &P5) -> i64 {
    let dx = (a.x - b.x) as i64;
    let dy = (a.y - b.y) as i64;
    let dz = (a.z - b.z) as i64;
    let dt = a.t as i64 - b.t as i64;
    let ds = a.s as i64 - b.s as i64;
    dx * dx + dy * dy + dz * dz + dt * dt + ds * ds
}

/// Bounded present-window 5D index holding recent contacts in a ring.
pub struct Index5D {
    ring: VecDeque<Poll5dContact>,
    cap: usize,
    window: u64,
    next_id: u32,
    ingested: u64,
}

impl Index5D {
    /// Create a new 5D index with count cap and time window.
    pub fn new(cap: usize, window: u64) -> Self {
        Self {
            ring: VecDeque::with_capacity(cap.min(4096)),
            cap: cap.max(1),
            window,
            next_id: 0,
            ingested: 0,
        }
    }

    /// Insert a positioned contact; returns its id.
    pub fn insert(&mut self, p: P5, weight: u32) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.ingested = self.ingested.wrapping_add(1);
        self.ring.push_back(Poll5dContact { id, p, weight, morton: morton_encode(&p) });
        while self.ring.len() > self.cap {
            self.ring.pop_front();
        }
        id
    }

    /// Drop every contact older than the present window.
    pub fn prune(&mut self, now_t: u64) {
        let horizon = now_t.saturating_sub(self.window);
        while let Some(front) = self.ring.front() {
            if front.p.t < horizon {
                self.ring.pop_front();
            } else {
                break;
            }
        }
    }

    /// Retained contact count.
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Lifetime count of contacts ever inserted.
    pub fn ingested(&self) -> u64 {
        self.ingested
    }

    /// Count of contacts whose tick lands inside the present window.
    pub fn recent(&self, now_t: u64) -> usize {
        let lo = now_t.saturating_sub(self.window);
        self.ring.iter().filter(|c| c.p.t >= lo && c.p.t <= now_t).count()
    }

    /// Sum of contact weights in the present window.
    pub fn recent_weight(&self, now_t: u64) -> u64 {
        let lo = now_t.saturating_sub(self.window);
        self.ring
            .iter()
            .filter(|c| c.p.t >= lo && c.p.t <= now_t)
            .map(|c| c.weight as u64)
            .sum()
    }

    /// k nearest contacts to point (gauge distance), nearest first.
    pub fn knn(&self, point: &P5, k: usize) -> Vec<(u32, i64)> {
        if k == 0 || self.ring.is_empty() {
            return Vec::new();
        }
        let mut v: Vec<(u32, i64)> =
            self.ring.iter().map(|c| (c.id, dist_sq(point, &c.p))).collect();
        v.sort_unstable_by_key(|(_, d)| *d);
        v.truncate(k);
        v
    }

    /// (min, max) Morton code across the ring.
    pub fn morton_span(&self) -> Option<(u64, u64)> {
        let mut it = self.ring.iter();
        let first = it.next()?;
        let mut lo = first.morton;
        let mut hi = first.morton;
        for c in it {
            lo = lo.min(c.morton);
            hi = hi.max(c.morton);
        }
        Some((lo, hi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morton_round_trips_masked_points() {
        let samples = [
            P5::new(0, 0, 0, 0, 0),
            P5::new(100, 200, 50, 12345, 3),
            P5::new(4095, 4095, 4095, u64::MAX, 4095),
        ];
        for &p in &samples {
            let code = morton_encode(&p);
            let decoded = morton_decode(code);
            assert_eq!(
                (decoded.x & AXIS_MASK as i32, decoded.y & AXIS_MASK as i32, decoded.z & AXIS_MASK as i32,
                 decoded.t & AXIS_MASK as u64, decoded.s & AXIS_MASK),
                (p.x & AXIS_MASK as i32, p.y & AXIS_MASK as i32, p.z & AXIS_MASK as i32,
                 p.t & AXIS_MASK as u64, p.s & AXIS_MASK),
                "morton round-trip failed for {:?}", p
            );
        }
    }

    #[test]
    fn index_insert_and_len() {
        let mut idx = Index5D::new(10, 100);
        assert_eq!(idx.len(), 0);
        idx.insert(P5::new(0, 0, 0, 1, 0), 1);
        assert_eq!(idx.len(), 1);
        idx.insert(P5::new(1, 1, 1, 2, 0), 1);
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn index_caps_ring_at_capacity() {
        let mut idx = Index5D::new(5, 100);
        for i in 0..10 {
            idx.insert(P5::new(i as i32, 0, 0, i as u64, 0), 1);
        }
        assert_eq!(idx.len(), 5);
    }

    #[test]
    fn index_prune_removes_old() {
        let mut idx = Index5D::new(100, 50);
        idx.insert(P5::new(0, 0, 0, 10, 0), 1);
        idx.insert(P5::new(1, 0, 0, 60, 0), 1);
        idx.prune(100);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.ring[0].p.t, 60);
    }

    #[test]
    fn knn_ordering() {
        let mut idx = Index5D::new(100, 1000);
        idx.insert(P5::new(0, 0, 0, 0, 0), 1);
        idx.insert(P5::new(10, 0, 0, 0, 0), 1);
        idx.insert(P5::new(100, 0, 0, 0, 0), 1);
        let query = P5::new(5, 0, 0, 0, 0);
        let neighbors = idx.knn(&query, 2);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors[0].1 <= neighbors[1].1, "k-NN must be sorted by distance");
    }
}
